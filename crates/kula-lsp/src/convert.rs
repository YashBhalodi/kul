//! Byte offset ↔ LSP `Position` conversion.
//!
//! LSP positions are 0-indexed lines and 0-indexed UTF-16 code units within
//! the line. `kula-core` uses UTF-8 byte offsets. The translation is
//! error-prone (off-by-one on UTF-8 multi-byte input causes silently-wrong
//! highlight ranges), so this module is small but heavily tested.
//!
//! CRLF: the `\r` is treated as part of the line content for column
//! purposes. LSP clients tolerate both conventions; staying byte-faithful
//! avoids ambiguity when the editor and server disagree about line endings.

use std::sync::Arc;

use tower_lsp::lsp_types::{Position, Range};

use kula_core::span::ByteSpan;

/// Index of line-start byte offsets in a source string.
///
/// Built once per source; lookup is O(log lines).
///
/// Holds the source as an [`Arc<str>`] so callers (notably [`crate::state::Document`])
/// can share the same heap buffer rather than carrying a duplicate copy. Constructing
/// a `LineIndex` from a `&str` allocates a fresh `Arc<str>`; constructing from an
/// existing `Arc<str>` is just a refcount bump.
#[derive(Debug, Clone)]
pub struct LineIndex {
    line_starts: Vec<usize>,
    source: Arc<str>,
}

impl LineIndex {
    /// Build a line index from `source`. Accepts anything that converts into
    /// `Arc<str>` — `&str` (clones once), `String` (reuses the heap buffer),
    /// `Arc<str>` (refcount bump only, source is shared).
    pub fn new(source: impl Into<Arc<str>>) -> Self {
        let source: Arc<str> = source.into();
        let mut line_starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self {
            line_starts,
            source,
        }
    }

    /// The source string this index was built from.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The shared `Arc<str>` backing this index, so callers can hold the
    /// same heap buffer without copying it.
    pub fn source_arc(&self) -> Arc<str> {
        Arc::clone(&self.source)
    }

    /// Convert a UTF-8 byte offset into an LSP `Position`. Out-of-range
    /// offsets clamp to the end of the source.
    pub fn position(&self, byte_offset: usize) -> Position {
        let offset = byte_offset.min(self.source.len());
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };
        let line_start = self.line_starts[line_idx];
        let line_text = &self.source[line_start..offset];
        let character = line_text.encode_utf16().count();
        Position {
            line: line_idx as u32,
            character: character as u32,
        }
    }

    /// Convert an LSP `Position` into a UTF-8 byte offset. Returns `None` if
    /// the line is past EOF.
    ///
    /// Out-of-range characters (e.g. cursor past the end of a line) clamp to
    /// the line's last code unit — matches what VSCode does.
    pub fn byte_offset(&self, position: Position) -> Option<usize> {
        let line_idx = position.line as usize;
        if line_idx >= self.line_starts.len() {
            return None;
        }
        let line_start = self.line_starts[line_idx];
        let line_end = self
            .line_starts
            .get(line_idx + 1)
            .copied()
            .unwrap_or(self.source.len());
        // Strip `\n` and a preceding `\r` so the column count reflects the
        // logical line, not the trailing newline machinery.
        let raw_line = &self.source[line_start..line_end];
        let logical_line = raw_line
            .strip_suffix('\n')
            .unwrap_or(raw_line)
            .strip_suffix('\r')
            .unwrap_or_else(|| raw_line.strip_suffix('\n').unwrap_or(raw_line));

        let mut utf16_count: u32 = 0;
        let mut byte_offset = line_start;
        for c in logical_line.chars() {
            if utf16_count >= position.character {
                break;
            }
            let units = c.len_utf16() as u32;
            if utf16_count + units > position.character {
                // Cursor lands inside a surrogate pair — clamp to the
                // character's start byte (closest valid byte boundary).
                break;
            }
            utf16_count += units;
            byte_offset += c.len_utf8();
        }
        Some(byte_offset)
    }

    /// Convert a `kula_core::ByteSpan` into an LSP `Range`.
    pub fn range(&self, span: ByteSpan) -> Range {
        Range {
            start: self.position(span.start),
            end: self.position(span.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn ascii_round_trip() {
        let idx = LineIndex::new("hello\nworld\n");
        assert_eq!(idx.position(0), pos(0, 0));
        assert_eq!(idx.position(5), pos(0, 5));
        assert_eq!(idx.position(6), pos(1, 0));
        assert_eq!(idx.position(11), pos(1, 5));
        assert_eq!(idx.byte_offset(pos(0, 0)), Some(0));
        assert_eq!(idx.byte_offset(pos(0, 5)), Some(5));
        assert_eq!(idx.byte_offset(pos(1, 0)), Some(6));
        assert_eq!(idx.byte_offset(pos(1, 5)), Some(11));
    }

    #[test]
    fn utf8_multi_byte_emoji_is_two_utf16_units() {
        // 🎉 is U+1F389: 4 bytes in UTF-8, 2 code units in UTF-16.
        let src = "a🎉b";
        let idx = LineIndex::new(src);
        assert_eq!(idx.position(0), pos(0, 0));
        assert_eq!(idx.position(1), pos(0, 1)); // before emoji
        assert_eq!(idx.position(5), pos(0, 3)); // after emoji (1 + 2 utf16)
        assert_eq!(idx.position(6), pos(0, 4)); // after b
        assert_eq!(idx.byte_offset(pos(0, 0)), Some(0));
        assert_eq!(idx.byte_offset(pos(0, 1)), Some(1));
        assert_eq!(idx.byte_offset(pos(0, 3)), Some(5));
        assert_eq!(idx.byte_offset(pos(0, 4)), Some(6));
    }

    #[test]
    fn utf8_two_byte_accented_is_one_utf16_unit() {
        // é is U+00E9: 2 bytes in UTF-8, 1 code unit in UTF-16.
        let src = "café";
        let idx = LineIndex::new(src);
        assert_eq!(idx.position(0), pos(0, 0));
        assert_eq!(idx.position(3), pos(0, 3)); // before é
        assert_eq!(idx.position(5), pos(0, 4)); // after é (1 utf16 unit)
        assert_eq!(idx.byte_offset(pos(0, 4)), Some(5));
    }

    #[test]
    fn crlf_line_breaks() {
        let src = "ab\r\ncd\r\n";
        let idx = LineIndex::new(src);
        // CR is part of line 0 (bytes 0..3 are "ab\r"); the newline at byte
        // 3 starts line 1 at byte 4.
        assert_eq!(idx.position(2), pos(0, 2)); // before \r
        assert_eq!(idx.position(3), pos(0, 3)); // on \r
        assert_eq!(idx.position(4), pos(1, 0)); // after \n
        // Round-trip from line 1 col 0 lands at byte 4 (start of "cd").
        assert_eq!(idx.byte_offset(pos(1, 0)), Some(4));
        assert_eq!(idx.byte_offset(pos(1, 2)), Some(6));
    }

    #[test]
    fn position_past_eof_clamps() {
        let idx = LineIndex::new("a");
        assert_eq!(idx.position(99), pos(0, 1));
    }

    #[test]
    fn byte_offset_past_eol_clamps_to_logical_end() {
        let idx = LineIndex::new("ab\ncd\n");
        // Asking for column 99 on line 0 clamps to end of "ab" = byte 2.
        assert_eq!(idx.byte_offset(pos(0, 99)), Some(2));
        // Asking for column 99 on line 1 clamps to end of "cd" = byte 5.
        assert_eq!(idx.byte_offset(pos(1, 99)), Some(5));
    }

    #[test]
    fn byte_offset_past_eof_returns_none() {
        let idx = LineIndex::new("ab\n");
        assert_eq!(idx.byte_offset(pos(99, 0)), None);
    }

    #[test]
    fn empty_source() {
        let idx = LineIndex::new("");
        assert_eq!(idx.position(0), pos(0, 0));
        assert_eq!(idx.byte_offset(pos(0, 0)), Some(0));
        assert_eq!(idx.byte_offset(pos(0, 5)), Some(0));
    }

    #[test]
    fn range_from_byte_span() {
        let idx = LineIndex::new("hello\nworld\n");
        let span = ByteSpan::new(6, 11);
        let r = idx.range(span);
        assert_eq!(r.start, pos(1, 0));
        assert_eq!(r.end, pos(1, 5));
    }
}

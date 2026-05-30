//! Byte spans and conversion to line/column.
//!
//! AST nodes carry bare [`ByteSpan`]s (file context comes from the owning
//! [`crate::ast::KulFile`]). Diagnostics carry [`FileSpan`]s so a
//! project-level list can interleave issues across files without losing
//! provenance. Rendering converts to line/column at the edge via
//! [`SourceMap::line_col`].

use std::ops::Range;

/// Opaque file identifier inside a [`crate::ast::Document`].
///
/// `FileId(0)` is the manifest; subsequent ids are `.kul` files in input
/// order. Equality is the only operation downstream should rely on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub(crate) u32);

impl FileId {
    /// Conventional id of the project manifest (`kul.yml`).
    pub const MANIFEST: FileId = FileId(0);

    /// Construct from a raw index. For tests and synthetic fixtures.
    pub const fn from_raw(idx: u32) -> Self {
        FileId(idx)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// `(file, byte-range)` locator for project-wide diagnostic anchoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileSpan {
    pub file: FileId,
    pub span: ByteSpan,
}

impl FileSpan {
    pub const fn new(file: FileId, span: ByteSpan) -> Self {
        Self { file, span }
    }
}

/// A half-open byte range `[start, end)` into the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteSpan {
    pub start: usize,
    pub end: usize,
}

impl ByteSpan {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl From<ByteSpan> for Range<usize> {
    fn from(span: ByteSpan) -> Self {
        span.start..span.end
    }
}

impl From<ByteSpan> for miette::SourceSpan {
    fn from(span: ByteSpan) -> Self {
        (span.start, span.len()).into()
    }
}

/// 1-indexed line/column position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    pub line: usize,
    pub column: usize,
}

/// Maps byte offsets to 1-indexed line/column. O(log lines) lookup.
#[derive(Debug, Clone)]
pub struct SourceMap {
    line_starts: Vec<usize>,
    source_len: usize,
}

impl SourceMap {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self {
            line_starts,
            source_len: source.len(),
        }
    }

    /// Byte offset → 1-indexed `LineCol`. Column is in bytes from the
    /// line start; renderers receive the source string for UTF-8 display.
    pub fn line_col(&self, offset: usize) -> LineCol {
        let offset = offset.min(self.source_len);
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };
        let line_start = self.line_starts[line_idx];
        LineCol {
            line: line_idx + 1,
            column: offset - line_start + 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_first_line() {
        let map = SourceMap::new("hello\nworld\n");
        assert_eq!(map.line_col(0), LineCol { line: 1, column: 1 });
        assert_eq!(map.line_col(4), LineCol { line: 1, column: 5 });
    }

    #[test]
    fn line_col_after_newline() {
        let map = SourceMap::new("hello\nworld\n");
        assert_eq!(map.line_col(6), LineCol { line: 2, column: 1 });
        assert_eq!(map.line_col(10), LineCol { line: 2, column: 5 });
    }

    #[test]
    fn line_col_clamps_past_end() {
        let map = SourceMap::new("a");
        assert_eq!(map.line_col(99), LineCol { line: 1, column: 2 });
    }
}

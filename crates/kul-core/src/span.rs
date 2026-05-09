//! Byte spans and conversion to line/column.
//!
//! All AST nodes carry a [`ByteSpan`] — `(start, end)` byte offsets into the
//! source — because they live inside one [`crate::ast::KulFile`] and the
//! file context is implicit. Diagnostics, by contrast, carry a [`FileSpan`]
//! — a [`ByteSpan`] paired with a [`FileId`] — so a project-level
//! diagnostic list can interleave manifest issues, parse errors, and rule
//! violations across all files of a [`crate::ast::Document`] without losing
//! file provenance. Rendering converts byte offsets to line/column once at
//! the edge, via [`SourceMap::line_col`].

use std::ops::Range;

/// Opaque identifier for one input file in a [`crate::ast::Document`].
///
/// Indices into `Document.files`, with a stable convention: `FileId(0)`
/// is always the project manifest (`kul.yml`); subsequent ids are the
/// `.kul` files in the order the toolchain assembled them. Construction
/// is mostly internal to `kul_core` — adapters and tests reach for
/// [`FileId::MANIFEST`] or read ids out of an existing [`FileSpan`] —
/// but [`FileId::from_raw`] is available as a back door for testing
/// edge cases that need a synthetic id. The integer itself is opaque:
/// downstream consumers are not expected to interpret it beyond "two
/// equal ids are the same file."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub(crate) u32);

impl FileId {
    /// The conventional id of the project manifest (`kul.yml`) inside a
    /// [`crate::ast::Document`]. Adapters that want to anchor a manifest
    /// diagnostic without first consulting the document use this
    /// constant.
    pub const MANIFEST: FileId = FileId(0);

    /// Construct a [`FileId`] from its raw index. Reserved for tests and
    /// adapter code that builds synthetic [`crate::ast::Document`]s for
    /// fixtures; production code reads ids out of existing values.
    pub const fn from_raw(idx: u32) -> Self {
        FileId(idx)
    }

    /// The raw index. Exposed for diagnostic tooling that wants to embed
    /// the id in a serialized form (e.g. JSON snapshots).
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// A `(file, byte-range)` pair: the project-wide locator a diagnostic
/// anchors on. Decouples a span from the implicit "this file" context that
/// AST nodes can rely on, so a [`crate::diagnostic::Diagnostic`] can point
/// into any file of a multi-file [`crate::ast::Document`] — the manifest,
/// or any `.kul` file — without ambiguity.
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

/// Maps byte offsets to 1-indexed line/column positions.
///
/// Built once per source string. Uses a sorted vector of line-start byte
/// offsets; lookup is O(log lines).
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

    /// Convert a byte offset into a 1-indexed line and column.
    ///
    /// Column is counted in bytes from the start of the line. (Sufficient for
    /// the diagnostic anchors we need; UTF-8 multi-byte characters render
    /// correctly when the renderer also receives the source string.)
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

//! Kul's language features, transport-free.
//!
//! Every feature is a pure function over a checked project
//! ([`kul_core::CheckResult`]): diagnostics, hover, completion, definition,
//! references, rename, symbols, quick fixes, semantic and lexical tokens,
//! formatting, entity-at, locate and details. No IO, no editor protocol and no
//! async runtime; the browser `Workspace` in `kul-wasm` is the adapter that
//! holds the files, caches the check and calls in here.
//!
//! **Addressing.** A file is addressed by its name (`InputFile.name`), the key
//! the project itself uses. Every offset is a UTF-8 **byte** offset into that
//! file's source — the unit `kul-core` works in. Converting offsets to another
//! unit (UTF-16 code units for JavaScript) is the adapter's job, done once at
//! its boundary.
//!
//! **Seams.** Features ask their questions through
//! [`ResolvedDocument`](kul_core::semantic::ResolvedDocument) and
//! [`node_at`](kul_core::semantic::ResolvedDocument::node_at) (ADR-0001), never
//! by walking another module's internals. Results are cross-file wherever the
//! language is (ADR-0015): a reference in one file resolves to a declaration in
//! another.

use kul_core::CheckResult;
use kul_core::node_at::EntityNode;
use kul_core::span::{ByteSpan, FileId, FileSpan};
use serde::Serialize;
#[cfg(feature = "tsify")]
use tsify::Tsify;

mod completion;
mod definition;
mod details;
mod diagnostics;
mod entity_at;
mod format;
mod hover;
mod lexical_tokens;
mod locate;
mod quick_fixes;
mod references;
mod rename;
mod semantic_tokens;
mod symbols;
#[cfg(test)]
mod test_support;

pub use completion::{CompletionItem, CompletionKind, Insertion, complete};
pub use definition::definition;
pub use details::details;
pub use diagnostics::{Diagnostic, Related, Severity, diagnostics};
pub use entity_at::{Entity, EntityKind, entity_at};
pub use format::format;
pub use hover::{Hover, hover};
pub use lexical_tokens::lexical_tokens;
pub use locate::locate;
pub use quick_fixes::{QuickFix, quick_fixes};
pub use references::references;
pub use rename::{RenameError, prepare_rename, rename};
pub use semantic_tokens::semantic_tokens;
pub use symbols::{OutlineSymbol, SymbolKind, symbols};

/// A half-open range `[start, end)` of offsets into one file's text.
///
/// In JavaScript the offsets are UTF-16 code units, the unit CodeMirror and
/// `String` indexing use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
pub struct Span {
    /// Offset of the first character in the range.
    pub start: usize,
    /// Offset just past the last character in the range.
    pub end: usize,
}

impl Span {
    /// Whether the two ranges overlap, counting ranges that only touch.
    #[must_use]
    pub fn touches(self, other: Span) -> bool {
        !(self.end < other.start || other.end < self.start)
    }
}

impl From<ByteSpan> for Span {
    fn from(span: ByteSpan) -> Self {
        Span {
            start: span.start,
            end: span.end,
        }
    }
}

/// A range in a named project file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
pub struct Location {
    /// The file's name, as the project names it (`family.kul`).
    pub file: String,
    /// The range in that file.
    pub span: Span,
}

/// Replace the text in `span` with `text`. An empty span inserts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
pub struct TextEdit {
    /// The range to replace.
    pub span: Span,
    /// The replacement text.
    pub text: String,
}

/// The edits one change makes to one file, in source order. Ranges are
/// against the file's text before any of the edits apply, and never overlap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
pub struct FileEdit {
    /// The file's name.
    pub file: String,
    /// Its edits, in source order.
    pub edits: Vec<TextEdit>,
}

/// What a highlighted token is.
///
/// Lexical tokens are classified from the lexer alone; semantic tokens from
/// the checked syntax tree, and they refine the lexical ones where both cover
/// the same text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum TokenKind {
    /// `person`, `marriage`, `birth`, `adoption`.
    Keyword,
    /// A field's name: `name`, `born`, `end_reason`, …
    FieldName,
    /// The id a `person` or `marriage` declares.
    Declaration,
    /// An id that refers to a declaration: a spouse, a `birth` or
    /// `adoption` marriage.
    Reference,
    /// An enumerated value: `female`, `divorce`, …
    EnumValue,
    /// A date literal: `1950`, `~1950-04`, …
    Date,
    /// A quoted string.
    String,
    /// A `#` comment.
    Comment,
}

/// One highlighted token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
pub struct Token {
    /// The token's text range. Never empty and never spans a line break.
    pub span: Span,
    /// What it is.
    pub kind: TokenKind,
}

/// The [`FileId`] of the file called `name`, if the project has one.
fn file_id(check: &CheckResult, name: &str) -> Option<FileId> {
    check
        .document()
        .kul_files()
        .find(|(_, file)| file.name == name)
        .map(|(id, _)| id)
}

/// The source text of `file`.
fn source_of(check: &CheckResult, file: FileId) -> Option<&str> {
    check.document().source_of(file)
}

/// The person or marriage id (declaration or reference) at `offset` in
/// `file`. Shared by definition, references, rename and entity-at, so the
/// four agree on what the cursor points at.
fn entity_under<'a>(check: &'a CheckResult, file: FileId, offset: usize) -> Option<EntityNode<'a>> {
    check
        .resolved()
        .node_at(file, offset)?
        .entity_reference(file)
}

/// A project-wide [`FileSpan`] as a named [`Location`].
fn location(check: &CheckResult, span: FileSpan) -> Option<Location> {
    let file = check.document().name_of(span.file)?;
    Some(Location {
        file: file.to_owned(),
        span: span.span.into(),
    })
}

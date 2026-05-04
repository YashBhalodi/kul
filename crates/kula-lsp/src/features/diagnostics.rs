//! Diagnostic translation: `kula_core::Diagnostic` → `lsp_types::Diagnostic`.
//!
//! Stub for the scaffold milestone — returns an empty list. The real
//! translation lands in #16.

use kula_core::diagnostic::Diagnostic as CoreDiagnostic;
use tower_lsp::lsp_types::Diagnostic;

use crate::convert::LineIndex;

/// Translate `kula-core` diagnostics into LSP diagnostics for one document.
///
/// Returns an empty list at this milestone; #16 wires the real conversion.
pub fn to_lsp(_diagnostics: &[CoreDiagnostic], _line_index: &LineIndex) -> Vec<Diagnostic> {
    Vec::new()
}

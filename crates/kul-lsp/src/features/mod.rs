//! LSP capability handlers.
//!
//! Each submodule implements one capability (diagnostics, hover, definition,
//! completion). They are pure functions over the cached document state,
//! which makes them unit-testable without any LSP plumbing.

pub mod code_action;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod document_symbol;
pub mod export;
pub mod formatting;
pub mod hover;
pub mod locate;
pub mod references;
pub mod rename;
pub mod render;
pub mod semantic_tokens;

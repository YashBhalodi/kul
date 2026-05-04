//! LSP capability handlers.
//!
//! Each submodule implements one capability (diagnostics, hover, definition,
//! completion). They are pure functions over the cached document state,
//! which makes them unit-testable without any LSP plumbing.

pub mod definition;
pub mod diagnostics;
pub mod hover;

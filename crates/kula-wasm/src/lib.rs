//! WebAssembly bindings for `kula-core`, published as `@kulalang/wasm`.
//!
//! Thin adapter at the workspace edge: translates a foreign protocol
//! (the JS / WASM ABI) into native Kula calls. The crate has no language
//! semantics of its own — every behavior comes from `kula-core`. Same
//! deletion-test position as `kula-lsp`: removing this crate would either
//! reproduce the JS adapter elsewhere or cut JS-ecosystem consumers off
//! from `kula-core` capabilities.
//!
//! # JS surface
//!
//! - [`format_source`] — JS-visible as `format`. Reformats a Kula source
//!   string. Always returns a string; mirrors `kula_core::format::format_source`'s
//!   best-effort contract for partial-parse input.
//! - [`kula_core_version`] — JS-visible as `KULA_CORE_VERSION`. The version
//!   of the `kula-core` crate compiled into this artifact.
//! - [`kula_language_version`] — JS-visible as `KULA_LANGUAGE_VERSION`.
//!   The version of the Kula language this artifact understands.
//! - [`export_schema_version`] — JS-visible as `EXPORT_SCHEMA_VERSION`.
//!   Schema version of the export-envelope JSON.
//!
//! # Naming
//!
//! Rust keeps snake_case; `#[wasm_bindgen(js_name = "…")]` projects each
//! item under its JS name. Function-shaped getters carry the version
//! constants because wasm-bindgen does not currently support `&'static str`
//! consts at the top level.
//!
//! # Panics
//!
//! `console_error_panic_hook::set_once()` is called from each entry point
//! so genuine bugs in `kula-core` surface as readable JS console errors
//! rather than opaque WASM traps. Idempotent across calls.
//!
//! See [PRD-0004](../../docs/prd/0004-wasm-packaging.md) for design
//! rationale and the `check` / `exportGraph` follow-on slices.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "KULA_CORE_VERSION")]
pub fn kula_core_version() -> String {
    kula_core::VERSION.into()
}

#[wasm_bindgen(js_name = "KULA_LANGUAGE_VERSION")]
pub fn kula_language_version() -> String {
    kula_core::export::LANGUAGE_VERSION.into()
}

#[wasm_bindgen(js_name = "EXPORT_SCHEMA_VERSION")]
pub fn export_schema_version() -> u32 {
    kula_core::export::SCHEMA_VERSION
}

#[wasm_bindgen(js_name = "format")]
pub fn format_source(source: &str) -> String {
    console_error_panic_hook::set_once();
    kula_core::format::format_source(source)
}

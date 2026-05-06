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
//! - [`check`] — JS-visible as `check`. Lex / parse / resolve / validate a
//!   Kula source string and return a [`CheckEnvelope`] carrying every
//!   diagnostic. Always succeeds; an empty `diagnostics` array means a clean
//!   document — emptiness is the discriminator, no `ok` field.
//! - [`export_graph`] — JS-visible as `exportGraph`. Lex / parse / resolve /
//!   validate / project to the export envelope. Strict-on-errors per
//!   [ADR-0009](../../docs/adr/0009-export-strict-on-diagnostics.md): any
//!   error-severity diagnostic produces a [`FailureEnvelope`]; otherwise a
//!   [`SuccessEnvelope`] carrying the kinship-native or cytoscape graph.
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

use kula_core::export::{ExportEnvelope, ExportOptions, ExportedDiagnostic};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

/// JS-side return type of [`check`]. Carries the full diagnostic list —
/// errors, warnings, and notes alike. An empty `diagnostics` array means
/// a clean document; consumers discriminate on emptiness rather than an
/// `ok` field, per [PRD-0004](../../docs/prd/0004-wasm-packaging.md).
///
/// Diagnostic entries reuse `kula_core::export::ExportedDiagnostic` — the
/// same shape that the failure-envelope path of `kula export` emits, so the
/// TS type lands as a single source of truth across CLI export and WASM
/// check.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct CheckEnvelope {
    pub diagnostics: Vec<ExportedDiagnostic>,
}

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

#[wasm_bindgen(js_name = "check")]
pub fn check(source: &str) -> CheckEnvelope {
    console_error_panic_hook::set_once();
    let result = kula_core::check(source);
    let diagnostics = kula_core::export::export_diagnostics(source, &result);
    CheckEnvelope { diagnostics }
}

#[wasm_bindgen(js_name = "exportGraph")]
pub fn export_graph(source: &str, options: Option<ExportOptions>) -> ExportEnvelope {
    console_error_panic_hook::set_once();
    export_with(source, options.unwrap_or_default())
}

/// Native-callable variant of [`export_graph`]. Same semantics, but takes
/// a typed [`ExportOptions`] so non-wasm tests can call into this crate
/// without round-tripping through `JsValue`. The wasm-bridge `exportGraph`
/// is a thin deserializer in front of this fn.
pub fn export_with(source: &str, options: ExportOptions) -> ExportEnvelope {
    let result = kula_core::check(source);
    kula_core::export::export(source, &result, options)
}

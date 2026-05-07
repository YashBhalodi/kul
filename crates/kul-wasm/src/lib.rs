//! WebAssembly bindings for `kul-core`, published as `@kul/wasm`.
//!
//! Thin adapter at the workspace edge: translates a foreign protocol
//! (the JS / WASM ABI) into native Kul calls. The crate has no language
//! semantics of its own — every behavior comes from `kul-core`. Same
//! deletion-test position as `kul-lsp`: removing this crate would either
//! reproduce the JS adapter elsewhere or cut JS-ecosystem consumers off
//! from `kul-core` capabilities.
//!
//! # JS surface
//!
//! - [`format_source`] — JS-visible as `format`. Reformats a Kul source
//!   string. Always returns a string; mirrors `kul_core::format::format_source`'s
//!   best-effort contract for partial-parse input.
//! - [`check`] — JS-visible as `check`. Lex / parse / resolve / validate a
//!   Kul source string and return a [`CheckEnvelope`] carrying every
//!   diagnostic. Always succeeds; an empty `diagnostics` array means a clean
//!   document — emptiness is the discriminator, no `ok` field.
//! - [`export_graph`] — JS-visible as `exportGraph`. Lex / parse / resolve /
//!   validate / project to the export envelope. Strict-on-errors per
//!   [ADR-0009](../../docs/adr/0009-export-strict-on-diagnostics.md): any
//!   error-severity diagnostic produces a [`FailureEnvelope`]; otherwise a
//!   [`SuccessEnvelope`] carrying the kinship-native or cytoscape graph.
//! - [`kul_core_version`] — JS-visible as `KUL_CORE_VERSION`. The version
//!   of the `kul-core` crate compiled into this artifact.
//! - [`kul_language_version`] — JS-visible as `KUL_LANGUAGE_VERSION`.
//!   The version of the Kul language this artifact understands.
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
//! so genuine bugs in `kul-core` surface as readable JS console errors
//! rather than opaque WASM traps. Idempotent across calls.
//!
//! See [ADR-0011](../../docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md)
//! for the surface-shape decision (three operations, three shapes, no
//! convenience layer) and [ADR-0012](../../docs/adr/0012-tsify-derived-types-committed-and-diffed.md)
//! for the TypeScript-types-from-Rust discipline.

use kul_core::export::{ExportEnvelope, ExportOptions, ExportedDiagnostic};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

/// JS-side return type of [`check`]. Carries the full diagnostic list —
/// errors, warnings, and notes alike. An empty `diagnostics` array means
/// a clean document; consumers discriminate on emptiness rather than an
/// `ok` field, per [ADR-0011](../../docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md).
///
/// Diagnostic entries reuse `kul_core::export::ExportedDiagnostic` — the
/// same shape that the failure-envelope path of `kul export` emits, so the
/// TS type lands as a single source of truth across CLI export and WASM
/// check.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct CheckEnvelope {
    pub diagnostics: Vec<ExportedDiagnostic>,
}

#[wasm_bindgen(js_name = "KUL_CORE_VERSION")]
pub fn kul_core_version() -> String {
    kul_core::VERSION.into()
}

#[wasm_bindgen(js_name = "KUL_LANGUAGE_VERSION")]
pub fn kul_language_version() -> String {
    kul_core::export::LANGUAGE_VERSION.into()
}

#[wasm_bindgen(js_name = "EXPORT_SCHEMA_VERSION")]
pub fn export_schema_version() -> u32 {
    kul_core::export::SCHEMA_VERSION
}

#[wasm_bindgen(js_name = "format")]
pub fn format_source(source: &str) -> String {
    console_error_panic_hook::set_once();
    kul_core::format::format_source(source)
}

#[wasm_bindgen(js_name = "check")]
pub fn check(source: &str) -> CheckEnvelope {
    console_error_panic_hook::set_once();
    let result = kul_core::check(source);
    let diagnostics = kul_core::export::export_diagnostics(source, &result);
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
    let result = kul_core::check(source);
    kul_core::export::export(source, &result, options)
}

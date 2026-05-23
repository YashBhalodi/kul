//! WebAssembly bindings for `kul-core`, published as `@kullang/wasm`.
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
//!   string. Takes a single `source: string`; formatting is per-file by
//!   nature (the underlying `kul_core::format::format_source` is
//!   single-source) whereas `check` and `exportGraph` are project-scoped
//!   and take the full file array — see the asymmetry note below. Always
//!   returns a string; mirrors `kul_core::format::format_source`'s
//!   best-effort contract for partial-parse input.
//! - [`check`] — JS-visible as `check`. Lex / parse / resolve / validate a
//!   Kul project (an array of `{name, source}` pairs handed in by the JS
//!   host) and return a [`CheckEnvelope`] carrying every diagnostic.
//!   Always succeeds; an empty `diagnostics` array means a clean project —
//!   emptiness is the discriminator, no `ok` field.
//! - [`export_graph`] — JS-visible as `exportGraph`. Lex / parse / resolve /
//!   validate / project the same multi-file input to the export envelope.
//!   Strict-on-errors per
//!   [ADR-0009](../../docs/adr/0009-export-strict-on-diagnostics.md): any
//!   error-severity diagnostic produces a [`FailureEnvelope`]; otherwise a
//!   [`SuccessEnvelope`] carrying the kinship-native or cytoscape graph.
//! - [`render_svg`] — JS-visible as `renderSvg`. Run the same canonical
//!   render pipeline the LSP's `kul/render` request runs
//!   (`kul_render::compute` → `kul_layout::layout` → `kul_svg::render`)
//!   and return a [`RenderEnvelope`]. Success → `{ ok: true, svg: string }`;
//!   failure (any error-severity diagnostic) → `{ ok: false, diagnostics: [...] }`.
//!   JSON shape is bit-identical to `kul_lsp::features::render::RenderResponse`.
//! - [`kul_core_version`] — JS-visible as `KUL_CORE_VERSION`. The version
//!   of the `kul-core` crate compiled into this artifact.
//! - [`kul_language_version`] — JS-visible as `KUL_LANGUAGE_VERSION`.
//!   The version of the Kul language this artifact understands.
//! - [`export_schema_version`] — JS-visible as `EXPORT_SCHEMA_VERSION`.
//!   Schema version of the export-envelope JSON.
//!
//! ## `format` asymmetry
//!
//! `check` and `exportGraph` are project-scoped — kinship resolution and
//! validation reach across files, so the JS host hands the whole file set
//! in. `format` rewrites one file's bytes; there is no cross-file
//! interaction in the formatter, so it stays per-file. The JS host
//! enumerates and reformats each file independently.
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

use kul_core::ast::InputFile;
use kul_core::export::{ExportEnvelope, ExportOptions, ExportedDiagnostic};
use kul_core::manifest::Manifest;
use kul_layout::{LayoutConfig, layout};
use kul_render::{RenderShape, compute};
use kul_svg::{ThemeConfig, render};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

/// Default opaque name for the manifest the JS host hands us. The
/// bridge receives a typed `Manifest` directly (not raw YAML), so
/// there's no real `kul.yml` body to surface; we still pick a stable
/// label so failure-envelope `file:` strings and any manifest-anchored
/// diagnostic surface a recognizable filename.
const WASM_MANIFEST_NAME: &str = "kul.yml";

/// One `.kul` input file as the JS host hands it to the bridge — a name
/// (path / URI / opaque label) plus the raw source bytes. Mirrors
/// [`kul_core::ast::InputFile`] one-to-one; the bridge converts on the
/// way in. The wasm-bridge type exists so `tsify` can derive a TS type
/// without leaking the `tsify` feature dependency onto `kul-core`'s
/// public input shape.
#[derive(Debug, Clone, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct WasmInputFile {
    pub name: String,
    pub source: String,
}

impl From<WasmInputFile> for InputFile {
    fn from(file: WasmInputFile) -> Self {
        InputFile::new(file.name, file.source)
    }
}

/// JS-side return type of [`check`]. Carries the full diagnostic list —
/// errors, warnings, and notes alike. An empty `diagnostics` array means
/// a clean project; consumers discriminate on emptiness rather than an
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
pub fn check(files: Vec<WasmInputFile>, manifest: Manifest) -> CheckEnvelope {
    console_error_panic_hook::set_once();
    let inputs: Vec<InputFile> = files.into_iter().map(Into::into).collect();
    let result = kul_core::check_with_manifest(WASM_MANIFEST_NAME, "", &manifest, &inputs);
    let diagnostics = kul_core::export::export_diagnostics(&result);
    CheckEnvelope { diagnostics }
}

#[wasm_bindgen(js_name = "exportGraph")]
pub fn export_graph(
    files: Vec<WasmInputFile>,
    manifest: Manifest,
    options: Option<ExportOptions>,
) -> ExportEnvelope {
    console_error_panic_hook::set_once();
    let inputs: Vec<InputFile> = files.into_iter().map(Into::into).collect();
    export_with(&inputs, &manifest, options.unwrap_or_default())
}

/// Native-callable variant of [`export_graph`]. Same semantics, but takes
/// the already-converted [`InputFile`] slice and a typed [`ExportOptions`]
/// so non-wasm tests can call into this crate without round-tripping
/// through `JsValue`. The wasm-bridge `exportGraph` is a thin deserializer
/// in front of this fn.
pub fn export_with(
    inputs: &[InputFile],
    manifest: &Manifest,
    options: ExportOptions,
) -> ExportEnvelope {
    let result = kul_core::check_with_manifest(WASM_MANIFEST_NAME, "", manifest, inputs);
    kul_core::export::export(&result, options)
}

/// JS-side return type of [`render_svg`]. Untagged success/failure
/// discriminated by `ok`, bit-identical at the JSON level to
/// `kul_lsp::features::render::RenderResponse` — the two adapters
/// independently construct the same envelope so JS consumers and LSP
/// clients see the same bytes regardless of how the pipeline is
/// invoked. Rule-of-three: the two adapters declare their own
/// envelopes today; a shared crate emerges only when a third
/// independent consumer materializes.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(untagged)]
pub enum RenderEnvelope {
    Success(RenderSuccess),
    Failure(RenderFailure),
}

/// Success arm of [`RenderEnvelope`]. Carries the rendered SVG string.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct RenderSuccess {
    /// Always `true`. Consumer-facing discriminator.
    pub ok: bool,
    /// The rendered SVG string. Theme-agnostic — semantic CSS classes
    /// only, no inline colours. See kul-svg for the class vocabulary.
    pub svg: String,
}

/// Failure arm of [`RenderEnvelope`]. Same diagnostic shape as the
/// failure path of [`export_graph`]; consumers narrowing on `ok: false`
/// reuse the diagnostic-rendering code they already have.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct RenderFailure {
    /// Always `false`. Consumer-facing discriminator.
    pub ok: bool,
    /// Every diagnostic the validator produced — errors, warnings, and
    /// notes alike — so the consumer sees the full picture of why the
    /// render refused.
    pub diagnostics: Vec<ExportedDiagnostic>,
}

#[wasm_bindgen(js_name = "renderSvg")]
pub fn render_svg(files: Vec<WasmInputFile>, manifest: Manifest) -> RenderEnvelope {
    console_error_panic_hook::set_once();
    let inputs: Vec<InputFile> = files.into_iter().map(Into::into).collect();
    render_svg_with(&inputs, &manifest)
}

/// Native-callable variant of [`render_svg`]. Same semantics, but takes
/// the already-converted [`InputFile`] slice so non-wasm tests can call
/// into this crate without round-tripping through `JsValue`. The wasm-
/// bridge `renderSvg` is a thin deserializer in front of this fn.
///
/// Runs the same pipeline the LSP's `kul/render` handler runs
/// ([`kul_render::compute`] → [`kul_layout::layout`] → [`kul_svg::render`])
/// with the default [`LayoutConfig`] and [`ThemeConfig`]; no options are
/// surfaced in v1 (see [ADR-0011](../../docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md)
/// — no convenience layer on the bridge).
pub fn render_svg_with(inputs: &[InputFile], manifest: &Manifest) -> RenderEnvelope {
    let result = kul_core::check_with_manifest(WASM_MANIFEST_NAME, "", manifest, inputs);
    let shape = compute(&result);
    match shape {
        RenderShape::Failure(f) => RenderEnvelope::Failure(RenderFailure {
            ok: false,
            diagnostics: f.diagnostics,
        }),
        RenderShape::Success(_) => {
            let positioned = layout(&shape, &LayoutConfig::default());
            let svg = render(&positioned, &ThemeConfig::default());
            RenderEnvelope::Success(RenderSuccess { ok: true, svg })
        }
    }
}

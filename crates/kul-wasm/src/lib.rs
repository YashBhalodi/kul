//! WebAssembly bindings for `kul-core`, published as `@kullang/wasm`.
//!
//! Thin adapter: translates the JS/WASM ABI into native Kul calls. No
//! language semantics of its own.
//!
//! `check` and `exportGraph` are project-scoped (take the full file
//! array); `format` is per-file because the underlying formatter has no
//! cross-file interaction.
//!
//! Function-shaped getters carry the version constants because
//! wasm-bindgen does not support `&'static str` consts at the top level.
//!
//! `console_error_panic_hook::set_once()` runs from each entry point so
//! `kul-core` bugs surface as readable JS console errors. Idempotent.
//!
//! See ADR-0011 (surface shape) and ADR-0012 (tsify-derived TS types).

use kul_core::ast::InputFile;
use kul_core::export::{ExportEnvelope, ExportOptions, ExportedDiagnostic};
use kul_core::manifest::Manifest;
use kul_layout::{LayoutConfig, layout};
use kul_render::{RenderShape, compute};
use kul_svg::{ThemeConfig, render};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

/// Stable label for manifest-anchored diagnostics; the bridge receives
/// a typed `Manifest` directly, not raw YAML.
const WASM_MANIFEST_NAME: &str = "kul.yml";

/// One `.kul` input file as the JS host hands it to the bridge. Mirrors
/// [`kul_core::ast::InputFile`]; exists separately so `tsify` can derive
/// a TS type without leaking the feature dependency onto `kul-core`.
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

/// JS-side return type of [`check`]. Empty `diagnostics` means clean;
/// consumers discriminate on emptiness, not an `ok` field (ADR-0011).
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

/// Native-callable variant of [`export_graph`]; lets non-wasm tests
/// call in without round-tripping through `JsValue`.
pub fn export_with(
    inputs: &[InputFile],
    manifest: &Manifest,
    options: ExportOptions,
) -> ExportEnvelope {
    let result = kul_core::check_with_manifest(WASM_MANIFEST_NAME, "", manifest, inputs);
    kul_core::export::export(&result, options)
}

/// JS-side return type of [`render_svg`]. Untagged success/failure
/// discriminated by `ok`, bit-identical to
/// `kul_lsp::features::render::RenderResponse`. Rule-of-three: a shared
/// crate emerges only when a third independent consumer materializes.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(untagged)]
pub enum RenderEnvelope {
    Success(RenderSuccess),
    Failure(RenderFailure),
}

/// Success arm of [`RenderEnvelope`].
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct RenderSuccess {
    /// Always `true`. Consumer-facing discriminator.
    pub ok: bool,
    /// Theme-agnostic SVG (semantic CSS classes, no inline colours).
    pub svg: String,
}

/// Failure arm of [`RenderEnvelope`]. Same diagnostic shape as
/// [`export_graph`]'s failure path.
#[derive(Debug, Clone, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct RenderFailure {
    /// Always `false`. Consumer-facing discriminator.
    pub ok: bool,
    pub diagnostics: Vec<ExportedDiagnostic>,
}

#[wasm_bindgen(js_name = "renderSvg")]
pub fn render_svg(files: Vec<WasmInputFile>, manifest: Manifest) -> RenderEnvelope {
    console_error_panic_hook::set_once();
    let inputs: Vec<InputFile> = files.into_iter().map(Into::into).collect();
    render_svg_with(&inputs, &manifest)
}

/// Native-callable variant of [`render_svg`]; lets non-wasm tests call
/// in without round-tripping through `JsValue`.
///
/// Runs [`compute`] → [`layout`] → [`render`] with default configs; no
/// options surfaced in v1 (ADR-0011).
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

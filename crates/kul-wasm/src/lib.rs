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
use kul_core::query::{
    MarriageLookupResult, PersonLookupResult, Query, QueryEnvelope, QueryResult, ResolveConfig,
    ResolveResult, kin_query, marriage_lookup, person_lookup, query_envelope, resolve_relationship,
};
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
        RenderShape::Success(s) => {
            let positioned = layout(&s, &LayoutConfig::default());
            let svg = render(&positioned, &ThemeConfig::default());
            RenderEnvelope::Success(RenderSuccess { ok: true, svg })
        }
    }
}

/// Named payload aliases for the query envelopes. `Option<Exported*>`
/// serializes to `Exported* | null` (json-compatible serializer, per the
/// export surface); tsify erases the transparent Rust type alias, so we
/// declare the TS names here and pin them onto the function signatures via
/// `unchecked_return_type`. This keeps the surface reading exactly like the
/// pinned sketch in issue #255 / PRD 0005.
#[wasm_bindgen(typescript_custom_section)]
const QUERY_LOOKUP_TYPES: &'static str = r#"
export type PersonLookupResult = ExportedPerson | null;
export type MarriageLookupResult = ExportedMarriage | null;
"#;

/// The fourth WASM shape (ADR-0011): the kinship query surface. Looks up a
/// person by id, gated on the project passing its checks (strict-on-errors,
/// ADR-0009). Never throws — a failing project yields the envelope's error
/// arm; a clean project yields the ok arm carrying the person in the export
/// shape, or `null` when no person has that id.
#[wasm_bindgen(
    js_name = "queryPerson",
    unchecked_return_type = "QueryEnvelope<PersonLookupResult>"
)]
pub fn query_person(
    files: Vec<WasmInputFile>,
    manifest: Manifest,
    id: String,
) -> QueryEnvelope<PersonLookupResult> {
    console_error_panic_hook::set_once();
    let inputs: Vec<InputFile> = files.into_iter().map(Into::into).collect();
    query_person_with(&inputs, &manifest, &id)
}

/// Native-callable variant of [`query_person`]; lets non-wasm tests call in
/// without round-tripping through `JsValue`.
pub fn query_person_with(
    inputs: &[InputFile],
    manifest: &Manifest,
    id: &str,
) -> QueryEnvelope<PersonLookupResult> {
    let result = kul_core::check_with_manifest(WASM_MANIFEST_NAME, "", manifest, inputs);
    person_lookup(&result, id)
}

/// Marriage-lookup counterpart to [`query_person`]. Same load-and-check
/// gate and never-throwing envelope; the ok arm carries the marriage in the
/// export shape, or `null` when no marriage has that id.
#[wasm_bindgen(
    js_name = "queryMarriage",
    unchecked_return_type = "QueryEnvelope<MarriageLookupResult>"
)]
pub fn query_marriage(
    files: Vec<WasmInputFile>,
    manifest: Manifest,
    id: String,
) -> QueryEnvelope<MarriageLookupResult> {
    console_error_panic_hook::set_once();
    let inputs: Vec<InputFile> = files.into_iter().map(Into::into).collect();
    query_marriage_with(&inputs, &manifest, &id)
}

/// Native-callable variant of [`query_marriage`].
pub fn query_marriage_with(
    inputs: &[InputFile],
    manifest: &Manifest,
    id: &str,
) -> QueryEnvelope<MarriageLookupResult> {
    let result = kul_core::check_with_manifest(WASM_MANIFEST_NAME, "", manifest, inputs);
    marriage_lookup(&result, id)
}

/// Kin-set queries on the fourth WASM shape: evaluate a declarative
/// [`Query`] value and return the matching members (person id + descriptor,
/// **no person payload** — consumers hydrate via [`query_person`]) in the
/// pinned deterministic order. Same load-and-check gate as the lookups; a
/// failing project or an unknown anchor yields the envelope's error arm with
/// a diagnostic, never a throw.
#[wasm_bindgen(
    js_name = "queryKin",
    unchecked_return_type = "QueryEnvelope<QueryResult>"
)]
pub fn query_kin(
    files: Vec<WasmInputFile>,
    manifest: Manifest,
    query: Query,
) -> QueryEnvelope<QueryResult> {
    console_error_panic_hook::set_once();
    let inputs: Vec<InputFile> = files.into_iter().map(Into::into).collect();
    query_kin_with(&inputs, &manifest, &query)
}

/// Native-callable variant of [`query_kin`]; lets non-wasm tests call in
/// without round-tripping through `JsValue`.
pub fn query_kin_with(
    inputs: &[InputFile],
    manifest: &Manifest,
    query: &Query,
) -> QueryEnvelope<QueryResult> {
    let result = kul_core::check_with_manifest(WASM_MANIFEST_NAME, "", manifest, inputs);
    kin_query(&result, query)
}

/// The general query surface on the fourth WASM shape: evaluate any
/// declarative [`Query`] — an `allPersons` or `kinOf` source, an optional
/// `where` filter, `sort`, certainty `mode`, and a `members`/`count`
/// projection — and return the [`QueryResult`] (`members`, `personIds`, or
/// `count`). The single evaluation path underlying [`query_kin`]; use this for
/// attribute-filter and count queries. Same load-and-check gate; a failing
/// project, unknown anchor, or malformed predicate yields the envelope's error
/// arm with a diagnostic, never a throw.
#[wasm_bindgen(
    js_name = "runQuery",
    unchecked_return_type = "QueryEnvelope<QueryResult>"
)]
pub fn run_query(
    files: Vec<WasmInputFile>,
    manifest: Manifest,
    query: Query,
) -> QueryEnvelope<QueryResult> {
    console_error_panic_hook::set_once();
    let inputs: Vec<InputFile> = files.into_iter().map(Into::into).collect();
    run_query_with(&inputs, &manifest, &query)
}

/// Native-callable variant of [`run_query`].
pub fn run_query_with(
    inputs: &[InputFile],
    manifest: &Manifest,
    query: &Query,
) -> QueryEnvelope<QueryResult> {
    let result = kul_core::check_with_manifest(WASM_MANIFEST_NAME, "", manifest, inputs);
    query_envelope(&result, query)
}

/// Relationship resolution on the fourth WASM shape (issue #259): return
/// **all** the ways two persons `xId` and `yId` are related, each a
/// terminology-neutral descriptor, plus — only when there are none — an
/// honest emptiness reason (`disconnected` vs `noneWithinBounds`). An omitted
/// `config` uses the default generation budget of 5. Same load-and-check gate
/// as the other query shapes; a failing project or an unknown / wrong-kind id
/// yields the envelope's error arm with a diagnostic, never a throw.
#[wasm_bindgen(
    js_name = "queryResolve",
    unchecked_return_type = "QueryEnvelope<ResolveResult>"
)]
pub fn query_resolve(
    files: Vec<WasmInputFile>,
    manifest: Manifest,
    x_id: String,
    y_id: String,
    config: Option<ResolveConfig>,
) -> QueryEnvelope<ResolveResult> {
    console_error_panic_hook::set_once();
    let inputs: Vec<InputFile> = files.into_iter().map(Into::into).collect();
    query_resolve_with(&inputs, &manifest, &x_id, &y_id, config)
}

/// Native-callable variant of [`query_resolve`]; lets non-wasm tests call in
/// without round-tripping through `JsValue`.
pub fn query_resolve_with(
    inputs: &[InputFile],
    manifest: &Manifest,
    x_id: &str,
    y_id: &str,
    config: Option<ResolveConfig>,
) -> QueryEnvelope<ResolveResult> {
    let result = kul_core::check_with_manifest(WASM_MANIFEST_NAME, "", manifest, inputs);
    resolve_relationship(&result, x_id, y_id, &config.unwrap_or_default())
}

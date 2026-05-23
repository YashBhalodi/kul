//! `kul/render` custom LSP request.
//!
//! Routes the full canonical-visual pipeline through the language
//! server so a client (today, the VSCode preview panel) can produce an
//! SVG visualisation of the in-memory buffer (including unsaved edits)
//! without shelling out to a second binary.
//!
//! Mirrors the `kul/export` shape ([`crate::features::export`]):
//! request takes a document URI; success response carries the
//! rendered SVG string; failure response carries the same diagnostic
//! list the upstream pipeline produced.

use kul_core::export::ExportedDiagnostic;
use kul_layout::{LayoutConfig, layout};
use kul_render::{RenderShape, compute};
use kul_svg::{ThemeConfig, render};
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::Url;

use crate::state::ProjectEntry;

/// Request parameters for `kul/render`. Camel-case to match LSP custom
/// requests, which conventionally mirror the protocol's casing.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderParams {
    /// The document to render. Must already be open
    /// (`textDocument/didOpen`).
    pub uri: Url,
}

/// `kul/render` response envelope. Untagged success/failure
/// discriminated by `ok`, matching the [`kul_core::export::ExportEnvelope`]
/// precedent.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum RenderResponse {
    Success(RenderSuccess),
    Failure(RenderFailure),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSuccess {
    /// Always `true`. Consumer-facing discriminator.
    pub ok: bool,
    /// The rendered SVG string (theme-agnostic; see kul-svg).
    pub svg: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderFailure {
    /// Always `false`. Consumer-facing discriminator.
    pub ok: bool,
    /// Verbatim copy of the upstream export failure's diagnostic list.
    pub diagnostics: Vec<ExportedDiagnostic>,
}

/// Possible reasons `kul/render` cannot satisfy the request before
/// even running the pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderRequestError {
    /// The URI is not in the document cache.
    DocumentNotOpen,
}

impl RenderRequestError {
    pub fn message(&self) -> String {
        match self {
            RenderRequestError::DocumentNotOpen => {
                "document is not open in the language server".to_owned()
            }
        }
    }
}

/// Pure projection: turn a cached [`ProjectEntry`] plus parsed params
/// into a render response. Lives outside `Backend` so the unit tests
/// can exercise it without spawning the full LSP server.
///
/// The render is project-wide: every URI in the same project produces
/// the same SVG (one project = one graph per ADR-0015). The `uri`
/// parameter is decorative for this layer; the integration tests pin
/// the protocol-level contract that the request identifies the
/// project to render.
pub fn render_for(
    entry: &ProjectEntry,
    _params: &RenderParams,
) -> Result<RenderResponse, RenderRequestError> {
    let shape = compute(&entry.check);
    match shape {
        RenderShape::Failure(f) => Ok(RenderResponse::Failure(RenderFailure {
            ok: false,
            diagnostics: f.diagnostics,
        })),
        RenderShape::Success(_) => {
            let positioned = layout(&shape, &LayoutConfig::default());
            let svg = render(&positioned, &ThemeConfig::default());
            Ok(RenderResponse::Success(RenderSuccess { ok: true, svg }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;

    fn dummy_uri() -> Url {
        Url::parse("file:///dummy.kul").unwrap()
    }

    fn dummy_params() -> RenderParams {
        RenderParams { uri: dummy_uri() }
    }

    #[test]
    fn render_clean_document_returns_success_with_svg() {
        let doc = test_open_file(
            "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\nmarriage m alice bob start:1972\n",
        );
        let response = render_for(&doc, &dummy_params()).expect("ok");
        match response {
            RenderResponse::Success(s) => {
                assert!(s.ok);
                assert!(
                    s.svg.starts_with("<svg"),
                    "expected an SVG document, got: {}",
                    &s.svg[..s.svg.len().min(80)]
                );
                assert!(
                    s.svg.contains("kul-card"),
                    "expected the canonical card class in SVG"
                );
            }
            RenderResponse::Failure(f) => {
                panic!("expected success, got failure: {:?}", f.diagnostics);
            }
        }
    }

    #[test]
    fn render_dirty_document_returns_failure_with_diagnostics() {
        // Missing required `name:` triggers R03.
        let doc = test_open_file("person alice gender:female\n");
        let response = render_for(&doc, &dummy_params()).expect("ok");
        match response {
            RenderResponse::Failure(f) => {
                assert!(!f.ok);
                assert!(
                    f.diagnostics.iter().any(|d| d.code == "KUL-R03"),
                    "expected R03 in failure diagnostics: {:?}",
                    f.diagnostics
                );
            }
            RenderResponse::Success(_) => panic!("expected failure for dirty document"),
        }
    }
}

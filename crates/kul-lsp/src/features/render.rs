//! `kul/render` custom LSP request — routes the canonical-visual pipeline
//! (render → layout → svg) so the preview panel can render the in-memory
//! buffer without shelling out. Mirrors the `kul/export` envelope shape.

use kul_svg::ThemeConfig;
use serde::Deserialize;
use tower_lsp::lsp_types::Url;

use crate::features::svg_envelope::{RenderResponse, SvgRequestError, render_svg_for};
use crate::state::ProjectEntry;

/// Request parameters for `kul/render`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderParams {
    /// The document to render. Must already be open.
    pub uri: Url,
}

/// Turn a cached [`ProjectEntry`] plus parsed params into a render
/// response. Thin delegator over [`render_svg_for`] with the preview
/// theme; project-wide (ADR-0015): every URI in the same project
/// produces the same SVG.
pub fn render_for(
    entry: &ProjectEntry,
    _params: &RenderParams,
) -> Result<RenderResponse, SvgRequestError> {
    Ok(render_svg_for(entry, &ThemeConfig::default()))
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
        let doc = test_open_file("person alice gender:female\n");
        let response = render_for(&doc, &dummy_params()).expect("ok");
        match response {
            RenderResponse::Failure(f) => {
                assert!(!f.ok);
                let r03 = f
                    .diagnostics
                    .iter()
                    .find(|d| d.code == "KUL-R03")
                    .expect("R03 in failure diagnostics");
                // Errors-only filter passed through.
                assert_eq!(r03.severity, "error");
                // Anchored error carries URI + LSP range so the webview can
                // click through to the source location (#203).
                assert!(r03.uri.is_some(), "expected anchored URI: {r03:?}");
                assert!(r03.range.is_some(), "expected anchored range: {r03:?}");
            }
            RenderResponse::Success(_) => panic!("expected failure for dirty document"),
        }
    }
}

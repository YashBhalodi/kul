//! `kul/exportSvg` custom LSP request — routes the canonical-visual
//! pipeline (render → layout → svg) with the *file-export* `ThemeConfig`
//! so the VSCode "Kul: Export SVG" command can write a self-contained
//! file without shelling out to the CLI. Same wire envelope as
//! `kul/render`; the only behavioural difference is the baked theme +
//! legend (ADR-0022) via [`ThemeConfig::for_file_export`].

use kul_visual::ThemeConfig;
use serde::Deserialize;
use tower_lsp::lsp_types::Url;

use crate::features::svg_envelope::{RenderResponse, render_svg_for};
use crate::state::ProjectEntry;

/// Request parameters for `kul/exportSvg`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSvgParams {
    /// The document to export. Must already be open.
    pub uri: Url,
}

/// Turn a cached [`ProjectEntry`] plus parsed params into a file-export
/// SVG response. Thin delegator over [`render_svg_for`] with the
/// file-export theme; project-wide (ADR-0015): every URI in the same
/// project produces the same SVG. The output is byte-identical to
/// `kul export --format=svg` for the same project — both call sites
/// route through [`ThemeConfig::for_file_export`].
pub fn export_svg_for(entry: &ProjectEntry, _params: &ExportSvgParams) -> RenderResponse {
    render_svg_for(entry, &ThemeConfig::for_file_export())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;

    fn dummy_uri() -> Url {
        Url::parse("file:///dummy.kul").unwrap()
    }

    fn dummy_params() -> ExportSvgParams {
        ExportSvgParams { uri: dummy_uri() }
    }

    #[test]
    fn export_svg_clean_document_returns_self_contained_svg() {
        let doc = test_open_file(
            "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\nmarriage m alice bob start:1972\n",
        );
        let response = export_svg_for(&doc, &dummy_params());
        match response {
            RenderResponse::Success(s) => {
                assert!(s.ok);
                assert!(
                    s.svg.starts_with("<svg"),
                    "expected an SVG document, got: {}",
                    &s.svg[..s.svg.len().min(80)]
                );
                // Self-contained marker — distinguishes this from the
                // theme-agnostic preview output (ADR-0016 vs ADR-0022).
                assert!(
                    s.svg.contains("<style>"),
                    "expected an inline <style> for file-export"
                );
            }
            RenderResponse::Failure(f) => {
                panic!("expected success, got failure: {:?}", f.diagnostics);
            }
        }
    }

    #[test]
    fn export_svg_dirty_document_returns_failure_with_diagnostics() {
        let doc = test_open_file("person alice gender:female\n");
        let response = export_svg_for(&doc, &dummy_params());
        match response {
            RenderResponse::Failure(f) => {
                assert!(!f.ok);
                let r03 = f
                    .diagnostics
                    .iter()
                    .find(|d| d.code == "KUL-R03")
                    .expect("R03 in failure diagnostics");
                assert_eq!(r03.severity, "error");
                assert!(r03.uri.is_some(), "expected anchored URI: {r03:?}");
                assert!(r03.range.is_some(), "expected anchored range: {r03:?}");
            }
            RenderResponse::Success(_) => panic!("expected failure for dirty document"),
        }
    }
}

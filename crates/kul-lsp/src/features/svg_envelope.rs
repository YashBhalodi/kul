//! Shared SVG surface for the LSP custom requests `kul/render` and
//! `kul/exportSvg`. Owns the wire envelope, failure projection, and the
//! two theme-choosing entrypoints — theme choice stays at this LSP
//! surface (ADR-0031); the only behavioural difference between the
//! requests is which [`ThemeConfig`] is passed to [`render_from_check`].

use kul_core::diagnostic::Severity;
use kul_visual::{ThemeConfig, render_from_check};
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Range, Url};

use crate::state::ProjectEntry;

/// Request parameters for `kul/render`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderParams {
    /// The document to render. Must already be open.
    pub uri: Url,
}

/// Request parameters for `kul/exportSvg`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSvgParams {
    /// The document to export. Must already be open.
    pub uri: Url,
}

/// Turn a cached [`ProjectEntry`] into a preview-theme SVG response.
/// Project-wide (ADR-0015): every URI in the same project produces the
/// same SVG.
pub fn render_for(entry: &ProjectEntry, _params: &RenderParams) -> RenderResponse {
    render_svg_for(entry, &ThemeConfig::default())
}

/// Turn a cached [`ProjectEntry`] into a file-export SVG response
/// (self-contained + legend via [`ThemeConfig::for_file_export`]).
/// Byte-identical to `kul export --format=svg` for the same project.
pub fn export_svg_for(entry: &ProjectEntry, _params: &ExportSvgParams) -> RenderResponse {
    render_svg_for(entry, &ThemeConfig::for_file_export())
}

/// Route the canonical-visual pipeline through the shared
/// [`render_from_check`] facade for a cached [`ProjectEntry`], projecting
/// into the shared [`RenderResponse`] envelope. This surface owns only
/// its `theme`, its failure projection (URI/range-anchored diagnostics
/// via [`errors_for_preview`]), and its output sink (the
/// `RenderResponse` envelope).
fn render_svg_for(entry: &ProjectEntry, theme: &ThemeConfig) -> RenderResponse {
    match render_from_check(&entry.check, theme) {
        Ok(svg) => RenderResponse::Success(RenderSuccess { ok: true, svg }),
        Err(_) => RenderResponse::Failure(RenderFailure {
            ok: false,
            diagnostics: errors_for_preview(entry),
        }),
    }
}

/// `kul/render` / `kul/exportSvg` response envelope, discriminated by
/// `ok` (matches [`kul_core::export::ExportEnvelope`]).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum RenderResponse {
    Success(RenderSuccess),
    Failure(RenderFailure),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSuccess {
    /// Always `true`.
    pub ok: bool,
    pub svg: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderFailure {
    /// Always `false`.
    pub ok: bool,
    pub diagnostics: Vec<RenderDiagnostic>,
}

/// Preview-tailored diagnostic. Carries an LSP `Range` (not raw byte
/// offsets) so the webview can post it back unchanged in a `revealSource`
/// message and the extension can reveal it via `vscode.window.showTextDocument`.
/// `uri` and `range` are `None` for unanchored diagnostics (e.g. `KUL-M01`),
/// which surface in the popover but cannot be clicked through.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderDiagnostic {
    pub code: String,
    pub severity: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
}

/// Project the entry's error-severity diagnostics into [`RenderDiagnostic`]s
/// for the preview's error popover. Warnings stay in the Problems pane (#203).
/// Anchored diagnostics carry their primary file's URI and LSP `Range` so the
/// webview can post them back for click-to-source.
fn errors_for_preview(entry: &ProjectEntry) -> Vec<RenderDiagnostic> {
    entry
        .check
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| {
            let location = d.primary.and_then(|primary| entry.location_for(primary));
            let (uri, range) = match location {
                Some(loc) => (Some(loc.uri.to_string()), Some(loc.range)),
                None => (None, None),
            };
            RenderDiagnostic {
                code: d.code.to_owned(),
                severity: "error",
                message: d.message.clone(),
                uri,
                range,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;

    fn dummy_uri() -> Url {
        Url::parse("file:///dummy.kul").unwrap()
    }

    fn dummy_render_params() -> RenderParams {
        RenderParams { uri: dummy_uri() }
    }

    fn dummy_export_params() -> ExportSvgParams {
        ExportSvgParams { uri: dummy_uri() }
    }

    #[test]
    fn render_clean_document_returns_success_with_svg() {
        let doc = test_open_file(
            "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\nmarriage m alice bob start:1972\n",
        );
        let response = render_for(&doc, &dummy_render_params());
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
        let response = render_for(&doc, &dummy_render_params());
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

    #[test]
    fn export_svg_clean_document_returns_self_contained_svg() {
        let doc = test_open_file(
            "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\nmarriage m alice bob start:1972\n",
        );
        let response = export_svg_for(&doc, &dummy_export_params());
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
        let response = export_svg_for(&doc, &dummy_export_params());
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

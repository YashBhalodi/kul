//! Shared response envelope for the SVG-producing LSP custom requests
//! (`kul/render`, `kul/exportSvg`). Neither feature module owns the
//! wire type — both project into the same `Success | Failure` shape so
//! the extension's failure handler is one code path.

use kul_core::diagnostic::Severity;
use kul_layout::{LayoutConfig, layout};
use kul_render::{RenderShape, compute};
use kul_svg::{ThemeConfig, render};
use serde::Serialize;
use tower_lsp::lsp_types::Range;

use crate::state::ProjectEntry;

/// Shared error for the SVG-producing requests (`kul/render`,
/// `kul/exportSvg`). Both handlers surface the identical
/// `DocumentNotOpen` case when the cached project entry is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvgRequestError {
    DocumentNotOpen,
}

impl SvgRequestError {
    pub fn message(&self) -> String {
        match self {
            SvgRequestError::DocumentNotOpen => {
                "document is not open in the language server".to_owned()
            }
        }
    }
}

/// Run the canonical-visual pipeline (render → layout → svg) for a cached
/// [`ProjectEntry`], projecting into the shared [`RenderResponse`] envelope.
/// Project-wide (ADR-0015): every URI in the same project produces the same
/// SVG. The pipeline is parameterized only by `theme` — the sole behavioural
/// difference between `kul/render` and `kul/exportSvg`.
pub fn render_svg_for(entry: &ProjectEntry, theme: &ThemeConfig) -> RenderResponse {
    let shape = compute(&entry.check);
    match shape {
        RenderShape::Failure(_) => RenderResponse::Failure(RenderFailure {
            ok: false,
            diagnostics: errors_for_preview(entry),
        }),
        RenderShape::Success(_) => {
            let positioned = layout(&shape, &LayoutConfig::default());
            let svg = render(&positioned, theme);
            RenderResponse::Success(RenderSuccess { ok: true, svg })
        }
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
pub fn errors_for_preview(entry: &ProjectEntry) -> Vec<RenderDiagnostic> {
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

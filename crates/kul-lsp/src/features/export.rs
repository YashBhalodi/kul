//! `kul/export` custom LSP request — routes the `kul export` projection
//! through the language server so the extension can export the in-memory
//! buffer without shelling out. Strict-on-errors is the export function's
//! contract; this adapter passes the envelope through verbatim.

use kul_core::export::{ExportEnvelope, ExportFormat, ExportOptions, export};
use serde::Deserialize;
use tower_lsp::lsp_types::Url;

use crate::state::ProjectEntry;

/// Request parameters for `kul/export`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportParams {
    /// The document to export. Must already be open.
    pub uri: Url,
    /// `"json"` (canonical) or `"cytoscape"` (bipartite marriage-as-node).
    pub format: String,
    /// When `true`, each exported entity carries `span: [start, end]`.
    #[serde(default)]
    pub with_positions: bool,
}

/// Reasons `kul/export` rejects the request before running the export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportRequestError {
    DocumentNotOpen,
    UnknownFormat(String),
}

impl ExportRequestError {
    pub fn message(&self) -> String {
        match self {
            ExportRequestError::DocumentNotOpen => {
                "document is not open in the language server".to_owned()
            }
            ExportRequestError::UnknownFormat(s) => {
                format!("unknown export format `{s}` (expected `json` or `cytoscape`)")
            }
        }
    }
}

/// Turn a cached [`ProjectEntry`] plus parsed params into an envelope.
/// Project-wide (ADR-0015): every URI in the same project produces the
/// same envelope.
pub fn export_for(
    entry: &ProjectEntry,
    params: &ExportParams,
) -> Result<ExportEnvelope, ExportRequestError> {
    let format = parse_format(&params.format)?;
    Ok(export(
        &entry.check,
        ExportOptions {
            format,
            with_positions: params.with_positions,
        },
    ))
}

fn parse_format(s: &str) -> Result<ExportFormat, ExportRequestError> {
    match s {
        "json" => Ok(ExportFormat::Json),
        "cytoscape" => Ok(ExportFormat::Cytoscape),
        other => Err(ExportRequestError::UnknownFormat(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::test_open_file;

    fn dummy_uri() -> Url {
        Url::parse("file:///dummy.kul").unwrap()
    }

    fn json_params() -> ExportParams {
        ExportParams {
            uri: dummy_uri(),
            format: "json".into(),
            with_positions: false,
        }
    }

    #[test]
    fn export_clean_document_returns_success_envelope() {
        let doc = test_open_file("person alice name:\"A\" gender:female\n");
        let env = export_for(&doc, &json_params()).expect("ok");
        assert!(env.is_ok(), "expected success envelope");
    }

    #[test]
    fn export_dirty_document_returns_failure_envelope() {
        let doc = test_open_file("person alice gender:female\n");
        let env = export_for(&doc, &json_params()).expect("ok");
        assert!(!env.is_ok(), "expected failure envelope");
    }

    #[test]
    fn export_cytoscape_format_routes_through_transformer() {
        let doc = test_open_file(
            "person alice name:\"A\" gender:female\nperson bob name:\"B\" gender:male\nmarriage m alice bob start:1972\n",
        );
        let params = ExportParams {
            uri: dummy_uri(),
            format: "cytoscape".into(),
            with_positions: false,
        };
        let env = export_for(&doc, &params).expect("ok");
        let ExportEnvelope::Success(s) = env else {
            panic!("expected success");
        };
        assert!(s.graph.as_cytoscape().is_some());
    }

    #[test]
    fn unknown_format_returns_request_error() {
        let doc = test_open_file("person alice name:\"A\" gender:female\n");
        let params = ExportParams {
            uri: dummy_uri(),
            format: "graphviz".into(),
            with_positions: false,
        };
        let err = export_for(&doc, &params).expect_err("expected error");
        assert!(matches!(err, ExportRequestError::UnknownFormat(_)));
        assert!(err.message().contains("graphviz"));
    }
}

//! `kul/export` custom LSP request.
//!
//! Routes the `kul export` projection through the language server so the
//! VSCode extension can produce an envelope from the in-memory buffer
//! (including unsaved edits) without shelling out to a second binary.
//!
//! The request takes a document URI plus a format (`json` or `cytoscape`)
//! and an optional `withPositions` flag, reads the cached
//! [`crate::state::Document`], and runs `kul_core::export::export`. The
//! response is the envelope (success or failure) verbatim — strict-on-
//! errors discipline is owned by the export function, not by this
//! adapter.

use kul_core::export::{ExportEnvelope, ExportFormat, ExportOptions, export};
use serde::Deserialize;
use tower_lsp::lsp_types::Url;

use crate::state::ProjectEntry;

/// Request parameters for `kul/export`. Camel-case to match LSP custom
/// requests, which conventionally mirror the protocol's casing.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportParams {
    /// The document to export. Must already be open (`textDocument/didOpen`).
    pub uri: Url,
    /// Output format — `"json"` (canonical kinship-native) or
    /// `"cytoscape"` (bipartite marriage-as-node).
    pub format: String,
    /// When `true`, every exported entity carries a `span: [start, end]`
    /// pointing back to its source declaration. Default `false`.
    #[serde(default)]
    pub with_positions: bool,
}

/// Possible reasons `kul/export` cannot satisfy the request before even
/// running the export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportRequestError {
    /// The URI is not in the document cache.
    DocumentNotOpen,
    /// The `format` field carried a value other than `"json"` or
    /// `"cytoscape"`.
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

/// Pure projection: turn a cached [`ProjectEntry`] plus parsed params
/// into an envelope. Lives outside `Backend` so the integration test
/// can exercise it without spawning the full LSP server.
///
/// Manifest failures (KUL-Mxx) flow through the export envelope as
/// regular failure-envelope diagnostics now (post-issue-70); this
/// function no longer needs a separate manifest-unavailable error.
/// The export is project-wide: every URI in the same project produces
/// the same envelope (one project = one graph per ADR-0015).
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

    /// `export_for` consumes only the cached `OpenFile` and the parsed
    /// params; the params' URI is decorative for these tests, so we
    /// reuse one stable opaque URL across the suite and let
    /// `test_open_file` build the cached check from in-memory source.
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

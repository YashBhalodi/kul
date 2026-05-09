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

use crate::state::OpenFile;

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

/// Pure projection: turn a cached [`OpenFile`] plus parsed params into an
/// envelope. Lives outside `Backend` so the integration test can
/// exercise it without spawning the full LSP server.
///
/// Manifest failures (KUL-Mxx) flow through the export envelope as
/// regular failure-envelope diagnostics now (post-issue-70); this
/// function no longer needs a separate manifest-unavailable error.
pub fn export_for(
    doc: &OpenFile,
    params: &ExportParams,
) -> Result<ExportEnvelope, ExportRequestError> {
    let format = parse_format(&params.format)?;
    Ok(export(
        &doc.check,
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
    use crate::state::Documents;

    /// Set up a unique on-disk fixture directory plus its `kul.yml` and
    /// return the URL. Each call creates a fresh directory so concurrent
    /// tests don't trip over each other.
    fn fixture_url(name: &str) -> Url {
        let dir = std::env::temp_dir().join("kul_lsp_export_unit").join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create fixture dir");
        std::fs::write(dir.join("kul.yml"), "kul: \"0.1\"\n").expect("write kul.yml");
        let kul_path = dir.join("t.kul");
        std::fs::write(&kul_path, "").expect("write kul fixture");
        Url::from_file_path(&kul_path).expect("file URL")
    }

    async fn run_export<R>(
        name: &str,
        source: &str,
        params: impl FnOnce(Url) -> ExportParams,
        f: impl FnOnce(&OpenFile, &ExportParams) -> R,
    ) -> R {
        let url = fixture_url(name);
        let docs = Documents::new();
        docs.open(url.clone(), source.to_owned()).await;
        let params = params(url.clone());
        docs.with(&url, |doc| f(doc, &params))
            .await
            .expect("doc should be open")
    }

    fn json_params(uri: Url) -> ExportParams {
        ExportParams {
            uri,
            format: "json".into(),
            with_positions: false,
        }
    }

    #[tokio::test]
    async fn export_clean_document_returns_success_envelope() {
        let env = run_export(
            "clean_doc",
            "person alice name:\"A\" gender:female\n",
            json_params,
            export_for,
        )
        .await
        .expect("ok");
        assert!(env.is_ok(), "expected success envelope");
    }

    #[tokio::test]
    async fn export_dirty_document_returns_failure_envelope() {
        let env = run_export(
            "dirty_doc",
            "person alice gender:female\n",
            json_params,
            export_for,
        )
        .await
        .expect("ok");
        assert!(!env.is_ok(), "expected failure envelope");
    }

    #[tokio::test]
    async fn export_cytoscape_format_routes_through_transformer() {
        let env = run_export(
            "cytoscape_doc",
            "person alice name:\"A\" gender:female\nperson bob name:\"B\" gender:male\nmarriage m alice bob start:1972\n",
            |uri| ExportParams {
                uri,
                format: "cytoscape".into(),
                with_positions: false,
            },
            export_for,
        )
        .await
        .expect("ok");
        let ExportEnvelope::Success(s) = env else {
            panic!("expected success");
        };
        assert!(s.graph.as_cytoscape().is_some());
    }

    #[tokio::test]
    async fn unknown_format_returns_request_error() {
        let err = run_export(
            "unknown_format",
            "person alice name:\"A\" gender:female\n",
            |uri| ExportParams {
                uri,
                format: "graphviz".into(),
                with_positions: false,
            },
            export_for,
        )
        .await
        .expect_err("expected error");
        assert!(matches!(err, ExportRequestError::UnknownFormat(_)));
        assert!(err.message().contains("graphviz"));
    }
}

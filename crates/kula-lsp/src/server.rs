//! `tower-lsp::LanguageServer` implementation.
//!
//! One method per LSP capability. Each method is a thin shell that:
//! 1. Translates the LSP request into a `kula-core` query (or pure feature
//!    function — see `features::*`).
//! 2. Translates the result back into an LSP response.
//!
//! This is the only async layer in the crate — every callee is sync.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, Hover,
    HoverParams, HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams,
    MessageType, ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    Url,
};
use tower_lsp::{Client, LanguageServer};

use crate::features::{diagnostics, hover};
use crate::state::Documents;

/// The Kula language server.
pub struct Backend {
    client: Client,
    documents: Documents,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Documents::new(),
        }
    }

    /// Translate the cached diagnostics for `uri` and publish them.
    /// Called after `did_open` and `did_change`. A no-op if the document
    /// isn't in the cache.
    async fn publish_for(&self, uri: Url, version: Option<i32>) {
        let translated = self
            .documents
            .with(&uri, |doc| {
                diagnostics::to_lsp(&uri, &doc.check.diagnostics, &doc.line_index)
            })
            .await;
        if let Some(diags) = translated {
            self.client.publish_diagnostics(uri, diags, version).await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        tracing::info!("initialize");
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "kula-lsp".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "kula-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        tracing::info!("shutdown");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let source = params.text_document.text;
        tracing::info!(uri = %uri, "document opened");
        self.documents.open(uri.clone(), source).await;
        self.publish_for(uri, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        // Full sync: the last content change carries the whole document.
        let Some(change) = params.content_changes.into_iter().next_back() else {
            return;
        };
        tracing::debug!(uri = %uri, "document changed");
        self.documents.update(uri.clone(), change.text).await;
        self.publish_for(uri, Some(version)).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        tracing::info!(uri = %uri, "document closed");
        self.documents.close(&uri).await;
        // Clear the squiggles for this document.
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let result = self
            .documents
            .with(&uri, |doc| {
                let offset = doc.line_index.byte_offset(position)?;
                let (resolved, _) = kula_core::semantic::resolve(&doc.check.document);
                hover::hover(&resolved, &doc.line_index, offset)
            })
            .await;
        Ok(result.flatten())
    }
}

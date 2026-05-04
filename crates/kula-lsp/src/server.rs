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
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use tower_lsp::{Client, LanguageServer};

use crate::state::Documents;

/// The Kula language server.
pub struct Backend {
    #[allow(dead_code)] // used by future features (publish_diagnostics, etc.)
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
        let source = params.text_document.text;
        tracing::info!(uri = %uri, "document opened");
        self.documents.open(uri, source).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // Full sync: the last content change carries the whole document.
        let Some(change) = params.content_changes.into_iter().next_back() else {
            return;
        };
        tracing::debug!(uri = %uri, "document changed");
        self.documents.update(uri, change.text).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        tracing::info!(uri = %uri, "document closed");
        self.documents.close(&uri).await;
    }
}

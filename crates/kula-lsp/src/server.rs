//! `tower-lsp::LanguageServer` implementation.
//!
//! One method per LSP capability. Each method is a thin shell that:
//! 1. Translates the LSP request into a `kula-core` query (or pure feature
//!    function — see `features::*`).
//! 2. Translates the result back into an LSP response.
//!
//! This is the only async layer in the crate — every callee is sync.

use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::{
    CodeActionParams, CodeActionProviderCapability, CodeActionResponse, CompletionOptions,
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentFormattingParams, DocumentSymbolParams,
    DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams,
    HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, Location,
    MessageType, OneOf, PrepareRenameResponse, ReferenceParams, RenameOptions, RenameParams,
    SemanticTokensFullOptions, SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, TextDocumentPositionParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url, WorkspaceEdit,
};
use tower_lsp::{Client, LanguageServer};

use crate::features::{
    code_action, completion, definition, diagnostics, document_symbol, formatting, hover,
    references, rename, semantic_tokens,
};
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
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![":".to_owned(), " ".to_owned()]),
                    ..Default::default()
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens::legend(),
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            work_done_progress_options: Default::default(),
                        },
                    ),
                ),
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
                let resolved = doc.check.resolved();
                hover::hover(&resolved, &doc.line_index, offset)
            })
            .await;
        Ok(result.flatten())
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let result = self
            .documents
            .with(&uri, |doc| {
                let offset = doc.line_index.byte_offset(position)?;
                let resolved = doc.check.resolved();
                definition::definition(&resolved, &doc.line_index, &uri, offset)
            })
            .await;
        Ok(result.flatten().map(GotoDefinitionResponse::Scalar))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;
        let actions = self
            .documents
            .with(&uri, |doc| {
                let resolved = doc.check.resolved();
                code_action::code_actions(
                    &resolved,
                    &doc.check.diagnostics,
                    &doc.line_index,
                    &uri,
                    range,
                )
            })
            .await;
        Ok(actions.filter(|a| !a.is_empty()))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;
        let result = self
            .documents
            .with(&uri, |doc| {
                let offset = doc.line_index.byte_offset(position)?;
                let resolved = doc.check.resolved();
                rename::prepare_rename(&resolved, &doc.line_index, offset)
            })
            .await;
        Ok(result.flatten())
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;
        let result = self
            .documents
            .with(&uri, |doc| {
                let offset = doc
                    .line_index
                    .byte_offset(position)
                    .ok_or(rename::RenameError::NotRenameable)?;
                let resolved = doc.check.resolved();
                rename::rename(&resolved, &doc.line_index, &uri, offset, &new_name)
            })
            .await;
        match result {
            None => Ok(None),
            Some(Ok(we)) => Ok(Some(we)),
            Some(Err(e)) => Err(Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidRequest,
                message: e.message().into(),
                data: None,
            }),
        }
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_decl = params.context.include_declaration;
        let result = self
            .documents
            .with(&uri, |doc| {
                let offset = doc.line_index.byte_offset(position)?;
                let resolved = doc.check.resolved();
                references::references(&resolved, &doc.line_index, &uri, offset, include_decl)
            })
            .await;
        Ok(result.flatten())
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let symbols = self
            .documents
            .with(&uri, |doc| {
                let resolved = doc.check.resolved();
                document_symbol::document_symbols(&resolved, &doc.line_index)
            })
            .await;
        Ok(symbols.map(DocumentSymbolResponse::Nested))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let tokens = self
            .documents
            .with(&uri, |doc| {
                let resolved = doc.check.resolved();
                semantic_tokens::semantic_tokens(&resolved, &doc.line_index)
            })
            .await;
        Ok(tokens.map(SemanticTokensResult::Tokens))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let edits = self
            .documents
            .with(&uri, |doc| {
                formatting::formatting(&doc.source, &doc.check.diagnostics, &doc.line_index)
            })
            .await;
        Ok(edits.flatten())
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let items = self
            .documents
            .with(&uri, |doc| {
                let offset = doc.line_index.byte_offset(position)?;
                let resolved = doc.check.resolved();
                Some(completion::complete(
                    doc.line_index.source(),
                    &resolved,
                    offset,
                ))
            })
            .await
            .flatten();
        Ok(items.map(CompletionResponse::Array))
    }
}

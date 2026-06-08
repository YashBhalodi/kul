//! `tower-lsp::LanguageServer` implementation. One method per LSP
//! capability; each is a thin shell over a pure `features::*` function.
//! This is the only async layer in the crate.

use kul_core::export::ExportEnvelope;
use serde_json::json;
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::{
    CodeActionParams, CodeActionProviderCapability, CodeActionResponse, CompletionOptions,
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWatchedFilesRegistrationOptions, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentFormattingParams, DocumentSymbolParams,
    DocumentSymbolResponse, FileSystemWatcher, GlobPattern, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverParams, HoverProviderCapability, InitializeParams,
    InitializeResult, InitializedParams, Location, MessageType, OneOf, PrepareRenameResponse,
    ReferenceParams, Registration, RenameOptions, RenameParams, SemanticTokensFullOptions,
    SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, TextDocumentPositionParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Url, WorkspaceEdit,
};
use tower_lsp::{Client, LanguageServer};

use crate::features::entity_at::{EntityAtParams, EntityAtResponse, entity_at};
use crate::features::export::{ExportParams, ExportRequestError, export_for};
use crate::features::locate::{LocateParams, LocateResponse, locate};
use crate::features::render::{RenderParams, RenderRequestError, render_for};
use crate::features::svg_envelope::RenderResponse;
use crate::features::{
    code_action, completion, definition, diagnostics, document_symbol, formatting, hover,
    references, rename, semantic_tokens,
};
use crate::state::{Documents, ProjectEntry, WatchAction};

/// The Kul language server.
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

    /// Handler for `kul/export`. Returns the envelope verbatim;
    /// strict-on-errors is the export function's contract.
    pub async fn export(&self, params: ExportParams) -> Result<ExportEnvelope> {
        let uri = params.uri.clone();
        let result = self
            .documents
            .with_project(&uri, |entry| export_for(entry, &params))
            .await;
        match result {
            None => Err(Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: ExportRequestError::DocumentNotOpen.message().into(),
                data: None,
            }),
            Some(Err(e)) => Err(Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: e.message().into(),
                data: None,
            }),
            Some(Ok(envelope)) => Ok(envelope),
        }
    }

    /// Handler for `kul/render`. Runs render → layout → svg.
    pub async fn render(&self, params: RenderParams) -> Result<RenderResponse> {
        let uri = params.uri.clone();
        let result = self
            .documents
            .with_project(&uri, |entry| render_for(entry, &params))
            .await;
        match result {
            None => Err(Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: RenderRequestError::DocumentNotOpen.message().into(),
                data: None,
            }),
            Some(Err(e)) => Err(Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: e.message().into(),
                data: None,
            }),
            Some(Ok(response)) => Ok(response),
        }
    }

    /// Handler for `kul/locate` — id → decl `Location` for click-to-source.
    pub async fn locate(&self, params: LocateParams) -> Result<LocateResponse> {
        let uri = params.uri.clone();
        let result = self
            .documents
            .with_project(&uri, |entry| locate(entry, &params))
            .await;
        match result {
            None => Err(Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: format!("document not open: {uri}").into(),
                data: None,
            }),
            Some(response) => Ok(response),
        }
    }

    /// Handler for `kul/entityAt` — cursor → entity id (inverse of [`Self::locate`]).
    pub async fn entity_at(&self, params: EntityAtParams) -> Result<EntityAtResponse> {
        let uri = params.uri.clone();
        let result = self
            .documents
            .with_project(&uri, |entry| entity_at(entry, &params))
            .await;
        match result {
            None => Err(Error {
                code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
                message: format!("document not open: {uri}").into(),
                data: None,
            }),
            Some(response) => Ok(response),
        }
    }

    /// Broadcast diagnostics for every `.kul` file in the project so the
    /// Problems pane reflects project-wide health. Other URIs publish
    /// with version `None` since they aren't the active one.
    async fn publish_project(&self, active_uri: &Url, active_uri_version: Option<i32>) {
        let snapshot = self
            .documents
            .with_project(active_uri, collect_project_diagnostics)
            .await;
        let Some(snapshot) = snapshot else {
            return;
        };
        for (url, diags) in snapshot {
            let version = if &url == active_uri {
                active_uri_version
            } else {
                None
            };
            self.client.publish_diagnostics(url, diags, version).await;
        }
    }
}

/// Per-URL LSP diagnostic lists for every `.kul` file in the project.
/// Each URL gets either its translated diagnostics or an empty list —
/// the empty list clears stale squiggles for files that left the error state.
fn collect_project_diagnostics(
    entry: &ProjectEntry,
) -> Vec<(Url, Vec<tower_lsp::lsp_types::Diagnostic>)> {
    let mut out = Vec::with_capacity(entry.project_urls().len());
    for url in entry.project_urls() {
        let Some(file) = entry.file_id_for(url) else {
            continue;
        };
        let diags = diagnostics::to_lsp(entry, file, &entry.check.diagnostics);
        out.push((url.clone(), diags));
    }
    out
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
                // Custom capabilities advertised under `experimental` so
                // clients can detect support before sending the request.
                experimental: Some(json!({
                    "kulExport": {
                        "formats": ["json", "cytoscape"],
                        "supportsPositions": true,
                    },
                    "kulRender": {
                        // Theme-agnostic SVG with semantic CSS classes (ADR-0016).
                        "format": "svg",
                    },
                })),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "kul-lsp".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "kul-lsp initialized")
            .await;
        // Register file watchers for `.kul` and `kul.yml`. Fire-and-forget
        // via `tokio::spawn` so a client that doesn't support dynamic
        // registration can't stall the LSP lifecycle.
        let client = self.client.clone();
        tokio::spawn(async move {
            let registrations = vec![Registration {
                id: "kul-watched-files".to_owned(),
                method: "workspace/didChangeWatchedFiles".to_owned(),
                register_options: Some(
                    serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                        watchers: vec![
                            FileSystemWatcher {
                                glob_pattern: GlobPattern::String("**/*.kul".to_owned()),
                                kind: None,
                            },
                            FileSystemWatcher {
                                glob_pattern: GlobPattern::String("**/kul.yml".to_owned()),
                                kind: None,
                            },
                        ],
                    })
                    .expect("DidChangeWatchedFilesRegistrationOptions serializes"),
                ),
            }];
            if let Err(e) = client.register_capability(registrations).await {
                tracing::debug!(error = ?e, "client rejected workspace/didChangeWatchedFiles registration");
            }
        });
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
        self.publish_project(&uri, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        // Full sync: last change carries the whole document.
        let Some(change) = params.content_changes.into_iter().next_back() else {
            return;
        };
        tracing::debug!(uri = %uri, "document changed");
        self.documents.update(uri.clone(), change.text).await;
        self.publish_project(&uri, Some(version)).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for event in params.changes {
            let uri = event.uri;
            let kind = event.typ;
            let action = self.documents.process_watcher_event(&uri, kind).await;
            tracing::debug!(
                uri = %uri,
                kind = ?kind,
                action = action.log_label(),
                "workspace/didChangeWatchedFiles",
            );
            match action {
                WatchAction::Ignored { .. } => {}
                WatchAction::Reloaded { cleared } => {
                    self.publish_project(&uri, None).await;
                    for url in cleared {
                        self.client.publish_diagnostics(url, Vec::new(), None).await;
                    }
                }
                WatchAction::Evicted { cleared } => {
                    for url in cleared {
                        self.client.publish_diagnostics(url, Vec::new(), None).await;
                    }
                }
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        tracing::info!(uri = %uri, "document closed");
        let (urls, evicted) = self.documents.close(&uri).await;
        if evicted {
            for url in urls {
                self.client.publish_diagnostics(url, Vec::new(), None).await;
            }
            // Also clear the closing URI in case the project entry was never built.
            self.client.publish_diagnostics(uri, Vec::new(), None).await;
        } else {
            self.client
                .publish_diagnostics(uri.clone(), Vec::new(), None)
                .await;
            self.publish_project(&uri, None).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let result = self
            .documents
            .with_project(&uri, |entry| {
                let c = entry.cursor_for_uri(&uri, position)?;
                hover::hover(c.file, c.resolved, c.line_index, c.offset)
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
            .with_project(&uri, |entry| definition::definition(entry, &uri, position))
            .await;
        Ok(result.flatten().map(GotoDefinitionResponse::Scalar))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;
        let actions = self
            .documents
            .with_project(&uri, |entry| {
                let file = entry.file_id_for(&uri)?;
                let line_index = entry.line_index_for(file)?;
                let check = &entry.check;
                Some(code_action::code_actions(
                    file,
                    check.resolved(),
                    &check.diagnostics,
                    line_index,
                    &uri,
                    range,
                ))
            })
            .await
            .flatten();
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
            .with_project(&uri, |entry| {
                let c = entry.cursor_for_uri(&uri, position)?;
                rename::prepare_rename(c.file, c.resolved, c.line_index, c.offset)
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
            .with_project(&uri, |entry| {
                rename::rename(entry, &uri, position, &new_name)
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
            .with_project(&uri, |entry| {
                references::references(entry, &uri, position, include_decl)
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
            .with_project(&uri, |entry| {
                let v = entry.view_for_uri(&uri)?;
                Some(document_symbol::document_symbols(
                    v.file,
                    v.resolved,
                    v.line_index,
                ))
            })
            .await
            .flatten();
        Ok(symbols.map(DocumentSymbolResponse::Nested))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let tokens = self
            .documents
            .with_project(&uri, |entry| {
                let v = entry.view_for_uri(&uri)?;
                Some(semantic_tokens::semantic_tokens(
                    v.file,
                    v.resolved,
                    v.line_index,
                ))
            })
            .await
            .flatten();
        Ok(tokens.map(SemanticTokensResult::Tokens))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let edits = self
            .documents
            .with_project(&uri, |entry| {
                let v = entry.view_for_uri(&uri)?;
                formatting::formatting(
                    v.line_index.source(),
                    &entry.check.diagnostics,
                    v.line_index,
                    v.file,
                )
            })
            .await
            .flatten();
        Ok(edits)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let items = self
            .documents
            .with_project(&uri, |entry| {
                let c = entry.cursor_for_uri(&uri, position)?;
                Some(completion::complete(
                    c.line_index.source(),
                    c.file,
                    c.resolved,
                    c.offset,
                ))
            })
            .await
            .flatten();
        Ok(items.map(CompletionResponse::Array))
    }
}

//! Kula language server.
//!
//! Thin adapter over `kula-core`: exposes lex/parse/resolve/validate as LSP
//! capabilities. This crate intentionally has no language semantics of its
//! own — every diagnostic, hover, definition, completion answer comes from
//! `kula-core`. The async layer here is purely for LSP-protocol concurrency.

pub mod convert;
pub mod features;
pub mod server;
pub mod state;

use tower_lsp::{LspService, Server};

/// Run the language server over stdio. Blocks until the client disconnects.
///
/// Initializes `tracing` to stderr (LSP convention — stdin/stdout are the
/// protocol channel). Set `RUST_LOG=kula_lsp=debug` for verbose logs.
pub async fn run() {
    init_tracing();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(server::Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("kula_lsp=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();
}

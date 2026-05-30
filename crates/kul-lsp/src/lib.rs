//! Kul language server.
//!
//! Thin adapter over `kul-core`: every diagnostic, hover, definition, and
//! completion answer comes from the core query seam (ADR-0001). The async
//! layer here is purely for LSP-protocol concurrency.

pub mod convert;
pub mod features;
pub mod server;
pub mod state;

use tower_lsp::{LspService, Server};

/// Run the language server over stdio. Blocks until the client disconnects.
///
/// Logs to stderr (stdin/stdout are the protocol channel). Set
/// `RUST_LOG=kul_lsp=debug` for verbose logs.
pub async fn run() {
    init_tracing();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::build(server::Backend::new)
        .custom_method("kul/export", server::Backend::export)
        .custom_method("kul/render", server::Backend::render)
        .custom_method("kul/locate", server::Backend::locate)
        .custom_method("kul/entityAt", server::Backend::entity_at)
        .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("kul_lsp=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();
}

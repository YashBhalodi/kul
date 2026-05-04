//! `kula-lsp` binary entry point.

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    kula_lsp::run().await;
}

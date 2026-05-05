//! `kula-lsp` binary entry point.

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("kula-lsp {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--help" | "-h" => {
                println!(
                    "kula-lsp {} — Kula language server (LSP over stdio)\n\n\
                     USAGE:\n  \
                     kula-lsp                  speak LSP over stdin/stdout\n  \
                     kula-lsp --version        print version and exit\n  \
                     kula-lsp --help           print this help and exit\n\n\
                     ENVIRONMENT:\n  \
                     RUST_LOG  Filter directive for tracing logs (e.g. `kula_lsp=debug`).\n            \
                     Defaults to `kula_lsp=info`.",
                    env!("CARGO_PKG_VERSION")
                );
                return;
            }
            _ => {
                eprintln!("kula-lsp: unknown argument `{arg}` (try --help)");
                std::process::exit(2);
            }
        }
    }
    kula_lsp::run().await;
}

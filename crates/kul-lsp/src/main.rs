#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("kul-lsp {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--help" | "-h" => {
                println!(
                    "kul-lsp {} — Kul language server (LSP over stdio)\n\n\
                     USAGE:\n  \
                     kul-lsp                  speak LSP over stdin/stdout\n  \
                     kul-lsp --version        print version and exit\n  \
                     kul-lsp --help           print this help and exit\n\n\
                     ENVIRONMENT:\n  \
                     RUST_LOG  Filter directive for tracing logs (e.g. `kul_lsp=debug`).\n            \
                     Defaults to `kul_lsp=info`.",
                    env!("CARGO_PKG_VERSION")
                );
                return;
            }
            _ => {
                eprintln!("kul-lsp: unknown argument `{arg}` (try --help)");
                std::process::exit(2);
            }
        }
    }
    kul_lsp::run().await;
}

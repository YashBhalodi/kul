use clap::Parser;

const VERSION_STRING: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (kula-core ",
    env!("CARGO_PKG_VERSION"),
    ")",
);

#[derive(Parser, Debug)]
#[command(name = "kula", version = VERSION_STRING, about = "Kula language CLI")]
struct Cli {}

fn main() -> anyhow::Result<()> {
    let _ = Cli::parse();
    // Subcommands land in later issues.
    Ok(())
}

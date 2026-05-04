use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod commands;

const VERSION_STRING: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (kula-core ",
    env!("CARGO_PKG_VERSION"),
    ")",
);

#[derive(Parser, Debug)]
#[command(
    name = "kula",
    version = VERSION_STRING,
    about = "Kula language CLI",
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Validate one or more `.kula` files.
    Validate {
        /// Files to validate.
        #[arg(value_name = "FILE", required = true)]
        files: Vec<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate { files } => commands::validate::run(&files),
    }
}

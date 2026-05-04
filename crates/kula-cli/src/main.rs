use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

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
    /// Validate one or more `.kula` files. Use `-` to read from stdin.
    Validate {
        /// Files to validate. Use `-` to read from standard input.
        #[arg(value_name = "FILE", required = true)]
        files: Vec<PathBuf>,

        /// Suppress per-file `<path>: ok` lines on success.
        #[arg(short, long)]
        quiet: bool,

        /// Output format for diagnostics.
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,

        /// Force colorless output even when stderr is a TTY.
        #[arg(long)]
        no_color: bool,
    },

    /// (stub — Phase 4) Format a `.kula` file in place.
    Format {
        /// Files to format.
        #[arg(value_name = "FILE")]
        files: Vec<PathBuf>,
    },

    /// (stub — Phase 3) Run the Kula language server over stdio.
    Lsp,
}

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Validate {
            files,
            quiet,
            format,
            no_color,
        } => commands::validate::run(commands::validate::Options {
            files,
            quiet,
            format,
            no_color,
        }),
        Command::Format { .. } => {
            eprintln!("kula format: not yet implemented (Phase 4)");
            ExitCode::from(2)
        }
        Command::Lsp => {
            eprintln!("kula lsp: not yet implemented (Phase 3)");
            ExitCode::from(2)
        }
    }
}

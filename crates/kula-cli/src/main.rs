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

const TOP_LONG_ABOUT: &str = "\
Kula language CLI — parse, validate, and (later) format .kula documents.

A Kula document describes a family: persons, marriages, biological births,
and adoptions. `kula validate` parses a document and reports the 13
spec-defined errors with line/column anchors.

EXAMPLES:
  kula validate family.kula
  kula validate examples/*.kula
  cat family.kula | kula validate -
  kula validate --format json family.kula | jq .
  kula validate --quiet family.kula && echo ok

EXIT CODES:
  0  every input validated cleanly
  1  at least one input had error diagnostics
  2  CLI usage error or stub subcommand invoked

SEE ALSO:
  Spec ........ https://github.com/YashBhalodi/kulalang/tree/main/spec
  Issues ...... https://github.com/YashBhalodi/kulalang/issues
";

#[derive(Parser, Debug)]
#[command(
    name = "kula",
    version = VERSION_STRING,
    about = "Kula language CLI",
    long_about = TOP_LONG_ABOUT,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

const VALIDATE_LONG_ABOUT: &str = "\
Validate one or more .kula files against the Kula language specification.

Each file is parsed and run through the validator. The validator reports the
13 spec-defined errors (KULA-R01 through KULA-R13) with file/line/column
anchors. The exit code is 0 if every input is clean and 1 if any input had
error diagnostics.

Pass `-` as a filename to read source from standard input; the file label
in diagnostics will be `<stdin>`.

EXAMPLES:
  # Validate a single file.
  kula validate family.kula

  # Validate every example.
  kula validate examples/*.kula

  # Read from stdin (label `<stdin>`).
  cat family.kula | kula validate -

  # Quiet mode for scripts: only diagnostics, no `ok` lines.
  kula validate --quiet family.kula

  # Machine-readable output (one JSON object per diagnostic, jsonl).
  kula validate --format json family.kula

  # Force colorless output (useful in CI logs).
  kula validate --no-color family.kula

JSON OUTPUT (--format json):
  Each diagnostic is one JSON object on its own line, with this schema:

  {
    \"code\":     \"KULA-R03\",
    \"severity\": \"error\",
    \"message\":  \"person `alice` is missing required field `name`\",
    \"file\":     \"family.kula\",
    \"primary\":  { \"byte_start\": 7, \"byte_end\": 12,
                    \"line\": 1, \"column\": 8 },
    \"related\":  [ { \"label\": \"prior declaration\",
                      \"byte_start\": …, \"byte_end\": …,
                      \"line\": …, \"column\": … } ]
  }
";

#[derive(Subcommand, Debug)]
enum Command {
    /// Validate one or more `.kula` files. Use `-` to read from stdin.
    #[command(long_about = VALIDATE_LONG_ABOUT)]
    Validate {
        /// Files to validate. Use `-` to read from standard input.
        #[arg(value_name = "FILE", required = true)]
        files: Vec<PathBuf>,

        /// Suppress per-file `<path>: ok` lines on success.
        ///
        /// Diagnostics are still printed; only the success line is suppressed.
        /// Useful for scripts that only care about the exit code.
        #[arg(short, long)]
        quiet: bool,

        /// Output format for diagnostics.
        ///
        /// `human` — Rust-compiler-style rendering with source snippets and
        /// caret anchors (default).
        /// `json`  — one JSON object per diagnostic, newline-delimited
        /// (jsonl). See `kula help validate` for the schema.
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,

        /// Force colorless output even when stderr is a TTY.
        ///
        /// Color is auto-disabled when stderr is not a TTY (e.g. when piped
        /// into a file). This flag forces it off unconditionally.
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

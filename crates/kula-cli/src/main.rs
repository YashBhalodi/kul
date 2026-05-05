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
Kula language CLI — parse and validate .kula documents.

A Kula document describes a family: persons, marriages, biological births,
and adoptions. `kula validate` parses a document and reports the 13
spec-defined errors with line/column anchors. `kula lsp` runs the language
server over stdio for editor integrations.

EXAMPLES:
  kula validate family.kula
  kula validate examples/*.kula
  cat family.kula | kula validate -
  kula validate --format json family.kula | jq .
  kula validate --quiet family.kula && echo ok
  kula format family.kula       # canonicalize the file in place
  kula format --check family.kula  # CI gate: non-zero if not canonical
  kula lsp                      # speak LSP over stdio (editor integrations)

EXIT CODES:
  0  every input validated/formatted cleanly
  1  at least one input had error diagnostics, or (under --check) was not
     in canonical form
  2  CLI usage error (e.g. unknown flag, missing argument)

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

const FORMAT_LONG_ABOUT: &str = "\
Format one or more `.kula` files in canonical form (per ADR 0004).

Without `--check`, each file is rewritten in place. With `--check`, no file
is modified — the command exits non-zero if any input is not already in
canonical form, which is the right shape for a CI gate.

Pass `-` as a filename to read from standard input. In default mode the
formatted source is written to stdout; in `--check` mode the command is
silent on success and prints `<stdin>: not formatted` to stderr if not.

EXAMPLES:
  # Canonicalize a file in place.
  kula format family.kula

  # Canonicalize every example.
  kula format examples/*.kula

  # CI gate: fail if anything is out of canonical form.
  kula format --check examples/*.kula

  # Read from stdin, write canonical form to stdout.
  cat family.kula | kula format -

EXIT CODES:
  0  every file is in canonical form (or was successfully formatted)
  1  at least one input had parse errors, or (under --check) was not in
     canonical form
  2  CLI usage error
";

const LSP_LONG_ABOUT: &str = "\
Run the Kula language server over stdio.

This subcommand is intended for editor integrations: an editor's LSP client
spawns `kula lsp` as a child process and exchanges JSON-RPC messages over
stdin/stdout. Logs go to stderr.

ENVIRONMENT:
  RUST_LOG  Filter directive for tracing logs (e.g. `kula_lsp=debug`).
            Defaults to `kula_lsp=info`.
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

    /// Format one or more `.kula` files. Use `-` to read from stdin.
    #[command(long_about = FORMAT_LONG_ABOUT)]
    Format {
        /// Files to format. Use `-` to read from standard input.
        #[arg(value_name = "FILE", required = true)]
        files: Vec<PathBuf>,

        /// Verify formatting without modifying files. Exits non-zero if any
        /// input is not already in canonical form. Suitable for CI.
        #[arg(long)]
        check: bool,
    },

    /// Run the language server over stdio.
    #[command(long_about = LSP_LONG_ABOUT)]
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
        Command::Format { files, check } => {
            commands::format::run(commands::format::Options { files, check })
        }
        Command::Lsp => run_lsp(),
    }
}

fn run_lsp() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("kula: failed to start language server runtime: {err}");
            return ExitCode::from(1);
        }
    };
    runtime.block_on(kula_lsp::run());
    ExitCode::SUCCESS
}

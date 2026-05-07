use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

mod commands;

const VERSION_STRING: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (kul-core ",
    env!("CARGO_PKG_VERSION"),
    ")",
);

const TOP_LONG_ABOUT: &str = "\
Kul language CLI — parse, validate, format, and export .kul documents.

A Kul document describes a family: persons, marriages, biological births,
and adoptions. `kul validate` parses a document and reports the 13
spec-defined errors with line/column anchors. `kul export` projects a
clean document into a JSON graph for downstream consumers. `kul lsp` runs
the language server over stdio for editor integrations.

EXAMPLES:
  kul validate family.kul
  kul validate examples/*.kul
  cat family.kul | kul validate -
  kul validate --format json family.kul | jq .
  kul validate --quiet family.kul && echo ok
  kul format family.kul       # canonicalize the file in place
  kul format --check family.kul  # CI gate: non-zero if not canonical
  kul export family.kul | jq .   # project clean document to JSON
  kul lsp                      # speak LSP over stdio (editor integrations)

EXIT CODES:
  0  every input validated/formatted/exported cleanly
  1  at least one input had error diagnostics, or (under --check) was not
     in canonical form
  2  CLI usage error (e.g. unknown flag, missing argument)

SEE ALSO:
  Spec ........ https://github.com/YashBhalodi/kul/tree/main/spec
  Issues ...... https://github.com/YashBhalodi/kul/issues
";

#[derive(Parser, Debug)]
#[command(
    name = "kul",
    version = VERSION_STRING,
    about = "Kul language CLI",
    long_about = TOP_LONG_ABOUT,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

const VALIDATE_LONG_ABOUT: &str = "\
Validate one or more .kul files against the Kul language specification.

Each file is parsed and run through the validator. The validator reports the
13 spec-defined errors (KUL-R01 through KUL-R13) with file/line/column
anchors. The exit code is 0 if every input is clean and 1 if any input had
error diagnostics.

Pass `-` as a filename to read source from standard input; the file label
in diagnostics will be `<stdin>`.

EXAMPLES:
  # Validate a single file.
  kul validate family.kul

  # Validate every example.
  kul validate examples/*.kul

  # Read from stdin (label `<stdin>`).
  cat family.kul | kul validate -

  # Quiet mode for scripts: only diagnostics, no `ok` lines.
  kul validate --quiet family.kul

  # Machine-readable output (one JSON object per diagnostic, jsonl).
  kul validate --format json family.kul

  # Force colorless output (useful in CI logs).
  kul validate --no-color family.kul

JSON OUTPUT (--format json):
  Each diagnostic is one JSON object on its own line, with this schema:

  {
    \"code\":     \"KUL-R03\",
    \"severity\": \"error\",
    \"message\":  \"person `alice` is missing required field `name`\",
    \"file\":     \"family.kul\",
    \"primary\":  { \"byte_start\": 7, \"byte_end\": 12,
                    \"line\": 1, \"column\": 8 },
    \"related\":  [ { \"label\": \"prior declaration\",
                      \"byte_start\": …, \"byte_end\": …,
                      \"line\": …, \"column\": … } ]
  }
";

const FORMAT_LONG_ABOUT: &str = "\
Format one or more `.kul` files in canonical form (per ADR 0004).

Without `--check`, each file is rewritten in place. With `--check`, no file
is modified — the command exits non-zero if any input is not already in
canonical form, which is the right shape for a CI gate.

Pass `-` as a filename to read from standard input. In default mode the
formatted source is written to stdout; in `--check` mode the command is
silent on success and prints `<stdin>: not formatted` to stderr if not.

EXAMPLES:
  # Canonicalize a file in place.
  kul format family.kul

  # Canonicalize every example.
  kul format examples/*.kul

  # CI gate: fail if anything is out of canonical form.
  kul format --check examples/*.kul

  # Read from stdin, write canonical form to stdout.
  cat family.kul | kul format -

EXIT CODES:
  0  every file is in canonical form (or was successfully formatted)
  1  at least one input had parse errors, or (under --check) was not in
     canonical form
  2  CLI usage error
";

const EXPORT_LONG_ABOUT: &str = "\
Project a `.kul` document into the canonical JSON envelope.

The export is **strict**: any error-severity diagnostic blocks projection
and the failure envelope (carrying the diagnostics) is written to stdout
with a non-zero exit code. Warnings do not block.

The success envelope shape is:

  {
    \"ok\":     true,
    \"schema\": <integer>,
    \"kul\":   \"<language version>\",
    \"graph\":  {
      \"persons\":           [ ... ],
      \"marriages\":         [ ... ],
      \"parenthood_links\":  [ ... ]
    }
  }

The failure envelope shape is:

  {
    \"ok\":          false,
    \"diagnostics\": [ ... ]  // same schema as `kul validate --format json`
  }

Pass `-` as a filename to read source from standard input. Multiple inputs
write one envelope per line in input order. See
`spec/15-export-schema.md` for the normative schema.

EXAMPLES:
  # Single file.
  kul export family.kul | jq .

  # Read from stdin.
  cat family.kul | kul export -

  # Batch.
  kul export examples/*.kul

EXIT CODES:
  0  every input projected to a success envelope
  1  at least one input failed (errors blocked the projection, or the file
     could not be read)
  2  CLI usage error
";

const LSP_LONG_ABOUT: &str = "\
Run the Kul language server over stdio.

This subcommand is intended for editor integrations: an editor's LSP client
spawns `kul lsp` as a child process and exchanges JSON-RPC messages over
stdin/stdout. Logs go to stderr.

ENVIRONMENT:
  RUST_LOG  Filter directive for tracing logs (e.g. `kul_lsp=debug`).
            Defaults to `kul_lsp=info`.
";

#[derive(Subcommand, Debug)]
enum Command {
    /// Validate one or more `.kul` files. Use `-` to read from stdin.
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
        /// (jsonl). See `kul help validate` for the schema.
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,

        /// Force colorless output even when stderr is a TTY.
        ///
        /// Color is auto-disabled when stderr is not a TTY (e.g. when piped
        /// into a file). This flag forces it off unconditionally.
        #[arg(long)]
        no_color: bool,
    },

    /// Format one or more `.kul` files. Use `-` to read from stdin.
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

    /// Project one or more `.kul` files to the canonical JSON envelope.
    #[command(long_about = EXPORT_LONG_ABOUT)]
    Export {
        /// Files to export. Use `-` to read from standard input.
        #[arg(value_name = "FILE", required = true)]
        files: Vec<PathBuf>,

        /// Output format. `json` (default) is the canonical
        /// kinship-native shape; alternative shapes land additively as
        /// follow-up issues.
        #[arg(long, value_enum, default_value_t = commands::export::CliExportFormat::Json)]
        format: commands::export::CliExportFormat,

        /// Include `span: [byte_start, byte_end]` on every exported
        /// entity. Useful for editor-side tooling that wants to map a
        /// click on a graph node back to its source declaration.
        /// Default off — keeps the envelope compact.
        #[arg(long)]
        with_positions: bool,
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
        Command::Export {
            files,
            format,
            with_positions,
        } => commands::export::run(commands::export::Options {
            files,
            format,
            with_positions,
        }),
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
            eprintln!("kul: failed to start language server runtime: {err}");
            return ExitCode::from(1);
        }
    };
    runtime.block_on(kul_lsp::run());
    ExitCode::SUCCESS
}

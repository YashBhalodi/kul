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
Kul language CLI — parse, validate, format, and export Kul projects.

A Kul *project* is a directory holding a `kul.yml` manifest plus one or
more sibling `.kul` files. `kul validate`, `kul format`, and
`kul export` all operate on the project rooted at the current working
directory — no positional file argument is taken. `kul lsp` runs the
language server over stdio for editor integrations.

EXAMPLES:
  cd examples/08-multi-file-project && kul validate
  cd my-family && kul validate --format json | jq .
  cd my-family && kul format          # canonicalize every .kul in place
  cd my-family && kul format --check  # CI gate: non-zero if anything is dirty
  cd my-family && kul export | jq .   # project graph to JSON
  kul lsp                              # speak LSP over stdio (editor integrations)

EXIT CODES:
  0  project validated/formatted/exported cleanly
  1  one or more files had error diagnostics, the project root was not a
     Kul project (no `kul.yml`), or (under `--check`) some file was not
     in canonical form
  2  CLI usage error (e.g. unknown flag)

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
Validate every `.kul` file in the current Kul project.

The project is the directory containing `kul.yml` — the command is run
from that directory and takes no positional argument. Every `.kul` file
sibling of `kul.yml` is parsed and run through the validator in one
pass; project-wide rules (cross-file duplicate ids, cross-file
references, cross-file cycle detection) fire here.

EXAMPLES:
  # From a project root.
  kul validate

  # Quiet mode for scripts: only diagnostics, no `ok` line.
  kul validate --quiet

  # Machine-readable output (one JSON object per diagnostic, jsonl).
  kul validate --format json

  # Force colorless output (useful in CI logs).
  kul validate --no-color

JSON OUTPUT (--format json):
  Each diagnostic is one JSON object on its own line, with this schema:

  {
    \"code\":     \"KUL-R03\",
    \"severity\": \"error\",
    \"message\":  \"person `alice` is missing required field `name`\",
    \"primary\":  { \"file\": \"alice.kul\",
                    \"byte_start\": 7, \"byte_end\": 12,
                    \"line\": 1, \"column\": 8 },
    \"related\":  [ { \"label\": \"prior declaration\",
                      \"file\": \"bob.kul\",
                      \"byte_start\": …, \"byte_end\": …,
                      \"line\": …, \"column\": … } ]
  }
";

const FORMAT_LONG_ABOUT: &str = "\
Canonicalize every `.kul` file in the current Kul project (per ADR
0004).

The project is the directory containing `kul.yml` — the command is run
from that directory and takes no positional argument. Without
`--check`, each `.kul` file is rewritten in place. With `--check`, no
file is modified — the command exits non-zero if any input is not
already in canonical form, which is the right shape for a CI gate.

EXAMPLES:
  # From a project root.
  kul format

  # CI gate.
  kul format --check

EXIT CODES:
  0  every file in the project is in canonical form (or was
     successfully formatted)
  1  the project had parse errors, or (under `--check`) at least one
     file was not in canonical form
  2  CLI usage error
";

const EXPORT_LONG_ABOUT: &str = "\
Project the current Kul project to the canonical JSON envelope, an
alternative graph JSON, or a self-contained SVG.

The project is the directory containing `kul.yml` — the command is run
from that directory and takes no positional argument. The export is
**strict**: any error-severity diagnostic blocks projection and the
failure envelope (carrying the diagnostics) is written to stdout with
a non-zero exit code. Warnings do not block.

One envelope is written for the whole project, carrying the union of
every file's persons, marriages, and parenthood links.

FORMATS (`--format`):

  json       (default) the canonical kinship-native envelope —
             `persons`, `marriages`, `parenthood_links`. Spec §16.
  cytoscape  Cytoscape JSON (`nodes` + `edges`), loadable into
             Cytoscape.js, Sigma.js, vis-network, etc.
  svg        a self-contained SVG of the canonical visual — the same
             render pipeline the VSCode preview uses, with a neutral
             light theme baked in (inline `<style>`) so the file
             renders standalone in any browser or `<img>`. Streams to
             stdout: `kul export --format=svg > tree.svg`. On a project
             with error-severity diagnostics, nothing is written to
             stdout, the diagnostics render to stderr, and the exit
             code is 1. `--with-positions` does not apply to svg and is
             rejected as a usage error (exit 2).

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

See `spec/16-export-schema.md` for the normative schema.

EXAMPLES:
  kul export | jq .

EXIT CODES:
  0  project projected to a success envelope
  1  errors blocked the projection, or the project could not be
     loaded
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
    /// Validate every `.kul` file in the current Kul project.
    #[command(long_about = VALIDATE_LONG_ABOUT)]
    Validate {
        /// Suppress the `ok` line on success.
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

    /// Format every `.kul` file in the current Kul project.
    #[command(long_about = FORMAT_LONG_ABOUT)]
    Format {
        /// Verify formatting without modifying files. Exits non-zero if any
        /// input is not already in canonical form. Suitable for CI.
        #[arg(long)]
        check: bool,
    },

    /// Project the current Kul project to the canonical JSON envelope.
    #[command(long_about = EXPORT_LONG_ABOUT)]
    Export {
        /// Output format. `json` (default) is the canonical
        /// kinship-native envelope; `cytoscape` is the Cytoscape
        /// node/edge JSON; `svg` is a self-contained SVG of the
        /// canonical visual (`kul export --format=svg > tree.svg`).
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
            quiet,
            format,
            no_color,
        } => commands::validate::run(commands::validate::Options {
            quiet,
            format,
            no_color,
        }),
        Command::Format { check } => commands::format::run(commands::format::Options { check }),
        Command::Export {
            format,
            with_positions,
        } => commands::export::run(commands::export::Options {
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

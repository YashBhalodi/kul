//! `kul export` subcommand — project-wide.
//!
//! Reads every `.kul` file in the current Kul project (CWD must hold
//! a sibling `kul.yml`), runs `check` on the union, and writes one
//! envelope to stdout carrying every person, marriage, and
//! parenthood-link in the project. Errors block: any error-severity
//! diagnostic produces the failure envelope and a non-zero exit.

use std::io::{self, Write};
use std::process::ExitCode;

use kul_core::export::{ExportFormat, ExportOptions, export};
use kul_layout::{LayoutConfig, layout};
use kul_render::{RenderShape, compute};
use kul_svg::{ThemeConfig, render};

use crate::commands::diag;
use crate::commands::project::load_and_check;

#[derive(Copy, Clone, Debug, clap::ValueEnum, PartialEq, Eq)]
pub enum CliExportFormat {
    /// Canonical kinship-native shape — `persons`, `marriages`,
    /// `parenthood_links`. Spec §15.
    Json,
    /// Cytoscape JSON — `nodes` + `edges`, marriage-as-node bipartite
    /// modeling. Loadable into Cytoscape.js, Sigma.js, vis-network, etc.
    Cytoscape,
    /// Self-contained SVG of the canonical visual — the same pipeline
    /// the VSCode preview uses, with a neutral light theme baked in so
    /// the file renders standalone. Streams to stdout (`> tree.svg`).
    Svg,
}

impl From<CliExportFormat> for ExportFormat {
    fn from(value: CliExportFormat) -> Self {
        match value {
            CliExportFormat::Json => ExportFormat::Json,
            CliExportFormat::Cytoscape => ExportFormat::Cytoscape,
            // `Svg` runs the canonical-visual pipeline, not the kinship
            // envelope projection — `run` branches before reaching here.
            CliExportFormat::Svg => {
                unreachable!("svg is rendered via the visual pipeline, not an ExportFormat")
            }
        }
    }
}

pub struct Options {
    pub format: CliExportFormat,
    pub with_positions: bool,
}

pub fn run(opts: Options) -> ExitCode {
    match opts.format {
        CliExportFormat::Svg => run_svg(opts.with_positions),
        CliExportFormat::Json | CliExportFormat::Cytoscape => run_envelope(opts),
    }
}

/// The kinship-native envelope path (`json` / `cytoscape`): project the
/// checked project to an [`ExportEnvelope`](kul_core::export::ExportEnvelope)
/// and write it to stdout.
fn run_envelope(opts: Options) -> ExitCode {
    let (_project, check) = match load_and_check() {
        Ok(x) => x,
        Err(code) => return code,
    };
    let envelope = export(
        &check,
        ExportOptions {
            format: opts.format.into(),
            with_positions: opts.with_positions,
        },
    );
    let json = serde_json::to_string(&envelope).expect("serialize export envelope");
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if let Err(err) = writeln!(out, "{json}") {
        eprintln!("kul: failed to write envelope: {err}");
        return ExitCode::from(1);
    }
    if envelope.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// The canonical-visual path (`svg`): run the same pipeline the LSP's
/// `kul/render` uses (`compute → layout → render`) and stream a
/// self-contained SVG to stdout. `--with-positions` is a JSON-envelope
/// concept (it attaches source spans to exported entities) and is
/// meaningless for SVG, so combining the two is a usage error.
fn run_svg(with_positions: bool) -> ExitCode {
    if with_positions {
        eprintln!(
            "kul: --with-positions is not valid with --format=svg (it applies only to the json/cytoscape envelopes)"
        );
        return ExitCode::from(2);
    }
    let (_project, check) = match load_and_check() {
        Ok(x) => x,
        Err(code) => return code,
    };
    let shape = compute(&check);
    match shape {
        // Error-severity diagnostics block the render: nothing reaches
        // stdout, the diagnostics render to stderr (the same surface
        // `validate` uses), and the exit code is non-zero.
        RenderShape::Failure(_) => {
            diag::render_human(&check, false);
            ExitCode::from(1)
        }
        RenderShape::Success(_) => {
            let positioned = layout(&shape, &LayoutConfig::default());
            let svg = render(&positioned, &ThemeConfig::with_self_contained(true));
            let stdout = io::stdout();
            let mut out = stdout.lock();
            if let Err(err) = writeln!(out, "{svg}") {
                eprintln!("kul: failed to write svg: {err}");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
    }
}

//! `kul export` subcommand — projects the CWD Kul project to a single
//! envelope on stdout. Error-severity diagnostics block and produce the
//! failure envelope with a non-zero exit.

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
    /// Canonical kinship-native shape (spec §15).
    Json,
    /// Cytoscape JSON — `nodes` + `edges`, marriage-as-node bipartite.
    Cytoscape,
    /// Self-contained SVG of the canonical visual, streamed to stdout.
    Svg,
}

impl From<CliExportFormat> for ExportFormat {
    fn from(value: CliExportFormat) -> Self {
        match value {
            CliExportFormat::Json => ExportFormat::Json,
            CliExportFormat::Cytoscape => ExportFormat::Cytoscape,
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

/// `--with-positions` is JSON-envelope-only; combining with SVG is a
/// usage error.
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
        RenderShape::Failure(_) => {
            diag::render_human(&check, false);
            ExitCode::from(1)
        }
        RenderShape::Success(_) => {
            let positioned = layout(&shape, &LayoutConfig::default());
            // Self-contained + legend per ADR-0022; shared with `kul/exportSvg`.
            let svg = render(&positioned, &ThemeConfig::for_file_export());
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

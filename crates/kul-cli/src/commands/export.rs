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

use crate::commands::project::load_and_check;

#[derive(Copy, Clone, Debug, clap::ValueEnum, PartialEq, Eq)]
pub enum CliExportFormat {
    /// Canonical kinship-native shape — `persons`, `marriages`,
    /// `parenthood_links`. Spec §15.
    Json,
    /// Cytoscape JSON — `nodes` + `edges`, marriage-as-node bipartite
    /// modeling. Loadable into Cytoscape.js, Sigma.js, vis-network, etc.
    Cytoscape,
}

impl From<CliExportFormat> for ExportFormat {
    fn from(value: CliExportFormat) -> Self {
        match value {
            CliExportFormat::Json => ExportFormat::Json,
            CliExportFormat::Cytoscape => ExportFormat::Cytoscape,
        }
    }
}

pub struct Options {
    pub format: CliExportFormat,
    pub with_positions: bool,
}

pub fn run(opts: Options) -> ExitCode {
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

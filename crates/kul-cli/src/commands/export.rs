//! `kul export` subcommand.
//!
//! Wraps [`kul_core::export::export`]. Reads each input (file path or `-`
//! for stdin), runs `check`, projects the result into the canonical
//! envelope, and writes the JSON to stdout. Errors block: a document with
//! any error-severity diagnostic prints the failure envelope and exits
//! non-zero.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kul_core::export::{ExportFormat, ExportOptions, export};

use crate::commands::manifest::load_for as load_manifest;

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
    pub files: Vec<PathBuf>,
    pub format: CliExportFormat,
    pub with_positions: bool,
    pub manifest: Option<PathBuf>,
}

pub fn run(opts: Options) -> ExitCode {
    let mut had_error = false;
    for file in &opts.files {
        match export_one(file, &opts) {
            Ok(file_had_error) => {
                if file_had_error {
                    had_error = true;
                }
            }
            Err(err) => {
                eprintln!("kul: {}: {err}", file.display());
                had_error = true;
            }
        }
    }
    if had_error {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn export_one(path: &Path, opts: &Options) -> io::Result<bool> {
    let source = read_input(path)?;
    let label = if path == Path::new("-") {
        "<stdin>".to_string()
    } else {
        path.to_string_lossy().into_owned()
    };
    let manifest = match load_manifest(path, opts.manifest.as_deref()) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("kul: {label}: {err}");
            return Ok(true);
        }
    };
    let check = kul_core::check(&source, &manifest);
    let envelope = export(
        &source,
        &check,
        ExportOptions {
            format: opts.format.into(),
            with_positions: opts.with_positions,
        },
    );
    let json = serde_json::to_string(&envelope).expect("serialize export envelope");
    {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        writeln!(out, "{json}")?;
    }
    Ok(!envelope.is_ok())
}

fn read_input(path: &Path) -> io::Result<String> {
    if path == Path::new("-") {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path)
    }
}

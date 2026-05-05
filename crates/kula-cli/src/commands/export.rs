//! `kula export` subcommand.
//!
//! Wraps [`kula_core::export::export`]. Reads each input (file path or `-`
//! for stdin), runs `check`, projects the result into the canonical
//! envelope, and writes the JSON to stdout. Errors block: a document with
//! any error-severity diagnostic prints the failure envelope and exits
//! non-zero.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kula_core::export::{ExportFormat, ExportOptions, export};

#[derive(Copy, Clone, Debug, clap::ValueEnum, PartialEq, Eq)]
pub enum CliExportFormat {
    Json,
}

impl From<CliExportFormat> for ExportFormat {
    fn from(value: CliExportFormat) -> Self {
        match value {
            CliExportFormat::Json => ExportFormat::Json,
        }
    }
}

pub struct Options {
    pub files: Vec<PathBuf>,
    pub format: CliExportFormat,
    pub with_positions: bool,
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
                eprintln!("kula: {}: {err}", file.display());
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
    let check = kula_core::check(&source);
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

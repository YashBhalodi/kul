//! `kul format` subcommand.
//!
//! Wraps [`kul_core::format::format_source`]. Without `--check`, each file
//! is rewritten in place. With `--check`, nothing is modified and the
//! process exits non-zero if any input is not already in canonical form —
//! the right shape for a CI gate.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kul_core::diagnostic::{Diagnostic, Severity};

use crate::commands::manifest::load_for as load_manifest;

pub struct Options {
    pub files: Vec<PathBuf>,
    pub check: bool,
}

pub fn run(opts: Options) -> ExitCode {
    let mut had_error = false;
    let mut had_diff = false;
    for file in &opts.files {
        match format_one(file, &opts) {
            Outcome::Ok => {}
            Outcome::Diff => had_diff = true,
            Outcome::Error => had_error = true,
        }
    }
    if had_error || had_diff {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

enum Outcome {
    Ok,
    Diff,
    Error,
}

fn format_one(path: &Path, opts: &Options) -> Outcome {
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("kul: {}: {err}", path.display());
            return Outcome::Error;
        }
    };
    let label = path.to_string_lossy().into_owned();
    let manifest = match load_manifest(path) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("kul: {label}: {err}");
            return Outcome::Error;
        }
    };
    let result = kul_core::check(&source, &manifest);
    if has_parse_errors(&result.diagnostics) {
        eprintln!("kul: {label}: cannot format input with parse errors");
        for d in &result.diagnostics {
            if matches!(d.severity, Severity::Error) && is_parse_code(d.code) {
                eprintln!("  {}: {}", d.code, d.message);
            }
        }
        return Outcome::Error;
    }
    let formatted = kul_core::format::format_source(&source);
    if opts.check {
        if formatted != source {
            eprintln!("{label}: not formatted");
            return Outcome::Diff;
        }
        return Outcome::Ok;
    }
    if formatted != source {
        if let Err(err) = std::fs::write(path, &formatted) {
            eprintln!("kul: {}: write: {err}", path.display());
            return Outcome::Error;
        }
    }
    Outcome::Ok
}

fn has_parse_errors(diags: &[Diagnostic]) -> bool {
    diags
        .iter()
        .any(|d| matches!(d.severity, Severity::Error) && is_parse_code(d.code))
}

fn is_parse_code(code: &str) -> bool {
    code.starts_with("KUL-L") || code.starts_with("KUL-P")
}

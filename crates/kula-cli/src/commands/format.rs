//! `kula format` subcommand.
//!
//! Wraps [`kula_core::format::format_source`]. Without `--check`, each file
//! is rewritten in place (and stdin streams to stdout). With `--check`,
//! nothing is modified and the process exits non-zero if any input is not
//! already in canonical form — the right shape for a CI gate.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kula_core::diagnostic::{Diagnostic, Severity};

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
    let is_stdin = path == Path::new("-");
    let (source, label) = match read_input(path) {
        Ok(x) => x,
        Err(err) => {
            eprintln!("kula: {}: {err}", path.display());
            return Outcome::Error;
        }
    };
    let result = kula_core::check(&source);
    if has_parse_errors(&result.diagnostics) {
        eprintln!("kula: {label}: cannot format input with parse errors");
        for d in &result.diagnostics {
            if matches!(d.severity, Severity::Error) && is_parse_code(d.code) {
                eprintln!("  {}: {}", d.code, d.message);
            }
        }
        return Outcome::Error;
    }
    let formatted = kula_core::format::format_source(&source);
    if opts.check {
        if formatted != source {
            eprintln!("{label}: not formatted");
            return Outcome::Diff;
        }
        return Outcome::Ok;
    }
    if is_stdin {
        if let Err(err) = io::stdout().write_all(formatted.as_bytes()) {
            eprintln!("kula: write stdout: {err}");
            return Outcome::Error;
        }
        return Outcome::Ok;
    }
    if formatted != source {
        if let Err(err) = std::fs::write(path, &formatted) {
            eprintln!("kula: {}: write: {err}", path.display());
            return Outcome::Error;
        }
    }
    Outcome::Ok
}

fn read_input(path: &Path) -> io::Result<(String, String)> {
    if path == Path::new("-") {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok((buf, "<stdin>".to_string()))
    } else {
        let s = std::fs::read_to_string(path)?;
        Ok((s, path.to_string_lossy().into_owned()))
    }
}

fn has_parse_errors(diags: &[Diagnostic]) -> bool {
    diags
        .iter()
        .any(|d| matches!(d.severity, Severity::Error) && is_parse_code(d.code))
}

fn is_parse_code(code: &str) -> bool {
    code.starts_with("KULA-L") || code.starts_with("KULA-P")
}

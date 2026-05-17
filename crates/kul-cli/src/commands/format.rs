//! `kul format` subcommand — project-wide.
//!
//! Formats every `.kul` file in the current Kul project (CWD must
//! hold a sibling `kul.yml`). Without `--check`, each file is
//! rewritten in place. With `--check`, no file is modified — the
//! command exits non-zero if any input is not already in canonical
//! form, which is the right shape for a CI gate.

use std::path::PathBuf;
use std::process::ExitCode;

use kul_core::diagnostic::{Diagnostic, Severity};

use crate::commands::project::load_cwd_project;

pub struct Options {
    pub check: bool,
}

pub fn run(opts: Options) -> ExitCode {
    let project = match load_cwd_project() {
        Ok(p) => p,
        Err(err) => return err.report(),
    };
    let cwd = match std::env::current_dir() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("kul: failed to read current working directory: {err}");
            return ExitCode::from(1);
        }
    };

    let result = kul_core::check(
        project.manifest_name,
        &project.manifest_yaml,
        &project.inputs,
    );
    if has_parse_errors(&result.diagnostics) {
        eprintln!("kul: cannot format project with parse errors");
        for d in &result.diagnostics {
            if matches!(d.severity, Severity::Error) && is_parse_code(d.code) {
                eprintln!("  {}: {}", d.code, d.message);
            }
        }
        return ExitCode::from(1);
    }

    let mut had_diff = false;
    let mut had_error = false;
    for input in &project.inputs {
        // Per ADR-0015's flat-directory rule, every project input
        // lives directly under the project root; its on-disk path is
        // `<cwd>/<name>`.
        let path: PathBuf = cwd.join(&input.name);
        let formatted = kul_core::format::format_source(&input.source);
        if formatted == input.source {
            continue;
        }
        if opts.check {
            eprintln!("{}: not formatted", input.name);
            had_diff = true;
            continue;
        }
        if let Err(err) = std::fs::write(&path, &formatted) {
            eprintln!("kul: {}: write: {err}", path.display());
            had_error = true;
        }
    }

    if had_error || had_diff {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn has_parse_errors(diags: &[Diagnostic]) -> bool {
    diags
        .iter()
        .any(|d| matches!(d.severity, Severity::Error) && is_parse_code(d.code))
}

fn is_parse_code(code: &str) -> bool {
    code.starts_with("KUL-L") || code.starts_with("KUL-P")
}

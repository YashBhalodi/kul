//! `kul format` subcommand. Rewrites every `.kul` in the CWD project in
//! place; `--check` makes it a non-mutating CI gate. Parse errors block
//! formatting since the formatter requires a parseable input.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::commands::diag;
use crate::commands::project::load_and_check;

pub struct Options {
    pub check: bool,
}

pub fn run(opts: Options) -> ExitCode {
    let (project, result) = match load_and_check() {
        Ok(x) => x,
        Err(code) => return code,
    };

    if result.diagnostics.iter().any(diag::is_blocking_parse_error) {
        eprintln!("kul: cannot format project with parse errors");
        diag::render_human_matching(&result, false, diag::is_blocking_parse_error);
        return ExitCode::from(1);
    }

    let mut had_diff = false;
    let mut had_error = false;
    for input in &project.inputs {
        // Flat directory per ADR-0015.
        let path: PathBuf = project.root.join(&input.name);
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

//! `kul format` subcommand. Rewrites every `.kul` file of a project in
//! place; `--check` makes it a non-mutating CI gate. Parse errors block
//! formatting since the formatter requires a parseable input.

use std::process::ExitCode;

use crate::commands::diag;
use crate::commands::project::{ProjectArg, load_and_check};

pub struct Options {
    pub check: bool,
    pub project: ProjectArg,
}

pub fn run(opts: Options) -> ExitCode {
    let result = match load_and_check(&opts.project.dir) {
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
    for (_, file) in result.document().kul_files() {
        let formatted = kul_core::format::format_source(&file.source);
        if formatted.as_str() == file.source.as_ref() {
            continue;
        }
        if opts.check {
            eprintln!("{}: not formatted", file.name);
            had_diff = true;
            continue;
        }
        let path = opts.project.dir.join(&file.name);
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

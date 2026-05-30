//! `kul validate` subcommand. Validates every `.kul` in the CWD project
//! in one pass and reports every diagnostic, cross-file or not.

use std::process::ExitCode;

use crate::OutputFormat;
use crate::commands::diag;
use crate::commands::project::load_and_check;

pub struct Options {
    pub quiet: bool,
    pub format: OutputFormat,
    pub no_color: bool,
}

pub fn run(opts: Options) -> ExitCode {
    let (_project, result) = match load_and_check() {
        Ok(x) => x,
        Err(code) => return code,
    };

    match opts.format {
        OutputFormat::Human => diag::render_human(&result, opts.no_color),
        OutputFormat::Json => {
            if let Err(err) = diag::render_json(&result) {
                eprintln!("kul: failed to render diagnostics: {err}");
                return ExitCode::from(1);
            }
        }
    }

    let has_errors = result.has_errors();
    if !has_errors && !opts.quiet && opts.format == OutputFormat::Human {
        println!("ok");
    }
    if has_errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

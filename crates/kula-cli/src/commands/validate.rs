use std::path::Path;
use std::process::ExitCode;

use kula_core::diagnostic::{Diagnostic, RenderableDiagnostic, Severity};
use miette::{GraphicalReportHandler, GraphicalTheme};

pub fn run(files: &[std::path::PathBuf]) -> ExitCode {
    let mut had_error = false;
    for file in files {
        match validate_one(file) {
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

fn validate_one(path: &Path) -> std::io::Result<bool> {
    let source = std::fs::read_to_string(path)?;
    let result = kula_core::check(&source);
    let path_str = path.to_string_lossy();
    render(&source, &path_str, &result.diagnostics);
    Ok(result.has_errors())
}

fn render(source: &str, source_name: &str, diagnostics: &[Diagnostic]) {
    let theme = if std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        GraphicalTheme::unicode()
    } else {
        GraphicalTheme::unicode_nocolor()
    };
    let handler = GraphicalReportHandler::new_themed(theme);
    let mut buf = String::new();
    for diag in diagnostics {
        let renderable = RenderableDiagnostic::new(source, source_name, diag);
        buf.clear();
        let _ = handler.render_report(&mut buf, &renderable);
        eprint!("{buf}");
    }
    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    if errors == 0 {
        println!("{source_name}: ok");
    }
}

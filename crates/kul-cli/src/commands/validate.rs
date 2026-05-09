use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kul_core::diagnostic::{Diagnostic, RelatedSpan, RenderableDiagnostic, Severity};
use kul_core::span::{ByteSpan, SourceMap};
use miette::{GraphicalReportHandler, GraphicalTheme};
use serde::Serialize;

use crate::OutputFormat;
use crate::commands::manifest::load_for as load_manifest;

pub struct Options {
    pub files: Vec<PathBuf>,
    pub quiet: bool,
    pub format: OutputFormat,
    pub no_color: bool,
    pub manifest: Option<PathBuf>,
}

pub fn run(opts: Options) -> ExitCode {
    let mut had_error = false;
    for file in &opts.files {
        match validate_one(file, &opts) {
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

fn validate_one(path: &Path, opts: &Options) -> io::Result<bool> {
    let (source, label) = if path == Path::new("-") {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        (buf, "<stdin>".to_string())
    } else {
        let source = std::fs::read_to_string(path)?;
        (source, path.to_string_lossy().into_owned())
    };

    let manifest = match load_manifest(path, opts.manifest.as_deref()) {
        Ok(m) => m,
        Err(err) => {
            eprintln!("kul: {label}: {err}");
            return Ok(true);
        }
    };

    let result = kul_core::check(&source, &manifest);
    match opts.format {
        OutputFormat::Human => render_human(&source, &label, &result.diagnostics, opts),
        OutputFormat::Json => render_json(&source, &label, &result.diagnostics)?,
    }

    let has_errors = result.has_errors();
    if !has_errors && !opts.quiet && opts.format == OutputFormat::Human {
        println!("{label}: ok");
    }
    Ok(has_errors)
}

fn render_human(source: &str, label: &str, diagnostics: &[Diagnostic], opts: &Options) {
    let theme = if !opts.no_color && std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        GraphicalTheme::unicode()
    } else {
        GraphicalTheme::unicode_nocolor()
    };
    let handler = GraphicalReportHandler::new_themed(theme);
    let mut buf = String::new();
    for diag in diagnostics {
        let renderable = RenderableDiagnostic::new(source, label, diag);
        buf.clear();
        let _ = handler.render_report(&mut buf, &renderable);
        eprint!("{buf}");
    }
}

fn render_json(source: &str, label: &str, diagnostics: &[Diagnostic]) -> io::Result<()> {
    let map = SourceMap::new(source);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    use std::io::Write;
    for diag in diagnostics {
        let record = JsonDiagnostic::new(label, &map, diag);
        let line = serde_json::to_string(&record).expect("serialize diagnostic");
        writeln!(out, "{line}")?;
    }
    Ok(())
}

#[derive(Serialize)]
struct JsonDiagnostic<'a> {
    code: &'a str,
    severity: &'static str,
    message: &'a str,
    file: &'a str,
    primary: JsonSpan,
    related: Vec<JsonRelated<'a>>,
}

#[derive(Serialize)]
struct JsonSpan {
    byte_start: usize,
    byte_end: usize,
    line: usize,
    column: usize,
}

#[derive(Serialize)]
struct JsonRelated<'a> {
    label: &'a str,
    #[serde(flatten)]
    span: JsonSpan,
}

impl<'a> JsonDiagnostic<'a> {
    fn new(file: &'a str, map: &SourceMap, diag: &'a Diagnostic) -> Self {
        Self {
            code: diag.code,
            severity: severity_str(diag.severity),
            message: &diag.message,
            file,
            primary: JsonSpan::new(diag.primary, map),
            related: diag
                .related
                .iter()
                .map(|r: &RelatedSpan| JsonRelated {
                    label: &r.label,
                    span: JsonSpan::new(r.span, map),
                })
                .collect(),
        }
    }
}

impl JsonSpan {
    fn new(span: ByteSpan, map: &SourceMap) -> Self {
        let lc = map.line_col(span.start);
        Self {
            byte_start: span.start,
            byte_end: span.end,
            line: lc.line,
            column: lc.column,
        }
    }
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}

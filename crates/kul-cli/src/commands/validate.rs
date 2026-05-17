//! `kul validate` subcommand — project-wide.
//!
//! Validates every `.kul` file in the current Kul project (CWD must
//! hold a sibling `kul.yml`). Reports every diagnostic — cross-file or
//! not — in one run.

use std::collections::HashMap;
use std::io;
use std::process::ExitCode;

use kul_core::diagnostic::{Diagnostic, RelatedSpan, RenderableDiagnostic, Severity};
use kul_core::span::{FileSpan, SourceMap};
use miette::{GraphicalReportHandler, GraphicalTheme};
use serde::Serialize;

use crate::OutputFormat;
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
        OutputFormat::Human => render_human(&result, &opts),
        OutputFormat::Json => {
            if let Err(err) = render_json(&result) {
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

fn render_human(result: &kul_core::CheckResult, opts: &Options) {
    let theme = if !opts.no_color && std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        GraphicalTheme::unicode()
    } else {
        GraphicalTheme::unicode_nocolor()
    };
    let handler = GraphicalReportHandler::new_themed(theme);
    let mut buf = String::new();
    let document = result.document();
    for diag in &result.diagnostics {
        let renderable = RenderableDiagnostic::for_diagnostic(document, diag);
        buf.clear();
        let _ = handler.render_report(&mut buf, &renderable);
        eprint!("{buf}");
    }
}

fn render_json(result: &kul_core::CheckResult) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    use std::io::Write;

    let document = result.document();
    // Lazily build per-file SourceMaps so we don't pay the cost for
    // files the diagnostic list never anchors into.
    let mut maps: HashMap<kul_core::span::FileId, SourceMap> = HashMap::new();
    for diag in &result.diagnostics {
        let record = JsonDiagnostic::new(document, &mut maps, diag);
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
    #[serde(skip_serializing_if = "Option::is_none")]
    primary: Option<JsonSpan>,
    related: Vec<JsonRelated<'a>>,
}

#[derive(Serialize)]
struct JsonSpan {
    file: String,
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
    fn new(
        document: &kul_core::ast::Document,
        maps: &mut HashMap<kul_core::span::FileId, SourceMap>,
        diag: &'a Diagnostic,
    ) -> Self {
        Self {
            code: diag.code,
            severity: severity_str(diag.severity),
            message: &diag.message,
            primary: diag.primary.and_then(|s| JsonSpan::new(s, document, maps)),
            related: diag
                .related
                .iter()
                .filter_map(|r: &RelatedSpan| {
                    let span = JsonSpan::new(r.span, document, maps)?;
                    Some(JsonRelated {
                        label: &r.label,
                        span,
                    })
                })
                .collect(),
        }
    }
}

impl JsonSpan {
    fn new(
        span: FileSpan,
        document: &kul_core::ast::Document,
        maps: &mut HashMap<kul_core::span::FileId, SourceMap>,
    ) -> Option<Self> {
        let source = document.source_of(span.file)?;
        let map = maps
            .entry(span.file)
            .or_insert_with(|| SourceMap::new(source));
        let lc = map.line_col(span.span.start);
        Some(Self {
            file: document.name_of(span.file).unwrap_or("").to_string(),
            byte_start: span.span.start,
            byte_end: span.span.end,
            line: lc.line,
            column: lc.column,
        })
    }
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}

//! Shared diagnostic rendering (miette terminal + JSONL) for `validate`,
//! `format`, and `export`.

use std::collections::HashMap;
use std::io::{self, Write};

use kul_core::CheckResult;
use kul_core::diagnostic::{Diagnostic, RelatedSpan, RenderableDiagnostic, Severity};
use kul_core::span::{FileId, FileSpan, SourceMap};
use miette::{GraphicalReportHandler, GraphicalTheme};
use serde::Serialize;

/// Render every diagnostic in `result` to stderr in miette's terminal
/// format. Cross-file related-info is surfaced as a `see also` footnote
/// line since miette's single-source layout can't draw it inline.
pub fn render_human(result: &CheckResult, no_color: bool) {
    render_human_matching(result, no_color, |_| true);
}

/// Like [`render_human`] but renders only diagnostics for which `keep`
/// returns true.
pub fn render_human_matching(
    result: &CheckResult,
    no_color: bool,
    keep: impl Fn(&Diagnostic) -> bool,
) {
    let theme = if !no_color && std::io::IsTerminal::is_terminal(&std::io::stderr()) {
        GraphicalTheme::unicode()
    } else {
        GraphicalTheme::unicode_nocolor()
    };
    let handler = GraphicalReportHandler::new_themed(theme);
    let mut buf = String::new();
    let document = result.document();
    let mut maps: HashMap<FileId, SourceMap> = HashMap::new();
    for diag in result.diagnostics.iter().filter(|d| keep(d)) {
        let renderable = RenderableDiagnostic::for_diagnostic(document, diag);
        buf.clear();
        let _ = handler.render_report(&mut buf, &renderable);
        eprint!("{buf}");
        let primary_file = diag.primary.map(|p| p.file);
        for r in &diag.related {
            if primary_file != Some(r.span.file) {
                eprintln!("{}", cross_file_related_line(document, &mut maps, r));
            }
        }
    }
}

fn cross_file_related_line(
    document: &kul_core::ast::Document,
    maps: &mut HashMap<FileId, SourceMap>,
    r: &RelatedSpan,
) -> String {
    let name = document.name_of(r.span.file).unwrap_or("");
    let source = document.source_of(r.span.file).unwrap_or("");
    let map = maps
        .entry(r.span.file)
        .or_insert_with(|| SourceMap::new(source));
    let lc = map.line_col(r.span.span.start);
    format!(
        "  see also: {}:{}:{} — {}",
        name, lc.line, lc.column, r.label
    )
}

/// Render every diagnostic as one JSON object per line on stdout.
/// Schema is documented in `kul help validate`.
pub fn render_json(result: &CheckResult) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let document = result.document();
    let mut maps: HashMap<FileId, SourceMap> = HashMap::new();
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
        maps: &mut HashMap<FileId, SourceMap>,
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
        maps: &mut HashMap<FileId, SourceMap>,
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

/// True for lex/parse diagnostic codes (`KUL-Lxx` / `KUL-Pxx`). These
/// block in-place formatting; semantic-rule errors do not.
pub fn is_parse_code(code: &str) -> bool {
    code.starts_with("KUL-L") || code.starts_with("KUL-P")
}

/// Error-severity lex/parse diagnostics — the formatting blockers.
pub fn is_blocking_parse_error(d: &Diagnostic) -> bool {
    matches!(d.severity, Severity::Error) && is_parse_code(d.code)
}

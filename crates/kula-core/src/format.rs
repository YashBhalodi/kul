//! Canonical formatter for Kula documents.
//!
//! The formatter has two entry points:
//!
//! - [`format`] takes only an AST and produces canonical output. It is the
//!   right call for code-generation tools that build a [`Document`] in
//!   memory and want to print it: there is no source to thread through, and
//!   comments aren't in the AST anyway.
//! - [`format_source`] takes a Kula source string and reformats it. It
//!   parses internally and threads the original bytes through so comments
//!   round-trip per [ADR 0004] rule 7. The CLI and LSP use this entry point.
//!
//! ## Per-block alignment
//!
//! Internally both entry points convert each rendered line to a sequence of
//! [`Cell`]s and accumulate adjacent same-shape lines into a [`BlockBuffer`]
//! that flushes with column padding. "Same shape" means `(indent, sequence
//! of cell kinds)` matches exactly — so adding or removing a field from
//! one statement automatically excludes it from the surrounding block, and
//! the sparse-vs-rectangular question never arises. Boundaries that flush
//! the buffer:
//!
//! - blank line in the source (only relevant to [`format_source`]);
//! - whole-line comment (only relevant to [`format_source`]);
//! - shape change (any kind/field-set change);
//! - indent change (top-level → sub-statement and back).
//!
//! [ADR 0004]: https://github.com/YashBhalodi/kulalang/blob/main/docs/adr/0004-formatter-canonical-rules.md

use std::fmt::Write as _;

use crate::ast::{
    AdoptionFieldKind, AdoptionSub, BirthSub, Document, EndReason, Gender, MarriageFieldKind,
    MarriageStmt, PersonFieldKind, PersonStmt, Statement, VersionDecl,
};
use crate::date::DateLit;
use crate::lexer::FieldName;

// === Public entry points ===

/// Format a parsed `Document` to canonical Kula source.
///
/// The output ends with a trailing newline if the document is non-empty.
/// Comments are not preserved — the AST doesn't model them, so the only
/// caller who should reach for this entry point is code that builds a
/// `Document` in memory (e.g. a code-generation tool). For
/// source-to-source formatting use [`format_source`].
pub fn format(doc: &Document) -> String {
    let mut emitter = Emitter::new();
    if let Some(v) = &doc.version {
        emitter.emit_line(0, build_version_cells(v));
    }
    for stmt in &doc.statements {
        emitter.emit_statement(stmt);
    }
    emitter.finish()
}

/// Format a Kula source string to its canonical form.
///
/// Comments are preserved byte-for-byte per [ADR 0004] rule 7. The function
/// lexes and parses internally; if the parser produces a partial AST
/// (because of recoverable parse errors), this still returns *some* output
/// reflecting what was parseable. Callers that need to reject malformed
/// input should run [`crate::check`] first and bail on parse-error
/// diagnostics.
///
/// [ADR 0004]: https://github.com/YashBhalodi/kulalang/blob/main/docs/adr/0004-formatter-canonical-rules.md
pub fn format_source(source: &str) -> String {
    let result = crate::check(source);
    SourceFormatter::new(source, result.document()).run()
}

// === Cells, shapes, and blocks ===

#[derive(Debug, Clone)]
struct Cell {
    text: String,
    kind: CellKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellKind {
    /// A statement keyword: `kula`, `person`, `marriage`, `birth`, `adoption`.
    Keyword,
    /// A positional id (the bound id of a `person` or `marriage`, or the
    /// version literal). Single space after.
    Positional,
    /// A reference positional in a `marriage` or `birth`/`adoption` line.
    /// Single space after.
    Reference,
    /// A `name:value` field. Two spaces after.
    Field(FieldName),
    /// An inline comment, always the last cell on its line.
    Comment,
}

impl CellKind {
    /// Two cells of these kinds are "same shape" iff this returns true for
    /// each pair at matching positions. Field names participate in the
    /// shape signature so two persons with different field sets don't try
    /// to align with each other.
    fn shape_eq(self, other: CellKind) -> bool {
        self == other
    }
}

/// Buffers a run of consecutive same-shape lines so they can be flushed
/// together with column padding.
struct BlockBuffer {
    indent: usize,
    shape: Vec<CellKind>,
    lines: Vec<Vec<Cell>>,
}

impl BlockBuffer {
    fn empty() -> Self {
        Self {
            indent: 0,
            shape: Vec::new(),
            lines: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    fn would_extend(&self, indent: usize, shape: &[CellKind]) -> bool {
        if self.is_empty() {
            return false;
        }
        if self.indent != indent {
            return false;
        }
        if self.shape.len() != shape.len() {
            return false;
        }
        self.shape
            .iter()
            .zip(shape.iter())
            .all(|(a, b)| a.shape_eq(*b))
    }

    fn push(&mut self, indent: usize, shape: Vec<CellKind>, cells: Vec<Cell>) {
        if self.is_empty() {
            self.indent = indent;
            self.shape = shape;
        }
        self.lines.push(cells);
    }

    fn flush(&mut self, out: &mut String) {
        if self.is_empty() {
            return;
        }
        emit_block(self.indent, &self.lines, out);
        self.lines.clear();
        self.shape.clear();
        self.indent = 0;
    }
}

fn emit_block(indent: usize, lines: &[Vec<Cell>], out: &mut String) {
    debug_assert!(!lines.is_empty(), "emit_block on empty");
    let cols = lines[0].len();
    let mut widths = vec![0usize; cols];
    for line in lines {
        for (i, cell) in line.iter().enumerate() {
            // Use char count rather than byte length — the corpus is ASCII
            // today, but a non-ASCII identifier (e.g. "Élise") should still
            // count as one column position per Unicode scalar. Display
            // width for CJK is a separate problem we punt on for now.
            widths[i] = widths[i].max(cell.text.chars().count());
        }
    }
    for line in lines {
        for _ in 0..indent {
            out.push(' ');
        }
        for (i, cell) in line.iter().enumerate() {
            out.push_str(&cell.text);
            let is_last = i + 1 == line.len();
            if is_last {
                continue;
            }
            let pad = widths[i].saturating_sub(cell.text.chars().count());
            for _ in 0..pad {
                out.push(' ');
            }
            match separator_between(cell.kind, line[i + 1].kind) {
                Sep::Single => out.push(' '),
                Sep::Double => out.push_str("  "),
            }
        }
        out.push('\n');
    }
}

#[derive(Clone, Copy)]
enum Sep {
    Single,
    Double,
}

fn separator_between(prev: CellKind, next: CellKind) -> Sep {
    // Spec rules: single space after a keyword and between positionals;
    // two spaces before any field; two spaces before an inline comment.
    match next {
        CellKind::Field(_) | CellKind::Comment => Sep::Double,
        CellKind::Keyword | CellKind::Positional | CellKind::Reference => match prev {
            CellKind::Keyword | CellKind::Positional | CellKind::Reference => Sep::Single,
            // Field-or-comment → positional shouldn't happen in canonical
            // output (positionals come first). Treat as single space if it
            // ever does so we don't panic.
            CellKind::Field(_) | CellKind::Comment => Sep::Single,
        },
    }
}

// === Generic emitter (used by both entry points) ===

struct Emitter {
    out: String,
    buffer: BlockBuffer,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            buffer: BlockBuffer::empty(),
        }
    }

    fn emit_line(&mut self, indent: usize, cells: Vec<Cell>) {
        let shape: Vec<CellKind> = cells.iter().map(|c| c.kind).collect();
        if !self.buffer.would_extend(indent, &shape) {
            self.buffer.flush(&mut self.out);
        }
        self.buffer.push(indent, shape, cells);
    }

    /// Force the current block to flush even if the next push would extend
    /// it. Used at boundaries that are visible to the user (blank lines,
    /// whole-line comments) but invisible to shape matching.
    fn flush(&mut self) {
        self.buffer.flush(&mut self.out);
    }

    fn finish(mut self) -> String {
        self.buffer.flush(&mut self.out);
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out
    }

    fn emit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Person(p) => self.emit_person(p),
            Statement::Marriage(m) => self.emit_marriage(m),
        }
    }

    fn emit_person(&mut self, p: &PersonStmt) {
        self.emit_line(0, build_person_cells(p, None));
        let subs = collect_sub_refs(p);
        if !subs.is_empty() {
            // The person's header is the last row of the top-level block;
            // a sub-statement at indent=2 forces a flush. After the
            // sub-statements, the next top-level statement starts a fresh
            // block (which may or may not align with statements before
            // this one — alignment requires *adjacency*, and the
            // sub-statements break adjacency).
            self.flush();
            for sub in &subs {
                self.emit_line(2, build_sub_cells(sub, None));
            }
            self.flush();
        }
    }

    fn emit_marriage(&mut self, m: &MarriageStmt) {
        self.emit_line(0, build_marriage_cells(m, None));
    }
}

// === AST → cells ===

fn build_version_cells(v: &VersionDecl) -> Vec<Cell> {
    vec![
        Cell {
            text: "kula".to_string(),
            kind: CellKind::Keyword,
        },
        Cell {
            text: v.version.clone(),
            kind: CellKind::Positional,
        },
    ]
}

fn build_person_cells(p: &PersonStmt, inline_comment: Option<&str>) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(8);
    cells.push(Cell {
        text: "person".to_string(),
        kind: CellKind::Keyword,
    });
    cells.push(Cell {
        text: p.id.name.clone(),
        kind: CellKind::Positional,
    });
    // Spec table order: name, gender, family, given, born, died.
    if let Some(s) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Name(s) => Some(s),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Name, &quote_string(&s.value)));
    }
    if let Some(g) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Gender(g) => Some(g),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Gender, gender_str(g.value)));
    }
    if let Some(s) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Family(s) => Some(s),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Family, &quote_string(&s.value)));
    }
    if let Some(s) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Given(s) => Some(s),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Given, &quote_string(&s.value)));
    }
    if let Some(d) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Born(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Born, &date_str(d)));
    }
    if let Some(d) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Died(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Died, &date_str(d)));
    }
    if let Some(text) = inline_comment {
        cells.push(Cell {
            text: text.to_string(),
            kind: CellKind::Comment,
        });
    }
    cells
}

fn build_marriage_cells(m: &MarriageStmt, inline_comment: Option<&str>) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(7);
    cells.push(Cell {
        text: "marriage".to_string(),
        kind: CellKind::Keyword,
    });
    cells.push(Cell {
        text: m.id.name.clone(),
        kind: CellKind::Positional,
    });
    cells.push(Cell {
        text: m.spouse_a.name.clone(),
        kind: CellKind::Reference,
    });
    cells.push(Cell {
        text: m.spouse_b.name.clone(),
        kind: CellKind::Reference,
    });
    if let Some(d) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::Start(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Start, &date_str(d)));
    }
    if let Some(d) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::End(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::End, &date_str(d)));
    }
    if let Some(er) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::EndReason(v) => Some(v),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::EndReason, &end_reason_str(&er.value)));
    }
    if let Some(text) = inline_comment {
        cells.push(Cell {
            text: text.to_string(),
            kind: CellKind::Comment,
        });
    }
    cells
}

enum SubRef<'a> {
    Birth(&'a BirthSub),
    Adoption(&'a AdoptionSub),
}

impl SubRef<'_> {
    fn span_start(&self) -> usize {
        match self {
            SubRef::Birth(b) => b.span.start,
            SubRef::Adoption(a) => a.span.start,
        }
    }
}

fn collect_sub_refs(p: &PersonStmt) -> Vec<SubRef<'_>> {
    let mut subs: Vec<SubRef> = Vec::new();
    if let Some(b) = &p.birth {
        subs.push(SubRef::Birth(b));
    }
    for a in &p.adoptions {
        subs.push(SubRef::Adoption(a));
    }
    subs.sort_by_key(|s| s.span_start());
    subs
}

fn build_sub_cells(sub: &SubRef<'_>, inline_comment: Option<&str>) -> Vec<Cell> {
    match sub {
        SubRef::Birth(b) => {
            let mut cells = vec![
                Cell {
                    text: "birth".to_string(),
                    kind: CellKind::Keyword,
                },
                Cell {
                    text: b.marriage_ref.name.clone(),
                    kind: CellKind::Reference,
                },
            ];
            if let Some(text) = inline_comment {
                cells.push(Cell {
                    text: text.to_string(),
                    kind: CellKind::Comment,
                });
            }
            cells
        }
        SubRef::Adoption(a) => {
            let mut cells = vec![
                Cell {
                    text: "adoption".to_string(),
                    kind: CellKind::Keyword,
                },
                Cell {
                    text: a.marriage_ref.name.clone(),
                    kind: CellKind::Reference,
                },
            ];
            if let Some(d) = a.fields.iter().find_map(|f| match &f.kind {
                AdoptionFieldKind::Start(d) => Some(d),
                _ => None,
            }) {
                cells.push(field_cell(FieldName::Start, &date_str(d)));
            }
            if let Some(d) = a.fields.iter().find_map(|f| match &f.kind {
                AdoptionFieldKind::End(d) => Some(d),
                _ => None,
            }) {
                cells.push(field_cell(FieldName::End, &date_str(d)));
            }
            if let Some(text) = inline_comment {
                cells.push(Cell {
                    text: text.to_string(),
                    kind: CellKind::Comment,
                });
            }
            cells
        }
    }
}

fn field_cell(name: FieldName, value: &str) -> Cell {
    Cell {
        text: format!("{}:{}", name.as_str(), value),
        kind: CellKind::Field(name),
    }
}

// === Source-level formatter (preserves comments) ===

#[derive(Debug, Clone)]
struct Comment {
    /// 0-indexed source line number.
    line: usize,
    /// `true` iff the line had non-whitespace content before `#`.
    is_inline: bool,
    /// Byte offset of `#`.
    hash_start: usize,
    /// Byte offset just past the comment text — exclusive of `\r` and `\n`.
    end: usize,
}

struct SourceFormatter<'a> {
    source: &'a str,
    doc: &'a Document,
    line_starts: Vec<usize>,
    /// `comment_by_line[L] = index into comments`, or `usize::MAX` if line L
    /// has no comment. At most one comment per source line by construction.
    comment_by_line: Vec<usize>,
    comments: Vec<Comment>,
    emitter: Emitter,
}

impl<'a> SourceFormatter<'a> {
    fn new(source: &'a str, doc: &'a Document) -> Self {
        let line_starts = compute_line_starts(source);
        let comments = scan_comments(source);
        let mut comment_by_line = vec![usize::MAX; line_starts.len()];
        for (idx, c) in comments.iter().enumerate() {
            if c.line < comment_by_line.len() {
                comment_by_line[c.line] = idx;
            }
        }
        Self {
            source,
            doc,
            line_starts,
            comments,
            comment_by_line,
            emitter: Emitter::new(),
        }
    }

    fn run(mut self) -> String {
        let mut cursor_line: usize = 0;
        let mut pending_blank = false;

        if let Some(v) = &self.doc.version {
            let v_line = self.line_of_byte(v.span.start);
            self.flush_loose(cursor_line..v_line, &mut pending_blank);
            self.maybe_blank_separator(&mut pending_blank);
            let inline = self.inline_comment_text(v_line).map(str::to_owned);
            let mut cells = build_version_cells(v);
            if let Some(text) = inline.as_deref() {
                cells.push(Cell {
                    text: text.to_string(),
                    kind: CellKind::Comment,
                });
            }
            self.emitter.emit_line(0, cells);
            cursor_line = self.line_of_byte_end(v.span.end) + 1;
        }

        let stmts: Vec<(usize, usize, &Statement)> = self
            .doc
            .statements
            .iter()
            .map(|s| {
                let span = match s {
                    Statement::Person(p) => p.span,
                    Statement::Marriage(m) => m.span,
                };
                let start_line = self.line_of_byte(span.start);
                let end_line = self.line_of_byte_end(span.end);
                (start_line, end_line, s)
            })
            .collect();

        for (start_line, end_line, stmt) in stmts {
            self.flush_loose(cursor_line..start_line, &mut pending_blank);
            self.maybe_blank_separator(&mut pending_blank);
            self.emit_statement(stmt, start_line);
            cursor_line = end_line + 1;
        }

        self.flush_loose(cursor_line..self.line_count(), &mut pending_blank);
        self.emitter.finish()
    }

    fn emit_statement(&mut self, stmt: &Statement, start_line: usize) {
        match stmt {
            Statement::Person(p) => self.emit_person(p, start_line),
            Statement::Marriage(m) => self.emit_marriage(m, start_line),
        }
    }

    fn emit_person(&mut self, p: &PersonStmt, header_line: usize) {
        let inline = self.inline_comment_text(header_line).map(str::to_owned);
        let cells = build_person_cells(p, inline.as_deref());
        self.emitter.emit_line(0, cells);

        let subs = collect_sub_refs(p);
        if subs.is_empty() {
            return;
        }
        // Top-level block ends here; sub-statements start a new block at
        // indent=2.
        self.emitter.flush();

        let mut sub_cursor = header_line + 1;
        for sub in &subs {
            let sub_line = self.line_of_byte(sub.span_start());
            // Whole-line comments inside the person block break the
            // sub-statement alignment block, just like at top level.
            for line in sub_cursor..sub_line {
                if let Some((is_inline, range)) = self.comment_view(line) {
                    if !is_inline {
                        self.emitter.flush();
                        let text = &self.source[range];
                        let mut s = String::new();
                        for _ in 0..2 {
                            s.push(' ');
                        }
                        s.push_str(text);
                        s.push('\n');
                        self.emitter.out.push_str(&s);
                    }
                }
                // Blank lines inside a person block are removed (ADR rule 6).
            }
            let inline = self.inline_comment_text(sub_line).map(str::to_owned);
            let cells = build_sub_cells(sub, inline.as_deref());
            self.emitter.emit_line(2, cells);
            sub_cursor = sub_line + 1;
        }
        self.emitter.flush();
    }

    fn emit_marriage(&mut self, m: &MarriageStmt, header_line: usize) {
        let inline = self.inline_comment_text(header_line).map(str::to_owned);
        let cells = build_marriage_cells(m, inline.as_deref());
        self.emitter.emit_line(0, cells);
    }

    fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    fn line_of_byte(&self, byte: usize) -> usize {
        match self.line_starts.binary_search(&byte) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        }
    }

    fn line_of_byte_end(&self, end: usize) -> usize {
        if end == 0 {
            return 0;
        }
        self.line_of_byte(end - 1)
    }

    fn comment_for_line(&self, line: usize) -> Option<&Comment> {
        let idx = *self.comment_by_line.get(line)?;
        if idx == usize::MAX {
            None
        } else {
            Some(&self.comments[idx])
        }
    }

    fn comment_view(&self, line: usize) -> Option<(bool, std::ops::Range<usize>)> {
        let c = self.comment_for_line(line)?;
        Some((c.is_inline, c.hash_start..c.end))
    }

    fn inline_comment_text(&self, line: usize) -> Option<&str> {
        let c = self.comment_for_line(line)?;
        if !c.is_inline {
            return None;
        }
        Some(&self.source[c.hash_start..c.end])
    }

    fn line_is_blank(&self, line: usize) -> bool {
        if self.comment_for_line(line).is_some() {
            return false;
        }
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.source.len());
        let raw = &self.source[start..end];
        raw.bytes()
            .all(|b| b == b' ' || b == b'\t' || b == b'\r' || b == b'\n')
    }

    /// Walk `range` of source lines, emitting whole-line comments at column 0
    /// and accumulating blank-line state for collapse.
    fn flush_loose(&mut self, range: std::ops::Range<usize>, pending_blank: &mut bool) {
        for line in range {
            if let Some((is_inline, text_range)) = self.comment_view(line) {
                if is_inline {
                    // The line is part of an emitted statement; the inline
                    // comment is appended where the statement is rendered.
                    continue;
                }
                self.emitter.flush();
                if *pending_blank && !self.emitter.out.is_empty() {
                    self.emitter.out.push('\n');
                }
                *pending_blank = false;
                let text = &self.source[text_range];
                self.emitter.out.push_str(text);
                self.emitter.out.push('\n');
                continue;
            }
            if self.line_is_blank(line) {
                *pending_blank = true;
            }
        }
    }

    fn maybe_blank_separator(&mut self, pending_blank: &mut bool) {
        if *pending_blank {
            // A blank line is a hard block boundary even if the surrounding
            // shapes match — the user's whitespace is the signal.
            self.emitter.flush();
            if !self.emitter.out.is_empty() {
                self.emitter.out.push('\n');
            }
        }
        *pending_blank = false;
    }
}

// === Utilities ===

fn date_str(d: &DateLit) -> String {
    let mut s = String::with_capacity(11);
    if d.circa {
        s.push('~');
    }
    write!(s, "{:04}", d.year).expect("write year");
    if let Some(m) = d.month {
        write!(s, "-{:02}", m).expect("write month");
    }
    if let Some(day) = d.day {
        write!(s, "-{:02}", day).expect("write day");
    }
    s
}

fn quote_string(value: &str) -> String {
    let mut s = String::with_capacity(value.len() + 2);
    s.push('"');
    for c in value.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            other => s.push(other),
        }
    }
    s.push('"');
    s
}

fn gender_str(g: Gender) -> &'static str {
    match g {
        Gender::Male => "male",
        Gender::Female => "female",
        Gender::Other => "other",
    }
}

fn end_reason_str(e: &EndReason) -> String {
    match e {
        EndReason::Divorce => "divorce".to_string(),
        // The validator surfaces unknown end_reason values as KULA-R05b; the
        // formatter still re-emits whatever the user wrote so the diagnostic
        // anchors stay meaningful and the file isn't silently mangled.
        EndReason::Unknown(s) => s.clone(),
    }
}

fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut v = vec![0];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            v.push(i + 1);
        }
    }
    v
}

/// Walk `source` byte-by-byte and collect every comment, skipping `#` that
/// fall inside a string literal. The lexer drops comments entirely, so the
/// formatter has to re-scan the source itself.
fn scan_comments(source: &str) -> Vec<Comment> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line: usize = 0;
    let mut in_string = false;
    let mut line_has_non_ws = false;
    let mut commented_this_line = false;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' || b == b'\r' {
            if b == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                i += 2;
            } else {
                i += 1;
            }
            line += 1;
            line_has_non_ws = false;
            commented_this_line = false;
            // `in_string` carries across lines; the lexer treats raw newlines
            // inside a string as part of its content. Real Kula docs don't
            // exercise this, but we mirror the lexer to keep parity.
            continue;
        }
        if in_string {
            if b == b'\\' && i + 1 < bytes.len() {
                let next = bytes[i + 1];
                if next != b'\n' && next != b'\r' {
                    i += 2;
                    continue;
                }
            }
            if b == b'"' {
                in_string = false;
            }
            line_has_non_ws = true;
            i += 1;
            continue;
        }
        if !commented_this_line && b == b'#' {
            let hash_start = i;
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'\n' && bytes[j] != b'\r' {
                j += 1;
            }
            out.push(Comment {
                line,
                is_inline: line_has_non_ws,
                hash_start,
                end: j,
            });
            commented_this_line = true;
            i = j;
            continue;
        }
        if b == b'"' {
            in_string = true;
            line_has_non_ws = true;
            i += 1;
            continue;
        }
        if b != b' ' && b != b'\t' {
            line_has_non_ws = true;
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::date::DateLit;
    use crate::span::ByteSpan;

    fn s(value: &str) -> String {
        value.to_owned()
    }

    #[test]
    fn date_str_full() {
        let d = DateLit {
            span: ByteSpan::new(0, 0),
            circa: false,
            year: 1925,
            month: Some(3),
            day: Some(10),
        };
        assert_eq!(date_str(&d), "1925-03-10");
    }

    #[test]
    fn date_str_year_only_circa() {
        let d = DateLit {
            span: ByteSpan::new(0, 0),
            circa: true,
            year: 1980,
            month: None,
            day: None,
        };
        assert_eq!(date_str(&d), "~1980");
    }

    #[test]
    fn quote_string_escapes_backslash_and_quote() {
        assert_eq!(quote_string(""), "\"\"");
        assert_eq!(quote_string("Alice"), "\"Alice\"");
        assert_eq!(quote_string("O\"Brien"), "\"O\\\"Brien\"");
        assert_eq!(quote_string("a\\b"), "\"a\\\\b\"");
    }

    #[test]
    fn gender_str_round_trip() {
        assert_eq!(gender_str(Gender::Male), "male");
        assert_eq!(gender_str(Gender::Female), "female");
        assert_eq!(gender_str(Gender::Other), "other");
    }

    #[test]
    fn end_reason_divorce() {
        assert_eq!(end_reason_str(&EndReason::Divorce), "divorce");
        assert_eq!(end_reason_str(&EndReason::Unknown(s("oops"))), "oops");
    }

    #[test]
    fn scan_comments_basic() {
        let src = "person alice  # inline\n# whole\nperson bob\n";
        let cs = scan_comments(src);
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].line, 0);
        assert!(cs[0].is_inline);
        assert_eq!(&src[cs[0].hash_start..cs[0].end], "# inline");
        assert_eq!(cs[1].line, 1);
        assert!(!cs[1].is_inline);
        assert_eq!(&src[cs[1].hash_start..cs[1].end], "# whole");
    }

    #[test]
    fn scan_comments_ignores_hash_in_string() {
        let src = "person alice name:\"# not a comment\"  # but this is\n";
        let cs = scan_comments(src);
        assert_eq!(cs.len(), 1);
        assert_eq!(&src[cs[0].hash_start..cs[0].end], "# but this is");
        assert!(cs[0].is_inline);
    }

    #[test]
    fn format_empty_doc_is_empty_string() {
        let result = crate::check("");
        assert_eq!(format(result.document()), "");
    }

    #[test]
    fn align_two_consecutive_persons_with_same_shape() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bo name:\"Bob\" gender:male\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female\n\
             person bo     name:\"Bob\"    gender:male\n"
        );
    }

    #[test]
    fn shape_change_breaks_block() {
        // alice has born; bob doesn't. Different shapes → no alignment.
        let src = "person alice name:\"Alice\" gender:female born:1950\n\
                   person bo name:\"Bob\" gender:male\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female  born:1950\n\
             person bo  name:\"Bob\"  gender:male\n"
        );
    }

    #[test]
    fn blank_line_breaks_block() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   \n\
                   person bo name:\"Bob\" gender:male\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female\n\
             \n\
             person bo  name:\"Bob\"  gender:male\n"
        );
    }

    #[test]
    fn whole_line_comment_breaks_block() {
        let src = "person alice name:\"Alice\" gender:female\n\
                   # divider\n\
                   person bo name:\"Bob\" gender:male\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female\n\
             # divider\n\
             person bo  name:\"Bob\"  gender:male\n"
        );
    }

    #[test]
    fn marriage_block_aligns_positionals_too() {
        let src = "person a name:\"A\" gender:female\n\
                   person bb name:\"B\" gender:male\n\
                   person cc name:\"C\" gender:male\n\
                   marriage m1 a bb start:1972\n\
                   marriage mm cc a start:1990\n";
        let out = format_source(src);
        // m1/mm align at col 10; spouse_a column at 13 (a/cc, padded so the
        // next column starts at the same offset); spouse_b column at 16
        // (bb/a, padded); start: at 20.
        assert!(
            out.contains("marriage m1 a  bb  start:1972\n"),
            "row 1 missing alignment, got:\n{out}"
        );
        assert!(
            out.contains("marriage mm cc a   start:1990\n"),
            "row 2 missing alignment, got:\n{out}"
        );
    }

    #[test]
    fn sub_statements_align_within_a_person_block() {
        // Two adoptions with same shape: they form an aligned block.
        let src = "person alice name:\"A\" gender:female\n\
                   person bb name:\"B\" gender:male\n\
                   marriage m alice bb start:1972\n\
                   person ravi name:\"Ravi\" gender:male\n\
                   \x20\x20adoption m start:1985\n\
                   \x20\x20adoption m start:1990\n";
        let out = format_source(src);
        assert!(
            out.contains("  adoption m  start:1985\n  adoption m  start:1990\n"),
            "expected aligned adoptions, got:\n{out}"
        );
    }

    #[test]
    fn person_with_substatement_does_not_align_to_neighbors() {
        // alice has a sub-statement; bob doesn't. The sub-statement breaks
        // alice's adjacency with the next top-level person.
        let src = "person alice name:\"A\" gender:female\n\
                   \x20\x20birth m_a\n\
                   person bo name:\"B\" gender:male\n";
        let out = format_source(src);
        // alice's columns are independent of bob's.
        assert_eq!(
            out,
            "person alice  name:\"A\"  gender:female\n\
             \x20\x20birth m_a\n\
             person bo  name:\"B\"  gender:male\n"
        );
    }

    #[test]
    fn inline_comments_align_when_present_on_every_block_line() {
        let src = "person alice name:\"Alice\" gender:female  # alpha\n\
                   person bo name:\"Bob\" gender:male  # beta\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female  # alpha\n\
             person bo     name:\"Bob\"    gender:male    # beta\n"
        );
    }

    #[test]
    fn inline_comment_on_one_row_breaks_block() {
        let src = "person alice name:\"Alice\" gender:female  # alpha\n\
                   person bo name:\"Bob\" gender:male\n";
        let out = format_source(src);
        // alice has an extra cell (the comment); bo does not. Different
        // shapes → independent layouts.
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female  # alpha\n\
             person bo  name:\"Bob\"  gender:male\n"
        );
    }

    #[test]
    fn format_source_idempotent_on_canonical_input() {
        let canonical = "kula 0.1\n\
            \n\
            person alice  name:\"Alice\"  gender:female  born:1950-04-12\n\
            person bo     name:\"Bob\"    gender:male    born:1948-11-30\n\
            \n\
            marriage m_alice_bo alice bo  start:1972-05-12\n";
        let once = format_source(canonical);
        assert_eq!(once, canonical);
        let twice = format_source(&once);
        assert_eq!(twice, once);
    }

    #[test]
    fn format_reorders_person_fields_per_spec() {
        let src = "person alice born:1950 family:\"Sharma\" name:\"Alice\" gender:female\n";
        let formatted = format_source(src);
        assert_eq!(
            formatted,
            "person alice  name:\"Alice\"  gender:female  family:\"Sharma\"  born:1950\n"
        );
    }

    #[test]
    fn format_collapses_blank_runs() {
        let src = "person a name:\"A\" gender:female\n\n\n\nperson b name:\"B\" gender:male\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person a  name:\"A\"  gender:female\n\nperson b  name:\"B\"  gender:male\n"
        );
    }

    #[test]
    fn format_removes_blank_lines_inside_person_block() {
        let src = "person alice name:\"A\" gender:female\n\n  birth m\n";
        let out = format_source(src);
        assert_eq!(out, "person alice  name:\"A\"  gender:female\n  birth m\n");
    }

    #[test]
    fn format_preserves_whole_line_comment_at_column_0() {
        let src = "# header\nperson alice name:\"A\" gender:female\n";
        let out = format_source(src);
        assert_eq!(out, "# header\nperson alice  name:\"A\"  gender:female\n");
    }

    #[test]
    fn format_does_not_treat_hash_in_string_as_comment() {
        let src = "person alice name:\"# bracketed\" gender:female\n";
        let out = format_source(src);
        assert_eq!(out, "person alice  name:\"# bracketed\"  gender:female\n");
    }

    #[test]
    fn format_re_escapes_string_value() {
        let src = "person alice name:\"O\\\"Brien\" gender:female\n";
        let out = format_source(src);
        assert!(out.contains("name:\"O\\\"Brien\""));
    }
}

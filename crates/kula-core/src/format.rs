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
//! The acceptance criteria for the formatter are property-tested in
//! [`tests/format.rs`]: every example in the corpus is idempotent under
//! [`format_source`] and round-trips through `parse → format → parse`
//! with an AST-equal result.
//!
//! [ADR 0004]: https://github.com/YashBhalodi/kulalang/blob/main/docs/adr/0004-formatter-canonical-rules.md

use std::fmt::Write as _;

use crate::ast::{
    AdoptionFieldKind, AdoptionSub, BirthSub, Document, EndReason, Gender, MarriageFieldKind,
    MarriageStmt, PersonFieldKind, PersonStmt, Statement, VersionDecl,
};
use crate::date::DateLit;

/// Canonical sub-statement indent (spec rule: exactly two spaces).
const INDENT: &str = "  ";
/// Canonical separator between fields on a single line (spec rule: exactly
/// two spaces).
const FIELD_SEP: &str = "  ";

// === Public entry points ===

/// Format a parsed `Document` to canonical Kula source.
///
/// The output ends with a trailing newline if the document is non-empty.
/// Comments are not preserved — the AST doesn't model them, so the only
/// caller who should reach for this entry point is code that builds a
/// `Document` in memory (e.g. a code-generation tool). For
/// source-to-source formatting use [`format_source`].
pub fn format(doc: &Document) -> String {
    let mut out = String::new();
    let mut first = true;
    if let Some(v) = &doc.version {
        write_version(&mut out, v);
        out.push('\n');
        first = false;
    }
    for stmt in &doc.statements {
        if !first {
            out.push('\n');
        }
        first = false;
        write_statement_no_comments(&mut out, stmt);
    }
    out
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
    SourceFormatter::new(source, &result.document).run()
}

// === Source-level formatter ===

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
    out: String,
    source: &'a str,
    doc: &'a Document,
    line_starts: Vec<usize>,
    /// `comment_by_line[L] = index into comments`, or `usize::MAX` if line L
    /// has no comment. At most one comment per source line by construction.
    comment_by_line: Vec<usize>,
    comments: Vec<Comment>,
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
            out: String::new(),
            source,
            doc,
            line_starts,
            comments,
            comment_by_line,
        }
    }

    fn run(mut self) -> String {
        let mut cursor_line: usize = 0;
        let mut pending_blank = false;

        if let Some(v) = &self.doc.version {
            let v_line = self.line_of_byte(v.span.start);
            self.flush_loose(cursor_line..v_line, &mut pending_blank);
            self.maybe_blank_separator(&mut pending_blank);
            self.emit_version(v);
            cursor_line = self.line_of_byte_end(v.span.end) + 1;
        }

        // The parser builds Document.statements in source order, so iterating
        // the vector is iterating left-to-right through the file.
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
            self.emit_statement(stmt);
            cursor_line = end_line + 1;
        }

        self.flush_loose(cursor_line..self.line_count(), &mut pending_blank);

        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out
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

    /// Line containing the last byte of a span (one byte before `end`).
    /// Spans use exclusive ends, so `line_of_byte(span.end)` over-counts when
    /// `span.end` lands on a line boundary.
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

    /// Returns `(is_inline, text_byte_range)` for the comment on `line`, or
    /// `None` if there is no comment. Avoids the borrow-checker tangle of
    /// holding `&Comment` while pushing to `self.out`.
    fn comment_view(&self, line: usize) -> Option<(bool, std::ops::Range<usize>)> {
        let c = self.comment_for_line(line)?;
        Some((c.is_inline, c.hash_start..c.end))
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
                if *pending_blank && !self.out.is_empty() {
                    self.out.push('\n');
                }
                *pending_blank = false;
                let text = &self.source[text_range];
                self.out.push_str(text);
                self.out.push('\n');
                continue;
            }
            if self.line_is_blank(line) {
                *pending_blank = true;
            }
        }
    }

    /// Emit a single blank line before the next top-level element if one is
    /// queued. Suppressed at file start so the document never begins with a
    /// blank line.
    fn maybe_blank_separator(&mut self, pending_blank: &mut bool) {
        if *pending_blank && !self.out.is_empty() {
            self.out.push('\n');
        }
        *pending_blank = false;
    }

    fn emit_version(&mut self, v: &VersionDecl) {
        let line = self.line_of_byte(v.span.start);
        write_version(&mut self.out, v);
        self.append_inline_comment(line);
        self.out.push('\n');
    }

    fn emit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Person(p) => self.emit_person(p),
            Statement::Marriage(m) => self.emit_marriage(m),
        }
    }

    fn emit_person(&mut self, p: &PersonStmt) {
        let header_line = self.line_of_byte(p.span.start);
        write_person_signature(&mut self.out, p);
        self.append_inline_comment(header_line);
        self.out.push('\n');

        // Combine birth + adoptions in source order via spans. The AST
        // stores them in two fields, but the original ordering is what the
        // user wrote and ADR 0004 rule 2 (positional → required → optional)
        // only governs *fields within a statement*, not sub-statement order.
        let mut subs: Vec<(usize, SubRef)> = Vec::new();
        if let Some(b) = &p.birth {
            subs.push((b.span.start, SubRef::Birth(b)));
        }
        for a in &p.adoptions {
            subs.push((a.span.start, SubRef::Adoption(a)));
        }
        subs.sort_by_key(|(start, _)| *start);

        let block_last_line = self.line_of_byte_end(p.span.end);
        let mut sub_cursor = header_line + 1;
        for (_, sub) in &subs {
            let sub_line = match sub {
                SubRef::Birth(b) => self.line_of_byte(b.span.start),
                SubRef::Adoption(a) => self.line_of_byte(a.span.start),
            };
            self.emit_block_internal_comments(sub_cursor..sub_line);
            self.emit_sub(sub);
            self.append_inline_comment(sub_line);
            self.out.push('\n');
            sub_cursor = sub_line + 1;
        }
        // No trailing inside-block comments by construction: PersonStmt.span
        // ends at the last sub-statement, so any whole-line comment after the
        // last sub but before the next top-level statement falls in the
        // following `flush_loose` window and emits at column 0.
        let _ = block_last_line;
    }

    fn emit_block_internal_comments(&mut self, range: std::ops::Range<usize>) {
        for line in range {
            if let Some((is_inline, text_range)) = self.comment_view(line) {
                if !is_inline {
                    self.out.push_str(INDENT);
                    let text = &self.source[text_range];
                    self.out.push_str(text);
                    self.out.push('\n');
                }
            }
            // Blank lines inside a person block are removed (ADR rule 6).
        }
    }

    fn emit_sub(&mut self, sub: &SubRef<'_>) {
        self.out.push_str(INDENT);
        match sub {
            SubRef::Birth(b) => write_birth_signature(&mut self.out, b),
            SubRef::Adoption(a) => write_adoption_signature(&mut self.out, a),
        }
    }

    fn emit_marriage(&mut self, m: &MarriageStmt) {
        let line = self.line_of_byte(m.span.start);
        write_marriage_signature(&mut self.out, m);
        self.append_inline_comment(line);
        self.out.push('\n');
    }

    fn append_inline_comment(&mut self, line: usize) {
        if let Some((is_inline, text_range)) = self.comment_view(line) {
            if is_inline {
                self.out.push_str(FIELD_SEP);
                let text = &self.source[text_range];
                self.out.push_str(text);
            }
        }
    }
}

enum SubRef<'a> {
    Birth(&'a BirthSub),
    Adoption(&'a AdoptionSub),
}

// === AST → canonical text (no comments) ===

fn write_statement_no_comments(out: &mut String, stmt: &Statement) {
    match stmt {
        Statement::Person(p) => {
            write_person_signature(out, p);
            out.push('\n');
            let mut subs: Vec<(usize, SubRef)> = Vec::new();
            if let Some(b) = &p.birth {
                subs.push((b.span.start, SubRef::Birth(b)));
            }
            for a in &p.adoptions {
                subs.push((a.span.start, SubRef::Adoption(a)));
            }
            subs.sort_by_key(|(s, _)| *s);
            for (_, sub) in subs {
                out.push_str(INDENT);
                match sub {
                    SubRef::Birth(b) => write_birth_signature(out, b),
                    SubRef::Adoption(a) => write_adoption_signature(out, a),
                }
                out.push('\n');
            }
        }
        Statement::Marriage(m) => {
            write_marriage_signature(out, m);
            out.push('\n');
        }
    }
}

fn write_version(out: &mut String, v: &VersionDecl) {
    out.push_str("kula ");
    out.push_str(&v.version);
}

fn write_person_signature(out: &mut String, p: &PersonStmt) {
    out.push_str("person ");
    out.push_str(&p.id.name);
    for v in canonical_person_fields(p) {
        out.push_str(FIELD_SEP);
        out.push_str(&v);
    }
}

fn write_marriage_signature(out: &mut String, m: &MarriageStmt) {
    out.push_str("marriage ");
    out.push_str(&m.id.name);
    out.push(' ');
    out.push_str(&m.spouse_a.name);
    out.push(' ');
    out.push_str(&m.spouse_b.name);
    for v in canonical_marriage_fields(m) {
        out.push_str(FIELD_SEP);
        out.push_str(&v);
    }
}

fn write_birth_signature(out: &mut String, b: &BirthSub) {
    out.push_str("birth ");
    out.push_str(&b.marriage_ref.name);
}

fn write_adoption_signature(out: &mut String, a: &AdoptionSub) {
    out.push_str("adoption ");
    out.push_str(&a.marriage_ref.name);
    for v in canonical_adoption_fields(a) {
        out.push_str(FIELD_SEP);
        out.push_str(&v);
    }
}

fn canonical_person_fields(p: &PersonStmt) -> Vec<String> {
    // Spec table order: name, gender, family, given, born, died.
    let mut out = Vec::new();
    if let Some(name) = first_field(p, |f| matches!(&f.kind, PersonFieldKind::Name(_))) {
        if let PersonFieldKind::Name(s) = &name.kind {
            out.push(format!("name:{}", quote_string(&s.value)));
        }
    }
    if let Some(g) = first_field(p, |f| matches!(&f.kind, PersonFieldKind::Gender(_))) {
        if let PersonFieldKind::Gender(v) = &g.kind {
            out.push(format!("gender:{}", gender_str(v.value)));
        }
    }
    if let Some(f) = first_field(p, |f| matches!(&f.kind, PersonFieldKind::Family(_))) {
        if let PersonFieldKind::Family(s) = &f.kind {
            out.push(format!("family:{}", quote_string(&s.value)));
        }
    }
    if let Some(f) = first_field(p, |f| matches!(&f.kind, PersonFieldKind::Given(_))) {
        if let PersonFieldKind::Given(s) = &f.kind {
            out.push(format!("given:{}", quote_string(&s.value)));
        }
    }
    if let Some(f) = first_field(p, |f| matches!(&f.kind, PersonFieldKind::Born(_))) {
        if let PersonFieldKind::Born(d) = &f.kind {
            out.push(format!("born:{}", date_str(d)));
        }
    }
    if let Some(f) = first_field(p, |f| matches!(&f.kind, PersonFieldKind::Died(_))) {
        if let PersonFieldKind::Died(d) = &f.kind {
            out.push(format!("died:{}", date_str(d)));
        }
    }
    out
}

fn first_field(
    p: &PersonStmt,
    pred: impl Fn(&crate::ast::PersonField) -> bool,
) -> Option<&crate::ast::PersonField> {
    p.fields.iter().find(|f| pred(f))
}

fn canonical_marriage_fields(m: &MarriageStmt) -> Vec<String> {
    // Spec table order: start, end, end_reason.
    let mut out = Vec::new();
    if let Some(start) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::Start(d) => Some(d),
        _ => None,
    }) {
        out.push(format!("start:{}", date_str(start)));
    }
    if let Some(end) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::End(d) => Some(d),
        _ => None,
    }) {
        out.push(format!("end:{}", date_str(end)));
    }
    if let Some(er) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::EndReason(v) => Some(v),
        _ => None,
    }) {
        out.push(format!("end_reason:{}", end_reason_str(&er.value)));
    }
    out
}

fn canonical_adoption_fields(a: &AdoptionSub) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(d) = a.fields.iter().find_map(|f| match &f.kind {
        AdoptionFieldKind::Start(d) => Some(d),
        _ => None,
    }) {
        out.push(format!("start:{}", date_str(d)));
    }
    if let Some(d) = a.fields.iter().find_map(|f| match &f.kind {
        AdoptionFieldKind::End(d) => Some(d),
        _ => None,
    }) {
        out.push(format!("end:{}", date_str(d)));
    }
    out
}

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

// === Helpers ===

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
        assert_eq!(format(&result.document), "");
    }

    #[test]
    fn format_source_idempotent_on_canonical_input() {
        let canonical = "kula 0.1\n\
            \n\
            person alice  name:\"Alice\"  gender:female  born:1950-04-12\n\
            person bob  name:\"Bob\"  gender:male  born:1948-11-30\n\
            \n\
            marriage m_alice_bob alice bob  start:1972-05-12\n";
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
    fn format_reorders_marriage_fields_per_spec() {
        let src = "person a name:\"A\" gender:female\n\
                   person b name:\"B\" gender:male\n\
                   marriage m a b end_reason:divorce end:1990 start:1972\n";
        let formatted = format_source(src);
        assert!(formatted.contains("marriage m a b  start:1972  end:1990  end_reason:divorce\n"));
    }

    #[test]
    fn format_two_space_separators_between_fields() {
        let src = "person alice name:\"A\" gender:female\n";
        let out = format_source(src);
        assert!(out.contains("alice  name:\"A\"  gender:female"));
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
        // No marriage `m`, but the formatter doesn't care — it operates on
        // structure, not semantics.
        let out = format_source(src);
        assert_eq!(out, "person alice  name:\"A\"  gender:female\n  birth m\n");
    }

    #[test]
    fn format_preserves_inline_comment_with_two_space_gap() {
        let src = "person alice name:\"A\" gender:female  # alpha\n";
        let out = format_source(src);
        assert_eq!(out, "person alice  name:\"A\"  gender:female  # alpha\n");
    }

    #[test]
    fn format_preserves_whole_line_comment_at_column_0() {
        let src = "# header\nperson alice name:\"A\" gender:female\n";
        let out = format_source(src);
        assert_eq!(out, "# header\nperson alice  name:\"A\"  gender:female\n");
    }

    #[test]
    fn format_preserves_indented_comment_inside_person_block() {
        let src = "person alice name:\"A\" gender:female\n\
                   \x20\x20# adopted from registry\n\
                   \x20\x20birth m\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"A\"  gender:female\n  # adopted from registry\n  birth m\n"
        );
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

    #[test]
    fn format_canonicalizes_dates() {
        let src = "person alice name:\"A\" gender:female born:~1950\n";
        let out = format_source(src);
        assert!(out.contains("born:~1950"));
    }
}

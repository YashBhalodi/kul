//! Source-pass formatter: reformats a `.kul` source string by routing the
//! parsed AST through the [`super::emit::Emitter`] while keeping comments
//! threaded through at their original lines (per ADR-0004 rule 7).
//!
//! The lexer drops comments entirely, so this module has to re-scan the
//! source byte-by-byte to know where they live. Only the `format_source`
//! entry point in [`super`] uses this; the AST-only `format` entry point
//! stays comment-free.

use crate::ast::{Document, MarriageStmt, PersonStmt, Statement};

use super::cells::{
    KindTag, build_marriage_cells, build_person_cells, build_sub_cells, collect_sub_refs,
};
use super::emit::Emitter;

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

pub(super) struct SourceFormatter<'a> {
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
    pub(super) fn new(source: &'a str, doc: &'a Document) -> Self {
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

    pub(super) fn run(mut self) -> String {
        let mut cursor_line: usize = 0;
        let mut pending_blank = false;

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
            self.queue_loose_lines(cursor_line..start_line, &mut pending_blank);
            self.close_region_if_pending(&mut pending_blank);
            self.emit_statement(stmt, start_line);
            cursor_line = end_line + 1;
        }

        self.queue_loose_lines(cursor_line..self.line_count(), &mut pending_blank);
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
        self.emitter.emit_top_level(0, KindTag::Person, cells);

        let subs = collect_sub_refs(p);
        if subs.is_empty() {
            return;
        }
        let parent = self.emitter.allocate_parent_id();

        let mut sub_cursor = header_line + 1;
        for sub in &subs {
            let sub_line = self.line_of_byte(sub.span_start());
            // Whole-line comments inside the person block ride along in the
            // region buffer at indent 2 (per spec §14.7). They don't break
            // sub-statement alignment — same-keyword subs under the same
            // parent still join one group.
            for line in sub_cursor..sub_line {
                if let Some((is_inline, range)) = self.comment_view(line) {
                    if !is_inline {
                        let text = self.source[range].to_string();
                        self.emitter.emit_comment(2, text);
                    }
                }
                // Blank lines inside a person block are removed (ADR rule 6).
            }
            let inline = self.inline_comment_text(sub_line).map(str::to_owned);
            let (kind, cells) = build_sub_cells(sub, inline.as_deref());
            self.emitter.emit_sub(parent, 2, kind, cells);
            sub_cursor = sub_line + 1;
        }
    }

    fn emit_marriage(&mut self, m: &MarriageStmt, header_line: usize) {
        let inline = self.inline_comment_text(header_line).map(str::to_owned);
        let cells = build_marriage_cells(m, inline.as_deref());
        self.emitter.emit_top_level(0, KindTag::Marriage, cells);
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

    /// Walk `range` of source lines between two top-level statements: queue
    /// any whole-line comments into the current region (or close the region
    /// first if a blank line appeared) and remember whether a blank line was
    /// seen so the caller can emit a separator before the next statement.
    fn queue_loose_lines(&mut self, range: std::ops::Range<usize>, pending_blank: &mut bool) {
        for line in range {
            if let Some((is_inline, text_range)) = self.comment_view(line) {
                if is_inline {
                    // The line is part of an emitted statement; the inline
                    // comment is appended where the statement is rendered.
                    continue;
                }
                if *pending_blank {
                    self.close_region(true);
                    *pending_blank = false;
                }
                let text = self.source[text_range].to_string();
                self.emitter.emit_comment(0, text);
                continue;
            }
            if self.line_is_blank(line) {
                *pending_blank = true;
            }
        }
    }

    fn close_region_if_pending(&mut self, pending_blank: &mut bool) {
        if *pending_blank {
            self.close_region(true);
            *pending_blank = false;
        }
    }

    /// End the current region, optionally emitting a blank-line separator
    /// before the next region begins. The blank line is suppressed when the
    /// output is empty so the file never starts with one (ADR rule 6).
    fn close_region(&mut self, emit_blank: bool) {
        self.emitter.end_region();
        if emit_blank {
            self.emitter.append_separator();
        }
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
            // inside a string as part of its content. Real Kul docs don't
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
}

//! Canonical formatter for Kul documents.
//!
//! The formatter has two entry points:
//!
//! - [`format`] takes only an AST and produces canonical output. It is the
//!   right call for code-generation tools that build a [`Document`] in
//!   memory and want to print it: there is no source to thread through, and
//!   comments aren't in the AST anyway.
//! - [`format_source`] takes a Kul source string and reformats it. It
//!   parses internally and threads the original bytes through so comments
//!   round-trip per [ADR 0004] rule 7. The CLI and LSP use this entry point.
//!
//! ## Per-region sparse alignment
//!
//! Both entry points convert each rendered line to a sequence of [`Cell`]s
//! and queue it into a region buffer. A *region* is the run of lines between
//! two blank lines (or document start/end); the blank line is the only
//! region boundary.
//!
//! When a region flushes, lines are bucketed into *alignment groups* by a
//! key that captures who they share columns with:
//!
//! - top-level lines: `(indent, kind)`;
//! - sub-statements (`birth`, `adoption`): `(indent, kind, parent_person_id)`,
//!   so two sub-statements under different persons never share a group even
//!   when they're in the same region.
//!
//! Each statement kind carries a fixed canonical column ordering (matching
//! the field-order spec §14.2). Each cell is tagged with its column index in
//! that ordering when built. A column is *present* in a group iff at least
//! one line in the group has a cell at that index; the column's width is the
//! max content width across lines that carry it. When emitting a line, the
//! formatter walks the canonical column sequence in order, padding actual
//! cells, emitting whitespace placeholders for columns the line lacks but
//! the group has, and stopping at the line's last actual cell — no trailing
//! whitespace is added through subsequent column slots. Shape no longer
//! participates in grouping; same-keyword lines share columns regardless of
//! which optional fields each carries.
//!
//! [ADR 0004]: https://github.com/YashBhalodi/kul/blob/main/docs/adr/0004-formatter-canonical-rules.md

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::ast::{
    AdoptionFieldKind, AdoptionSub, BirthSub, Document, EndReason, Gender, MarriageFieldKind,
    MarriageStmt, PersonFieldKind, PersonStmt, Statement, VersionDecl,
};
use crate::date::DateLit;
use crate::lexer::FieldName;

// === Public entry points ===

/// Format a parsed `Document` to canonical Kul source.
///
/// The output ends with a trailing newline if the document is non-empty.
/// Comments are not preserved — the AST doesn't model them, so the only
/// caller who should reach for this entry point is code that builds a
/// `Document` in memory (e.g. a code-generation tool). For
/// source-to-source formatting use [`format_source`].
pub fn format(doc: &Document) -> String {
    let mut emitter = Emitter::new();
    if let Some(v) = &doc.version {
        emitter.emit_top_level(0, KindTag::Version, build_version_cells(v));
    }
    for stmt in &doc.statements {
        emitter.emit_statement(stmt);
    }
    emitter.finish()
}

/// Format a Kul source string to its canonical form.
///
/// Comments are preserved byte-for-byte per [ADR 0004] rule 7. The function
/// lexes and parses internally; if the parser produces a partial AST
/// (because of recoverable parse errors), this still returns *some* output
/// reflecting what was parseable. Callers that need to reject malformed
/// input should run [`crate::check`] first and bail on parse-error
/// diagnostics.
///
/// [ADR 0004]: https://github.com/YashBhalodi/kul/blob/main/docs/adr/0004-formatter-canonical-rules.md
pub fn format_source(source: &str) -> String {
    let result = crate::check(source);
    SourceFormatter::new(source, result.document()).run()
}

// === Cells, kinds, and groups ===

#[derive(Debug, Clone)]
struct Cell {
    text: String,
    /// Index of this cell in the canonical column ordering for its statement
    /// kind (see [`canonical_columns`]). Each cell builder assigns this when
    /// constructing the line so the alignment pass can place cells in their
    /// canonical column regardless of which optional fields are present. The
    /// cell's `CellKind` is therefore implicit — `canonical_columns(kind)[col]`
    /// is the single source of truth.
    col: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CellKind {
    /// A statement keyword: `kul`, `person`, `marriage`, `birth`, `adoption`.
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

/// Identifies the statement kind of a line. Drives both the canonical column
/// ordering and the alignment-group key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KindTag {
    Version,
    Person,
    Marriage,
    Birth,
    Adoption,
}

/// Canonical column kinds for a statement kind, in the order they appear on
/// a line. The column at index `i` always has the same `CellKind` for every
/// line in a group; lines may *omit* optional columns (everything past the
/// required structural prefix), in which case the formatter emits whitespace
/// of that column's width if the line has any further cells to its right.
fn canonical_columns(kind: KindTag) -> &'static [CellKind] {
    use CellKind::*;
    use FieldName as F;
    match kind {
        KindTag::Version => &[Keyword, Positional, Comment],
        KindTag::Person => &[
            Keyword,
            Positional,
            Field(F::Name),
            Field(F::Gender),
            Field(F::Family),
            Field(F::Given),
            Field(F::Born),
            Field(F::Died),
            Comment,
        ],
        KindTag::Marriage => &[
            Keyword,
            Positional,
            Reference,
            Reference,
            Field(F::Start),
            Field(F::End),
            Field(F::EndReason),
            Comment,
        ],
        KindTag::Birth => &[Keyword, Reference, Comment],
        KindTag::Adoption => &[Keyword, Reference, Field(F::Start), Field(F::End), Comment],
    }
}

/// Identifies which lines share columns with each other within a region.
///
/// Two lines align iff their `GroupKey`s are equal. `parent` is `None` for
/// top-level statements (region-scoped) and `Some(id)` for sub-statements
/// (parent-scoped); two sub-statements under different persons never share
/// a group even with identical `(indent, kind)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GroupKey {
    indent: usize,
    kind: KindTag,
    parent: Option<u32>,
}

#[derive(Debug, Clone)]
enum RegionItem {
    /// A statement (or version) line that participates in an alignment group.
    Aligned {
        indent: usize,
        cells: Vec<Cell>,
        group: GroupKey,
    },
    /// A whole-line comment with its already-resolved indent and the raw
    /// `#…` text. Comments never participate in alignment.
    Comment { indent: usize, text: String },
}

// === Emitter (used by both entry points) ===

struct Emitter {
    out: String,
    region: Vec<RegionItem>,
    /// Monotonic id used to scope each `person`'s sub-statements. Reset at
    /// every region flush so ids don't grow unbounded across a document.
    next_parent_id: u32,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            region: Vec::new(),
            next_parent_id: 0,
        }
    }

    fn allocate_parent_id(&mut self) -> u32 {
        let id = self.next_parent_id;
        self.next_parent_id += 1;
        id
    }

    fn emit_top_level(&mut self, indent: usize, kind: KindTag, cells: Vec<Cell>) {
        let group = GroupKey {
            indent,
            kind,
            parent: None,
        };
        self.region.push(RegionItem::Aligned {
            indent,
            cells,
            group,
        });
    }

    fn emit_sub(&mut self, parent_id: u32, indent: usize, kind: KindTag, cells: Vec<Cell>) {
        let group = GroupKey {
            indent,
            kind,
            parent: Some(parent_id),
        };
        self.region.push(RegionItem::Aligned {
            indent,
            cells,
            group,
        });
    }

    fn emit_comment(&mut self, indent: usize, text: String) {
        self.region.push(RegionItem::Comment { indent, text });
    }

    /// Compute per-group, per-column widths and emit the buffered region.
    /// After this returns, the buffer is empty and the parent-id counter
    /// resets, so each region's sub-statement scoping is independent.
    ///
    /// Widths are stored per group as `Vec<Option<usize>>` indexed by the
    /// canonical column index of the group's kind; `None` marks a column
    /// that no line in the group carries (and therefore is *not present* in
    /// the rendered layout — the renderer emits no placeholder for it).
    fn end_region(&mut self) {
        if self.region.is_empty() {
            return;
        }
        let mut widths: HashMap<GroupKey, Vec<Option<usize>>> = HashMap::new();
        for item in &self.region {
            if let RegionItem::Aligned { cells, group, .. } = item {
                let canonical = canonical_columns(group.kind);
                let entry = widths
                    .entry(*group)
                    .or_insert_with(|| vec![None; canonical.len()]);
                for cell in cells {
                    let slot = &mut entry[cell.col as usize];
                    let w = cell.text.chars().count();
                    *slot = Some(slot.unwrap_or(0).max(w));
                }
            }
        }
        let items = std::mem::take(&mut self.region);
        for item in items {
            match item {
                RegionItem::Aligned {
                    indent,
                    cells,
                    group,
                } => {
                    let cols = widths.get(&group).expect("group widths");
                    emit_aligned_line(indent, group.kind, &cells, cols, &mut self.out);
                }
                RegionItem::Comment { indent, text } => {
                    for _ in 0..indent {
                        self.out.push(' ');
                    }
                    self.out.push_str(&text);
                    self.out.push('\n');
                }
            }
        }
        self.next_parent_id = 0;
    }

    fn finish(mut self) -> String {
        self.end_region();
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
        self.emit_top_level(0, KindTag::Person, build_person_cells(p, None));
        let subs = collect_sub_refs(p);
        if subs.is_empty() {
            return;
        }
        let parent = self.allocate_parent_id();
        for sub in &subs {
            let (kind, cells) = build_sub_cells(sub, None);
            self.emit_sub(parent, 2, kind, cells);
        }
    }

    fn emit_marriage(&mut self, m: &MarriageStmt) {
        self.emit_top_level(0, KindTag::Marriage, build_marriage_cells(m, None));
    }
}

/// Render one line of an alignment group.
///
/// Walks the canonical column sequence for `kind` left-to-right. For each
/// column that is *present in the group* (i.e. has a `Some` entry in
/// `widths`), the line either:
///
/// - emits the cell at that column padded to the column width (if the line
///   carries the cell and it is not the line's last cell), or
/// - emits the cell unpadded (if it is the line's last cell), or
/// - emits whitespace of the column's width (if the line lacks the cell but
///   has further actual cells to its right).
///
/// After the line's last actual cell the renderer stops — no trailing
/// whitespace is emitted through subsequent column slots, because trailing
/// whitespace would corrupt idempotence on editors that strip it.
///
/// Inter-column separators are determined by the two adjacent columns'
/// canonical `CellKind`s (single space after a keyword or between
/// positionals/references; two spaces before fields/comments). The
/// separator is independent of which cells the current line carries.
fn emit_aligned_line(
    indent: usize,
    kind: KindTag,
    cells: &[Cell],
    widths: &[Option<usize>],
    out: &mut String,
) {
    let canonical = canonical_columns(kind);
    debug_assert_eq!(canonical.len(), widths.len());
    for _ in 0..indent {
        out.push(' ');
    }

    // The line's last actual cell — beyond this column, the renderer stops.
    let last_col = cells
        .last()
        .expect("a line always has at least the keyword cell")
        .col;

    let mut prev_col: Option<u8> = None;
    for (col_idx, slot_width) in widths.iter().enumerate() {
        let col_idx = col_idx as u8;
        if col_idx > last_col {
            break;
        }
        let Some(width) = *slot_width else {
            continue;
        };
        if let Some(prev) = prev_col {
            match separator_between(canonical[prev as usize], canonical[col_idx as usize]) {
                Sep::Single => out.push(' '),
                Sep::Double => out.push_str("  "),
            }
        }
        // Use char count rather than byte length — the corpus is ASCII
        // today, but a non-ASCII identifier (e.g. "Élise") should still
        // count as one column position per Unicode scalar. Display width
        // for CJK is a separate problem we punt on for now.
        if let Some(cell) = cells.iter().find(|c| c.col == col_idx) {
            out.push_str(&cell.text);
            if col_idx != last_col {
                let pad = width.saturating_sub(cell.text.chars().count());
                for _ in 0..pad {
                    out.push(' ');
                }
            }
        } else {
            for _ in 0..width {
                out.push(' ');
            }
        }
        prev_col = Some(col_idx);
    }
    out.push('\n');
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

// === AST → cells ===

fn build_version_cells(v: &VersionDecl) -> Vec<Cell> {
    vec![
        Cell {
            text: "kul".to_string(),
            col: 0,
        },
        Cell {
            text: v.version.clone(),
            col: 1,
        },
    ]
}

fn build_person_cells(p: &PersonStmt, inline_comment: Option<&str>) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(9);
    cells.push(Cell {
        text: "person".to_string(),
        col: 0,
    });
    cells.push(Cell {
        text: p.id.name.clone(),
        col: 1,
    });
    // Spec table order matches the canonical column layout for `person`:
    // col 2 = name, 3 = gender, 4 = family, 5 = given, 6 = born, 7 = died,
    // 8 = comment.
    if let Some(s) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Name(s) => Some(s),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Name, &quote_string(&s.value), 2));
    }
    if let Some(g) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Gender(g) => Some(g),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Gender, gender_str(g.value), 3));
    }
    if let Some(s) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Family(s) => Some(s),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Family, &quote_string(&s.value), 4));
    }
    if let Some(s) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Given(s) => Some(s),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Given, &quote_string(&s.value), 5));
    }
    if let Some(d) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Born(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Born, &date_str(d), 6));
    }
    if let Some(d) = p.fields.iter().find_map(|f| match &f.kind {
        PersonFieldKind::Died(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Died, &date_str(d), 7));
    }
    if let Some(text) = inline_comment {
        cells.push(Cell {
            text: text.to_string(),
            col: 8,
        });
    }
    cells
}

fn build_marriage_cells(m: &MarriageStmt, inline_comment: Option<&str>) -> Vec<Cell> {
    let mut cells = Vec::with_capacity(8);
    cells.push(Cell {
        text: "marriage".to_string(),
        col: 0,
    });
    cells.push(Cell {
        text: m.id.name.clone(),
        col: 1,
    });
    cells.push(Cell {
        text: m.spouse_a.name.clone(),
        col: 2,
    });
    cells.push(Cell {
        text: m.spouse_b.name.clone(),
        col: 3,
    });
    if let Some(d) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::Start(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::Start, &date_str(d), 4));
    }
    if let Some(d) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::End(d) => Some(d),
        _ => None,
    }) {
        cells.push(field_cell(FieldName::End, &date_str(d), 5));
    }
    if let Some(er) = m.fields.iter().find_map(|f| match &f.kind {
        MarriageFieldKind::EndReason(v) => Some(v),
        _ => None,
    }) {
        cells.push(field_cell(
            FieldName::EndReason,
            &end_reason_str(&er.value),
            6,
        ));
    }
    if let Some(text) = inline_comment {
        cells.push(Cell {
            text: text.to_string(),
            col: 7,
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

fn build_sub_cells(sub: &SubRef<'_>, inline_comment: Option<&str>) -> (KindTag, Vec<Cell>) {
    match sub {
        SubRef::Birth(b) => {
            let mut cells = vec![
                Cell {
                    text: "birth".to_string(),
                    col: 0,
                },
                Cell {
                    text: b.marriage_ref.name.clone(),
                    col: 1,
                },
            ];
            if let Some(text) = inline_comment {
                cells.push(Cell {
                    text: text.to_string(),
                    col: 2,
                });
            }
            (KindTag::Birth, cells)
        }
        SubRef::Adoption(a) => {
            let mut cells = vec![
                Cell {
                    text: "adoption".to_string(),
                    col: 0,
                },
                Cell {
                    text: a.marriage_ref.name.clone(),
                    col: 1,
                },
            ];
            if let Some(d) = a.fields.iter().find_map(|f| match &f.kind {
                AdoptionFieldKind::Start(d) => Some(d),
                _ => None,
            }) {
                cells.push(field_cell(FieldName::Start, &date_str(d), 2));
            }
            if let Some(d) = a.fields.iter().find_map(|f| match &f.kind {
                AdoptionFieldKind::End(d) => Some(d),
                _ => None,
            }) {
                cells.push(field_cell(FieldName::End, &date_str(d), 3));
            }
            if let Some(text) = inline_comment {
                cells.push(Cell {
                    text: text.to_string(),
                    col: 4,
                });
            }
            (KindTag::Adoption, cells)
        }
    }
}

fn field_cell(name: FieldName, value: &str, col: u8) -> Cell {
    Cell {
        text: format!("{}:{}", name.as_str(), value),
        col,
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
            self.queue_loose_lines(cursor_line..v_line, &mut pending_blank);
            self.close_region_if_pending(&mut pending_blank);
            let inline = self.inline_comment_text(v_line).map(str::to_owned);
            let mut cells = build_version_cells(v);
            if let Some(text) = inline.as_deref() {
                cells.push(Cell {
                    text: text.to_string(),
                    col: 2,
                });
            }
            self.emitter.emit_top_level(0, KindTag::Version, cells);
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
        if emit_blank && !self.emitter.out.is_empty() {
            self.emitter.out.push('\n');
        }
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
        // The validator surfaces unknown end_reason values as KUL-R05b; the
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
    fn shape_difference_aligns_on_shared_columns() {
        // alice has `born:`; bob doesn't. Under sparse-by-field-name (ADR-0004
        // amendment 2026-05-07), the two share columns up through their
        // common cells: keyword, positional, name, gender. bob's line stops
        // at his last actual cell (`gender:male`) — no whitespace placeholder
        // for `born:` because bob has nothing further to its right.
        let src = "person alice name:\"Alice\" gender:female born:1950\n\
                   person bo name:\"Bob\" gender:male\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female  born:1950\n\
             person bo     name:\"Bob\"    gender:male\n"
        );
    }

    #[test]
    fn blank_line_breaks_alignment() {
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
    fn whole_line_comment_is_transparent_to_alignment() {
        // Comments no longer break alignment under the per-region rule —
        // same-shape lines on either side of a comment join one group.
        let src = "person alice name:\"Alice\" gender:female\n\
                   # divider\n\
                   person bo name:\"Bob\" gender:male\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female\n\
             # divider\n\
             person bo     name:\"Bob\"    gender:male\n"
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
        // Two adoptions with same shape under one person align.
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
    fn sub_statements_under_different_persons_do_not_cross_align() {
        // Two persons in the same region, each with one adoption. Because
        // sub-statements scope per parent, the adoption-id columns are sized
        // independently and the longer id does not pad the shorter one.
        let src = "person alice name:\"A\" gender:female\n\
                   \x20\x20adoption m_short  start:1980\n\
                   person bob name:\"B\" gender:male\n\
                   \x20\x20adoption m_a_much_longer_id  start:1985\n";
        let out = format_source(src);
        assert!(
            out.contains("  adoption m_short  start:1980\n"),
            "alice's adoption should be at natural width, got:\n{out}"
        );
        assert!(
            out.contains("  adoption m_a_much_longer_id  start:1985\n"),
            "bob's adoption should be at natural width, got:\n{out}"
        );
    }

    #[test]
    fn person_with_substatement_aligns_to_neighbor_across_sub() {
        // Per the 2026-05-06 amendment: a sub-statement between two same-
        // shape persons no longer breaks alignment.
        let src = "person alice name:\"A\" gender:female\n\
                   \x20\x20birth m_a\n\
                   person bo name:\"B\" gender:male\n";
        let out = format_source(src);
        // alice and bo share columns; the birth sits at indent 2 between.
        assert_eq!(
            out,
            "person alice  name:\"A\"  gender:female\n\
             \x20\x20birth m_a\n\
             person bo     name:\"B\"  gender:male\n"
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
    fn inline_comment_on_one_row_aligns_shared_columns() {
        // alice has a trailing inline comment; bo does not. Under
        // sparse-by-field-name they're still in the same group (same indent,
        // same keyword) and share columns up through bo's last actual cell.
        // bo's `gender:male` ends shorter — no trailing whitespace for the
        // missing comment column.
        let src = "person alice name:\"Alice\" gender:female  # alpha\n\
                   person bo name:\"Bob\" gender:male\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female  # alpha\n\
             person bo     name:\"Bob\"    gender:male\n"
        );
    }

    #[test]
    fn format_source_idempotent_on_canonical_input() {
        let canonical = "kul 0.1\n\
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

    #[test]
    fn person_with_extra_died_field_aligns_shared_columns_with_neighbor() {
        // Reproduces the Gen 2 case from examples/03-three-generations.kul:
        // alice (no `died`) and bob (with `died`) are consecutive in the same
        // region. The user expects the columns they share — name, gender,
        // born — to line up; bob's extra `died:` cell sits past the shared
        // prefix at its natural width.
        let src = "person alice name:\"Alice Sharma\" gender:female born:1950-04-12\n\
                   person bob name:\"Bob Sharma\" gender:male born:1948-11-30 died:2020-03-15\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice Sharma\"  gender:female  born:1950-04-12\n\
             person bob    name:\"Bob Sharma\"    gender:male    born:1948-11-30  died:2020-03-15\n"
        );
    }

    #[test]
    fn missing_middle_field_inserts_whitespace_placeholder() {
        // alice has [name, gender, born]; bob has [name, gender, family,
        // born, died]. `family:` is in the middle of bob's line per spec
        // §14.2 canonical order. Sparse-by-field-name puts a whitespace
        // placeholder of `family:`-column-width on alice's line so her
        // `born:` column-aligns with bob's.
        let src = "person alice name:\"Alice\" gender:female born:1950\n\
                   person bob name:\"Bob\" gender:male family:\"Sharma\" born:1948 died:2020\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female                   born:1950\n\
             person bob    name:\"Bob\"    gender:male    family:\"Sharma\"  born:1948  died:2020\n"
        );
    }

    #[test]
    fn adjacent_regions_size_columns_independently() {
        // Two regions of `person` lines, separated by a blank line. Each
        // region has its own multi-line group with its own widest cells, so
        // a long id in region A must not pad ids in region B.
        let src = "person alexandria name:\"A\" gender:female\n\
                   person beatrice   name:\"B\" gender:female\n\
                   \n\
                   person c name:\"C\" gender:male\n\
                   person d name:\"D\" gender:male\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alexandria  name:\"A\"  gender:female\n\
             person beatrice    name:\"B\"  gender:female\n\
             \n\
             person c  name:\"C\"  gender:male\n\
             person d  name:\"D\"  gender:male\n"
        );
    }

    #[test]
    fn three_regions_each_keep_their_own_widths() {
        // Region 1 has the widest `name:` (long string); region 2 has the
        // widest id; region 3 is narrowest. Confirms each region's column
        // widths are computed in isolation — no cross-region bleed.
        let src = "person a name:\"Alexandria the Great\" gender:female\n\
                   person b name:\"Bo\"                    gender:female\n\
                   \n\
                   person aaaaa name:\"X\" gender:male\n\
                   person bbbbb name:\"Y\" gender:male\n\
                   \n\
                   person p name:\"P\" gender:other\n\
                   person q name:\"Q\" gender:other\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person a  name:\"Alexandria the Great\"  gender:female\n\
             person b  name:\"Bo\"                    gender:female\n\
             \n\
             person aaaaa  name:\"X\"  gender:male\n\
             person bbbbb  name:\"Y\"  gender:male\n\
             \n\
             person p  name:\"P\"  gender:other\n\
             person q  name:\"Q\"  gender:other\n"
        );
    }

    #[test]
    fn marriage_region_between_person_regions_is_independent() {
        // person block, marriage block, person block — three regions, three
        // independent groups (with different keywords, so even within a
        // region they wouldn't have shared columns). Verifies the blank-line
        // boundary plus the per-keyword grouping interact cleanly.
        let src = "person alice name:\"Alice\" gender:female\n\
                   person bo    name:\"Bob\"   gender:male\n\
                   \n\
                   marriage m_alice_bo alice bo start:1972\n\
                   marriage m_short    alice bo start:1980\n\
                   \n\
                   person c name:\"C\" gender:other\n\
                   person dd name:\"D\" gender:other\n";
        let out = format_source(src);
        assert_eq!(
            out,
            "person alice  name:\"Alice\"  gender:female\n\
             person bo     name:\"Bob\"    gender:male\n\
             \n\
             marriage m_alice_bo alice bo  start:1972\n\
             marriage m_short    alice bo  start:1980\n\
             \n\
             person c   name:\"C\"  gender:other\n\
             person dd  name:\"D\"  gender:other\n"
        );
    }

    #[test]
    fn last_cell_left_of_rightmost_column_emits_no_trailing_whitespace() {
        // bob's last actual cell is `gender:male` even though the group's
        // rightmost present column is `comment` (alice has one). Bob's line
        // must end immediately after `gender:male` — no padding through the
        // remaining slots.
        let src = "person alice name:\"Alice\" gender:female  # n\n\
                   person bo name:\"Bob\" gender:male\n";
        let out = format_source(src);
        let bo_line = out.lines().nth(1).expect("two lines");
        assert_eq!(bo_line, "person bo     name:\"Bob\"    gender:male");
        assert!(
            !bo_line.ends_with(' '),
            "bo's line must not have trailing whitespace, got {bo_line:?}"
        );
    }
}

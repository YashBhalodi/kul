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
//! Both entry points convert each rendered line to a sequence of
//! [`cells::Cell`]s and queue it into a region buffer. A *region* is the run
//! of lines between two blank lines (or document start/end); the blank line
//! is the only region boundary.
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
//! ## Internal layout
//!
//! - [`cells`] — the cell grammar: `Cell`, `CellKind`, `KindTag`, the
//!   canonical column tables, and the AST→Cell builders. Also owns the
//!   small value-stringifiers (`quote_string`, `gender_str`,
//!   `end_reason_str`).
//! - [`emit`] — the layout engine: `Emitter`, region buffering, per-group
//!   width computation, and the line-emission walk.
//! - [`source`] — the source-pass formatter that preserves comments by
//!   re-scanning the original bytes (the lexer drops them) and threading
//!   them through the emitter at their original lines.
//!
//! [ADR 0004]: https://github.com/YashBhalodi/kul/blob/main/docs/adr/0004-formatter-canonical-rules.md

mod cells;
mod emit;
mod source;

use crate::ast::KulFile;

use emit::Emitter;
use source::SourceFormatter;

/// Format a parsed [`KulFile`] to canonical Kul source.
///
/// Comments are not preserved — the AST doesn't model them, so the only
/// caller who should reach for this entry point is code that builds a
/// `KulFile` in memory (e.g. a code-generation tool). For
/// source-to-source formatting use [`format_source`]. The output ends
/// with a trailing newline if the file is non-empty.
pub fn format(file: &KulFile) -> String {
    let mut emitter = Emitter::new();
    for stmt in &file.statements {
        emitter.emit_statement(stmt);
    }
    emitter.finish()
}

/// Format a Kul source string to its canonical form.
///
/// Per ADR-0011, formatting is per-file (project-scoped canonicalization
/// is out of scope). Comments are preserved byte-for-byte per
/// [ADR 0004] rule 7. The function lexes and parses internally; if the
/// parser produces a partial AST (because of recoverable parse errors),
/// this still returns *some* output reflecting what was parseable.
/// Callers that need to reject malformed input should run
/// [`crate::check`] first and bail on parse-error diagnostics.
///
/// [ADR 0004]: https://github.com/YashBhalodi/kul/blob/main/docs/adr/0004-formatter-canonical-rules.md
pub fn format_source(source: &str) -> String {
    use crate::span::FileId;
    let tokens = crate::lexer::tokenize(source);
    let (statements, _) = crate::parser::parse(&tokens, FileId(1));
    let file = KulFile {
        name: String::new(),
        source: source.to_string(),
        statements,
    };
    SourceFormatter::new(source, &file).run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_empty_doc_is_empty_string() {
        let file = KulFile {
            name: String::new(),
            source: String::new(),
            statements: Vec::new(),
        };
        assert_eq!(format(&file), "");
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
        let canonical = "person alice  name:\"Alice\"  gender:female  born:1950-04-12\n\
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

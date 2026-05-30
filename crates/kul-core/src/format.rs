//! Canonical formatter for Kul documents.
//!
//! Two entry points: [`format`] (AST-only, for code-gen tools — comments
//! aren't in the AST) and [`format_source`] (string-in / string-out,
//! comments preserved per ADR-0004 rule 7).
//!
//! ## Per-region sparse alignment
//!
//! A *region* is the run of lines between blank lines (the only region
//! boundary). Within a region, lines bucket into *alignment groups* by
//! `(indent, kind)`, or `(indent, kind, parent_person_id)` for
//! sub-statements so siblings under different parents don't cross-align.
//!
//! Each kind has a fixed canonical column ordering (spec §14.2). Each cell
//! is tagged with its column index when built. A column is *present* in a
//! group iff at least one line carries it; the width is the max across
//! carriers. When emitting, the formatter walks the canonical sequence,
//! padding cells and whitespace placeholders, then stops at the line's
//! last actual cell — no trailing whitespace through later slots. Shape
//! is not part of the group key.
//!
//! ## Internal layout
//!
//! - [`cells`] — cell grammar (`Cell`, `CellKind`, `KindTag`), canonical
//!   column tables, AST→Cell builders, value stringifiers.
//! - [`emit`] — `Emitter`, region buffering, width computation, line walk.
//! - [`source`] — source-pass formatter; preserves comments by re-scanning
//!   the original bytes (the lexer drops them).
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
/// Comments are NOT preserved (not in the AST). For source-to-source use
/// [`format_source`]. Output ends with a trailing newline if non-empty.
pub fn format(file: &KulFile) -> String {
    let mut emitter = Emitter::new();
    for stmt in &file.statements {
        emitter.emit_statement(stmt);
    }
    emitter.finish()
}

/// Format a Kul source string to its canonical form.
///
/// Per-file (ADR-0011). Comments preserved byte-for-byte per ADR-0004
/// rule 7. Returns partial output on recoverable parse errors; callers
/// who need strictness should run [`crate::check`] first.
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
        let src = "person alice name:\"A\" gender:female\n\
                   \x20\x20birth m_a\n\
                   person bo name:\"B\" gender:male\n";
        let out = format_source(src);
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

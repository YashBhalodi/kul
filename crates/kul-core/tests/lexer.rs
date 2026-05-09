//! Snapshot tests for the lexer.

use kul_core::lexer::{Token, tokenize};

fn render_tokens(source: &str) -> String {
    let tokens = tokenize(source);
    tokens
        .iter()
        .map(|Token { kind, span }| format!("{:>3}..{:<3} {kind:?}", span.start, span.end))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn lex_minimal_person() {
    let source = "person alice name:\"Alice\" gender:female\n";
    insta::assert_snapshot!(render_tokens(source));
}

#[test]
fn lex_kul_is_a_normal_identifier_now() {
    // Sanity: after the manifest refactor (issue 69), `kul` is no longer a
    // keyword — it lexes like any other identifier.
    let source = "person kul name:\"K\" gender:other\n";
    insta::assert_snapshot!(render_tokens(source));
}

#[test]
fn lex_comment_after_content() {
    let source = "person alice name:\"Alice\" gender:female  # the founder\n";
    insta::assert_snapshot!(render_tokens(source));
}

#[test]
fn lex_indented_line() {
    let source = "person alice name:\"Alice\" gender:female\n  birth m_x\n";
    insta::assert_snapshot!(render_tokens(source));
}

#[test]
fn lex_unterminated_string() {
    let source = "person alice name:\"Alice\n";
    insta::assert_snapshot!(render_tokens(source));
}

#[test]
fn lex_string_escapes() {
    let source = "person alice name:\"He said \\\"hi\\\"\" gender:other\n";
    insta::assert_snapshot!(render_tokens(source));
}

#[test]
fn lex_invalid_escape() {
    let source = "person alice name:\"a\\nb\"\n";
    insta::assert_snapshot!(render_tokens(source));
}

#[test]
fn lex_marriage_with_divorce() {
    let source = "marriage m_a_b alice bob start:1972-05-12 end:1990-08-01 end_reason:divorce\n";
    insta::assert_snapshot!(render_tokens(source));
}

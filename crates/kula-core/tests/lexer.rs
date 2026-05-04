//! Snapshot tests for the lexer.

use kula_core::lexer::{Token, tokenize};

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
fn lex_version_decl() {
    let source = "kula 0.1\n\nperson alice name:\"Alice\" gender:female\n";
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

//! Snapshot tests for the parser AST.

use kula_core::lexer::tokenize;
use kula_core::parser::parse;

fn render(source: &str) -> String {
    let tokens = tokenize(source);
    let (doc, diags) = parse(&tokens);
    let mut out = String::new();
    out.push_str(&format!("ast: {doc:#?}\n"));
    out.push_str(&format!("diagnostics: {diags:#?}\n"));
    out
}

#[test]
fn parse_minimal_person() {
    insta::assert_snapshot!(render("person alice name:\"Alice Sharma\" gender:female\n"));
}

#[test]
fn parse_version_decl() {
    insta::assert_snapshot!(render(
        "kula 0.1\n\nperson alice name:\"Alice\" gender:female\n"
    ));
}

#[test]
fn parse_two_persons() {
    insta::assert_snapshot!(render(
        "person alice name:\"Alice\" gender:female\nperson bob name:\"Bob\" gender:male\n"
    ));
}

#[test]
fn parse_recovers_from_bad_field_then_continues() {
    insta::assert_snapshot!(render(
        "person alice oops:bad gender:female\nperson bob name:\"Bob\" gender:male\n"
    ));
}

#[test]
fn parse_unsupported_person_field_diagnoses() {
    insta::assert_snapshot!(render(
        "person alice name:\"Alice\" born:1950 gender:female\n"
    ));
}

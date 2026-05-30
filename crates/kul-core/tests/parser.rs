//! Snapshot tests for the parser AST.

use kul_core::lexer::tokenize;
use kul_core::parser::parse;
use kul_core::span::FileId;

/// Per-file statement-list view that the parser snapshots stringify;
/// matches the legacy `ast::Document` shape so the snapshots stay stable
/// independently of the multi-file `Document` refactor.
#[derive(Debug)]
#[allow(dead_code)]
struct Document {
    statements: Vec<kul_core::ast::Statement>,
}

fn render(source: &str) -> String {
    let tokens = tokenize(source);
    let (statements, diags) = parse(&tokens, FileId::MANIFEST);
    let doc = Document { statements };
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
fn parse_kul_token_treated_as_normal_identifier() {
    insta::assert_snapshot!(render("person kul name:\"K\" gender:other\n"));
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

#[test]
fn parse_marriage_minimal() {
    insta::assert_snapshot!(render("marriage m_a_b alice bob start:1972-05-12\n"));
}

#[test]
fn parse_marriage_with_end() {
    insta::assert_snapshot!(render(
        "marriage m_a_b alice bob start:1972-05-12 end:1990-08-01 end_reason:divorce\n"
    ));
}

#[test]
fn parse_marriage_self_marriage_still_parses() {
    insta::assert_snapshot!(render("marriage m alice alice start:1972-05-12\n"));
}

#[test]
fn parse_invalid_marriage_field_recovers() {
    insta::assert_snapshot!(render(
        "marriage m_a_b alice bob name:\"oops\"\nperson c name:\"C\" gender:female\n"
    ));
}

#[test]
fn parse_person_with_birth_sub_statement() {
    insta::assert_snapshot!(render(
        "person carol name:\"Carol\" gender:female\n  birth m_alice_bob\n"
    ));
}

#[test]
fn parse_person_with_adoption_sub_statement() {
    insta::assert_snapshot!(render(
        "person ravi name:\"Ravi\" gender:male\n  adoption m_alice_bob start:1985-06-01\n"
    ));
}

#[test]
fn parse_person_with_birth_and_adoption() {
    insta::assert_snapshot!(render(
        "person ravi name:\"Ravi\" gender:male\n  birth m_x\n  adoption m_y start:1985-06-01 end:1990-12-31\n"
    ));
}

#[test]
fn parse_person_with_two_birth_diagnoses() {
    insta::assert_snapshot!(render(
        "person carol name:\"Carol\" gender:female\n  birth m_one\n  birth m_two\n"
    ));
}

/// P07 must phrase the fix in plain English ("quoted string", with an example),
/// not the jargon "string literal".
#[test]
fn unquoted_string_value_message_hints_at_quotes() {
    let tokens = tokenize("person alice name:Alice gender:female\n");
    let (_, diags) = parse(&tokens, FileId::MANIFEST);
    let p07: Vec<_> = diags.iter().filter(|d| d.code == "KUL-P07").collect();
    assert_eq!(p07.len(), 1, "expected one KUL-P07, got: {diags:#?}");
    let msg = &p07[0].message;
    assert!(
        msg.contains("quoted string"),
        "message should say `quoted string`, not `string literal`; got: {msg}"
    );
    assert!(
        msg.contains(r#"name:"…""#) || msg.contains(r#"name:"Alice""#),
        "message should include an example like `name:\"…\"`; got: {msg}"
    );
    assert!(
        !msg.contains("string literal"),
        "message must not say `string literal` (jargon); got: {msg}"
    );
}

//! Validator tests, organized per rule. The corpus under `tests/corpus/`
//! provides the inputs; this file asserts the rendered diagnostic output.

use kula_core::CheckResult;
use kula_core::diagnostic::Diagnostic;

fn check(source: &str) -> CheckResult {
    kula_core::check(source)
}

fn render_diagnostics(diags: &[Diagnostic]) -> String {
    diags
        .iter()
        .map(|d| {
            format!(
                "{} [{}..{}]: {}",
                d.code, d.primary.start, d.primary.end, d.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_corpus(rel: &str) -> String {
    let path = format!("{}/tests/corpus/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing corpus file: {path}"))
}

#[test]
fn rule_03_missing_name() {
    let src = read_corpus("invalid/rule-03-missing-name.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_03_missing_gender() {
    let src = read_corpus("invalid/rule-03-missing-gender.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_03_missing_both() {
    let src = read_corpus("invalid/rule-03-missing-both.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn valid_single_person_is_clean() {
    let src = read_corpus("valid/01-single-person.kula");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_version_decl_is_clean() {
    let src = read_corpus("valid/02-version-decl.kula");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

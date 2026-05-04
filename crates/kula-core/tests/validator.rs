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
            let mut s = format!(
                "{} [{}..{}]: {}",
                d.code, d.primary.start, d.primary.end, d.message
            );
            for r in &d.related {
                s.push_str(&format!(
                    "\n  related [{}..{}]: {}",
                    r.span.start, r.span.end, r.label
                ));
            }
            s
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

#[test]
fn valid_couple_is_clean() {
    let src = read_corpus("valid/03-couple.kula");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_couple_with_divorce_is_clean() {
    let src = read_corpus("valid/04-couple-with-divorce.kula");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_01_duplicate_person_id() {
    let src = read_corpus("invalid/rule-01-duplicate-id.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_01_duplicate_id_cross_kind() {
    let src = read_corpus("invalid/rule-01-duplicate-id-cross-kind.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_04_self_marriage() {
    let src = read_corpus("invalid/rule-04-self-marriage.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_03_marriage_missing_start() {
    let result = check(
        "person a name:\"A\" gender:female\nperson b name:\"B\" gender:male\nmarriage m a b\n",
    );
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_02_unresolved_references() {
    let src = read_corpus("invalid/rule-02-unresolved.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_02_wrong_kind_references() {
    let src = read_corpus("invalid/rule-02-wrong-kind.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn valid_nuclear_family_is_clean() {
    let src = read_corpus("valid/05-nuclear-family.kula");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_with_adoption_is_clean() {
    let src = read_corpus("valid/06-with-adoption.kula");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

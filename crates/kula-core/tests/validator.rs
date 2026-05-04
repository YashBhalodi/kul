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

#[test]
fn valid_with_dates_is_clean() {
    let src = read_corpus("valid/07-with-dates.kula");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_circa_and_partial_is_clean() {
    let src = read_corpus("valid/08-circa-and-partial.kula");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_06_died_before_born() {
    let src = read_corpus("invalid/rule-06-died-before-born.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_07_marriage_end_before_start() {
    let src = read_corpus("invalid/rule-07-marriage-end-before-start.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_08_adoption_end_before_start() {
    let src = read_corpus("invalid/rule-08-adoption-end-before-start.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn date_feb_30_rejected() {
    let src = read_corpus("invalid/date-feb-30.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn date_feb_29_non_leap_rejected() {
    let src = read_corpus("invalid/date-feb-29-non-leap.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_06_partial_overlap_does_not_fire() {
    // born:1925, died:1925-08 — overlap, so we should NOT fire R06.
    let src = "person p name:\"P\" born:1925 died:1925-08 gender:other\n";
    let result = check(src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_06_circa_overlap_does_not_fire() {
    // ~1900 covers 1895..1905; 1903 is inside — strictly-before is false.
    let src = "person p name:\"P\" born:~1900 died:1903 gender:other\n";
    let result = check(src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_09_marriage_before_spouse_born() {
    let src = read_corpus("invalid/rule-09-marriage-before-spouse-born.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_10_spouse_died_before_marriage() {
    let src = read_corpus("invalid/rule-10-spouse-died-before-marriage.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_11_child_born_before_parent() {
    let src = read_corpus("invalid/rule-11-child-born-before-parent.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_12_adoption_before_adopter_born() {
    let src = read_corpus("invalid/rule-12-adoption-before-adopter-born.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_05_end_without_reason() {
    let src = read_corpus("invalid/rule-05-end-without-reason.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_05_reason_without_end() {
    let src = read_corpus("invalid/rule-05-reason-without-end.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_05b_unknown_end_reason() {
    let src = read_corpus("invalid/rule-05b-unknown-end-reason.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_13_self_parent() {
    let src = read_corpus("invalid/rule-13-self-parent.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_13_two_cycle() {
    let src = read_corpus("invalid/rule-13-two-cycle.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_13_mixed_bio_adoption() {
    let src = read_corpus("invalid/rule-13-mixed-bio-adoption.kula");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rules_9_through_12_clean_on_full_example() {
    let src = std::fs::read_to_string(format!(
        "{}/../../examples/03-three-generations.kula",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("examples file");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected example to validate clean, got: {:#?}",
        result.diagnostics
    );
}

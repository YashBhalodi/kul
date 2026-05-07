//! Validator tests, organized per rule. The corpus under `tests/corpus/`
//! provides the inputs; this file asserts the rendered diagnostic output.

use kul_core::CheckResult;
use kul_core::diagnostic::Diagnostic;

fn check(source: &str) -> CheckResult {
    kul_core::check(source)
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
    let src = read_corpus("invalid/rule-03-missing-name.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_03_missing_gender() {
    let src = read_corpus("invalid/rule-03-missing-gender.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_03_missing_both() {
    let src = read_corpus("invalid/rule-03-missing-both.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn valid_single_person_is_clean() {
    let src = read_corpus("valid/01-single-person.kul");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_version_decl_is_clean() {
    let src = read_corpus("valid/02-version-decl.kul");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_couple_is_clean() {
    let src = read_corpus("valid/03-couple.kul");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_couple_with_divorce_is_clean() {
    let src = read_corpus("valid/04-couple-with-divorce.kul");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_01_duplicate_person_id() {
    let src = read_corpus("invalid/rule-01-duplicate-id.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_01_duplicate_id_cross_kind() {
    let src = read_corpus("invalid/rule-01-duplicate-id-cross-kind.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_04_self_marriage() {
    let src = read_corpus("invalid/rule-04-self-marriage.kul");
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
    let src = read_corpus("invalid/rule-02-unresolved.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_02_wrong_kind_references() {
    let src = read_corpus("invalid/rule-02-wrong-kind.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn valid_nuclear_family_is_clean() {
    let src = read_corpus("valid/05-nuclear-family.kul");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_with_adoption_is_clean() {
    let src = read_corpus("valid/06-with-adoption.kul");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_with_dates_is_clean() {
    let src = read_corpus("valid/07-with-dates.kul");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn valid_circa_and_partial_is_clean() {
    let src = read_corpus("valid/08-circa-and-partial.kul");
    let result = check(&src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_06_died_before_born() {
    let src = read_corpus("invalid/rule-06-died-before-born.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_07_marriage_end_before_start() {
    let src = read_corpus("invalid/rule-07-marriage-end-before-start.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_08_adoption_end_before_start() {
    let src = read_corpus("invalid/rule-08-adoption-end-before-start.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn date_feb_30_rejected() {
    let src = read_corpus("invalid/date-feb-30.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn date_feb_29_non_leap_rejected() {
    let src = read_corpus("invalid/date-feb-29-non-leap.kul");
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
    let src = read_corpus("invalid/rule-09-marriage-before-spouse-born.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_10_spouse_died_before_marriage() {
    let src = read_corpus("invalid/rule-10-spouse-died-before-marriage.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_11_child_born_before_parent() {
    let src = read_corpus("invalid/rule-11-child-born-before-parent.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_12_adoption_before_adopter_born() {
    let src = read_corpus("invalid/rule-12-adoption-before-adopter-born.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_05_end_without_reason() {
    let src = read_corpus("invalid/rule-05-end-without-reason.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_05_reason_without_end() {
    let src = read_corpus("invalid/rule-05-reason-without-end.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_05b_unknown_end_reason() {
    let src = read_corpus("invalid/rule-05b-unknown-end-reason.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_13_self_parent() {
    let src = read_corpus("invalid/rule-13-self-parent.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_13_two_cycle() {
    let src = read_corpus("invalid/rule-13-two-cycle.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_13_mixed_bio_adoption() {
    let src = read_corpus("invalid/rule-13-mixed-bio-adoption.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

/// When a string-field value fails to parse, the parser should still mark
/// the field as attempted so the validator's required-field check (R03)
/// doesn't add a misleading "missing field" diagnostic on top of the parse
/// error. And recovery shouldn't swallow the rest of the line — the
/// `gender:` field after the malformed `name:` should still parse, so its
/// R03 doesn't fire either.
#[test]
fn malformed_string_value_suppresses_cascading_missing_field_r03() {
    let result = check("person alice name:Alice gender:female\n");
    let codes: Vec<&str> = result.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(
        codes,
        vec!["KUL-P07"],
        "expected only KUL-P07; got: {:#?}",
        result.diagnostics
    );
}

/// Real repro from the LSP report: the malformed value spans multiple
/// tokens (`Alice Sharma`). Recovery still has to skip past `Sharma` and
/// land on `gender:female` so it parses cleanly with no cascading R03.
#[test]
fn malformed_multi_token_string_value_recovers_at_next_field() {
    let result = check("person alice name:Alice Sharma gender:female\n");
    let codes: Vec<&str> = result.diagnostics.iter().map(|d| d.code).collect();
    assert_eq!(
        codes,
        vec!["KUL-P07"],
        "expected only KUL-P07; got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rules_9_through_12_clean_on_full_example() {
    let src = std::fs::read_to_string(format!(
        "{}/../../examples/03-three-generations.kul",
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

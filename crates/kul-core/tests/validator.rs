//! Validator tests, organized per rule. The corpus under `tests/corpus/`
//! provides the inputs; this file asserts the rendered diagnostic output.

mod common;

use kul_core::diagnostic::Diagnostic;

use crate::common::check_one as check;

fn render_diagnostics(diags: &[Diagnostic]) -> String {
    diags
        .iter()
        .map(|d| {
            let primary = d.primary.expect("diagnostic must have anchor in tests");
            let mut s = format!(
                "{} [{}..{}]: {}",
                d.code, primary.span.start, primary.span.end, d.message
            );
            for r in &d.related {
                s.push_str(&format!(
                    "\n  related [{}..{}]: {}",
                    r.span.span.start, r.span.span.end, r.label
                ));
            }
            s
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_corpus(rel: &str) -> String {
    // Each fixture is a one-file project (ADR-0015): a `rel` of
    // `valid/<name>.kul` resolves to `valid/<name>/<name>.kul`.
    let (dir, file) = rel.split_once('/').expect("corpus key is `<dir>/<file>`");
    let name = file
        .strip_suffix(".kul")
        .expect("corpus key ends in `.kul`");
    let path = format!(
        "{}/tests/corpus/{dir}/{name}/{name}.kul",
        env!("CARGO_MANIFEST_DIR")
    );
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
fn valid_two_persons_is_clean() {
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
fn marriage_without_start_is_clean() {
    let result = check(
        "person a name:\"A\" gender:female\nperson b name:\"B\" gender:male\nmarriage m a b\n",
    );
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn marriage_with_end_but_no_start_is_clean() {
    let result = check(
        "person a name:\"A\" gender:female\nperson b name:\"B\" gender:male\nmarriage m a b end:1990 end_reason:divorce\n",
    );
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_07_does_not_fire_when_start_is_absent() {
    let result = check(
        "person a name:\"A\" gender:female\nperson b name:\"B\" gender:male\nmarriage m a b end:1990 end_reason:divorce\n",
    );
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "KUL-R07"),
        "R07 must not fire when marriage.start is absent: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_09_does_not_fire_when_start_is_absent() {
    let result = check(
        "person a name:\"A\" gender:female born:1950\nperson b name:\"B\" gender:male born:1955\nmarriage m a b\n",
    );
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "KUL-R09"),
        "R09 must not fire when marriage.start is absent: {:#?}",
        result.diagnostics
    );
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
    // born:1925 and died:1925-08 overlap, so R06 must not fire.
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
    // ~1900 covers 1895..1905; 1903 falls inside, so strictly-before is false.
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

#[test]
fn rule_14_mixed_role_concurrent() {
    let src = read_corpus("invalid/rule-14-mixed-role-concurrent.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_14_pure_join_concurrent() {
    let src = read_corpus("invalid/rule-14-pure-join-concurrent.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_14_mutually_polygamous() {
    let src = read_corpus("invalid/rule-14-mutually-polygamous.kul");
    let result = check(&src);
    insta::assert_snapshot!(render_diagnostics(&result.diagnostics));
}

#[test]
fn rule_14_monogamy_is_clean() {
    // N=1 un-ended marriage: rule does not fire even when the joining
    // spouse is not the host.
    let src = "\
person alice name:\"Alice\" gender:female
person bob   name:\"Bob\"   gender:male

marriage m_alice_bob alice bob start:1990-01-01
";
    let result = check(src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_14_sequential_mixed_role_is_clean() {
    // Alice's un_ended_count is 1 (one ended, one current) → not a
    // polygamy hub, R14 does not fire.
    let src = "\
person alice  name:\"Alice\"  gender:female
person bob    name:\"Bob\"    gender:male
person devraj name:\"Devraj\" gender:male

marriage m_alice_bob    alice  bob    start:1980-01-01 end:1988-06-01 end_reason:divorce
marriage m_devraj_alice devraj alice  start:1995-06-15
";
    let result = check(src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_14_all_ended_is_clean() {
    // Every marriage ended → un_ended_count is zero everywhere → R14
    // cannot fire.
    let src = "\
person alice  name:\"Alice\"  gender:female
person bob    name:\"Bob\"    gender:male
person devraj name:\"Devraj\" gender:male

marriage m_alice_bob    alice  bob    start:1980-01-01 end:1988-06-01 end_reason:divorce
marriage m_devraj_alice devraj alice  start:1990-01-01 end:1998-04-12 end_reason:divorce
";
    let result = check(src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_14_pure_host_concurrent_n2_is_clean() {
    // Hub hosts every concurrent marriage → R14 satisfied.
    let src = "\
person alice  name:\"Alice\"  gender:female
person devraj name:\"Devraj\" gender:male
person meera  name:\"Meera\"  gender:female

marriage m_devraj_meera devraj meera  start:1990-01-01
marriage m_devraj_alice devraj alice  start:1992-02-14
";
    let result = check(src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

#[test]
fn rule_14_pure_host_concurrent_n3_is_clean() {
    // N=3 shape: hub hosts each concurrent marriage; R14 stays satisfied
    // at every N.
    let src = "\
person alice  name:\"Alice\"  gender:female
person devraj name:\"Devraj\" gender:male
person meera  name:\"Meera\"  gender:female
person priya  name:\"Priya\"  gender:female

marriage m_devraj_meera devraj meera  start:1990-01-01
marriage m_devraj_alice devraj alice  start:1992-02-14
marriage m_devraj_priya devraj priya  start:1995-08-22
";
    let result = check(src);
    assert!(
        result.diagnostics.is_empty(),
        "expected no diagnostics, got: {:#?}",
        result.diagnostics
    );
}

/// A malformed string-field value must not cascade into a misleading R03
/// "missing field" diagnostic, and recovery must still parse the next field
/// on the line.
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

/// A malformed value spanning multiple tokens (`Alice Sharma`) must skip
/// past `Sharma` to the next field without a cascading R03.
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
        "{}/../../examples/02-three-generations/three-generations.kul",
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

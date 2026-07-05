//! Core-seam tests for **attribute filtering, sort, and count** (issue #260).
//!
//! Kinship correctness is proven in `kin.rs`; this file proves the third
//! query capability — the [`Query`] value's `where` / `sort` / `mode` /
//! `count` extensions over both the `allPersons` and `kinOf` sources.
//!
//! The load-bearing product here is the **true/false/unknown boundary** of the
//! three-valued predicates, so the primary fixture carries a person for each
//! date shape (exact, partial-year, partial-month, circa, missing) and every
//! operator is snapshotted against it in **both certainty modes**. Two kinds
//! of assertion:
//! - **Contract snapshots**: serialize the `query_envelope` bytes (the WASM
//!   `runQuery` and CLI-`json` surfaces mirror them).
//! - **Targeted behavioural checks**: the sort determinism rules (missing-last,
//!   id tiebreak, date-bounds order), composition onto a traversal, and the
//!   malformed-predicate typed error.

mod common;

use kul_core::CheckResult;
use kul_core::query::{
    FilterMode, IntRange, PersonField, Predicate, Query, QueryEvalError, QueryResult,
    SortDirection, SortSpec, run_query,
};

use crate::common::check_one;

/// A cohort spanning every date shape the three-valued predicates must handle,
/// plus family / gender variety (including `gender:other`) for membership and
/// a person with no `born` and no `family` for the missing-field paths. Ids are
/// chosen so the id-tiebreak order is not the declaration order.
const COHORT: &str = "\
person delta   name:\"Delta\"   gender:female  family:\"Rao\"    born:1950-04-12  died:2001-03-01
person bravo   name:\"Bravo\"   gender:male    family:\"Rao\"    born:1950
person alpha   name:\"Alpha\"   gender:other   family:\"Sen\"    born:1950-04
person charlie name:\"Charlie\" gender:male    family:\"Sen\"    born:~1950
person echo    name:\"Echo\"    gender:female  family:\"Rao\"    born:1961-07-30
person foxtrot name:\"Foxtrot\" gender:male                     born:1972
person golf    name:\"Golf\"    gender:female  family:\"Sen\"
";

fn cohort() -> CheckResult {
    check_one(COHORT)
}

/// Run an `allPersons` query and return the resulting person ids (the
/// `personIds` projection), or panic with the wrong shape.
fn person_ids(check: &CheckResult, query: &Query) -> Vec<String> {
    match run_query(check.resolved(), query).expect("query evaluates") {
        QueryResult::PersonIds { person_ids } => person_ids,
        other => panic!("expected personIds, got {other:?}"),
    }
}

/// The date comparison operators, for the matrix helper below.
enum DateOp {
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Neq,
}

/// A single `born` predicate over `allPersons` (certain mode by default).
fn born(op: DateOp, value: &str) -> Query {
    let field = PersonField::Born;
    let value = value.to_string();
    let predicate = match op {
        DateOp::Lt => Predicate::Lt { field, value },
        DateOp::Lte => Predicate::Lte { field, value },
        DateOp::Gt => Predicate::Gt { field, value },
        DateOp::Gte => Predicate::Gte { field, value },
        DateOp::Eq => Predicate::Eq { field, value },
        DateOp::Neq => Predicate::Neq { field, value },
    };
    Query::all_persons().filtered(predicate)
}

// ---------------------------------------------------------------------------
// The operator × date-shape × mode contract matrix
// ---------------------------------------------------------------------------
//
// The COHORT's `born` values, as closed intervals:
//   delta  1950-04-12  exact day
//   bravo  1950        whole year   [1950-01-01, 1950-12-31]
//   alpha  1950-04     whole month  [1950-04-01, 1950-04-30]
//   charlie ~1950      circa ±5y    [1945-01-01, 1955-12-31]
//   echo   1961-07-30  exact day
//   foxtrot 1972       whole year
//   golf   (missing)   → every predicate is unknown

#[test]
fn born_gt_1950_certain() {
    // gt 1950: true only where the whole interval is after 1950-12-31 —
    // echo and foxtrot. bravo/alpha/charlie overlap 1950 ⇒ unknown ⇒ excluded.
    insta::assert_json_snapshot!(person_ids(&cohort(), &born(DateOp::Gt, "1950")));
}

#[test]
fn born_gt_1950_include_uncertain() {
    // includeUncertain also keeps the overlapping (unknown) rows — the fuzzy
    // records a gap-finder wants — but never the certainly-false delta/bravo?
    // bravo is 1950 (unknown vs gt 1950), so it joins; delta 1950-04-12 is
    // within 1950 ⇒ unknown ⇒ joins; only nothing is certainly-false here.
    insta::assert_json_snapshot!(person_ids(
        &cohort(),
        &born(DateOp::Gt, "1950").including_uncertain()
    ));
}

#[test]
fn born_gt_1949_certain() {
    // gt 1949: every recorded interval is wholly after 1949 ⇒ true for all but
    // the missing golf. The `born>1949 vs born:1950 = true` boundary.
    insta::assert_json_snapshot!(person_ids(&cohort(), &born(DateOp::Gt, "1949")));
}

#[test]
fn born_lt_1950_certain() {
    // lt 1950: true only where the whole interval precedes 1950-01-01 — none
    // (charlie's ~1950 reaches back to 1945 but also forward to 1955 ⇒ unknown).
    insta::assert_json_snapshot!(person_ids(&cohort(), &born(DateOp::Lt, "1950")));
}

#[test]
fn born_lte_1950_certain() {
    // lte 1950: true where the whole interval is ≤ 1950-01-01. delta/bravo/alpha
    // start on/after 1950-01-01 but extend past it ⇒ unknown; nothing certain.
    insta::assert_json_snapshot!(person_ids(&cohort(), &born(DateOp::Lte, "1950")));
}

#[test]
fn born_gte_1961_certain() {
    // gte 1961: true where the whole interval is ≥ 1961-12-31's lower bound —
    // echo (1961-07-30) and foxtrot (1972).
    insta::assert_json_snapshot!(person_ids(&cohort(), &born(DateOp::Gte, "1961")));
}

#[test]
fn born_gt_1950_06_certain() {
    // gt 1950-06 vs born:1950 = unknown (1950 spans past 1950-06); a partial
    // literal boundary. delta (1950-04-12) is certainly ≤ 1950-06 ⇒ false.
    insta::assert_json_snapshot!(person_ids(&cohort(), &born(DateOp::Gt, "1950-06")));
}

#[test]
fn born_eq_1950_certain() {
    // eq as containment: true iff the recorded interval is wholly within 1950 —
    // bravo (=1950), delta (1950-04-12 ⊂ 1950), alpha (1950-04 ⊂ 1950).
    // charlie ~1950 overflows 1950 ⇒ unknown; echo/foxtrot disjoint ⇒ false.
    insta::assert_json_snapshot!(person_ids(&cohort(), &born(DateOp::Eq, "1950")));
}

#[test]
fn born_eq_1950_include_uncertain() {
    // includeUncertain adds charlie (~1950 overlaps but isn't contained).
    insta::assert_json_snapshot!(person_ids(
        &cohort(),
        &born(DateOp::Eq, "1950").including_uncertain()
    ));
}

#[test]
fn born_neq_1950_certain() {
    // neq is the mirror of eq: true iff disjoint from 1950 — echo, foxtrot.
    insta::assert_json_snapshot!(person_ids(&cohort(), &born(DateOp::Neq, "1950")));
}

#[test]
fn born_eq_1950_04_certain() {
    // eq 1950-04: true only for a recorded interval wholly within April 1950 —
    // alpha (1950-04) and delta (1950-04-12). bravo (whole 1950) overflows April
    // ⇒ unknown ⇒ excluded in certain.
    insta::assert_json_snapshot!(person_ids(&cohort(), &born(DateOp::Eq, "1950-04")));
}

// ---------------------------------------------------------------------------
// Presence, membership, string equality
// ---------------------------------------------------------------------------

#[test]
fn absent_died_gap_finding() {
    // The "born after 1950, no recorded death" story: absent(died) is
    // two-valued, always decidable — everyone but delta.
    let query = Query::all_persons().filtered(Predicate::Absent {
        field: PersonField::Died,
    });
    insta::assert_json_snapshot!(person_ids(&cohort(), &query));
}

#[test]
fn present_family_two_valued() {
    // present(family) excludes only foxtrot (no family recorded); missing is a
    // decidable *false* here, not unknown, so certain-mode keeps the rest.
    let query = Query::all_persons().filtered(Predicate::Present {
        field: PersonField::Family,
    });
    insta::assert_json_snapshot!(person_ids(&cohort(), &query));
}

#[test]
fn membership_family_set() {
    // family ∈ {Rao} — exact, case-sensitive. golf (family Sen) and foxtrot
    // (missing ⇒ unknown ⇒ excluded) drop out.
    let query = Query::all_persons().filtered(Predicate::In {
        field: PersonField::Family,
        values: vec!["Rao".to_string()],
    });
    insta::assert_json_snapshot!(person_ids(&cohort(), &query));
}

#[test]
fn membership_gender_including_other() {
    // gender ∈ {female, other} — `other` is a first-class value, not a gap.
    let query = Query::all_persons().filtered(Predicate::In {
        field: PersonField::Gender,
        values: vec!["female".to_string(), "other".to_string()],
    });
    insta::assert_json_snapshot!(person_ids(&cohort(), &query));
}

#[test]
fn membership_missing_field_unknown_certain_vs_uncertain() {
    // foxtrot has no family: `family ∈ {Rao,Sen}` is unknown for it — excluded
    // in certain, included with includeUncertain.
    let base = Query::all_persons().filtered(Predicate::In {
        field: PersonField::Family,
        values: vec!["Rao".to_string(), "Sen".to_string()],
    });
    let certain = person_ids(&cohort(), &base);
    let uncertain = person_ids(&cohort(), &base.clone().including_uncertain());
    assert!(!certain.contains(&"foxtrot".to_string()));
    assert!(uncertain.contains(&"foxtrot".to_string()));
}

// ---------------------------------------------------------------------------
// Conjunction (three-valued AND)
// ---------------------------------------------------------------------------

/// `family=Rao AND born>1950`: for delta/bravo (family Rao, born within 1950)
/// the second predicate is unknown ⇒ the conjunction is unknown; only echo
/// (Rao, 1961) is certainly true.
fn conjunction_query() -> Query {
    Query::all_persons()
        .filtered(Predicate::Eq {
            field: PersonField::Family,
            value: "Rao".to_string(),
        })
        .filtered(Predicate::Gt {
            field: PersonField::Born,
            value: "1950".to_string(),
        })
}

#[test]
fn conjunction_certain_excludes_unknown() {
    // certain keeps only the certainly-true echo.
    insta::assert_json_snapshot!(person_ids(&cohort(), &conjunction_query()));
}

#[test]
fn conjunction_include_uncertain_keeps_unknown() {
    // includeUncertain adds every row whose conjunction is unknown rather than
    // false: delta/bravo (Rao, born within 1950 ⇒ born>1950 unknown) and
    // foxtrot (born 1972 ⇒ true, but family missing ⇒ family=Rao unknown, so the
    // AND is unknown — the engine won't assert foxtrot *isn't* Rao).
    insta::assert_json_snapshot!(person_ids(
        &cohort(),
        &conjunction_query().including_uncertain()
    ));
}

// ---------------------------------------------------------------------------
// Sort determinism
// ---------------------------------------------------------------------------

fn sorted_ids(check: &CheckResult, field: PersonField, direction: SortDirection) -> Vec<String> {
    person_ids(
        check,
        &Query::all_persons().sorted(SortSpec { field, direction }),
    )
}

#[test]
fn sort_born_asc_missing_last_date_bounds() {
    // Date order by (lower, upper): bravo (1950, [..-12-31]) sorts after alpha
    // (1950-04) and delta (1950-04-12)? No — lower bound: bravo 1950-01-01 is
    // earliest. charlie ~1950 lower-bounds to 1945-01-01, earliest of all. golf
    // (missing born) sorts LAST regardless of direction.
    insta::assert_json_snapshot!(sorted_ids(&cohort(), PersonField::Born, SortDirection::Asc));
}

#[test]
fn sort_born_desc_missing_still_last() {
    // desc reverses the present values but keeps the missing golf last.
    insta::assert_json_snapshot!(sorted_ids(
        &cohort(),
        PersonField::Born,
        SortDirection::Desc
    ));
}

#[test]
fn sort_family_asc_missing_last_id_tiebreak() {
    // family by codepoint: Rao before Sen; within each, ties break by id
    // ascending (bravo, delta, echo | alpha, charlie, golf). foxtrot (no family)
    // sorts last.
    insta::assert_json_snapshot!(sorted_ids(
        &cohort(),
        PersonField::Family,
        SortDirection::Asc
    ));
}

#[test]
fn sort_family_desc_missing_last_id_tiebreak_still_ascending() {
    // desc flips Sen-before-Rao, but the id tiebreak within a family stays
    // ascending, and missing (foxtrot) stays last.
    insta::assert_json_snapshot!(sorted_ids(
        &cohort(),
        PersonField::Family,
        SortDirection::Desc
    ));
}

// ---------------------------------------------------------------------------
// Count over both sources; empty-set count = 0
// ---------------------------------------------------------------------------

fn count(check: &CheckResult, query: &Query) -> usize {
    match run_query(check.resolved(), query).expect("query evaluates") {
        QueryResult::Count { count } => count,
        other => panic!("expected count, got {other:?}"),
    }
}

#[test]
fn count_all_persons() {
    assert_eq!(count(&cohort(), &Query::all_persons().counting()), 7);
}

#[test]
fn count_filtered_and_empty_is_zero() {
    // A predicate no one satisfies → count 0 (and the empty set is exit-0 at
    // the CLI). family = a value nobody records.
    let query = Query::all_persons()
        .filtered(Predicate::Eq {
            field: PersonField::Family,
            value: "Nobody".to_string(),
        })
        .counting();
    assert_eq!(count(&cohort(), &query), 0);
}

// ---------------------------------------------------------------------------
// Composition onto a kin-set traversal
// ---------------------------------------------------------------------------

/// A three-generation lineage where descendants carry varied birth years and
/// one grandchild shares no family, so a filtered descendants query is
/// meaningful. Parenthood is via each child's `birth <marriage-id>` sub-hop.
const LINEAGE: &str = "\
person gramps  name:\"Gramps\"  gender:male    family:\"Rao\"  born:1920
person grandma name:\"Grandma\" gender:female  family:\"Sen\"  born:1925
marriage m1 gramps grandma
person mum     name:\"Mum\"     gender:female  family:\"Rao\"  born:1948
  birth m1
person uncle   name:\"Uncle\"   gender:male    family:\"Rao\"  born:1952
  birth m1
person dad     name:\"Dad\"     gender:male    family:\"Sen\"  born:1946
marriage m2 mum dad
person kid_a   name:\"Kid A\"   gender:female  family:\"Rao\"  born:1975
  birth m2
person kid_b   name:\"Kid B\"   gender:male    family:\"Sen\"  born:1978
  birth m2
";

#[test]
fn composition_descendants_where_family_sort_born_keeps_descriptors() {
    // `descendants of gramps where family=Rao sort born` — members keep their
    // descriptors (it stays the `members` shape), the filter drops the Sen
    // descendants, and the attribute sort replaces the pinned member order.
    let check = check_one(LINEAGE);
    let query = Query::kin_descendants("gramps", IntRange::from_one(None), None)
        .filtered(Predicate::Eq {
            field: PersonField::Family,
            value: "Rao".to_string(),
        })
        .sorted(SortSpec {
            field: PersonField::Born,
            direction: SortDirection::Asc,
        });
    insta::assert_snapshot!(
        serde_json::to_string_pretty(&run_query(check.resolved(), &query).expect("evaluates"))
            .expect("serialize")
    );
}

#[test]
fn composition_count_respects_certainty_mode() {
    // Count descendants born after 1950 — uncle (1952), kid_a (1975), kid_b
    // (1978) are certainly after; mum (1948) certainly before. All exact years,
    // so certain and includeUncertain agree here (no unknown rows).
    let check = check_one(LINEAGE);
    let query = Query::kin_descendants("gramps", IntRange::from_one(None), None)
        .filtered(Predicate::Gt {
            field: PersonField::Born,
            value: "1950".to_string(),
        })
        .counting();
    assert_eq!(count(&check, &query), 3);
}

// ---------------------------------------------------------------------------
// Bad input: typed error at the core seam
// ---------------------------------------------------------------------------

#[test]
fn malformed_date_literal_is_typed_error() {
    let check = cohort();
    let query = Query::all_persons().filtered(Predicate::Gt {
        field: PersonField::Born,
        value: "nope".to_string(),
    });
    let err = run_query(check.resolved(), &query).expect_err("malformed date rejected");
    assert!(matches!(err, QueryEvalError::BadPredicate { .. }));
}

#[test]
fn ordering_on_non_date_field_is_typed_error() {
    let check = cohort();
    let query = Query::all_persons().filtered(Predicate::Lt {
        field: PersonField::Name,
        value: "Zed".to_string(),
    });
    let err = run_query(check.resolved(), &query).expect_err("ordering on a string field rejected");
    assert!(matches!(err, QueryEvalError::BadPredicate { .. }));
}

#[test]
fn membership_on_date_field_is_typed_error() {
    let check = cohort();
    let query = Query::all_persons().filtered(Predicate::In {
        field: PersonField::Born,
        values: vec!["1950".to_string()],
    });
    let err = run_query(check.resolved(), &query).expect_err("`in` on a date field rejected");
    assert!(matches!(err, QueryEvalError::BadPredicate { .. }));
}

#[test]
fn bad_predicate_errors_even_on_empty_project() {
    // The predicate is compiled *before* traversal, so a malformed one errors
    // even when there are no persons to test.
    let check = check_one("");
    let query = Query::all_persons().filtered(Predicate::Gt {
        field: PersonField::Born,
        value: "not-a-date".to_string(),
    });
    assert!(matches!(
        run_query(check.resolved(), &query),
        Err(QueryEvalError::BadPredicate { .. })
    ));
}

#[test]
fn default_mode_is_certain() {
    // A freshly built query defaults to certain; the extension fields serialize
    // away when at their defaults (kept minimal on the wire).
    let query = Query::all_persons();
    assert_eq!(query.mode, FilterMode::Certain);
    assert!(query.predicates.is_empty());
    assert!(query.sort.is_none());
}

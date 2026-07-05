//! Core-seam tests for relationship resolution (`resolve`, issue #259).
//!
//! Resolution shares the kin-set traversal engine and descriptor derivation
//! (issue #256–#258); its correctness is proven **once**, here, the same way
//! the kin-set queries are. Two kinds of test:
//! - **Contract snapshots** over the small, deterministic cases (self,
//!   disconnected, none-within-bounds, unknown id): serialize the
//!   `resolve_relationship` envelope and pin its bytes — the WASM and
//!   CLI-`json` surfaces mirror them.
//! - **Targeted behavioural fixtures** for the hazards the design pinned:
//!   per-segment cap vs unbounded lineal, cousin-marriage multiplicity,
//!   adoption-into-relatives, double cousins, step subsumption, deterministic
//!   ordering, and the typed unknown/wrong-kind-id error.
//!
//! Richly-married families make an affinal-route haystack (every co-parent
//! marriage is an alternate route), so the busy corpus cases assert on the
//! *presence* and *ordering* of the load-bearing descriptors rather than
//! snapshotting the full — correct but voluminous — list.

mod common;

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::query::{
    Affinity, Classification, EdgeNature, LinealRole, ResolveConfig, Side, resolve,
    resolve_relationship,
};

use crate::common::check_one;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn examples_dir() -> PathBuf {
    workspace_root().join("examples")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn check_example(dir: &str, stem: &str) -> CheckResult {
    let path = examples_dir().join(dir).join(format!("{stem}.kul"));
    check_one(&read(&path))
}

/// Serialize the resolve envelope exactly as the WASM / CLI-`json` surfaces do
/// (the contract snapshot).
fn envelope_json(check: &CheckResult, x: &str, y: &str, config: &ResolveConfig) -> String {
    serde_json::to_string_pretty(&resolve_relationship(check, x, y, config))
        .expect("serialize resolve envelope")
}

fn default() -> ResolveConfig {
    ResolveConfig::default()
}

fn cap(n: u32) -> ResolveConfig {
    ResolveConfig {
        max_apex_generations: n,
    }
}

// ---------------------------------------------------------------------------
// Contract snapshots: the small, deterministic cases
// ---------------------------------------------------------------------------

#[test]
fn self_is_a_single_self_descriptor() {
    // x == y → exactly one `self` descriptor: empty path, sharing / side /
    // seniority / apexSeniority all notApplicable.
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(envelope_json(&check, "hiroshi", "hiroshi", &default()));
}

#[test]
fn direct_parent_is_one_lineal_descriptor() {
    // A direct parent tie: one lineal ancestor descriptor, and the step route
    // (via the co-parent's marriage) is suppressed by the real edge — a clean,
    // small list good for pinning the full descriptor serialization.
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(envelope_json(&check, "akiko", "hiroshi", &default()));
}

#[test]
fn disconnected_pair_reports_disconnected() {
    // Two persons in different components of the full relation graph → empty
    // list, `disconnected`. Raising the cap can never help.
    let check = check_example("07-disconnected-lineages", "disconnected-lineages");
    insta::assert_snapshot!(envelope_json(&check, "minjun", "lucas", &default()));
}

#[test]
fn same_component_beyond_cap_reports_none_within_bounds() {
    // Second cousins (apex up 3 / down 3) at cap 1: every blood route exceeds
    // the per-segment cap and no ≤2-affinal-hop detour bridges both branches →
    // empty, `noneWithinBounds` (same component; a bigger cap might help).
    let check = check_example("09-family-across-a-century", "family-across-a-century");
    insta::assert_snapshot!(envelope_json(&check, "tobi", "ife", &cap(1)));
}

#[test]
fn unknown_id_yields_error_envelope() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(envelope_json(&check, "hiroshi", "nobody", &default()));
}

// ---------------------------------------------------------------------------
// Typed unknown / wrong-kind id error (never an empty result)
// ---------------------------------------------------------------------------

#[test]
fn unknown_id_is_typed_error_at_core() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    // Either endpoint unknown → typed error naming the bad id.
    let err = resolve(check.resolved(), "hiroshi", "nobody", &default()).unwrap_err();
    assert_eq!(
        err,
        kul_core::query::QueryEvalError::UnknownPerson {
            id: "nobody".to_string()
        }
    );
    // `x` is checked first.
    let err_x = resolve(check.resolved(), "ghost", "hiroshi", &default()).unwrap_err();
    assert_eq!(
        err_x,
        kul_core::query::QueryEvalError::UnknownPerson {
            id: "ghost".to_string()
        }
    );
}

#[test]
fn marriage_id_where_person_expected_is_typed_error() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    let err = resolve(check.resolved(), "m_hiroshi_yuki", "akiko", &default()).unwrap_err();
    assert_eq!(
        err,
        kul_core::query::QueryEvalError::UnknownPerson {
            id: "m_hiroshi_yuki".to_string()
        }
    );
}

// ---------------------------------------------------------------------------
// Per-segment cap vs unbounded pure-lineal detection
// ---------------------------------------------------------------------------

#[test]
fn unbounded_lineal_detected_past_the_cap() {
    // tobi's great-grandparent oludare is up 3. Even at cap 1 — far below the
    // ascent depth — the direct-line tie is detected (unbounded pure-lineal
    // walk), so `noneWithinBounds` never hides a recorded direct-line tie.
    let check = check_example("09-family-across-a-century", "family-across-a-century");
    let result = resolve(check.resolved(), "tobi", "oludare", &cap(1)).unwrap();
    assert!(result.empty_reason.is_none());
    // At least one pure-blood lineal-ancestor descriptor of 3 generations.
    assert!(
        result.relationships.iter().any(|d| {
            matches!(
                d.classification,
                Classification::Lineal {
                    role: LinealRole::Ancestor,
                    generations: 3
                }
            ) && d.affinity == Affinity::Blood
                && d.edge_nature == EdgeNature::Blood
        }),
        "the direct-line tie survives the cap"
    );
}

#[test]
fn second_cousins_found_at_default_cap() {
    // The same pair the cap-1 test finds empty is found at the default cap —
    // the budget is the only difference.
    let check = check_example("09-family-across-a-century", "family-across-a-century");
    let result = resolve(check.resolved(), "tobi", "ife", &default()).unwrap();
    assert!(result.empty_reason.is_none());
    assert!(
        result.relationships.iter().any(|d| matches!(
            d.classification,
            Classification::Collateral {
                cousin_degree: 2,
                removed: 0,
                ..
            }
        ) && d.affinity == Affinity::Blood),
        "a blood second-cousin descriptor is found at the default cap"
    );
}

// ---------------------------------------------------------------------------
// Cousin-marriage: BOTH the spouse (self / inLaw) AND the collateral cousin
// descriptor — the tool never lies by omission.
// ---------------------------------------------------------------------------

const COUSIN_MARRIAGE: &str = "\
person gf name:\"GF\" gender:male born:1920
person gm name:\"GM\" gender:female born:1922
marriage m_g gf gm start:1945
person p1 name:\"P1\" gender:male born:1946
  birth m_g
person p2 name:\"P2\" gender:female born:1948
  birth m_g
person sp1 name:\"SP1\" gender:female born:1947
marriage m_p1 p1 sp1 start:1970
person sp2 name:\"SP2\" gender:male born:1945
marriage m_p2 p2 sp2 start:1972
person x name:\"X\" gender:male born:1972
  birth m_p1
person y name:\"Y\" gender:female born:1974
  birth m_p2
marriage m_xy x y start:1995
";

#[test]
fn cousin_marriage_returns_spouse_and_cousin() {
    // x and y are first cousins (their fathers-and-mothers p1/p2 are siblings)
    // who are also married. resolution must surface BOTH ties.
    let check = check_one(COUSIN_MARRIAGE);
    let result = resolve(check.resolved(), "x", "y", &default()).unwrap();

    // The spouse tie: self-classification, inLaw, one marriage hop.
    assert!(
        result.relationships.iter().any(|d| {
            matches!(d.classification, Classification::SelfRel)
                && d.affinity == Affinity::InLaw
                && d.path.len() == 1
        }),
        "the spouse descriptor is present"
    );
    // The blood cousin tie: collateral first cousins, blood.
    assert!(
        result.relationships.iter().any(|d| {
            matches!(
                d.classification,
                Classification::Collateral {
                    cousin_degree: 1,
                    removed: 0,
                    ..
                }
            ) && d.affinity == Affinity::Blood
                && d.edge_nature == EdgeNature::Blood
        }),
        "the blood first-cousin descriptor is present"
    );
}

// ---------------------------------------------------------------------------
// Adoption-into-relatives: two descriptors (adoptive lineal + blood
// collateral); terminates despite the cycle.
// ---------------------------------------------------------------------------

const ADOPTION_INTO_RELATIVES: &str = "\
person gp1 name:\"GP1\" gender:male born:1940
person gp2 name:\"GP2\" gender:female born:1942
marriage m_gp gp1 gp2 start:1965
person parent name:\"Parent\" gender:male born:1966
  birth m_gp
person aunt name:\"Aunt\" gender:female born:1968
  birth m_gp
person ps name:\"PS\" gender:female born:1967
marriage m_par parent ps start:1990
person ego name:\"Ego\" gender:male born:1992
  birth m_par
  adoption m_aunt start:1995
person asp name:\"ASp\" gender:male born:1966
marriage m_aunt aunt asp start:1990
";

#[test]
fn adoption_into_relatives_two_readings() {
    // `aunt` is ego's blood aunt (parent's sibling) AND adopts ego directly.
    // resolution returns both — an adoptive lineal parent and a blood aunt.
    let check = check_one(ADOPTION_INTO_RELATIVES);
    let result = resolve(check.resolved(), "ego", "aunt", &default()).unwrap();

    assert!(
        result.relationships.iter().any(|d| matches!(
            d.classification,
            Classification::Lineal {
                role: LinealRole::Ancestor,
                generations: 1
            }
        ) && d.edge_nature == EdgeNature::Adoptive),
        "the adoptive-parent reading is present"
    );
    assert!(
        result.relationships.iter().any(|d| matches!(
            d.classification,
            Classification::Collateral { up: 2, down: 1, .. }
        ) && d.edge_nature == EdgeNature::Blood
            && d.affinity == Affinity::Blood),
        "the blood-aunt reading is present"
    );
}

// ---------------------------------------------------------------------------
// Double cousins: two collateral descriptors differing in side and backbone.
// ---------------------------------------------------------------------------

const DOUBLE_COUSINS: &str = "\
person pgf name:\"PGF\" gender:male born:1920
person pgm name:\"PGM\" gender:female born:1922
marriage m_p pgf pgm start:1945
person b1 name:\"B1\" gender:male born:1946
  birth m_p
person b2 name:\"B2\" gender:male born:1948
  birth m_p
person mgf name:\"MGF\" gender:male born:1921
person mgm name:\"MGM\" gender:female born:1923
marriage m_m mgf mgm start:1946
person s1 name:\"S1\" gender:female born:1947
  birth m_m
person s2 name:\"S2\" gender:female born:1949
  birth m_m
marriage m_1 b1 s1 start:1970
marriage m_2 b2 s2 start:1972
person ego name:\"Ego\" gender:male born:1975
  birth m_1
person cous name:\"Cous\" gender:female born:1977
  birth m_2
";

#[test]
fn double_cousins_two_collateral_descriptors_differing_side() {
    let check = check_one(DOUBLE_COUSINS);
    let result = resolve(check.resolved(), "ego", "cous", &default()).unwrap();

    // The two blood first-cousin ties: one paternal (via the brothers), one
    // maternal (via the sisters), with distinct backbones.
    let blood_cousins: Vec<_> = result
        .relationships
        .iter()
        .filter(|d| {
            matches!(
                d.classification,
                Classification::Collateral {
                    cousin_degree: 1,
                    removed: 0,
                    ..
                }
            ) && d.affinity == Affinity::Blood
                && d.edge_nature == EdgeNature::Blood
        })
        .collect();
    assert_eq!(blood_cousins.len(), 2, "double cousins → two blood ties");
    let sides: Vec<Side> = blood_cousins.iter().map(|d| d.side).collect();
    assert!(
        sides.contains(&Side::Paternal) && sides.contains(&Side::Maternal),
        "one paternal, one maternal: {sides:?}"
    );
}

// ---------------------------------------------------------------------------
// Step subsumption holds in resolution: a real (adoptive) parent edge
// suppresses the step-parent reading.
// ---------------------------------------------------------------------------

const STEP_ADOPTED: &str = "\
person ego name:\"Ego\" gender:female born:2000
  birth m_dad_mom
  adoption m_dad_step start:2006
person dad name:\"Dad\" gender:male born:1970
person mom name:\"Mom\" gender:female born:1972
marriage m_dad_mom dad mom start:1998 end:2003 end_reason:divorce
person stepmom name:\"StepMom\" gender:female born:1974
marriage m_dad_step dad stepmom start:2005
";

#[test]
fn step_subsumed_by_real_edge_in_resolution() {
    // stepmom is both a step-parent (dad's wife) and a real adoptive parent.
    // resolution emits the adoptive parent and suppresses the step reading.
    let check = check_one(STEP_ADOPTED);
    let result = resolve(check.resolved(), "ego", "stepmom", &default()).unwrap();

    assert!(
        result.relationships.iter().any(|d| matches!(
            d.classification,
            Classification::Lineal {
                role: LinealRole::Ancestor,
                generations: 1
            }
        ) && d.edge_nature == EdgeNature::Adoptive),
        "the adoptive-parent reading is present"
    );
    assert!(
        !result.relationships.iter().any(|d| {
            matches!(
                d.classification,
                Classification::Lineal {
                    role: LinealRole::Ancestor,
                    generations: 1
                }
            ) && d.affinity == Affinity::Step
        }),
        "the step-parent reading is suppressed by the real edge"
    );
}

// ---------------------------------------------------------------------------
// Deterministic ordering: shortest tie first, then serialized backbone.
// ---------------------------------------------------------------------------

#[test]
fn relationships_sorted_by_hops_then_backbone() {
    let check = check_one(COUSIN_MARRIAGE);
    let result = resolve(check.resolved(), "x", "y", &default()).unwrap();
    assert!(
        result.relationships.len() >= 2,
        "a pair with several ties pins the order"
    );
    // Pinned order: path hop count ascending is monotone across the list.
    let lengths: Vec<usize> = result.relationships.iter().map(|d| d.path.len()).collect();
    let mut sorted = lengths.clone();
    sorted.sort_unstable();
    assert_eq!(lengths, sorted, "hop count is non-decreasing: {lengths:?}");
    // The shortest tie (the one-hop spouse) comes first.
    assert_eq!(result.relationships[0].path.len(), 1);
}

// ---------------------------------------------------------------------------
// Shared engine: resolution's blood-sibling descriptor matches the kin-set
// sibling descriptor (one vocabulary, no forked logic).
// ---------------------------------------------------------------------------

#[test]
fn resolution_sibling_matches_kin_set() {
    let check = check_example("02-three-generations", "three-generations");
    let resolved = check.resolved();
    // The kin-set sibling of chidi.
    let sibling = kul_core::query::siblings_of(resolved, "chidi").unwrap();
    let kin_desc = &sibling
        .iter()
        .find(|m| m.person.id.name == "amara")
        .expect("amara is chidi's sibling")
        .descriptor;
    // The same tie via resolution.
    let result = resolve(resolved, "chidi", "amara", &default()).unwrap();
    let res_desc = result
        .relationships
        .iter()
        .find(|d| {
            matches!(
                d.classification,
                Classification::Collateral { up: 1, down: 1, .. }
            ) && d.affinity == Affinity::Blood
        })
        .expect("resolution finds the sibling tie");
    // Same classification, sharing, side, seniority, and backbone.
    assert_eq!(res_desc.classification, kin_desc.classification);
    assert_eq!(res_desc.sharing, kin_desc.sharing);
    assert_eq!(res_desc.side, kin_desc.side);
    assert_eq!(res_desc.seniority, kin_desc.seniority);
    assert_eq!(res_desc.path, kin_desc.path);
}

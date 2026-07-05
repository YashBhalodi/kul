//! Core-seam tests for the lineal kin-set queries (issue #256).
//!
//! Kinship correctness is proven **once**, here at the `query` seam. Two
//! kinds of test:
//! - **Contract snapshots** over the example corpus: serialize the
//!   `kin_query` envelope and pin its bytes (the WASM and CLI-`json`
//!   surfaces mirror them). These cover ordering, edge-tagging, unbounded
//!   depth, and the full descriptor serialization.
//! - **Targeted behavioural fixtures** for the structural hazards the design
//!   pinned: cycle-guarded termination, doubly-reachable ancestors, multi-
//!   parent sets, side derivation (incl. `other`), seniority decidability,
//!   the edge-nature filter, and the unknown-anchor typed error.

mod common;

use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;
use kul_core::query::{
    Classification, EdgeNature, IntRange, LinealRole, Query, QueryEvalError, Seniority, Side,
    ancestors_of, children_of, descendants_of, evaluate, kin_query, parents_of,
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

fn check_multi_file(dir: &str) -> CheckResult {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(examples_dir().join(dir))
        .expect("read multi-file example directory")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("kul"))
        .collect();
    entries.sort();
    let inputs: Vec<InputFile> = entries
        .iter()
        .map(|p| {
            InputFile::new(
                p.file_name().unwrap().to_string_lossy().into_owned(),
                read(p),
            )
        })
        .collect();
    kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs)
}

/// Serialize the kin-query envelope exactly as the WASM / CLI-`json`
/// surfaces do (the contract snapshot).
fn envelope_json(check: &CheckResult, query: &Query) -> String {
    serde_json::to_string_pretty(&kin_query(check, query)).expect("serialize kin envelope")
}

// ---------------------------------------------------------------------------
// Contract snapshots over the example corpus
// ---------------------------------------------------------------------------

#[test]
fn nuclear_parents() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_ancestors("akiko", IntRange::exactly(1), None)
    ));
}

#[test]
fn nuclear_children() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_descendants("hiroshi", IntRange::exactly(1), None)
    ));
}

#[test]
fn three_generations_ancestors_unbounded() {
    // chidi → parents (emeka, ngozi) and grandparents (chinua, adaeze via
    // emeka). Unbounded depth; ngozi has no recorded parents.
    let check = check_example("02-three-generations", "three-generations");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_ancestors("chidi", IntRange::from_one(None), None)
    ));
}

#[test]
fn three_generations_descendants_unbounded() {
    let check = check_example("02-three-generations", "three-generations");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_descendants("chinua", IntRange::from_one(None), None)
    ));
}

#[test]
fn century_deep_ancestors_unbounded() {
    // tobi → bisi/tunji → babatunde/yetunde → oludare/folake: three
    // generations of unbounded lineal ascent.
    let check = check_example("09-family-across-a-century", "family-across-a-century");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_ancestors("tobi", IntRange::from_one(None), None)
    ));
}

#[test]
fn adoption_multi_parent_edge_tagged() {
    // dalisay carries a birth + two adoptions → a 6-parent set, each edge-
    // tagged bio/adoptive. Pins the descriptor's `edgeNature` and backbone.
    let check = check_example("04-adoption-and-belonging", "adoption-and-belonging");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_ancestors("dalisay", IntRange::exactly(1), None)
    ));
}

#[test]
fn adoption_other_gender_parent() {
    // bayani (gender:other) is adopted by the Mendozas — the adoptive
    // parents render, and bayani's own gender rides the descriptor.
    let check = check_example("04-adoption-and-belonging", "adoption-and-belonging");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_ancestors("bayani", IntRange::exactly(1), None)
    ));
}

#[test]
fn multi_file_descendants() {
    let check = check_multi_file("08-multi-file-project");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_descendants("diego", IntRange::from_one(None), None)
    ));
}

#[test]
fn empty_parent_set_is_empty_members() {
    // A person with no recorded parents → an empty (but ok) members set.
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_ancestors("hiroshi", IntRange::exactly(1), None)
    ));
}

#[test]
fn unknown_anchor_yields_error_envelope() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    insta::assert_snapshot!(envelope_json(
        &check,
        &Query::kin_ancestors("nobody", IntRange::exactly(1), None)
    ));
}

// ---------------------------------------------------------------------------
// Sugar desugars to the Query value (one evaluation path)
// ---------------------------------------------------------------------------

#[test]
fn sugar_matches_query_value() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    let resolved = check.resolved();

    // parents_of ≡ kin_ancestors {1,1}
    let via_sugar = parents_of(resolved, "akiko").unwrap();
    let via_query = evaluate(
        resolved,
        &Query::kin_ancestors("akiko", IntRange::exactly(1), None),
    )
    .unwrap();
    assert_eq!(
        member_ids(&via_sugar),
        member_ids(&via_query),
        "parents_of must desugar to the Query value"
    );

    // children_of ≡ kin_descendants {1,1}
    assert_eq!(
        member_ids(&children_of(resolved, "hiroshi").unwrap()),
        ["akiko", "kenji"]
    );

    // ancestors_of / descendants_of with unbounded depth
    assert!(ancestors_of(resolved, "hiroshi", None).unwrap().is_empty());
    assert_eq!(
        member_ids(&descendants_of(resolved, "hiroshi", Some(1)).unwrap()),
        ["akiko", "kenji"]
    );
}

fn member_ids(members: &[kul_core::query::KinMember<'_>]) -> Vec<String> {
    members.iter().map(|m| m.person.id.name.clone()).collect()
}

// ---------------------------------------------------------------------------
// Typed unknown-anchor error (never an empty set)
// ---------------------------------------------------------------------------

#[test]
fn unknown_anchor_is_typed_error() {
    let check = check_example("01-nuclear-family", "nuclear-family");
    let err = evaluate(
        check.resolved(),
        &Query::kin_ancestors("nobody", IntRange::exactly(1), None),
    )
    .unwrap_err();
    assert_eq!(
        err,
        QueryEvalError::UnknownPerson {
            id: "nobody".to_string()
        }
    );
}

#[test]
fn marriage_id_as_anchor_is_typed_error() {
    // An id that names a marriage where a person is required → the same
    // typed error, never an empty set.
    let check = check_example("01-nuclear-family", "nuclear-family");
    let err = parents_of(check.resolved(), "m_hiroshi_yuki").unwrap_err();
    assert_eq!(
        err,
        QueryEvalError::UnknownPerson {
            id: "m_hiroshi_yuki".to_string()
        }
    );
}

// ---------------------------------------------------------------------------
// Edge-nature filter
// ---------------------------------------------------------------------------

#[test]
fn edge_nature_filter_splits_bio_and_adoptive() {
    let check = check_example("04-adoption-and-belonging", "adoption-and-belonging");
    let resolved = check.resolved();

    let bio = evaluate(
        resolved,
        &Query::kin_ancestors("dalisay", IntRange::exactly(1), Some(EdgeNature::Blood)),
    )
    .unwrap();
    assert_eq!(member_ids(&bio), ["eduardo", "luz"]);

    let adoptive = evaluate(
        resolved,
        &Query::kin_ancestors("dalisay", IntRange::exactly(1), Some(EdgeNature::Adoptive)),
    )
    .unwrap();
    assert_eq!(member_ids(&adoptive), ["carlos", "elena", "rosa", "tomas"]);
}

// ---------------------------------------------------------------------------
// Side derivation (maternal / paternal / other / notApplicable)
// ---------------------------------------------------------------------------

const SIDE_FIXTURE: &str = "\
person ego name:\"Ego\" gender:male born:2000
  birth m_parents
person mom name:\"Mom\" gender:female born:1975
  birth m_maternal
person dad name:\"Dad\" gender:male born:1973
  birth m_paternal
marriage m_parents dad mom start:1998

person mgm name:\"MatGM\" gender:female born:1950
person mgf name:\"MatGF\" gender:male born:1948
marriage m_maternal mgf mgm start:1972

person pgm name:\"PatGM\" gender:female born:1945
person pgf name:\"PatGF\" gender:male born:1943
marriage m_paternal pgf pgm start:1970
";

#[test]
fn side_derivation() {
    let check = check_one(SIDE_FIXTURE);
    let resolved = check.resolved();

    // Direct parents → notApplicable (your mother is not your "maternal side").
    for parent in parents_of(resolved, "ego").unwrap() {
        assert_eq!(parent.descriptor.side, Side::NotApplicable);
    }

    // Grandparents through the female parent (mom) → maternal; through the
    // male parent (dad) → paternal.
    let grands = ancestors_of(resolved, "ego", Some(2)).unwrap();
    let side_of = |id: &str| {
        grands
            .iter()
            .find(|m| m.person.id.name == id)
            .unwrap()
            .descriptor
            .side
    };
    assert_eq!(side_of("mgm"), Side::Maternal);
    assert_eq!(side_of("mgf"), Side::Maternal);
    assert_eq!(side_of("pgm"), Side::Paternal);
    assert_eq!(side_of("pgf"), Side::Paternal);

    // `both` is a couple-apex phenomenon absent from any lineal path.
    let deep = ancestors_of(resolved, "ego", None).unwrap();
    assert!(deep.iter().all(|m| m.descriptor.side != Side::Both));

    // Descendants never carry a side.
    for kid in children_of(resolved, "dad").unwrap() {
        assert_eq!(kid.descriptor.side, Side::NotApplicable);
    }
}

#[test]
fn side_other_via_other_gender_parent() {
    // A child whose linking parent has gender:other → grandparents through
    // that parent get side `other`.
    let src = "\
person kid name:\"Kid\" gender:male born:2000
  birth m_parents
person pnb name:\"PNB\" gender:other born:1975
  birth m_grand
person pf name:\"PF\" gender:female born:1974
marriage m_parents pf pnb start:1998
person gx name:\"GX\" gender:male born:1950
person gy name:\"GY\" gender:female born:1952
marriage m_grand gx gy start:1972
";
    let check = check_one(src);
    let grands = ancestors_of(check.resolved(), "kid", Some(2)).unwrap();
    // Only the grandparents (path length 2) carry a side; the direct
    // parents at depth 1 are notApplicable. gx/gy are reached through pnb
    // (gender:other) → side other.
    let grandparents: Vec<_> = grands
        .iter()
        .filter(|m| m.descriptor.path.len() == 2)
        .collect();
    assert_eq!(grandparents.len(), 2, "gx and gy via pnb");
    for m in grandparents {
        assert_eq!(
            m.descriptor.side,
            Side::Other,
            "member {}",
            m.person.id.name
        );
    }
}

// ---------------------------------------------------------------------------
// Seniority (endpoint) via before_strict — decidable and indeterminate
// ---------------------------------------------------------------------------

#[test]
fn seniority_decidable_and_unknown() {
    // Each parent/child pair isolates one seniority outcome. Ancestors:
    // alter = parent, ego = child.
    let src = "\
person c_a name:\"CA\" gender:male born:1981
  birth m_a
person p_a name:\"PA\" gender:female born:1980
person p_a2 name:\"PA2\" gender:male born:1980
marriage m_a p_a2 p_a start:1979

person c_b name:\"CB\" gender:male born:1986
  birth m_b
person p_b name:\"PB\" gender:female born:~1980
person p_b2 name:\"PB2\" gender:male born:1979
marriage m_b p_b2 p_b start:1978

person c_c name:\"CC\" gender:male born:1980
  birth m_c
person p_c name:\"PC\" gender:female born:1980
person p_c2 name:\"PC2\" gender:male born:1979
marriage m_c p_c2 p_c start:1978

person c_d name:\"CD\" gender:male born:1983
  birth m_d
person p_d name:\"PD\" gender:female born:~1980
person p_d2 name:\"PD2\" gender:male born:1979
marriage m_d p_d2 p_d start:1978

person c_e name:\"CE\" gender:male born:2000
  birth m_e
person p_e name:\"PE\" gender:female
person p_e2 name:\"PE2\" gender:male born:1970
marriage m_e p_e2 p_e start:1990
";
    let check = check_one(src);
    let resolved = check.resolved();
    let sen = |child: &str, parent: &str| {
        parents_of(resolved, child)
            .unwrap()
            .into_iter()
            .find(|m| m.person.id.name == parent)
            .unwrap()
            .descriptor
            .seniority
    };
    // 1980 strictly before 1981 → parent elder.
    assert_eq!(sen("c_a", "p_a"), Seniority::Elder);
    // ~1980 (≤1985) strictly before 1986 → elder (decidable circa).
    assert_eq!(sen("c_b", "p_b"), Seniority::Elder);
    // Both 1980 → overlapping intervals → unknown.
    assert_eq!(sen("c_c", "p_c"), Seniority::Unknown);
    // ~1980 vs 1983 → intervals overlap → unknown.
    assert_eq!(sen("c_d", "p_d"), Seniority::Unknown);
    // Missing birth date → unknown.
    assert_eq!(sen("c_e", "p_e"), Seniority::Unknown);
    // A younger endpoint: a child (born 2000) is younger than its parent.
    let kid = children_of(resolved, "p_e2").unwrap();
    assert_eq!(kid[0].descriptor.seniority, Seniority::Younger);
}

// ---------------------------------------------------------------------------
// Cycle guarding: adoption-into-relatives terminates; doubly-reachable
// ancestor yields two members with distinct backbones.
// ---------------------------------------------------------------------------

#[test]
fn doubly_reachable_ancestor_two_members() {
    // g is both a bio parent (via m1) and an adoptive parent (via m2) of p.
    // The result carries two members for g — same alter, distinct backbones
    // (bio vs adoptive). No engine-side collapsing.
    let src = "\
person g name:\"G\" gender:male born:1940
person gm name:\"GM\" gender:female born:1942
person gm2 name:\"GM2\" gender:female born:1945
marriage m1 g gm start:1960
marriage m2 g gm2 start:1970
person p name:\"P\" gender:male born:1980
  birth m1
  adoption m2 start:1985
";
    let check = check_one(src);
    let parents = parents_of(check.resolved(), "p").unwrap();
    let g_members: Vec<_> = parents.iter().filter(|m| m.person.id.name == "g").collect();
    assert_eq!(g_members.len(), 2, "g is reachable two ways → two members");
    let natures: Vec<EdgeNature> = g_members.iter().map(|m| m.descriptor.edge_nature).collect();
    assert!(natures.contains(&EdgeNature::Blood) && natures.contains(&EdgeNature::Adoptive));
}

#[test]
fn adoption_into_relatives_terminates() {
    // A real cycle: `a` (a grandparent) is adopted into a marriage of their
    // own descendant, so `a → c → a` closes a loop in the relation graph.
    // Unbounded ascent must still terminate (simple-path guard) and never
    // include the anchor itself.
    let cycle_src = "\
person a name:\"A\" gender:male born:1940
  adoption m_c_sp start:2010
person b name:\"B\" gender:female born:1942
marriage m_ab a b start:1960
person c name:\"C\" gender:male born:1962
  birth m_ab
person sp name:\"SP\" gender:female born:1965
marriage m_c_sp c sp start:1985
person d name:\"D\" gender:male born:1986
  birth m_c_sp
";
    let check = check_one(cycle_src);
    // Must return (not hang) and never include the anchor.
    let anc = ancestors_of(check.resolved(), "d", None).unwrap();
    assert!(anc.iter().all(|m| m.person.id.name != "d"));
    // d's ancestors include c and sp (parents) and, via c, a and b.
    let ids = member_ids(&anc);
    assert!(ids.contains(&"c".to_string()));
    assert!(ids.contains(&"a".to_string()));
    assert!(ids.contains(&"b".to_string()));
    // Every descriptor is a lineal ancestor.
    assert!(anc.iter().all(|m| matches!(
        m.descriptor.classification,
        Classification::Lineal {
            role: LinealRole::Ancestor,
            ..
        }
    )));
}

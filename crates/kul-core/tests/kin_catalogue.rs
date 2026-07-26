//! The **kin-set catalogue contract** — the thirteen named sugars, as the
//! `Query` values a non-Rust consumer has to build for itself.
//!
//! `crates/kul-core/src/query/sugar.rs` is Rust-only: the WASM surface exposes
//! `queryKin(files, manifest, query)` and no sugar at all, so a JS consumer
//! that wants "siblings" constructs `kinOf(x, collateral up{1,1} down{1,1})`
//! by hand. `packages/preview/src/kin-sets.ts` does exactly that for all
//! thirteen, and until this file existed nothing checked the copy.
//!
//! Two tests, and they close different holes:
//!
//! - [`every_sugar_matches_its_catalogue_query`] proves each catalogue `Query`
//!   is *behaviourally* the sugar it claims to be, by running both over the
//!   example corpus and comparing member lists. A drift in the arguments
//!   `sugar.rs` passes — `IntRange::exactly(2)` becoming `exactly(3)` — fails
//!   here.
//! - [`catalogue_patterns`] pins the serialized `KinPattern` of each, as JSON.
//!   `packages/preview/tests/kin-sets.test.ts` reads this snapshot and fails if
//!   the preview's hand-built patterns have drifted from it. A drift *inside*
//!   `Query::kin_*` — `any{2,2}` becoming `any{3,3}` — fails there.
//!
//! Kinship correctness is not re-tested here; `kin.rs` owns that. What is
//! tested is that one spelling of a question is the same question as another.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kul_core::CheckResult;
use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;
use kul_core::query::{
    IntRange, KinMember, Query, QueryEvalError, QuerySource, ancestors_of, aunts_uncles_of,
    children_of, cousins_of, descendants_of, evaluate, in_laws_of, nieces_nephews_of, parents_of,
    siblings_of, spouses_of, step_children_of, step_parents_of, step_siblings_of,
};
use kul_core::semantic::ResolvedDocument;

/// One catalogue row: the sugar's name, the sugar itself, and the `Query`
/// value a consumer builds to ask the same question.
struct Row {
    /// The `pub fn` in `sugar.rs`. Also the snapshot key, and what the
    /// preview's row ids are derived from.
    sugar: &'static str,
    /// The sugar, applied to an anchor.
    call: for<'a> fn(&'a ResolvedDocument, &str) -> Result<Vec<KinMember<'a>>, QueryEvalError>,
    /// The Query value the preview builds for that row.
    query: fn(&str) -> Query,
}

/// The catalogue, in the order the preview's list shows it (#276's
/// "Parents … Step-children"). Each `query` is the expansion the paired
/// sugar's own doc comment states.
fn catalogue() -> Vec<Row> {
    vec![
        Row {
            sugar: "parents_of",
            call: |r, a| parents_of(r, a),
            query: |a| Query::kin_ancestors(a, IntRange::exactly(1), None),
        },
        Row {
            sugar: "children_of",
            call: |r, a| children_of(r, a),
            query: |a| Query::kin_descendants(a, IntRange::exactly(1), None),
        },
        Row {
            sugar: "siblings_of",
            call: |r, a| siblings_of(r, a),
            query: |a| Query::kin_collateral(a, IntRange::exactly(1), IntRange::exactly(1), None),
        },
        Row {
            sugar: "spouses_of",
            call: |r, a| spouses_of(r, a),
            query: |a| Query::kin_spouses(a),
        },
        Row {
            sugar: "ancestors_of",
            call: |r, a| ancestors_of(r, a, None),
            query: |a| Query::kin_ancestors(a, IntRange::from_one(None), None),
        },
        Row {
            sugar: "descendants_of",
            call: |r, a| descendants_of(r, a, None),
            query: |a| Query::kin_descendants(a, IntRange::from_one(None), None),
        },
        Row {
            sugar: "aunts_uncles_of",
            call: |r, a| aunts_uncles_of(r, a),
            query: |a| Query::kin_collateral(a, IntRange::exactly(2), IntRange::exactly(1), None),
        },
        Row {
            sugar: "nieces_nephews_of",
            call: |r, a| nieces_nephews_of(r, a),
            query: |a| Query::kin_collateral(a, IntRange::exactly(1), IntRange::exactly(2), None),
        },
        Row {
            // The list offers the one cell a reader means by "cousins";
            // `cousins_of` spans the whole degree/removal lattice.
            sugar: "cousins_of",
            call: |r, a| cousins_of(r, a, 1, 0),
            query: |a| {
                Query::kin_collateral_by_degree(a, IntRange::exactly(1), IntRange::exactly(0), None)
            },
        },
        Row {
            sugar: "in_laws_of",
            call: |r, a| in_laws_of(r, a),
            query: |a| Query::kin_in_laws(a),
        },
        Row {
            sugar: "step_parents_of",
            call: |r, a| step_parents_of(r, a),
            query: |a| Query::kin_step_parents(a),
        },
        Row {
            sugar: "step_siblings_of",
            call: |r, a| step_siblings_of(r, a),
            query: |a| Query::kin_step_siblings(a),
        },
        Row {
            sugar: "step_children_of",
            call: |r, a| step_children_of(r, a),
            query: |a| Query::kin_step_children(a),
        },
    ]
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// Load one multi-`.kul` example directory as a project.
fn check_example_dir(dir: &str) -> CheckResult {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(workspace_root().join("examples").join(dir))
        .expect("read example directory")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("kul"))
        .collect();
    paths.sort();
    let inputs: Vec<InputFile> = paths
        .iter()
        .map(|path| {
            InputFile::new(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(path).expect("read example file"),
            )
        })
        .collect();
    kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs)
}

fn member_ids(members: &[KinMember<'_>]) -> Vec<String> {
    members
        .iter()
        .map(|member| member.person.id.name.clone())
        .collect()
}

#[test]
fn every_sugar_matches_its_catalogue_query() {
    // Two corpora, chosen for coverage of the affinal and step sets: 05 has
    // cousins and in-laws, 03 has a remarriage and therefore step relations.
    for dir in ["05-cousins-and-in-laws", "03-divorce-and-remarriage"] {
        let check = check_example_dir(dir);
        assert!(
            !check.has_errors(),
            "{dir} must check clean for this contract to mean anything"
        );
        let anchors: Vec<String> = check
            .resolved
            .persons()
            .map(|person| person.id.name.clone())
            .collect();
        assert!(!anchors.is_empty(), "{dir} declares no persons");

        for row in catalogue() {
            for anchor in &anchors {
                let query = (row.query)(anchor);
                let via_sugar = (row.call)(&check.resolved, anchor)
                    .unwrap_or_else(|err| panic!("{}({anchor}): {err:?}", row.sugar));
                let via_query = evaluate(&check.resolved, &query)
                    .unwrap_or_else(|err| panic!("{} as a Query({anchor}): {err:?}", row.sugar));
                assert_eq!(
                    member_ids(&via_sugar),
                    member_ids(&via_query),
                    "{} and its catalogue Query disagree about {anchor} in {dir}",
                    row.sugar,
                );
            }
        }
    }
}

#[test]
fn catalogue_patterns() {
    // The anchor is irrelevant to a pattern; it is stripped so the snapshot is
    // about the *question shape* and nothing else. `packages/preview/tests/
    // kin-sets.test.ts` reads this file, so its shape is a contract: a map of
    // sugar name → serialized `KinPattern`.
    let patterns: BTreeMap<&'static str, _> = catalogue()
        .into_iter()
        .map(|row| {
            let QuerySource::KinOf { pattern, .. } = (row.query)("anchor").source else {
                panic!("{} must build a kinOf source", row.sugar);
            };
            (row.sugar, pattern)
        })
        .collect();
    insta::assert_json_snapshot!(patterns);
}

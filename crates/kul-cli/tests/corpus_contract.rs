//! Full-corpus contract snapshot harness (issue #261, PRD 0005).
//!
//! The `kul query ... --format json` path is the epic's pinned
//! contract-serialization harness: the JSON it emits is byte-identical to the
//! `QueryEnvelope` the WASM surface returns (single-sourced in `kul-core`), so
//! snapshotting it over the **whole example corpus** pins the contract —
//! envelope shape plus relationship-descriptor serialization — against every
//! topology the engine must survive. Any future engine or serialization change
//! surfaces here as a reviewed snapshot diff rather than as silent drift on
//! consumers.
//!
//! This is a *contract* harness, not a *correctness* one: kinship correctness
//! is proven once at the core `query` seam (`crates/kul-core/tests/{kin,
//! resolve,filter}.rs`, ADR-0003). Here we only assert the bytes the adapter
//! serializes, over the real corpus.
//!
//! The suite is **table-driven** (`CASES`): adding an example later means
//! adding one row. Each row pins, for one anchor chosen to exercise that
//! example's specialty (the adoptee, a polygamous household's child, a
//! cross-file person, …):
//!
//! - the four unbounded/one-hop kin sets — `parents`, `children`, `siblings`,
//!   `ancestors`;
//! - one `rel <x> <y>` pair — including the disconnected pair in
//!   `07-disconnected-lineages`, which pins the `emptyReason` serialization;
//! - one filtered query — `persons --where "absent(died)" --sort born`.

use std::path::PathBuf;
use std::process::Command;

/// One corpus row: the project directory, a short snapshot label, the kin
/// anchor, and the two-anchor `rel` pair.
struct Case {
    /// Project directory under `examples/`.
    dir: &'static str,
    /// Short, filename-safe label the snapshots are named after.
    label: &'static str,
    /// The anchor whose kin sets are pinned — chosen to exercise the example's
    /// specialty.
    anchor: &'static str,
    /// The `rel <x> <y>` pair pinned for this example.
    rel: (&'static str, &'static str),
}

/// The corpus contract table. One row per `examples/` project; the anchor and
/// `rel` pair are chosen to exercise each example's specialty (adoption's
/// multi-parent adoptee, polygamy's half-siblings and co-wives, the
/// disconnected pair, the cross-file cousins, the century-spanning lineage, …).
const CASES: &[Case] = &[
    Case {
        dir: "01-nuclear-family",
        label: "01_nuclear",
        anchor: "akiko",
        rel: ("akiko", "kenji"),
    },
    Case {
        dir: "02-three-generations",
        label: "02_three_generations",
        anchor: "chidi",
        rel: ("chidi", "chinua"),
    },
    Case {
        dir: "03-divorce-and-remarriage",
        label: "03_remarriage",
        anchor: "linnea",
        rel: ("linnea", "oskar"),
    },
    Case {
        dir: "04-adoption-and-belonging",
        label: "04_adoption",
        anchor: "dalisay",
        rel: ("dalisay", "carlos"),
    },
    Case {
        dir: "05-cousins-and-in-laws",
        label: "05_cousins",
        anchor: "matteo",
        rel: ("matteo", "giulia"),
    },
    Case {
        dir: "06-polygamous-household",
        label: "06_polygamy",
        anchor: "yusuf",
        rel: ("aisha", "layla"),
    },
    Case {
        dir: "07-disconnected-lineages",
        label: "07_disconnected",
        anchor: "minjun",
        rel: ("minjun", "lucas"),
    },
    Case {
        dir: "08-multi-file-project",
        label: "08_multi_file",
        anchor: "mateo",
        rel: ("mateo", "lucia"),
    },
    Case {
        dir: "09-family-across-a-century",
        label: "09_century",
        anchor: "ife",
        rel: ("ife", "dele"),
    },
];

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .join("examples")
}

/// Run `kul query … --format json` in the example project and return stdout.
/// The JSON envelope is the contract answer for every outcome (a non-empty
/// set, an empty set, a disconnected pair), so the command exits 0 — a nonzero
/// exit here means a bad anchor id or a project that failed its checks, which
/// is a harness bug, not a contract to pin.
fn query_json(dir: &str, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_kul"))
        .current_dir(examples_dir().join(dir))
        .args(args)
        .args(["--format", "json"])
        .output()
        .unwrap_or_else(|e| panic!("run `kul query` in {dir}: {e}"));
    assert!(
        output.status.success(),
        "`kul query {}` in {dir} exited nonzero:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout is utf-8")
}

#[test]
fn corpus_contract_snapshots() {
    for case in CASES {
        // The four kin sets. `parents`/`children`/`siblings` are one-hop;
        // `ancestors` is unbounded (every ancestor, to any height).
        for relation in ["parents", "children", "siblings", "ancestors"] {
            let out = query_json(case.dir, &["query", "kin", case.anchor, relation]);
            insta::assert_snapshot!(format!("{}__kin_{relation}", case.label), out);
        }

        // One relationship-resolution pair — every way `x` and `y` are related,
        // or the honest `emptyReason` when there is none (the disconnected pin).
        let (x, y) = case.rel;
        let out = query_json(case.dir, &["query", "rel", x, y]);
        insta::assert_snapshot!(format!("{}__rel", case.label), out);

        // One filtered query, identical across the corpus: persons with no
        // recorded death date, ordered by birth — the deterministic sort keeps
        // the snapshot stable.
        let out = query_json(
            case.dir,
            &[
                "query",
                "persons",
                "--where",
                "absent(died)",
                "--sort",
                "born",
            ],
        );
        insta::assert_snapshot!(format!("{}__persons_absent_died", case.label), out);
    }
}

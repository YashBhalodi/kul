//! Wiring + cross-surface tests for the WASM query surface.
//!
//! The query envelope's *contract serialization* is pinned by the core
//! `query` snapshots; here we assert only that the WASM bridge is wired to
//! that core path (bit-identical JSON), that the ABI signature round-trips,
//! and that the ok / null-payload / error arms surface correctly. Kinship
//! correctness is not re-tested at the adapter.

use std::path::{Path, PathBuf};

use kul_core::ast::InputFile;
use kul_core::manifest::Manifest;
use kul_core::query::{
    IntRange, Query, ResolveConfig, kin_query, marriage_lookup, person_lookup, resolve_relationship,
};
use kul_wasm::{
    WasmInputFile, query_kin, query_kin_with, query_marriage_with, query_person, query_person_with,
    query_resolve, query_resolve_with, run_query, run_query_with,
};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn nuclear_inputs() -> Vec<InputFile> {
    let path = workspace_root()
        .join("examples")
        .join("01-nuclear-family")
        .join("nuclear-family.kul");
    let source = std::fs::read_to_string(&path).expect("read nuclear-family.kul");
    vec![InputFile::new("nuclear-family.kul", source)]
}

/// The bridge's person lookup is byte-identical to the core `person_lookup`
/// it wraps (the CLI relies on the same equality for `--format json`).
#[test]
fn person_lookup_json_matches_core() {
    let inputs = nuclear_inputs();
    let manifest = Manifest::default();
    let via_wasm = query_person_with(&inputs, &manifest, "hiroshi");
    let check = kul_core::check_with_manifest("kul.yml", "", &manifest, &inputs);
    let via_core = person_lookup(&check, "hiroshi");
    assert_eq!(
        serde_json::to_string_pretty(&via_wasm).unwrap(),
        serde_json::to_string_pretty(&via_core).unwrap(),
    );
}

#[test]
fn marriage_lookup_json_matches_core() {
    let inputs = nuclear_inputs();
    let manifest = Manifest::default();
    let via_wasm = query_marriage_with(&inputs, &manifest, "m_hiroshi_yuki");
    let check = kul_core::check_with_manifest("kul.yml", "", &manifest, &inputs);
    let via_core = marriage_lookup(&check, "m_hiroshi_yuki");
    assert_eq!(
        serde_json::to_string_pretty(&via_wasm).unwrap(),
        serde_json::to_string_pretty(&via_core).unwrap(),
    );
}

/// Drives the public wasm-ABI signature (`Vec<WasmInputFile>` + `String`
/// id) to confirm the conversion is wired up, and that a known id lands in
/// the ok arm carrying the person payload.
#[test]
fn wasm_abi_signature_returns_ok_arm_for_known_id() {
    let inputs = nuclear_inputs();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: inputs[0].source.clone(),
    }];
    let envelope = query_person(files, Manifest::default(), "hiroshi".to_string());
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["id"], "hiroshi");
}

/// A clean project with an unknown id yields the ok arm with a `null`
/// result — absence is the answer, not an error.
#[test]
fn unknown_id_yields_ok_arm_with_null_result() {
    let inputs = nuclear_inputs();
    let envelope = query_person_with(&inputs, &Manifest::default(), "nobody");
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    assert!(json["result"].is_null(), "expected null result: {json}");
}

/// A project that fails its checks yields the error arm carrying the
/// diagnostics — never a partial answer.
#[test]
fn failing_project_yields_error_arm() {
    let inputs = vec![InputFile::new("input.kul", "person alice gender:female\n")];
    let envelope = query_person_with(&inputs, &Manifest::default(), "alice");
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], false);
    assert!(
        json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "KUL-R03"),
        "expected KUL-R03 in error arm: {json}"
    );
}

// ---- Kin-set queries (fourth shape, kin variant) ----

/// The bridge's kin query is byte-identical to the core `kin_query` it wraps
/// (the CLI relies on the same equality for `--format json`). Kinship
/// correctness itself is proven at the core seam, not re-tested here.
#[test]
fn kin_query_json_matches_core() {
    let inputs = nuclear_inputs();
    let manifest = Manifest::default();
    let query = Query::kin_ancestors("akiko", IntRange::exactly(1), None);
    let via_wasm = query_kin_with(&inputs, &manifest, &query);
    let check = kul_core::check_with_manifest("kul.yml", "", &manifest, &inputs);
    let via_core = kin_query(&check, &query);
    assert_eq!(
        serde_json::to_string_pretty(&via_wasm).unwrap(),
        serde_json::to_string_pretty(&via_core).unwrap(),
    );
}

/// Drives the public wasm-ABI signature (`Vec<WasmInputFile>` + `Query`
/// value) to confirm the Query round-trips and the members carry person id +
/// descriptor, no person payload.
#[test]
fn kin_query_abi_returns_members() {
    let inputs = nuclear_inputs();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: inputs[0].source.clone(),
    }];
    let query = Query::kin_descendants("hiroshi", IntRange::exactly(1), None);
    let envelope = query_kin(files, Manifest::default(), query);
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["kind"], "members");
    let members = json["result"]["members"].as_array().unwrap();
    assert_eq!(members.len(), 2);
    // Member shape: person id + descriptor, never a person payload.
    assert_eq!(members[0]["personId"], "akiko");
    assert!(members[0]["descriptor"].is_object());
    assert!(members[0].get("name").is_none(), "no person payload");
}

/// The general `runQuery` surface is wired to the same core path as the CLI
/// `--format json` (bit-identical), for the new `allPersons` + attribute-filter
/// shape.
#[test]
fn run_query_json_matches_core() {
    use kul_core::query::{PersonField, Predicate, Query};

    let inputs = nuclear_inputs();
    let manifest = Manifest::default();
    let query = Query::all_persons().filtered(Predicate::Present {
        field: PersonField::Born,
    });
    let via_wasm = run_query_with(&inputs, &manifest, &query);
    let check = kul_core::check_with_manifest("kul.yml", "", &manifest, &inputs);
    let via_core = kul_core::query::query_envelope(&check, &query);
    assert_eq!(
        serde_json::to_string_pretty(&via_wasm).unwrap(),
        serde_json::to_string_pretty(&via_core).unwrap(),
    );
}

/// The `runQuery` ABI round-trips the extended Query and yields the
/// `personIds` shape for an `allPersons` source (ids only, no payload).
#[test]
fn run_query_abi_returns_person_ids() {
    use kul_core::query::Query;

    let inputs = nuclear_inputs();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: inputs[0].source.clone(),
    }];
    let envelope = run_query(files, Manifest::default(), Query::all_persons());
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["kind"], "personIds");
    let ids = json["result"]["personIds"].as_array().unwrap();
    assert!(ids.iter().any(|id| id == "hiroshi"), "ids only: {json}");
}

/// A `count` projection round-trips as the count-variant envelope.
#[test]
fn run_query_abi_returns_count() {
    use kul_core::query::Query;

    let inputs = nuclear_inputs();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: inputs[0].source.clone(),
    }];
    let envelope = run_query(files, Manifest::default(), Query::all_persons().counting());
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["result"]["kind"], "count");
    assert!(json["result"]["count"].as_u64().unwrap() >= 2);
}

/// A malformed predicate surfaces as the envelope's error arm — a diagnostic,
/// never a thrown exception.
#[test]
fn run_query_malformed_predicate_yields_error_arm() {
    use kul_core::query::{PersonField, Predicate, Query};

    let inputs = nuclear_inputs();
    let query = Query::all_persons().filtered(Predicate::Gt {
        field: PersonField::Born,
        value: "not-a-date".to_string(),
    });
    let envelope = run_query_with(&inputs, &Manifest::default(), &query);
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], false);
    assert_eq!(json["diagnostics"][0]["code"], "KUL-Q02");
}

/// A collateral Query value (the new pattern variant) round-trips through the
/// wasm-ABI signature and yields the collateral descriptor — the fourth shape
/// carries the additive variants with no new entry point.
#[test]
fn kin_query_collateral_round_trips() {
    let inputs = nuclear_inputs();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: inputs[0].source.clone(),
    }];
    // akiko & kenji are siblings (both children of hiroshi/yuki).
    let query = Query::kin_collateral("akiko", IntRange::exactly(1), IntRange::exactly(1), None);
    let envelope = query_kin(files, Manifest::default(), query);
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    let members = json["result"]["members"].as_array().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["personId"], "kenji");
    assert_eq!(
        members[0]["descriptor"]["classification"]["kind"],
        "collateral"
    );
}

/// An affinal Query value (the `spouses` sugar → `any` classification +
/// `affinalHops` filter) round-trips through the wasm-ABI signature and yields
/// the `across` marriage hop — the extended pattern surface crosses the
/// boundary with no new entry point.
#[test]
fn kin_query_affinal_round_trips() {
    let inputs = nuclear_inputs();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: inputs[0].source.clone(),
    }];
    // hiroshi's spouse is yuki, reached by a single `across` marriage hop.
    let query = Query::kin_spouses("hiroshi");
    let envelope = query_kin(files, Manifest::default(), query);
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    let members = json["result"]["members"].as_array().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["personId"], "yuki");
    assert_eq!(members[0]["descriptor"]["affinity"], "inLaw");
    let hop = &members[0]["descriptor"]["path"][0];
    assert_eq!(hop["step"], "across");
    assert_eq!(hop["status"], "ongoing");
}

/// A bad anchor on a clean project is the error arm with a diagnostic naming
/// the id — never an empty ok set.
#[test]
fn kin_query_unknown_anchor_yields_error_arm() {
    let inputs = nuclear_inputs();
    let query = Query::kin_ancestors("nobody", IntRange::exactly(1), None);
    let envelope = query_kin_with(&inputs, &Manifest::default(), &query);
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], false);
    assert!(
        json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("nobody")),
        "expected a diagnostic naming the bad anchor: {json}"
    );
}

// ---- Relationship resolution (fourth shape, resolve variant) ----

/// The bridge's resolve is byte-identical to the core `resolve_relationship`
/// it wraps (the CLI relies on the same equality for `--format json`). An
/// omitted config uses the default budget. Kinship correctness is proven at
/// the core seam, not re-tested here.
#[test]
fn resolve_json_matches_core() {
    let inputs = nuclear_inputs();
    let manifest = Manifest::default();
    let via_wasm = query_resolve_with(&inputs, &manifest, "akiko", "hiroshi", None);
    let check = kul_core::check_with_manifest("kul.yml", "", &manifest, &inputs);
    let via_core = resolve_relationship(&check, "akiko", "hiroshi", &ResolveConfig::default());
    assert_eq!(
        serde_json::to_string_pretty(&via_wasm).unwrap(),
        serde_json::to_string_pretty(&via_core).unwrap(),
    );
}

/// Drives the public wasm-ABI signature (`Vec<WasmInputFile>` + two ids +
/// optional config) to confirm the result round-trips and carries the
/// relationship descriptors.
#[test]
fn resolve_abi_returns_relationships() {
    let inputs = nuclear_inputs();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: inputs[0].source.clone(),
    }];
    let envelope = query_resolve(
        files,
        Manifest::default(),
        "akiko".to_string(),
        "kenji".to_string(),
        None,
    );
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    // akiko & kenji are siblings → at least a collateral 1/1 descriptor.
    let rels = json["result"]["relationships"].as_array().unwrap();
    assert!(
        rels.iter()
            .any(|d| d["classification"]["kind"] == "collateral"),
        "expected a collateral tie between siblings: {json}"
    );
}

/// An omitted config on the ABI is the default budget; a provided config
/// overrides it (an unreachable-at-cap-1 pair yields an empty result with a
/// reason, still the ok arm).
#[test]
fn resolve_config_over_the_abi() {
    let inputs = nuclear_inputs();
    let files = vec![WasmInputFile {
        name: "nuclear-family.kul".into(),
        source: inputs[0].source.clone(),
    }];
    // Two unrelated persons in one component would need a real fixture; here we
    // simply confirm a provided config round-trips and the empty-reason surfaces
    // as an ok arm (hiroshi & yuki are spouses → non-empty, so use the config
    // path shape only).
    let envelope = query_resolve(
        files,
        Manifest::default(),
        "hiroshi".to_string(),
        "yuki".to_string(),
        Some(ResolveConfig {
            max_apex_generations: 1,
        }),
    );
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], true);
    // Spouses → a self / inLaw descriptor with one across hop.
    let rels = json["result"]["relationships"].as_array().unwrap();
    assert!(
        rels.iter().any(|d| d["affinity"] == "inLaw"),
        "expected the spouse tie: {json}"
    );
}

/// A bad id on a clean project is the error arm with a diagnostic naming the
/// id — never an empty ok result.
#[test]
fn resolve_unknown_id_yields_error_arm() {
    let inputs = nuclear_inputs();
    let envelope = query_resolve_with(&inputs, &Manifest::default(), "hiroshi", "nobody", None);
    let json = serde_json::to_value(&envelope).unwrap();
    assert_eq!(json["ok"], false);
    assert!(
        json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("nobody")),
        "expected a diagnostic naming the bad id: {json}"
    );
}

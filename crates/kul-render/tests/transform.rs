//! Fabricated-envelope unit snapshots covering edge cases the
//! `examples/` corpus doesn't naturally surface.

use kul_core::export::{
    ExportEnvelope, ExportedDate, ExportedDiagnostic, ExportedGraph, ExportedMarriage,
    ExportedParenthoodLink, ExportedPerson, FailureEnvelope, GraphPayload, ParenthoodLinkKind,
    SuccessEnvelope,
};
use kul_render::transform;

const SCHEMA: u32 = 1;
const KUL: &str = "0.1";

fn year(y: u32) -> ExportedDate {
    ExportedDate {
        value: format!("{y:04}"),
        precision: "year",
        circa: false,
    }
}

fn person(id: &str, name: &str, gender: &'static str) -> ExportedPerson {
    ExportedPerson {
        id: id.to_string(),
        name: name.to_string(),
        family: None,
        given: None,
        gender,
        born: None,
        died: None,
        span: None,
    }
}

fn marriage(id: &str, a: &str, b: &str, start: u32) -> ExportedMarriage {
    ExportedMarriage {
        id: id.to_string(),
        spouses: [a.to_string(), b.to_string()],
        start: Some(year(start)),
        end: None,
        end_reason: None,
        span: None,
    }
}

fn bio(child: &str, marriage_id: &str) -> ExportedParenthoodLink {
    ExportedParenthoodLink {
        marriage_id: marriage_id.to_string(),
        child_id: child.to_string(),
        kind: ParenthoodLinkKind::Biological,
        start: None,
        end: None,
        span: None,
    }
}

fn adoption(child: &str, marriage_id: &str, start_y: u32) -> ExportedParenthoodLink {
    ExportedParenthoodLink {
        marriage_id: marriage_id.to_string(),
        child_id: child.to_string(),
        kind: ParenthoodLinkKind::Adoptive,
        start: Some(year(start_y)),
        end: None,
        span: None,
    }
}

fn success(graph: ExportedGraph) -> ExportEnvelope {
    ExportEnvelope::Success(SuccessEnvelope {
        ok: true,
        schema: SCHEMA,
        kul: KUL.to_string(),
        graph: GraphPayload::Native(graph),
    })
}

fn render_pretty(envelope: &ExportEnvelope) -> String {
    let shape = transform(envelope);
    serde_json::to_string_pretty(&shape).expect("serialize render shape")
}

/// Absorb rule: Bob's birth family nests at his joining slot in
/// `m_alice_bob` (cross-component, so recursion does not terminate).
#[test]
fn p6_joining_spouse_birth_family_nests_at_connection_point() {
    let envelope = success(ExportedGraph {
        persons: vec![
            person("alice", "Alice", "female"),
            person("bob", "Bob", "male"),
            person("bob_dad", "Bob's Dad", "male"),
            person("bob_mom", "Bob's Mom", "female"),
            person("kid", "Kid", "other"),
        ],
        marriages: vec![
            marriage("m_bob_parents", "bob_dad", "bob_mom", 1945),
            marriage("m_alice_bob", "alice", "bob", 1972),
        ],
        parenthood_links: vec![bio("bob", "m_bob_parents"), bio("kid", "m_alice_bob")],
    });
    insta::assert_snapshot!(render_pretty(&envelope));
}

/// Three adoptions: most-recent is canonical, earlier two each get a
/// child-ghost at their adoption bar.
#[test]
fn p16_three_adoptions_emit_one_canonical_and_two_past_ghosts() {
    let envelope = success(ExportedGraph {
        persons: vec![
            person("a1", "A1", "female"),
            person("a2", "A2", "male"),
            person("b1", "B1", "female"),
            person("b2", "B2", "male"),
            person("c1", "C1", "female"),
            person("c2", "C2", "male"),
            person("kid", "Kid", "other"),
        ],
        marriages: vec![
            marriage("m_a", "a1", "a2", 1970),
            marriage("m_b", "b1", "b2", 1975),
            marriage("m_c", "c1", "c2", 1978),
        ],
        parenthood_links: vec![
            adoption("kid", "m_a", 1981),
            adoption("kid", "m_b", 1985),
            adoption("kid", "m_c", 1992),
        ],
    });
    insta::assert_snapshot!(render_pretty(&envelope));
}

/// Pure-host polygamy collapses onto one canonical card (ADR-0017):
/// Devraj's two un-ended hosted marriages share one root PersonCard,
/// each co-spouse canonical at her own bar.
#[test]
fn p8_pure_host_polygamy_shares_canonical_anchor() {
    let envelope = success(ExportedGraph {
        persons: vec![
            person("devraj", "Devraj", "male"),
            person("meera", "Meera", "female"),
            person("alice", "Alice", "female"),
        ],
        marriages: vec![
            marriage("m_devraj_meera", "devraj", "meera", 1990),
            marriage("m_devraj_alice", "devraj", "alice", 1995),
        ],
        parenthood_links: vec![],
    });
    let shape = transform(&envelope);
    let success = shape.as_success().expect("success envelope");

    assert_eq!(
        success.components.len(),
        1,
        "pure-host polygamy collapses onto one component, got {}: {}",
        success.components.len(),
        serde_json::to_string_pretty(&shape).unwrap()
    );
    let component = &success.components[0];
    let root = match &component.kind {
        kul_render::ComponentKind::FamilyTree { root } => root,
        kul_render::ComponentKind::OrphanPerson { .. } => {
            panic!("expected FamilyTree, got OrphanPerson")
        }
    };
    assert_eq!(root.slot.person_id, "devraj");
    assert!(
        matches!(root.slot.kind, kul_render::SlotKind::Canonical),
        "root PersonCard should be Devraj's canonical card, got: {:?}",
        root.slot.kind
    );
    assert_eq!(
        root.hosted_marriages.len(),
        2,
        "Devraj's single canonical card should host both un-ended bars",
    );
    let marriage_ids: Vec<&str> = root
        .hosted_marriages
        .iter()
        .map(|m| m.bar.marriage_id.as_str())
        .collect();
    assert_eq!(marriage_ids, vec!["m_devraj_meera", "m_devraj_alice"]);
    for branch in &root.hosted_marriages {
        assert!(
            !branch.bar.ended,
            "neither bar is ended (both are current intimacies)",
        );
        assert!(
            matches!(
                branch.bar.joining_slot.kind,
                kul_render::SlotKind::Canonical
            ),
            "co-spouse joining slot is canonical (no ghost for current intimacy): {:?}",
            branch.bar.joining_slot.kind,
        );
    }

    // Lock the full shape so regressions surface as snapshot diffs.
    insta::assert_snapshot!(render_pretty(&envelope));
}

/// Joining spouse of an ended marriage with no birth family becomes
/// an orphan component, rendering as a ghost at the past bar.
#[test]
fn p8_joining_spouse_of_ended_marriage_becomes_orphan_component() {
    let envelope = success(ExportedGraph {
        persons: vec![
            person("alice", "Alice", "female"),
            person("bob", "Bob", "male"),
            person("carol", "Carol", "female"),
        ],
        marriages: vec![ExportedMarriage {
            id: "m_alice_bob".to_string(),
            spouses: ["alice".to_string(), "bob".to_string()],
            start: Some(year(1972)),
            end: Some(year(1990)),
            end_reason: Some("divorce".to_string()),
            span: None,
        }],
        parenthood_links: vec![bio("carol", "m_alice_bob")],
    });
    insta::assert_snapshot!(render_pretty(&envelope));
}

/// Empty document yields empty `components`/`edges`, no panic.
#[test]
fn empty_document_yields_empty_shape() {
    let envelope = success(ExportedGraph {
        persons: vec![],
        marriages: vec![],
        parenthood_links: vec![],
    });
    insta::assert_snapshot!(render_pretty(&envelope));
}

/// Regression: #207 — pre-change, the absorb rule's union-find collapsed
/// these two rootless host marriages into one component, dropping one of
/// the qualifying root marriages and panicking downstream layout. The
/// projection is exactly the minimal reproducer from the issue body,
/// fabricated at the envelope layer. Post-change each host lineage is
/// its own component, in source order, with `e` canonical at her
/// joining slot in `m_x_e` and a `PastBirth` ghost at her bio bar `m_c_d`.
#[test]
fn multi_rootless_host_lineage_n2() {
    let envelope = success(ExportedGraph {
        persons: vec![
            person("a", "A", "male"),
            person("b", "B", "female"),
            person("c", "C", "male"),
            person("d", "D", "female"),
            person("e", "E", "female"),
            person("x", "X", "male"),
            person("f", "F", "male"),
        ],
        marriages: vec![
            marriage("m_a_b", "a", "b", 1900),
            marriage("m_c_d", "c", "d", 1925),
            marriage("m_x_e", "x", "e", 1950),
        ],
        parenthood_links: vec![bio("c", "m_a_b"), bio("e", "m_c_d"), bio("f", "m_x_e")],
    });
    let shape = transform(&envelope);
    let success = shape.as_success().expect("success envelope");

    assert_eq!(
        success.components.len(),
        2,
        "two rootless host marriages must render as two independent components, got {}: {}",
        success.components.len(),
        serde_json::to_string_pretty(&shape).unwrap()
    );

    // Components ordered by source order: `m_a_b` lineage first, then `m_x_e`.
    let roots: Vec<&str> = success
        .components
        .iter()
        .map(|c| match &c.kind {
            kul_render::ComponentKind::FamilyTree { root } => root.slot.person_id.as_str(),
            kul_render::ComponentKind::OrphanPerson { card } => card.person_id.as_str(),
        })
        .collect();
    assert_eq!(roots, vec!["a", "x"]);

    // `e` is canonical at the joining slot of `m_x_e` and emits a
    // `PastBirth` ghost child of `m_c_d` so the bio edge terminates locally.
    let json = render_pretty(&envelope);
    assert!(
        json.contains("\"reason\": \"pastBirth\""),
        "expected a PastBirth ghost in the render shape: {json}"
    );

    insta::assert_snapshot!(json);
}

/// Regression: #207 — extends the N=2 case with a third rootless host
/// lineage (`m_p_f`) joined by descent through `f`. Three independent
/// components must render in source order; pre-change this would have
/// collapsed into a single component with two qualifying roots dropped.
#[test]
fn multi_rootless_host_lineage_n3() {
    let envelope = success(ExportedGraph {
        persons: vec![
            person("a", "A", "male"),
            person("b", "B", "female"),
            person("c", "C", "male"),
            person("d", "D", "female"),
            person("e", "E", "female"),
            person("x", "X", "male"),
            person("f", "F", "male"),
            person("p", "P", "female"),
            person("g", "G", "other"),
        ],
        marriages: vec![
            marriage("m_a_b", "a", "b", 1900),
            marriage("m_c_d", "c", "d", 1925),
            marriage("m_x_e", "x", "e", 1950),
            marriage("m_p_f", "p", "f", 1975),
        ],
        parenthood_links: vec![
            bio("c", "m_a_b"),
            bio("e", "m_c_d"),
            bio("f", "m_x_e"),
            bio("g", "m_p_f"),
        ],
    });
    let shape = transform(&envelope);
    let success = shape.as_success().expect("success envelope");

    assert_eq!(
        success.components.len(),
        3,
        "three rootless host marriages must render as three independent components, got {}: {}",
        success.components.len(),
        serde_json::to_string_pretty(&shape).unwrap()
    );

    let roots: Vec<&str> = success
        .components
        .iter()
        .map(|c| match &c.kind {
            kul_render::ComponentKind::FamilyTree { root } => root.slot.person_id.as_str(),
            kul_render::ComponentKind::OrphanPerson { card } => card.person_id.as_str(),
        })
        .collect();
    assert_eq!(roots, vec!["a", "x", "p"]);

    insta::assert_snapshot!(render_pretty(&envelope));
}

/// Failure envelopes pass through verbatim, carrying the same diagnostics.
#[test]
fn failure_envelope_passes_through_with_diagnostics() {
    let envelope = ExportEnvelope::Failure(FailureEnvelope {
        ok: false,
        diagnostics: vec![ExportedDiagnostic {
            code: "KUL-R02".to_string(),
            severity: "error",
            message: "unresolved reference `ghost`".to_string(),
            primary: None,
            related: vec![],
        }],
    });
    insta::assert_snapshot!(render_pretty(&envelope));
}

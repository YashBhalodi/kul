//! Regression tests for layout-adapter bugs. Each test crafts the
//! minimal `.kul` source that previously panicked or mis-positioned,
//! then asserts the pipeline completes with the expected shape.

use kul_core::ast::InputFile;
use kul_core::diagnostic::Severity;
use kul_core::manifest::Manifest;
use kul_layout::{EdgeKind, LayoutConfig, layout};
use kul_render::compute;

fn layout_from_source(source: &str) -> kul_layout::PositionedShape {
    let inputs = vec![InputFile::new(
        "regression.kul".to_owned(),
        source.to_owned(),
    )];
    let check = kul_core::check_with_manifest("kul.yml", "", &Manifest::default(), &inputs);
    assert!(
        check
            .diagnostics
            .iter()
            .all(|d| !matches!(d.severity, Severity::Error)),
        "regression source must check clean; diagnostics: {:?}",
        check.diagnostics
    );
    let shape = compute(&check);
    layout(&shape, &LayoutConfig::default())
}

/// Reproducer for #208: a polygamy hub whose only child of one marriage
/// has its canonical_location pointing elsewhere (the child marries as
/// the joining spouse of a separate marriage) and is adopted by exactly
/// one of the hub's marriages. `build_children` emits no card for that
/// marriage, but the adoption render edge still references it. Before
/// the fix, the `PolygamyHub` branch skipped the `bar_centers` insert
/// when `child_roots` was empty, so `route_edges` panicked.
#[test]
fn polygamy_hub_with_cross_component_adoption_child() {
    let source = r#"person dad name:"Dad" gender:male
person mom1 name:"Mom1" gender:female
person mom2 name:"Mom2" gender:female

marriage m_dad_mom1 dad mom1 start:1970
marriage m_dad_mom2 dad mom2 start:1980

person kid name:"Kid" gender:male
  adoption m_dad_mom2 start:1985

person ks name:"KS" gender:female
marriage m_kid ks kid start:2010
"#;

    let positioned = layout_from_source(source);

    // Before the fix, `layout_from_source` panicked on the missing
    // `bar_centers` entry for `m_dad_mom2`. Reaching here means the
    // panic is gone. Beyond that, every positioned marriage edge from
    // the source must be routed.
    let marriage_ids: Vec<&str> = positioned
        .edges
        .iter()
        .filter_map(|e| match &e.kind {
            EdgeKind::Marriage { .. } => Some(e.marriage_id.as_str()),
            _ => None,
        })
        .collect();
    // `m_dad_mom2` must have a marriage edge — that's the polygamy-hub
    // marriage whose missing `bar_centers` entry caused the panic.
    for expected in ["m_dad_mom1", "m_dad_mom2"] {
        assert!(
            marriage_ids.contains(&expected),
            "expected marriage edge for {expected}, got {marriage_ids:?}"
        );
    }
    for edge in &positioned.edges {
        assert!(
            edge.points.len() >= 2,
            "edge {edge:?} must have a routed polyline"
        );
    }
}

/// Regression: #207 — pre-change, the absorb rule's union-find merged
/// these two rootless host marriages into one component and dropped
/// one of the qualifying root marriages, panicking at the missing
/// `bar_centers` entry. Post-change, they render as two independent
/// components in source order. This is the exact minimal reproducer
/// from the issue body.
#[test]
fn multi_rootless_host_lineage_n2() {
    let source = r#"person a name:"A" gender:male
person b name:"B" gender:female
marriage m_a_b a b

person c name:"C" gender:male
  birth m_a_b
person d name:"D" gender:female
marriage m_c_d c d

person e name:"E" gender:female
  birth m_c_d

person x name:"X" gender:male
marriage m_x_e x e

person f name:"F" gender:male
  birth m_x_e
"#;

    let positioned = layout_from_source(source);

    let marriage_ids: Vec<&str> = positioned
        .edges
        .iter()
        .filter_map(|e| match &e.kind {
            EdgeKind::Marriage { .. } => Some(e.marriage_id.as_str()),
            _ => None,
        })
        .collect();
    for expected in ["m_a_b", "m_c_d", "m_x_e"] {
        assert!(
            marriage_ids.contains(&expected),
            "expected marriage edge for {expected}, got {marriage_ids:?}"
        );
    }
    for edge in &positioned.edges {
        assert!(
            edge.points.len() >= 2,
            "edge {edge:?} must have a routed polyline"
        );
    }
}

/// Regression: #249 — a childless polygamy hub (every marriage has no
/// children) places its co-spouse cards one row below the hub, but those
/// cards are not walker nodes and, with no child forest below to raise
/// `max_gen`, the canvas height was computed as if the hub were the last
/// row. The co-spouse cards then extended past the SVG `viewBox` and
/// rendered clipped. Every placed card's bottom edge must fit within the
/// canvas height.
#[test]
fn childless_polygamy_hub_cospouse_cards_fit_within_canvas() {
    let source = r#"person dad name:"Dad" gender:male
person mom1 name:"Mom1" gender:female
person mom2 name:"Mom2" gender:female

marriage m_dad_mom1 dad mom1 start:1970
marriage m_dad_mom2 dad mom2 start:1980
"#;

    let positioned = layout_from_source(source);

    // The two co-spouse cards (mom1, mom2) sit a full row below the hub.
    // Every card bottom must fall within the canvas height, or it clips
    // in the SVG viewBox.
    for card in &positioned.cards {
        assert!(
            card.y + card.height <= positioned.height,
            "card {} bottom {} exceeds canvas height {}",
            card.person_id,
            card.y + card.height,
            positioned.height
        );
    }
    // Sanity: the co-spouse cards really are the bottom-most row, so the
    // assertion above is exercising the fix rather than passing vacuously.
    let hub_bottom = positioned
        .cards
        .iter()
        .find(|c| c.person_id == "dad")
        .map(|c| c.y + c.height)
        .expect("hub card present");
    let cospouse_bottom = positioned
        .cards
        .iter()
        .filter(|c| c.person_id == "mom1" || c.person_id == "mom2")
        .map(|c| c.y + c.height)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        cospouse_bottom > hub_bottom,
        "co-spouse cards ({cospouse_bottom}) must sit below the hub ({hub_bottom})"
    );
}

/// Regression: #207 — extends the N=2 case with a third rootless host
/// lineage joined by descent. Three independent host-lineage components
/// should position cleanly in source order with no panic.
#[test]
fn multi_rootless_host_lineage_n3() {
    let source = r#"person a name:"A" gender:male
person b name:"B" gender:female
marriage m_a_b a b

person c name:"C" gender:male
  birth m_a_b
person d name:"D" gender:female
marriage m_c_d c d

person e name:"E" gender:female
  birth m_c_d

person x name:"X" gender:male
marriage m_x_e x e

person f name:"F" gender:male
  birth m_x_e

person p name:"P" gender:female
marriage m_p_f p f

person g name:"G" gender:other
  birth m_p_f
"#;

    let positioned = layout_from_source(source);

    let marriage_ids: Vec<&str> = positioned
        .edges
        .iter()
        .filter_map(|e| match &e.kind {
            EdgeKind::Marriage { .. } => Some(e.marriage_id.as_str()),
            _ => None,
        })
        .collect();
    for expected in ["m_a_b", "m_c_d", "m_x_e", "m_p_f"] {
        assert!(
            marriage_ids.contains(&expected),
            "expected marriage edge for {expected}, got {marriage_ids:?}"
        );
    }
    for edge in &positioned.edges {
        assert!(
            edge.points.len() >= 2,
            "edge {edge:?} must have a routed polyline"
        );
    }
}

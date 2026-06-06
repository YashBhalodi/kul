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

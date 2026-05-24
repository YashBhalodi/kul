//! Visual-vocabulary tests: pin the CSS-class seam decisions
//! [ADR-0019](../../../docs/adr/0019-kul-svg-crate-boundary.md) commits
//! to (canonical vs ghost cards, ended bars, birth vs adoption edges).
//!
//! These tests construct `PositionedShape` values by hand so the
//! emitter is exercised independent of the kul-render / kul-layout
//! pipeline.

use kul_layout::{
    EdgeKind, EdgeRouting, PositionedBar, PositionedCard, PositionedEdge, PositionedShape, SlotKind,
};
use kul_render::GhostReason;
use kul_svg::{ThemeConfig, render};

fn empty_shape() -> PositionedShape {
    PositionedShape {
        width: 200.0,
        height: 200.0,
        cards: Vec::new(),
        bars: Vec::new(),
        edges: Vec::new(),
        fan_connectors: Vec::new(),
    }
}

#[test]
fn canonical_card_emits_canonical_class_no_ghost_attributes() {
    let mut shape = empty_shape();
    shape.cards.push(PositionedCard {
        person_id: "alice".to_owned(),
        kind: SlotKind::Canonical,
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 50.0,
        name: "Alice".to_owned(),
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"class="kul-card kul-card--canonical""#),
        "expected canonical class, got: {svg}"
    );
    assert!(
        !svg.contains("stroke-dasharray"),
        "canonical card must not ship a stroke-dasharray: {svg}"
    );
    assert!(
        !svg.contains("kul-ghost-badge"),
        "canonical card must not emit the ↺ badge: {svg}"
    );
}

#[test]
fn past_marriage_ghost_card_emits_ghost_class_dasharray_and_badge() {
    let mut shape = empty_shape();
    shape.cards.push(PositionedCard {
        person_id: "bob".to_owned(),
        kind: SlotKind::Ghost {
            reason: GhostReason::PastMarriage,
        },
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 50.0,
        name: "Bob".to_owned(),
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"class="kul-card kul-card--ghost""#),
        "expected ghost class: {svg}"
    );
    assert!(
        svg.contains(r#"stroke-dasharray="3 2""#),
        "expected ghost dasharray: {svg}"
    );
    assert!(
        svg.contains("kul-ghost-badge"),
        "expected ↺ badge for past-marriage ghost: {svg}"
    );
    assert!(svg.contains("↺"), "expected ↺ glyph: {svg}");
}

#[test]
fn past_adoption_ghost_card_emits_ghost_class_and_badge() {
    let mut shape = empty_shape();
    shape.cards.push(PositionedCard {
        person_id: "alex".to_owned(),
        kind: SlotKind::Ghost {
            reason: GhostReason::PastAdoption,
        },
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 50.0,
        name: "Alex".to_owned(),
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(svg.contains("kul-card--ghost"));
    assert!(svg.contains("kul-ghost-badge"));
    assert!(svg.contains(r#"stroke-dasharray="3 2""#));
}

#[test]
fn ended_marriage_bar_emits_ended_class() {
    let mut shape = empty_shape();
    shape.bars.push(PositionedBar {
        marriage_id: "m1".to_owned(),
        x: 50.0,
        y: 40.0,
        width: 20.0,
        height: 10.0,
        ended: true,
    });
    shape.bars.push(PositionedBar {
        marriage_id: "m2".to_owned(),
        x: 100.0,
        y: 40.0,
        width: 20.0,
        height: 10.0,
        ended: false,
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"class="kul-bar kul-bar--ended""#),
        "expected ended bar class: {svg}"
    );
    assert!(
        svg.contains(r#"class="kul-bar""#),
        "expected unended bar class: {svg}"
    );
}

#[test]
fn birth_edge_emits_birth_class_no_dasharray() {
    let mut shape = empty_shape();
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Birth,
        routing: EdgeRouting::InTree,
        child_id: "carol".to_owned(),
        marriage_id: "m1".to_owned(),
        points: vec![(0.0, 0.0), (50.0, 50.0)],
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains("kul-edge--birth"),
        "expected birth class: {svg}"
    );
    assert!(
        !svg.contains("stroke-dasharray"),
        "birth edge must not ship a dasharray: {svg}"
    );
}

#[test]
fn adoption_edge_emits_adoption_class_with_dasharray() {
    let mut shape = empty_shape();
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Adoption,
        routing: EdgeRouting::InTree,
        child_id: "ravi".to_owned(),
        marriage_id: "m1".to_owned(),
        points: vec![(0.0, 0.0), (50.0, 50.0)],
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains("kul-edge--adoption"),
        "expected adoption class: {svg}"
    );
    assert!(
        svg.contains(r#"stroke-dasharray="6 4""#),
        "adoption edge must ship a dasharray: {svg}"
    );
}

#[test]
fn emitted_svg_has_no_inline_fill_or_stroke_color() {
    // Theme-agnostic invariant: ADR-0019. Construct a shape with one
    // of each primitive and confirm none of the emitted attributes
    // assert a colour.
    let mut shape = empty_shape();
    shape.cards.push(PositionedCard {
        person_id: "a".to_owned(),
        kind: SlotKind::Canonical,
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 50.0,
        name: "A".to_owned(),
    });
    shape.cards.push(PositionedCard {
        person_id: "b".to_owned(),
        kind: SlotKind::Ghost {
            reason: GhostReason::PastMarriage,
        },
        x: 110.0,
        y: 0.0,
        width: 100.0,
        height: 50.0,
        name: "B".to_owned(),
    });
    shape.bars.push(PositionedBar {
        marriage_id: "m".to_owned(),
        x: 100.0,
        y: 20.0,
        width: 10.0,
        height: 10.0,
        ended: true,
    });
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Adoption,
        routing: EdgeRouting::InTree,
        child_id: "a".to_owned(),
        marriage_id: "m".to_owned(),
        points: vec![(0.0, 0.0), (10.0, 10.0)],
    });
    let svg = render(&shape, &ThemeConfig::default());
    // `fill="none"` on edge polylines is structural (a polyline with
    // no fill is the contract a stroked line wants). Everything else
    // must not carry a colour-bearing attribute.
    let stripped = svg.replace(r#"fill="none""#, "");
    assert!(
        !stripped.contains(" fill=\""),
        "found inline fill in emitted SVG: {svg}"
    );
    assert!(
        !stripped.contains(" stroke=\""),
        "found inline stroke in emitted SVG: {svg}"
    );
    assert!(
        !stripped.contains(" color=\""),
        "found inline color in emitted SVG: {svg}"
    );
}

#[test]
fn name_label_is_xml_escaped() {
    let mut shape = empty_shape();
    shape.cards.push(PositionedCard {
        person_id: "a".to_owned(),
        kind: SlotKind::Canonical,
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 50.0,
        // Crafted to exercise every escape branch.
        name: r#"<Ann & "Co" 'Lt'>"#.to_owned(),
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(svg.contains("&lt;Ann &amp; &quot;Co&quot; &apos;Lt&apos;&gt;"));
    assert!(!svg.contains("<Ann &"));
}

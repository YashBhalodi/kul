//! Visual-vocabulary tests: pin the emitted-attribute seam decisions
//! [ADR-0016](../../../docs/adr/0016-visualization-pipeline-crate-boundaries.md) and
//! [ADR-0021](../../../docs/adr/0021-language-properties-plumb-to-svg.md)
//! commit to. Entity classes (`kul-card`, `kul-edge`) name the type
//! only; every property is a `data-*` attribute (booleans as
//! `data-is-*`, enums as explicit strings, missing optionals omitted).
//!
//! These tests construct `PositionedShape` values by hand so the
//! emitter is exercised independent of the kul-render / kul-layout
//! pipeline.

use kul_layout::{EdgeKind, PositionedCard, PositionedEdge, PositionedShape, SlotKind};
use kul_render::GhostReason;
use kul_svg::{ThemeConfig, render};

fn empty_shape() -> PositionedShape {
    PositionedShape {
        width: 200.0,
        height: 200.0,
        cards: Vec::new(),
        edges: Vec::new(),
    }
}

/// A minimal canonical card; tests override the fields they exercise.
fn canonical_card(person_id: &str, name: &str) -> PositionedCard {
    PositionedCard {
        person_id: person_id.to_owned(),
        kind: SlotKind::Canonical,
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 50.0,
        name: name.to_owned(),
        generation: 0,
        gender: "female",
        family: None,
        given: None,
        born: None,
        died: None,
    }
}

#[test]
fn canonical_card_emits_entity_class_and_kind_attribute() {
    let mut shape = empty_shape();
    shape.cards.push(canonical_card("alice", "Alice"));
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"<g class="kul-card" data-person-id="alice" data-kind="canonical""#),
        "expected entity class + data-kind, got: {svg}"
    );
    assert!(
        !svg.contains("kul-card--"),
        "no BEM modifier class may remain on a card: {svg}"
    );
    assert!(
        !svg.contains("data-ghost-reason"),
        "a canonical card has no ghost reason: {svg}"
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
fn card_emits_person_property_attributes() {
    let mut shape = empty_shape();
    let mut card = canonical_card("alice", "Alice");
    card.generation = 2;
    card.gender = "female";
    card.family = Some("Sharma".to_owned());
    card.given = Some("Alice".to_owned());
    card.born = Some("1950-04-12".to_owned());
    shape.cards.push(card);
    let svg = render(&shape, &ThemeConfig::default());
    assert!(svg.contains(r#"data-gender="female""#), "{svg}");
    assert!(svg.contains(r#"data-generation="2""#), "{svg}");
    assert!(svg.contains(r#"data-family="Sharma""#), "{svg}");
    assert!(svg.contains(r#"data-given="Alice""#), "{svg}");
    assert!(svg.contains(r#"data-born="1950-04-12""#), "{svg}");
    // Alive: no `died:` recorded.
    assert!(svg.contains(r#"data-is-alive="true""#), "{svg}");
    assert!(!svg.contains("data-died"), "no death recorded: {svg}");
}

#[test]
fn card_with_died_is_not_alive_and_omits_undeclared_optionals() {
    let mut shape = empty_shape();
    let mut card = canonical_card("bob", "Bob");
    card.died = Some("1998".to_owned());
    shape.cards.push(card);
    let svg = render(&shape, &ThemeConfig::default());
    assert!(svg.contains(r#"data-is-alive="false""#), "{svg}");
    assert!(svg.contains(r#"data-died="1998""#), "{svg}");
    // Undeclared optionals omit the attribute entirely (no empty strings).
    assert!(!svg.contains("data-family"), "{svg}");
    assert!(!svg.contains("data-given"), "{svg}");
    assert!(!svg.contains("data-born"), "{svg}");
}

#[test]
fn past_marriage_ghost_card_emits_ghost_reason_dasharray_and_badge() {
    let mut shape = empty_shape();
    let mut card = canonical_card("bob", "Bob");
    card.kind = SlotKind::Ghost {
        reason: GhostReason::PastMarriage,
    };
    shape.cards.push(card);
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"data-kind="ghost" data-ghost-reason="past-marriage""#),
        "expected ghost kind + reason: {svg}"
    );
    assert!(
        svg.contains(r#"stroke-dasharray="3 2""#),
        "expected ghost dasharray: {svg}"
    );
    assert!(
        svg.contains("kul-ghost-badge"),
        "expected ↺ badge for a ghost: {svg}"
    );
    assert!(svg.contains("↺"), "expected ↺ glyph: {svg}");
}

#[test]
fn past_adoption_ghost_card_emits_its_reason() {
    let mut shape = empty_shape();
    let mut card = canonical_card("alex", "Alex");
    card.kind = SlotKind::Ghost {
        reason: GhostReason::PastAdoption,
    };
    shape.cards.push(card);
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"data-ghost-reason="past-adoption""#),
        "{svg}"
    );
    assert!(svg.contains(r#"stroke-dasharray="3 2""#), "{svg}");
}

#[test]
fn past_birth_ghost_card_emits_its_reason() {
    let mut shape = empty_shape();
    let mut card = canonical_card("dalisay", "Dalisay");
    card.kind = SlotKind::Ghost {
        reason: GhostReason::PastBirth,
    };
    shape.cards.push(card);
    let svg = render(&shape, &ThemeConfig::default());
    assert!(svg.contains(r#"data-ghost-reason="past-birth""#), "{svg}");
}

#[test]
fn monogamy_marriage_edge_emits_marriage_link_kind_no_bar_rect() {
    // The unified marriage connector (ADR-0020): a monogamy marriage
    // renders as a thick `data-link-kind="marriage"` edge, not a bar rect.
    let mut shape = empty_shape();
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Marriage {
            host_id: "ramesh".to_owned(),
            joining_id: "sita".to_owned(),
            start: "1970".to_owned(),
            end: None,
            end_reason: None,
            is_ended: false,
        },
        points: vec![(185.0, 56.0), (215.0, 56.0)],
        marriage_id: "m_ramesh_sita".to_owned(),
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"data-link-kind="marriage""#),
        "expected marriage link kind: {svg}"
    );
    assert!(
        svg.contains(r#"data-host-id="ramesh" data-joining-id="sita" data-start="1970" data-is-ended="false""#),
        "expected marriage property attributes: {svg}"
    );
    assert!(
        !svg.contains("kul-edge--"),
        "no BEM modifier on edges: {svg}"
    );
    assert!(
        !svg.contains("data-end-reason"),
        "an un-ended marriage has no end reason: {svg}"
    );
    assert!(
        !svg.contains("<rect class=\"kul-bar"),
        "the marriage connector must not emit a bar rect: {svg}"
    );
}

#[test]
fn ended_monogamy_marriage_edge_emits_end_and_reason() {
    // Per current-intimacy placement: an ended (divorced) monogamy
    // marriage carries `data-is-ended="true"` plus its end fields.
    let mut shape = empty_shape();
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Marriage {
            host_id: "alice".to_owned(),
            joining_id: "bob".to_owned(),
            start: "1972".to_owned(),
            end: Some("1990".to_owned()),
            end_reason: Some("divorce".to_owned()),
            is_ended: true,
        },
        points: vec![(185.0, 216.0), (215.0, 216.0)],
        marriage_id: "m_alice_bob".to_owned(),
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(svg.contains(r#"data-is-ended="true""#), "{svg}");
    assert!(svg.contains(r#"data-end="1990""#), "{svg}");
    assert!(svg.contains(r#"data-end-reason="divorce""#), "{svg}");
}

#[test]
fn birth_edge_emits_birth_link_kind_no_dasharray() {
    let mut shape = empty_shape();
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Birth {
            child_id: "carol".to_owned(),
            is_past: false,
        },
        points: vec![(0.0, 0.0), (50.0, 50.0)],
        marriage_id: "m1".to_owned(),
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"data-link-kind="birth" data-child-id="carol" data-is-past="false""#),
        "expected birth attributes: {svg}"
    );
    assert!(
        !svg.contains("stroke-dasharray"),
        "birth edge must not ship a dasharray: {svg}"
    );
}

#[test]
fn marriage_edge_shares_base_class_and_is_solid() {
    // ADR-0020 marriage edge: solid, thick (weight set by the consuming
    // stylesheet via `data-link-kind="marriage"`), shares the base
    // `kul-edge` class with birth / adoption.
    let mut shape = empty_shape();
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Marriage {
            host_id: "devraj".to_owned(),
            joining_id: "alice".to_owned(),
            start: "1992".to_owned(),
            end: None,
            end_reason: None,
            is_ended: false,
        },
        points: vec![(0.0, 0.0), (50.0, 50.0)],
        marriage_id: "m_devraj_alice".to_owned(),
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"<path class="kul-edge""#),
        "expected base `kul-edge` class shared with birth / adoption: {svg}"
    );
    assert!(
        !svg.contains("stroke-dasharray"),
        "marriage edge must be solid (no dasharray): {svg}"
    );
}

#[test]
fn adoption_edge_emits_adoption_link_kind_with_dasharray_and_start() {
    let mut shape = empty_shape();
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Adoption {
            child_id: "ravi".to_owned(),
            is_past: true,
            start: Some("1985-06-01".to_owned()),
            end: None,
        },
        points: vec![(0.0, 0.0), (50.0, 50.0)],
        marriage_id: "m1".to_owned(),
    });
    let svg = render(&shape, &ThemeConfig::default());
    assert!(
        svg.contains(r#"data-link-kind="adoption" data-child-id="ravi" data-is-past="true""#),
        "expected adoption attributes: {svg}"
    );
    assert!(
        svg.contains(r#"data-adoption-start="1985-06-01""#),
        "expected adoption start: {svg}"
    );
    assert!(
        !svg.contains("data-adoption-end"),
        "undeclared adoption end is omitted: {svg}"
    );
    assert!(
        svg.contains(r#"stroke-dasharray="6 4""#),
        "adoption edge must ship a dasharray: {svg}"
    );
}

#[test]
fn emitted_svg_has_no_inline_fill_or_stroke_color() {
    // Theme-agnostic invariant: ADR-0016. Construct a shape with one
    // of each primitive and confirm none of the emitted attributes
    // assert a colour.
    let mut shape = empty_shape();
    shape.cards.push(canonical_card("a", "A"));
    let mut ghost = canonical_card("b", "B");
    ghost.kind = SlotKind::Ghost {
        reason: GhostReason::PastMarriage,
    };
    ghost.x = 110.0;
    shape.cards.push(ghost);
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Marriage {
            host_id: "a".to_owned(),
            joining_id: "b".to_owned(),
            start: "1990".to_owned(),
            end: Some("2000".to_owned()),
            end_reason: Some("divorce".to_owned()),
            is_ended: true,
        },
        points: vec![(100.0, 25.0), (110.0, 25.0)],
        marriage_id: "m".to_owned(),
    });
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Adoption {
            child_id: "a".to_owned(),
            is_past: false,
            start: Some("1995".to_owned()),
            end: None,
        },
        points: vec![(0.0, 0.0), (10.0, 10.0)],
        marriage_id: "m".to_owned(),
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
    let mut card = canonical_card("a", r#"<Ann & "Co" 'Lt'>"#);
    // Crafted to exercise every escape branch on a data-attribute too.
    card.family = Some(r#"O'<&>"Brien"#.to_owned());
    shape.cards.push(card);
    let svg = render(&shape, &ThemeConfig::default());
    assert!(svg.contains("&lt;Ann &amp; &quot;Co&quot; &apos;Lt&apos;&gt;"));
    assert!(!svg.contains("<Ann &"));
    // Attribute values are escaped too.
    assert!(
        svg.contains(r#"data-family="O&apos;&lt;&amp;&gt;&quot;Brien""#),
        "{svg}"
    );
}

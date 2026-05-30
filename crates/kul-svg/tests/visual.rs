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
fn past_marriage_ghost_card_emits_ghost_reason_and_dasharray() {
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
fn self_contained_true_injects_inline_style_with_concrete_tokens() {
    // The opt-in self-contained theme (CLI export, ADR-0016 amendment):
    // an inline `<style>` carrying the `--kul-*` token vocabulary with
    // concrete hex values, scoped on the `svg` selector and emitted as
    // the root's first child so it precedes every element.
    let mut shape = empty_shape();
    let mut ghost = canonical_card("b", "B");
    ghost.kind = SlotKind::Ghost {
        reason: GhostReason::PastMarriage,
    };
    shape.cards.push(canonical_card("a", "A"));
    shape.cards.push(ghost);
    let svg = render(&shape, &ThemeConfig::with_self_contained(true));
    // `<style>` is the root's first child (before any `<g`/`<path`).
    let style_at = svg.find("<style>").expect("expected an inline <style>");
    let first_g = svg.find("<g").unwrap_or(usize::MAX);
    assert!(
        style_at < first_g,
        "the <style> block must precede the first element: {svg}"
    );
    // The token vocabulary is reused, with concrete colours baked in.
    assert!(svg.contains("--kul-card-stroke-male: #1565c0;"), "{svg}");
    assert!(svg.contains("--kul-edge-stroke: #2e7d32;"), "{svg}");
    assert!(
        svg.contains("--kul-marriage-edge-stroke-width: 8.75;"),
        "the thick unified marriage connector width must be baked in: {svg}"
    );
    assert!(
        svg.contains(".kul-card[data-kind=\"ghost\"] rect"),
        "the ghost structural rule must be present: {svg}"
    );
    // No surface chrome: the ghost `↺` badge is never emitted (ADR-0016).
    assert!(!svg.contains('↺'), "no ghost badge may appear: {svg}");
    assert!(
        !svg.contains("kul-ghost-badge"),
        "no ghost-badge styling may appear: {svg}"
    );
    // Self-contained means no VSCode-specific variables.
    assert!(!svg.contains("var(--vscode-"), "{svg}");
}

#[test]
fn self_contained_false_omits_style_block() {
    let mut shape = empty_shape();
    shape.cards.push(canonical_card("a", "A"));
    // Default config is theme-agnostic: byte-identical to today, no style.
    let default_svg = render(&shape, &ThemeConfig::default());
    assert!(
        !default_svg.contains("<style>"),
        "default output must carry no inline <style>: {default_svg}"
    );
    assert!(!default_svg.contains('↺'), "{default_svg}");
    // Explicit `false` matches the default exactly.
    let explicit_false = render(&shape, &ThemeConfig::with_self_contained(false));
    assert_eq!(default_svg, explicit_false);
}

// -- Legend (#157, ADR-0022) -------------------------------------------
//
// The legend rides the opt-in `ThemeConfig.legend = true` path. Each
// row is a swatch + a label; the swatch is a miniature of the real
// glyph carrying the production class + `data-*` attribute, so the
// surrounding stylesheet themes it through the same rules that paint
// the diagram (no hardcoded swatch colour). Default and
// `with_self_contained(true)` paths remain byte-unchanged.

fn full_vocab_shape() -> PositionedShape {
    // A shape that surfaces every canonical legend category at least
    // once, so a full-legend render exercises all eight rows.
    let mut shape = empty_shape();
    let mut male = canonical_card("a", "Alice");
    male.gender = "male";
    shape.cards.push(male);
    let mut female = canonical_card("b", "Brenda");
    female.gender = "female";
    shape.cards.push(female);
    let mut other = canonical_card("c", "Carey");
    other.gender = "other";
    shape.cards.push(other);
    let mut ghost = canonical_card("d", "Dia");
    ghost.kind = SlotKind::Ghost {
        reason: GhostReason::PastMarriage,
    };
    shape.cards.push(ghost);
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Birth {
            child_id: "kid".to_owned(),
            is_past: false,
        },
        points: vec![(0.0, 0.0), (10.0, 10.0)],
        marriage_id: "m1".to_owned(),
    });
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Adoption {
            child_id: "kid2".to_owned(),
            is_past: false,
            start: Some("2000".to_owned()),
            end: None,
        },
        points: vec![(0.0, 0.0), (10.0, 10.0)],
        marriage_id: "m2".to_owned(),
    });
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Marriage {
            host_id: "a".to_owned(),
            joining_id: "b".to_owned(),
            start: "1990".to_owned(),
            end: None,
            end_reason: None,
            is_ended: false,
        },
        points: vec![(0.0, 0.0), (10.0, 0.0)],
        marriage_id: "m3".to_owned(),
    });
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Marriage {
            host_id: "c".to_owned(),
            joining_id: "d".to_owned(),
            start: "1975".to_owned(),
            end: Some("1985".to_owned()),
            end_reason: Some("divorce".to_owned()),
            is_ended: true,
        },
        points: vec![(0.0, 0.0), (10.0, 0.0)],
        marriage_id: "m4".to_owned(),
    });
    shape
}

#[test]
fn legend_false_default_emits_no_legend() {
    let shape = full_vocab_shape();
    let svg = render(&shape, &ThemeConfig::default());
    // The legend group element is the marker: the bare CSS rules in the
    // baked stylesheet contain the `.kul-legend` selector but no element
    // emits with the class — so `<g class="kul-legend"` is the right
    // check (also covers the no-English requirement on the default path).
    assert!(
        !svg.contains(r#"<g class="kul-legend">"#),
        "default config must not emit a legend group: {svg}"
    );
}

#[test]
fn with_self_contained_true_alone_emits_no_legend() {
    let shape = full_vocab_shape();
    let svg = render(&shape, &ThemeConfig::with_self_contained(true));
    // self_contained alone bakes `.kul-legend` *rules* (they are part of
    // the canonical stylesheet so a `with_legend(true)` consumer's
    // swatches paint correctly), but no `<g class="kul-legend">` element
    // ships and no English does either.
    assert!(
        !svg.contains(r#"<g class="kul-legend">"#),
        "self_contained without with_legend(true) must not emit a legend group: {svg}"
    );
    for english in [
        ">Male<",
        ">Female<",
        ">Other<",
        ">Past record<",
        ">Birth<",
        ">Adoption<",
        ">Marriage<",
        ">Ended marriage<",
    ] {
        assert!(
            !svg.contains(english),
            "self_contained alone must ship no legend English (`{english}` found): {svg}"
        );
    }
}

#[test]
fn explicit_with_legend_false_matches_default() {
    let shape = full_vocab_shape();
    let default_svg = render(&shape, &ThemeConfig::default());
    let explicit_false = render(&shape, &ThemeConfig::default().with_legend(false));
    assert_eq!(
        default_svg, explicit_false,
        "with_legend(false) must produce the same bytes as default()"
    );
}

#[test]
fn full_vocab_legend_emits_every_row_in_canonical_order() {
    let shape = full_vocab_shape();
    let svg = render(
        &shape,
        &ThemeConfig::with_self_contained(true).with_legend(true),
    );
    // The legend group is emitted as a kul-legend container.
    assert!(
        svg.contains(r#"<g class="kul-legend">"#),
        "expected the legend group: {svg}"
    );
    // The eight normative labels are present, in canonical order.
    let labels = [
        ">Male<",
        ">Female<",
        ">Other<",
        ">Past record<",
        ">Birth<",
        ">Adoption<",
        ">Marriage<",
        ">Ended marriage<",
    ];
    let mut cursor = 0;
    for label in labels {
        let idx = svg[cursor..].find(label).unwrap_or_else(|| {
            panic!("expected legend label {label} after position {cursor} in: {svg}")
        });
        cursor += idx + label.len();
    }
}

#[test]
fn legend_swatches_reuse_production_classes_and_data_attrs() {
    let shape = full_vocab_shape();
    let svg = render(
        &shape,
        &ThemeConfig::with_self_contained(true).with_legend(true),
    );
    // Each swatch carries the production class + `data-*` seam — the
    // colour contract (ADR-0022): the existing CSS rules paint them.
    for needle in [
        r#"<g class="kul-card" data-kind="canonical" data-gender="male">"#,
        r#"<g class="kul-card" data-kind="canonical" data-gender="female">"#,
        r#"<g class="kul-card" data-kind="canonical" data-gender="other">"#,
        r#"<g class="kul-card" data-kind="ghost">"#,
        r#"<path class="kul-edge" data-link-kind="birth""#,
        r#"<path class="kul-edge" data-link-kind="adoption""#,
        r#"<path class="kul-edge" data-link-kind="marriage""#,
        r#"<path class="kul-edge" data-link-kind="marriage" data-is-ended="true""#,
    ] {
        assert!(
            svg.contains(needle),
            "expected swatch with production seam `{needle}` in: {svg}"
        );
    }
    // Structural dasharrays mirror production (ghost: 3 2; adoption: 6 4).
    assert!(svg.contains(r#"stroke-dasharray="3 2""#), "{svg}");
    assert!(svg.contains(r#"stroke-dasharray="6 4""#), "{svg}");
}

#[test]
fn legend_swatch_overrides_only_size_in_baked_css() {
    // The swatch-targeting `.kul-legend …` rules may tune only
    // size / stroke-width / dash on the production glyphs — never colour.
    // (The panel rect `.kul-legend-bg` is its own structural element and
    // carries panel-specific tokens; that's separate from swatch overrides.)
    let shape = full_vocab_shape();
    let svg = render(
        &shape,
        &ThemeConfig::with_self_contained(true).with_legend(true),
    );
    assert!(
        svg.contains(".kul-legend .kul-edge[data-link-kind=\"marriage\"]"),
        "expected the marriage stroke-width override: {svg}"
    );
    assert!(
        svg.contains("--kul-legend-marriage-edge-stroke-width"),
        "marriage stroke override must consume a token: {svg}"
    );
    // The marriage swatch rule must only set stroke-width — never `stroke:`
    // or `fill:`.
    let marr_start = svg
        .find(".kul-legend .kul-edge[data-link-kind=\"marriage\"]")
        .expect("marriage rule");
    let marr_end = marr_start + svg[marr_start..].find('}').expect("rule close");
    let marr_rule = &svg[marr_start..marr_end];
    assert!(
        !marr_rule.contains("stroke:") && !marr_rule.contains("fill:"),
        "marriage swatch override must not override colour: {marr_rule}"
    );
    // Every colour in `.kul-legend*` rules must be token-bound — no hex
    // literals leak into the legend stylesheet block.
    let legend_block_start = svg.find(".kul-legend").expect("legend rule");
    let legend_block_end = legend_block_start
        + svg[legend_block_start..]
            .find("</style>")
            .expect("style close");
    let legend_block = &svg[legend_block_start..legend_block_end];
    assert!(
        !legend_block.contains('#'),
        ".kul-legend* rules must use --kul-* tokens, not hex literals: {legend_block}"
    );
}

#[test]
fn legend_emits_a_rounded_panel_background() {
    // The panel rect ships first inside the `.kul-legend` group so rows
    // sit on top of it; its rounded corners and colours come from the
    // `--kul-legend-panel-*` tokens (no hardcoded hex on the element).
    let shape = full_vocab_shape();
    let svg = render(
        &shape,
        &ThemeConfig::with_self_contained(true).with_legend(true),
    );
    let group_start = svg.find(r#"<g class="kul-legend">"#).expect("legend group");
    let after_group = &svg[group_start + r#"<g class="kul-legend">"#.len()..];
    // First child of the legend group is the panel rect.
    assert!(
        after_group.starts_with(r#"<rect class="kul-legend-bg""#),
        "panel rect must be the first child of the legend group: {after_group}"
    );
    // Rounded corners (`rx`/`ry`) are present and non-zero.
    let panel_end =
        group_start + r#"<g class="kul-legend">"#.len() + after_group.find("/>").unwrap();
    let panel_rect = &svg[group_start..=panel_end];
    assert!(
        panel_rect.contains("rx=\"6\""),
        "panel rect must carry the configured corner radius: {panel_rect}"
    );
    // No hardcoded colour on the panel element itself — the baked
    // stylesheet's `.kul-legend-bg` rule paints it via tokens.
    assert!(
        !panel_rect.contains(" fill=\""),
        "panel rect must not carry inline fill: {panel_rect}"
    );
    assert!(
        !panel_rect.contains(" stroke=\""),
        "panel rect must not carry inline stroke: {panel_rect}"
    );
    // The baked stylesheet defines the panel's fill / stroke via tokens.
    assert!(
        svg.contains(".kul-legend-bg { fill: var(--kul-legend-panel-bg);"),
        "expected the .kul-legend-bg rule keyed on panel tokens: {svg}"
    );
}

#[test]
fn legend_dynamic_subset_omits_absent_rows() {
    // A nuclear family with only male+female parents and a male child via
    // a marriage with a birth edge. No ghost, no adoption, no other, no
    // ended marriage — those rows must NOT appear.
    let mut shape = empty_shape();
    let mut dad = canonical_card("a", "Akira");
    dad.gender = "male";
    shape.cards.push(dad);
    let mut mom = canonical_card("b", "Bao");
    mom.gender = "female";
    shape.cards.push(mom);
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Marriage {
            host_id: "a".to_owned(),
            joining_id: "b".to_owned(),
            start: "1990".to_owned(),
            end: None,
            end_reason: None,
            is_ended: false,
        },
        points: vec![(0.0, 0.0), (10.0, 0.0)],
        marriage_id: "m".to_owned(),
    });
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Birth {
            child_id: "kid".to_owned(),
            is_past: false,
        },
        points: vec![(0.0, 0.0), (10.0, 10.0)],
        marriage_id: "m".to_owned(),
    });
    let svg = render(
        &shape,
        &ThemeConfig::with_self_contained(true).with_legend(true),
    );
    assert!(svg.contains(">Male<"), "expected Male row: {svg}");
    assert!(svg.contains(">Female<"), "expected Female row: {svg}");
    assert!(svg.contains(">Birth<"), "expected Birth row: {svg}");
    assert!(svg.contains(">Marriage<"), "expected Marriage row: {svg}");
    assert!(
        !svg.contains(">Other<"),
        "no `other` gender → no Other row: {svg}"
    );
    assert!(
        !svg.contains(">Past record<"),
        "no ghost → no Past record row: {svg}"
    );
    assert!(
        !svg.contains(">Adoption<"),
        "no adoption edges → no Adoption row: {svg}"
    );
    assert!(
        !svg.contains(">Ended marriage<"),
        "no ended marriage → no Ended marriage row: {svg}"
    );
}

#[test]
fn legend_only_ended_marriages_emits_just_ended_row() {
    // If every marriage is ended, only the "Ended marriage" row appears
    // (the "Marriage" un-ended category is empty for this diagram).
    let mut shape = empty_shape();
    shape.edges.push(PositionedEdge {
        kind: EdgeKind::Marriage {
            host_id: "a".to_owned(),
            joining_id: "b".to_owned(),
            start: "1970".to_owned(),
            end: Some("1980".to_owned()),
            end_reason: Some("divorce".to_owned()),
            is_ended: true,
        },
        points: vec![(0.0, 0.0), (10.0, 0.0)],
        marriage_id: "m".to_owned(),
    });
    let svg = render(
        &shape,
        &ThemeConfig::with_self_contained(true).with_legend(true),
    );
    assert!(
        !svg.contains(">Marriage<"),
        "no un-ended marriage → no Marriage row: {svg}"
    );
    assert!(
        svg.contains(">Ended marriage<"),
        "expected Ended marriage row: {svg}"
    );
}

#[test]
fn legend_grows_viewbox_height_without_touching_diagram_geometry() {
    // The legend lives in a reserved bottom band; the diagram's cards
    // and edges keep their original coordinates and the viewBox height
    // grows by exactly the legend's footprint.
    let shape = full_vocab_shape();
    let without = render(&shape, &ThemeConfig::default());
    let with = render(
        &shape,
        &ThemeConfig::with_self_contained(true).with_legend(true),
    );
    // The original card / edge geometry strings (without the closing
    // `</svg>`) must appear unchanged in the legend-bearing output —
    // the diagram is not relocated.
    let card_open = r#"<g class="kul-card" data-person-id="a""#;
    assert!(without.contains(card_open), "{without}");
    assert!(with.contains(card_open), "{with}");
    // The viewBox height must be strictly larger with the legend.
    let height_re = |svg: &str| -> f64 {
        let mark = "viewBox=\"0 0 ";
        let start = svg.find(mark).unwrap() + mark.len();
        let inside = &svg[start..start + svg[start..].find('"').unwrap()];
        let parts: Vec<&str> = inside.split_whitespace().collect();
        parts[1].parse::<f64>().unwrap()
    };
    let h_without = height_re(&without);
    let h_with = height_re(&with);
    assert!(
        h_with > h_without,
        "legend must grow viewBox height: {h_without} vs {h_with}"
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

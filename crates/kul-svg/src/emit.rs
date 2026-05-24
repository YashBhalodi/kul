//! SVG string templating from `PositionedShape`.
//!
//! Stateless. Walks the shape once and writes into a single `String`
//! buffer. Uses semantic CSS classes only — no inline colours, no
//! script.

use std::fmt::Write;

use kul_layout::{
    EdgeKind, EdgeRouting, PositionedBar, PositionedCard, PositionedEdge, PositionedFanConnector,
    PositionedShape, SlotKind,
};
use kul_render::GhostReason;

/// Theme / emission configuration.
///
/// Forward-compatibility seam (per [ADR-0019](../../docs/adr/0019-kul-svg-crate-boundary.md));
/// only [`ThemeConfig::default()`] is constructed by any consumer in
/// v1. Future fields (opt-in inline CSS for self-contained CLI export,
/// opt-in source-span data attributes for click-to-jump) add here
/// without changing [`crate::render`]'s signature.
#[derive(Debug, Clone, Default)]
pub struct ThemeConfig {
    #[doc(hidden)]
    _private: (),
}

pub(crate) fn render(positioned: &PositionedShape, _config: &ThemeConfig) -> String {
    let mut out = String::with_capacity(2048);
    write_open(&mut out, positioned);
    // Fan connectors render *under* bars + cards (so the bar visually
    // terminates the drop) but *above* edges so a child's birth edge
    // doesn't draw through the fan's trunk. The class is independent
    // of the edge classes — fans are not parent-child edges.
    for fan in &positioned.fan_connectors {
        write_fan_connector(&mut out, fan);
    }
    for edge in &positioned.edges {
        write_edge(&mut out, edge);
    }
    for bar in &positioned.bars {
        write_bar(&mut out, bar);
    }
    for card in &positioned.cards {
        write_card(&mut out, card);
    }
    out.push_str("</svg>");
    out
}

fn write_open(out: &mut String, shape: &PositionedShape) {
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">"#,
        w = fmt_num(shape.width),
        h = fmt_num(shape.height),
    );
}

fn write_card(out: &mut String, card: &PositionedCard) {
    let (kind_class, ghost_badge) = match card.kind {
        SlotKind::Canonical => ("kul-card--canonical", None),
        SlotKind::Ghost {
            reason: GhostReason::PastMarriage,
        } => ("kul-card--ghost", Some("↺")),
        SlotKind::Ghost {
            reason: GhostReason::PastAdoption,
        } => ("kul-card--ghost", Some("↺")),
        SlotKind::Ghost {
            reason: GhostReason::PastBirth,
        } => ("kul-card--ghost", Some("↺")),
    };
    let _ = write!(out, r#"<g class="kul-card {kind_class}">"#);
    // Ghost cards ship with stroke-dasharray inline (structural, per
    // P15 — see ADR-0019 §"Ghost visual treatment is structural").
    let dash = if matches!(card.kind, SlotKind::Ghost { .. }) {
        r#" stroke-dasharray="3 2""#
    } else {
        ""
    };
    // Soft corner radius — pure visual polish, consumers can override
    // via CSS (`rect { rx: 0 }`) for a sharper look.
    let _ = write!(
        out,
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" ry="{r}"{dash}/>"#,
        x = fmt_num(card.x),
        y = fmt_num(card.y),
        w = fmt_num(card.width),
        h = fmt_num(card.height),
        r = fmt_num(CARD_CORNER_RADIUS),
    );
    let label_x = card.x + card.width / 2.0;
    let label_y = card.y + card.height / 2.0;
    let _ = write!(
        out,
        r#"<text class="kul-label-name" x="{x}" y="{y}" text-anchor="middle" dominant-baseline="central">{name}</text>"#,
        x = fmt_num(label_x),
        y = fmt_num(label_y),
        name = escape_xml(&card.name),
    );
    if let Some(glyph) = ghost_badge {
        let badge_x = card.x + card.width - 12.0;
        let badge_y = card.y + 14.0;
        let _ = write!(
            out,
            r#"<text class="kul-ghost-badge" x="{x}" y="{y}" text-anchor="middle">{g}</text>"#,
            x = fmt_num(badge_x),
            y = fmt_num(badge_y),
            g = glyph,
        );
    }
    out.push_str("</g>");
}

fn write_bar(out: &mut String, bar: &PositionedBar) {
    let extra = if bar.ended { " kul-bar--ended" } else { "" };
    let _ = write!(
        out,
        r#"<rect class="kul-bar{extra}" x="{x}" y="{y}" width="{w}" height="{h}"/>"#,
        x = fmt_num(bar.x),
        y = fmt_num(bar.y),
        w = fmt_num(bar.width),
        h = fmt_num(bar.height),
    );
}

fn write_fan_connector(out: &mut String, fan: &PositionedFanConnector) {
    // One `<path>` per segment so the rounded-corner helper can treat
    // each polyline independently. The `fan-connector` class carries
    // the visual styling (matches the marriage-bar stroke weight per
    // ADR-0027 / ADR-0019); no inline `stroke=` or `fill=` colour
    // beyond the structural `fill="none"` that any stroked path
    // wants.
    for segment in &fan.segments {
        let d = polyline_to_rounded_path(segment, EDGE_CORNER_RADIUS);
        let _ = write!(
            out,
            r#"<path class="kul-fan-connector" data-hub-id="{hub}" fill="none" d="{d}"/>"#,
            hub = escape_xml(&fan.hub_id),
        );
    }
}

fn write_edge(out: &mut String, edge: &PositionedEdge) {
    let kind_class = match edge.kind {
        EdgeKind::Birth => "kul-edge--birth",
        EdgeKind::Adoption => "kul-edge--adoption",
    };
    let routing_class = match edge.routing {
        EdgeRouting::InTree => "kul-edge--in-tree",
        EdgeRouting::CrossTree => "kul-edge--cross-tree",
    };
    // Adoption edges ship with stroke-dasharray inline (structural,
    // per P5 — see ADR-0019 §"Edge dasharrays are structural").
    let dash = match edge.kind {
        EdgeKind::Adoption => r#" stroke-dasharray="6 4""#,
        EdgeKind::Birth => "",
    };
    let d = polyline_to_rounded_path(&edge.points, EDGE_CORNER_RADIUS);
    let _ = write!(
        out,
        r#"<path class="kul-edge {kind_class} {routing_class}" fill="none" d="{d}"{dash}/>"#,
    );
}

/// Card-corner radius in pixels. Visual polish — surface stylesheets
/// can override via CSS (`rect { rx: 0 }`).
const CARD_CORNER_RADIUS: f64 = 8.0;

/// Edge-corner radius in pixels. The polyline's 90° bends become
/// quadratic-Bézier arcs of this radius; consumers wanting hard
/// corners can target the `kul-edge` class and re-emit, but the
/// default canonical visual is soft.
const EDGE_CORNER_RADIUS: f64 = 10.0;

/// Convert an orthogonal polyline (each segment axis-aligned) into an
/// SVG path string with each interior corner rounded by a
/// quadratic-Bézier arc of approximately `radius` pixels.
///
/// Two-point polylines pass through as a straight line; a polyline
/// whose adjacent segments are shorter than `2 * radius` shrinks the
/// arc to fit. Returned string is the contents of the `d=` attribute.
fn polyline_to_rounded_path(points: &[(f64, f64)], radius: f64) -> String {
    if points.is_empty() {
        return String::new();
    }
    let mut path = String::with_capacity(points.len() * 16);
    let _ = write!(path, "M {} {}", fmt_num(points[0].0), fmt_num(points[0].1));
    if points.len() == 1 {
        return path;
    }
    if points.len() == 2 {
        let _ = write!(path, " L {} {}", fmt_num(points[1].0), fmt_num(points[1].1));
        return path;
    }
    for i in 1..points.len() - 1 {
        let prev = points[i - 1];
        let here = points[i];
        let next = points[i + 1];
        let dx_in = here.0 - prev.0;
        let dy_in = here.1 - prev.1;
        let len_in = (dx_in * dx_in + dy_in * dy_in).sqrt();
        let dx_out = next.0 - here.0;
        let dy_out = next.1 - here.1;
        let len_out = (dx_out * dx_out + dy_out * dy_out).sqrt();
        if len_in == 0.0 || len_out == 0.0 {
            // Degenerate: emit a hard corner.
            let _ = write!(path, " L {} {}", fmt_num(here.0), fmt_num(here.1));
            continue;
        }
        // Effective radius can't exceed half of either adjacent
        // segment, so the arcs never overlap each other or shoot
        // past the segment endpoints.
        let r = radius.min(len_in / 2.0).min(len_out / 2.0);
        let arrive_x = here.0 - dx_in / len_in * r;
        let arrive_y = here.1 - dy_in / len_in * r;
        let depart_x = here.0 + dx_out / len_out * r;
        let depart_y = here.1 + dy_out / len_out * r;
        let _ = write!(
            path,
            " L {} {} Q {} {} {} {}",
            fmt_num(arrive_x),
            fmt_num(arrive_y),
            fmt_num(here.0),
            fmt_num(here.1),
            fmt_num(depart_x),
            fmt_num(depart_y),
        );
    }
    let last = points[points.len() - 1];
    let _ = write!(path, " L {} {}", fmt_num(last.0), fmt_num(last.1));
    path
}

/// Format a float without trailing zeros or trailing decimal points.
/// Layout produces integer-multiples of pixel constants in v1; this
/// keeps the snapshot tidy without forcing every coordinate through a
/// full ryu round-trip.
fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{:.0}", n)
    } else {
        // Round to 3 decimals so snapshots stay stable under
        // f64-level rounding drift; trims trailing zeros below.
        let raw = format!("{:.3}", n);
        let trimmed = raw.trim_end_matches('0').trim_end_matches('.');
        trimmed.to_owned()
    }
}

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

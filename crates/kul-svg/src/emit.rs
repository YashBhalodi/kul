//! SVG string templating from `PositionedShape`.
//!
//! Stateless. Walks the shape once and writes into a single `String`
//! buffer. Uses semantic CSS classes only — no inline colours, no
//! script.

use std::fmt::Write;

use kul_layout::{
    EdgeKind, EdgeRouting, PositionedBar, PositionedCard, PositionedEdge, PositionedShape, SlotKind,
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
    };
    let _ = write!(out, r#"<g class="kul-card {kind_class}">"#);
    // Ghost cards ship with stroke-dasharray inline (structural, per
    // P15 — see ADR-0019 §"Ghost visual treatment is structural").
    let dash = if matches!(card.kind, SlotKind::Ghost { .. }) {
        r#" stroke-dasharray="3 2""#
    } else {
        ""
    };
    let _ = write!(
        out,
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}"{dash}/>"#,
        x = fmt_num(card.x),
        y = fmt_num(card.y),
        w = fmt_num(card.width),
        h = fmt_num(card.height),
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
    let mut pts = String::with_capacity(edge.points.len() * 12);
    for (i, (x, y)) in edge.points.iter().enumerate() {
        if i > 0 {
            pts.push(' ');
        }
        let _ = write!(pts, "{},{}", fmt_num(*x), fmt_num(*y));
    }
    let _ = write!(
        out,
        r#"<polyline class="kul-edge {kind_class} {routing_class}" fill="none" points="{pts}"{dash}/>"#,
    );
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

//! [`PositionedShape`] and friends — the data form `kul-svg` and other
//! Rust-side surface adapters walk.
//!
//! These types are an **internal Rust seam**. They are deliberately
//! *not* `Serialize` and *not* schema-versioned: the wire shapes the
//! project pins are `RenderShape` (input) and the SVG string (output).
//! See [ADR-0018](../../docs/adr/0018-kul-layout-crate-boundary.md).

use kul_render::GhostReason;

/// Top-level positioned shape: every canvas, every card, every bar,
/// every edge, in absolute pixel coordinates, in the canonical UI
/// pattern's arrangement.
///
/// Coordinates are top-left origin; `width` and `height` describe the
/// outer bounding box (including outer padding) so a surface emitter can
/// set the `<svg viewBox>` directly without recomputing it.
#[derive(Debug, Clone)]
pub struct PositionedShape {
    /// Outer canvas width including outer padding.
    pub width: f64,
    /// Outer canvas height including outer padding.
    pub height: f64,
    /// Every positioned card, in stable iteration order (canonical
    /// cards first by component-source-order, then ghosts adjacent to
    /// their anchoring bars).
    pub cards: Vec<PositionedCard>,
    /// Every positioned marriage / adoption bar.
    pub bars: Vec<PositionedBar>,
    /// Every parent-child edge, with computed polyline geometry.
    pub edges: Vec<PositionedEdge>,
}

/// One positioned person card. The visual primitive surface renderers
/// project into the canonical UI pattern's uniform-card shape (P15).
#[derive(Debug, Clone)]
pub struct PositionedCard {
    /// Source-declaration id (`person <id>`). Stable across renders;
    /// carried so click-to-jump follow-ups (F10) can attach without
    /// changing the type.
    pub person_id: String,
    /// Canonical vs ghost (P2 / P8 / P16). The discriminator a surface
    /// renderer keys on for the dotted-border + faded-fill + ↺-badge
    /// visual vocabulary.
    pub kind: SlotKind,
    /// Top-left x coordinate of the card.
    pub x: f64,
    /// Top-left y coordinate of the card.
    pub y: f64,
    /// Card width — mirrors [`crate::LayoutConfig::card_width`]; carried
    /// per-card so surface renderers don't have to thread the config.
    pub width: f64,
    /// Card height.
    pub height: f64,
    /// Display name (P15 minimum).
    pub name: String,
}

/// Whether a [`PositionedCard`] is the person's canonical card (P2)
/// or a ghost (P8, P16) anchoring a past structural fact.
///
/// Mirrors [`kul_render::SlotKind`] one-to-one but lives here so
/// downstream surface emitters (kul-svg, future native preview) don't
/// have to pull in `kul_render` just for the discriminator.
#[derive(Debug, Clone, Copy)]
pub enum SlotKind {
    Canonical,
    Ghost { reason: GhostReason },
}

/// One positioned marriage / adoption bar. Sits adjacent to the two
/// spouse / parent cards (P9: child edges anchor at the bar, not at
/// either parent).
#[derive(Debug, Clone)]
pub struct PositionedBar {
    /// Source-declaration id (`marriage <id>`).
    pub marriage_id: String,
    /// Top-left x of the bar.
    pub x: f64,
    /// Top-left y of the bar.
    pub y: f64,
    /// Bar width.
    pub width: f64,
    /// Bar height.
    pub height: f64,
    /// `true` iff the marriage carries an `end:` field (P8's canonical
    /// "ended" predicate). Surfaces use this to switch the bar's class
    /// to `kul-bar--ended`.
    pub ended: bool,
}

/// One positioned parent-child edge with computed polyline geometry.
///
/// Birth edges (P5 solid) and adoption edges (P5 dashed) share the
/// same shape; the `kind` field discriminates and the polyline points
/// describe the orthogonal right-angle route a surface emits directly.
#[derive(Debug, Clone)]
pub struct PositionedEdge {
    /// Birth vs adoption.
    pub kind: EdgeKind,
    /// Routing variant — `InTree` for v1, `CrossTree` for future
    /// cross-component edges (F5).
    pub routing: EdgeRouting,
    /// Source-declaration id of the child this edge belongs to.
    pub child_id: String,
    /// Source-declaration id of the marriage this edge connects to.
    pub marriage_id: String,
    /// Polyline points, in draw order. Each entry is an absolute
    /// pixel coordinate; the emitter writes them straight into
    /// `<polyline points="x1,y1 x2,y2 …" />`.
    pub points: Vec<(f64, f64)>,
}

/// What kind of parent-child edge this is (P5).
#[derive(Debug, Clone, Copy)]
pub enum EdgeKind {
    /// Solid (P5).
    Birth,
    /// Dashed (P5).
    Adoption,
}

/// How an edge is routed. Extensible discriminator: v1 only constructs
/// [`EdgeRouting::InTree`]; the cross-tree follow-up (F5) lands the
/// `CrossTree` variant and a routing implementation for it without
/// changing this enum's shape.
#[derive(Debug, Clone, Copy)]
pub enum EdgeRouting {
    /// Standard descendency-tree route: from the bar's bottom midpoint
    /// down to a horizontal bus mid-row, across, then down to the
    /// child's card top. Matches classical convention (P1).
    InTree,
    /// Cross-tree route — between two components, or within one
    /// component across the canonical-card-tree hierarchy (P11). v1
    /// does not construct this; reserved for F5.
    CrossTree,
}

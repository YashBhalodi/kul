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
    /// Every positioned marriage / adoption bar. Monogamy
    /// (`hosted_marriages.len() == 1`) emits one bar per marriage between
    /// the two spouse cards. Polygamy (`hosted_marriages.len() >= 2`)
    /// emits **no** bars — each concurrent marriage renders as a
    /// thick [`EdgeKind::Marriage`] edge between hub and co-spouse
    /// instead (ADR-0027).
    pub bars: Vec<PositionedBar>,
    /// Every edge — birth, adoption, and marriage — with computed
    /// polyline geometry. Birth and adoption edges connect a marriage
    /// bar (monogamy) or a co-spouse card (polygamy) to one of the
    /// couple's children; marriage edges (polygamy only) connect the
    /// hub card to each co-spouse card per ADR-0027.
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

/// What kind of edge this is. Birth and adoption edges connect a
/// marriage to one of its children (P5). Marriage edges connect a
/// polygamy hub to each of its concurrent co-spouses (ADR-0027) —
/// only emitted when `hosted_marriages.len() >= 2`; monogamy renders
/// the marriage with a [`PositionedBar`] between adjacent spouse cards
/// instead, with no marriage edge.
#[derive(Debug, Clone, Copy)]
pub enum EdgeKind {
    /// Solid, thin (P5).
    Birth,
    /// Dashed, thin (P5).
    Adoption,
    /// Solid, thick. Connects a polygamy hub to one of its
    /// concurrent co-spouses; one edge per hosted marriage when the
    /// hub has ≥2 un-ended marriages (ADR-0027). The hub card sits
    /// alone on its data-level generation row; each co-spouse sits on
    /// the next row down; the marriage edge routes hub-bottom →
    /// horizontal bus → co-spouse-top using the same orthogonal right-
    /// angle geometry as a birth edge. Visually distinguished from
    /// birth / adoption by a thicker stroke (set by the consuming
    /// surface stylesheet via the `kul-edge--marriage` class).
    Marriage,
}

/// How an edge is routed. Both variants emit the **same** orthogonal
/// right-angle polyline geometry and the same attachment points —
/// bar bottom-midpoint, horizontal bus at `card_top - config.bus_drop`,
/// child card top-midpoint — so the entire diagram follows one
/// consistent edge-routing pattern (P1). The discriminator exists to
/// give surface consumers a future re-theming hook (the emitted CSS
/// classes differ); the layout layer treats them identically. See
/// [ADR-0018](../../docs/adr/0018-kul-layout-crate-boundary.md).
#[derive(Debug, Clone, Copy)]
pub enum EdgeRouting {
    /// Standard descendency-tree route: the child sits structurally
    /// inside this marriage's subtree, directly below the bar in the
    /// children row.
    InTree,
    /// Cross-tree route — both endpoints are positioned in the laid-out
    /// tree, but the child is not a structural descendant of this
    /// marriage's bar. The canonical exerciser is the cousin-marriage
    /// case (P11): the joining cousin's birth-edge connects back to a
    /// sibling marriage already in the rendering context.
    CrossTree,
}

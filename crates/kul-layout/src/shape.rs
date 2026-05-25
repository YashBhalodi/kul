//! [`PositionedShape`] and friends — the data form `kul-svg` and other
//! Rust-side surface adapters walk.
//!
//! These types are an **internal Rust seam**. They are deliberately
//! *not* `Serialize` and *not* schema-versioned: the wire shapes the
//! project pins are `RenderShape` (input) and the SVG string (output).
//! See [ADR-0016](../../docs/adr/0016-visualization-pipeline-crate-boundaries.md).

use kul_render::GhostReason;

/// Top-level positioned shape: every canvas, every card, every edge,
/// in absolute pixel coordinates, in the canonical UI pattern's
/// arrangement.
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
    /// their anchoring marriage edges).
    pub cards: Vec<PositionedCard>,
    /// Every edge — birth, adoption, and marriage — with computed
    /// polyline geometry. Birth and adoption edges connect a marriage's
    /// child-attach anchor to one of the couple's children. Marriage
    /// edges are the unified marriage connector (ADR-0020): for monogamy
    /// a thick horizontal segment between the two adjacent spouse cards;
    /// for polygamy one thick edge per concurrent marriage routed from
    /// the hub card to each co-spouse.
    pub edges: Vec<PositionedEdge>,
}

/// One positioned person card. The visual primitive surface renderers
/// project into the canonical UI pattern's uniform-card shape (the uniform card).
#[derive(Debug, Clone)]
pub struct PositionedCard {
    /// Source-declaration id (`person <id>`). Stable across renders;
    /// carried so click-to-jump follow-ups (F10) can attach without
    /// changing the type.
    pub person_id: String,
    /// Canonical vs ghost. The discriminator a surface
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
    /// Display name (the uniform card's minimum).
    pub name: String,
}

/// Whether a [`PositionedCard`] is the person's canonical card
/// or a ghost anchoring a past structural fact.
///
/// Mirrors [`kul_render::SlotKind`] one-to-one but lives here so
/// downstream surface emitters (kul-svg, future native preview) don't
/// have to pull in `kul_render` just for the discriminator.
#[derive(Debug, Clone, Copy)]
pub enum SlotKind {
    Canonical,
    Ghost { reason: GhostReason },
}

/// One positioned parent-child or marriage edge with computed polyline
/// geometry.
///
/// Birth edges (solid) and adoption edges (dashed) connect a
/// marriage's child-attach anchor to one of its children. Marriage
/// edges (ADR-0020) are the unified marriage connector for both
/// monogamy (a thick horizontal segment between adjacent spouse cards)
/// and polygamy (one thick edge per concurrent marriage, hub →
/// co-spouse). The `kind` field discriminates and the polyline points
/// describe the orthogonal right-angle route a surface emits directly.
#[derive(Debug, Clone)]
pub struct PositionedEdge {
    /// Birth, adoption, or marriage.
    pub kind: EdgeKind,
    /// Routing variant — `InTree` for v1, `CrossTree` for future
    /// cross-component edges (F5).
    pub routing: EdgeRouting,
    /// Source-declaration id of the child this edge belongs to. For a
    /// marriage edge this is the joining spouse (monogamy) or co-spouse
    /// (polygamy).
    pub child_id: String,
    /// Source-declaration id of the marriage this edge connects to.
    pub marriage_id: String,
    /// Polyline points, in draw order. Each entry is an absolute
    /// pixel coordinate; the emitter writes them straight into
    /// `<polyline points="x1,y1 x2,y2 …" />`.
    pub points: Vec<(f64, f64)>,
    /// `true` iff this is a marriage edge for a marriage that carries an
    /// `end:` field (current-intimacy placement's canonical "ended" predicate). Surfaces add the
    /// `kul-edge--ended` class to render the connector translucent.
    /// Always `false` for birth / adoption edges and for polygamy
    /// marriage edges (un-ended by R14).
    pub ended: bool,
}

/// What kind of edge this is. Birth and adoption edges connect a
/// marriage to one of its children (edges encode link kind). Marriage edges are the unified
/// marriage connector (ADR-0020): for monogamy a thick horizontal
/// segment between the two adjacent spouse cards; for polygamy one edge
/// per concurrent marriage connecting the hub to each co-spouse.
#[derive(Debug, Clone, Copy)]
pub enum EdgeKind {
    /// Solid, thin (edges encode link kind).
    Birth,
    /// Dashed, thin (edges encode link kind).
    Adoption,
    /// Solid, thick — the unified marriage connector (ADR-0020).
    ///
    /// For **monogamy** (`hosted_marriages.len() == 1`) it is the
    /// horizontal segment spanning the inter-card gap between the two
    /// adjacent spouse cards, at the cards' vertical mid-height; the
    /// couple's children drop from its midpoint.
    ///
    /// For **polygamy** (`hosted_marriages.len() >= 2`) one edge is
    /// emitted per hosted marriage, routed hub-bottom → horizontal bus
    /// → co-spouse-top using the same orthogonal right-angle geometry as
    /// a birth edge.
    ///
    /// Visually distinguished from birth / adoption by a thicker stroke
    /// (set by the consuming surface stylesheet via the
    /// `kul-edge--marriage` class).
    Marriage,
}

/// How an edge is routed. Both variants emit the **same** orthogonal
/// right-angle polyline geometry and the same attachment points —
/// bar bottom-midpoint, horizontal bus at `card_top - config.bus_drop`,
/// child card top-midpoint — so the entire diagram follows one
/// consistent edge-routing pattern (the classical descendency tree). The discriminator exists to
/// give surface consumers a future re-theming hook (the emitted CSS
/// classes differ); the layout layer treats them identically. See
/// [ADR-0018](../../docs/adr/0018-canonical-layout-algorithm.md).
#[derive(Debug, Clone, Copy)]
pub enum EdgeRouting {
    /// Standard descendency-tree route: the child sits structurally
    /// inside this marriage's subtree, directly below the bar in the
    /// children row.
    InTree,
    /// Cross-tree route — both endpoints are positioned in the laid-out
    /// tree, but the child is not a structural descendant of this
    /// marriage's bar. The canonical exerciser is the cousin-marriage
    /// case (the within-family absorb rule): the joining cousin's birth-edge connects back to a
    /// sibling marriage already in the rendering context.
    CrossTree,
}

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
    /// Every fan connector — the trunk + branch + drops geometry that
    /// links a polygamy hub to each of its concurrent marriage bars
    /// (ADR-0027). One entry per hub. Empty when no person hosts ≥2
    /// concurrent marriages.
    pub fan_connectors: Vec<PositionedFanConnector>,
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

/// Fan-connector geometry linking a polygamy hub to each of its
/// concurrent marriage bars (ADR-0027). The fan kicks in whenever a
/// person hosts ≥2 concurrent marriages; for monogamy (N=1) the
/// classical hub-and-flanks layout still applies and no fan connector
/// is emitted.
///
/// One fan per hub. The geometry is decomposed into orthogonal
/// segments so the emitter can render each as its own polyline
/// without retracing: a vertical trunk down from the hub card's
/// bottom-midpoint, a horizontal branch spanning the per-marriage
/// column centres, and a vertical drop from the branch to each
/// marriage bar's top-midpoint. The visual weight matches the
/// marriage bar (a thicker stroke than the birth / adoption edges)
/// so the fan reads as one continuous "hub manifold," not as a stack
/// of independent edges.
#[derive(Debug, Clone)]
pub struct PositionedFanConnector {
    /// Source-declaration id of the polygamy hub. Stable across renders.
    pub hub_id: String,
    /// One polyline per orthogonal segment, in draw order. The first
    /// segment is the trunk-plus-branch path
    /// (hub bottom → trunk elbow → branch ends); each subsequent
    /// segment is a per-bar drop (branch row → bar top-midpoint).
    /// Splitting the fan into segments avoids the polyline-retrace
    /// problem that a single connected path would have at the
    /// branch / drop intersections.
    pub segments: Vec<Vec<(f64, f64)>>,
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

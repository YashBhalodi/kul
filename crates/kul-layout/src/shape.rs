//! [`PositionedShape`] and friends — the data form `kul-svg` and other
//! Rust-side surface adapters walk.
//!
//! These types are an **internal Rust seam**. They are deliberately
//! *not* `Serialize` and *not* schema-versioned: the wire shapes the
//! project pins are `RenderShape` (input) and the SVG string (output).
//! See [ADR-0016](../../docs/adr/0016-visualization-pipeline-crate-boundaries.md).
//!
//! ## Properties plumb through ([ADR-0021](../../docs/adr/0021-language-properties-plumb-to-svg.md))
//!
//! Every language property declared on a Person, Marriage, or
//! parenthood link (birth / adoption) is carried on these positioned
//! shapes as a plain field — display-ready, like [`PositionedCard::name`]
//! — so [`kul_svg`](../../kul_svg/index.html) can surface it as a
//! `data-*` attribute without re-deriving anything. Dates arrive
//! pre-formatted in their source `~YYYY[-MM[-DD]]` form; missing
//! optional values are `None` (the emitter omits the attribute
//! entirely).

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
///
/// Every Person property the language declares is carried here so the
/// emitter surfaces it as a `data-*` attribute (ADR-0021); only the
/// geometry (`x`, `y`, `width`, `height`) and the display `name` are
/// not themselves source properties.
#[derive(Debug, Clone)]
pub struct PositionedCard {
    /// Source-declaration id (`person <id>`). Stable across renders;
    /// carried so click-to-jump follow-ups (F10) can attach without
    /// changing the type. Surfaces as `data-person-id`.
    pub person_id: String,
    /// Canonical vs ghost. The discriminator a surface renderer keys
    /// on for the ghost visual vocabulary — the dotted border, the
    /// faded fill, and the surface-injected `↺` badge (chrome;
    /// ADR-0016). Surfaces as `data-kind` (plus `data-ghost-reason`
    /// for a ghost).
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
    /// Display name (the uniform card's minimum). The visible label;
    /// not emitted as a `data-*` attribute.
    pub name: String,
    /// Genealogical generation index (the canonical-family graph depth;
    /// roots at 0). Surfaces as `data-generation`.
    pub generation: u32,
    /// `male | female | other` — required by R03, so always present.
    /// Surfaces as `data-gender`. The canonical pattern keeps the card
    /// shape gender-neutral; this is the structural seam a surface MAY
    /// opt into (the uniform card).
    pub gender: &'static str,
    /// `family:` name part, if declared. Surfaces as `data-family`.
    pub family: Option<String>,
    /// `given:` name part, if declared. Surfaces as `data-given`.
    pub given: Option<String>,
    /// `born:` date in source form (`~YYYY[-MM[-DD]]`), if declared.
    /// Surfaces as `data-born`.
    pub born: Option<String>,
    /// `died:` date in source form, if declared. Surfaces as `data-died`;
    /// its absence is also the `data-is-alive="true"` predicate (a person
    /// is alive iff no `died:` is recorded).
    pub died: Option<String>,
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
/// co-spouse). The [`kind`](PositionedEdge::kind) discriminates and
/// carries each edge's declared properties; the polyline points
/// describe the orthogonal right-angle route a surface emits directly.
#[derive(Debug, Clone)]
pub struct PositionedEdge {
    /// What kind of edge this is, plus the per-kind declared properties
    /// that plumb to `data-*` attributes (ADR-0021).
    pub kind: EdgeKind,
    /// Polyline points, in draw order. Each entry is an absolute
    /// pixel coordinate; the emitter writes them straight into the
    /// `<path d="…">` route.
    pub points: Vec<(f64, f64)>,
    /// Source-declaration id of the marriage this edge belongs to. For
    /// a marriage edge it is the marriage itself; for a birth / adoption
    /// edge it is the parent marriage the child attaches to. Surfaces as
    /// `data-marriage-id`.
    pub marriage_id: String,
}

/// What kind of edge this is, with the declared properties each kind
/// carries.
///
/// Birth and adoption edges connect a marriage to one of its children
/// (edges encode link kind). Marriage edges are the unified marriage
/// connector (ADR-0020). The variant discriminator is the `data-link-kind`
/// value (`birth` / `adoption` / `marriage`); the fields are the
/// remaining `data-*` attributes for that kind. There is no longer a
/// routing discriminator — every edge routes with one orthogonal
/// geometry, so the former `EdgeRouting` future-hook was removed
/// ([ADR-0018](../../docs/adr/0018-canonical-layout-algorithm.md)).
#[derive(Debug, Clone)]
pub enum EdgeKind {
    /// Solid, thin biological parent-child edge.
    Birth {
        /// Source-declaration id of the child. Surfaces as `data-child-id`.
        child_id: String,
        /// `true` iff the edge terminates on a past-bio child-ghost (the
        /// `birth` link the current-intimacy chain did not select).
        /// Surfaces as `data-is-past`.
        is_past: bool,
    },
    /// Dashed, thin adoptive parent-child edge.
    Adoption {
        /// Source-declaration id of the child. Surfaces as `data-child-id`.
        child_id: String,
        /// `true` iff the edge terminates on a past-adoption child-ghost
        /// (a demoted adoption). Surfaces as `data-is-past`.
        is_past: bool,
        /// `start:` of the adoption sub-statement, source form. Required
        /// by the grammar, so present whenever the link resolves;
        /// surfaces as `data-adoption-start`.
        start: Option<String>,
        /// `end:` of the adoption sub-statement, source form, if declared.
        /// Surfaces as `data-adoption-end`.
        end: Option<String>,
    },
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
    /// (set by the consuming surface stylesheet against the
    /// `data-link-kind="marriage"` selector).
    Marriage {
        /// Source-declaration id of the host (first-listed spouse).
        /// Surfaces as `data-host-id`.
        host_id: String,
        /// Source-declaration id of the joining spouse (second-listed).
        /// Surfaces as `data-joining-id`.
        joining_id: String,
        /// `start:` date in source form. Required by the grammar, so
        /// always present. Surfaces as `data-start`.
        start: String,
        /// `end:` date in source form, if the marriage ended.
        /// Surfaces as `data-end`.
        end: Option<String>,
        /// `end_reason:` enum (v1: `divorce`), if the marriage ended.
        /// Surfaces as `data-end-reason`.
        end_reason: Option<String>,
        /// `true` iff the marriage carries an `end:` field (current-intimacy
        /// placement's "ended" predicate — death is on the person, not the
        /// marriage). Surfaces as `data-is-ended`. Always `false` for a
        /// polygamy marriage (un-ended by R14).
        is_ended: bool,
    },
}

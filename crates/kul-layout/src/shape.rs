//! [`PositionedShape`] and friends — internal Rust seam (not
//! `Serialize`, not schema-versioned; ADR-0016).
//!
//! Every language property declared on a Person, Marriage, or
//! parenthood link is carried as a plain field, display-ready, so
//! `kul-svg` can surface it as a `data-*` attribute without
//! re-deriving (ADR-0021). Dates arrive pre-formatted in source
//! `~YYYY[-MM[-DD]]` form.

use kul_render::GhostReason;

/// Top-level positioned shape. Coordinates are top-left origin; `width`
/// and `height` describe the outer bounding box (including padding).
#[derive(Debug, Clone)]
pub struct PositionedShape {
    pub width: f64,
    pub height: f64,
    pub cards: Vec<PositionedCard>,
    /// Birth, adoption, and marriage edges (ADR-0020) with computed
    /// polyline geometry.
    pub edges: Vec<PositionedEdge>,
}

/// One positioned person card. Every declared Person property is
/// carried as a plain field for `data-*` plumbing (ADR-0021).
#[derive(Debug, Clone)]
pub struct PositionedCard {
    pub person_id: String,
    pub kind: SlotKind,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub name: String,
    pub generation: u32,
    /// `male | female | other` — required by R03.
    pub gender: &'static str,
    pub family: Option<String>,
    pub given: Option<String>,
    pub born: Option<String>,
    /// Absence is the `data-is-alive="true"` predicate.
    pub died: Option<String>,
}

/// Canonical vs ghost. Mirrors [`kul_render::SlotKind`] but lives here
/// so downstream surface emitters don't have to pull in `kul_render`.
#[derive(Debug, Clone, Copy)]
pub enum SlotKind {
    Canonical,
    Ghost { reason: GhostReason },
}

/// One positioned parent-child or marriage edge with computed
/// polyline geometry. The polyline points are absolute pixel
/// coordinates the emitter writes straight into `<path d="…">`.
#[derive(Debug, Clone)]
pub struct PositionedEdge {
    pub kind: EdgeKind,
    pub points: Vec<(f64, f64)>,
    /// For a marriage edge, the marriage itself; for birth/adoption,
    /// the parent marriage the child attaches to.
    pub marriage_id: String,
}

/// Edge kind discriminator (the `data-link-kind` value) plus the
/// per-kind declared properties that plumb to `data-*` (ADR-0021).
#[derive(Debug, Clone)]
pub enum EdgeKind {
    /// Solid, thin biological parent-child edge.
    Birth {
        child_id: String,
        /// `true` iff terminating on a past-bio child-ghost.
        is_past: bool,
    },
    /// Dashed, thin adoptive parent-child edge.
    Adoption {
        child_id: String,
        /// `true` iff terminating on a past-adoption child-ghost.
        is_past: bool,
        start: Option<String>,
        end: Option<String>,
    },
    /// Unified marriage connector (ADR-0020). For monogamy a horizontal
    /// segment between the two adjacent spouse cards; for polygamy one
    /// edge per hosted marriage routed hub-bottom → bus → co-spouse-top.
    Marriage {
        host_id: String,
        joining_id: String,
        start: String,
        end: Option<String>,
        end_reason: Option<String>,
        /// `data-is-ended`. Always `false` for a polygamy marriage (R14).
        is_ended: bool,
    },
}

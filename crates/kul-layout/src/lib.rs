//! Positioning pass for the canonical UI pattern.
//!
//! `kul-render` produces the canonical UI pattern's *structural* data
//! ([`kul_render::RenderShape`]) — components, marriage branches, card
//! slots, ghosts, nested sub-trees — without any positional
//! information ([ADR-0017](../../docs/adr/0017-render-shape-schema-and-versioning.md)).
//! Surface renderers still need to decide *where* every card and edge
//! segment goes on a 2D plane. This crate is that step: input is a
//! [`kul_render::RenderShape`], output is a [`PositionedShape`] whose
//! cards and edges carry absolute pixel coordinates plus computed
//! polyline geometry.
//!
//! # Two internal layers
//!
//! - [`walker`] — the canonical Reingold–Tilford–Walker port (Buchheim
//!   et al. 2002, O(n)). Takes an internal layout tree and emits
//!   preliminary x-coordinates with sibling-subtree collision
//!   avoidance.
//! - [`adapter`] — wraps Walker's for kul's pattern: thick marriage
//!   edges between adjacent spouses, ghost slots at the host's
//!   birth-family position per current-intimacy placement, generation rows from generation
//!   indices, orthogonal right-angle edge routing (`InTree` and
//!   `CrossTree` share one geometry; see [`EdgeRouting`]).
//!
//! # Internal seam, not a wire shape
//!
//! [`PositionedShape`] is **not** `Serialize`, not schema-versioned, and
//! not part of any cross-process contract. The wire shapes the project
//! pins are [`kul_render::RenderShape`] (input) and the SVG string
//! produced by `kul-svg` (output). See
//! [ADR-0016](../../docs/adr/0016-visualization-pipeline-crate-boundaries.md) for the
//! rationale.
//!
//! # Failure handling
//!
//! Failure render shapes are not positionable; this crate's surface
//! takes the success arm only. The LSP adapter shells out before
//! calling [`layout`] when the upstream pipeline produced a failure.

pub mod adapter;
pub mod walker;

mod metrics;
mod shape;

pub use metrics::LayoutConfig;
pub use shape::{EdgeKind, EdgeRouting, PositionedCard, PositionedEdge, PositionedShape, SlotKind};

use kul_render::RenderShape;

/// Run the positioning pipeline against a success [`RenderShape`].
///
/// Returns a [`PositionedShape`] whose cards and edges carry absolute
/// pixel coordinates. Theming and emission are downstream concerns owned
/// by surface adapters (today, `kul-svg`; tomorrow, alternative
/// renderers).
///
/// # Panics
///
/// Panics if `shape` is the failure arm. The LSP shells out to the
/// failure path before reaching here; callers that build a
/// [`RenderShape`] from other sources should pattern-match on
/// `as_success()` first.
pub fn layout(shape: &RenderShape, config: &LayoutConfig) -> PositionedShape {
    let success = shape
        .as_success()
        .expect("kul_layout::layout requires a success RenderShape");
    adapter::lay_out(success, config)
}

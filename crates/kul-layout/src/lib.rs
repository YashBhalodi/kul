//! Positioning pass for the canonical UI pattern.
//!
//! Input is a [`kul_render::RenderShape`]; output is a
//! [`PositionedShape`] whose cards and edges carry absolute pixel
//! coordinates and computed polyline geometry.
//!
//! Two internal layers:
//!
//! - [`walker`] — Reingold–Tilford–Walker port (Buchheim et al. 2002).
//! - [`adapter`] — kul-specific layout (ADR-0018).
//!
//! [`PositionedShape`] is an internal seam, not a wire shape: not
//! `Serialize`, not schema-versioned (ADR-0016). Failure render shapes
//! are not positionable; callers must pattern-match on `as_success()`
//! first.

pub mod adapter;
pub mod walker;

mod metrics;
mod shape;

pub use metrics::LayoutConfig;
pub use shape::{EdgeKind, PositionedCard, PositionedEdge, PositionedShape, SlotKind};

use kul_render::RenderShape;

/// Run the positioning pipeline against a success [`RenderShape`].
///
/// # Panics
///
/// Panics if `shape` is the failure arm.
pub fn layout(shape: &RenderShape, config: &LayoutConfig) -> PositionedShape {
    let success = shape
        .as_success()
        .expect("kul_layout::layout requires a success RenderShape");
    adapter::lay_out(success, config)
}

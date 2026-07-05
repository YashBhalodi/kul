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
//! are not positionable; [`layout`] takes `&SuccessRender`, so the type
//! enforces this — callers still match `RenderShape` to extract the
//! success arm, but the invariant is a compile-time guarantee, not a
//! convention to remember.

pub mod adapter;
pub mod walker;

mod metrics;
mod shape;

pub use metrics::LayoutConfig;
pub use shape::{EdgeKind, PositionedCard, PositionedEdge, PositionedShape, SlotKind};

use kul_render::SuccessRender;

/// Run the positioning pipeline against a success render shape.
pub fn layout(success: &SuccessRender, config: &LayoutConfig) -> PositionedShape {
    adapter::lay_out(success, config)
}

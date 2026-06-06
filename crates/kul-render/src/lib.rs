//! The canonical UI pattern as data.
//!
//! Projects the kinship-native [`ExportEnvelope`] into a [`RenderShape`]
//! whose hierarchy and primitives (components, marriage branches, card
//! slots, ghost cards) match the canonical UI pattern's data form
//! one-to-one. Every pattern decision — canonical-vs-ghost, generation
//! row, component order — is precomputed so a surface renderer is a
//! walker of the shape, not a re-implementer of the pattern.
//!
//! Reads only the kinship-native graph (never AST or
//! [`kul_core::semantic::ResolvedDocument`]); see ADR-0016. Failure
//! envelopes pass through verbatim as [`RenderShape::Failure`].
//!
//! - [`compute`] — entry from a [`CheckResult`].
//! - [`transform`] — pure transform over an already-exported envelope,
//!   so fabricated fixtures can drive the projection in tests.

pub mod shape;

mod build;

use kul_core::CheckResult;
use kul_core::export::{ExportEnvelope, ExportOptions, export};

pub use shape::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind, FailureRender, GhostReason, MarriageBar,
    MarriageBranch, PersonCard, RenderShape, SlotKind, SuccessRender,
};

/// Schema version for [`RenderShape`]. Bumped only when a schema change
/// would silently mis-represent data for older consumers (ADR-0010 / ADR-0017);
/// new optional fields, ghost reasons, or component kinds do not bump.
pub const RENDER_SCHEMA_VERSION: u32 = 3;

/// Export-then-project. Exports with `with_positions: true` so a surface
/// renderer can map clicks back to source declarations.
pub fn compute(check: &CheckResult) -> RenderShape {
    let envelope = export(
        check,
        ExportOptions {
            with_positions: true,
            ..ExportOptions::default()
        },
    );
    transform(&envelope)
}

/// Project a kinship-native [`ExportEnvelope`] into a [`RenderShape`].
/// Cytoscape envelopes are rejected — Cytoscape is a sibling projection,
/// not an input here (ADR-0016).
pub fn transform(envelope: &ExportEnvelope) -> RenderShape {
    match envelope {
        ExportEnvelope::Failure(f) => RenderShape::Failure(FailureRender {
            ok: false,
            diagnostics: f.diagnostics.clone(),
        }),
        ExportEnvelope::Success(s) => {
            let native = s
                .graph
                .as_native()
                .expect("kul-render::transform requires the kinship-native graph shape");
            let (components, edges) = build::build(native);
            RenderShape::Success(SuccessRender {
                ok: true,
                schema: RENDER_SCHEMA_VERSION,
                kul: s.kul.clone(),
                components,
                edges,
            })
        }
    }
}

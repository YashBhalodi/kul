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
use kul_core::export::{ExportEnvelope, ExportOptions, ExportedDiagnostic, export};

pub use shape::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind, FailureRender, GhostReason, MarriageBar,
    MarriageBranch, PersonCard, RenderShape, SlotKind, SuccessRender,
};

/// Schema version for [`RenderShape`]. Bumped only when a schema change
/// would silently mis-represent data for older consumers (ADR-0010 / ADR-0017);
/// new optional fields, ghost reasons, or component kinds do not bump.
pub const RENDER_SCHEMA_VERSION: u32 = 3;

/// Diagnostic code when [`transform`] receives a non-native (e.g. Cytoscape)
/// success envelope — caller misuse of the kinship-native contract (ADR-0016),
/// projected as a failure envelope like the ADR-0032 depth-cap path.
const KUL_V02: &str = "KUL-V02";

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
            let Some(native) = s.graph.as_native() else {
                // Programming misuse: a Cytoscape (or other non-native)
                // success envelope is not an input to this projection.
                // Recover with a stable diagnostic rather than panicking,
                // matching the ADR-0032 depth-cap failure shape.
                return RenderShape::Failure(FailureRender {
                    ok: false,
                    diagnostics: vec![non_native_graph_diagnostic()],
                });
            };
            match build::build(native) {
                Ok((components, edges)) => RenderShape::Success(SuccessRender {
                    ok: true,
                    schema: RENDER_SCHEMA_VERSION,
                    kul: s.kul.clone(),
                    components,
                    edges,
                }),
                // A lineage past the depth cap downgrades to a failure
                // shape rather than overflowing the stack downstream
                // (ADR-0032). Layout only ever runs on the success arm, so
                // this single guard bounds every recursive pass.
                Err(diagnostic) => RenderShape::Failure(FailureRender {
                    ok: false,
                    diagnostics: vec![*diagnostic],
                }),
            }
        }
    }
}

fn non_native_graph_diagnostic() -> ExportedDiagnostic {
    ExportedDiagnostic {
        code: KUL_V02.to_string(),
        severity: "error",
        message: "render transform requires the kinship-native graph shape; \
                  cytoscape envelopes are a sibling projection, not an input"
            .to_string(),
        // Caller misuse of the transform contract — unanchored, like KUL-V01.
        primary: None,
        related: Vec::new(),
    }
}

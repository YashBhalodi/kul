//! Stage 2 of the canonical renderer pipeline.
//!
//! Transforms the kinship-native [`ExportEnvelope`] (Stage 1) into a
//! [`RenderShape`] that realizes every canonical UI pattern principle
//! (P1–P16). The output is hierarchical card slots plus a flat edge list,
//! with generation indices computed and ghost cards (P8 past marriages,
//! P16 past adoptions) emitted up front so Stage 3 layout engines never
//! re-derive layout-meaningful facts.
//!
//! # Two surfaces
//!
//! - [`compute`] — full pipeline. Takes a [`CheckResult`], calls
//!   [`kul_core::export::export`] with positions on, then transforms the
//!   resulting envelope. The shape every downstream renderer wants.
//! - [`transform`] — pure transformer over an already-exported envelope.
//!   Surfaced so fabricated [`ExportEnvelope`] fixtures can drive the
//!   transformation in tests without having to round-trip through a `.kul`
//!   source.
//!
//! The internal flow is deliberately staged: Stage 2 reads only the
//! kinship-native shape (the audit in #117 verified that shape carries
//! every fact Stage 2 needs); it never reaches back into the AST or
//! [`kul_core::semantic::ResolvedDocument`]. That keeps the crate
//! boundary clean and the canonical UI pattern co-evolvable independent
//! of the rest of the toolchain — see [ADR
//! 0016](../../docs/adr/0016-kul-render-crate-boundary.md).
//!
//! # Failure handling
//!
//! If the input [`ExportEnvelope`] is a failure envelope, [`transform`]
//! and [`compute`] return [`RenderShape::Failure`] carrying the same
//! diagnostics — the canonical UI pattern only meaningfully applies to a
//! valid document.

pub mod shape;

mod build;

use kul_core::CheckResult;
use kul_core::export::{ExportEnvelope, ExportOptions, export};

pub use shape::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind, FailureRender, GhostReason, MarriageBar,
    MarriageBranch, PersonCard, RenderShape, SlotKind, SuccessRender,
};

/// Schema version for [`RenderShape`].
///
/// Bumped under the same policy as [`kul_core::export::SCHEMA_VERSION`]
/// (per [ADR-0010](../../docs/adr/0010-export-schema-versioning.md)):
/// a new integer is allocated only when downstream renderers might
/// silently mis-represent data by ignoring a new construct. Adding a new
/// optional field, a new ghost reason, or a new component kind value
/// does NOT bump the schema — consumers treat them as forward-compatible
/// additions. See [ADR-0017](../../docs/adr/0017-render-shape-schema-and-versioning.md).
pub const RENDER_SCHEMA_VERSION: u32 = 1;

/// Run the full Stage-1-plus-Stage-2 pipeline against a checked project.
///
/// Calls [`kul_core::export::export`] with `with_positions: true` (Stage 2
/// surfaces source spans on every slot so a Stage 3 renderer can map
/// clicks back to the source) and feeds the resulting envelope through
/// [`transform`]. If the export fails, the failure envelope's diagnostics
/// pass through verbatim.
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
///
/// Pure transformer. Stage 2 reads only the kinship-native graph
/// (`persons`, `marriages`, `parenthoodLinks`); the envelope's
/// `cytoscape` shape is rejected (Cytoscape is a sibling Stage-2-style
/// projection, not an input to this one). See
/// [ADR-0016](../../docs/adr/0016-kul-render-crate-boundary.md).
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

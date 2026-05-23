//! The canonical UI pattern as data.
//!
//! `kul-core::export` produces a kinship-native graph — `persons`,
//! `marriages`, `parenthood_links` — mirroring the language primitives
//! one-to-one. That shape is faithful to what the source *says*; it is
//! not yet shaped for what the canonical UI pattern
//! ([`docs/canonical-ui-pattern.md`](../../docs/canonical-ui-pattern.md))
//! *draws*. This crate is the projection between the two: input is the
//! kinship-native [`ExportEnvelope`], output is a [`RenderShape`] whose
//! hierarchy and primitives (components, marriage branches, card slots,
//! ghost cards, P6 nested birth-family sub-trees) match the pattern's
//! data form one-to-one.
//!
//! Every pattern decision — which spouse is canonical and which is a
//! ghost (P8, P16), which slot lives at which generation row (P1),
//! how components arrange in source order (P12), where P6 recursive
//! nesting terminates (P11) — is computed up front and surfaced as
//! data, so a surface renderer (VSCode preview, web visualizer,
//! anything else downstream) becomes a walker of the shape, not a
//! re-implementer of the pattern.
//!
//! # Two surfaces
//!
//! - [`compute`] — convenience entry point. Takes a [`CheckResult`],
//!   calls [`kul_core::export::export`] with positions on, then runs
//!   [`transform`] over the resulting envelope. The shape every
//!   downstream consumer wants when starting from a checked project.
//! - [`transform`] — pure transformer over an already-exported
//!   envelope. Surfaced so fabricated [`ExportEnvelope`] fixtures can
//!   drive the projection in tests without having to round-trip
//!   through a `.kul` source.
//!
//! The kinship-native shape is the only thing read here — never the
//! AST or [`kul_core::semantic::ResolvedDocument`]. The audit in
//! [#117] verified that shape carries every fact the canonical UI
//! pattern needs, and the rationale for keeping it that way is
//! recorded in [ADR-0016](../../docs/adr/0016-kul-render-crate-boundary.md).
//!
//! # Failure handling
//!
//! If the input [`ExportEnvelope`] is a failure envelope, [`transform`]
//! and [`compute`] return [`RenderShape::Failure`] carrying the same
//! diagnostics — the canonical UI pattern only meaningfully applies to
//! a valid document.
//!
//! [#117]: https://github.com/YashBhalodi/kul/issues/117

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

/// Run the export-then-project pipeline against a checked project and
/// return its [`RenderShape`].
///
/// Calls [`kul_core::export::export`] with `with_positions: true` —
/// source spans propagate through to the render shape so a surface
/// renderer can map a click on a card back to its source declaration —
/// then feeds the envelope through [`transform`]. If the export fails,
/// the failure envelope's diagnostics pass through verbatim.
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
/// Pure transformer. Reads only the kinship-native graph (`persons`,
/// `marriages`, `parenthoodLinks`); the envelope's `cytoscape` shape
/// is rejected — Cytoscape is a sibling projection of the kinship-
/// native graph, not an input to this one. See
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

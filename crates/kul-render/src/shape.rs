//! [`RenderShape`] — the canonical UI pattern's data form.
//!
//! Where the kinship-native export is shaped to mirror what the source
//! *says*, this shape is shaped to mirror what the canonical UI
//! pattern *draws*. Every layout-meaningful fact the pattern's
//! principles compute — generation index, canonical vs. ghost slot,
//! component grouping, P6 nested birth-family sub-trees — is a field
//! in the shape, so a surface renderer becomes a walker of the data
//! rather than a re-implementer of the pattern. The schema-versioning
//! contract is in [ADR-0017](../../docs/adr/0017-render-shape-schema-and-versioning.md).

use kul_core::export::{ExportedDate, ExportedDiagnostic};
use serde::Serialize;

/// Top-level [`RenderShape`]. Either a success payload (the
/// pattern-shaped graph) or a failure payload (the same diagnostic
/// list the input [`kul_core::export::ExportEnvelope::Failure`]
/// carried).
///
/// Untagged at the wire level: the `ok` boolean discriminates without
/// the consumer having to inspect other keys.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum RenderShape {
    Success(SuccessRender),
    Failure(FailureRender),
}

impl RenderShape {
    /// Test helper: returns the success envelope, or `None` on failure.
    pub fn as_success(&self) -> Option<&SuccessRender> {
        match self {
            RenderShape::Success(s) => Some(s),
            RenderShape::Failure(_) => None,
        }
    }

    /// Test helper: returns the failure envelope, or `None` on success.
    pub fn as_failure(&self) -> Option<&FailureRender> {
        match self {
            RenderShape::Failure(f) => Some(f),
            RenderShape::Success(_) => None,
        }
    }
}

/// Success envelope. Carries the pattern-shaped components and edges
/// plus the same `schema` / `kul` discriminators the export envelope
/// uses (per [ADR-0010](../../docs/adr/0010-export-schema-versioning.md)).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessRender {
    /// Always `true`. Consumer-facing discriminator.
    pub ok: bool,
    /// Render-shape schema version. See
    /// [`crate::RENDER_SCHEMA_VERSION`].
    pub schema: u32,
    /// Kul language version of the source document — passed through
    /// from the input [`kul_core::export::ExportEnvelope::Success`] so
    /// consumers can warn on version drift without re-reading the
    /// manifest.
    pub kul: String,
    /// Top-level layout components, in P12 order: by the source position
    /// of each component's first relevant declaration.
    pub components: Vec<Component>,
    /// Flat list of every parent-child edge in the document — birth
    /// (P5 solid) and adoption (P5 dashed) alike. Routing (within-tree
    /// vs. cross-tree) is renderer policy under P5/P11 and not pinned
    /// here.
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureRender {
    /// Always `false`. Consumer-facing discriminator.
    pub ok: bool,
    /// Verbatim copy of the input envelope's diagnostic list.
    pub diagnostics: Vec<ExportedDiagnostic>,
}

/// One top-level component in the [`RenderShape`].
///
/// A component is a connected unit of canonical-card placement: all the
/// canonical cards inside it anchor (directly or transitively) at the
/// component's root. Cross-component edges may still connect components
/// (e.g. an adopted child's bio-link to a marriage in a sibling
/// component). Per P12, components arrange left-to-right by `sourceOrder`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    /// Stable synthetic id (`comp-1`, `comp-2`, …) so cross-references
    /// in the edge list don't depend on array order.
    pub id: String,
    /// Byte offset of the component's first relevant declaration (P12):
    /// the earliest-declared marriage if the component has one,
    /// otherwise the earliest-declared person.
    pub source_order: usize,
    pub kind: ComponentKind,
}

/// What a [`Component`] is shaped like at its root.
///
/// Three variants cover every canonical case:
///
/// - [`ComponentKind::FamilyTree`] — a marriage and its descendants.
///   The marriage at the root is either a floating mini-component
///   (P8 fallback: host has no birth family) or a marriage whose host's
///   canonical-family root is itself the root of this component.
/// - [`ComponentKind::OrphanPerson`] — a single canonical card with no
///   anchor (P13 declared-with-no-edges orphans, plus the P8 fallback
///   case of a joining spouse whose marriage ended and who has no birth
///   family declared, e.g. Bob in `examples/03-three-generations/`).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
// Both variants box their payload to keep the enum compact — see
// `clippy::large-enum-variant`. The boxing is invisible on the wire
// (serde flattens `Box<T>` transparently).
pub enum ComponentKind {
    FamilyTree { root: Box<MarriageBranch> },
    OrphanPerson { card: Box<CardSlot> },
}

/// A marriage bar plus the children directly below it.
///
/// `MarriageBranch` is the recursive building block of a family tree:
/// each child may itself host marriages (P11 absorb rule applied
/// uniformly), and each such hosted marriage is another `MarriageBranch`
/// nested at the child's slot.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarriageBranch {
    pub bar: MarriageBar,
    /// Direct children of `bar` (biological or adopted), in declaration
    /// order. A child who hosts marriages of their own carries those
    /// marriages as `personCard.hostedMarriages`.
    pub children: Vec<PersonCard>,
}

/// A person card slot in the layout, plus any marriages that branch from
/// this slot.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonCard {
    pub slot: CardSlot,
    /// Marriages this person hosts, in declaration order. Each hosted
    /// marriage's bar nests visually at this slot (P8: the bar's
    /// canonical location is the host's birth-family slot). For a
    /// canonical child the host's bar-slot is canonical too; for a host
    /// who has moved on (newer current intimacy elsewhere) the host's
    /// bar-slot becomes a ghost.
    pub hosted_marriages: Vec<MarriageBranch>,
}

/// A marriage bar in the layout.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarriageBar {
    /// Stable id from the input document (`marriage <id>`).
    pub marriage_id: String,
    /// Generation row this bar sits on — derived from the host's
    /// canonical generation under the canonical-family graph.
    pub generation: u32,
    /// Source-declaration id of the host (first-listed spouse, P3).
    pub host_id: String,
    /// Source-declaration id of the joining spouse (second-listed, P3).
    pub joining_id: String,
    /// Slot for the host at the bar — canonical if the host hasn't
    /// moved on (no newer current intimacy), ghost otherwise (P8).
    pub host_slot: CardSlot,
    /// Slot for the joining spouse at the bar — same canonical/ghost
    /// rule as `host_slot`.
    pub joining_slot: CardSlot,
    pub start: ExportedDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
    /// `true` iff the marriage carries an `end:` field. Per P8 this is
    /// the canonical "ended" predicate (death is on the person, not
    /// the marriage); reified here so consumers don't have to re-
    /// derive it from `end.is_some()`.
    pub ended: bool,
    /// Joining spouse's birth-family sub-tree (P6 recursive nesting),
    /// when the joining spouse has a birth family that isn't already
    /// being rendered in this component's context. Per P6 termination,
    /// this is `None` when the recursion would re-enter the current
    /// rendering context (cousin / sibling marriage, P11).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joining_nested_birth_family: Option<Box<MarriageBranch>>,
}

/// One canonical or ghost card slot. The single visual primitive
/// downstream surfaces render into the canonical UI pattern's
/// "uniform card" (P15).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardSlot {
    pub person_id: String,
    pub kind: SlotKind,
    /// Layout-row index. For a canonical card this is the person's
    /// generation under the canonical-family graph; for a ghost it is
    /// the generation of the bar / row the ghost anchors. See
    /// [`MarriageBar::generation`].
    pub generation: u32,
    pub name: String,
    pub gender: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub born: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub died: Option<ExportedDate>,
}

/// Whether a [`CardSlot`] is the person's canonical card (P2 — exactly
/// one per person) or a ghost (P8, P16) anchoring a past structural
/// fact.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SlotKind {
    /// The canonical card. Exactly one per declared person, per P2.
    Canonical,
    /// A ghost card. Mute per P10 — connects only to the marriage /
    /// adoption bar it anchors.
    Ghost { reason: GhostReason },
}

/// Why a ghost card was emitted. The discriminator a surface
/// renderer keys on for the dotted-border / faded-fill / `↺`-badge
/// visual vocabulary.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GhostReason {
    /// P8 ghost: a spouse of a past marriage that produced children,
    /// or of a past marriage whose host moved on. Anchors the bar's
    /// children edges.
    PastMarriage,
    /// P16 ghost: child-ghost at a past adoptive family. The canonical
    /// card lives at the most-recent adoption; each prior adoption's
    /// bar gets one child-ghost connected by a dashed edge.
    PastAdoption,
}

/// One parent-child edge.
///
/// Birth edges are solid; adoption edges dashed (P5). Cross-component
/// edges (a child whose canonical family is in one component but
/// whose other parent-link points to a marriage in another component)
/// are represented here uniformly — routing geometry is renderer
/// policy, decided downstream based on whether the endpoints sit in
/// the same component.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub kind: EdgeKind,
    pub child_id: String,
    pub marriage_id: String,
    /// `start:` of an adoption sub-statement. Absent for biological
    /// edges and present for adoptive ones (mirrors the export shape).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ExportedDate>,
}

/// Whether an [`Edge`] is a biological (solid) or adoptive (dashed)
/// parent-child link, per P5.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind {
    Birth,
    Adoption,
}

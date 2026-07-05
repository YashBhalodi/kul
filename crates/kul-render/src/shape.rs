//! [`RenderShape`] — the canonical UI pattern's data form.
//!
//! Every layout-meaningful fact (generation index, canonical/ghost slot,
//! component grouping) is a field, so a surface renderer walks the data
//! rather than re-implementing the pattern. Schema-versioning contract
//! in ADR-0017.

use kul_core::export::{ExportedDate, ExportedDiagnostic};
use serde::Serialize;

/// Top-level [`RenderShape`]. Untagged on the wire — the `ok` boolean
/// discriminates success from failure.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum RenderShape {
    Success(SuccessRender),
    Failure(FailureRender),
}

impl RenderShape {
    pub fn as_success(&self) -> Option<&SuccessRender> {
        match self {
            RenderShape::Success(s) => Some(s),
            RenderShape::Failure(_) => None,
        }
    }

    pub fn as_failure(&self) -> Option<&FailureRender> {
        match self {
            RenderShape::Failure(f) => Some(f),
            RenderShape::Success(_) => None,
        }
    }
}

/// Success envelope (ADR-0010 schema/kul discriminators).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessRender {
    /// Always `true`. Consumer-facing discriminator.
    pub ok: bool,
    /// See [`crate::RENDER_SCHEMA_VERSION`].
    pub schema: u32,
    /// Kul language version of the source — for version-drift warnings.
    pub kul: String,
    /// Layout components in source order.
    pub components: Vec<Component>,
    /// All parent-child edges (birth solid, adoption dashed). Routing
    /// geometry is renderer policy.
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureRender {
    /// Always `false`. Consumer-facing discriminator.
    pub ok: bool,
    pub diagnostics: Vec<ExportedDiagnostic>,
}

/// A connected unit of canonical-card placement. Cross-component edges
/// (e.g. an adopted child's bio-link) are still possible.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    /// Stable synthetic id (`comp-1`, `comp-2`, …) so cross-references
    /// in the edge list don't depend on array order.
    pub id: String,
    /// Byte offset of the component's first relevant declaration:
    /// earliest marriage if any, else earliest person.
    pub source_order: usize,
    pub kind: ComponentKind,
}

/// What a [`Component`] is shaped like at its root.
///
/// - [`ComponentKind::FamilyTree`] — a root `PersonCard` (the outermost
///   canonical host) with branching marriage bars. For a past-ended
///   floating bar with no canonical host, the root is a `PastMarriage`
///   ghost rooted at the declared host (ADR-0017).
/// - [`ComponentKind::OrphanPerson`] — a single unanchored canonical card.
// Payloads boxed for `clippy::large-enum-variant`; serde flattens `Box<T>`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ComponentKind {
    FamilyTree { root: Box<PersonCard> },
    OrphanPerson { card: Box<CardSlot> },
}

/// A marriage bar plus its direct children. Recursive building block of
/// a family tree: each child's hosted marriages are further branches.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarriageBranch {
    pub bar: MarriageBar,
    /// Direct children of `bar`, declaration order.
    pub children: Vec<PersonCard>,
}

/// A card slot plus the marriages branching from it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonCard {
    pub slot: CardSlot,
    /// Marriages this person hosts, declaration order. Each bar nests
    /// at this slot; the host's bar-slot is canonical when the host
    /// hasn't moved on, ghost otherwise.
    pub hosted_marriages: Vec<MarriageBranch>,
}

/// A marriage bar. The host face is implicit (the parent `PersonCard.slot`);
/// only the joining slot is reified since it may independently be canonical
/// or a ghost (ADR-0017).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarriageBar {
    pub marriage_id: String,
    pub generation: u32,
    pub host_id: String,
    pub joining_id: String,
    pub joining_slot: CardSlot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
    /// Reified `end.is_some()` so consumers don't re-derive it. Death is
    /// on the person, not the marriage — `ended` is the canonical predicate.
    pub ended: bool,
}

/// One canonical or ghost card slot — the uniform visual primitive of
/// the canonical UI pattern.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardSlot {
    pub person_id: String,
    pub kind: SlotKind,
    /// For a canonical card, the person's generation; for a ghost,
    /// the row of the bar it anchors.
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

/// Whether a [`CardSlot`] is the person's canonical card (exactly one
/// per person) or a ghost anchoring a past structural fact.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SlotKind {
    /// Exactly one per declared person.
    Canonical,
    /// Mute — connects only to the bar it anchors.
    Ghost { reason: GhostReason },
}

/// Why a ghost card was emitted. Surface renderers key on this for the
/// ghost visual vocabulary (dotted border, faded fill, `↺` badge — chrome
/// per ADR-0016).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GhostReason {
    /// Spouse of a past marriage (produced children or host moved on).
    PastMarriage,
    /// Child at a non-canonical adoption (most-recent adoption wins).
    PastAdoption,
    /// Child at the bio family when current intimacy selected an adoption.
    PastBirth,
}

/// One parent-child edge. Birth solid, adoption dashed. Cross-component
/// edges are uniform here; routing geometry is renderer policy.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub kind: EdgeKind,
    pub child_id: String,
    pub marriage_id: String,
    /// `start:` of an adoption sub-statement; absent for biological edges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ExportedDate>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind {
    Birth,
    Adoption,
}

//! [`RenderShape`] — the canonical UI pattern's data form.
//!
//! Where the kinship-native export is shaped to mirror what the source
//! *says*, this shape is shaped to mirror what the canonical UI
//! pattern *draws*. Every layout-meaningful fact the pattern's
//! principles compute — generation index, canonical vs. ghost slot,
//! component grouping, nested birth-family sub-trees — is a field
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
    /// Top-level layout components, in source order: by the source position
    /// of each component's first relevant declaration.
    pub components: Vec<Component>,
    /// Flat list of every parent-child edge in the document — birth
    /// (solid) and adoption (dashed) alike, per edges encode link kind.
    /// Geometry and routing are renderer policy and not pinned here.
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
/// component). Per source order, components arrange left-to-right by `sourceOrder`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    /// Stable synthetic id (`comp-1`, `comp-2`, …) so cross-references
    /// in the edge list don't depend on array order.
    pub id: String,
    /// Byte offset of the component's first relevant declaration (source order):
    /// the earliest-declared marriage if the component has one,
    /// otherwise the earliest-declared person.
    pub source_order: usize,
    pub kind: ComponentKind,
}

/// What a [`Component`] is shaped like at its root.
///
/// Two variants cover every canonical case:
///
/// - [`ComponentKind::FamilyTree`] — a `PersonCard` and its
///   descendants. The root `PersonCard` is the outermost canonical
///   host of the component, with one or more marriage bars branching
///   from its slot (per one canonical card per person and
///   current-intimacy placement — a person with concurrent un-ended
///   marriages shares one canonical anchor for all of them). For the
///   past-ended floating-bar fallback (no canonical host, e.g.
///   `examples/08`'s `m_alice_bob` after both spouses moved on), the
///   root `PersonCard` is a *ghost* — `slot.kind = Ghost { reason:
///   PastMarriage }` — rooted at the declared host. See
///   [ADR-0017](../../docs/adr/0017-render-shape-schema-and-versioning.md).
/// - [`ComponentKind::OrphanPerson`] — a single canonical card with no
///   anchor (declared-with-no-edges orphans, per absence not
///   placeholders, plus the current-intimacy-placement fallback
///   case of a joining spouse whose marriage ended and who has no birth
///   family declared).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
// Both variants box their payload to keep the enum compact — see
// `clippy::large-enum-variant`. The boxing is invisible on the wire
// (serde flattens `Box<T>` transparently).
pub enum ComponentKind {
    FamilyTree { root: Box<PersonCard> },
    OrphanPerson { card: Box<CardSlot> },
}

/// A marriage bar plus the children directly below it.
///
/// `MarriageBranch` is the recursive building block of a family tree:
/// each child may itself host marriages (the absorb rule applied
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
    /// marriage's bar nests visually at this slot (per current-intimacy
    /// placement: the bar's canonical location is the host's
    /// birth-family slot). For a
    /// canonical child the host's bar-slot is canonical too; for a host
    /// who has moved on (newer current intimacy elsewhere) the host's
    /// bar-slot becomes a ghost.
    pub hosted_marriages: Vec<MarriageBranch>,
}

/// A marriage bar in the layout.
///
/// The host face of every bar is implicit — it is the parent
/// [`PersonCard.slot`] in the tree (the bar branches from that
/// card per current-intimacy placement's "the bar's canonical location
/// is the host's birth-family slot"). Only the joining slot is
/// duplicated on the bar because a joining spouse may be canonical at
/// this bar or a ghost; the host slot's canonical/ghost state is the parent
/// `PersonCard.slot.kind`. See
/// [ADR-0017](../../docs/adr/0017-render-shape-schema-and-versioning.md).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarriageBar {
    /// Stable id from the input document (`marriage <id>`).
    pub marriage_id: String,
    /// Generation row this bar sits on — derived from the host's
    /// canonical generation under the canonical-family graph.
    pub generation: u32,
    /// Source-declaration id of the host (first-listed spouse, per the
    /// absorb rule). Kept for consumers cross-referencing by id; the
    /// host's `CardSlot` is the parent `PersonCard.slot`.
    pub host_id: String,
    /// Source-declaration id of the joining spouse (second-listed, per
    /// the absorb rule).
    pub joining_id: String,
    /// Slot for the joining spouse at the bar — canonical if the
    /// joining spouse's canonical card is this bar (per current-intimacy
    /// placement), ghost otherwise.
    pub joining_slot: CardSlot,
    pub start: ExportedDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<ExportedDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_reason: Option<String>,
    /// `true` iff the marriage carries an `end:` field. Per
    /// current-intimacy placement this is
    /// the canonical "ended" predicate (death is on the person, not
    /// the marriage); reified here so consumers don't have to re-
    /// derive it from `end.is_some()`.
    pub ended: bool,
    /// Joining spouse's birth-family sub-tree (the absorb rule's
    /// recursive nesting), when the joining spouse has a birth family
    /// that isn't already being rendered in this component's context.
    /// Per the absorb rule's termination, this is `None` when the
    /// recursion would re-enter the current rendering context (cousin /
    /// sibling marriage). The
    /// sub-tree is shaped exactly like a top-level
    /// [`ComponentKind::FamilyTree`]: a `PersonCard` rooted at the
    /// birth-family's outermost canonical host.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joining_nested_birth_family: Option<Box<PersonCard>>,
}

/// One canonical or ghost card slot. The single visual primitive
/// downstream surfaces render into the canonical UI pattern's
/// uniform card.
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

/// Whether a [`CardSlot`] is the person's canonical card (exactly
/// one per person) or a ghost anchoring a past structural fact.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SlotKind {
    /// The canonical card. Exactly one per declared person, per one
    /// canonical card per person.
    Canonical,
    /// A ghost card. Mute (ghosts are mute) — connects only to the
    /// marriage / adoption bar it anchors.
    Ghost { reason: GhostReason },
}

/// Why a ghost card was emitted. The discriminator a surface
/// renderer keys on for the ghost visual vocabulary — the dotted
/// border, the faded fill, and the surface-injected `↺` badge
/// (chrome; ADR-0016).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GhostReason {
    /// Past-marriage spouse-ghost: a spouse of a past marriage that
    /// produced children, or of a past marriage whose host moved on.
    /// Anchors the bar's children edges.
    PastMarriage,
    /// Past-adoption child-ghost: at a past adoptive family. The canonical
    /// card lives at the most-recent adoption; each prior adoption's
    /// bar gets one child-ghost connected by a dashed edge.
    PastAdoption,
    /// Past-bio child-ghost: at the bio family when the current-intimacy
    /// chain selects a different intimacy (any adoption demotes the
    /// bio family from current to past). The bio marriage's bar gets
    /// one child-ghost in its children row connected by a solid edge.
    PastBirth,
}

/// One parent-child edge.
///
/// Birth edges are solid; adoption edges dashed (edges encode link kind). Cross-component
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
/// parent-child link, per edges encode link kind.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeKind {
    Birth,
    Adoption,
}

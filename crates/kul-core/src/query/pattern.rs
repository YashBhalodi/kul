//! The **Query value** — the declarative, serializable request every
//! surface (Rust sugar, WASM, CLI) constructs and the single
//! [`evaluate`](super::evaluate) entry point consumes. There is exactly one
//! evaluation path (ADR-0025); the surfaces are thin constructors of this
//! value, never second engines.
//!
//! This slice implements the *lineal* subset. The enums are defined so
//! later variants (`allPersons` source, `collateral` patterns, `count`
//! projection, attribute filters) extend additively — a new variant, never
//! a reshape.

use serde::{Deserialize, Serialize};
#[cfg(feature = "tsify")]
use tsify::Tsify;

use super::descriptor::{Affinity, EdgeNature, LinealRole, RelationshipDescriptor, Sharing, Side};

/// An inclusive integer range; an absent `max` means unbounded. Used for a
/// lineal pattern's generation bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct IntRange {
    pub min: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max: Option<u32>,
}

impl IntRange {
    /// The range containing exactly `n` (`{min: n, max: n}`).
    #[must_use]
    pub fn exactly(n: u32) -> Self {
        IntRange {
            min: n,
            max: Some(n),
        }
    }

    /// `{min: 1, max}` — at least one generation, up to `max` (unbounded
    /// when `max` is `None`). The generation shape all four lineal sugars
    /// use.
    #[must_use]
    pub fn from_one(max: Option<u32>) -> Self {
        IntRange { min: 1, max }
    }

    /// Whether `value` falls within `[min, max]` (inclusive; unbounded
    /// above when `max` is `None`).
    #[must_use]
    pub fn contains(&self, value: u32) -> bool {
        value >= self.min && self.max.is_none_or(|max| value <= max)
    }
}

/// The classification a [`KinPattern`] selects for, internally tagged on
/// `kind`. `any` (an unclassified match) arrives with a later slice as a
/// further additive variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PatternClassification {
    /// Direct ancestors or descendants whose generation count falls in
    /// `generations`.
    Lineal {
        role: LinealRole,
        generations: IntRange,
    },
    /// Collateral relatives selected by raw hop counts: `up` hops to the
    /// common apex and `down` hops to the alter both falling in their ranges.
    /// `siblings` / `aunts_uncles` / `nieces_nephews` desugar here.
    Collateral { up: IntRange, down: IntRange },
    /// Collateral relatives selected by **cousin degree and removal**.
    /// Matches *both orientations* by construction: a `{degree: d, removed:
    /// r}` pattern matches every path whose `min(up,down) − 1` falls in
    /// `degree` and whose `|up − down|` falls in `removed`, so `up`/`down`
    /// may appear either way round. Corollary: `degree 0, removed 1` matches
    /// aunts/uncles **and** nieces/nephews. `cousins_of(degree, removed)`
    /// desugars here.
    CollateralByDegree { degree: IntRange, removed: IntRange },
    /// Any relationship shape whose total vertical displacement fits within
    /// `maxUp` ascent hops and `maxDown` descent hops — the unclassified
    /// match. Unlike the other variants it does not pin a specific
    /// classification; it exists so an affinity-scoped query (`in_laws_of`,
    /// `spouses_of`) can select every path within a bound regardless of
    /// whether it lands lineal, collateral, or self. Always paired with an
    /// `affinity` and/or `affinalHops` filter — on its own it would match
    /// every blood relative within the bound too.
    #[serde(rename_all = "camelCase")]
    Any { max_up: u32, max_down: u32 },
}

/// A declarative descriptor pattern: which relationships to a person count
/// as matches. The named sugar (`parents_of`, `ancestors_of`, …) each
/// desugar to one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct KinPattern {
    pub classification: PatternClassification,
    /// Optional filter on the path's edge nature; omitted (`None`) matches
    /// both blood and adoptive.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub edge_nature: Option<EdgeNature>,
    /// Optional filter on the sibling-junction [`Sharing`]; omitted (`None`)
    /// matches every sharing. Only ever narrows collateral results — a lineal
    /// path is always `notApplicable`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sharing: Option<Sharing>,
    /// Optional filter on the derived [`Side`]; omitted (`None`) matches every
    /// side. `Some(Side::Both)` selects couple-apex-rooted relations,
    /// `Some(Side::Maternal)` a single family branch, and so on.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub side: Option<Side>,
    /// Optional filter on the derived [`Affinity`]; omitted (`None`) matches
    /// every affinity. `Some(Affinity::Step)` / `Some(Affinity::InLaw)` select
    /// affinal relations (and, since a blood path is always `blood`, exclude
    /// blood ones). Also the switch that lets the engine spend affinal hops:
    /// with no `affinal_hops` bound set, a `step` / `inLaw` affinity filter
    /// raises the marriage-hop budget to the fixed ceiling of 2.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub affinity: Option<Affinity>,
    /// Optional filter on the number of marriage (`across`) hops on the path.
    /// Omitted (`None`) leaves the count unconstrained (0 when no affinity
    /// filter opens the budget, up to the ceiling of 2 otherwise). The engine
    /// caps traversal at 2 affinal hops regardless of this bound — the ceiling
    /// is fixed semantics, not a knob (ADR-0027).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub affinal_hops: Option<IntRange>,
}

/// Where a query draws its candidate persons from. This slice ships only
/// `kinOf`; `{ kind: "allPersons" }` arrives with the filtering slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum QuerySource {
    /// Every person whose relationship to `anchor` matches `pattern`.
    KinOf { anchor: String, pattern: KinPattern },
}

/// What the query produces. This slice ships only `members`; `count` (and
/// the `personIds` shape of the `allPersons` source) arrive later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum Projection {
    Members,
}

/// The single contract artifact: a declarative, serializable query. Every
/// surface builds this and hands it to [`evaluate`](super::evaluate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct Query {
    pub source: QuerySource,
    pub projection: Projection,
}

impl Query {
    /// A lineal-ancestor kin query: every ancestor of `anchor` whose
    /// generation depth falls in `generations`, optionally filtered by
    /// `edge_nature`. Projects `members`.
    #[must_use]
    pub fn kin_ancestors(
        anchor: impl Into<String>,
        generations: IntRange,
        edge_nature: Option<EdgeNature>,
    ) -> Self {
        Query::lineal(anchor, LinealRole::Ancestor, generations, edge_nature)
    }

    /// A lineal-descendant kin query, the descendant counterpart to
    /// [`Query::kin_ancestors`].
    #[must_use]
    pub fn kin_descendants(
        anchor: impl Into<String>,
        generations: IntRange,
        edge_nature: Option<EdgeNature>,
    ) -> Self {
        Query::lineal(anchor, LinealRole::Descendant, generations, edge_nature)
    }

    fn lineal(
        anchor: impl Into<String>,
        role: LinealRole,
        generations: IntRange,
        edge_nature: Option<EdgeNature>,
    ) -> Self {
        Query::kin(
            anchor,
            PatternClassification::Lineal { role, generations },
            edge_nature,
        )
    }

    /// A collateral kin query by raw hop counts: every relative reached by
    /// `up` ascent hops and `down` descent hops through a single apex, both
    /// falling in their ranges. `siblings_of` / `aunts_uncles_of` /
    /// `nieces_nephews_of` desugar through here. Projects `members`.
    #[must_use]
    pub fn kin_collateral(
        anchor: impl Into<String>,
        up: IntRange,
        down: IntRange,
        edge_nature: Option<EdgeNature>,
    ) -> Self {
        Query::kin(
            anchor,
            PatternClassification::Collateral { up, down },
            edge_nature,
        )
    }

    /// A collateral kin query by **cousin degree and removal**, matching both
    /// orientations by construction (see
    /// [`PatternClassification::CollateralByDegree`]). `cousins_of(degree,
    /// removed)` desugars through here. Projects `members`.
    #[must_use]
    pub fn kin_collateral_by_degree(
        anchor: impl Into<String>,
        degree: IntRange,
        removed: IntRange,
        edge_nature: Option<EdgeNature>,
    ) -> Self {
        Query::kin(
            anchor,
            PatternClassification::CollateralByDegree { degree, removed },
            edge_nature,
        )
    }

    /// Narrow a `kinOf` query to only members whose derived sharing equals
    /// `sharing`. A no-op on a non-`kinOf` source. Desugars to the pattern's
    /// `sharing` filter — no second evaluation path.
    #[must_use]
    pub fn with_sharing(mut self, sharing: Sharing) -> Self {
        let QuerySource::KinOf { pattern, .. } = &mut self.source;
        pattern.sharing = Some(sharing);
        self
    }

    /// Narrow a `kinOf` query to only members whose derived side equals
    /// `side`. Side counterpart to [`Query::with_sharing`].
    #[must_use]
    pub fn with_side(mut self, side: Side) -> Self {
        let QuerySource::KinOf { pattern, .. } = &mut self.source;
        pattern.side = Some(side);
        self
    }

    /// Narrow a `kinOf` query to only members whose derived affinity equals
    /// `affinity`. Also opens the affinal-hop budget: a `step` / `inLaw`
    /// filter lets the engine cross up to the ceiling of 2 marriages.
    #[must_use]
    pub fn with_affinity(mut self, affinity: Affinity) -> Self {
        let QuerySource::KinOf { pattern, .. } = &mut self.source;
        pattern.affinity = Some(affinity);
        self
    }

    /// Narrow a `kinOf` query to only members whose marriage-hop count falls
    /// in `hops`. Opens the affinal-hop budget to that range's upper bound
    /// (still capped at the fixed ceiling of 2).
    #[must_use]
    pub fn with_affinal_hops(mut self, hops: IntRange) -> Self {
        let QuerySource::KinOf { pattern, .. } = &mut self.source;
        pattern.affinal_hops = Some(hops);
        self
    }

    /// A spouse kin query: every spouse of `anchor` across every marriage,
    /// past or current (each `across` hop is tagged with the marriage's
    /// status). `spouses_of` desugars here. Zero vertical displacement plus
    /// exactly one marriage hop — a `self`-classification, `inLaw` member.
    #[must_use]
    pub fn kin_spouses(anchor: impl Into<String>) -> Self {
        Query::kin(
            anchor,
            PatternClassification::Any {
                max_up: 0,
                max_down: 0,
            },
            None,
        )
        .with_affinal_hops(IntRange::exactly(1))
    }

    /// An in-law kin query: every relation reached through at least one
    /// non-ancestor marriage hop, within 2 ascent and 2 descent hops.
    /// `in_laws_of` desugars here.
    #[must_use]
    pub fn kin_in_laws(anchor: impl Into<String>) -> Self {
        Query::kin(
            anchor,
            PatternClassification::Any {
                max_up: 2,
                max_down: 2,
            },
            None,
        )
        .with_affinity(Affinity::InLaw)
    }

    /// A step-parent kin query: the spouse of a parent via a marriage `anchor`
    /// has no birth/adoption link to. `step_parents_of` desugars here.
    #[must_use]
    pub fn kin_step_parents(anchor: impl Into<String>) -> Self {
        Query::lineal(anchor, LinealRole::Ancestor, IntRange::exactly(1), None)
            .with_affinity(Affinity::Step)
    }

    /// A step-child kin query: the child of a spouse `anchor` has no
    /// birth/adoption link to. `step_children_of` desugars here.
    #[must_use]
    pub fn kin_step_children(anchor: impl Into<String>) -> Self {
        Query::lineal(anchor, LinealRole::Descendant, IntRange::exactly(1), None)
            .with_affinity(Affinity::Step)
    }

    /// A step-sibling kin query: the child of a step-parent who shares no
    /// parent with `anchor`. `step_siblings_of` desugars here.
    #[must_use]
    pub fn kin_step_siblings(anchor: impl Into<String>) -> Self {
        Query::kin(
            anchor,
            PatternClassification::Collateral {
                up: IntRange::exactly(1),
                down: IntRange::exactly(1),
            },
            None,
        )
        .with_affinity(Affinity::Step)
    }

    /// Build a `members`-projecting `kinOf` query from a classification and an
    /// optional edge filter (no sharing / side filter — those are set on the
    /// returned pattern by callers that need them).
    fn kin(
        anchor: impl Into<String>,
        classification: PatternClassification,
        edge_nature: Option<EdgeNature>,
    ) -> Self {
        Query {
            source: QuerySource::KinOf {
                anchor: anchor.into(),
                pattern: KinPattern {
                    classification,
                    edge_nature,
                    sharing: None,
                    side: None,
                    affinity: None,
                    affinal_hops: None,
                },
            },
            projection: Projection::Members,
        }
    }
}

/// One member of a `members` result on the wire: the person id plus the
/// [`RelationshipDescriptor`] recording how it was reached. Carries **no
/// person payload** — consumers hydrate via the `person(id)` lookup. The
/// Rust-native evaluator returns a borrowed [`KinMember`](super::KinMember)
/// instead; this is its serialized projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub person_id: String,
    pub descriptor: RelationshipDescriptor,
}

/// The result of evaluating a [`Query`]. A tagged union so later
/// projections (`count`, the `allPersons` `personIds` shape) slot in without
/// reshaping. This slice produces only the `members` variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum QueryResult {
    /// The set of matching persons, each with its descriptor, in the pinned
    /// deterministic order (ADR-0026).
    Members { members: Vec<Member> },
}

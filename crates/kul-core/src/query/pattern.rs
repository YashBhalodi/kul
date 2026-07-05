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

use super::descriptor::{EdgeNature, LinealRole, RelationshipDescriptor};

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

/// The classification a [`KinPattern`] selects for. This slice ships only
/// the lineal arm; `collateral`, `collateralByDegree`, and `any` arrive in
/// later slices as additional internally-tagged variants.
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
    /// both blood and adoptive. Affinity / sharing / side filters arrive in
    /// later slices.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub edge_nature: Option<EdgeNature>,
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
        Query {
            source: QuerySource::KinOf {
                anchor: anchor.into(),
                pattern: KinPattern {
                    classification: PatternClassification::Lineal { role, generations },
                    edge_nature,
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

//! The **relationship descriptor** — the terminology-neutral, maximally
//! discriminating record of *how* one person (the alter) relates to another
//! (the ego), plus the lossless [`PathHop`] backbone the derivation walked.
//!
//! The descriptor is a *contract type*: its shape and serialization are
//! pinned (PRD 0005, ADR-0026) and committed as TypeScript. This slice only
//! populates the lineal subset of its dimensions; the rest serialize as
//! their `notApplicable` values until later slices reach the path shapes
//! that make them meaningful. **Defining the full type now** is deliberate —
//! a future culture pack keys terminology off these fields, so every
//! distinction any culture could need must already exist for that layer to
//! be pure data.
//!
//! Serialization rules (pinned):
//! - camelCase field names.
//! - Unions are *internally tagged* (`kind` / `step`) so TypeScript
//!   consumers get a discriminated union to `switch` on.
//! - `unknown` and `notApplicable` are **explicit enum values, never `null`
//!   or absent**. `unknown` = the data is insufficient to decide;
//!   `notApplicable` = the dimension does not apply to this path shape. The
//!   two are never conflated.
//! - `cousinDegree` / `removed` are materialized numbers (the formulas are
//!   off-by-one traps a consumer would fumble).
//! - Path hops carry ids only, never entity payloads (consumers hydrate via
//!   the `person(id)` / `marriage(id)` lookups). `endReason` is the one
//!   presence-based optional field.

use serde::{Deserialize, Serialize};
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::ast::{self, PersonStmt};
use crate::date::before_strict;
use crate::semantic::ParentLinkKind;

/// Endpoint / linking-relative gender. Wire form: `"male" | "female" |
/// "other"`. Mirrors [`ast::Gender`] but lives here so the descriptor's
/// serialized surface is self-contained (the AST enum is not part of any
/// wire contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    Other,
}

impl From<ast::Gender> for Gender {
    fn from(g: ast::Gender) -> Self {
        match g {
            ast::Gender::Male => Gender::Male,
            ast::Gender::Female => Gender::Female,
            ast::Gender::Other => Gender::Other,
        }
    }
}

/// A person's gender for descriptor purposes. A checked project always
/// records `gender:` (R03), but [`derive`](RelationshipDescriptor::derive)
/// and the traversal are total functions over any `ResolvedDocument`, so a
/// missing gender resolves to [`Gender::Other`] rather than panicking.
#[must_use]
pub(crate) fn gender_of(person: &PersonStmt) -> Gender {
    person
        .gender()
        .map(|g| Gender::from(g.value))
        .unwrap_or(Gender::Other)
}

/// How the alter is classified relative to the ego. Internally tagged on
/// `kind`. This slice emits only [`Classification::Lineal`]; `self` and
/// `collateral` derive mechanically from the same hop counts (see
/// [`RelationshipDescriptor::derive`]) so later slices need no rework.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Classification {
    /// The alter *is* the ego. Never produced by a kin-set query (the
    /// anchor excludes itself), but part of the pinned type.
    #[serde(rename = "self")]
    SelfRel,
    /// A direct ancestor or descendant, `generations` hops away.
    Lineal { role: LinealRole, generations: u32 },
    /// A collateral relative: `up` hops to the common apex, `down` hops to
    /// the alter. `cousinDegree` / `removed` are materialized so consumers
    /// never re-derive the off-by-one formulas. Not produced this slice.
    #[serde(rename_all = "camelCase")]
    Collateral {
        up: u32,
        down: u32,
        cousin_degree: u32,
        removed: u32,
    },
}

/// Direction of a [`Classification::Lineal`] relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum LinealRole {
    Ancestor,
    Descendant,
}

/// Whether the parent-child edges on the path are all blood or include at
/// least one adoption. `adoptive` iff *any* hop is an adoption edge; the
/// per-hop truth stays lossless in the [`PathHop`] backbone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "lowercase")]
pub enum EdgeNature {
    Blood,
    Adoptive,
}

/// Whether the relationship runs through marriage hops. Strictly about
/// `across` hops: none ⇒ `blood`. This slice produces only blood segments
/// (no `across` hops exist yet), so `affinity` is always `blood`; `step`
/// and `inLaw` arrive with the affinal-hop slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum Affinity {
    Blood,
    Step,
    InLaw,
}

/// Sibling-junction parent-set sharing. An apex-junction comparison, so
/// `notApplicable` for every lineal / self path (there is no sibling
/// junction). This slice produces only lineal paths ⇒ always
/// `notApplicable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum Sharing {
    Full,
    Half,
    NotApplicable,
}

/// Which side of the family the relationship routes through. Derived from
/// the path's *initial ascent*, never guessed. `both` (couple-apex
/// collateral paths) arrives with the next slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Maternal,
    Paternal,
    Other,
    Both,
    NotApplicable,
}

/// A birth-order comparison under the strict-interval rule. `elder` /
/// `younger` only when *every* interpretation of one date strictly precedes
/// the other; `unknown` when dates are missing or intervals overlap;
/// `notApplicable` is reserved for `self`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum Seniority {
    Elder,
    Younger,
    Unknown,
    NotApplicable,
}

/// The edge tag on a vertical [`PathHop`]. Wire form: `"bio" | "adoptive"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "lowercase")]
pub enum HopEdge {
    Bio,
    Adoptive,
}

impl From<ParentLinkKind> for HopEdge {
    fn from(kind: ParentLinkKind) -> Self {
        match kind {
            ParentLinkKind::Bio => HopEdge::Bio,
            ParentLinkKind::Adoption => HopEdge::Adoptive,
        }
    }
}

/// A marriage hop's status. Wire form: `"ongoing" | "ended"`. Not produced
/// this slice (no `across` hops yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "lowercase")]
pub enum MarriageStatus {
    Ongoing,
    Ended,
}

/// One hop of the lossless path backbone. Internally tagged on `step`.
/// Vertical hops (`up` / `down`) carry the person landed on, that person's
/// gender, and the edge kind. The `across` variant (a marriage hop) is part
/// of the pinned type but not produced this slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum PathHop {
    /// Ascend one parent edge to `to`.
    Up {
        to: String,
        gender: Gender,
        edge: HopEdge,
    },
    /// Descend one child edge to `to`.
    Down {
        to: String,
        gender: Gender,
        edge: HopEdge,
    },
    /// Cross a marriage to `to`. Carries the marriage id, its status, and
    /// the one presence-based optional field, `endReason`.
    #[serde(rename_all = "camelCase")]
    Across {
        to: String,
        gender: Gender,
        marriage: String,
        status: MarriageStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_reason: Option<String>,
    },
}

impl PathHop {
    /// The person id this hop lands on.
    #[must_use]
    pub fn to(&self) -> &str {
        match self {
            PathHop::Up { to, .. } | PathHop::Down { to, .. } | PathHop::Across { to, .. } => to,
        }
    }
}

/// The terminology-neutral record of how the alter relates to the ego, plus
/// the lossless [`PathHop`] backbone. One descriptor per distinct
/// relationship path — descriptor identity *is* path identity, and the
/// engine never collapses same-classification descriptors (ADR-0026).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub struct RelationshipDescriptor {
    pub ego_id: String,
    pub alter_id: String,
    pub ego_gender: Gender,
    pub alter_gender: Gender,
    pub classification: Classification,
    pub edge_nature: EdgeNature,
    pub affinity: Affinity,
    pub sharing: Sharing,
    pub side: Side,
    pub seniority: Seniority,
    pub apex_seniority: Seniority,
    pub path: Vec<PathHop>,
}

impl RelationshipDescriptor {
    /// Derive the descriptor for reaching `alter` from `ego` along `path`.
    ///
    /// Every derivation rule below is pinned by PRD 0005 / issue #256. This
    /// slice's paths are exactly one blood segment — `up+` (ancestors) or
    /// `down+` (descendants) — so the collateral / affinal dimensions are
    /// derived at their `notApplicable` defaults, but the mechanical rules
    /// (classification, edge nature) are written in full so later slices
    /// extend rather than rewrite them.
    #[must_use]
    pub fn derive(ego: &PersonStmt, alter: &PersonStmt, path: Vec<PathHop>) -> Self {
        let classification = derive_classification(&path);
        let edge_nature = derive_edge_nature(&path);
        let affinity = derive_affinity(&path);
        let side = derive_side(&path);
        let seniority = derive_seniority(alter, ego);
        RelationshipDescriptor {
            ego_id: ego.id.name.clone(),
            alter_id: alter.id.name.clone(),
            ego_gender: gender_of(ego),
            alter_gender: gender_of(alter),
            classification,
            edge_nature,
            affinity,
            // No sibling junction on a lineal path ⇒ sharing is n/a.
            sharing: Sharing::NotApplicable,
            side,
            seniority,
            // Reserved for a sibling junction; no junction on a lineal path.
            apex_seniority: Seniority::NotApplicable,
            path,
        }
    }
}

/// `classification` from vertical hop counts. With `u` = number of `up`
/// hops and `d` = number of `down` hops anywhere on the path:
/// `u>0 ∧ d=0` → lineal ancestor (`generations = u`); `d>0 ∧ u=0` → lineal
/// descendant (`generations = d`); `u=d=0` → self; `u>0 ∧ d>0` → collateral
/// (`cousinDegree = min(u,d) − 1`, `removed = |u − d|`). Only the two lineal
/// arms are reachable this slice; the rest are derived mechanically so a
/// later slice inherits them for free.
fn derive_classification(path: &[PathHop]) -> Classification {
    let up = path
        .iter()
        .filter(|h| matches!(h, PathHop::Up { .. }))
        .count() as u32;
    let down = path
        .iter()
        .filter(|h| matches!(h, PathHop::Down { .. }))
        .count() as u32;
    match (up, down) {
        (0, 0) => Classification::SelfRel,
        (u, 0) => Classification::Lineal {
            role: LinealRole::Ancestor,
            generations: u,
        },
        (0, d) => Classification::Lineal {
            role: LinealRole::Descendant,
            generations: d,
        },
        (u, d) => Classification::Collateral {
            up: u,
            down: d,
            cousin_degree: u.min(d).saturating_sub(1),
            removed: u.abs_diff(d),
        },
    }
}

/// `edgeNature`: `adoptive` iff any vertical hop is an adoption edge, else
/// `blood`. Marriage hops carry no edge nature and are ignored here.
fn derive_edge_nature(path: &[PathHop]) -> EdgeNature {
    let any_adoptive = path.iter().any(|h| {
        matches!(
            h,
            PathHop::Up {
                edge: HopEdge::Adoptive,
                ..
            } | PathHop::Down {
                edge: HopEdge::Adoptive,
                ..
            }
        )
    });
    if any_adoptive {
        EdgeNature::Adoptive
    } else {
        EdgeNature::Blood
    }
}

/// `affinity`: `blood` when the path has no marriage (`across`) hop. This
/// slice produces only blood segments, so this is always `blood`; the
/// `step` / `inLaw` disambiguation (by marriage-hop position) lands with the
/// affinal-hop slice.
fn derive_affinity(path: &[PathHop]) -> Affinity {
    if path.iter().any(|h| matches!(h, PathHop::Across { .. })) {
        // Unreachable this slice; the position-based step/in-law rule is a
        // later slice's concern. Kept honest rather than guessed.
        Affinity::InLaw
    } else {
        Affinity::Blood
    }
}

/// `side`, derived from the path's *initial ascent* (the maximal run of
/// `up` hops at the start), never guessed:
/// - no initial ascent (the path never starts by ascending, i.e. every
///   descendant path) → `notApplicable`;
/// - the entire path is exactly one `up` hop (a direct parent) →
///   `notApplicable` (your mother is not your "maternal side");
/// - otherwise the gender of the first parent-person on the initial ascent:
///   female → `maternal`, male → `paternal`, `other` → `other`.
///
/// `both` (a couple apex reached without passing through an individual
/// parent) arrives with couple-apex collateral paths.
fn derive_side(path: &[PathHop]) -> Side {
    let first = match path.first() {
        Some(PathHop::Up { gender, .. }) => *gender,
        // Descendants and any non-ascending start: side does not apply.
        _ => return Side::NotApplicable,
    };
    // A direct parent (the whole path is one up hop) has no "side".
    if path.len() == 1 {
        return Side::NotApplicable;
    }
    match first {
        Gender::Female => Side::Maternal,
        Gender::Male => Side::Paternal,
        Gender::Other => Side::Other,
    }
}

/// `seniority` (endpoint): the alter's birth order versus the ego under the
/// strict-interval rule, reusing the toolchain's single
/// [`before_strict`](crate::date::before_strict) comparison. `elder` iff
/// every interpretation of the alter's birth date is strictly before every
/// interpretation of the ego's; `younger` for the reverse; otherwise
/// `unknown` (a missing date, overlapping partial/circa intervals, or
/// same-day twins are all `unknown`). `notApplicable` is reserved for self.
fn derive_seniority(alter: &PersonStmt, ego: &PersonStmt) -> Seniority {
    let (Some(alter_born), Some(ego_born)) = (alter.born(), ego.born()) else {
        return Seniority::Unknown;
    };
    if before_strict(alter_born, ego_born) {
        Seniority::Elder
    } else if before_strict(ego_born, alter_born) {
        Seniority::Younger
    } else {
        Seniority::Unknown
    }
}

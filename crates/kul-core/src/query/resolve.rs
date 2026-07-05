//! **Relationship resolution** — the two-anchor question (issue #259, PRD
//! 0005): given persons `x` and `y`, return *all* the ways they are related,
//! each as a [`RelationshipDescriptor`], with **honest emptiness**.
//!
//! This module owns the *contract types* of resolution; the enumeration
//! itself lives next to the kin-set traversal in
//! [`engine::resolve`](super::engine::resolve) so the two questions share one
//! traversal engine and one descriptor derivation — resolution never forks the
//! kin-set logic (ADR-0028).
//!
//! ## The semantics-vs-budget line (ADR-0028)
//!
//! Resolution has exactly one knob, [`ResolveConfig::max_apex_generations`]
//! (default [`DEFAULT_MAX_APEX_GENERATIONS`]) — a *search budget* bounding each
//! blood segment's ascent and descent. It is a caller preference: two apps may
//! legitimately look differently far. The **2-affinal-hop ceiling is not a
//! knob** — it is fixed engine *semantics* (no culture lexicalizes three
//! affinal hops, ADR-0027), so two apps may look differently far but can never
//! disagree about *what relationships exist*.
//!
//! ## Honest emptiness
//!
//! An empty result is an answer, but a *why* comes with it (present iff the
//! list is empty):
//! - [`EmptyReason::Disconnected`] — `x` and `y` lie in different connected
//!   components of the full relation graph; raising the cap can never help.
//! - [`EmptyReason::NoneWithinBounds`] — same component, but nothing is
//!   derivable under the semantics and the current budget; a bigger cap might
//!   reveal something.
//!
//! Collapsing the two into a bare empty list would invite apps to render "not
//! related" when the truth is "not related as far as we looked".

use serde::Serialize;
#[cfg(feature = "tsify")]
use tsify::Tsify;

use super::descriptor::RelationshipDescriptor;

/// The default generation budget: bound each blood segment to five ascent and
/// five descent hops. Five reaches through fourth cousins — a strict superset
/// of every lexicalized kinship term in any culture (they run out by third
/// cousins) — while cutting off the remote-ancestor haystack (PRD 0005).
pub const DEFAULT_MAX_APEX_GENERATIONS: u32 = 5;

fn default_max_apex_generations() -> u32 {
    DEFAULT_MAX_APEX_GENERATIONS
}

/// The one caller knob for [`resolve`](super::resolve): the *search budget*.
///
/// `max_apex_generations` bounds **each blood segment's** ascent and descent
/// (the up-count and down-count of every `up* down*` run) — a nearest-common-
/// ancestor bound, not a total-path-length bound. It is a budget, never
/// semantics: the fixed 2-affinal-hop ceiling is not configurable here.
///
/// Deserializes with the field defaulted, so an over-the-wire `{}` (or an
/// omitted config) yields the default budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(from_wasm_abi, into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct ResolveConfig {
    /// Per-blood-segment ascent/descent cap. Defaults to
    /// [`DEFAULT_MAX_APEX_GENERATIONS`].
    #[serde(default = "default_max_apex_generations")]
    pub max_apex_generations: u32,
}

impl Default for ResolveConfig {
    fn default() -> Self {
        ResolveConfig {
            max_apex_generations: DEFAULT_MAX_APEX_GENERATIONS,
        }
    }
}

/// Why a [`ResolveResult`] is empty. Present **iff** the relationship list is
/// empty (PRD 0005) — the distinction is the product: it lets an app say "not
/// related" only when it truly means it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum EmptyReason {
    /// `x` and `y` are in different connected components of the full relation
    /// graph (undirected reachability over every parent-child edge of both
    /// kinds plus every spouse edge). No budget can connect them.
    Disconnected,
    /// `x` and `y` are in the same component, but nothing is derivable under
    /// the semantics (the affinal ceiling, step subsumption) and the current
    /// generation budget. A larger `max_apex_generations` might reveal a tie.
    NoneWithinBounds,
}

/// The result of [`resolve`](super::resolve): every way `x` and `y` are
/// related, plus — **only when the list is empty** — the reason.
///
/// One descriptor per distinct relationship path (path identity, exactly as in
/// the kin-set queries; ADR-0026), in the pinned deterministic order
/// ([`resolve`](super::resolve) sorts by path hop count then serialized
/// backbone). Never a bare set: `empty_reason` carries the honest "why" when
/// there is nothing to show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
#[serde(rename_all = "camelCase")]
pub struct ResolveResult {
    /// Every distinct relationship path from `x` to `y`, deterministically
    /// ordered. Empty when the two are unrelated (see `empty_reason`).
    pub relationships: Vec<RelationshipDescriptor>,
    /// Present **iff** `relationships` is empty: why no tie was found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_reason: Option<EmptyReason>,
}

//! Named sugar — documented conveniences, each defined **as its Query-value
//! expansion**. Every one desugars to a [`Query`] and calls
//! [`evaluate`](super::evaluate); there is no second evaluation path
//! (ADR-0025). Raw up/down/across step composition stays *internal* —
//! exposing it would recreate the "compute the derivation yourself" trap
//! (self-exclusion, cycle guarding, and subsumption are engine-owned and
//! must stay unreachable by consumers).

use crate::semantic::ResolvedDocument;

use super::engine::{KinMember, QueryEvalError, evaluate};
use super::pattern::{IntRange, Query};

/// `parents_of(x)` ≡ `kinOf(x, lineal ancestor, generations {1,1})`.
///
/// Returns **all** parents — 0, 1, 2, or 4+ (birth + multiple adoptions),
/// each edge-tagged in its descriptor. A person with no recorded parents
/// yields an empty set (never an error).
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn parents_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(
        resolved,
        &Query::kin_ancestors(anchor, IntRange::exactly(1), None),
    )
}

/// `children_of(x)` ≡ `kinOf(x, lineal descendant, generations {1,1})`.
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn children_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(
        resolved,
        &Query::kin_descendants(anchor, IntRange::exactly(1), None),
    )
}

/// `ancestors_of(x, depth?)` ≡ `kinOf(x, lineal ancestor, generations {1,
/// depth})`. `depth = None` is unbounded (every ancestor, to any height).
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn ancestors_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
    depth: Option<u32>,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(
        resolved,
        &Query::kin_ancestors(anchor, IntRange::from_one(depth), None),
    )
}

/// `descendants_of(x, depth?)` ≡ `kinOf(x, lineal descendant, generations
/// {1, depth})`. `depth = None` is unbounded.
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn descendants_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
    depth: Option<u32>,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(
        resolved,
        &Query::kin_descendants(anchor, IntRange::from_one(depth), None),
    )
}

/// `siblings_of(x)` ≡ `kinOf(x, collateral, up {1,1}, down {1,1})`.
///
/// The apex is the parent (couple or single) shared with the sibling, so each
/// member carries its `sharing` (full / half), `apexSeniority`, and — for a
/// couple-apex full sibling — `side: both`.
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn siblings_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(
        resolved,
        &Query::kin_collateral(anchor, IntRange::exactly(1), IntRange::exactly(1), None),
    )
}

/// `aunts_uncles_of(x)` ≡ `kinOf(x, collateral, up {2,2}, down {1,1})` — a
/// parent's siblings. `side` is ego's linking-parent gender (maternal vs
/// paternal), and `apexSeniority` is the aunt/uncle's birth order versus that
/// parent — the two distinctions *mama*/*chacha* and *chacha*/*tau* need.
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn aunts_uncles_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(
        resolved,
        &Query::kin_collateral(anchor, IntRange::exactly(2), IntRange::exactly(1), None),
    )
}

/// `nieces_nephews_of(x)` ≡ `kinOf(x, collateral, up {1,1}, down {2,2})` — a
/// sibling's children, the inverse orientation of [`aunts_uncles_of`].
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn nieces_nephews_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(
        resolved,
        &Query::kin_collateral(anchor, IntRange::exactly(1), IntRange::exactly(2), None),
    )
}

/// `cousins_of(x, degree, removed = 0)` ≡ `kinOf(x, collateralByDegree,
/// degree {degree,degree}, removed {removed,removed})`.
///
/// Degree 1 removed 0 is first cousins; degree 2 removed 1 is "second cousins
/// once removed" — expressible by construction, no dedicated API. Because
/// `collateralByDegree` matches both orientations, `removed ≥ 1` returns the
/// relation both up-heavy and down-heavy (e.g. degree 0 removed 1 is aunts/
/// uncles *and* nieces/nephews).
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn cousins_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
    degree: u32,
    removed: u32,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(
        resolved,
        &Query::kin_collateral_by_degree(
            anchor,
            IntRange::exactly(degree),
            IntRange::exactly(removed),
            None,
        ),
    )
}

/// `spouses_of(x)` ≡ `kinOf(x, classification any {0,0}, affinalHops {1,1})` —
/// every spouse across every marriage, past or current. Each member is a
/// `self`-classification, `inLaw` relation whose `across` hop is tagged with
/// the marriage's status (and end reason when ended).
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn spouses_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(resolved, &Query::kin_spouses(anchor))
}

/// `in_laws_of(x)` ≡ `kinOf(x, classification any {2,2}, affinity inLaw)` —
/// every relation reached through a non-ancestor marriage hop within two
/// ascent and two descent hops (spouse's kin, kin's spouse, the *samdhi*,
/// co-spouses, and so on).
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn in_laws_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(resolved, &Query::kin_in_laws(anchor))
}

/// `step_parents_of(x)` ≡ `kinOf(x, lineal ancestor {1,1}, affinity step)` —
/// the spouse of a parent via a marriage `x` has no birth/adoption link to. A
/// person who is *also* an actual parent is emitted only as that real parent
/// (step subsumption), never doubled as a step-parent.
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn step_parents_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(resolved, &Query::kin_step_parents(anchor))
}

/// `step_siblings_of(x)` ≡ `kinOf(x, collateral {1,1}/{1,1}, affinity step)` —
/// a step-parent's child who shares no parent with `x`. Anyone sharing a parent
/// is a full/half sibling and is suppressed here (step subsumption).
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn step_siblings_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(resolved, &Query::kin_step_siblings(anchor))
}

/// `step_children_of(x)` ≡ `kinOf(x, lineal descendant {1,1}, affinity step)`
/// — the child of a spouse `x` has no birth/adoption link to. The inverse of
/// [`step_parents_of`], with the mirror subsumption.
///
/// # Errors
///
/// [`QueryEvalError::UnknownPerson`] when `anchor` names no person.
pub fn step_children_of<'a>(
    resolved: &'a ResolvedDocument,
    anchor: &str,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    evaluate(resolved, &Query::kin_step_children(anchor))
}

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

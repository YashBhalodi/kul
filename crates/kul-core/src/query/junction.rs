//! The **sibling junction** (apex) — the load-bearing concept of the
//! collateral slice (issue #257).
//!
//! In a single-blood-segment `up* down*` path, the **apex** is the person
//! where ascent turns to descent. The junction is the triple *(apex,
//! egoChild, alterChild)*: *egoChild* is the person on the path immediately
//! before the apex (the one the last `up` hop ascended from — for siblings
//! this is ego itself); *alterChild* is the person immediately after the apex
//! (the one the first `down` hop descended to — for siblings this is alter
//! itself). egoChild and alterChild are the two **branch siblings**; the
//! no-backtracking rule (ADR-0025) guarantees they are distinct.
//!
//! Two dimensions are derived here, both defined only at a sibling junction:
//! - [`Sharing`] — a **per-edge-kind** parent-set comparison of the branch
//!   siblings (bio set vs bio set; adoptive set vs adoptive set).
//! - the **couple apex** — whether the branch siblings share the *same two
//!   parents* (identical bio-parent sets of size 2, or both adopted by the
//!   same couple). A couple apex is *one relationship fact* even though the
//!   backbone could route through either co-parent, so the engine
//!   canonicalizes it through the smaller-id co-parent and emits one
//!   descriptor. It also drives `side = both`.
//!
//! This module owns the junction computation shared by the descriptor
//! (which reads `sharing` / the couple flag) and the traversal engine (which
//! canonicalizes the couple-apex backbone before de-duplicating).

use std::collections::BTreeSet;

use crate::ast::PersonStmt;
use crate::semantic::{ParentLinkKind, ResolvedDocument};

use super::descriptor::{PathHop, Sharing};

/// The sibling junction of a collateral `up* down*` path: the two branch
/// siblings, the canonical apex person, whether it is a couple apex, and the
/// [`Sharing`] the branch siblings' parent sets imply. `None` for lineal /
/// self paths (no ascent-then-descent, so no junction).
pub(crate) struct Junction<'a> {
    /// Number of `up` hops (the apex sits at node index `up` on the path,
    /// counting ego as node 0). Also the length of the initial ascent.
    pub up: u32,
    /// The person the last `up` hop ascended from (ego for siblings).
    pub ego_child: &'a PersonStmt,
    /// The person the first `down` hop descended to (alter for siblings).
    pub alter_child: &'a PersonStmt,
    /// The apex to canonicalize the backbone through: the smaller-id
    /// co-parent at a couple apex, else the (single) apex person.
    pub canonical_apex: &'a PersonStmt,
    /// Whether the branch siblings share the same two parents (bio set of
    /// size 2 equal, or adoptive set of size 2 equal).
    pub is_couple_apex: bool,
    /// The per-edge-kind parent-set sharing of the branch siblings.
    pub sharing: Sharing,
}

/// Compute the sibling junction of `path` from `ego`. Returns `None` when the
/// path is lineal or self (no `up` hop followed by a `down` hop) — those
/// shapes have no sibling junction, so `sharing` / `apexSeniority` are
/// `notApplicable` and `side` is never `both`.
///
/// The path is a single blood segment `up^u down^d` (the only shape this
/// slice's traversal produces), so the initial ascent length `u` is the count
/// of leading `up` hops and the apex is the node the ascent ends on.
pub(crate) fn junction_of<'a>(
    resolved: &'a ResolvedDocument,
    ego: &'a PersonStmt,
    path: &[PathHop],
) -> Option<Junction<'a>> {
    let up = path
        .iter()
        .take_while(|h| matches!(h, PathHop::Up { .. }))
        .count();
    let down = path.len() - up;
    // A sibling junction needs at least one ascent hop and one descent hop.
    if up == 0 || down == 0 {
        return None;
    }

    // Node sequence: node[0] = ego, node[i+1] = path[i].to. The apex is
    // node[up]; egoChild = node[up-1]; alterChild = node[up+1].
    let ego_child_id = if up == 1 {
        ego.id.name.as_str()
    } else {
        path[up - 2].to()
    };
    let alter_child_id = path[up].to();

    let ego_child = resolved.person(ego_child_id)?;
    let alter_child = resolved.person(alter_child_id)?;
    let apex = resolved.person(path[up - 1].to())?;

    let ego_bio = parent_ids(resolved, ego_child, ParentLinkKind::Bio);
    let ego_adopt = parent_ids(resolved, ego_child, ParentLinkKind::Adoption);
    let alter_bio = parent_ids(resolved, alter_child, ParentLinkKind::Bio);
    let alter_adopt = parent_ids(resolved, alter_child, ParentLinkKind::Adoption);

    // Couple apex: the branch siblings share the *same two* parents of one
    // kind. Divorce-and-remarry of the same couple, or two births under
    // distinct marriages of the same pair, both land here — it is parent-*set*
    // equality, never a shared marriage record.
    let couple: Option<&BTreeSet<String>> = if ego_bio.len() == 2 && ego_bio == alter_bio {
        Some(&ego_bio)
    } else if ego_adopt.len() == 2 && ego_adopt == alter_adopt {
        Some(&ego_adopt)
    } else {
        None
    };

    let canonical_apex = couple
        .and_then(|set| set.iter().next())
        .and_then(|id| resolved.person(id))
        .unwrap_or(apex);

    let sharing = derive_sharing(&ego_bio, &ego_adopt, &alter_bio, &alter_adopt);

    Some(Junction {
        up: up as u32,
        ego_child,
        alter_child,
        canonical_apex,
        is_couple_apex: couple.is_some(),
        sharing,
    })
}

/// The ids of `person`'s parents of one `kind`, as an ordered set (so
/// equality is set-equality and the smallest id is `.next()`).
fn parent_ids(
    resolved: &ResolvedDocument,
    person: &PersonStmt,
    kind: ParentLinkKind,
) -> BTreeSet<String> {
    resolved
        .parents_of(person)
        .into_iter()
        .filter(|link| link.kind == kind)
        .map(|link| link.parent.id.name.clone())
        .collect()
}

/// `sharing` at a sibling junction, comparing the branch siblings' parent
/// sets **per edge kind** (bio vs bio; adoptive vs adoptive):
/// - `full` iff the bio sets are equal and non-empty, OR the adoptive sets
///   are equal and non-empty (adoptive-full). Set equality — not a shared
///   marriage — so full siblings stay `full` across a divorce-and-remarry of
///   the same couple, and a bio child and an adoptee of the same couple do
///   NOT read as full (no same-kind equality).
/// - `half` iff they share at least one parent of any kind but no same-kind
///   set equality holds (polygamy and remarriage collapse identically here).
///
/// A collateral junction's branch siblings always share the apex, so this is
/// never reached with an empty intersection; the final `Half` is the total
/// fallback rather than a distinct case.
fn derive_sharing(
    ego_bio: &BTreeSet<String>,
    ego_adopt: &BTreeSet<String>,
    alter_bio: &BTreeSet<String>,
    alter_adopt: &BTreeSet<String>,
) -> Sharing {
    let bio_full = !ego_bio.is_empty() && ego_bio == alter_bio;
    let adopt_full = !ego_adopt.is_empty() && ego_adopt == alter_adopt;
    if bio_full || adopt_full {
        Sharing::Full
    } else {
        Sharing::Half
    }
}

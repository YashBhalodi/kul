//! The traversal engine and the single [`evaluate`] entry point.
//!
//! `evaluate` is the *one* evaluation path (ADR-0025): every surface — Rust
//! sugar, WASM, CLI — builds a [`Query`] value and hands it here. Paths are
//! 1–3 blood segments (each an `up* down*` run) joined by at most two marriage
//! (`across`) hops — the full grammar: lineal, collateral, and affinal
//! (spouse, step, in-law) shapes.
//!
//! **Traversal invariants** (these ARE the product, PRD 0005 / ADR-0027):
//! - The engine builds its own in-memory adjacency per invocation — a parent /
//!   child index (inverse of the resolved parent links) plus a co-spouse index
//!   over the marriages — and never caches across queries.
//! - **The affinal ceiling is fixed at two `across` hops.** No culture
//!   lexicalizes three affinal hops, so this is semantics, not a knob — never
//!   configurable (ADR-0027).
//! - **Cycle guarding is unconditional.** Bio-parenthood is a DAG (R13), but
//!   adoption edges can reintroduce cycles (adoption-into-relatives is a
//!   real corpus case), so the simple-path rule (no person appears twice on
//!   a path) is the guard, and traversal terminates on every input.
//! - **Path identity, no collapsing:** one member per distinct path. A
//!   person reachable as both a bio and an adoptive ancestor yields two
//!   members with distinct backbones. The anchor is never a member —
//!   self-exclusion is engine-owned.

use std::collections::{HashMap, HashSet};

use crate::ast::{EndReason, PersonStmt};
use crate::export::ExportedDiagnostic;
use crate::semantic::{ParentLinkKind, ResolvedDocument};

use super::descriptor::{
    Affinity, Classification, HopEdge, LinealRole, MarriageStatus, PathHop, RelationshipDescriptor,
    gender_of,
};
use super::junction::junction_of;
use super::pattern::{IntRange, KinPattern, PatternClassification, Query, QuerySource};

/// The engine's fixed ceiling on marriage (`across`) hops per path. No culture
/// lexicalizes three affinal hops, so this is fixed semantics, not a knob
/// (issue #258, ADR-0027) — never exposed as configuration.
const AFFINAL_CEILING: u32 = 2;

/// A kin-set member in the Rust-native shape: a **borrowed** person
/// reference plus the **owned** [`RelationshipDescriptor`] recording how it
/// was reached (matching the existing `parents_of` idiom). Native consumers
/// get full field access immediately; the serialized
/// [`Member`](super::Member) shape (id + descriptor) is its wire projection.
#[derive(Debug, Clone)]
pub struct KinMember<'a> {
    pub person: &'a PersonStmt,
    pub descriptor: RelationshipDescriptor,
}

impl KinMember<'_> {
    /// Project to the serialized [`Member`](super::Member) shape — person id
    /// plus descriptor, no person payload.
    #[must_use]
    pub fn to_member(&self) -> super::Member {
        super::Member {
            person_id: self.person.id.name.clone(),
            descriptor: self.descriptor.clone(),
        }
    }
}

/// A caller error from [`evaluate`]. Distinct from a project that fails its
/// checks: this is a bug in the *query*, not the data. An anchor id that
/// names no person (an unknown id, or an id that names a marriage where a
/// person is required) is this typed error — **never an empty set**. An
/// empty set always means "no kin matched".
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QueryEvalError {
    /// The anchor id names no person.
    #[error("no person with id `{id}`")]
    UnknownPerson { id: String },
}

impl QueryEvalError {
    /// Project to the [`ExportedDiagnostic`] the [`QueryEnvelope`] error arm
    /// carries on the WASM and CLI-`json` surfaces. The synthesized
    /// diagnostic has no source span (the bad id came from the query, not
    /// the source), so `primary` is absent.
    ///
    /// [`QueryEnvelope`]: super::QueryEnvelope
    #[must_use]
    pub fn to_diagnostic(&self) -> ExportedDiagnostic {
        ExportedDiagnostic {
            code: "KUL-Q01".to_string(),
            severity: "error",
            message: self.to_string(),
            primary: None,
            related: Vec::new(),
        }
    }
}

/// Evaluate a [`Query`] over a checked project's [`ResolvedDocument`],
/// returning the matching kin-set members in the pinned deterministic order.
///
/// This is the single evaluation path. An unknown anchor is a typed
/// [`QueryEvalError`], not an empty result.
///
/// # Errors
///
/// Returns [`QueryEvalError::UnknownPerson`] when the anchor id names no
/// person.
pub fn evaluate<'a>(
    resolved: &'a ResolvedDocument,
    query: &Query,
) -> Result<Vec<KinMember<'a>>, QueryEvalError> {
    match &query.source {
        QuerySource::KinOf { anchor, pattern } => {
            let ego = resolved
                .person(anchor)
                .ok_or_else(|| QueryEvalError::UnknownPerson { id: anchor.clone() })?;
            Ok(eval_kin(resolved, ego, pattern))
        }
    }
}

/// A directed parent-child edge in the per-invocation adjacency: the person
/// the edge lands on plus the edge kind (bio / adoptive).
struct Edge<'a> {
    person: &'a PersonStmt,
    kind: ParentLinkKind,
}

/// A marriage (`across`) edge: the co-spouse landed on plus the marriage's id,
/// status, and end reason, so [`make_across_hop`] can build the backbone hop
/// without re-reading the marriage record.
struct SpouseEdge<'a> {
    person: &'a PersonStmt,
    marriage: &'a str,
    status: MarriageStatus,
    end_reason: Option<String>,
}

/// The engine's own in-memory adjacency, built once per [`evaluate`] call
/// and thrown away after. `up` maps a person id to its parents (the
/// resolved parent links); `down` is the inverse — a person id to its
/// children; `across` maps a person id to their co-spouses (both directions
/// of every marriage).
struct Adjacency<'a> {
    up: HashMap<&'a str, Vec<Edge<'a>>>,
    down: HashMap<&'a str, Vec<Edge<'a>>>,
    across: HashMap<&'a str, Vec<SpouseEdge<'a>>>,
}

impl<'a> Adjacency<'a> {
    /// Build all three directions from the resolved graph. Iterates in source
    /// order so path enumeration is deterministic even before the final member
    /// sort.
    fn build(resolved: &'a ResolvedDocument) -> Self {
        let mut up: HashMap<&str, Vec<Edge>> = HashMap::new();
        let mut down: HashMap<&str, Vec<Edge>> = HashMap::new();
        for child in resolved.persons() {
            for link in resolved.parents_of(child) {
                up.entry(child.id.name.as_str()).or_default().push(Edge {
                    person: link.parent,
                    kind: link.kind,
                });
                down.entry(link.parent.id.name.as_str())
                    .or_default()
                    .push(Edge {
                        person: child,
                        kind: link.kind,
                    });
            }
        }

        let mut across: HashMap<&str, Vec<SpouseEdge>> = HashMap::new();
        for marriage in resolved.marriages() {
            // `status` = ended iff the record carries an end date or reason
            // (R05 keeps them together, but either alone is enough to tag).
            let status = if marriage.end().is_some() || marriage.end_reason().is_some() {
                MarriageStatus::Ended
            } else {
                MarriageStatus::Ongoing
            };
            let end_reason = marriage.end_reason().map(|er| match &er.value {
                EndReason::Divorce => "divorce".to_string(),
                EndReason::Unknown(s) => s.clone(),
            });
            let a = resolved.person(&marriage.spouse_a.name);
            let b = resolved.person(&marriage.spouse_b.name);
            // Both spouses must resolve, and a self-marriage (R04) crosses to
            // nobody — skip it rather than emit a no-op hop.
            if let (Some(a), Some(b)) = (a, b)
                && a.id.name != b.id.name
            {
                let id = marriage.id.name.as_str();
                across
                    .entry(a.id.name.as_str())
                    .or_default()
                    .push(SpouseEdge {
                        person: b,
                        marriage: id,
                        status,
                        end_reason: end_reason.clone(),
                    });
                across
                    .entry(b.id.name.as_str())
                    .or_default()
                    .push(SpouseEdge {
                        person: a,
                        marriage: id,
                        status,
                        end_reason,
                    });
            }
        }

        Adjacency { up, down, across }
    }

    /// Neighbours of `node` in the traversal direction (`Ancestor` walks
    /// `up`, `Descendant` walks `down`).
    fn neighbours(&self, node: &str, role: LinealRole) -> &[Edge<'a>] {
        let map = match role {
            LinealRole::Ancestor => &self.up,
            LinealRole::Descendant => &self.down,
        };
        map.get(node).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The co-spouses of `node` across every marriage they are in.
    fn spouses(&self, node: &str) -> &[SpouseEdge<'a>] {
        self.across.get(node).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Evaluate a kin pattern from `ego`. Enumerate every qualifying simple path
/// (lineal `up+` / `down+`, or collateral `up+ down+` through one apex),
/// canonicalize couple-apex backbones and de-duplicate, derive a descriptor
/// per surviving path, apply the pattern's optional filters, and sort into
/// the pinned order.
fn eval_kin<'a>(
    resolved: &'a ResolvedDocument,
    ego: &'a PersonStmt,
    pattern: &KinPattern,
) -> Vec<KinMember<'a>> {
    let adjacency = Adjacency::build(resolved);

    // Phase 1: raw path enumeration. The blood-only classifications keep their
    // dedicated single-segment walkers; anything that can spend a marriage hop
    // (an `affinity` / `affinalHops` filter, or the unclassified `any` shape)
    // routes through the general affinal walk instead.
    let mut raw: Vec<(&'a PersonStmt, Vec<PathHop>)> = Vec::new();
    let across_budget = affinal_budget(pattern);
    if across_budget > 0 || matches!(pattern.classification, PatternClassification::Any { .. }) {
        let (up_max, down_max) = affinal_vertical_bounds(&pattern.classification);
        let mut walk = AffinalWalk {
            adjacency: &adjacency,
            up_max,
            down_max,
            across_max: across_budget,
            classification: &pattern.classification,
            emit: |alter, path| raw.push((alter, path)),
        };
        walk.walk(
            ego,
            &mut vec![ego.id.name.as_str()],
            &mut Vec::new(),
            0,
            0,
            0,
            false,
        );
    } else {
        match pattern.classification {
            PatternClassification::Lineal { role, generations } => {
                let mut walk = LinealWalk {
                    adjacency: &adjacency,
                    role,
                    generations,
                    emit: |alter, path| raw.push((alter, path)),
                };
                walk.descend(ego, &mut vec![ego.id.name.as_str()], &mut Vec::new());
            }
            PatternClassification::Collateral { up, down } => {
                let mut walk = CollateralWalk {
                    adjacency: &adjacency,
                    up_max: up.max,
                    down_max: down.max,
                    matches: |u, d| up.contains(u) && down.contains(d),
                    emit: |alter, path| raw.push((alter, path)),
                };
                walk.ascend(ego, &mut vec![ego.id.name.as_str()], &mut Vec::new());
            }
            PatternClassification::CollateralByDegree { degree, removed } => {
                // min(u,d) = degree+1 and max(u,d) = degree+1+removed, so both
                // hop counts are bounded by degree.max+1+removed.max —
                // unbounded (None) only if either range is, in which case the
                // simple-path guard still terminates the walk.
                let bound = match (degree.max, removed.max) {
                    (Some(dm), Some(rm)) => Some(dm + 1 + rm),
                    _ => None,
                };
                let mut walk = CollateralWalk {
                    adjacency: &adjacency,
                    up_max: bound,
                    down_max: bound,
                    matches: |u: u32, d: u32| {
                        degree.contains(u.min(d).saturating_sub(1))
                            && removed.contains(u.abs_diff(d))
                    },
                    emit: |alter, path| raw.push((alter, path)),
                };
                walk.ascend(ego, &mut vec![ego.id.name.as_str()], &mut Vec::new());
            }
            // `Any` always takes the affinal branch above.
            PatternClassification::Any { .. } => {
                unreachable!("Any routes through the affinal walk")
            }
        }
    }

    // Phase 2: canonicalize couple-apex backbones, de-duplicate the resulting
    // "same relationship fact" paths, derive descriptors, and filter. A step
    // path shadowed by a real parent / child / shared-parent edge is a derived
    // stand-in for that fact, so it is suppressed here.
    let mut members = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for (alter, path) in raw {
        let path = canonicalize_apex(resolved, ego, path);
        // Paths that canonicalize identically are one fact (father-route and
        // mother-route through the same couple apex); keep the first.
        if !seen.insert(backbone_key(&path)) {
            continue;
        }
        let descriptor = RelationshipDescriptor::derive(resolved, ego, alter, path);
        if pattern_matches(pattern, &descriptor)
            && !step_subsumed(resolved, ego, alter, &descriptor)
        {
            members.push(KinMember {
                person: alter,
                descriptor,
            });
        }
    }

    sort_members(&mut members);
    members
}

/// The path's marriage-hop budget: how many `across` hops the traversal may
/// spend, always capped at [`AFFINAL_CEILING`]. An explicit `affinalHops`
/// bound sets it (to its upper bound, or the ceiling when unbounded); with no
/// such bound, a `step` / `inLaw` affinity filter opens it to the ceiling and
/// anything else keeps it at zero (blood only).
fn affinal_budget(pattern: &KinPattern) -> u32 {
    if let Some(hops) = pattern.affinal_hops {
        return hops.max.unwrap_or(AFFINAL_CEILING).min(AFFINAL_CEILING);
    }
    match pattern.affinity {
        Some(Affinity::Step | Affinity::InLaw) => AFFINAL_CEILING,
        _ => 0,
    }
}

/// The `(up_max, down_max)` vertical bounds a classification imposes on the
/// affinal walk — the total ascent / descent hops across all blood segments.
fn affinal_vertical_bounds(classification: &PatternClassification) -> (Option<u32>, Option<u32>) {
    match classification {
        PatternClassification::Lineal { role, generations } => match role {
            LinealRole::Ancestor => (generations.max, Some(0)),
            LinealRole::Descendant => (Some(0), generations.max),
        },
        PatternClassification::Collateral { up, down } => (up.max, down.max),
        PatternClassification::CollateralByDegree { degree, removed } => {
            let bound = match (degree.max, removed.max) {
                (Some(dm), Some(rm)) => Some(dm + 1 + rm),
                _ => None,
            };
            (bound, bound)
        }
        PatternClassification::Any { max_up, max_down } => (Some(*max_up), Some(*max_down)),
    }
}

/// Whether `descriptor` is a step path suppressed by a real relationship fact
/// (issue #258). A step shape is a *derived stand-in* for parenthood, never
/// emitted alongside the real edge it stands in for:
/// - step-parent (lineal ancestor 1) whose alter is an actual (bio/adoptive)
///   parent of ego;
/// - step-child (lineal descendant 1) whose alter is an actual child of ego;
/// - step-sibling (collateral 1/1) who shares ≥1 actual parent with ego.
///
/// Only `step` affinity is subject; blood and in-law paths are never
/// suppressed. An explicit adoption edge counts as a real parent, so it always
/// beats the step reading.
fn step_subsumed(
    resolved: &ResolvedDocument,
    ego: &PersonStmt,
    alter: &PersonStmt,
    descriptor: &RelationshipDescriptor,
) -> bool {
    if descriptor.affinity != Affinity::Step {
        return false;
    }
    let is_parent_of = |child: &PersonStmt, parent_id: &str| {
        resolved
            .parents_of(child)
            .iter()
            .any(|link| link.parent.id.name == parent_id)
    };
    match descriptor.classification {
        // Step-parent shadowed by a real parent edge to the same person.
        Classification::Lineal {
            role: LinealRole::Ancestor,
            generations: 1,
        } => is_parent_of(ego, &alter.id.name),
        // Step-child shadowed by a real child edge.
        Classification::Lineal {
            role: LinealRole::Descendant,
            generations: 1,
        } => is_parent_of(alter, &ego.id.name),
        // Step-sibling shadowed by a shared parent (full / half sibling).
        Classification::Collateral { up: 1, down: 1, .. } => {
            let ego_parents: HashSet<&str> = resolved
                .parents_of(ego)
                .iter()
                .map(|link| link.parent.id.name.as_str())
                .collect();
            resolved
                .parents_of(alter)
                .iter()
                .any(|link| ego_parents.contains(link.parent.id.name.as_str()))
        }
        _ => false,
    }
}

/// Rewrite a collateral path's apex hop to route through the couple apex's
/// smaller-id co-parent, so that the two co-parent routes collapse to one
/// backbone under [`backbone_key`]. A no-op for lineal paths (no junction)
/// and for single-parent junctions (nothing to canonicalize).
fn canonicalize_apex<'a>(
    resolved: &'a ResolvedDocument,
    ego: &'a PersonStmt,
    mut path: Vec<PathHop>,
) -> Vec<PathHop> {
    // Extract the rewrite target before mutating (drops the junction's borrow
    // of `path`).
    let rewrite = junction_of(resolved, ego, &path).and_then(|j| {
        j.is_couple_apex.then(|| {
            (
                (j.up - 1) as usize,
                j.canonical_apex.id.name.clone(),
                gender_of(j.canonical_apex),
            )
        })
    });
    if let Some((apex_idx, to, gender)) = rewrite
        && let PathHop::Up { edge, .. } = path[apex_idx]
    {
        // The couple apex's co-parents share the same edge kind toward each
        // branch sibling, so the ascent hop's `edge` is preserved.
        path[apex_idx] = PathHop::Up { to, gender, edge };
    }
    path
}

/// Whether a derived descriptor passes the pattern's optional edge-nature,
/// sharing, side, affinity, and affinal-hop-count filters (each `None` matches
/// everything). Applied after derivation because sharing / side / affinity are
/// read from the derived descriptor, not from the raw hop sequence.
fn pattern_matches(pattern: &KinPattern, descriptor: &RelationshipDescriptor) -> bool {
    pattern
        .edge_nature
        .is_none_or(|want| descriptor.edge_nature == want)
        && pattern
            .sharing
            .is_none_or(|want| descriptor.sharing == want)
        && pattern.side.is_none_or(|want| descriptor.side == want)
        && pattern
            .affinity
            .is_none_or(|want| descriptor.affinity == want)
        && pattern
            .affinal_hops
            .is_none_or(|want| want.contains(across_count(&descriptor.path)))
}

/// The number of marriage (`across`) hops on a path.
fn across_count(path: &[PathHop]) -> u32 {
    path.iter()
        .filter(|h| matches!(h, PathHop::Across { .. }))
        .count() as u32
}

/// A depth-first lineal simple-path traversal (`up+` or `down+`). Bundles the
/// immutable per-query configuration (adjacency, direction, generation
/// bounds) and the emit sink so the recursion threads only the mutable path
/// state (`visited`, `backbone`).
struct LinealWalk<'a, 'adj, F> {
    adjacency: &'adj Adjacency<'a>,
    role: LinealRole,
    generations: IntRange,
    emit: F,
}

impl<'a, 'adj, F: FnMut(&'a PersonStmt, Vec<PathHop>)> LinealWalk<'a, 'adj, F> {
    /// Visit every neighbour of `node` in the traversal direction. `visited`
    /// holds the ids on the current path (anchor included) — the
    /// unconditional cycle guard; `backbone` is the hop sequence built so
    /// far. `emit` fires once per distinct qualifying path.
    fn descend(
        &mut self,
        node: &'a PersonStmt,
        visited: &mut Vec<&'a str>,
        backbone: &mut Vec<PathHop>,
    ) {
        for edge in self.adjacency.neighbours(node.id.name.as_str(), self.role) {
            let next_id = edge.person.id.name.as_str();
            // Simple-path rule: never revisit a person already on this path.
            // This is the cycle guard — traversal terminates on every input.
            if visited.contains(&next_id) {
                continue;
            }
            backbone.push(make_hop(self.role, edge));
            visited.push(next_id);

            let depth = backbone.len() as u32;
            if self.generations.contains(depth) {
                (self.emit)(edge.person, backbone.clone());
            }
            // Descend further only while the range's upper bound allows it;
            // an unbounded range recurses until the simple-path guard stops
            // it.
            if self.generations.max.is_none_or(|max| depth < max) {
                self.descend(edge.person, visited, backbone);
            }

            visited.pop();
            backbone.pop();
        }
    }
}

/// A depth-first collateral simple-path traversal: ascend to an apex, then
/// descend to the alter, sharing one `visited` set across both phases so the
/// no-backtracking rule (which guarantees the two branch siblings are
/// distinct) holds over the whole path. `matches(u, d)` decides which
/// `(up, down)` hop-count pairs qualify; `up_max` / `down_max` bound the
/// search (an unbounded `None` relies on the simple-path guard to terminate).
struct CollateralWalk<'a, 'adj, M, F> {
    adjacency: &'adj Adjacency<'a>,
    up_max: Option<u32>,
    down_max: Option<u32>,
    matches: M,
    emit: F,
}

impl<'a, 'adj, M: Fn(u32, u32) -> bool, F: FnMut(&'a PersonStmt, Vec<PathHop>)>
    CollateralWalk<'a, 'adj, M, F>
{
    /// Ascend from `node`. `path` holds the ascent hops so far (`u = path.len`
    /// once we treat `node` as an apex candidate). Every node reached at
    /// `u ≥ 1` is a candidate apex, from which we launch a descent; then we
    /// ascend one hop further while `up_max` allows.
    fn ascend(
        &mut self,
        node: &'a PersonStmt,
        visited: &mut Vec<&'a str>,
        path: &mut Vec<PathHop>,
    ) {
        let u = path.len() as u32;
        if u >= 1 {
            self.descend(node, visited, path, u);
        }
        if self.up_max.is_none_or(|max| u < max) {
            for edge in self
                .adjacency
                .neighbours(node.id.name.as_str(), LinealRole::Ancestor)
            {
                let next_id = edge.person.id.name.as_str();
                if visited.contains(&next_id) {
                    continue;
                }
                path.push(make_hop(LinealRole::Ancestor, edge));
                visited.push(next_id);
                self.ascend(edge.person, visited, path);
                visited.pop();
                path.pop();
            }
        }
    }

    /// Descend from `node` (a child chain rooted at the apex). `u` is the fixed
    /// ascent length; `d = path.len() − u` is the descent depth. `emit` fires
    /// once per qualifying `(u, d)` path; the first descent hop cannot reach
    /// the ego-branch sibling (it is on the ascent, hence in `visited`).
    fn descend(
        &mut self,
        node: &'a PersonStmt,
        visited: &mut Vec<&'a str>,
        path: &mut Vec<PathHop>,
        u: u32,
    ) {
        for edge in self
            .adjacency
            .neighbours(node.id.name.as_str(), LinealRole::Descendant)
        {
            let next_id = edge.person.id.name.as_str();
            if visited.contains(&next_id) {
                continue;
            }
            path.push(make_hop(LinealRole::Descendant, edge));
            visited.push(next_id);

            let d = path.len() as u32 - u;
            if (self.matches)(u, d) {
                (self.emit)(edge.person, path.clone());
            }
            if self.down_max.is_none_or(|max| d < max) {
                self.descend(edge.person, visited, path, u);
            }

            visited.pop();
            path.pop();
        }
    }
}

/// Build the hop for stepping across `edge` in the traversal direction.
fn make_hop(role: LinealRole, edge: &Edge<'_>) -> PathHop {
    let to = edge.person.id.name.clone();
    let gender = gender_of(edge.person);
    let hop_edge = HopEdge::from(edge.kind);
    match role {
        LinealRole::Ancestor => PathHop::Up {
            to,
            gender,
            edge: hop_edge,
        },
        LinealRole::Descendant => PathHop::Down {
            to,
            gender,
            edge: hop_edge,
        },
    }
}

/// Build the marriage (`across`) hop for stepping over `edge`.
fn make_across_hop(edge: &SpouseEdge<'_>) -> PathHop {
    PathHop::Across {
        to: edge.person.id.name.clone(),
        gender: gender_of(edge.person),
        marriage: edge.marriage.to_string(),
        status: edge.status,
        end_reason: edge.end_reason.clone(),
    }
}

/// The general affinal simple-path traversal (issue #258): 1–3 blood segments
/// (each an `up* down*` run) joined by up to [`across_max`](AffinalWalk::
/// across_max) marriage hops. The recursion carries the running `(u, d)`
/// vertical counts, the spent `across` count, and a `descending` flag that
/// enforces the per-segment `up*` before `down*` ordering; a marriage hop
/// opens a fresh segment (clears `descending`). Every node whose running
/// `(u, d)` satisfies the classification is emitted; the marriage-hop-count
/// and affinity filters are applied in Phase 2. `up_max` / `down_max` bound the
/// *total* ascent / descent across all segments; the simple-path guard
/// (`visited`) terminates the walk on every input.
struct AffinalWalk<'a, 'adj, F> {
    adjacency: &'adj Adjacency<'a>,
    up_max: Option<u32>,
    down_max: Option<u32>,
    across_max: u32,
    classification: &'adj PatternClassification,
    emit: F,
}

impl<'a, 'adj, F: FnMut(&'a PersonStmt, Vec<PathHop>)> AffinalWalk<'a, 'adj, F> {
    /// Whether the running `(u, d)` vertical counts satisfy the classification
    /// (the marriage-hop and affinity filters are Phase 2 concerns).
    fn ud_matches(&self, u: u32, d: u32) -> bool {
        match self.classification {
            PatternClassification::Lineal { role, generations } => match role {
                LinealRole::Ancestor => d == 0 && generations.contains(u),
                LinealRole::Descendant => u == 0 && generations.contains(d),
            },
            PatternClassification::Collateral { up, down } => up.contains(u) && down.contains(d),
            PatternClassification::CollateralByDegree { degree, removed } => {
                degree.contains(u.min(d).saturating_sub(1)) && removed.contains(u.abs_diff(d))
            }
            PatternClassification::Any { max_up, max_down } => u <= *max_up && d <= *max_down,
        }
    }

    /// Walk from `node`. `visited` is the cycle guard (ids on the current
    /// path, ego included); `backbone` the hops so far; `u` / `d` the total
    /// ascent / descent; `across` the marriage hops spent; `descending` marks
    /// that the current segment has begun descending (so no further `up`).
    #[allow(clippy::too_many_arguments)] // one recursion state; splitting hurts clarity.
    fn walk(
        &mut self,
        node: &'a PersonStmt,
        visited: &mut Vec<&'a str>,
        backbone: &mut Vec<PathHop>,
        u: u32,
        d: u32,
        across: u32,
        descending: bool,
    ) {
        // Ascend — only while the current segment has not turned to descent.
        if !descending && self.up_max.is_none_or(|max| u < max) {
            for edge in self
                .adjacency
                .neighbours(node.id.name.as_str(), LinealRole::Ancestor)
            {
                self.step(
                    visited,
                    backbone,
                    edge.person,
                    make_hop(LinealRole::Ancestor, edge),
                    u + 1,
                    d,
                    across,
                    false,
                );
            }
        }
        // Descend — begins (or continues) the segment's descent.
        if self.down_max.is_none_or(|max| d < max) {
            for edge in self
                .adjacency
                .neighbours(node.id.name.as_str(), LinealRole::Descendant)
            {
                self.step(
                    visited,
                    backbone,
                    edge.person,
                    make_hop(LinealRole::Descendant, edge),
                    u,
                    d + 1,
                    across,
                    true,
                );
            }
        }
        // Cross a marriage — opens a fresh segment (ascent allowed again).
        if across < self.across_max {
            for edge in self.adjacency.spouses(node.id.name.as_str()) {
                self.step(
                    visited,
                    backbone,
                    edge.person,
                    make_across_hop(edge),
                    u,
                    d,
                    across + 1,
                    false,
                );
            }
        }
    }

    /// Take one hop to `next` (guarded by the simple-path rule), emit if the
    /// landing `(u, d)` matches, then recurse and backtrack.
    #[allow(clippy::too_many_arguments)] // shared push/emit/recurse/pop step.
    fn step(
        &mut self,
        visited: &mut Vec<&'a str>,
        backbone: &mut Vec<PathHop>,
        next: &'a PersonStmt,
        hop: PathHop,
        u: u32,
        d: u32,
        across: u32,
        descending: bool,
    ) {
        let next_id = next.id.name.as_str();
        // Simple-path rule: never revisit a person already on this path.
        if visited.contains(&next_id) {
            return;
        }
        backbone.push(hop);
        visited.push(next_id);
        if self.ud_matches(u, d) {
            (self.emit)(next, backbone.clone());
        }
        self.walk(next, visited, backbone, u, d, across, descending);
        visited.pop();
        backbone.pop();
    }
}

/// The pinned deterministic member order (snapshots depend on it): by
/// (alter person id, codepoint ascending) → (path hop count ascending) →
/// (serialized backbone, codepoint ascending).
fn sort_members(members: &mut [KinMember<'_>]) {
    members.sort_by(|a, b| {
        a.descriptor
            .alter_id
            .cmp(&b.descriptor.alter_id)
            .then_with(|| a.descriptor.path.len().cmp(&b.descriptor.path.len()))
            .then_with(|| backbone_key(&a.descriptor.path).cmp(&backbone_key(&b.descriptor.path)))
    });
}

/// A total, codepoint-comparable serialization of a path backbone, used only
/// as the final tie-breaker in [`sort_members`]. Not a wire format — the
/// committed serialization is the descriptor's `path`; this is a stable key.
fn backbone_key(path: &[PathHop]) -> String {
    let mut key = String::new();
    for hop in path {
        match hop {
            PathHop::Up { to, edge, .. } => {
                key.push('u');
                push_edge(&mut key, *edge);
                key.push(':');
                key.push_str(to);
            }
            PathHop::Down { to, edge, .. } => {
                key.push('d');
                push_edge(&mut key, *edge);
                key.push(':');
                key.push_str(to);
            }
            PathHop::Across { to, marriage, .. } => {
                key.push('a');
                key.push(':');
                key.push_str(marriage);
                key.push(':');
                key.push_str(to);
            }
        }
        key.push('|');
    }
    key
}

fn push_edge(key: &mut String, edge: HopEdge) {
    key.push(match edge {
        HopEdge::Bio => 'b',
        HopEdge::Adoptive => 'a',
    });
}

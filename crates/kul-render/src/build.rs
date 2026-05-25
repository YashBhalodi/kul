//! The kinship-native → [`RenderShape`] projection algorithm.
//!
//! Reads the kinship-native graph (persons, marriages, parenthood
//! links) and produces the hierarchical card-slot tree plus the flat
//! edge list defined in [`crate::shape`]. Realises every canonical UI
//! pattern principle — the normative description lives in
//! [`docs/canonical-ui-pattern.md`](../../docs/canonical-ui-pattern.md).

use std::collections::{BTreeMap, HashMap, HashSet};

use kul_core::export::{
    ExportedDate, ExportedGraph, ExportedMarriage, ExportedParenthoodLink, ExportedPerson,
};

use crate::shape::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind, GhostReason, MarriageBar, MarriageBranch,
    PersonCard, SlotKind,
};

/// Entry point for [`crate::transform`].
///
/// Returns `(components, edges)` already in source order (left-to-right by
/// first-relevant-declaration source position). Pure function over the
/// kinship-native graph.
pub(crate) fn build(graph: &ExportedGraph) -> (Vec<Component>, Vec<Edge>) {
    let index = Index::new(graph);
    let edges = build_edges(graph);
    let components = build_components(&index);
    (components, edges)
}

/// Flat list of canonical edges, mirroring the export's parenthood
/// links one-to-one. Birth edges first per child, then adoptions in
/// source order — matching the export's declaration-order discipline so
/// downstream consumers can correlate by index.
fn build_edges(graph: &ExportedGraph) -> Vec<Edge> {
    graph
        .parenthood_links
        .iter()
        .map(|link| Edge {
            kind: match link.kind {
                "biological" => EdgeKind::Birth,
                "adoptive" => EdgeKind::Adoption,
                other => panic!("unknown parenthood link kind: {other}"),
            },
            child_id: link.child_id.clone(),
            marriage_id: link.marriage_id.clone(),
            start: link.start.clone(),
            end: link.end.clone(),
        })
        .collect()
}

/// Derived facts about a person — pre-computed once so the tree-walk
/// below stays a single linear pass over the precomputed data.
#[derive(Debug)]
struct PersonFacts<'a> {
    person: &'a ExportedPerson,
    /// Marriages this person hosts (spouses[0]), declaration order.
    hosted_marriages: Vec<usize>,
    /// Marriages this person joins (spouses[1]), declaration order.
    joined_marriages: Vec<usize>,
    /// Biological parents' marriage id, if declared (at most one).
    bio_marriage: Option<String>,
    /// Adoption marriage ids in **source-declaration order** — the
    /// most-recent adoption is *not* always last; resolve via
    /// [`PersonFacts::canonical_adoption`] which applies the
    /// `start:`-date sort with declaration-order tiebreak per
    /// past intimacies emit ghosts.
    adoption_marriages: Vec<String>,
    /// First-declared un-ended marriage index (by source order across
    /// hosted ∪ joined) — the marriage that determines current
    /// intimacy per current-intimacy placement (ADR-0017). `None` if
    /// every marriage carries `end:` or this person has no marriages.
    /// The semantics is "first-declared un-ended participation wins."
    primary_marriage: Option<usize>,
    /// Generation index under the canonical-family graph. Computed
    /// fixpoint-style: roots at 0, child = max(canonical-family
    /// spouses' gens) + 1.
    generation: u32,
}

impl<'a> PersonFacts<'a> {
    fn canonical_family(&self) -> Option<&str> {
        self.canonical_adoption().or(self.bio_marriage.as_deref())
    }

    fn canonical_adoption(&self) -> Option<&str> {
        // Most-recent: pick the latest `start:` among declared
        // adoptions, with declaration-order tiebreak. The sort runs
        // up front in `Index::new` so callers never re-derive "which
        // adoption is canonical" per past intimacies emit ghosts.
        if self.adoption_marriages.is_empty() {
            return None;
        }
        // The caller resolves the sort against the marriage index.
        // Kept thin here so we don't pull `Index` into `PersonFacts`.
        Some(self.adoption_marriages[0].as_str())
    }
}

/// Precomputed indices over the input graph. Held by `build` for the
/// duration of one transformation; constructed in `Index::new`.
struct Index<'a> {
    graph: &'a ExportedGraph,
    /// Map from person id to facts. Order-stable enough that iteration
    /// over `graph.persons` directly is used wherever source order
    /// matters; this lookup is for cross-references.
    persons_by_id: HashMap<&'a str, usize>,
    /// Same, for marriages.
    marriages_by_id: HashMap<&'a str, usize>,
    /// Person facts, indexed by `persons_by_id`.
    persons: Vec<PersonFacts<'a>>,
}

impl<'a> Index<'a> {
    fn new(graph: &'a ExportedGraph) -> Self {
        let persons_by_id: HashMap<&str, usize> = graph
            .persons
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.as_str(), i))
            .collect();
        let marriages_by_id: HashMap<&str, usize> = graph
            .marriages
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id.as_str(), i))
            .collect();
        let adoption_links: HashMap<(String, String), &ExportedParenthoodLink> = graph
            .parenthood_links
            .iter()
            .filter(|l| l.kind == "adoptive")
            .map(|l| ((l.child_id.clone(), l.marriage_id.clone()), l))
            .collect();

        // Per-person derived facts: hosted/joined marriages, bio
        // parents, adoption marriages (in declaration order — re-sorted
        // for canonical-pick later).
        let mut persons: Vec<PersonFacts<'a>> = graph
            .persons
            .iter()
            .map(|p| PersonFacts {
                person: p,
                hosted_marriages: Vec::new(),
                joined_marriages: Vec::new(),
                bio_marriage: None,
                adoption_marriages: Vec::new(),
                primary_marriage: None,
                generation: 0,
            })
            .collect();

        for (idx, m) in graph.marriages.iter().enumerate() {
            if let Some(&host) = persons_by_id.get(m.spouses[0].as_str()) {
                persons[host].hosted_marriages.push(idx);
            }
            if let Some(&joining) = persons_by_id.get(m.spouses[1].as_str()) {
                persons[joining].joined_marriages.push(idx);
            }
        }

        for link in &graph.parenthood_links {
            let Some(&child) = persons_by_id.get(link.child_id.as_str()) else {
                continue;
            };
            match link.kind {
                "biological" => persons[child].bio_marriage = Some(link.marriage_id.clone()),
                "adoptive" => persons[child]
                    .adoption_marriages
                    .push(link.marriage_id.clone()),
                _ => {}
            }
        }

        // Sort each person's adoption marriages by `start:` date
        // (descending), declaration-order tiebreak — the most-recent
        // sits at index 0 so `canonical_adoption()` is a one-line lookup
        // and `past adoptions` is `[1..]`. The ghost-emission rule.
        for facts in persons.iter_mut() {
            let person_id = facts.person.id.clone();
            facts.adoption_marriages.sort_by(|a, b| {
                let key_a = adoption_sort_key(&adoption_links, &person_id, a);
                let key_b = adoption_sort_key(&adoption_links, &person_id, b);
                key_b.cmp(&key_a)
            });
        }

        // Primary marriage: first-declared un-ended across hosted ∪
        // joined: first-declared un-ended participation, by source
        // order (current-intimacy placement, ADR-0017). Source order
        // across the union: a person's
        // hosted_marriages and joined_marriages each hold marriage
        // indices in declaration order, and marriage indices grow with
        // source position in the export, so the minimum index across
        // the union is the first-declared un-ended participation.
        for facts in persons.iter_mut() {
            facts.primary_marriage = facts
                .hosted_marriages
                .iter()
                .chain(facts.joined_marriages.iter())
                .copied()
                .filter(|&m| graph.marriages[m].end.is_none())
                .min();
        }

        let mut index = Self {
            graph,
            persons_by_id,
            marriages_by_id,
            persons,
        };
        index.compute_generations();
        index
    }

    /// Compute each person's generation by repeatedly relaxing the
    /// "child = max(canonical-family spouses) + 1" rule until stable.
    /// The export envelope is acyclic by R13 so this fixpoint converges
    /// in at most `persons.len()` iterations.
    fn compute_generations(&mut self) {
        // Initialise: a person without a canonical family is a root at
        // generation 0. The relaxation loop only updates persons whose
        // canonical-family spouses have all been computed.
        let mut known: Vec<Option<u32>> = vec![None; self.persons.len()];
        for (i, facts) in self.persons.iter().enumerate() {
            if facts.canonical_family().is_none() {
                known[i] = Some(0);
            }
        }
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..self.persons.len() {
                if known[i].is_some() {
                    continue;
                }
                let Some(family_id) = self.persons[i].canonical_family() else {
                    continue;
                };
                let Some(&marriage_idx) = self.marriages_by_id.get(family_id) else {
                    continue;
                };
                let marriage = &self.graph.marriages[marriage_idx];
                let spouse_indices: Vec<usize> = marriage
                    .spouses
                    .iter()
                    .filter_map(|s| self.persons_by_id.get(s.as_str()).copied())
                    .collect();
                if spouse_indices.iter().any(|&j| known[j].is_none()) {
                    continue;
                }
                let max_parent = spouse_indices
                    .iter()
                    .map(|&j| known[j].unwrap_or(0))
                    .max()
                    .unwrap_or(0);
                known[i] = Some(max_parent + 1);
                changed = true;
            }
        }
        for (i, facts) in self.persons.iter_mut().enumerate() {
            facts.generation = known[i].unwrap_or(0);
        }
    }

    fn person(&self, id: &str) -> Option<&PersonFacts<'a>> {
        self.persons_by_id.get(id).map(|&i| &self.persons[i])
    }

    fn marriage(&self, id: &str) -> Option<&'a ExportedMarriage> {
        self.marriages_by_id
            .get(id)
            .map(|&i| &self.graph.marriages[i])
    }

    /// "Bar-anchor" of a marriage per current-intimacy placement: the host's canonical-family
    /// marriage if the host has one (the bar nests inside that
    /// family); otherwise `None` (floating mini-component).
    fn bar_anchor(&self, marriage: &ExportedMarriage) -> Option<String> {
        let host = self.person(&marriage.spouses[0])?;
        host.canonical_family().map(str::to_string)
    }

    /// Where is this person's canonical card located? See current-intimacy
    /// placement (the most-recent-adoption rule is baked into
    /// `canonical_family`).
    fn canonical_location(&self, facts: &PersonFacts<'_>) -> CanonicalLocation {
        if let Some(primary_idx) = facts.primary_marriage {
            let primary = &self.graph.marriages[primary_idx];
            let host_id = &primary.spouses[0];
            if host_id == &facts.person.id {
                // Host of own current intimacy — canonical card lives
                // in host's canonical-family children row (which is
                // also where this marriage's bar nests), or at the bar
                // host-slot of a floating mini-component if no family.
                match facts.canonical_family() {
                    Some(family) => CanonicalLocation::ChildOf(family.to_string()),
                    None => CanonicalLocation::HostOfFloating(primary.id.clone()),
                }
            } else {
                // Joining spouse of own current intimacy.
                CanonicalLocation::JoiningOf(primary.id.clone())
            }
        } else if let Some(family) = facts.canonical_family() {
            CanonicalLocation::ChildOf(family.to_string())
        } else {
            CanonicalLocation::Orphan
        }
    }
}

/// Adoption ordering key: (start date desc, declaration-order tiebreak).
fn adoption_sort_key<'a>(
    links: &'a HashMap<(String, String), &'a ExportedParenthoodLink>,
    person_id: &str,
    marriage_id: &str,
) -> SortableDate {
    links
        .get(&(person_id.to_string(), marriage_id.to_string()))
        .and_then(|l| l.start.as_ref())
        .map(SortableDate::from)
        .unwrap_or(SortableDate::missing())
}

/// Comparable date representation for picking "most recent."
/// Missing dates sort earliest so they lose the tiebreak — a deliberate
/// choice consistent with the export's date-precision policy (a `start:`
/// the source omitted is informationally "less specific" than any real
/// date).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SortableDate(i64);

impl SortableDate {
    fn missing() -> Self {
        Self(i64::MIN)
    }
}

impl From<&ExportedDate> for SortableDate {
    fn from(d: &ExportedDate) -> Self {
        // `d.value` is `YYYY[-MM[-DD]]` per the export schema. Encode
        // as `YYYY*10000 + MM*100 + DD` so lexicographic order on the
        // numeric key matches calendar order across precisions.
        let mut parts = d.value.split('-');
        let year: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let month: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let day: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        Self(year * 10_000 + month * 100 + day)
    }
}

/// Where a person's canonical card anchors.
#[derive(Debug, Clone)]
enum CanonicalLocation {
    /// Children row of this marriage (bio or canonical-adoptive parents).
    ChildOf(String),
    /// Joining slot of this marriage's bar (current intimacy as joining
    /// spouse).
    JoiningOf(String),
    /// Host slot of this marriage's bar — only when the host has no
    /// canonical family, so the marriage is a floating mini-component.
    HostOfFloating(String),
    /// No anchor — a lone-card orphan, or the fallback (joining spouse of an
    /// ended marriage with no birth family).
    Orphan,
}

// ---------------------------------------------------------------------
// Component discovery — union-find over (persons, marriages).
// ---------------------------------------------------------------------

struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut cur = x;
        while self.parent[cur] != root {
            let next = self.parent[cur];
            self.parent[cur] = root;
            cur = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}

/// Top-level "build components" pass. Produces components in source
/// order (source position of each component's first relevant
/// declaration).
fn build_components(index: &Index<'_>) -> Vec<Component> {
    let n_persons = index.persons.len();
    let n_marriages = index.graph.marriages.len();
    let mut uf = UnionFind::new(n_persons + n_marriages);

    // Helper to address marriage slots in the union-find array.
    let m_idx = |i: usize| n_persons + i;

    // Each marriage's bar-anchor (if any) links it to the host's
    // canonical-family marriage — that's where the bar nests.
    for (i, m) in index.graph.marriages.iter().enumerate() {
        if let Some(anchor) = index.bar_anchor(m)
            && let Some(&j) = index.marriages_by_id.get(anchor.as_str())
        {
            uf.union(m_idx(i), m_idx(j));
        }
        // The absorb rule: the joining spouse's birth family nests at
        // this marriage's connection point — so the birth family belongs
        // to the same component as the host. The cousin / sibling
        // marriage case is just this union being a no-op because both
        // ends already share a root.
        if let Some(joining) = index.person(&m.spouses[1])
            && let Some(family_id) = joining.canonical_family()
            && let Some(&j) = index.marriages_by_id.get(family_id)
        {
            uf.union(m_idx(i), m_idx(j));
        }
    }

    // Each person unifies with every un-ended marriage they
    // participate in (host or join). For pure-host polygamy this
    // collapses N concurrent bars into one component anchored at the
    // polygamous person — the structural foundation for one canonical
    // card hosting N bars (ADR-0017).
    // Past-ended marriages still unify through `bar_anchor` /
    // `canonical_location` below; only the *current intimacy* unions
    // are added here.
    for (i, facts) in index.persons.iter().enumerate() {
        for &m in facts
            .hosted_marriages
            .iter()
            .chain(facts.joined_marriages.iter())
        {
            if index.graph.marriages[m].end.is_none() {
                uf.union(i, m_idx(m));
            }
        }
    }

    // Each person's canonical-card anchor unifies them with the
    // marriage they anchor to.
    for (i, facts) in index.persons.iter().enumerate() {
        match index.canonical_location(facts) {
            CanonicalLocation::ChildOf(mid)
            | CanonicalLocation::JoiningOf(mid)
            | CanonicalLocation::HostOfFloating(mid) => {
                if let Some(&j) = index.marriages_by_id.get(mid.as_str()) {
                    uf.union(i, m_idx(j));
                }
            }
            CanonicalLocation::Orphan => {}
        }
    }

    // Group marriages and persons by union-find root.
    let mut groups: BTreeMap<usize, ComponentMembers> = BTreeMap::new();
    for i in 0..n_persons {
        let root = uf.find(i);
        groups.entry(root).or_default().persons.push(i);
    }
    for i in 0..n_marriages {
        let root = uf.find(m_idx(i));
        groups.entry(root).or_default().marriages.push(i);
    }

    // Build a Component per group, in source order.
    let mut components: Vec<Component> = groups
        .into_values()
        .map(|members| build_one_component(index, &members))
        .collect();
    components.sort_by_key(|c| c.source_order);
    for (idx, comp) in components.iter_mut().enumerate() {
        comp.id = format!("comp-{}", idx + 1);
    }
    components
}

#[derive(Default)]
struct ComponentMembers {
    /// Person indices in `index.persons` belonging to this component.
    persons: Vec<usize>,
    /// Marriage indices in `index.graph.marriages` belonging to this
    /// component.
    marriages: Vec<usize>,
}

fn build_one_component(index: &Index<'_>, members: &ComponentMembers) -> Component {
    // Determine the component's first-relevant-declaration source
    // position per source order: the earliest-declared marriage if any,
    // otherwise the earliest-declared person.
    let earliest_marriage = members
        .marriages
        .iter()
        .filter_map(|&i| index.graph.marriages[i].span.as_ref().map(|s| (s[0], i)))
        .min();
    let earliest_person = members
        .persons
        .iter()
        .filter_map(|&i| index.persons[i].person.span.as_ref().map(|s| (s[0], i)))
        .min();
    let source_order = earliest_marriage
        .map(|(byte, _)| byte)
        .or_else(|| earliest_person.map(|(byte, _)| byte))
        .unwrap_or(0);

    // Two shapes per absence, not placeholders and current-intimacy
    // placement: a family tree (rooted by a PersonCard —
    // canonical or, for the past-ended floating-bar fallback, ghost) or
    // an orphan person (lone card).
    if members.marriages.is_empty() {
        // Lone orphan card. By construction there's exactly one person
        // here — an unanchored person becomes its own component.
        let &person_idx = members
            .persons
            .first()
            .expect("union-find produced an empty component");
        let facts = &index.persons[person_idx];
        let card = Box::new(canonical_card_slot(facts));
        return Component {
            id: String::new(), // assigned by the caller after sorting
            source_order,
            kind: ComponentKind::OrphanPerson { card },
        };
    }

    // Find the visible root of the component. The root is a
    // `PersonCard`: the outermost canonical host of the outermost
    // floating marriage in the component. "Outermost floating" means:
    // (a) no bar-anchor (floating mini-comp) and (b) not the
    // canonical-family of any joining spouse in this component — the
    // absorb rule doesn't nest *it* inside another marriage's
    // joining-slot.
    //
    // When multiple "true roots" exist (e.g. polygamy where a person
    // floats N bars), the union-find above has already collapsed them
    // into one component; we pick the earliest-declared root marriage,
    // which is the polygamous host's primary (first-declared un-ended)
    // anchor per ADR-0017.
    let nested_targets: HashSet<String> = members
        .marriages
        .iter()
        .filter_map(|&i| {
            index
                .person(&index.graph.marriages[i].spouses[1])
                .and_then(|p| p.canonical_family().map(|f| f.to_string()))
        })
        .collect();
    let root_marriage_idx = members
        .marriages
        .iter()
        .copied()
        .filter(|&i| {
            let m = &index.graph.marriages[i];
            index.bar_anchor(m).is_none() && !nested_targets.contains(m.id.as_str())
        })
        .min_by_key(|&i| {
            index.graph.marriages[i]
                .span
                .as_ref()
                .map(|s| s[0])
                .unwrap_or(usize::MAX)
        })
        .or_else(|| {
            // Fallback: no marriage qualifies as "true root" (shouldn't
            // happen in a well-formed acyclic graph, but be defensive).
            // Pick any floating-mini-comp marriage.
            members
                .marriages
                .iter()
                .copied()
                .filter(|&i| index.bar_anchor(&index.graph.marriages[i]).is_none())
                .min_by_key(|&i| {
                    index.graph.marriages[i]
                        .span
                        .as_ref()
                        .map(|s| s[0])
                        .unwrap_or(usize::MAX)
                })
        })
        .expect("non-orphan component must contain at least one floating marriage");

    // The "rendering context" of the absorb rule: the set of marriages
    // that will be visited by the main tree-walk (root → children →
    // hosted_marriages → recursion). Computed up-front so the
    // recursive build can ask, when considering a joining spouse's
    // birth family for nesting, whether that birth family is
    // already going to be rendered as part of the main walk. If so —
    // cousin / sibling marriage — terminate the nesting; if not,
    // pull the birth family in as a nested sub-tree at the joining
    // spouse's connection point (a true nested sub-tree).
    let root_host_id = index.graph.marriages[root_marriage_idx].spouses[0].as_str();
    let in_context = compute_main_walk_reachable(index, root_host_id, root_marriage_idx);

    let mut visited = HashSet::new();
    let root = Box::new(build_person_root(
        index,
        root_marriage_idx,
        &mut visited,
        &in_context,
    ));

    Component {
        id: String::new(),
        source_order,
        kind: ComponentKind::FamilyTree { root },
    }
}

/// Build the root `PersonCard` of a `FamilyTree` component.
///
/// The root is the outermost canonical host of the component. Three
/// shapes:
///
/// - The host's canonical card is `ChildOf(root_marriage)` — they
///   haven't moved on (no newer current intimacy) and the floating
///   bar is their first-declared un-ended participation. Root
///   PersonCard is canonical, with `hosted_marriages` carrying the
///   floating un-ended bars in declaration order.
/// - The host's canonical card is `HostOfFloating(root_marriage)` —
///   the host has no birth family; same canonical-rooted shape.
/// - The host has moved on (canonical lives elsewhere) and the
///   floating bar is past-ended — there is no canonical host, so the
///   root PersonCard is a ghost (`SlotKind::Ghost { reason:
///   PastMarriage }`) rooted at the declared host. Carries the
///   past-ended bar in `hosted_marriages` as a single entry so the
///   child edges still anchor (Q12 A.1 from #142's grilling notes).
fn build_person_root(
    index: &Index<'_>,
    root_marriage_idx: usize,
    visited: &mut HashSet<usize>,
    in_context: &HashSet<usize>,
) -> PersonCard {
    let host_id = index.graph.marriages[root_marriage_idx].spouses[0].as_str();
    let host_facts = index
        .person(host_id)
        .expect("root marriage's host must be a declared person");

    // Decide whether the root PersonCard is canonical or ghost. The
    // root marriage has no bar-anchor (it's a floating mini-comp), so
    // the host's canonical card either sits at this floating bar
    // (canonical root) or lives elsewhere — in which case the bar is
    // past-ended (the host moved on), and the root is a ghost.
    let host_is_canonical_here = matches!(
        index.canonical_location(host_facts),
        CanonicalLocation::HostOfFloating(ref m) if m == &index.graph.marriages[root_marriage_idx].id,
    );

    let slot = if host_is_canonical_here {
        canonical_card_slot(host_facts)
    } else {
        // Ghost-rooted: the PersonCard sits on the same generation row
        // as the bar it hosts (not one row below — that's the
        // child-ghost rule in `ghost_card_slot`). The bar is the
        // host's slot in their absent birth family, and the ghost
        // takes the host's spot at that slot.
        let bar_gen = bar_generation(index, &index.graph.marriages[root_marriage_idx]);
        card_slot(
            host_facts,
            SlotKind::Ghost {
                reason: GhostReason::PastMarriage,
            },
            bar_gen,
        )
    };

    let hosted_marriages = if host_is_canonical_here {
        // All of the host's un-ended hosted marriages — per
        // current-intimacy placement, polygamy collapses onto one
        // canonical card. The root
        // marriage is the first-declared un-ended; the rest are
        // additional concurrent intimacies in declaration order.
        // `build_hosted_marriages` already visits in declaration order
        // and skips ones already visited.
        build_hosted_marriages(index, host_facts, visited, in_context)
    } else {
        // Ghost-rooted: only the past-ended root bar surfaces here.
        // The host's other (canonical) participations live in their
        // own component anchored at the canonical card.
        vec![build_marriage_branch(
            index,
            root_marriage_idx,
            visited,
            in_context,
        )]
    };

    PersonCard {
        slot,
        hosted_marriages,
    }
}

/// Precompute the set of marriage indices the main tree-walk would
/// visit from the root PersonCard: the root host's hosted marriages,
/// each canonical child's hosted marriages, and recursively. Used as
/// the absorb rule's termination set for `build_nested_birth_family` — a
/// joining spouse's birth family that's already in this set is in the
/// current rendering context (cousin / sibling marriage) and doesn't
/// nest; one that isn't gets pulled in as a nested sub-tree (a true
/// nested sub-tree).
fn compute_main_walk_reachable(
    index: &Index<'_>,
    root_host_id: &str,
    root_marriage_idx: usize,
) -> HashSet<usize> {
    let mut reachable: HashSet<usize> = HashSet::new();
    let mut stack: Vec<usize> = Vec::new();

    // Seed with the root host's hosted marriages (canonical-rooted
    // path), or the root marriage itself (ghost-rooted path: the
    // host's canonical lives elsewhere so their other hosted
    // marriages don't belong to this component's walk).
    if let Some(host_facts) = index.person(root_host_id)
        && matches!(
            index.canonical_location(host_facts),
            CanonicalLocation::HostOfFloating(ref m) if m == &index.graph.marriages[root_marriage_idx].id,
        )
    {
        for &h in &host_facts.hosted_marriages {
            stack.push(h);
        }
    } else {
        stack.push(root_marriage_idx);
    }

    while let Some(m_idx) = stack.pop() {
        if !reachable.insert(m_idx) {
            continue;
        }
        let marriage_id = index.graph.marriages[m_idx].id.as_str();
        for facts in index.persons.iter() {
            if facts.canonical_family() != Some(marriage_id) {
                continue;
            }
            // Only canonical children (those whose canonical card
            // actually sits in this children row) contribute their
            // hosted marriages — a person who moved on to a host
            // elsewhere doesn't surface their hosted marriages here.
            if !matches!(
                index.canonical_location(facts),
                CanonicalLocation::ChildOf(ref id) if id == marriage_id
            ) {
                continue;
            }
            for &h in &facts.hosted_marriages {
                stack.push(h);
            }
        }
    }
    reachable
}

/// Recursively build a `MarriageBranch` rooted at the given marriage.
///
/// `visited` is the set of marriage indices already emitted in this
/// component's traversal. `in_context` is the precomputed set of
/// marriages reachable from the component's root via the main
/// tree-walk; it terminates the absorb rule's nesting when the joining
/// spouse's birth family would already be rendered via the main walk
/// (cousin / sibling marriage).
fn build_marriage_branch(
    index: &Index<'_>,
    marriage_idx: usize,
    visited: &mut HashSet<usize>,
    in_context: &HashSet<usize>,
) -> MarriageBranch {
    visited.insert(marriage_idx);
    let marriage = &index.graph.marriages[marriage_idx];
    let host_id = &marriage.spouses[0];
    let joining_id = &marriage.spouses[1];

    let joining_slot = bar_joining_slot(index, joining_id, marriage);

    let bar = MarriageBar {
        marriage_id: marriage.id.clone(),
        generation: bar_generation(index, marriage),
        host_id: host_id.clone(),
        joining_id: joining_id.clone(),
        joining_slot,
        start: marriage.start.clone(),
        end: marriage.end.clone(),
        end_reason: marriage.end_reason.clone(),
        ended: marriage.end.is_some(),
        joining_nested_birth_family: build_nested_birth_family(
            index, joining_id, visited, in_context,
        ),
    };

    // Children of this marriage, in source order, grouped per canonical
    // family ownership. Per current-intimacy placement, an adopted
    // child with a canonical-adoption sitting at this marriage renders
    // here, even if their bio family is elsewhere.
    let children = build_children(index, &marriage.id, visited, in_context);

    MarriageBranch { bar, children }
}

/// The absorb rule's nested birth-family sub-tree for the joining
/// spouse. `None` when the joining spouse has no birth family declared,
/// or when the recursion would re-enter the current rendering context
/// (cousin / sibling marriage — the birth family is already a sibling
/// structure in this component, reachable through the main walk).
///
/// Shaped exactly like a top-level [`ComponentKind::FamilyTree`] —
/// the returned `PersonCard` is the outermost canonical host of the
/// nested family, with the birth-family bar in its `hosted_marriages`.
fn build_nested_birth_family(
    index: &Index<'_>,
    joining_id: &str,
    visited: &mut HashSet<usize>,
    in_context: &HashSet<usize>,
) -> Option<Box<PersonCard>> {
    let facts = index.person(joining_id)?;
    let family_id = facts.canonical_family()?;
    let &family_idx = index.marriages_by_id.get(family_id)?;
    // The absorb rule (within-family): the joining spouse's birth
    // family is already in this component (cousin / sibling marriage).
    // Don't nest — the main walk will reach it.
    if in_context.contains(&family_idx) {
        return None;
    }
    // The absorb rule (across families): cross-component join — pull
    // the birth family in as a nested sub-tree at the connection point.
    if visited.contains(&family_idx) {
        return None;
    }
    Some(Box::new(build_person_root(
        index, family_idx, visited, in_context,
    )))
}

fn build_children(
    index: &Index<'_>,
    marriage_id: &str,
    visited: &mut HashSet<usize>,
    in_context: &HashSet<usize>,
) -> Vec<PersonCard> {
    let mut out = Vec::new();
    // The absorb rule / source-order semantic: iterate persons in
    // declaration order so canonical children and past child-ghosts interleave at
    // the same source-order key the children row uses for canonical
    // siblings. One person can play at most one role at any given
    // marriage (canonical child, past-adoption ghost, or past-bio
    // ghost), so the per-person branches are mutually exclusive.
    for facts in index.persons.iter() {
        // Canonical child: the child's canonical card sits in this
        // marriage's children row. A child who joined some current
        // intimacy elsewhere — or whose canonical adoption is at a
        // different marriage — doesn't surface canonically here.
        let canonical_here = facts.canonical_family() == Some(marriage_id)
            && matches!(
                index.canonical_location(facts),
                CanonicalLocation::ChildOf(ref id) if id == marriage_id
            );
        if canonical_here {
            out.push(PersonCard {
                slot: canonical_card_slot(facts),
                hosted_marriages: build_hosted_marriages(index, facts, visited, in_context),
            });
            continue;
        }
        // Past-adoption child-ghost: this marriage is one of the
        // person's adoption marriages, but the "most-recent wins" rule
        // resolved the canonical card elsewhere.
        if facts.adoption_marriages.len() >= 2
            && facts
                .adoption_marriages
                .iter()
                .skip(1)
                .any(|m| m == marriage_id)
        {
            out.push(PersonCard {
                slot: ghost_card_slot(facts, GhostReason::PastAdoption, marriage_id, index),
                hosted_marriages: Vec::new(),
            });
            continue;
        }
        // Past-bio child-ghost: derived-from-canonical trigger —
        // the person has a `birth` link at this marriage AND the
        // current-intimacy chain selected a different intimacy (adoption demotes the
        // bio family; a marriage's joining slot likewise relocates
        // the canonical card). Emit a ghost so the solid bio-birth
        // edge terminates locally at the bio family rather than
        // traversing the canvas to the canonical card. `canonical_here`
        // above already short-circuited the case where the bio family
        // is the canonical anchor.
        if facts.bio_marriage.as_deref() == Some(marriage_id) {
            out.push(PersonCard {
                slot: ghost_card_slot(facts, GhostReason::PastBirth, marriage_id, index),
                hosted_marriages: Vec::new(),
            });
        }
    }
    out
}

fn build_hosted_marriages(
    index: &Index<'_>,
    host: &PersonFacts<'_>,
    visited: &mut HashSet<usize>,
    in_context: &HashSet<usize>,
) -> Vec<MarriageBranch> {
    let mut out = Vec::new();
    for &m in &host.hosted_marriages {
        if visited.contains(&m) {
            continue;
        }
        // Only attach bars that belong to this PersonCard's component.
        // A past-ended bar whose host has moved on lives in its own
        // ghost-rooted component (Q12 A.1); it must not duplicate
        // under the host's canonical PersonCard.
        if !host_anchors_bar_here(index, host, m) {
            continue;
        }
        out.push(build_marriage_branch(index, m, visited, in_context));
    }
    out
}

/// True iff this canonical PersonCard for `host` should carry the
/// given bar as a `hosted_marriage`.
///
/// - An un-ended bar always anchors at the host's canonical card,
///   regardless of primary-vs-secondary in the polygamy cluster
///   (one canonical card per person, current-intimacy placement,
///   ADR-0017).
/// - A past-ended bar anchors here only if the host has *not* moved
///   on (no newer current intimacy) and the bar's canonical location
///   matches this PersonCard's slot — the host-slot canonicality
///   condition.
fn host_anchors_bar_here(index: &Index<'_>, host: &PersonFacts<'_>, marriage_idx: usize) -> bool {
    let marriage = &index.graph.marriages[marriage_idx];
    let location = index.canonical_location(host);

    if marriage.end.is_none() {
        // Un-ended bar: anchors at the canonical card iff that card
        // is the polygamy hub for this host's current intimacies.
        // The hub is `ChildOf(canonical_family)` for a host with a
        // birth family or `HostOfFloating(primary)` for a host
        // without one — i.e. anywhere except `JoiningOf` (where the
        // host is the *joining* spouse, not the host) or `Orphan`.
        return matches!(
            location,
            CanonicalLocation::ChildOf(_) | CanonicalLocation::HostOfFloating(_),
        );
    }

    // Past-ended bar: the host-slot canonical check.
    let bar_anchor = index.bar_anchor(marriage);
    match location {
        CanonicalLocation::ChildOf(ref family) => bar_anchor.as_deref() == Some(family.as_str()),
        CanonicalLocation::HostOfFloating(ref m) => m == &marriage.id,
        _ => false,
    }
}

/// Build the joining spouse's `CardSlot` for a marriage bar.
///
/// Per current-intimacy placement, the joining slot is canonical iff the joining spouse's
/// canonical card physically lives at this bar — i.e.
/// `canonical_location` resolves to `JoiningOf(this marriage)`. Any
/// other case is a past-marriage ghost (the canonical card is
/// somewhere else).
fn bar_joining_slot(index: &Index<'_>, joining_id: &str, marriage: &ExportedMarriage) -> CardSlot {
    let facts = index
        .person(joining_id)
        .expect("spouse must be a declared person");
    let canonical = matches!(
        index.canonical_location(facts),
        CanonicalLocation::JoiningOf(ref m) if m == &marriage.id,
    );
    let kind = if canonical {
        SlotKind::Canonical
    } else {
        SlotKind::Ghost {
            reason: GhostReason::PastMarriage,
        }
    };
    let generation = bar_generation(index, marriage);
    card_slot(facts, kind, generation)
}

fn canonical_card_slot(facts: &PersonFacts<'_>) -> CardSlot {
    card_slot(facts, SlotKind::Canonical, facts.generation)
}

fn ghost_card_slot(
    facts: &PersonFacts<'_>,
    reason: GhostReason,
    marriage_id: &str,
    index: &Index<'_>,
) -> CardSlot {
    let generation = index
        .marriage(marriage_id)
        .map(|m| bar_generation(index, m) + 1)
        .unwrap_or(facts.generation);
    card_slot(facts, SlotKind::Ghost { reason }, generation)
}

fn card_slot(facts: &PersonFacts<'_>, kind: SlotKind, generation: u32) -> CardSlot {
    let p = facts.person;
    CardSlot {
        person_id: p.id.clone(),
        kind,
        generation,
        name: p.name.clone(),
        gender: p.gender,
        family: p.family.clone(),
        given: p.given.clone(),
        born: p.born.clone(),
        died: p.died.clone(),
    }
}

/// The marriage bar's row index: max of the spouses' canonical
/// generations. The classical descendency tree's "older above, younger below" baseline.
fn bar_generation(index: &Index<'_>, marriage: &ExportedMarriage) -> u32 {
    marriage
        .spouses
        .iter()
        .filter_map(|s| index.person(s).map(|p| p.generation))
        .max()
        .unwrap_or(0)
}

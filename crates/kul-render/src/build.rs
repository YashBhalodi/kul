//! Kinship-native → [`RenderShape`] projection. Normative pattern in
//! [`docs/canonical-ui-pattern.md`](../../docs/canonical-ui-pattern.md).

use std::collections::{BTreeMap, HashMap, HashSet};

use kul_core::export::{
    ExportedDate, ExportedGraph, ExportedMarriage, ExportedParenthoodLink, ExportedPerson,
    ParenthoodLinkKind,
};

use crate::shape::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind, GhostReason, MarriageBar, MarriageBranch,
    PersonCard, SlotKind,
};

/// Entry point for [`crate::transform`]. Returns `(components, edges)` in
/// source order by each component's first-relevant-declaration position.
pub(crate) fn build(graph: &ExportedGraph) -> (Vec<Component>, Vec<Edge>) {
    let index = Index::new(graph);
    let edges = build_edges(graph);
    let components = build_components(&index);
    (components, edges)
}

/// Mirrors export's parenthood links one-to-one in declaration order so
/// downstream consumers can correlate by index.
fn build_edges(graph: &ExportedGraph) -> Vec<Edge> {
    graph
        .parenthood_links
        .iter()
        .map(|link| Edge {
            kind: match link.kind {
                ParenthoodLinkKind::Biological => EdgeKind::Birth,
                ParenthoodLinkKind::Adoptive => EdgeKind::Adoption,
            },
            child_id: link.child_id.clone(),
            marriage_id: link.marriage_id.clone(),
            start: link.start.clone(),
            end: link.end.clone(),
        })
        .collect()
}

/// Per-person derived facts, precomputed so the tree-walk is one linear pass.
#[derive(Debug)]
struct PersonFacts<'a> {
    person: &'a ExportedPerson,
    hosted_marriages: Vec<usize>,
    joined_marriages: Vec<usize>,
    bio_marriage: Option<String>,
    /// Sorted by `start:` desc with declaration-order tiebreak (most-recent
    /// at index 0); the canonical adoption.
    adoption_marriages: Vec<String>,
    /// First-declared un-ended participation across hosted ∪ joined —
    /// determines current intimacy per ADR-0017.
    primary_marriage: Option<usize>,
    /// Generation under the canonical-family graph: roots at 0,
    /// child = max(canonical-family spouses) + 1.
    generation: u32,
}

impl<'a> PersonFacts<'a> {
    fn canonical_family(&self) -> Option<&str> {
        self.canonical_adoption().or(self.bio_marriage.as_deref())
    }

    fn canonical_adoption(&self) -> Option<&str> {
        self.adoption_marriages.first().map(String::as_str)
    }
}

/// Precomputed indices over the input graph for one transformation.
struct Index<'a> {
    graph: &'a ExportedGraph,
    persons_by_id: HashMap<&'a str, usize>,
    marriages_by_id: HashMap<&'a str, usize>,
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
        let adoption_links: HashMap<(&'a str, &'a str), &'a ExportedParenthoodLink> = graph
            .parenthood_links
            .iter()
            .filter(|l| l.kind == ParenthoodLinkKind::Adoptive)
            .map(|l| ((l.child_id.as_str(), l.marriage_id.as_str()), l))
            .collect();

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
                ParenthoodLinkKind::Biological => {
                    persons[child].bio_marriage = Some(link.marriage_id.clone());
                }
                ParenthoodLinkKind::Adoptive => persons[child]
                    .adoption_marriages
                    .push(link.marriage_id.clone()),
            }
        }

        // Sort by `start:` desc, declaration-order tiebreak — most-recent
        // at index 0 so `canonical_adoption()` is a one-line lookup.
        for facts in persons.iter_mut() {
            let person: &ExportedPerson = facts.person;
            let person_id = person.id.as_str();
            facts.adoption_marriages.sort_by(|a, b| {
                let key_a = adoption_sort_key(&adoption_links, person_id, a);
                let key_b = adoption_sort_key(&adoption_links, person_id, b);
                key_b.cmp(&key_a)
            });
        }

        // Primary marriage: first-declared un-ended participation across
        // hosted ∪ joined (ADR-0017). Marriage indices grow with source
        // position, so min-index = first-declared.
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

    /// Relax `child = max(canonical-family spouses) + 1` to fixpoint.
    /// Export is acyclic (R13), converges in ≤ `persons.len()` iterations.
    fn compute_generations(&mut self) {
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

    /// The host's canonical-family marriage (where this bar nests), or
    /// `None` for a floating mini-component.
    fn bar_anchor(&self, marriage: &ExportedMarriage) -> Option<String> {
        let host = self.person(&marriage.spouses[0])?;
        host.canonical_family().map(str::to_string)
    }

    /// Where this person's canonical card anchors (current-intimacy placement).
    fn canonical_location(&self, facts: &PersonFacts<'_>) -> CanonicalLocation {
        if let Some(primary_idx) = facts.primary_marriage {
            let primary = &self.graph.marriages[primary_idx];
            let host_id = &primary.spouses[0];
            if host_id == &facts.person.id {
                match facts.canonical_family() {
                    Some(family) => CanonicalLocation::ChildOf(family.to_string()),
                    None => CanonicalLocation::HostOfFloating(primary.id.clone()),
                }
            } else {
                CanonicalLocation::JoiningOf(primary.id.clone())
            }
        } else if let Some(family) = facts.canonical_family() {
            CanonicalLocation::ChildOf(family.to_string())
        } else {
            CanonicalLocation::Orphan
        }
    }
}

fn adoption_sort_key(
    links: &HashMap<(&str, &str), &ExportedParenthoodLink>,
    person_id: &str,
    marriage_id: &str,
) -> SortableDate {
    links
        .get(&(person_id, marriage_id))
        .and_then(|l| l.start.as_ref())
        .map(SortableDate::from)
        .unwrap_or(SortableDate::missing())
}

/// Comparable date for "most recent" picks. Missing dates sort earliest
/// (a missing `start:` is less specific than any real date).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SortableDate(i64);

impl SortableDate {
    fn missing() -> Self {
        Self(i64::MIN)
    }
}

impl From<&ExportedDate> for SortableDate {
    fn from(d: &ExportedDate) -> Self {
        // `YYYY[-MM[-DD]]` encoded as `YYYY*10000 + MM*100 + DD` so
        // numeric order matches calendar order across precisions.
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
    /// Children row of this marriage.
    ChildOf(String),
    /// Joining slot of this marriage's bar.
    JoiningOf(String),
    /// Host slot — only when the host has no canonical family.
    HostOfFloating(String),
    /// No anchor — lone orphan or joining-of-ended-with-no-birth-family.
    Orphan,
}

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

fn build_components(index: &Index<'_>) -> Vec<Component> {
    let n_persons = index.persons.len();
    let n_marriages = index.graph.marriages.len();
    let mut uf = UnionFind::new(n_persons + n_marriages);

    let m_idx = |i: usize| n_persons + i;

    for (i, m) in index.graph.marriages.iter().enumerate() {
        if let Some(anchor) = index.bar_anchor(m)
            && let Some(&j) = index.marriages_by_id.get(anchor.as_str())
        {
            uf.union(m_idx(i), m_idx(j));
        }
        // Absorb rule: joining spouse's birth family belongs to the same
        // component as the host. Cousin/sibling case is a no-op union.
        if let Some(joining) = index.person(&m.spouses[1])
            && let Some(family_id) = joining.canonical_family()
            && let Some(&j) = index.marriages_by_id.get(family_id)
        {
            uf.union(m_idx(i), m_idx(j));
        }
    }

    // Pure-host polygamy collapses N concurrent bars into one component
    // (ADR-0017). Past-ended unions come through `bar_anchor` below.
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

    let mut groups: BTreeMap<usize, ComponentMembers> = BTreeMap::new();
    for i in 0..n_persons {
        let root = uf.find(i);
        groups.entry(root).or_default().persons.push(i);
    }
    for i in 0..n_marriages {
        let root = uf.find(m_idx(i));
        groups.entry(root).or_default().marriages.push(i);
    }

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
    persons: Vec<usize>,
    marriages: Vec<usize>,
}

fn build_one_component(index: &Index<'_>, members: &ComponentMembers) -> Component {
    // Source-order anchor: earliest marriage, else earliest person.
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

    if members.marriages.is_empty() {
        let &person_idx = members
            .persons
            .first()
            .expect("union-find produced an empty component");
        let facts = &index.persons[person_idx];
        let card = Box::new(canonical_card_slot(facts));
        return Component {
            id: String::new(),
            source_order,
            kind: ComponentKind::OrphanPerson { card },
        };
    }

    // Root = outermost canonical host of the outermost floating marriage:
    // (a) no bar-anchor, (b) not nested inside any joining spouse's
    // birth-family slot. Polygamy's multiple roots collapse via union-find;
    // pick the earliest-declared (primary, per ADR-0017).
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
            // Defensive: pick any floating-mini-comp if no clean root.
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

    // Precompute the main walk's reachable set so the absorb rule's
    // recursion can terminate on cousin/sibling marriages (the birth
    // family is already in this component) vs. nest cross-component.
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

/// Build the root `PersonCard` of a `FamilyTree`. Either a canonical
/// host (carrying their un-ended hosted bars) or — when the host has
/// moved on past this past-ended bar — a `PastMarriage` ghost rooted
/// at the declared host, carrying the past-ended bar alone.
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

    // Root canonical iff the host's canonical card sits at this floating
    // bar; otherwise the host has moved on and the root is a ghost.
    let host_is_canonical_here = matches!(
        index.canonical_location(host_facts),
        CanonicalLocation::HostOfFloating(ref m) if m == &index.graph.marriages[root_marriage_idx].id,
    );

    let slot = if host_is_canonical_here {
        canonical_card_slot(host_facts)
    } else {
        // Ghost-rooted: sits on the bar's row (not one below — that's
        // the child-ghost rule), taking the host's spot at the bar.
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
        // Polygamy collapses onto one canonical card (ADR-0017).
        build_hosted_marriages(index, host_facts, visited, in_context)
    } else {
        // Ghost-rooted: only the past-ended root bar surfaces here.
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

/// Marriages the main tree-walk will visit from the root. Used to
/// terminate the absorb rule's recursion when a joining spouse's
/// birth family is already in this rendering context (cousin/sibling).
fn compute_main_walk_reachable(
    index: &Index<'_>,
    root_host_id: &str,
    root_marriage_idx: usize,
) -> HashSet<usize> {
    let mut reachable: HashSet<usize> = HashSet::new();
    let mut stack: Vec<usize> = Vec::new();

    // Canonical-rooted: seed with host's hosted marriages. Ghost-rooted:
    // host's canonical is elsewhere, so seed with the root marriage only.
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
            // Only canonical-here children contribute; one who moved on
            // to host elsewhere surfaces their hosted marriages there.
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

/// Recursively build a `MarriageBranch`. `in_context` terminates the
/// absorb rule's nesting on cousin/sibling marriages.
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

    let children = build_children(index, &marriage.id, visited, in_context);

    MarriageBranch { bar, children }
}

/// The absorb rule's nested birth-family sub-tree. `None` when the
/// joining spouse has no birth family or it's already in this rendering
/// context (cousin/sibling). Shaped like a top-level `FamilyTree`.
fn build_nested_birth_family(
    index: &Index<'_>,
    joining_id: &str,
    visited: &mut HashSet<usize>,
    in_context: &HashSet<usize>,
) -> Option<Box<PersonCard>> {
    let facts = index.person(joining_id)?;
    let family_id = facts.canonical_family()?;
    let &family_idx = index.marriages_by_id.get(family_id)?;
    if in_context.contains(&family_idx) || visited.contains(&family_idx) {
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
    // Declaration order so canonical children and past ghosts interleave.
    // Per-person roles (canonical / past-adoption / past-bio) are mutually
    // exclusive at any given marriage.
    for facts in index.persons.iter() {
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
        // Past-adoption child-ghost at a non-canonical adoption marriage.
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
        // Past-bio child-ghost: bio link exists but current-intimacy
        // chose elsewhere (adoption or moved-on join). Emits so the
        // solid bio edge terminates locally instead of crossing canvas.
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
        // Past-ended bars whose host moved on live in their own
        // ghost-rooted component — don't duplicate here.
        if !host_anchors_bar_here(index, host, m) {
            continue;
        }
        out.push(build_marriage_branch(index, m, visited, in_context));
    }
    out
}

/// True iff this canonical card for `host` should carry the bar.
/// Un-ended: always anchors at the host's canonical hub (ADR-0017).
/// Past-ended: only when host has not moved on and the bar's canonical
/// location matches this card's slot.
fn host_anchors_bar_here(index: &Index<'_>, host: &PersonFacts<'_>, marriage_idx: usize) -> bool {
    let marriage = &index.graph.marriages[marriage_idx];
    let location = index.canonical_location(host);

    if marriage.end.is_none() {
        return matches!(
            location,
            CanonicalLocation::ChildOf(_) | CanonicalLocation::HostOfFloating(_),
        );
    }

    let bar_anchor = index.bar_anchor(marriage);
    match location {
        CanonicalLocation::ChildOf(ref family) => bar_anchor.as_deref() == Some(family.as_str()),
        CanonicalLocation::HostOfFloating(ref m) => m == &marriage.id,
        _ => false,
    }
}

/// Joining-slot for a bar. Canonical iff `canonical_location` resolves
/// to `JoiningOf(this marriage)`; otherwise a `PastMarriage` ghost.
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

/// Bar row index: max of the spouses' generations.
fn bar_generation(index: &Index<'_>, marriage: &ExportedMarriage) -> u32 {
    marriage
        .spouses
        .iter()
        .filter_map(|s| index.person(s).map(|p| p.generation))
        .max()
        .unwrap_or(0)
}

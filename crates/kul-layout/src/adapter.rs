//! Canonical UI pattern adapter — wraps [`crate::walker`] for kul's
//! pattern primitives (marriage bars between adjacent spouses, ghost
//! slots at host's birth-family position per P8, generation rows from
//! generation indices, orthogonal right-angle edge routing).
//!
//! The adapter consumes a [`kul_render::SuccessRender`] and builds an
//! internal layout tree, runs Walker's over it, then projects the
//! resulting positions back into a [`crate::PositionedShape`].
//!
//! ## Polygamy hubs (ADR-0027)
//!
//! When a person hosts ≥2 concurrent marriages, the adapter rearranges
//! the cluster into a **fan**: the hub card sits at row R alone; each
//! co-spouse sits as a walker child of the hub on the next row down
//! (R+1) and is reached by a thick [`EdgeKind::Marriage`] edge —
//! routed with the same orthogonal hub-bottom → bus → spouse-top
//! geometry as a birth edge, only with a heavier stroke. The polygamy
//! marriage emits no [`PositionedBar`] (the edge replaces the bar's
//! visual role of "this couple is married"); children of the marriage
//! anchor at the co-spouse card's bottom-midpoint instead of at a bar.
//! Monogamy (`hosted_marriages.len() == 1`) keeps the classical hub-
//! and-flanks shape with one bar between adjacent spouse cards.

use kul_render::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind as RenderEdgeKind, GhostReason, MarriageBar,
    PersonCard, SlotKind as RenderSlotKind, SuccessRender,
};

use crate::metrics::LayoutConfig;
use crate::shape::{
    EdgeKind, EdgeRouting, PositionedBar, PositionedCard, PositionedEdge, PositionedShape, SlotKind,
};
use crate::walker::{self, InputNode};

/// Main entry — see [`crate::layout`].
pub(crate) fn lay_out(success: &SuccessRender, config: &LayoutConfig) -> PositionedShape {
    let mut builder = Builder::new(config);
    for component in &success.components {
        builder.add_component(component);
    }
    builder.finish(&success.edges)
}

/// A virtual layout node Walker positions. Each `Node` is one cluster:
/// either a single card, a card-bar-card host cluster (monogamy), the
/// hub of a polygamy fan, or a co-spouse underneath a hub.
struct Node {
    /// Anchor type: what visual primitive this cluster is.
    kind: NodeKind,
    /// Horizontal extent of the cluster.
    width: f64,
    /// Surface layout row (0.0 = top). Computed bottom-up per
    /// ADR-0023, refined by ADR-0024. Carried as `f64` so future
    /// fractional-row primitives can flow through the same cascade
    /// without re-widening; in v1 every cluster lands on an integer
    /// row.
    ///
    /// ```text
    /// visual_row(cluster) = max(
    ///     host_card.slot.generation,
    ///     1.0 + max(visual_row(nested)) for nesting marriages,
    ///     min(visual_row(child)) - 1.0,
    /// )
    /// ```
    ///
    /// The nesting clause pushes a host *down* to make room for any
    /// P6 (grand-)nested sub-tree. The descendant-pull clause pulls a
    /// host *down* to sit one row above its closest descendant, so
    /// kin-symmetric ancestors across an inter-family marriage align
    /// on the same visual row. For the polygamy fan the co-spouse
    /// clusters are walker children of the hub at row `hub + 1.0`,
    /// so the same cascade governs them with no extra arithmetic.
    /// For leaves, orphans, and hosts whose descendants haven't been
    /// pushed below their data-level row by any nesting upstream,
    /// both extra clauses collapse to `host_card.slot.generation`.
    visual_row: f64,
    /// Children clusters (in declaration order).
    children: Vec<usize>,
}

enum NodeKind {
    /// A monogamy person host: card + bar + joining card in one
    /// cluster on a single row. Covers the
    /// `hosted_marriages.len() == 1` case at any depth (root or
    /// child). Children are the union of all hosted marriages'
    /// children, in declaration order.
    PersonHost {
        card: Box<PersonCard>,
        /// One entry per hosted marriage, in declaration order.
        hosted: Vec<HostedMarriage>,
    },
    /// The hub of a polygamy fan (ADR-0027): one card at row R; each
    /// co-spouse is a separate walker child of the hub at row R+1
    /// reached by a thick marriage edge. The hub has no bar geometry
    /// — the marriage edge replaces the bar as the "married to"
    /// visual.
    PolygamyHub {
        card: Box<PersonCard>,
        /// One entry per hosted marriage, in declaration order. The
        /// edge router consults this to emit one
        /// [`EdgeKind::Marriage`] edge per marriage from the hub
        /// card's bottom-midpoint to the matching co-spouse card's
        /// top-midpoint.
        marriages: Vec<HubMarriage>,
    },
    /// One co-spouse of a polygamy hub: just the co-spouse card. The
    /// "married to hub" visual is the thick marriage edge connecting
    /// hub-bottom to this card's top; no bar is emitted.
    FanCoSpouse {
        // `MarriageBar` is the heaviest type in this enum (it carries
        // an optional P6 nested birth-family `Box<PersonCard>` plus
        // four date / id strings), so box it to keep the discriminant
        // under `clippy::large_enum_variant`'s threshold.
        bar: Box<MarriageBar>,
        joining_slot: CardSlot,
    },
    /// A leaf person card with no hosted marriages.
    PersonLeaf { card: Box<PersonCard> },
    /// A single-card orphan component (P12 + P13).
    Orphan { card: Box<CardSlot> },
}

struct HostedMarriage {
    bar: MarriageBar,
    joining_slot: CardSlot,
}

/// One marriage hosted by a polygamy hub — the minimum the fan needs
/// to emit a marriage edge per marriage: the marriage id (the edge's
/// `marriage_id`) and the co-spouse id (the edge's far endpoint,
/// looked up in `card_tops`). R14 guarantees every polygamy marriage
/// is un-ended, so no `ended` flag is carried.
struct HubMarriage {
    marriage_id: String,
    joining_id: String,
}

struct Builder<'a> {
    config: &'a LayoutConfig,
    nodes: Vec<Node>,
    roots: Vec<usize>,
    /// The set of `(marriage_id, child_id)` pairs the adapter laid out
    /// as direct structural parent-child relationships — i.e. the
    /// child's `PersonCard` was placed in the children row directly
    /// below that marriage's bar. Edges whose endpoints both resolve
    /// but whose pair isn't here are the displaced-child / P11 case:
    /// route them as [`EdgeRouting::CrossTree`].
    structural_edges: std::collections::HashSet<(String, String)>,
    /// node_index → marriage_id, populated for every P16 child-ghost
    /// (past-adoption and past-bio). The edge router consults this so
    /// the parent-child edge from a past intimacy's bar terminates on
    /// the local child-ghost rather than crossing the canvas to the
    /// canonical card — without it the ghost would render as a visual
    /// orphan, contradicting its load-bearing role as a local anchor.
    child_ghost_marriage: std::collections::HashMap<usize, String>,
}

impl<'a> Builder<'a> {
    fn new(config: &'a LayoutConfig) -> Self {
        Self {
            config,
            nodes: Vec::new(),
            roots: Vec::new(),
            structural_edges: std::collections::HashSet::new(),
            child_ghost_marriage: std::collections::HashMap::new(),
        }
    }

    fn add_component(&mut self, component: &Component) {
        match &component.kind {
            ComponentKind::FamilyTree { root } => {
                // Pre-register the top root index so it sits at the
                // *front* of `self.roots` for this component. Any P6
                // nested birth-family sub-trees discovered during the
                // DFS pre-order recursion in `build_person` push their
                // roots onto `self.roots` immediately after, so each
                // nested sub-tree packs to the right of the host tree
                // (and grand-nesteds adjacent to their parent nested).
                let expected = self.nodes.len();
                self.roots.push(expected);
                let actual = self.build_person_root(root);
                debug_assert_eq!(expected, actual);
            }
            ComponentKind::OrphanPerson { card } => {
                let orphan = self.push_orphan((**card).clone());
                self.roots.push(orphan);
            }
        }
    }

    fn push_orphan(&mut self, card: CardSlot) -> usize {
        let visual_row = f64::from(card.generation);
        let width = self.config.card_width;
        self.nodes.push(Node {
            kind: NodeKind::Orphan {
                card: Box::new(card),
            },
            width,
            visual_row,
            children: Vec::new(),
        });
        self.nodes.len() - 1
    }

    /// Build a FamilyTree's root PersonCard. Same code path as a
    /// child PersonCard inside a MarriageBranch — `build_person`
    /// already handles the N=1 monogamy shape via `NodeKind::PersonHost`
    /// (and the leaf-shape via `NodeKind::PersonLeaf`) and the N≥2
    /// fan shape via `NodeKind::PolygamyHub` + `NodeKind::FanCoSpouse`.
    /// A ghost-rooted PersonCard flows through the same path; its
    /// `slot.kind` carries the ghost discriminator and `push_card`
    /// translates the visual styling.
    fn build_person_root(&mut self, card: &PersonCard) -> usize {
        self.build_person(card, 0.0)
    }

    /// Build a person subtree, with `min_visual_row` as the minimum
    /// visual row the subtree's root may sit at.
    ///
    /// `min_visual_row` exists because the polygamy fan (ADR-0027)
    /// inserts the co-spouse on its own row between the hub and the
    /// marriage's children, so every node strictly below a polygamy
    /// hub is visually one row deeper than its data-level
    /// `slot.generation` would predict. Each recursive call passes
    /// its own effective row plus 1 as the child's floor — for the
    /// non-polygamy corpus this floor stays below the data
    /// generation and the cascade is unchanged.
    fn build_person(&mut self, card: &PersonCard, min_visual_row: f64) -> usize {
        let host_floor = f64::from(card.slot.generation).max(min_visual_row);
        if card.hosted_marriages.is_empty() {
            let idx = self.nodes.len();
            self.nodes.push(Node {
                kind: NodeKind::PersonLeaf {
                    card: Box::new(card.clone()),
                },
                width: self.config.card_width,
                visual_row: host_floor,
                children: Vec::new(),
            });
            return idx;
        }

        if card.hosted_marriages.len() >= 2 {
            return self.build_polygamy_fan(card, host_floor);
        }

        // Monogamy (N=1): classical card + bar + joining card in one
        // cluster on a single row.
        let hosted: Vec<HostedMarriage> = card
            .hosted_marriages
            .iter()
            .map(|m| HostedMarriage {
                bar: m.bar.clone(),
                joining_slot: m.bar.joining_slot.clone(),
            })
            .collect();
        let per_marriage_extension =
            self.config.bar_gap * 2.0 + self.config.bar_width + self.config.card_width;
        let width = self.config.card_width + per_marriage_extension * hosted.len() as f64;
        let idx = self.nodes.len();
        self.nodes.push(Node {
            kind: NodeKind::PersonHost {
                card: Box::new(card.clone()),
                hosted,
            },
            width,
            // Provisional — recomputed below once nested roots are built.
            visual_row: host_floor,
            children: Vec::new(),
        });

        // Children of this host = union of all hosted marriages'
        // children, in declaration order across marriages. Each
        // (marriage, child) pair is recorded as a structural edge so
        // edge routing can distinguish displaced-child relationships
        // (P11, [`EdgeRouting::CrossTree`]) from the standard
        // descendency-tree shape (P1, [`EdgeRouting::InTree`]).
        //
        // ADR-0023: as we recurse into each P6 nested root we collect
        // its node index so the host's `visual_row` can be folded as
        // `max(host_floor, 1.0 + max(nested.visual_row))` after the
        // bottom-up traversal completes. Building nesteds (and their
        // descendants) before the fold guarantees each nested's
        // `visual_row` is final by the time we read it.
        let child_floor = host_floor + 1.0;
        let mut children: Vec<usize> = Vec::new();
        let mut nested_root_indices: Vec<usize> = Vec::new();
        for marriage in &card.hosted_marriages {
            // P6: if this marriage's joining spouse carries a nested
            // birth-family sub-tree, push it as an additional Walker
            // root *before* descending into the marriage's children
            // (ADR-0022 sibling-root packing, DFS pre-order). Walker's
            // multi-root pass places it adjacent to the host tree on
            // the right; any grand-nesteds discovered inside this
            // sub-tree push themselves further right in turn. Nested
            // birth-family sub-trees are independent walker roots —
            // they don't inherit the polygamy floor; reset to 0.
            if let Some(nested) = &marriage.bar.joining_nested_birth_family {
                let nested_expected = self.nodes.len();
                self.roots.push(nested_expected);
                let nested_actual = self.build_person(nested, 0.0);
                debug_assert_eq!(nested_expected, nested_actual);
                nested_root_indices.push(nested_actual);
            }
            for child in &marriage.children {
                self.structural_edges.insert((
                    marriage.bar.marriage_id.clone(),
                    child.slot.person_id.clone(),
                ));
                let child_idx = self.build_person(child, child_floor);
                if matches!(
                    child.slot.kind,
                    RenderSlotKind::Ghost {
                        reason: GhostReason::PastAdoption | GhostReason::PastBirth,
                    },
                ) {
                    // P16: this ghost is the child-anchor for the past
                    // intimacy represented by `marriage.bar`. Edge
                    // routing keys on (child_id, marriage_id) so the
                    // parent-child edge lands here rather than on the
                    // distant canonical card.
                    self.child_ghost_marriage
                        .insert(child_idx, marriage.bar.marriage_id.clone());
                }
                children.push(child_idx);
            }
        }
        let visual_row = fold_visual_row(host_floor, &self.nodes, &nested_root_indices, &children);
        self.nodes[idx].children = children;
        self.nodes[idx].visual_row = visual_row;
        idx
    }

    /// Build a polygamy hub: the host card alone at row R, plus one
    /// co-spouse cluster per hosted marriage as a walker child at
    /// row R+1. Each marriage's descendants attach as walker
    /// grandchildren of the hub (children of the matching
    /// `FanCoSpouse`) at row R+2, so each marriage's children hang in
    /// their own column directly below their co-spouse (ADR-0027).
    /// The "married to hub" visual is the thick marriage edge
    /// emitted in `finish()`; no bar is rendered for any polygamy
    /// marriage.
    fn build_polygamy_fan(&mut self, card: &PersonCard, host_floor: f64) -> usize {
        let hub_idx = self.nodes.len();
        // Provisional hub node — children pushed below; visual_row
        // recomputed after the co-spouse subtrees are built and
        // folded.
        self.nodes.push(Node {
            kind: NodeKind::PolygamyHub {
                card: Box::new(card.clone()),
                marriages: card
                    .hosted_marriages
                    .iter()
                    .map(|m| HubMarriage {
                        marriage_id: m.bar.marriage_id.clone(),
                        joining_id: m.bar.joining_id.clone(),
                    })
                    .collect(),
            },
            width: self.config.card_width,
            visual_row: host_floor,
            children: Vec::new(),
        });

        // Each co-spouse is its own walker child cluster of width =
        // card_width, sitting on the standard child generation row
        // (`host_floor + 1.0`). Children of the marriage become
        // walker children of this co-spouse, pushed one further row
        // down (`host_floor + 2.0`) via the `min_visual_row` floor
        // passed into `build_person` — the polygamy hub adds one
        // extra visual row that the canonical-family
        // `slot.generation` doesn't account for (the co-spouse
        // occupies the row that would otherwise host the marriage's
        // children).
        let cospouse_row = host_floor + 1.0;
        let grandchild_floor = host_floor + 2.0;
        let mut cospouse_indices: Vec<usize> = Vec::new();
        for marriage in &card.hosted_marriages {
            let cospouse_idx = self.nodes.len();
            self.nodes.push(Node {
                kind: NodeKind::FanCoSpouse {
                    bar: Box::new(marriage.bar.clone()),
                    joining_slot: marriage.bar.joining_slot.clone(),
                },
                width: self.config.card_width,
                // Provisional — recomputed by `fold_visual_row` below
                // so a deep P6 nested under the co-spouse can pull
                // the co-spouse (and transitively the hub) further
                // down per ADR-0023 / ADR-0024.
                visual_row: cospouse_row,
                children: Vec::new(),
            });

            // P6 nested birth-family of the co-spouse: push as an
            // additional Walker root, same as monogamy. The nested
            // sub-tree packs to the right of the host tree per
            // ADR-0022. Nested roots are independent walker roots
            // that don't inherit the polygamy floor; reset to 0.
            let mut nested_root_indices: Vec<usize> = Vec::new();
            if let Some(nested) = &marriage.bar.joining_nested_birth_family {
                let nested_expected = self.nodes.len();
                self.roots.push(nested_expected);
                let nested_actual = self.build_person(nested, 0.0);
                debug_assert_eq!(nested_expected, nested_actual);
                nested_root_indices.push(nested_actual);
            }
            let mut grandchildren: Vec<usize> = Vec::new();
            for child in &marriage.children {
                self.structural_edges.insert((
                    marriage.bar.marriage_id.clone(),
                    child.slot.person_id.clone(),
                ));
                let child_idx = self.build_person(child, grandchild_floor);
                if matches!(
                    child.slot.kind,
                    RenderSlotKind::Ghost {
                        reason: GhostReason::PastAdoption | GhostReason::PastBirth,
                    },
                ) {
                    self.child_ghost_marriage
                        .insert(child_idx, marriage.bar.marriage_id.clone());
                }
                grandchildren.push(child_idx);
            }
            let cospouse_visual_row = fold_visual_row(
                cospouse_row,
                &self.nodes,
                &nested_root_indices,
                &grandchildren,
            );
            self.nodes[cospouse_idx].children = grandchildren;
            self.nodes[cospouse_idx].visual_row = cospouse_visual_row;
            cospouse_indices.push(cospouse_idx);
        }

        // Hub fold: with co-spouses on integer row R+1, the standard
        // cascade applies — `fold_visual_row` reads
        // `min(cospouse.visual_row) - 1.0`, which is at least R and
        // grows past R when a deep sub-tree below a co-spouse has
        // pushed that co-spouse below R+1.
        let hub_visual_row = fold_visual_row(host_floor, &self.nodes, &[], &cospouse_indices);
        self.nodes[hub_idx].children = cospouse_indices;
        self.nodes[hub_idx].visual_row = hub_visual_row;
        hub_idx
    }

    fn finish(self, render_edges: &[Edge]) -> PositionedShape {
        let Builder {
            config,
            nodes,
            roots,
            structural_edges,
            child_ghost_marriage,
        } = self;

        let walker_input: Vec<InputNode> = nodes
            .iter()
            .map(|n| InputNode {
                width: n.width,
                children: n.children.clone(),
            })
            .collect();
        let positions = walker::run(&walker_input, &roots, config.sibling_gap);

        // Determine the bounding box. Walker centers each node on
        // `positions[i].x`. The cluster's left edge is `x - width/2`.
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_gen: f64 = 0.0;
        for (i, node) in nodes.iter().enumerate() {
            let left = positions[i].x - node.width / 2.0;
            let right = positions[i].x + node.width / 2.0;
            if left < min_x {
                min_x = left;
            }
            if right > max_x {
                max_x = right;
            }
            if node.visual_row > max_gen {
                max_gen = node.visual_row;
            }
        }
        if !min_x.is_finite() {
            // Empty document — return an empty canvas.
            return PositionedShape {
                width: config.padding * 2.0,
                height: config.padding * 2.0,
                cards: Vec::new(),
                bars: Vec::new(),
                edges: Vec::new(),
            };
        }

        let offset_x = config.padding - min_x;
        let offset_y = config.padding;

        // Project nodes back to PositionedShape primitives.
        let mut cards: Vec<PositionedCard> = Vec::new();
        let mut bars: Vec<PositionedBar> = Vec::new();
        // Track each marriage's bar centroid + bus row for edge routing.
        let mut bar_centers: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        // Track each canonical / leaf card's top-center for edge routing.
        let mut card_tops: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        // P16 child-ghost positions (past-adoption and past-bio),
        // keyed by (person_id, marriage_id). Consulted ahead of
        // `card_tops` so the parent-child edge from a past intimacy
        // terminates on the local ghost, not the distant canonical
        // card.
        let mut ghost_card_tops: std::collections::HashMap<(String, String), (f64, f64)> =
            std::collections::HashMap::new();
        // Hub bottom-midpoints, keyed by hub person_id. Marriage
        // edges (ADR-0027) originate here, one per concurrent
        // marriage hosted by the hub.
        let mut hub_bottoms: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        // Hub-derived marriage list, keyed by hub person_id. The
        // edge router walks each (hub, marriages) pair after card
        // positions are known to emit one EdgeKind::Marriage per
        // hosted marriage.
        let mut hub_marriages: Vec<(String, Vec<HubMarriage>)> = Vec::new();

        for (i, node) in nodes.iter().enumerate() {
            let cluster_left = positions[i].x - node.width / 2.0 + offset_x;
            let row_top = offset_y + node.visual_row * config.row_height;
            match &node.kind {
                NodeKind::PersonHost { card, hosted } => {
                    let host_x = cluster_left;
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        host_x,
                        row_top,
                        &card.slot,
                        config,
                    );
                    let mut cursor = host_x + config.card_width;
                    for entry in hosted {
                        let bar_x = cursor + config.bar_gap;
                        let bar_y = row_top + (config.card_height - config.bar_height) / 2.0;
                        let bar_center_x = bar_x + config.bar_width / 2.0;
                        bars.push(PositionedBar {
                            marriage_id: entry.bar.marriage_id.clone(),
                            x: bar_x,
                            y: bar_y,
                            width: config.bar_width,
                            height: config.bar_height,
                            ended: entry.bar.ended,
                        });
                        bar_centers.insert(
                            entry.bar.marriage_id.clone(),
                            (bar_center_x, bar_y + config.bar_height),
                        );
                        let joining_x = bar_x + config.bar_width + config.bar_gap;
                        push_card(
                            &mut cards,
                            &mut card_tops,
                            joining_x,
                            row_top,
                            &entry.joining_slot,
                            config,
                        );
                        cursor = joining_x + config.card_width;
                    }
                }
                NodeKind::PolygamyHub { card, marriages } => {
                    // Hub card alone at row R, centered on the
                    // cluster's walker-assigned x. `cluster_left` is
                    // the cluster's left edge (= centre - width/2),
                    // which equals the hub card's left edge because
                    // the hub's `width = card_width`.
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        cluster_left,
                        row_top,
                        &card.slot,
                        config,
                    );
                    hub_bottoms.insert(
                        card.slot.person_id.clone(),
                        (
                            cluster_left + config.card_width / 2.0,
                            row_top + config.card_height,
                        ),
                    );
                    let cloned: Vec<HubMarriage> = marriages
                        .iter()
                        .map(|m| HubMarriage {
                            marriage_id: m.marriage_id.clone(),
                            joining_id: m.joining_id.clone(),
                        })
                        .collect();
                    hub_marriages.push((card.slot.person_id.clone(), cloned));
                }
                NodeKind::FanCoSpouse { bar, joining_slot } => {
                    // Co-spouse cluster: just the card. No bar emitted
                    // — the "married to hub" visual is the marriage
                    // edge routed below from the hub's bottom-midpoint
                    // to this card's top-midpoint. Children of this
                    // marriage anchor at this card's bottom-midpoint,
                    // which `bar_centers` records under
                    // `bar.marriage_id` so the existing parent-child
                    // edge router needs no polygamy-specific branch.
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        cluster_left,
                        row_top,
                        joining_slot,
                        config,
                    );
                    bar_centers.insert(
                        bar.marriage_id.clone(),
                        (
                            cluster_left + config.card_width / 2.0,
                            row_top + config.card_height,
                        ),
                    );
                }
                NodeKind::PersonLeaf { card } => {
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        cluster_left,
                        row_top,
                        &card.slot,
                        config,
                    );
                    if let Some(marriage_id) = child_ghost_marriage.get(&i) {
                        ghost_card_tops.insert(
                            (card.slot.person_id.clone(), marriage_id.clone()),
                            (cluster_left + config.card_width / 2.0, row_top),
                        );
                    }
                }
                NodeKind::Orphan { card, .. } => {
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        cluster_left,
                        row_top,
                        card,
                        config,
                    );
                }
            }
        }

        let mut edges = route_edges(
            render_edges,
            &bar_centers,
            &card_tops,
            &ghost_card_tops,
            &structural_edges,
            config,
        );

        // Marriage edges (ADR-0027): one per hosted marriage of a
        // polygamy hub, routed hub-bottom → bus → co-spouse-top with
        // the same orthogonal geometry as a birth edge. Always
        // InTree (the co-spouse is by construction a walker child of
        // the hub in the same component).
        for (hub_id, marriages) in &hub_marriages {
            let &(hub_cx, hub_bottom_y) = hub_bottoms
                .get(hub_id)
                .expect("polygamy hub was emitted above");
            for marriage in marriages {
                let Some(&(spouse_cx, spouse_top_y)) = card_tops.get(&marriage.joining_id) else {
                    continue;
                };
                let bus_y = spouse_top_y - config.bus_drop;
                let points = vec![
                    (hub_cx, hub_bottom_y),
                    (hub_cx, bus_y),
                    (spouse_cx, bus_y),
                    (spouse_cx, spouse_top_y),
                ];
                edges.push(PositionedEdge {
                    kind: EdgeKind::Marriage,
                    routing: EdgeRouting::InTree,
                    child_id: marriage.joining_id.clone(),
                    marriage_id: marriage.marriage_id.clone(),
                    points,
                });
            }
        }

        let canvas_width = max_x - min_x + config.padding * 2.0;
        let canvas_height = (max_gen + 1.0) * config.row_height
            - (config.row_height - config.card_height)
            + config.padding * 2.0;

        PositionedShape {
            width: canvas_width,
            height: canvas_height,
            cards,
            bars,
            edges,
        }
    }
}

/// Bottom-up cascade for a cluster's `visual_row` per ADR-0023 +
/// ADR-0024:
///
/// ```text
/// visual_row(cluster) = max(
///     host_generation,
///     1.0 + max(visual_row(nested)) for nesting marriages,
///     min(visual_row(child)) - 1.0,
/// )
/// ```
///
/// Both `nested_root_indices` and `children` index into `nodes`. The
/// caller is responsible for ensuring every referenced node already
/// has its final `visual_row` (the DFS guarantees this: children and
/// nesteds are folded before the parent).
fn fold_visual_row(
    host_generation: f64,
    nodes: &[Node],
    nested_root_indices: &[usize],
    children: &[usize],
) -> f64 {
    let nested_max_row = nested_root_indices
        .iter()
        .map(|&i| nodes[i].visual_row)
        .reduce(f64::max);
    let child_min_row = children
        .iter()
        .map(|&i| nodes[i].visual_row)
        .reduce(f64::min);
    match (nested_max_row, child_min_row) {
        (Some(n), Some(c)) => host_generation.max(n + 1.0).max(c - 1.0),
        (Some(n), None) => host_generation.max(n + 1.0),
        (None, Some(c)) => host_generation.max(c - 1.0),
        (None, None) => host_generation,
    }
}

fn push_card(
    cards: &mut Vec<PositionedCard>,
    tops: &mut std::collections::HashMap<String, (f64, f64)>,
    x: f64,
    y: f64,
    slot: &CardSlot,
    config: &LayoutConfig,
) {
    let kind = match slot.kind {
        RenderSlotKind::Canonical => SlotKind::Canonical,
        RenderSlotKind::Ghost { reason } => SlotKind::Ghost { reason },
    };
    if matches!(kind, SlotKind::Canonical) {
        // Index canonical-card top-centres by person_id for the edge
        // router's fall-through lookup. P8 parent-ghosts intentionally
        // don't land here (P10: the ghost is mute and the child edge
        // attaches to the bar). P16 child-ghosts get their own
        // (person_id, marriage_id) index built alongside this map —
        // see `ghost_card_tops` in `finish()` — because the ghost IS
        // the edge's child endpoint at the past intimacy's row.
        tops.insert(slot.person_id.clone(), (x + config.card_width / 2.0, y));
    }
    cards.push(PositionedCard {
        person_id: slot.person_id.clone(),
        kind,
        x,
        y,
        width: config.card_width,
        height: config.card_height,
        name: slot.name.clone(),
    });
}

fn route_edges(
    render_edges: &[Edge],
    bar_centers: &std::collections::HashMap<String, (f64, f64)>,
    card_tops: &std::collections::HashMap<String, (f64, f64)>,
    ghost_card_tops: &std::collections::HashMap<(String, String), (f64, f64)>,
    structural_edges: &std::collections::HashSet<(String, String)>,
    config: &LayoutConfig,
) -> Vec<PositionedEdge> {
    let mut out = Vec::with_capacity(render_edges.len());
    for edge in render_edges {
        // P6 (ADR-0022): nested birth-family bars are positioned as
        // additional Walker roots, so every render edge's marriage id
        // is in `bar_centers`. For polygamy marriages (ADR-0027) no
        // `<rect class="kul-bar">` is emitted but the same map carries
        // the co-spouse card's bottom-midpoint under the marriage's
        // id, so the parent-child edge routing needs no polygamy
        // branch.
        let &(bar_cx, bar_by) = bar_centers
            .get(&edge.marriage_id)
            .expect("every render edge's marriage must have a positioned anchor");
        // P16: when a child has a child-ghost (past-adoption or
        // past-bio) at this marriage's children row, the parent-child
        // edge attaches to the local ghost rather than the canonical
        // card — the ghost is materialised precisely to be the local
        // anchor.
        let Some(&(card_cx, card_top)) = ghost_card_tops
            .get(&(edge.child_id.clone(), edge.marriage_id.clone()))
            .or_else(|| card_tops.get(&edge.child_id))
        else {
            continue;
        };
        let kind = match edge.kind {
            RenderEdgeKind::Birth => EdgeKind::Birth,
            RenderEdgeKind::Adoption => EdgeKind::Adoption,
        };
        let routing =
            if structural_edges.contains(&(edge.marriage_id.clone(), edge.child_id.clone())) {
                EdgeRouting::InTree
            } else {
                // P11 / displaced-child: both endpoints sit in the laid-out
                // tree but the child is not a structural descendant of this
                // marriage. The cousin-marriage case is the canonical
                // exerciser. Geometry matches `InTree` (per ADR-0018);
                // only the routing discriminator differs.
                EdgeRouting::CrossTree
            };
        let bus_y = card_top - config.bus_drop;
        let points = vec![
            (bar_cx, bar_by),
            (bar_cx, bus_y),
            (card_cx, bus_y),
            (card_cx, card_top),
        ];
        out.push(PositionedEdge {
            kind,
            routing,
            child_id: edge.child_id.clone(),
            marriage_id: edge.marriage_id.clone(),
            points,
        });
    }
    out
}

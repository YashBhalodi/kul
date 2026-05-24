//! Canonical UI pattern adapter — wraps [`crate::walker`] for kul's
//! pattern primitives (marriage bars between adjacent spouses, ghost
//! slots at host's birth-family position per P8, generation rows from
//! generation indices, orthogonal right-angle edge routing).
//!
//! The adapter consumes a [`kul_render::SuccessRender`] and builds an
//! internal layout tree, runs Walker's over it, then projects the
//! resulting positions back into a [`crate::PositionedShape`].

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
/// either a single card, or a card-bar-card host cluster.
struct Node {
    /// Anchor type: what visual primitive this cluster is.
    kind: NodeKind,
    /// Horizontal extent of the cluster.
    width: f64,
    /// Generation row (0 = top).
    generation: u32,
    /// Children clusters (in declaration order).
    children: Vec<usize>,
}

enum NodeKind {
    /// A person card (canonical or ghost); may host one or more
    /// marriages (each adding a bar + joining card to the cluster's
    /// right). Children are the union of all hosted marriages'
    /// children, in declaration order across marriages. This variant
    /// covers both the root case (a FamilyTree's root PersonCard, per
    /// ADR-0021) and the child case (a hosted person inside a
    /// MarriageBranch).
    PersonHost {
        card: Box<PersonCard>,
        /// One entry per hosted marriage, in declaration order.
        hosted: Vec<HostedMarriage>,
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
    /// node_index → marriage_id, populated only for P16 past-adoption
    /// ghost children. The edge router consults this so the dashed
    /// adoption edge from a past adoption's bar terminates on the
    /// local child-ghost rather than crossing the canvas to the
    /// canonical card — without it the ghost would render as a visual
    /// orphan, contradicting its load-bearing role as a local anchor.
    past_adoption_ghost_marriage: std::collections::HashMap<usize, String>,
}

impl<'a> Builder<'a> {
    fn new(config: &'a LayoutConfig) -> Self {
        Self {
            config,
            nodes: Vec::new(),
            roots: Vec::new(),
            structural_edges: std::collections::HashSet::new(),
            past_adoption_ghost_marriage: std::collections::HashMap::new(),
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
        let generation = card.generation;
        let width = self.config.card_width;
        self.nodes.push(Node {
            kind: NodeKind::Orphan {
                card: Box::new(card),
            },
            width,
            generation,
            children: Vec::new(),
        });
        self.nodes.len() - 1
    }

    /// Build a FamilyTree's root PersonCard. Same code path as a
    /// child PersonCard inside a MarriageBranch — `build_person`
    /// already handles N hosted marriages via `NodeKind::PersonHost`
    /// (and the leaf-shape via `NodeKind::PersonLeaf`). A
    /// ghost-rooted PersonCard flows through the same path; its
    /// `slot.kind` carries the ghost discriminator and `push_card`
    /// translates the visual styling.
    fn build_person_root(&mut self, card: &PersonCard) -> usize {
        self.build_person(card)
    }

    fn build_person(&mut self, card: &PersonCard) -> usize {
        let generation = card.slot.generation;
        if card.hosted_marriages.is_empty() {
            let idx = self.nodes.len();
            self.nodes.push(Node {
                kind: NodeKind::PersonLeaf {
                    card: Box::new(card.clone()),
                },
                width: self.config.card_width,
                generation,
                children: Vec::new(),
            });
            return idx;
        }

        // Host cluster width: host card + sum of (bar_gap + bar_width +
        // bar_gap + joining_card_width) per hosted marriage.
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
            generation,
            children: Vec::new(),
        });

        // Children of this host = union of all hosted marriages'
        // children, in declaration order across marriages. Each
        // (marriage, child) pair is recorded as a structural edge so
        // edge routing can distinguish displaced-child relationships
        // (P11, [`EdgeRouting::CrossTree`]) from the standard
        // descendency-tree shape (P1, [`EdgeRouting::InTree`]).
        let mut children: Vec<usize> = Vec::new();
        for marriage in &card.hosted_marriages {
            // P6: if this marriage's joining spouse carries a nested
            // birth-family sub-tree, push it as an additional Walker
            // root *before* descending into the marriage's children
            // (ADR-0022 sibling-root packing, DFS pre-order). Walker's
            // multi-root pass places it adjacent to the host tree on
            // the right; any grand-nesteds discovered inside this
            // sub-tree push themselves further right in turn.
            if let Some(nested) = &marriage.bar.joining_nested_birth_family {
                let nested_expected = self.nodes.len();
                self.roots.push(nested_expected);
                let nested_actual = self.build_person(nested);
                debug_assert_eq!(nested_expected, nested_actual);
            }
            for child in &marriage.children {
                self.structural_edges.insert((
                    marriage.bar.marriage_id.clone(),
                    child.slot.person_id.clone(),
                ));
                let child_idx = self.build_person(child);
                if matches!(
                    child.slot.kind,
                    RenderSlotKind::Ghost {
                        reason: GhostReason::PastAdoption,
                    },
                ) {
                    // P16: this ghost is the child-anchor for the past
                    // adoption represented by `marriage.bar`. Edge
                    // routing keys on (child_id, marriage_id) so the
                    // dashed adoption edge lands here rather than on
                    // the distant canonical card.
                    self.past_adoption_ghost_marriage
                        .insert(child_idx, marriage.bar.marriage_id.clone());
                }
                children.push(child_idx);
            }
        }
        self.nodes[idx].children = children;
        idx
    }

    fn finish(self, render_edges: &[Edge]) -> PositionedShape {
        let Builder {
            config,
            nodes,
            roots,
            structural_edges,
            past_adoption_ghost_marriage,
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
        let mut max_gen: u32 = 0;
        for (i, node) in nodes.iter().enumerate() {
            let left = positions[i].x - node.width / 2.0;
            let right = positions[i].x + node.width / 2.0;
            if left < min_x {
                min_x = left;
            }
            if right > max_x {
                max_x = right;
            }
            if node.generation > max_gen {
                max_gen = node.generation;
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
        // P16 past-adoption ghost positions, keyed by
        // (person_id, marriage_id). Consulted ahead of `card_tops` so
        // the dashed adoption edge from a past adoption terminates on
        // the local ghost, not the distant canonical card.
        let mut ghost_card_tops: std::collections::HashMap<(String, String), (f64, f64)> =
            std::collections::HashMap::new();

        for (i, node) in nodes.iter().enumerate() {
            let cluster_left = positions[i].x - node.width / 2.0 + offset_x;
            let row_top = offset_y + node.generation as f64 * config.row_height;
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
                NodeKind::PersonLeaf { card } => {
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        cluster_left,
                        row_top,
                        &card.slot,
                        config,
                    );
                    if let Some(marriage_id) = past_adoption_ghost_marriage.get(&i) {
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

        let edges = route_edges(
            render_edges,
            &bar_centers,
            &card_tops,
            &ghost_card_tops,
            &structural_edges,
            config,
        );

        let canvas_width = max_x - min_x + config.padding * 2.0;
        let canvas_height = (max_gen as f64 + 1.0) * config.row_height
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
        // the edge's child endpoint at the past adoption's row.
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
        // is in `bar_centers`. The old F8 silent-drop branch is gone.
        let &(bar_cx, bar_by) = bar_centers
            .get(&edge.marriage_id)
            .expect("every render edge's marriage must have a positioned bar");
        // P16: when a child has a past-adoption ghost at this
        // marriage's children row, the dashed adoption edge attaches
        // to the local ghost rather than the canonical card — the
        // ghost is materialised precisely to be the local anchor.
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

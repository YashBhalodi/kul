//! Canonical UI pattern adapter — wraps [`crate::walker`] for kul's
//! pattern primitives (marriage bars between adjacent spouses, ghost
//! slots at host's birth-family position per P8, generation rows from
//! generation indices, orthogonal right-angle edge routing).
//!
//! The adapter consumes a [`kul_render::SuccessRender`] and builds an
//! internal layout tree, runs Walker's over it, then projects the
//! resulting positions back into a [`crate::PositionedShape`].

use kul_render::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind as RenderEdgeKind, MarriageBar,
    MarriageBranch, PersonCard, SlotKind as RenderSlotKind, SuccessRender,
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
/// either a single card, or a card-bar-card host cluster, or a floating
/// top-level bar with two adjacent cards.
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
    /// A floating top-level marriage (P8 fallback or a component root):
    /// bar with host + joining cards adjacent. No anchor person card
    /// above it on the same row.
    RootMarriage {
        bar: Box<MarriageBar>,
        host_slot: Box<CardSlot>,
        joining_slot: Box<CardSlot>,
    },
    /// A canonical person card; may host one or more marriages (each
    /// adding a bar + joining card to the cluster's right). Children
    /// are the union of all hosted marriages' children, in declaration
    /// order across marriages.
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
}

impl<'a> Builder<'a> {
    fn new(config: &'a LayoutConfig) -> Self {
        Self {
            config,
            nodes: Vec::new(),
            roots: Vec::new(),
        }
    }

    fn add_component(&mut self, component: &Component) {
        let root = match &component.kind {
            ComponentKind::FamilyTree { root } => self.build_branch_root(root),
            ComponentKind::OrphanPerson { card } => self.push_orphan((**card).clone()),
        };
        self.roots.push(root);
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

    fn build_branch_root(&mut self, branch: &MarriageBranch) -> usize {
        let bar = branch.bar.clone();
        let generation = bar.generation;
        let host_slot = bar.host_slot.clone();
        let joining_slot = bar.joining_slot.clone();
        let width =
            self.config.card_width * 2.0 + self.config.bar_gap * 2.0 + self.config.bar_width;
        let idx = self.nodes.len();
        self.nodes.push(Node {
            kind: NodeKind::RootMarriage {
                bar: Box::new(bar),
                host_slot: Box::new(host_slot),
                joining_slot: Box::new(joining_slot),
            },
            width,
            generation,
            children: Vec::new(),
        });
        let children = self.build_children(&branch.children);
        self.nodes[idx].children = children;
        idx
    }

    fn build_children(&mut self, children: &[PersonCard]) -> Vec<usize> {
        children.iter().map(|c| self.build_person(c)).collect()
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
        // children, in declaration order across marriages.
        let mut children: Vec<usize> = Vec::new();
        for marriage in &card.hosted_marriages {
            for child in &marriage.children {
                children.push(self.build_person(child));
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

        for (i, node) in nodes.iter().enumerate() {
            let cluster_left = positions[i].x - node.width / 2.0 + offset_x;
            let row_top = offset_y + node.generation as f64 * config.row_height;
            match &node.kind {
                NodeKind::RootMarriage {
                    bar,
                    host_slot,
                    joining_slot,
                } => {
                    let host_x = cluster_left;
                    let bar_x = host_x + config.card_width + config.bar_gap;
                    let bar_center_x = bar_x + config.bar_width / 2.0;
                    let bar_y = row_top + (config.card_height - config.bar_height) / 2.0;
                    let joining_x = bar_x + config.bar_width + config.bar_gap;
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        host_x,
                        row_top,
                        host_slot,
                        config,
                    );
                    bars.push(PositionedBar {
                        marriage_id: bar.marriage_id.clone(),
                        x: bar_x,
                        y: bar_y,
                        width: config.bar_width,
                        height: config.bar_height,
                        ended: bar.ended,
                    });
                    bar_centers.insert(
                        bar.marriage_id.clone(),
                        (bar_center_x, bar_y + config.bar_height),
                    );
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        joining_x,
                        row_top,
                        joining_slot,
                        config,
                    );
                }
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

        let edges = route_edges(render_edges, &bar_centers, &card_tops, config);

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
        // Only canonical cards anchor child edges (P10: ghosts are
        // mute except for their own anchoring bar — child edges
        // attach to the bar, not the ghost). For child→parent edges,
        // the child end is always canonical; for parent ghosts, only
        // canonical-card lookups matter here.
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
    config: &LayoutConfig,
) -> Vec<PositionedEdge> {
    let mut out = Vec::with_capacity(render_edges.len());
    for edge in render_edges {
        let Some(&(bar_cx, bar_by)) = bar_centers.get(&edge.marriage_id) else {
            // The bar isn't positioned in this layout pass (e.g. an
            // edge whose marriage lives in a component the v1
            // adapter doesn't surface yet). Skip — the cross-tree
            // follow-up (F5) will route these.
            continue;
        };
        let Some(&(card_cx, card_top)) = card_tops.get(&edge.child_id) else {
            continue;
        };
        let kind = match edge.kind {
            RenderEdgeKind::Birth => EdgeKind::Birth,
            RenderEdgeKind::Adoption => EdgeKind::Adoption,
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
            routing: EdgeRouting::InTree,
            child_id: edge.child_id.clone(),
            marriage_id: edge.marriage_id.clone(),
            points,
        });
    }
    out
}

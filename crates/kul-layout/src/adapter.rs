//! Canonical UI pattern adapter — wraps [`crate::walker`] for kul's
//! pattern primitives.
//!
//! Consumes a [`kul_render::SuccessRender`], builds an internal layout
//! tree, runs Walker's over it, then projects positions back into a
//! [`crate::PositionedShape`].
//!
//! ## Polygamy hubs (ADR-0020)
//!
//! When a person hosts ≥2 concurrent marriages, the adapter builds a
//! **fan** governed by one invariant:
//!
//! ```text
//! children_center_i = (hub_cx + cospouse_cx_i) / 2
//! ```
//!
//! Each marriage's children gather under the midpoint of that
//! marriage's edge; the co-spouse is the mirror of the hub across that
//! midpoint. The hub is a single walker *leaf* whose width reserves the
//! full wing-to-wing extent so the fan packs cleanly against siblings.

use kul_core::export::ExportedDate;
use kul_render::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind as RenderEdgeKind, GhostReason, MarriageBar,
    PersonCard, SlotKind as RenderSlotKind, SuccessRender,
};

use crate::metrics::LayoutConfig;
use crate::shape::{EdgeKind, PositionedCard, PositionedEdge, PositionedShape, SlotKind};
use crate::walker::{self, InputNode};

pub(crate) fn lay_out(success: &SuccessRender, config: &LayoutConfig) -> PositionedShape {
    let mut builder = Builder::new(config);
    for component in &success.components {
        builder.add_component(component);
    }
    builder.finish(&success.edges)
}

/// A virtual layout node Walker positions. Each `Node` is one cluster.
struct Node {
    kind: NodeKind,
    width: f64,
    /// Surface layout row (0.0 = top). Computed bottom-up per ADR-0018:
    ///
    /// ```text
    /// visual_row(cluster) = max(
    ///     host_card.slot.generation,
    ///     1.0 + max(visual_row(nested)) for nesting marriages,
    ///     min(visual_row(child)) - 1.0,
    /// )
    /// ```
    ///
    /// The nesting clause pushes a host down to make room for a nested
    /// sub-tree from the absorb rule; the descendant-pull clause aligns
    /// kin-symmetric ancestors across an inter-family marriage. For a
    /// polygamy hub the fan's children sit two rows below the hub, so
    /// the descendant-pull clause reads `min(child.visual_row) - 2.0`.
    visual_row: f64,
    /// A polygamy hub's children are the flattened per-marriage
    /// children forests; co-spouse cards are placed by
    /// [`Builder::finish`], not walker nodes.
    children: Vec<usize>,
}

enum NodeKind {
    /// Monogamy host: card + marriage edge + joining card on a single
    /// row (`hosted_marriages.len() == 1`).
    PersonHost {
        card: Box<PersonCard>,
        hosted: Vec<HostedMarriage>,
    },
    /// Polygamy fan hub (ADR-0020). A single walker leaf whose width
    /// reserves the full wing-to-wing extent. Co-spouses and children
    /// forests are positioned by [`Builder::finish`] from the precomputed
    /// hub-local geometry in `marriages`.
    PolygamyHub {
        card: Box<PersonCard>,
        /// Hub centre in the fan's local x frame; positions project via
        /// `global_hub_x - hub_cx`.
        hub_cx: f64,
        marriages: Vec<FanMarriage>,
    },
    PersonLeaf {
        card: Box<PersonCard>,
    },
    Orphan {
        card: Box<CardSlot>,
    },
}

struct HostedMarriage {
    bar: MarriageBar,
    joining_slot: CardSlot,
}

/// One marriage of a polygamy hub, fan geometry precomputed in the
/// hub-local x frame (ADR-0020). R14 guarantees every polygamy marriage
/// is un-ended, but `end` / `end_reason` / `is_ended` are carried so the
/// marriage edge plumbs every declared property uniformly (ADR-0021).
struct FanMarriage {
    marriage_id: String,
    host_id: String,
    joining_id: String,
    joining_slot: CardSlot,
    start: Option<String>,
    end: Option<String>,
    end_reason: Option<String>,
    is_ended: bool,
    /// Co-spouse card centre, hub-local x.
    cospouse_cx: f64,
    /// Marriage-edge midpoint, hub-local x: `(hub_cx + cospouse_cx)/2`.
    /// The children forest's block centre and the child-edge origins
    /// both pin here.
    children_center: f64,
    /// Forest roots in declaration order; translated rigidly in
    /// [`Builder::finish`] so the block centre lands on
    /// `children_center`. Empty for a childless marriage.
    child_roots: Vec<usize>,
}

struct Builder<'a> {
    config: &'a LayoutConfig,
    nodes: Vec<Node>,
    roots: Vec<usize>,
    /// node_index → marriage_id for every past-intimacy child-ghost.
    /// Routes the parent-child edge to the local ghost instead of the
    /// distant canonical card (the ghost's load-bearing role as a local
    /// anchor).
    child_ghost_marriage: std::collections::HashMap<usize, String>,
}

impl<'a> Builder<'a> {
    fn new(config: &'a LayoutConfig) -> Self {
        Self {
            config,
            nodes: Vec::new(),
            roots: Vec::new(),
            child_ghost_marriage: std::collections::HashMap::new(),
        }
    }

    fn add_component(&mut self, component: &Component) {
        match &component.kind {
            ComponentKind::FamilyTree { root } => {
                // Pre-register the top root so nested birth-family
                // sub-trees discovered during DFS pre-order pack to the
                // right of the host tree (ADR-0018 sibling-root packing).
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

    fn build_person_root(&mut self, card: &PersonCard) -> usize {
        self.build_person(card, 0.0)
    }

    /// Build a person subtree. `min_visual_row` floors the subtree's
    /// root row — used by the polygamy fan (ADR-0020), which inserts
    /// the co-spouse row between the hub and its children so descendants
    /// sit one row below their data-level `slot.generation`.
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

        // Monogamy (N=1): card + bar + joining card on a single row.
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
            visual_row: host_floor,
            children: Vec::new(),
        });

        let child_floor = host_floor + 1.0;
        let mut children: Vec<usize> = Vec::new();
        let nested_root_indices: Vec<usize> = Vec::new();
        for marriage in &card.hosted_marriages {
            for child in &marriage.children {
                let child_idx = self.build_person(child, child_floor);
                if matches!(
                    child.slot.kind,
                    RenderSlotKind::Ghost {
                        reason: GhostReason::PastAdoption | GhostReason::PastBirth,
                    },
                ) {
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

    /// Build a polygamy hub (ADR-0020). Geometry is prescribed
    /// analytically in children-centre space:
    ///
    /// 1. [`fan_children_centers`] places each `C_i` with adjacent
    ///    spacing `max((CW_i + CW_{i+1})/2 + gap, clr)` (`clr =
    ///    (cw + gap)/2`), enforcing the child-drop clearance
    ///    `|C_i - hub_cx| >= clr`.
    /// 2. `cospouse_cx_i = 2 * C_i - hub_cx`.
    /// 3. Translate marriage `i`'s forest so its block centre lands on
    ///    `C_i`.
    fn build_polygamy_fan(&mut self, card: &PersonCard, host_floor: f64) -> usize {
        let hub_idx = self.nodes.len();
        self.nodes.push(Node {
            kind: NodeKind::PolygamyHub {
                card: Box::new(card.clone()),
                hub_cx: 0.0,
                marriages: Vec::new(),
            },
            width: self.config.card_width,
            visual_row: host_floor,
            children: Vec::new(),
        });

        let cw = self.config.card_width;
        let gap = self.config.sibling_gap;
        let children_floor = host_floor + 2.0;

        struct PendingMarriage {
            marriage_id: String,
            host_id: String,
            joining_id: String,
            joining_slot: CardSlot,
            start: Option<String>,
            end: Option<String>,
            end_reason: Option<String>,
            is_ended: bool,
            child_roots: Vec<usize>,
            children_width: f64,
        }
        let mut pending: Vec<PendingMarriage> = Vec::new();
        let mut min_child_row: Option<f64> = None;
        for marriage in &card.hosted_marriages {
            let mut child_roots: Vec<usize> = Vec::new();
            for child in &marriage.children {
                let child_idx = self.build_person(child, children_floor);
                if matches!(
                    child.slot.kind,
                    RenderSlotKind::Ghost {
                        reason: GhostReason::PastAdoption | GhostReason::PastBirth,
                    },
                ) {
                    self.child_ghost_marriage
                        .insert(child_idx, marriage.bar.marriage_id.clone());
                }
                let row = self.nodes[child_idx].visual_row;
                min_child_row = Some(min_child_row.map_or(row, |m: f64| m.min(row)));
                child_roots.push(child_idx);
            }
            let children_width = self.measure_forest_width(&child_roots);
            pending.push(PendingMarriage {
                marriage_id: marriage.bar.marriage_id.clone(),
                host_id: marriage.bar.host_id.clone(),
                joining_id: marriage.bar.joining_id.clone(),
                joining_slot: marriage.bar.joining_slot.clone(),
                start: marriage.bar.start.as_ref().map(fmt_date),
                end: marriage.bar.end.as_ref().map(fmt_date),
                end_reason: marriage.bar.end_reason.clone(),
                is_ended: marriage.bar.ended,
                child_roots,
                children_width,
            });
        }

        let clr = (cw + gap) / 2.0;
        let widths: Vec<f64> = pending.iter().map(|m| m.children_width).collect();
        let bearing: Vec<bool> = pending.iter().map(|m| !m.child_roots.is_empty()).collect();
        let relative = fan_children_centers(&widths, &bearing, gap, clr);

        let hub_cx = 0.0_f64;

        let mut marriages: Vec<FanMarriage> = Vec::with_capacity(pending.len());
        let mut min_wing = hub_cx;
        let mut max_wing = hub_cx;
        for (m, &children_center) in pending.iter().zip(&relative) {
            let cospouse_cx = 2.0 * children_center - hub_cx;
            min_wing = min_wing.min(cospouse_cx - cw / 2.0);
            max_wing = max_wing.max(cospouse_cx + cw / 2.0);
            if !m.child_roots.is_empty() {
                min_wing = min_wing.min(children_center - m.children_width / 2.0);
                max_wing = max_wing.max(children_center + m.children_width / 2.0);
            }
            marriages.push(FanMarriage {
                marriage_id: m.marriage_id.clone(),
                host_id: m.host_id.clone(),
                joining_id: m.joining_id.clone(),
                joining_slot: m.joining_slot.clone(),
                start: m.start.clone(),
                end: m.end.clone(),
                end_reason: m.end_reason.clone(),
                is_ended: m.is_ended,
                cospouse_cx,
                children_center,
                child_roots: m.child_roots.clone(),
            });
        }

        // Symmetric wing-to-wing extent so the global walker's contour
        // packing keeps siblings clear of the widest wing.
        let reserved = 2.0 * (hub_cx - min_wing).max(max_wing - hub_cx);

        // Fan children sit two rows below the hub (co-spouse row
        // between), so the descendant-pull clause is `min - 2.0`.
        let hub_visual_row = match min_child_row {
            Some(c) => host_floor.max(c - 2.0),
            None => host_floor,
        };

        // Attach the forests as the hub's walker children so the
        // global walker reserves the hub's contour against siblings;
        // natural positions are overridden in `finish`.
        let forest_children: Vec<usize> = marriages
            .iter()
            .flat_map(|m| m.child_roots.iter().copied())
            .collect();

        let hub = &mut self.nodes[hub_idx];
        hub.width = reserved.max(self.config.card_width);
        hub.visual_row = hub_visual_row;
        hub.children = forest_children;
        if let NodeKind::PolygamyHub {
            hub_cx: stored_cx,
            marriages: stored,
            ..
        } = &mut hub.kind
        {
            *stored_cx = hub_cx;
            *stored = marriages;
        }
        hub_idx
    }

    /// Local walker pass over `self.nodes`; returns the forest's packed
    /// extent width (`0.0` if empty). Used to size co-spouse spacing
    /// before the global walker has run.
    fn measure_forest_width(&self, roots: &[usize]) -> f64 {
        if roots.is_empty() {
            return 0.0;
        }
        let walker_input = walker_input(&self.nodes);
        let local = walker::run(&walker_input, roots, self.config.sibling_gap);
        let (min_x, max_x) = forest_extent(&self.nodes, roots, &local);
        max_x - min_x
    }

    fn finish(self, render_edges: &[Edge]) -> PositionedShape {
        let Builder {
            config,
            nodes,
            roots,
            child_ghost_marriage,
        } = self;

        let walker_input: Vec<InputNode> = nodes
            .iter()
            .map(|n| InputNode {
                width: n.width,
                children: n.children.clone(),
            })
            .collect();
        let mut positions = walker::run(&walker_input, &roots, config.sibling_gap);

        // Polygamy fan reposition (ADR-0020): override each hub's
        // forest positions so each marriage's block centre lands on its
        // prescribed `children_center`. The hub's reserved width already
        // cleared siblings of the widest wing.
        for (hub_idx, node) in nodes.iter().enumerate() {
            let NodeKind::PolygamyHub {
                hub_cx, marriages, ..
            } = &node.kind
            else {
                continue;
            };
            let hub_x = positions[hub_idx].x;
            for marriage in marriages {
                if marriage.child_roots.is_empty() {
                    continue;
                }
                let (min_x, max_x) = forest_extent(&nodes, &marriage.child_roots, &positions);
                let block_center = (min_x + max_x) / 2.0;
                let target = hub_x + (marriage.children_center - hub_cx);
                let delta = target - block_center;
                if delta != 0.0 {
                    translate_forest(&nodes, &marriage.child_roots, &mut positions, delta);
                }
            }
        }

        // Co-spouse cards aren't walker nodes — their wing extent is
        // already covered by the hub's reserved width.
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
            return PositionedShape {
                width: config.padding * 2.0,
                height: config.padding * 2.0,
                cards: Vec::new(),
                edges: Vec::new(),
            };
        }

        let offset_x = config.padding - min_x;
        let offset_y = config.padding;

        let mut cards: Vec<PositionedCard> = Vec::new();
        // Per-marriage child-attach anchor (gap midpoint for monogamy,
        // marriage-edge midpoint just below the hub for polygamy).
        let mut bar_centers: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        let mut card_tops: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        // Past-intimacy child-ghost positions, consulted ahead of
        // `card_tops` so the parent-child edge from a past intimacy
        // terminates on the local ghost.
        let mut ghost_card_tops: std::collections::HashMap<(String, String), (f64, f64)> =
            std::collections::HashMap::new();
        let mut marriage_edges: Vec<PositionedEdge> = Vec::new();

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
                    // Cursor: `[host][bar_gap][gap][bar_gap][joining]…`.
                    let mut cursor = host_x + config.card_width;
                    let mid_y = row_top + config.card_height / 2.0;
                    for entry in hosted {
                        let bar_x = cursor + config.bar_gap;
                        let left_card_right_edge = bar_x - config.bar_gap;
                        let right_card_left_edge = bar_x + config.bar_width + config.bar_gap;
                        bar_centers.insert(
                            entry.bar.marriage_id.clone(),
                            (bar_x + config.bar_width / 2.0, mid_y),
                        );
                        marriage_edges.push(PositionedEdge {
                            kind: EdgeKind::Marriage {
                                host_id: entry.bar.host_id.clone(),
                                joining_id: entry.bar.joining_id.clone(),
                                start: entry.bar.start.as_ref().map(fmt_date),
                                end: entry.bar.end.as_ref().map(fmt_date),
                                end_reason: entry.bar.end_reason.clone(),
                                is_ended: entry.bar.ended,
                            },
                            marriage_id: entry.bar.marriage_id.clone(),
                            points: vec![
                                (left_card_right_edge, mid_y),
                                (right_card_left_edge, mid_y),
                            ],
                        });
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
                NodeKind::PolygamyHub {
                    card,
                    hub_cx,
                    marriages,
                } => {
                    // Hub leaf's reserved width is the wing-to-wing
                    // extent, so `positions[i].x` is the hub centre
                    // (not the card's left edge).
                    let hub_center_abs = positions[i].x + offset_x;
                    let hub_left = hub_center_abs - config.card_width / 2.0;
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        hub_left,
                        row_top,
                        &card.slot,
                        config,
                    );
                    let hub_bottom_y = row_top + config.card_height;
                    let cospouse_row_top = row_top + config.row_height;
                    // Marriage-edge bus runs just below the hub; its
                    // midpoint `(children_center, bus_y)` is where each
                    // marriage's child birth edges originate.
                    let bus_y = cospouse_row_top - config.bus_drop;
                    for marriage in marriages {
                        let cospouse_cx = hub_center_abs + (marriage.cospouse_cx - hub_cx);
                        let children_center_abs =
                            hub_center_abs + (marriage.children_center - hub_cx);
                        let cospouse_left = cospouse_cx - config.card_width / 2.0;
                        push_card(
                            &mut cards,
                            &mut card_tops,
                            cospouse_left,
                            cospouse_row_top,
                            &marriage.joining_slot,
                            config,
                        );

                        // hub-bottom → bus → co-spouse top-centre.
                        marriage_edges.push(PositionedEdge {
                            kind: EdgeKind::Marriage {
                                host_id: marriage.host_id.clone(),
                                joining_id: marriage.joining_id.clone(),
                                start: marriage.start.clone(),
                                end: marriage.end.clone(),
                                end_reason: marriage.end_reason.clone(),
                                is_ended: marriage.is_ended,
                            },
                            marriage_id: marriage.marriage_id.clone(),
                            points: vec![
                                (hub_center_abs, hub_bottom_y),
                                (hub_center_abs, bus_y),
                                (cospouse_cx, bus_y),
                                (cospouse_cx, cospouse_row_top),
                            ],
                        });

                        // Unconditionally — mirrors `PersonHost`'s
                        // insert at the top of this match. A render edge
                        // can target this marriage even when
                        // `child_roots` is empty (e.g. an adoption-only
                        // child whose canonical_location resolves
                        // elsewhere), and `route_edges` requires the
                        // anchor for every render edge.
                        bar_centers
                            .insert(marriage.marriage_id.clone(), (children_center_abs, bus_y));
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
            config,
        );

        // Append marriage edges after birth/adoption edges.
        edges.append(&mut marriage_edges);

        let canvas_width = max_x - min_x + config.padding * 2.0;
        let canvas_height = (max_gen + 1.0) * config.row_height
            - (config.row_height - config.card_height)
            + config.padding * 2.0;

        PositionedShape {
            width: canvas_width,
            height: canvas_height,
            cards,
            edges,
        }
    }
}

fn walker_input(nodes: &[Node]) -> Vec<InputNode> {
    nodes
        .iter()
        .map(|n| InputNode {
            width: n.width,
            children: n.children.clone(),
        })
        .collect()
}

/// Bounding x-extent of a forest given a position table.
fn forest_extent(nodes: &[Node], roots: &[usize], positions: &[walker::LaidOut]) -> (f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut stack: Vec<usize> = roots.to_vec();
    while let Some(v) = stack.pop() {
        let half = nodes[v].width / 2.0;
        min_x = min_x.min(positions[v].x - half);
        max_x = max_x.max(positions[v].x + half);
        stack.extend(nodes[v].children.iter().copied());
    }
    (min_x, max_x)
}

/// Rigidly shift a forest by `delta` in `positions`.
fn translate_forest(
    nodes: &[Node],
    roots: &[usize],
    positions: &mut [walker::LaidOut],
    delta: f64,
) {
    let mut stack: Vec<usize> = roots.to_vec();
    while let Some(v) = stack.pop() {
        positions[v].x += delta;
        stack.extend(nodes[v].children.iter().copied());
    }
}

/// Bottom-up cascade for `visual_row` per ADR-0018:
///
/// ```text
/// visual_row(cluster) = max(
///     host_generation,
///     1.0 + max(visual_row(nested)) for nesting marriages,
///     min(visual_row(child)) - 1.0,
/// )
/// ```
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

/// Children-centre x for every marriage of a polygamy fan, in a
/// hub-local frame where the hub sits at `0.0` (ADR-0020). The caller
/// derives each co-spouse as `cospouse_cx = 2 * children_center - hub_cx`.
///
/// Two constraints are honoured:
///
/// 1. **Adjacent spacing** `c_{i+1} - c_i >= max((CW_i + CW_{i+1})/2 +
///    gap, clr)`.
/// 2. **Band clearance** every child-bearing marriage has
///    `|c_i| >= clr`, keeping its child-drop outside its co-spouse card.
///
/// Centres landing inside the forbidden band `(-clr, clr)` are nudged
/// out; the fan re-packs outward and the outer pair is mirrored so the
/// hub stays at their midpoint.
fn fan_children_centers(widths: &[f64], bearing: &[bool], gap: f64, clr: f64) -> Vec<f64> {
    let n = widths.len();
    debug_assert_eq!(n, bearing.len());
    if n == 0 {
        return Vec::new();
    }

    // Adjacent children-centre spacing (one per gap between marriages).
    let spacing: Vec<f64> = widths
        .windows(2)
        .map(|w| ((w[0] + w[1]) / 2.0 + gap).max(clr))
        .collect();

    // Natural cumulative placement, then centre on the midpoint of the
    // ends so the outer two are symmetric about 0.
    let mut c: Vec<f64> = Vec::with_capacity(n);
    let mut t = 0.0_f64;
    c.push(t);
    for &s in &spacing {
        t += s;
        c.push(t);
    }
    let mid = (c[0] + c[n - 1]) / 2.0;
    for v in &mut c {
        *v -= mid;
    }

    // Pivot = first centre at or right of the hub; sweep outward in
    // both directions.
    let pivot = c.iter().position(|&v| v >= 0.0).unwrap_or(n);

    for i in pivot..n {
        let mut floor = if i > 0 {
            c[i - 1] + spacing[i - 1]
        } else {
            c[i]
        };
        if bearing[i] {
            floor = floor.max(clr);
        }
        c[i] = c[i].max(floor);
    }
    for i in (0..pivot).rev() {
        let mut ceil = if i + 1 < n {
            c[i + 1] - spacing[i]
        } else {
            c[i]
        };
        if bearing[i] {
            ceil = ceil.min(-clr);
        }
        c[i] = c[i].min(ceil);
    }

    // Pin the hub to the midpoint of the outer pair (mirror to the
    // wider). Inner marriages keep their swept positions.
    let extent = c[0].abs().max(c[n - 1].abs());
    c[0] = -extent;
    c[n - 1] = extent;

    c
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
        // Past-intimacy child-ghosts get their own (person_id,
        // marriage_id) index via `ghost_card_tops`; spouse-ghosts are
        // mute and don't land here.
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
        generation: slot.generation,
        gender: slot.gender,
        family: slot.family.clone(),
        given: slot.given.clone(),
        born: slot.born.as_ref().map(fmt_date),
        died: slot.died.as_ref().map(fmt_date),
    });
}

/// Format an [`ExportedDate`] back into source `~YYYY[-MM[-DD]]` form.
fn fmt_date(date: &ExportedDate) -> String {
    if date.circa {
        format!("~{}", date.value)
    } else {
        date.value.clone()
    }
}

fn route_edges(
    render_edges: &[Edge],
    bar_centers: &std::collections::HashMap<String, (f64, f64)>,
    card_tops: &std::collections::HashMap<String, (f64, f64)>,
    ghost_card_tops: &std::collections::HashMap<(String, String), (f64, f64)>,
    config: &LayoutConfig,
) -> Vec<PositionedEdge> {
    let mut out = Vec::with_capacity(render_edges.len());
    for edge in render_edges {
        let &(bar_cx, bar_by) = bar_centers
            .get(&edge.marriage_id)
            .expect("every render edge's marriage must have a positioned anchor");
        // A past-intimacy child-ghost shadows the canonical card here;
        // resolving via the ghost map is exactly the `is_past` predicate.
        let ghost_hit = ghost_card_tops.get(&(edge.child_id.clone(), edge.marriage_id.clone()));
        let is_past = ghost_hit.is_some();
        let Some(&(card_cx, card_top)) = ghost_hit.or_else(|| card_tops.get(&edge.child_id)) else {
            continue;
        };
        let kind = match edge.kind {
            RenderEdgeKind::Birth => EdgeKind::Birth {
                child_id: edge.child_id.clone(),
                is_past,
            },
            RenderEdgeKind::Adoption => EdgeKind::Adoption {
                child_id: edge.child_id.clone(),
                is_past,
                start: edge.start.as_ref().map(fmt_date),
                end: edge.end.as_ref().map(fmt_date),
            },
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
            points,
            marriage_id: edge.marriage_id.clone(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::fan_children_centers;

    // Defaults: card_width 160, sibling_gap 32 → clr = 96, leaf = 160.
    const GAP: f64 = 32.0;
    const CLR: f64 = 96.0;
    const LEAF: f64 = 160.0;

    /// N=2, one childless + one single-child. Natural half-spacing
    /// narrower than `clr`, so both centres push to `±clr`.
    #[test]
    fn n2_one_childless_one_child_clears_to_clr() {
        let centers = fan_children_centers(&[0.0, LEAF], &[false, true], GAP, CLR);
        assert_eq!(centers, vec![-CLR, CLR]);
    }

    /// N=3, one child each. The middle would land on the hub column;
    /// it nudges to `+clr` and the outer pair splays.
    #[test]
    fn n3_middle_nudged_off_hub_outer_splays() {
        let centers = fan_children_centers(&[LEAF; 3], &[true; 3], GAP, CLR);
        assert_eq!(centers, vec![-288.0, CLR, 288.0]);

        for &c in &centers {
            assert!(c.abs() >= CLR, "center {c} inside forbidden band");
        }
        assert_eq!((centers[0] + centers[2]) / 2.0, 0.0);
    }

    /// N=4: the inner pair straddles the band at `±clr`, no nudge.
    #[test]
    fn n4_inner_pair_straddles_band_no_nudge() {
        let centers = fan_children_centers(&[LEAF; 4], &[true; 4], GAP, CLR);
        assert_eq!(centers, vec![-288.0, -CLR, CLR, 288.0]);
    }

    /// N=5: middle nudged off the hub column, fan re-packs outward.
    #[test]
    fn n5_middle_nudged_hub_centered_and_clear() {
        let centers = fan_children_centers(&[LEAF; 5], &[true; 5], GAP, CLR);
        assert_eq!((centers[0] + centers[4]) / 2.0, 0.0);
        for &c in &centers {
            assert!(c.abs() >= CLR, "center {c} inside forbidden band");
        }
        for pair in centers.windows(2) {
            assert!(pair[1] - pair[0] >= CLR, "adjacent centres overlap");
        }
    }
}

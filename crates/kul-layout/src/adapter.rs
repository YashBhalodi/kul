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
//! co-spouse plus its marriage bar sits at row `R + fan_drop_fraction`
//! as a walker child of the hub; each marriage's descendants follow
//! per ADR-0023's bottom-up cascade. The fan connector — a trunk plus
//! a branch plus per-bar drops — is emitted in [`Builder::finish`]
//! from the laid-out hub and bar centres. Monogamy
//! (`hosted_marriages.len() == 1`) keeps the classical hub-and-flanks
//! shape; the cluster carries the host card, the bar, and the joining
//! card in one Walker node.

use kul_render::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind as RenderEdgeKind, GhostReason, MarriageBar,
    PersonCard, SlotKind as RenderSlotKind, SuccessRender,
};

use crate::metrics::LayoutConfig;
use crate::shape::{
    EdgeKind, EdgeRouting, PositionedBar, PositionedCard, PositionedEdge, PositionedFanConnector,
    PositionedShape, SlotKind,
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
/// hub of a polygamy fan, or a co-spouse + bar pair underneath a hub.
struct Node {
    /// Anchor type: what visual primitive this cluster is.
    kind: NodeKind,
    /// Horizontal extent of the cluster.
    width: f64,
    /// Surface layout row (0.0 = top). Computed bottom-up per
    /// ADR-0023, refined by ADR-0024, and generalised to fractional
    /// rows by ADR-0027 for the fan primitive:
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
    /// clusters sit at `hub.visual_row + fan_drop_fraction` (a
    /// fractional offset less than 1.0) so the hub and the co-spouses
    /// remain visually in the same generation — the fractional row
    /// flows through the descendant-pull arithmetic without special-
    /// casing. For leaves, orphans, and hosts whose descendants
    /// haven't been pushed below their data-level row by any nesting
    /// upstream, both extra clauses collapse to
    /// `host_card.slot.generation`.
    visual_row: f64,
    /// Children clusters (in declaration order).
    children: Vec<usize>,
}

enum NodeKind {
    /// A monogamy person host: card + (bar + joining card) for each
    /// hosted marriage, all in one cluster on a single row. Covers the
    /// `hosted_marriages.len() == 1` case at any depth (root or
    /// child). Children are the union of all hosted marriages'
    /// children, in declaration order.
    PersonHost {
        card: Box<PersonCard>,
        /// One entry per hosted marriage, in declaration order.
        hosted: Vec<HostedMarriage>,
    },
    /// The hub of a polygamy fan (ADR-0027): one card at row R; the
    /// co-spouse + bar pairs are separate walker children at row
    /// `R + fan_drop_fraction`. The hub itself has no bar geometry —
    /// the fan connector replaces what the monogamy cluster would
    /// emit inline.
    PolygamyHub {
        card: Box<PersonCard>,
        /// One entry per hosted marriage, in declaration order. The
        /// bar geometry is owned by each co-spouse walker child's
        /// `FanCoSpouse` node (so the bar sits adjacent to its
        /// co-spouse), but the marriage id list lives here so
        /// `finish()` can stitch the fan connector to each bar.
        marriage_ids: Vec<String>,
    },
    /// One co-spouse of a polygamy hub: the co-spouse card plus the
    /// marriage bar that ties them to the hub. The bar abuts the
    /// co-spouse card on the side facing the hub's vertical axis (per
    /// ADR-0027): the **first**-declared co-spouse renders as
    /// `[Spouse][bar]` (bar on the right, spouse on the outer left),
    /// every other co-spouse renders as `[bar][Spouse]` (bar on the
    /// left). For N=2 this puts both bars facing inward toward the
    /// hub axis; for N≥3 middle spouses get the consistent left-bar
    /// treatment the spec calls out, and the last spouse is also
    /// `[bar][Spouse]` but its overall column sits on the outer
    /// right by virtue of being the last walker child.
    FanCoSpouse {
        // `MarriageBar` is the heaviest type in this enum (it carries
        // an optional P6 nested birth-family `Box<PersonCard>` plus
        // four date / id strings), so box it to keep the discriminant
        // under `clippy::large_enum_variant`'s threshold.
        bar: Box<MarriageBar>,
        joining_slot: CardSlot,
        /// `true` for the first-declared co-spouse (bar emitted on the
        /// right of the card); `false` for every other co-spouse (bar
        /// on the left).
        bar_on_right: bool,
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
        self.build_person(card)
    }

    fn build_person(&mut self, card: &PersonCard) -> usize {
        let host_generation = f64::from(card.slot.generation);
        if card.hosted_marriages.is_empty() {
            let idx = self.nodes.len();
            self.nodes.push(Node {
                kind: NodeKind::PersonLeaf {
                    card: Box::new(card.clone()),
                },
                width: self.config.card_width,
                visual_row: host_generation,
                children: Vec::new(),
            });
            return idx;
        }

        if card.hosted_marriages.len() >= 2 {
            return self.build_polygamy_fan(card, host_generation);
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
            visual_row: host_generation,
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
        // `max(host_generation, 1.0 + max(nested.visual_row))` after the
        // bottom-up traversal completes. Building nesteds (and their
        // descendants) before the fold guarantees each nested's
        // `visual_row` is final by the time we read it.
        let mut children: Vec<usize> = Vec::new();
        let mut nested_root_indices: Vec<usize> = Vec::new();
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
                nested_root_indices.push(nested_actual);
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
        let visual_row = fold_visual_row(
            host_generation,
            &self.nodes,
            &nested_root_indices,
            &children,
        );
        self.nodes[idx].children = children;
        self.nodes[idx].visual_row = visual_row;
        idx
    }

    /// Build a polygamy hub: the host card at row R, plus one
    /// co-spouse + bar cluster per hosted marriage at row
    /// `R + fan_drop_fraction`. Each marriage's descendants attach as
    /// walker grandchildren of the hub (children of the matching
    /// `FanCoSpouse`), so each marriage's children hang in their own
    /// column directly below their bar (ADR-0027).
    fn build_polygamy_fan(&mut self, card: &PersonCard, host_generation: f64) -> usize {
        let hub_idx = self.nodes.len();
        // Provisional hub node — children pushed below; visual_row
        // and width recomputed after the co-spouse subtrees are built
        // and folded.
        self.nodes.push(Node {
            kind: NodeKind::PolygamyHub {
                card: Box::new(card.clone()),
                marriage_ids: card
                    .hosted_marriages
                    .iter()
                    .map(|m| m.bar.marriage_id.clone())
                    .collect(),
            },
            width: self.config.card_width,
            visual_row: host_generation,
            children: Vec::new(),
        });

        let cospouse_row = host_generation + self.config.fan_drop_fraction;

        // One co-spouse cluster per hosted marriage. The cluster
        // carries the joining card plus the marriage bar (width =
        // bar_gap + bar_width + bar_gap + card_width) so the bar
        // abuts the co-spouse. Each marriage's children (and any P6
        // nested birth-family) become walker children of the
        // co-spouse cluster, so they pack directly below the bar in
        // their own column.
        let cospouse_cluster_width =
            self.config.bar_gap * 2.0 + self.config.bar_width + self.config.card_width;
        let mut cospouse_indices: Vec<usize> = Vec::new();
        for (m_idx, marriage) in card.hosted_marriages.iter().enumerate() {
            let bar_on_right = m_idx == 0;
            let cospouse_idx = self.nodes.len();
            self.nodes.push(Node {
                kind: NodeKind::FanCoSpouse {
                    bar: Box::new(marriage.bar.clone()),
                    joining_slot: marriage.bar.joining_slot.clone(),
                    bar_on_right,
                },
                width: cospouse_cluster_width,
                visual_row: cospouse_row,
                children: Vec::new(),
            });

            // P6 nested birth-family of the co-spouse: push as an
            // additional Walker root, same as monogamy. The nested
            // sub-tree packs to the right of the host tree per
            // ADR-0022.
            let mut nested_root_indices: Vec<usize> = Vec::new();
            if let Some(nested) = &marriage.bar.joining_nested_birth_family {
                let nested_expected = self.nodes.len();
                self.roots.push(nested_expected);
                let nested_actual = self.build_person(nested);
                debug_assert_eq!(nested_expected, nested_actual);
                nested_root_indices.push(nested_actual);
            }
            let mut grandchildren: Vec<usize> = Vec::new();
            for child in &marriage.children {
                self.structural_edges.insert((
                    marriage.bar.marriage_id.clone(),
                    child.slot.person_id.clone(),
                ));
                let child_idx = self.build_person(child);
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

        // Hub fold: pull the hub down toward its closest descendant
        // (per ADR-0024), but ignore the co-spouse row when computing
        // the pull — co-spouses sit at `R + fan_drop_fraction`, so
        // `min(cospouse) - 1.0 = R + fan_drop_fraction - 1.0`, which
        // for any sensible `fan_drop_fraction` (< 1.0) is less than R
        // and the outer max collapses to R anyway. The descendant-pull
        // therefore reads from the *grandchildren* row (one row below
        // the co-spouses) so an outer cluster with deeper ancestry on
        // its co-spouse side can still pull the hub down. The hub has
        // no nesting clause of its own — the per-marriage nesteds
        // attach to their own co-spouse cluster.
        let grandchild_min_row = cospouse_indices
            .iter()
            .flat_map(|&i| self.nodes[i].children.clone())
            .map(|i| self.nodes[i].visual_row)
            .reduce(f64::min);
        let hub_visual_row = match grandchild_min_row {
            // A grandchild already shifted below `cospouse_row + 1.0`
            // (because of a deep nested sub-tree on the co-spouse
            // side) pulls the hub down too; otherwise the hub stays
            // at its data-level generation.
            Some(g) => host_generation.max(g - 1.0 - self.config.fan_drop_fraction),
            None => host_generation,
        };
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
                fan_connectors: Vec::new(),
            };
        }

        let offset_x = config.padding - min_x;
        let offset_y = config.padding;

        // Project nodes back to PositionedShape primitives.
        let mut cards: Vec<PositionedCard> = Vec::new();
        let mut bars: Vec<PositionedBar> = Vec::new();
        let mut fan_connectors: Vec<PositionedFanConnector> = Vec::new();
        // Track each marriage's bar centroid + bus row for edge routing.
        let mut bar_centers: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        // Track each marriage's bar top-midpoint so the fan connector
        // can drop a vertical from the branch onto the bar.
        let mut bar_tops: std::collections::HashMap<String, (f64, f64)> =
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
        // Hub bottom-midpoints, keyed by hub person_id. The fan
        // connector's trunk starts here.
        let mut hub_bottoms: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();

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
                        bar_tops.insert(entry.bar.marriage_id.clone(), (bar_center_x, bar_y));
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
                NodeKind::PolygamyHub { card, .. } => {
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
                }
                NodeKind::FanCoSpouse {
                    bar,
                    joining_slot,
                    bar_on_right,
                } => {
                    // Co-spouse cluster: bar abuts the card on the side
                    // facing the hub's vertical axis (ADR-0027). For the
                    // first co-spouse (leftmost outer column) the bar
                    // sits on the right of the card; every other
                    // co-spouse renders bar-on-left so the bars cluster
                    // toward the hub's centerline. Cluster width is the
                    // same either way (bar_gap + bar_width + bar_gap +
                    // card_width); only the inner ordering flips.
                    let bar_y = row_top + (config.card_height - config.bar_height) / 2.0;
                    let (bar_x, joining_x) = if *bar_on_right {
                        let joining_x = cluster_left;
                        let bar_x = joining_x + config.card_width + config.bar_gap;
                        (bar_x, joining_x)
                    } else {
                        let bar_x = cluster_left + config.bar_gap;
                        let joining_x = bar_x + config.bar_width + config.bar_gap;
                        (bar_x, joining_x)
                    };
                    let bar_center_x = bar_x + config.bar_width / 2.0;
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
                    bar_tops.insert(bar.marriage_id.clone(), (bar_center_x, bar_y));
                    push_card(
                        &mut cards,
                        &mut card_tops,
                        joining_x,
                        row_top,
                        joining_slot,
                        config,
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

        // Emit one fan connector per polygamy hub. We walk the hub
        // nodes again to stitch the (hub_bottom, bar_tops...) geometry
        // — the hub's `marriage_ids` list preserves declaration order
        // so the branch endpoints follow source order.
        for node in &nodes {
            let NodeKind::PolygamyHub { card, marriage_ids } = &node.kind else {
                continue;
            };
            let &hub_bottom = hub_bottoms
                .get(&card.slot.person_id)
                .expect("polygamy hub was emitted above");
            let bar_top_points: Vec<(f64, f64)> = marriage_ids
                .iter()
                .map(|mid| {
                    *bar_tops
                        .get(mid)
                        .expect("every fan marriage has a positioned bar")
                })
                .collect();
            let segments = build_fan_segments(hub_bottom, &bar_top_points, config);
            fan_connectors.push(PositionedFanConnector {
                hub_id: card.slot.person_id.clone(),
                segments,
            });
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
        let canvas_height = (max_gen + 1.0) * config.row_height
            - (config.row_height - config.card_height)
            + config.padding * 2.0;

        PositionedShape {
            width: canvas_width,
            height: canvas_height,
            cards,
            bars,
            edges,
            fan_connectors,
        }
    }
}

/// Bottom-up cascade for a cluster's `visual_row` per ADR-0023 +
/// ADR-0024 (generalised to fractional rows by ADR-0027 for the fan
/// primitive):
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

/// Build the fan-connector segments from the hub's bottom-midpoint to
/// each bar's top-midpoint (ADR-0027). The geometry is decomposed
/// into orthogonal segments so the SVG emitter can render each as
/// its own polyline without retracing:
///
/// 1. **Trunk** — `hub_bottom → (hub_x, branch_y)`.
/// 2. **Branch** — `(leftmost_bar_x, branch_y) → (rightmost_bar_x, branch_y)`.
/// 3. **Drops** — one `(bar_x, branch_y) → (bar_x, bar_y)` per bar.
///
/// The trunk and the branch share the endpoint `(hub_x, branch_y)`,
/// so the two paths visually meet at a T-intersection that — at one
/// stroke colour — reads as a continuous "trunk plus branch" element
/// without the polyline-corner machinery having to draw a reverse
/// curve at the junction. The branch sits at
/// `bar_top.y - bus_drop / 2.0`, far enough above the bars that the
/// rounded corners on the per-bar drops have room to render without
/// overlapping the bars; the clearance is derived from `bus_drop` so
/// the fan's vertical proportions track the rest of the layout's
/// spacing constants.
fn build_fan_segments(
    hub_bottom: (f64, f64),
    bar_tops: &[(f64, f64)],
    config: &LayoutConfig,
) -> Vec<Vec<(f64, f64)>> {
    if bar_tops.is_empty() {
        return Vec::new();
    }
    // Branch sits a fixed distance above the bar-top row. All bars
    // sit at the same y (they share the co-spouse row), so picking
    // any bar's y suffices.
    let bar_y = bar_tops[0].1;
    let branch_y = bar_y - config.bus_drop / 2.0;
    let trunk_x = hub_bottom.0;
    let leftmost_x = bar_tops.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let rightmost_x = bar_tops
        .iter()
        .map(|p| p.0)
        .fold(f64::NEG_INFINITY, f64::max);

    let trunk = vec![hub_bottom, (trunk_x, branch_y)];
    let branch = vec![(leftmost_x, branch_y), (rightmost_x, branch_y)];

    let mut segments: Vec<Vec<(f64, f64)>> = vec![trunk, branch];
    for &(bar_x, _) in bar_tops {
        segments.push(vec![(bar_x, branch_y), (bar_x, bar_y)]);
    }
    segments
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
        // is in `bar_centers`. The old F8 silent-drop branch is gone.
        let &(bar_cx, bar_by) = bar_centers
            .get(&edge.marriage_id)
            .expect("every render edge's marriage must have a positioned bar");
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

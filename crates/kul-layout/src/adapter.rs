//! Canonical UI pattern adapter — wraps [`crate::walker`] for kul's
//! pattern primitives (thick marriage edges between adjacent spouses,
//! ghost slots at host's birth-family position per current-intimacy
//! placement, generation rows from generation indices, orthogonal
//! right-angle edge routing).
//!
//! The adapter consumes a [`kul_render::SuccessRender`] and builds an
//! internal layout tree, runs Walker's over it, then projects the
//! resulting positions back into a [`crate::PositionedShape`].
//!
//! ## Polygamy hubs (ADR-0020)
//!
//! When a person hosts ≥2 concurrent marriages, the adapter rearranges
//! the cluster into a **fan** built on one invariant:
//!
//! ```text
//! children_center_i = (hub_cx + cospouse_cx_i) / 2
//! ```
//!
//! i.e. each marriage's children gather under the *midpoint* of that
//! marriage's edge, and the co-spouse is the mirror of the hub across
//! that midpoint. Spouses therefore splay out toward the wings while
//! every marriage's children pull toward the centre, directly below
//! the thick marriage edge's horizontal segment.
//!
//! - Hub card alone at row R.
//! - Each co-spouse card at row R+1, at the wing position
//!   `2 * children_center_i - hub_cx`.
//! - Each marriage's children at row R+2, centred at the marriage-edge
//!   midpoint `children_center_i`. Children forests keep their full
//!   walker layout (nested birth families, deeper generations, even
//!   recursive polygamy) — only the forest's *block centre* is pinned
//!   to the midpoint.
//! - One thick [`EdgeKind::Marriage`] edge per marriage, routed
//!   hub-bottom → horizontal bus → co-spouse-top with the same
//!   orthogonal geometry as a birth edge (heavier stroke only). Its
//!   horizontal segment's midpoint is `children_center_i`, so each
//!   marriage's child birth edges originate there and fan down.
//!
//! A **childless** co-spouse keeps the same wing/mirror treatment with
//! an empty children block (its marriage edge lands on its top-centre).
//! Monogamy (`hosted_marriages.len() == 1`) keeps the classical hub-
//! and-flanks shape, the marriage rendered as a thick horizontal
//! [`EdgeKind::Marriage`] edge spanning the gap between the two adjacent
//! spouse cards at their vertical mid-height (the unified marriage
//! connector, ADR-0020).
//!
//! The whole fan is laid out in a hub-local x via a per-marriage local
//! walker pass (so children forests still get the tidy-tree treatment),
//! then projected against the hub's globally-assigned x. The hub is a
//! single **leaf** in the global walker tree whose width reserves the
//! full wing-to-wing extent, so a fan packs cleanly against sibling
//! components and nests inside a larger tree (example 12) without
//! overlap.

use kul_core::export::ExportedDate;
use kul_render::{
    CardSlot, Component, ComponentKind, Edge, EdgeKind as RenderEdgeKind, GhostReason, MarriageBar,
    PersonCard, SlotKind as RenderSlotKind, SuccessRender,
};

use crate::metrics::LayoutConfig;
use crate::shape::{EdgeKind, PositionedCard, PositionedEdge, PositionedShape, SlotKind};
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
    /// ADR-0018. Carried as `f64` so future
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
    /// (grand-)nested sub-tree from the absorb rule. The descendant-pull clause pulls a
    /// host *down* to sit one row above its closest descendant, so
    /// kin-symmetric ancestors across an inter-family marriage align
    /// on the same visual row. For a polygamy hub the fan's children
    /// sit two rows below the hub (the co-spouse row sits between), so
    /// the hub's row folds from `min(child.visual_row) - 2.0` — see
    /// [`Builder::build_polygamy_fan`]. For leaves, orphans, and hosts
    /// whose descendants haven't been pushed below their data-level row
    /// by any nesting upstream, both extra clauses collapse to
    /// `host_card.slot.generation`.
    visual_row: f64,
    /// Children clusters (in declaration order). A polygamy hub's
    /// children are the flattened per-marriage children forests, in
    /// declaration order; the co-spouse cards are *not* walker nodes
    /// (the adapter places them at their prescribed wing positions in
    /// [`Builder::finish`]).
    children: Vec<usize>,
}

enum NodeKind {
    /// A monogamy person host: card + marriage edge + joining card in
    /// one cluster on a single row. Covers the
    /// `hosted_marriages.len() == 1` case at any depth (root or
    /// child). Children are the union of all hosted marriages'
    /// children, in declaration order.
    PersonHost {
        card: Box<PersonCard>,
        /// One entry per hosted marriage, in declaration order.
        hosted: Vec<HostedMarriage>,
    },
    /// The hub of a polygamy fan (ADR-0020): one card at row R, sitting
    /// alone. The hub is a single walker *leaf* whose width reserves the
    /// full wing-to-wing extent of the fan so it packs cleanly against
    /// siblings; the co-spouses (row R+1) and children forests (row
    /// R+2) are positioned by [`Builder::finish`] from the precomputed
    /// hub-local geometry in `marriages`. The hub has no bar geometry —
    /// the thick marriage edge replaces the bar as the "married to"
    /// visual.
    PolygamyHub {
        card: Box<PersonCard>,
        /// Hub centre in the fan's local x frame (the global x assigned
        /// to the hub leaf maps here). Every per-marriage local
        /// position is projected by adding `global_hub_x - hub_cx`.
        hub_cx: f64,
        /// One entry per hosted marriage, in declaration order.
        marriages: Vec<FanMarriage>,
    },
    /// A leaf person card with no hosted marriages.
    PersonLeaf { card: Box<PersonCard> },
    /// A single-card orphan component (a lone-card component, per source order).
    Orphan { card: Box<CardSlot> },
}

struct HostedMarriage {
    bar: MarriageBar,
    joining_slot: CardSlot,
}

/// One marriage of a polygamy hub, with its fan geometry precomputed in
/// the hub-local x frame (ADR-0020). The co-spouse card sits at the
/// wing position `cospouse_cx` (= `2 * children_center - hub_cx`); the
/// marriage's children forest is rigidly translated so its block centre
/// lands on `children_center`, which is also the midpoint of the
/// marriage edge's horizontal segment (where the child birth edges
/// originate). R14 guarantees every polygamy marriage is un-ended, so
/// `is_ended` is always `false` and `end` / `end_reason` always `None`;
/// they are carried anyway so the marriage edge plumbs every declared
/// property uniformly (ADR-0021).
struct FanMarriage {
    marriage_id: String,
    /// Host (first-listed spouse) — the hub. Surfaces as `data-host-id`.
    host_id: String,
    joining_id: String,
    joining_slot: CardSlot,
    /// `start:` date, source form. Surfaces as `data-start`.
    start: String,
    /// `end:` date, source form (always `None` by R14).
    end: Option<String>,
    /// `end_reason:` (always `None` by R14).
    end_reason: Option<String>,
    /// `true` iff the marriage carries `end:` (always `false` by R14).
    is_ended: bool,
    /// Co-spouse card centre, hub-local x. The card draws at
    /// `cospouse_cx - card_width/2`.
    cospouse_cx: f64,
    /// Marriage-edge midpoint, hub-local x: `(hub_cx + cospouse_cx)/2`.
    /// The children forest's block centre and the child-edge origins
    /// both pin here.
    children_center: f64,
    /// Global walker node indices of this marriage's children forest
    /// roots, in declaration order. The forest (these roots and, via
    /// `Node::children`, their descendants) was laid out by the global
    /// walker; in [`Builder::finish`] it is rigidly translated so the
    /// forest's block centre lands on `children_center`. Empty for a
    /// childless marriage.
    child_roots: Vec<usize>,
}

struct Builder<'a> {
    config: &'a LayoutConfig,
    nodes: Vec<Node>,
    roots: Vec<usize>,
    /// node_index → marriage_id, populated for every past-intimacy child-ghost
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
            child_ghost_marriage: std::collections::HashMap::new(),
        }
    }

    fn add_component(&mut self, component: &Component) {
        match &component.kind {
            ComponentKind::FamilyTree { root } => {
                // Pre-register the top root index so it sits at the
                // *front* of `self.roots` for this component. Any
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
    /// fan shape via `NodeKind::PolygamyHub` ([`Builder::build_polygamy_fan`]).
    /// A ghost-rooted PersonCard flows through the same path; its
    /// `slot.kind` carries the ghost discriminator and `push_card`
    /// translates the visual styling.
    fn build_person_root(&mut self, card: &PersonCard) -> usize {
        self.build_person(card, 0.0)
    }

    /// Build a person subtree, with `min_visual_row` as the minimum
    /// visual row the subtree's root may sit at.
    ///
    /// `min_visual_row` exists because the polygamy fan (ADR-0020)
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
        // children, in declaration order across marriages. Every edge
        // routes with one orthogonal geometry regardless of whether the
        // child is a structural descendant or a displaced-child /
        // cousin-marriage cross-edge (ADR-0018), so no per-edge routing
        // discriminator is recorded here.
        //
        // ADR-0018: as we recurse into each nested root we collect
        // its node index so the host's `visual_row` can be folded as
        // `max(host_floor, 1.0 + max(nested.visual_row))` after the
        // bottom-up traversal completes. Building nesteds (and their
        // descendants) before the fold guarantees each nested's
        // `visual_row` is final by the time we read it.
        let child_floor = host_floor + 1.0;
        let mut children: Vec<usize> = Vec::new();
        let mut nested_root_indices: Vec<usize> = Vec::new();
        for marriage in &card.hosted_marriages {
            // The absorb rule: if this marriage's joining spouse carries a nested
            // birth-family sub-tree, push it as an additional Walker
            // root *before* descending into the marriage's children
            // (ADR-0018 sibling-root packing, DFS pre-order). Walker's
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
                let child_idx = self.build_person(child, child_floor);
                if matches!(
                    child.slot.kind,
                    RenderSlotKind::Ghost {
                        reason: GhostReason::PastAdoption | GhostReason::PastBirth,
                    },
                ) {
                    // Past intimacies emit ghosts: this ghost is the
                    // child-anchor for the past
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

    /// Build a polygamy hub per Approach 1 (ADR-0020): the host card
    /// alone at row R; each co-spouse mirrored across its marriage-edge
    /// midpoint at row R+1; each marriage's children forest centred on
    /// that midpoint at row R+2.
    ///
    /// The fan is laid out in a hub-local x. Children forests still get
    /// the tidy-tree treatment (nested birth families, deeper
    /// generations, recursive polygamy) — each is built through the
    /// usual `build_person` recursion and measured by a local walker
    /// pass for its packed width `CW_i`. The geometry is then prescribed
    /// analytically in **children-centre space** (the invariant ties each
    /// co-spouse to its marriage's children-centre, so laying out the
    /// centres directly is cleaner than co-spouse space):
    ///
    /// 1. [`fan_children_centers`] places each marriage's children-centre
    ///    `C_i` with adjacent spacing `max((CW_i + CW_{i+1})/2 + gap, clr)`
    ///    (`clr = (cw + gap)/2`), centred so the outer two are symmetric
    ///    about the hub, and enforces the child-drop clearance
    ///    `|C_i - hub_cx| >= clr` for every child-bearing marriage — which
    ///    keeps that marriage's drop outside its co-spouse card. For an
    ///    odd N the middle marriage would land on the hub column; it is
    ///    nudged off to `+clr` and the fan re-packs outward, keeping the
    ///    hub centred (the outer co-spouses splay wider in exchange).
    /// 2. `cospouse_cx_i = 2 * C_i - hub_cx` (the co-spouse mirrors the
    ///    hub across the children-centre).
    /// 3. Shift marriage `i`'s children forest so its block centre lands
    ///    on `C_i`.
    ///
    /// The hub is a single walker *leaf* whose width reserves the full
    /// wing-to-wing extent (symmetric about `hub_cx`), so it packs
    /// cleanly against siblings and nests inside a larger tree. The
    /// children forests and co-spouse cards are projected from this
    /// hub-local geometry against the hub's globally-assigned x in
    /// [`Builder::finish`].
    fn build_polygamy_fan(&mut self, card: &PersonCard, host_floor: f64) -> usize {
        let hub_idx = self.nodes.len();
        // Provisional hub node — geometry, width, and visual_row are
        // filled in below once the children forests are built and
        // measured. The hub leaf carries no walker children (the
        // forests are positioned locally, projected in `finish`).
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

        // Per-marriage: build the children forest, measure its packed
        // width, and record its joining-spouse slot. Forest roots are
        // collected so `finish` can rigidly translate the forest to its
        // prescribed midpoint.
        struct PendingMarriage {
            marriage_id: String,
            host_id: String,
            joining_id: String,
            joining_slot: CardSlot,
            start: String,
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
                start: fmt_date(&marriage.bar.start),
                end: marriage.bar.end.as_ref().map(fmt_date),
                end_reason: marriage.bar.end_reason.clone(),
                is_ended: marriage.bar.ended,
                child_roots,
                children_width,
            });
        }

        // Children-centre geometry (hub-local x). The fan is laid out in
        // children-centre space because the governing invariant ties each
        // co-spouse to its marriage's children-centre: `cospouse_cx = 2 *
        // children_center - hub_cx`. Working in this frame lets the
        // child-drop clearance be expressed as a single constraint on the
        // children-centre — `|children_center - hub_cx| >= clr` with
        // `clr = (cw + gap)/2` — which is what keeps a child-drop at the
        // marriage-edge midpoint outside its co-spouse card.
        let clr = (cw + gap) / 2.0;
        let widths: Vec<f64> = pending.iter().map(|m| m.children_width).collect();
        let bearing: Vec<bool> = pending.iter().map(|m| !m.child_roots.is_empty()).collect();
        let relative = fan_children_centers(&widths, &bearing, gap, clr);

        // Origin the local frame so the hub (midpoint of the outer two
        // children-centres, which `fan_children_centers` keeps at 0) sits
        // at a convenient hub-local x. Any constant works — the fan is
        // projected against the hub's global walker x in `finish` — but
        // anchoring at 0 keeps the geometry readable.
        let hub_cx = 0.0_f64;

        // Children centres + co-spouse wings. `cospouse_cx` is the mirror
        // of the hub across the children-centre; the forest's block centre
        // pins to `children_center` (the marriage-edge midpoint) in
        // `finish`.
        let mut marriages: Vec<FanMarriage> = Vec::with_capacity(pending.len());
        let mut min_wing = hub_cx;
        let mut max_wing = hub_cx;
        for (m, &children_center) in pending.iter().zip(&relative) {
            let cospouse_cx = 2.0 * children_center - hub_cx;
            // Reserve the co-spouse card extent (always present) and
            // the children block extent (when child-bearing).
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

        // Reserve a symmetric wing-to-wing extent so the global walker's
        // contour packing keeps siblings clear of the widest wing.
        let reserved = 2.0 * (hub_cx - min_wing).max(max_wing - hub_cx);

        // Hub row fold: the fan's children sit two rows below the hub
        // (the co-spouse row is between), so the descendant-pull clause
        // reads `min(child.visual_row) - 2.0`. A deep nested sub-tree under a
        // child forest pushes that child below R+2 and pulls the whole
        // fan down in lockstep.
        let hub_visual_row = match min_child_row {
            Some(c) => host_floor.max(c - 2.0),
            None => host_floor,
        };

        // Attach the children forests as the hub's walker children so
        // the global walker positions them (and any nested roots
        // they declared, via the usual `self.roots` path) and reserves
        // the hub's contour against siblings. Their natural walker
        // positions are then *overridden* in `finish` — each forest is
        // rigidly translated so its block centre lands on its
        // `children_center` relative to the hub's walker x. The wide
        // `reserved` width guarantees the hub's contour covers the wings
        // regardless of the forests' narrower natural spread.
        let forest_children: Vec<usize> = marriages
            .iter()
            .flat_map(|m| m.child_roots.clone())
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

    /// Lay out the given forest roots with a local walker pass over the
    /// already-built `self.nodes` and return the packed extent width
    /// (`0.0` for an empty forest). Used for sizing co-spouse spacing in
    /// the polygamy fan before the global walker has run.
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

        // Polygamy fan reposition (ADR-0020, Approach 1). The global
        // walker positioned each hub's children forests by their natural
        // tidy-tree spread; override them so each marriage's forest is
        // rigidly translated to its prescribed marriage-edge midpoint
        // `children_center`, measured relative to the hub's walker x.
        // The hub's wide reserved width already kept siblings clear of
        // the wings, so widening the forests' spread here cannot collide
        // with a neighbouring component.
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

        // Determine the bounding box. Walker centers each node on
        // `positions[i].x`. The cluster's left edge is `x - width/2`.
        // Co-spouse cards are not walker nodes; their wing extent is
        // already covered by the hub's reserved width, so the per-node
        // loop below captures it via the hub node.
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
                edges: Vec::new(),
            };
        }

        let offset_x = config.padding - min_x;
        let offset_y = config.padding;

        // Project nodes back to PositionedShape primitives.
        let mut cards: Vec<PositionedCard> = Vec::new();
        // Track each marriage's child-attach centroid + bus row for edge
        // routing. For monogamy this is the marriage-edge's gap midpoint
        // at the cards' mid-height; for polygamy it is the marriage-edge
        // midpoint just below the hub.
        let mut bar_centers: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        // Track each canonical / leaf card's top-center for edge routing.
        let mut card_tops: std::collections::HashMap<String, (f64, f64)> =
            std::collections::HashMap::new();
        // Past-intimacy child-ghost positions (past-adoption and past-bio),
        // keyed by (person_id, marriage_id). Consulted ahead of
        // `card_tops` so the parent-child edge from a past intimacy
        // terminates on the local ghost, not the distant canonical
        // card.
        let mut ghost_card_tops: std::collections::HashMap<(String, String), (f64, f64)> =
            std::collections::HashMap::new();
        // Marriage edges (ADR-0020): the unified marriage connector. One
        // thick horizontal edge per monogamy marriage (built in the
        // `PersonHost` arm, spanning the gap between the two adjacent
        // spouse cards) plus one thick edge per hosted marriage of a
        // polygamy hub (built in the `PolygamyHub` arm). Appended after
        // the birth/adoption edges from `route_edges`.
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
                    // The cursor walks `[host][bar_gap][gap][bar_gap][joining]…`
                    // exactly as before so spouse-card x positions stay
                    // byte-identical; the marriage now renders as a thick
                    // horizontal edge spanning the inter-card gap at the
                    // cards' mid-height instead of a bar rect (ADR-0020).
                    let mut cursor = host_x + config.card_width;
                    let mid_y = row_top + config.card_height / 2.0;
                    for entry in hosted {
                        let bar_x = cursor + config.bar_gap;
                        let left_card_right_edge = bar_x - config.bar_gap;
                        let right_card_left_edge = bar_x + config.bar_width + config.bar_gap;
                        // Child birth edges drop from the gap midpoint at
                        // the marriage edge's y; `route_edges` consumes
                        // this anchor with no monogamy-specific branch.
                        bar_centers.insert(
                            entry.bar.marriage_id.clone(),
                            (bar_x + config.bar_width / 2.0, mid_y),
                        );
                        marriage_edges.push(PositionedEdge {
                            kind: EdgeKind::Marriage {
                                host_id: entry.bar.host_id.clone(),
                                joining_id: entry.bar.joining_id.clone(),
                                start: fmt_date(&entry.bar.start),
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
                    // Hub card alone at row R, centered on the hub's
                    // walker-assigned x. The hub leaf's reserved width is
                    // the wide wing-to-wing extent (symmetric about the
                    // hub centre), so `positions[i].x` is the hub centre
                    // and `cluster_left` is *not* the card's left edge.
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

                    // Co-spouses sit on row R+1; their cards, the
                    // marriage edges, and the per-marriage child-edge
                    // origins are all projected here from the hub-local
                    // geometry, anchored at the hub's global centre.
                    let cospouse_row_top = row_top + config.row_height;
                    // The marriage edge's horizontal bus runs just below
                    // the hub at `cospouse_top - bus_drop`; its midpoint
                    // `(children_center, bus_y)` is where each marriage's
                    // child birth edges originate.
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

                        // Marriage edge: hub-bottom → bus → co-spouse
                        // top-centre, fanning out of the single hub
                        // bottom-midpoint. Its horizontal segment's
                        // midpoint is `(children_center, bus_y)`.
                        marriage_edges.push(PositionedEdge {
                            kind: EdgeKind::Marriage {
                                host_id: marriage.host_id.clone(),
                                joining_id: marriage.joining_id.clone(),
                                start: marriage.start.clone(),
                                // Polygamy marriages are always un-ended (R14).
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

                        // Child birth edges originate at the marriage-
                        // edge midpoint and fan down to the children
                        // (already repositioned so their block centre
                        // sits on `children_center`). `route_edges`
                        // consumes this anchor with no polygamy-specific
                        // branch.
                        if !marriage.child_roots.is_empty() {
                            bar_centers
                                .insert(marriage.marriage_id.clone(), (children_center_abs, bus_y));
                        }
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

        // The polygamy marriage edges were built in the projection loop
        // (the `PolygamyHub` arm); append them after the birth/adoption
        // edges so the edge order stays birth/adoption-first.
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

/// Snapshot the layout nodes as walker input (widths + child
/// adjacency). Used both for the global walker run and for the
/// per-forest measuring pass in the polygamy fan.
fn walker_input(nodes: &[Node]) -> Vec<InputNode> {
    nodes
        .iter()
        .map(|n| InputNode {
            width: n.width,
            children: n.children.clone(),
        })
        .collect()
}

/// Bounding x-extent of a forest (its `roots` and, via `Node::children`,
/// all descendants) given a position table.
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

/// Rigidly shift a forest (its `roots` and all descendants) by `delta`
/// in `positions`. Used by the polygamy fan to pin each marriage's
/// children block centre onto its marriage-edge midpoint (ADR-0020).
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

/// Bottom-up cascade for a cluster's `visual_row` per ADR-0018 +
/// ADR-0018:
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

/// Children-centre x for every marriage of a polygamy fan, in a hub-local
/// frame where the hub sits at `0.0` (the midpoint of the outer two
/// centres). The caller derives each co-spouse from the invariant
/// `cospouse_cx = 2 * children_center - hub_cx` (ADR-0020, Approach 1).
///
/// `widths[i]` is marriage `i`'s children-block width (`0.0` if childless),
/// `bearing[i]` whether marriage `i` has any children, `gap` the sibling
/// gap, and `clr = (cw + gap)/2` the half-clearance that keeps a
/// child-bearing marriage's drop (at its children-centre) outside its
/// co-spouse card.
///
/// Two constraints are honoured:
///
/// 1. **Adjacent spacing** `c_{i+1} - c_i >= max((CW_i + CW_{i+1})/2 + gap,
///    clr)` — children blocks live half a co-spouse step apart, so this
///    keeps neighbouring blocks (and co-spouse cards) from overlapping.
/// 2. **Band clearance** — every child-bearing marriage has
///    `|c_i| >= clr`, so its child-drop lands at least `gap/2` outside its
///    co-spouse card rather than through it.
///
/// The natural cumulative placement is centred (symmetric about 0). Each
/// child-bearing centre that lands inside the forbidden band `(-clr, clr)`
/// is then nudged out to the nearer edge (`+clr` for the lone middle of an
/// odd N, which sits exactly on the hub column), and the fan re-packs
/// outward from the centre so spacing is preserved; finally the outer two
/// centres are mirrored so the hub stays at their midpoint (the inner
/// marriages may sit asymmetrically — only the *outer* pair pins the hub).
/// For N=2 with one childless side this pushes the child-bearing co-spouse
/// (and its mirror) out to `±clr`; for the odd-N middle it splays the
/// outer co-spouses wider in exchange for clearing the middle child.
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

    // Pivot = first centre at or right of the hub column. Everything from
    // the pivot rightward packs outward to the right; everything left of
    // it packs outward to the left. A child-bearing centre inside the band
    // is the only thing that triggers re-packing; for the corpus that is
    // the lone middle of an odd N (which lands exactly on the hub column),
    // or the child-bearing side of an N=2 pair whose natural half-spacing
    // is narrower than `clr`.
    let pivot = c.iter().position(|&v| v >= 0.0).unwrap_or(n);

    // Right of (and including) the pivot: sweep outward, holding each
    // child-bearing centre at >= clr and each centre >= its inner
    // neighbour plus that gap's spacing.
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
    // Left of the pivot: mirror sweep outward to the left.
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

    // Pin the hub to the midpoint of the outer two centres: mirror the end
    // pair to the wider of the two. Inner marriages keep their swept
    // positions (they already clear the band and respect spacing).
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
        // Index canonical-card top-centres by person_id for the edge
        // router's fall-through lookup. Past-marriage spouse-ghosts intentionally
        // don't land here (ghosts are mute: the ghost is mute and the child edge
        // attaches to the bar). Past-intimacy child-ghosts get their own
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
        generation: slot.generation,
        gender: slot.gender,
        family: slot.family.clone(),
        given: slot.given.clone(),
        born: slot.born.as_ref().map(fmt_date),
        died: slot.died.as_ref().map(fmt_date),
    });
}

/// Format an [`ExportedDate`] back into its source `~YYYY[-MM[-DD]]`
/// form: the circa marker (if any) prefixed to the value (whose
/// component count already encodes year / month / day precision).
/// Carried onto the positioned shapes display-ready so the emitter
/// surfaces dates as `data-*` attributes without re-deriving (ADR-0021).
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
        // The absorb rule (ADR-0018): nested birth-family marriages are positioned as
        // additional Walker roots, so every render edge's marriage id
        // is in `bar_centers`. The map carries each marriage's
        // child-attach anchor — the gap midpoint of the monogamy
        // marriage edge, or the polygamy marriage-edge midpoint below the
        // hub (ADR-0020) — so the parent-child edge routing needs no
        // per-shape branch.
        let &(bar_cx, bar_by) = bar_centers
            .get(&edge.marriage_id)
            .expect("every render edge's marriage must have a positioned anchor");
        // Past intimacies emit ghosts: when a child has a child-ghost (past-adoption or
        // past-bio) at this marriage's children row, the parent-child
        // edge attaches to the local ghost rather than the canonical
        // card — the ghost is materialised precisely to be the local
        // anchor. Resolving via that ghost map is exactly the
        // `is_past` predicate: the edge terminates on a past-intimacy
        // child-ghost.
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

    // Default layout metrics that drive the corpus (see `metrics.rs`):
    // card_width 160, sibling_gap 32 → clr = (160 + 32) / 2 = 96, and a
    // leaf children block is one card wide (160).
    const GAP: f64 = 32.0;
    const CLR: f64 = 96.0;
    const LEAF: f64 = 160.0;

    /// N=2 with a childless co-spouse and a single-child co-spouse
    /// (example 04). The child-bearing side's natural half-spacing is
    /// narrower than `clr`, so both centres are pushed out to `±clr` —
    /// reproducing the byte-identical example-04 geometry (co-spouses at
    /// `±2*clr = ±192` about the hub).
    #[test]
    fn n2_one_childless_one_child_clears_to_clr() {
        let centers = fan_children_centers(&[0.0, LEAF], &[false, true], GAP, CLR);
        assert_eq!(centers, vec![-CLR, CLR]);
    }

    /// N=3, one child each (example 15). The middle marriage would land
    /// on the hub column; it is nudged to `+clr` and the outer pair
    /// splays to `±(clr + spacing) = ±288`, keeping the hub at their
    /// midpoint. Co-spouses derive as `2*center`: `[-576, +192, +576]`.
    #[test]
    fn n3_middle_nudged_off_hub_outer_splays() {
        let centers = fan_children_centers(&[LEAF; 3], &[true; 3], GAP, CLR);
        assert_eq!(centers, vec![-288.0, CLR, 288.0]);

        // Every child-bearing centre clears the band, and the hub stays
        // at the midpoint of the outer two.
        for &c in &centers {
            assert!(c.abs() >= CLR, "center {c} inside forbidden band");
        }
        assert_eq!((centers[0] + centers[2]) / 2.0, 0.0);
    }

    /// N=4, one child each: the inner pair straddles the band at
    /// `±spacing/2 = ±96 = clr`, so nothing is nudged and the layout is
    /// symmetric.
    #[test]
    fn n4_inner_pair_straddles_band_no_nudge() {
        let centers = fan_children_centers(&[LEAF; 4], &[true; 4], GAP, CLR);
        assert_eq!(centers, vec![-288.0, -CLR, CLR, 288.0]);
    }

    /// Odd N=5: the lone middle is nudged off the hub column and the fan
    /// re-packs outward, with the hub still centred and every centre clear
    /// of the band.
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

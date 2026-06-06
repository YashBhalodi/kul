# ADR 0018 — Canonical layout algorithm: Walker port and the shared generation grid

**Status:** Accepted
**Date:** 2026-06-07
**Deciders:** owner

## Context

[`RenderShape`](./0017-render-shape-schema-and-versioning.md) is structural, not positional. Turning it into a `PositionedShape` — cards, marriage edges, and edge segments in absolute pixels — is `kul-layout`'s job ([ADR-0016](./0016-visualization-pipeline-crate-boundaries.md)). That job has two parts:

1. **Tidy-tree positioning within one host-lineage tree.** Place a tree of cards so siblings do not overlap and parents centre over their children — the classical descendency-tree arrangement.
2. **Arranging components on a shared canvas.** Every host-lineage tree is its own component; a document with multiple components places them left-to-right in source order on a shared global generation grid so kin-symmetric persons across components read on the same row.

## Decision

### Walker's algorithm, wrapped by a canonical-pattern adapter

`kul_layout::walker` is a Reingold–Tilford–Walker port (O(n)). It takes an internal tree derived from `RenderShape` and emits preliminary x positions with sibling-subtree collision avoidance; it is small (~200 lines) and owns its own state (preliminary x, modifier, threads, ancestors). `kul_layout::adapter` wraps it for the canonical pattern — marriage edges, ghost slots at the host's birth-family position, generation rows, edge routing, the polygamy fan ([ADR-0020](./0020-polygamy-hub-and-fan.md)). Walker is implemented from day one even where the corpus does not exercise sibling-subtree collisions; `tests/walker.rs` covers the algorithm's contract on hand-fabricated trees independent of corpus content, so an example that does collide lands without algorithmic work.

### Edge routing is orthogonal for every edge

Every edge — birth, adoption, and marriage — routes with the same orthogonal right-angle geometry and the same attachment points: it originates at the marriage edge's midpoint, drops to a horizontal bus at `card_top - config.bus_drop`, and drops again to the child card's top-midpoint. This matches the classical descendency-tree convention and keeps the whole diagram on one routing pattern.

This holds whether or not the child is a structural descendant of the marriage. A within-family cousin marriage — where the joining cousin's birth edge connects back to a sibling marriage already in the tree — emits the **same** geometry, attachment points, dashes, and stroke as a standard descendency edge; it is not distinguished in any way.

> **History.** Earlier revisions carried a `PositionedEdge::routing` discriminator (`InTree` / `CrossTree`) emitting `kul-edge--in-tree` / `kul-edge--cross-tree` classes as a future re-theming hook. Because both variants were byte-identical and no surface ever consumed the distinction, the enum and its field were removed (closes #156); the routing is one geometry, full stop. A future re-theming need would re-introduce a discriminator as a `data-*` attribute under the [ADR-0021](./0021-language-properties-plumb-to-svg.md) plumb-through convention rather than as a CSS modifier class.

### Components on a shared generation grid

Each host-lineage tree is a component. The adapter lays each component out independently with Walker, producing per-component bounding boxes, then packs components left-to-right on the canvas in source order — separated by `sibling_gap` — and aligns each card's y on a **shared global generation grid**: row R is at the same canvas y in every component, so a person at data-level generation R in one component reads on the same row as a person at generation R in another. Kin-symmetric persons across an inter-family marriage (a joining spouse's bio-family component and the host's component) therefore read as the same generation directly, without any layout-side cascade.

Card placement is a one-line function of data-level generation: a card sits at `row_top(card.slot.generation)`. A marriage bar sits at `row_top(max(spouses.generation))`. The host card and the joining card both occupy the bar's row. Each marriage's children sit at `bar.row + 1`. The polygamy fan ([ADR-0020](./0020-polygamy-hub-and-fan.md)) seats its children two rows below the hub (the co-spouse row sits between), per the geometry that ADR specifies; the row math is direct, not derived through any intermediate layout vocabulary.

`LayoutConfig` is `Default`-only in v1.

## Consequences

- **Walker is the single positioning authority within a component.** Every node reaches `finish()` with a Walker-assigned x; the adapter performs no ad-hoc post-pass, and the bounding-box sweep flows over every positioned node unchanged.
- **Kin-symmetric ancestors across an inter-family marriage land on one row** because the generation grid is shared globally; no layout-side row computation reinterprets data-level generation.
- **Each component's tidy-tree layout is local.** A document with N components performs N Walker passes whose bounding boxes are then packed left-to-right; intra-component positioning is independent of inter-component packing.
- **Cross-family kinship reads through ghost + name pairing, not geometry.** A joining spouse's bio family is its own component carrying a past-bio child-ghost ([ADR-0019](./0019-ghost-model-and-bio-anchor.md)); the reader reconciles the cross-family link through the shared name on canonical card and ghost, in source order.

## Anti-suggestions (do not re-propose)

- **"Inline a joining spouse's bio family beside the host tree as a nested sub-tree."** That re-introduces multi-rooted components, the absorb-rule complexity, and a second routing path for cross-tree edges. The shared generation grid plus name pairing carries the cross-family kinship cleanly with one routing path and one component primitive.
- **"Add a per-component visual-row cascade that shifts cards off their data-level generation."** Data-level generation is the canvas row. A cascade is layout vocabulary that re-derives placement from local descendant depth; the shared global grid already aligns kin-symmetric persons across components, so the cascade has no work to do.
- **"Push placement vocabulary into `RenderShape` / `CardSlot.generation`."** Data-level generation stays structural so non-layout consumers (validator, exports, hover queries) stay agnostic to canvas arithmetic. The layout pass reads it as the row index directly.
- **"Use an external layout library (dagre, elkjs, cytoscape)."** None speak the canonical pattern's vocabulary — ghost slots, the shared global generation grid, the polygamy fan. A custom Walker port is small (~200 lines); a dependency to avoid it is poor value.
- **"Let cross-family edges rise out of their own bar."** Letting a bus rise above the bar (negative y from the bar's perspective) breaks "parents above children" and clips outside the SVG viewBox. Past-bio child-ghosts terminate cross-family edges locally; no cross-component edge is drawn.

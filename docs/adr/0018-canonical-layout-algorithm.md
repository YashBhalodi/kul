# ADR 0018 — Canonical layout algorithm: Walker port, sibling-root packing, and the visual-row cascade

**Status:** Accepted
**Date:** 2026-05-25
**Deciders:** owner

## Context

[`RenderShape`](./0017-render-shape-schema-and-versioning.md) is structural, not positional. Turning it into a `PositionedShape` — cards, marriage edges, and edge segments in absolute pixels — is `kul-layout`'s job ([ADR-0016](./0016-visualization-pipeline-crate-boundaries.md)). That job has three parts, each a distinct positioning sub-problem:

1. **Tidy-tree positioning.** Place a tree of cards so siblings do not overlap and parents centre over their children — the classical descendency-tree arrangement.
2. **Placing a joining spouse's nested birth family.** The absorb rule nests an unrelated joining spouse's birth family adjacent to the host tree, recursively. Layout must decide *where* that sub-tree goes and how the cross-tree edge back to it routes.
3. **Assigning each cluster a canvas row.** `RenderShape`'s generation is data-level kinship depth, which is stable for downstream consumers. It is not always the right *canvas* row: nesting depth and cross-family ancestor alignment both pull a cluster off its data-level generation.

## Decision

### Walker's algorithm, wrapped by a canonical-pattern adapter

`kul_layout::walker` is a Reingold–Tilford–Walker port (O(n)). It takes an internal tree derived from `RenderShape` and emits preliminary x positions with sibling-subtree collision avoidance; it is small (~200 lines) and owns its own state (preliminary x, modifier, threads, ancestors). `kul_layout::adapter` wraps it for the canonical pattern — marriage edges, ghost slots at the host's birth-family position, generation rows, edge routing, nested-family packing, the polygamy fan ([ADR-0020](./0020-polygamy-hub-and-fan.md)). Walker is implemented from day one even where the corpus does not exercise sibling-subtree collisions; `tests/walker.rs` covers the algorithm's contract on hand-fabricated trees independent of corpus content, so an example that does collide lands without algorithmic work.

### Edge routing is orthogonal for every edge

Every edge — birth, adoption, and marriage — routes with the same orthogonal right-angle geometry and the same attachment points: it originates at the marriage edge's midpoint, drops to a horizontal bus at `card_top - config.bus_drop`, and drops again to the child card's top-midpoint. This matches the classical descendency-tree convention and keeps the whole diagram on one routing pattern.

This holds whether or not the child is a structural descendant of the marriage. The within-family cross-edge of a cousin marriage — where the joining cousin's birth edge connects back to a sibling marriage already in the rendering context — emits the **same** geometry, attachment points, dashes, and stroke as a standard descendency edge; it is not distinguished in any way.

> **History.** Earlier revisions carried a `PositionedEdge::routing` discriminator (`InTree` / `CrossTree`) emitting `kul-edge--in-tree` / `kul-edge--cross-tree` classes as a future re-theming hook. Because both variants were byte-identical and no surface ever consumed the distinction, the enum and its field were removed (closes #156); the routing is one geometry, full stop. A future re-theming need would re-introduce a discriminator as a `data-*` attribute under the [ADR-0021](./0021-language-properties-plumb-to-svg.md) plumb-through convention rather than as a CSS modifier class.

### Nested birth families pack as additional Walker roots

When the absorb rule nests an unrelated joining spouse's birth family, that sub-tree becomes an **additional Walker root**. `kul-render` has already built the data (`MarriageBar::joining_nested_birth_family`, with within-family termination pre-applied) and decided *which* families nest; the adapter walks what it is handed and pushes each nested root onto the Walker root array in DFS pre-order — before walking that marriage's children. Walker's existing multi-root pass places it left-to-right after the host tree's bounding box, separated by `sibling_gap`; a grand-nested sub-tree (a nested family whose own joining spouse carries a birth family) packs adjacent to its parent nested in declaration order, satisfying both the absorb rule's recursion and source-order arrangement. Every edge into a nested bar routes through the same one orthogonal geometry as the cousin case, because the joining spouse is excluded from the nested sub-tree's children.

This is the durable arrangement, not a stopgap. In a dynasty-shaped host tree the joining spouse's birth-family cluster sits to the right of the host tree's *entire* bounding box, so the cross-tree edge bus can run long and horizontal. That length is visually acceptable and is paid back by an implementation that reuses Walker's one collision-avoidance pass and routes every edge — within-tree and cross-tree — through one code path.

### The visual-row cascade

Each adapter cluster carries a `visual_row` (an `f64`, leaving room for future fractional-row primitives), distinct from the data-level generation. It is computed bottom-up during the adapter's single DFS, which finishes every nested root and every child before the host folds them in:

```text
visual_row(cluster) = max(
    host_card.slot.generation,                            // data-level floor
    1 + max(visual_row(nested)) over nesting marriages,   // push-down
    min(visual_row(child)) - 1,                           // descendant-pull
)
```

- The **data-level floor** keeps every non-nesting, leaf-ward cluster on its data-level generation, so a document with no nesting positions exactly as the tidy tree would.
- The **push-down clause** drops a cluster one row below its deepest nested sub-tree, so a cross-tree birth edge into that sub-tree always flows top-to-bottom however many nesting layers separate the joining spouse's birth family from the host tree's founders.
- The **descendant-pull clause** reads "this cluster sits one row above its closest (smallest-row) descendant," pulling host-tree ancestors *down* to align with the deeper side of an inter-family marriage so kin-symmetric ancestors share a row.

`finish()` reads `visual_row` (not the data-level generation) for each cluster's `row_top` and for the bounding box. The data-level generation on `RenderShape` is untouched: the cascade is layout vocabulary that never leaks into the render shape. A polygamy fan ([ADR-0020](./0020-polygamy-hub-and-fan.md)) seats its children two rows below the hub (the co-spouse row sits between), so a hub's descendant-pull term reads `min(child.visual_row) - 2.0`; a deep sub-tree under one of its children pulls the whole fan down in lockstep through this same cascade with no special-casing.

`LayoutConfig` is `Default`-only in v1; the cascade and packing use one Walker port for every corpus example.

## Consequences

- **Walker is the single positioning authority.** Every node — host-tree or nested — reaches `finish()` with a Walker-assigned x; the adapter performs no ad-hoc post-pass, and the bounding-box sweep flows over every positioned node unchanged.
- **Cross-tree birth edges always flow top-to-bottom**, regardless of nesting depth, and kin-symmetric ancestors across an inter-family marriage land on one row, so the two ancestor stacks read as the same generation. Row 0 is "the deepest ancestor in the canvas after alignment," which is the data-level founders only when no sibling-root sub-tree reaches deeper.
- **Recursion is free.** A grand-nested sub-tree recurses through the same `build_person` traversal and the same root array; no separate code path. Examples with no nesting position identically to the plain tidy tree because the cascade collapses to the data-level floor.
- **Two cascades, two responsibilities.** `kul-render`'s data-level generation computes structural depth; `kul-layout`'s visual-row cascade computes canvas placement. Neither pass knows about the other.

## Anti-suggestions (do not re-propose)

- **"Joining-spouse-anchored placement with a per-row overlap sweep."** Anchoring each nested cluster to the joining spouse's right edge produces shorter cross-edge buses only when the host tree is shallow; for a dynasty-shaped host tree the sweep has to shift the cluster right anyway to clear deeper descendants, undoing the benefit and introducing a second positioning pass that duplicates Walker's collision avoidance. The long horizontal cross-edge bus is visually acceptable and is not a defect to refine. **Do not re-propose this.**
- **"Position the nested sub-tree below the joining spouse's row instead of beside it."** That breaks the absorb rule's "adjacent at the connection point" arrangement and makes the joining spouse's row taller than the host row, breaking the uniform generation-row band the emitter assumes.
- **"Add a second layout algorithm specialised for nesting and dispatch on `LayoutConfig`."** `LayoutConfig` is a forward-compatibility seam, not a request for alternative algorithms. One Walker port covers every corpus example; an alternative algorithm appears only when an example demonstrably cannot be expressed in the current one.
- **"Re-derive the nesting set during layout (read `RenderShape` and recompute which families nest)."** `kul-render` already materialised `joining_nested_birth_family` with within-family termination applied; re-deriving here would put that termination in two places. The adapter walks what it is handed.
- **"Push `visual_row` into `RenderShape` / `CardSlot.generation`."** Data-level generation must stay structural so non-layout consumers (validator, exports, hover queries) stay agnostic to canvas arithmetic. The two passes describe different things — structural depth vs canvas placement — that only coincide in the no-nesting case.
- **"Let cross-tree edges rise out of their own bar instead of shifting the cluster down."** Letting the bus rise above the bar (negative y from the bar's perspective) breaks "parents above children" and clips outside the SVG viewBox. Visual correctness outranks snapshot-diff hygiene.
- **"Add a `max(joining_slot.generation)` clause to the cascade."** In a nesting case the joining spouse always sits at the bottom row of their own birth-family sub-tree, so the clause is dominated by `1 + nested.visual_row` and adds nothing; in a within-family cross-edge (cousin / uncle-niece) the joining spouse's data-level generation can exceed the host's, and the clause would over-shift the host downward, contradicting the absorb rule's "the host does not relocate."
- **"Make the descendant-pull clause `max(child.visual_row) - 1` instead of `min`."** A host with two branches of mismatched depth has children at different rows; `max - 1` over-shifts the host below its *closer* child, breaking parent-above-child on that branch. `min - 1` preserves the invariant on every descendant.
- **"Treat the host tree's founders specially so they never cascade below row 0."** Pinning founders to row 0 re-introduces the cross-family ancestor-misalignment the descendant-pull clause exists to remove. Founders shifting down when the other side has deeper ancestry is the intended behaviour.
- **"Add a top-down `parent_visual_row + 1` propagation pass."** A host carrying one nesting marriage and one non-nesting marriage of mismatched depth would expose a corner the bottom-up cascade does not cover. No corpus example demands it; the fix (one additive top-down pass) lands when an example surfaces the case, not before.

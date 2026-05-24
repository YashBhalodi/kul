# ADR 0023 — kul-layout assigns visual rows via `max(host.gen, 1 + nested.visual_row)` bottom-up

**Status:** Accepted
**Date:** 2026-05-24
**Deciders:** owner
**Parent:** [ADR-0018](./0018-kul-layout-crate-boundary.md) — `kul-layout` is a separate crate
**Related:** [ADR-0017](./0017-render-shape-schema-and-versioning.md), [ADR-0022](./0022-p6-layout-sibling-root-packing.md)

## Context

`kul-render`'s data-level generation cascade — `compute_generations`
in `crates/kul-render/src/build.rs` — relaxes the rule
`child = max(canonical_family_spouses) + 1` to a fixpoint over the
project's persons. The resulting `CardSlot::generation` is structural
per [ADR-0017](./0017-render-shape-schema-and-versioning.md): it
describes how deep a person sits in the data-level kinship cascade,
not where the renderer must place them on the canvas.

`kul-layout`'s adapter was reading `card.slot.generation` directly as
each cluster's visual row (`row_top = offset_y + node.generation *
row_height`). For the corpus through example 13 this works — every
nested sub-tree's host is itself a generation-0 founder, so the host
tree's data-level rows happen to line up with the nested sub-tree's
visual top row.

[ADR-0022](./0022-p6-layout-sibling-root-packing.md) introduced P6
sibling-root packing: each `joining_nested_birth_family` becomes an
additional Walker root. Once P6 starts recursing (a nested sub-tree's
own joining spouse carries *their* birth family — "grand-nesting"),
two visual defects surface in the adapter:

1. The grand-nested sub-tree renders on the same row as the joining
   spouse who carries it instead of one row above. Its cross-tree
   birth edge bus lands at negative y and clips above the SVG
   viewBox.
2. Canonical descendants of the host tree inherit the data-level
   cascade through the joining spouse's generation, leaving an empty
   intermediate row between the marriage bar and the descendant
   (Kiran lands at data-level row 3 even though `m_alice_bob`'s
   visual row is 1, so row 2 stays empty).

The data-level cascade can't fix this on its own: data-level
generation must stay structural and stable so consumers downstream of
`RenderShape` (validator, exports, hover queries) keep agreeing on
"who is in which generation." Layout policy — *how* to place the
generations on the canvas — is the layer that needs to react to P6
nesting depth.

## Decision

`kul-layout::adapter::Node` carries a `visual_row` field per cluster,
distinct from `card.slot.generation`. It's computed bottom-up during
the existing DFS in `build_person`:

```
visual_row(cluster) = max(
    host_card.slot.generation,
    1 + max(visual_row(nested)) over m in cluster.hosted_marriages
                                where m.bar.joining_nested_birth_family is Some,
)
```

`build_person` already recurses into each nested root before the host
finishes, so every nested's `visual_row` is final by the time the
host folds it in. For `PersonLeaf`, `Orphan`, and `PersonHost` with
no nesting-bearing marriages the formula collapses to
`host_card.slot.generation`, so every non-nesting cluster's row stays
byte-identical with the pre-ADR-0023 snapshots. `finish()` reads
`node.visual_row` instead of `node.generation` when computing
`row_top` and the bounding-box's `max_gen`.

`RenderShape::CardSlot.generation` and `MarriageBar::generation` keep
their data-level semantics per ADR-0017. The visual row is layout
vocabulary, not domain vocabulary; it lives in `kul-layout` and never
leaks into the render shape.

## Consequences

- **Cross-tree birth edges always flow top-to-bottom.** A joining
  spouse's cluster sits one visual row below the marriage bar that
  birthed them, regardless of how many P6 layers separate the
  spouse's birth family from the host tree's founders. The
  bus-and-drop geometry from ADR-0018 unchanged; what changed is the
  row each endpoint anchors at.
- **The host tree cascades down to make room for nesting depth.** In
  `examples/14-grand-nested-inter-family/`, Alice's cluster sits at
  visual row 2 (rather than data-level row 1) because Bob's nested
  family sits at row 1 to make room for the grand-nested family at
  row 0. Kiran follows at visual row 3.
- **Examples 01–13 snapshots stay byte-identical.** The formula
  collapses to `host_card.slot.generation` when no marriage carries
  a `joining_nested_birth_family`, so the pre-ADR-0023 corpus
  regenerates unchanged. The new `example_14_grand_nested_inter_family`
  is the first corpus exerciser of grand-nesting.
- **`compute_generations` retains a single responsibility.** The
  data-level cascade computes structural depth; the layout cascade
  computes visual placement. Neither pass has to know about the
  other.

## Anti-suggestions (do not re-propose)

- **"Push `visual_row` into `RenderShape` / `CardSlot.generation`."**
  Rejected. ADR-0017 keeps `RenderShape` structural and
  non-positional so non-layout consumers (validator, exports, hover
  queries, the WASM `exportGraph` envelope) stay agnostic to canvas
  arithmetic. Layout policy belongs in `kul-layout`. The two passes
  describe two different things — structural depth vs. canvas
  placement — that only happen to coincide in the no-nesting case.
- **"Only shift nested Walker roots; let cross-tree edges rise out of
  their own bar (Option A)."** Rejected. Letting the bus rise above
  the bar (negative y from the bar's perspective) breaks the
  "parents above children" invariant for cross-tree birth edges and
  would clip outside the SVG viewBox. User-facing visual correctness
  is the north star; snapshot-diff hygiene (the appeal of "fewer
  snapshots regenerate") does not outweigh visibly-broken geometry.
- **"Add a third clause `max(joining_slot.gen)` to the formula."**
  Rejected. In every P6 case the third clause is dominated by
  `1 + nested.visual_row` (the joining spouse always sits at the
  bottom row of their own birth-family sub-tree), so it adds no
  information. In P11 within-family cross-tree marriages
  (cousin / uncle-niece) the joining spouse's data-level generation
  can exceed the host's, and adding the clause would over-shift the
  host cluster downward — contradicting P11's rule that the joining
  spouse's canonical card moves adjacent to the host without
  relocating the host. The two-clause formula is the right
  generalisation.
- **"Add a top-down `parent_bar_visual_row + 1` pass for multi-marriage
  mixed nesting."** Deferred — not rejected. A host carrying two
  concurrent marriages of mismatched nesting depth (one with a P6
  nested family, one without) would expose a corner where the
  bottom-up formula correctly raises the host's `visual_row` but
  doesn't push the non-nesting marriage's descendants into the same
  shifted row. No corpus example currently demands this; the fix
  (one extra top-down pass that propagates the host's `visual_row`
  to each marriage's descendant clusters) is additive and can land
  when a corpus example surfaces the case.

# ADR 0024 — kul-layout extends `visual_row` to pull host-tree ancestors toward their closest descendant

**Status:** Accepted
**Date:** 2026-05-24
**Deciders:** owner
**Parent:** [ADR-0023](./0023-kul-layout-visual-row-cascade.md) — `kul-layout` cascades `visual_row` bottom-up
**Related:** [ADR-0017](./0017-render-shape-schema-and-versioning.md), [ADR-0018](./0018-kul-layout-crate-boundary.md), [ADR-0022](./0022-p6-layout-sibling-root-packing.md)

## Context

[ADR-0023](./0023-kul-layout-visual-row-cascade.md) introduced
`Node::visual_row` and the bottom-up cascade

```
visual_row(cluster) = max(
    host_card.slot.generation,
    1 + max(visual_row(nested)) for m in cluster.hosted_marriages
                                where m.bar.joining_nested_birth_family is Some,
)
```

The nesting clause pushes a cluster *down* on the canvas to make room
for a P6 (grand-)nested birth-family sub-tree on its joining side.
That clause alone is enough to keep every cross-tree birth edge
flowing top-to-bottom.

`examples/14-grand-nested-inter-family` exposes a follow-on defect.
Bob's side carries two layers of nesting (Bob's birth family +
Lata's birth family), so on the right of the canvas the deepest
ancestors sit at row 0, Bob's parents at row 1, and Bob himself at
row 2. The host tree, however, has no P6 nesting at all: the nesting
clause is empty on every host-side cluster, so Rajesh + Saroj — Kiran's
host-side grandparents — stay at data-level row 0, while Mahesh + Lata
— Kiran's birth-side grandparents — sit at row 1.

Rajesh + Saroj and Mahesh + Lata are kin-symmetric grandparents from
Kiran's perspective. Rendering them one row apart leaves dead space
above the host tree and makes the inter-family marriage's two
ancestor stacks look like they describe different generations. The
data-level cascade is correct (Rajesh + Saroj genuinely *are* one
generation higher than Mahesh + Lata structurally); the layout
cascade is what needs to align the two ancestor stacks on the canvas.

The fix is symmetric to ADR-0023's push-down: instead of pushing a
host down from below (via its nested), pull it down from above (via
its descendants). The bottom-up DFS already finishes the descendants'
`visual_row` before the host's fold runs, so the rule needs no extra
pass.

## Decision

Extend ADR-0023's formula with a third clause:

```
visual_row(cluster) = max(
    host_card.slot.generation,
    1 + max(visual_row(nested)) for nesting marriages,
    min(visual_row(child)) - 1,
)
```

The descendant-pull clause reads as: *"this cluster sits one row
above its closest (shallowest, smallest-row-number) descendant."*
Combined with ADR-0023's two clauses, the host tree's ancestors
cascade *down* to align with whichever side of an inter-family
marriage has the deeper nesting stack.

In `kul-layout::adapter::build_person`, the existing fold gains a
`child_min_row` term alongside `nested_max_row`:

```rust
let nested_max_row = nested_root_indices
    .iter()
    .map(|&i| self.nodes[i].visual_row)
    .max();
let child_min_row = children.iter().map(|&i| self.nodes[i].visual_row).min();
let visual_row = match (nested_max_row, child_min_row) {
    (Some(n), Some(c)) => host_generation.max(n + 1).max(c.saturating_sub(1)),
    (Some(n), None)    => host_generation.max(n + 1),
    (None,    Some(c)) => host_generation.max(c.saturating_sub(1)),
    (None,    None)    => host_generation,
};
```

`saturating_sub` is safe: a child at row 0 implies `host_gen` would
have to be ≤ 0, and `host_gen: u32` already enforces that floor
through the outer `max`. The fold stays a single pass and adds no
new field or struct.

`RenderShape::CardSlot.generation` and `MarriageBar::generation`
keep their structural data-level semantics per ADR-0017. The
descendant-pull is layout vocabulary, not domain vocabulary.

## Consequences

- **Kin-symmetric ancestors align on the same visual row.** In
  `examples/14`, Rajesh + Saroj (host-side founders) shift from row 0
  to row 1 so they sit alongside Mahesh + Lata, the birth-side
  grandparents of Kiran. Row 0 now holds only Ramprasad + Sunita +
  `m_lata_parents`, the deepest ancestor in the canvas.
- **Examples 01–13 snapshots stay byte-identical.** For every cluster
  in those examples, `min(child.visual_row) - 1` either is undefined
  (no children) or equals `host_card.slot.generation` (no nesting
  amplifies a descendant's row past its data-level position). The
  three-clause formula therefore collapses to the ADR-0023 result.
  Only `examples/14` regenerates.
- **Row 0 changes meaning slightly.** Pre-ADR-0024, row 0 was "the
  data-level founders" by construction. Post-ADR-0024, row 0 is "the
  deepest ancestor in the canvas after alignment" — founders shift
  down when a sibling-root sub-tree reaches deeper. The shift is
  intentional and is what the ADR exists to achieve.
- **The single-pass DFS in `build_person` is preserved.** Both
  `nested_root_indices` and `children` are populated by recursing
  before the host's fold, so the new clause adds one line to the
  existing fold without any second pass.

## Anti-suggestions (do not re-propose)

- **"Make the new clause `max(child.visual_row) - 1` instead of
  `min`."** Rejected. A host carrying two marriages with mismatched
  descendant depths (e.g. one P6-nesting branch and one straight-line
  branch) can have children at different visual rows. `max - 1`
  would over-shift the host below its *closer* child, breaking the
  parent-above-child invariant for that branch. `min - 1` is the
  only choice that preserves the invariant on every descendant.
- **"Run a separate top-down pass after the bottom-up to propagate
  the parent's `visual_row` to children (the ADR-0023 deferred fix)."**
  Out of scope here. ADR-0023's deferred case — a host carrying one
  P6-nesting marriage and one non-nesting marriage, where the host's
  row is raised but the non-nesting marriage's descendants stay at
  their data-level row and collide with the host — is a different
  bug. No corpus example currently demands it; it remains deferred
  until one does. The descendant-pull rule from this ADR is
  orthogonal: it pulls parents toward children, not the other way.
- **"Treat the host tree's founders specially so they never cascade
  below row 0."** Rejected. The whole point of this ADR is to let
  founders shift down when the inter-family marriage's other side
  has deeper ancestry. Pinning founders to row 0 would re-introduce
  the symmetry break this ADR exists to remove. The change in what
  row 0 means — from "data-level founders" to "deepest ancestor in
  the canvas after alignment" — is intentional and is the central
  decision of this ADR.

# ADR 0022 — P6 nested birth-family sub-trees pack as additional Walker roots

**Status:** Accepted
**Date:** 2026-05-24
**Deciders:** owner
**Parent:** [ADR-0018](./0018-kul-layout-crate-boundary.md) — `kul-layout` is a separate crate

## Context

[P6](../canonical-ui-pattern.md#p6-recursive-nesting-of-the-joining-spouses-birth-family)
says: when a marriage joins two unrelated families, the joining
spouse's birth family must render as a nested sub-tree adjacent to
the host tree at the joining spouse's connection point, and the rule
applies recursively at every scale. P11 terminates the recursion
when the joining spouse's birth family is already in the current
rendering context (cousin / sibling marriage).

`kul-render` already builds the structural data: every
`MarriageBar` carries `joining_nested_birth_family:
Option<Box<PersonCard>>`, materialised by `build_nested_birth_family`
in `crates/kul-render/src/build.rs` with P11 termination pre-applied
(`compute_main_walk_reachable` does the work). The remaining concern
is purely layout: given that data, where on the canvas does the
nested sub-tree go, and how do the cross-tree edges from the joining
spouse back to their birth-family bar route.

Two placements were on the table:

1. **Sibling-root packing.** Each P6 nested sub-tree becomes an
   additional Walker root in `kul_layout::walker::run`'s `roots[]`
   array, pushed in DFS pre-order during the adapter's traversal of
   the host tree. Walker's existing multi-root pass places it
   left-to-right after the host tree's bounding box, separated by
   `sibling_gap`. Grand-nesteds (P6 recursing on a nested sub-tree's
   own joining spouse) pack adjacent to their parent nested in
   declaration order, satisfying P12.
2. **Joining-spouse-anchored placement with a post-Walker per-row
   overlap sweep.** Position the host tree first with Walker, then
   place each nested cluster so its left edge sits immediately to
   the right of the joining spouse's canonical card, and finally
   sweep every generation row for cluster overlaps and shift the
   nested clusters right as needed.

`PositionedEdge::routing` already encodes the same orthogonal
bus-and-drop geometry for both `InTree` and `CrossTree` variants per
[ADR-0018](./0018-kul-layout-crate-boundary.md), and the adapter's
existing fall-through in `route_edges` routes any `(marriage_id,
child_id)` pair missing from `structural_edges` as `CrossTree`.
P6 cross-tree edges (e.g. the joining spouse's birth edge from their
canonical card at the host bar back to their parents' marriage bar)
satisfy this automatically once both endpoints are positioned, so
the edge-routing layer needs no change.

## Decision

Sibling-root packing.

The adapter's `add_component` pre-registers the host root's node
index in `self.roots` *before* descending into the host tree, and
`build_person` pushes each `joining_nested_birth_family`'s root onto
`self.roots` immediately upon encountering it — before walking that
marriage's children. The DFS pre-order placement keeps grand-nesteds
adjacent to their parent nested, matching P6's recursive semantics
and P12's joining-spouse declaration-order rule.

The previous F8 silent-drop branch in `route_edges` (an early-`continue`
when a render edge's `marriage_id` was not in `bar_centers`, dropping
every cross-tree edge into a nested birth-family bar) is removed.
With nested bars now positioned, every render edge's marriage has a
positioned bar; the existing routing logic emits the edge as
`CrossTree` because the joining spouse is excluded from the nested
sub-tree's children (so `structural_edges` does not contain
`(nested_bar, joining_spouse)`).

This is the **durable** answer, not a v1 stopgap. Long horizontal
cross-edge buses in dynasty-shaped host trees are visually
acceptable and are not a defect to refine later. The aesthetic cost
of "the joining spouse's birth-family cluster sits to the right of
the host tree's *entire* bounding box, rather than hugging the
joining spouse's slot" is paid back by an implementation that
re-uses Walker's existing multi-root collision-avoidance pass and
needs no per-row overlap sweep, and by edge routing that flows
through one consistent code path for both within-tree and cross-tree
edges.

## Consequences

- **Walker is the single positioning authority.** The adapter never
  performs an ad-hoc post-pass; every node — host-tree or nested —
  reaches `finish()` with a Walker-assigned x. The existing
  bounding-box / canvas-sizing sweep in `finish()` flows through
  unchanged because it already iterates every positioned node.
- **Edge routing simplifies.** The F8 silent-drop branch's
  three-line escape hatch becomes a one-line lookup with an
  `.expect`. Cross-tree edges into nested bars route through the
  same `EdgeRouting::CrossTree` path the P11 cousin-marriage case
  already exercises (ADR-0018).
- **Corpus regenerates for nesting examples only.** Examples with no
  P6 nesting (01–10, 12) produce identical positioned shapes; only
  `examples/11-cousin-marriage/` (where the nested branch terminates
  per P11 and the silent-drop branch was previously masking) and the
  new `examples/13-inter-family-marriage/` need fresh snapshots. The
  cousin-marriage snapshot updates because the now-unguarded edge
  routing for `(m_bharat_janaki, nikhil)` and `(m_arjun_indira, maya)`
  was already correct — the silent-drop branch never fired for them
  in practice — but the assertion now lives along a different code
  path.
- **Grand-nesting is free.** A nested sub-tree's own joining spouse
  carrying another birth family recurses through the same
  `build_person` traversal; the grand-nested root pushes onto
  `self.roots` after its parent nested, packing further right. No
  separate code path is needed.

## Anti-suggestions (do not re-propose)

- **"Joining-spouse-anchored placement with a per-row overlap sweep."**
  Considered and rejected. Anchoring each nested cluster to the
  joining spouse's right edge produces shorter cross-edge buses,
  but only when the host tree is shallow. For dynasty-shaped host
  trees the per-row sweep has to shift the nested cluster right
  anyway to avoid overlapping deeper descendants, undoing the
  visual benefit and introducing a second positioning pass that
  duplicates Walker's collision-avoidance discipline. The long
  horizontal cross-edge bus is visually acceptable and is not a
  defect to refine. **Do not re-propose this in a future issue.**
- **"Add a second layout algorithm specialised for P6 nesting and
  dispatch on `LayoutConfig`."** ADR-0018's `LayoutConfig` is a
  forward-compatibility seam, not a request for alternative
  algorithms. One Walker port covers every corpus example;
  alternative algorithms appear only when a corpus example
  demonstrably can't be expressed in the current one.
- **"Position the nested sub-tree below the joining spouse's row
  instead of beside it."** That violates P6's "adjacent at the
  connection point" rule and would make the joining spouse's row
  vertically taller than the host row, breaking the
  generation-row-as-uniform-band invariant the SVG renderer
  assumes.
- **"Reach into `RenderShape` and re-derive the nesting set during
  layout."** The data is already in `MarriageBar::joining_nested_
  birth_family`; re-deriving here would put P11 termination in two
  places. The adapter walks what kul-render hands it.

# ADR 0027 — Fan rendering primitive for polygamy hubs (any N ≥ 2)

**Status:** Accepted
**Date:** 2026-05-25
**Deciders:** owner
**Amends:** [ADR-0021](./0021-render-shape-family-tree-rooted-at-person-card.md) — closes the open structural question on `RenderShape` for concurrent un-ended marriages
**Supersedes:** the scrapped [#163](https://github.com/YashBhalodi/kul/issues/163) proposal (renderer-level relocation of joining-side bars)
**Related:** [ADR-0026](./0026-polygamy-hub-equals-host.md) (R14 — hub = host language invariant), [ADR-0023](./0023-kul-layout-visual-row-cascade.md) + [ADR-0024](./0024-kul-layout-visual-row-descendant-pull.md) (visual-row cascade)
**Closes:** [#165](https://github.com/YashBhalodi/kul/issues/165)

## Context

Before this ADR, the layout adapter rendered polygamy via the same
"hub-and-flanks" cluster shape it used for monogamy: the host card on
the left, then `[bar][joining-card]` for each hosted marriage, all on
one horizontal row. The cluster grew wider as `hosted_marriages.len()`
grew but stayed on one generation row. ADR-0021 reshaped the render
data so the polygamy host's canonical card carries `N` bars in its
`hosted_marriages` field, but it deferred the *visual* primitive for
N≥3 to a follow-up: a single row only affords two adjacencies, so
N≥3 would either have to invent a new shape (one for monogamy, one
for polygamy) or stretch the existing shape past its visual breaking
point.

The follow-up split into two questions, answered together:

1. **Hub-vs-host divergence at the language level.** A pure-host
   polygamy hub coincides with the per-marriage host. Mixed-role and
   pure-join concurrent polygamy diverge. The original #144 attempt
   (scrapped as #163) tried to repair the divergence inside the
   renderer by re-anchoring the joining-side bar to the hub's card.
   ADR-0026 closes that ambiguity at the language level — the new
   R14 validator rule rejects mixed-role and pure-join concurrent
   polygamy, so the fan primitive can assume `hub = host` by
   invariant rather than by repair.
2. **The visual primitive itself.** Two stances were considered:
   - **Stance A** (unified primitive at N≥2). Replace the
     hub-and-flanks shape with a single fan primitive that kicks in
     whenever a person has ≥2 concurrent un-ended marriages,
     including the common N=2 case. Polygamy reads as visually
     distinct from monogamy at every scale.
   - **Stance B** (escalation at N≥3). Keep hub-and-flanks for N=2
     and introduce a new primitive only at N≥3. Two visual shapes
     for "polygamy" depending on the count.

The user's lean toward unified-distinct primitives (per the durable
authoring memory) and P14's scale-invariance argument both point at
Stance A: one primitive that scales naturally is preferable to
graceful escalation, even when the simpler primitive works at the
common case. The shape of polygamy is a *category* the canonical
pattern recognises, not a continuous deformation of monogamy.

## Decision

Replace the existing hub-and-flanks cluster for `hosted_marriages.len() >= 2`
with a **fan-from-top-hub** primitive that scales from N=2 to any N:

- **Hub on top, alone, at row R** (the host's data-level generation).
  The hub card occupies its own cluster; no bars share its row.
- **Co-spouses at row `R + fan_drop_fraction`**, one per hosted
  marriage in declaration order. Each co-spouse cluster is a walker
  child of the hub; the bar for that marriage sits adjacent to the
  co-spouse card on the side facing the hub's vertical axis (the
  first co-spouse renders as `[Spouse][bar]`, every other co-spouse
  as `[bar][Spouse]`, so the bars cluster toward the centerline).
- **Fan connector** — trunk from the hub's bottom-midpoint plus
  horizontal branch spanning the per-bar drops plus per-bar drops
  from the branch to each bar's top-midpoint. Decomposed into
  separate orthogonal segments (one trunk, one branch, N drops) so
  the SVG emitter renders each as its own polyline without retracing
  at the branch / drop intersections.
- **Children of each marriage** are walker children of the
  corresponding co-spouse cluster, so each marriage's children hang
  in their own column below their bar (P9). Half-siblings render in
  distinct sub-trees, one per bar.

Monogamy (`hosted_marriages.len() == 1`) is unchanged: the classical
hub-and-flanks cluster (host card + bar + joining card on one row)
still applies.

`visual_row` in the layout adapter switches from `u32` to `f64`. The
fractional row position the fan needs (`R + fan_drop_fraction`, with
the default `fan_drop_fraction = 0.5`) flows through the descendant-
pull arithmetic from ADR-0023 / ADR-0024 without special-casing —
the cascade formula becomes `max(host.gen, 1.0 + nested.visual_row,
min(child.visual_row) - 1.0)` over floats. For the no-fan corpus
every cluster's `visual_row` is an integer and the snapshots stay
byte-identical.

`RenderShape` **does not change**. ADR-0021 already gave the polygamy
hub the right shape — one canonical `PersonCard` carrying `N` bars
in `hosted_marriages`. The fan is a layout-level concept;
`RENDER_SCHEMA_VERSION` stays at `2` (no additive shape change per
ADR-0017).

`PositionedShape` gains one additive field: `fan_connectors:
Vec<PositionedFanConnector>`, one entry per polygamy hub, each
carrying the trunk-branch-drops segments in absolute pixel
coordinates. `kul-svg` emits each segment as a `<path
class="kul-fan-connector">` with the marriage-bar visual weight
(~3-4px stroke).

## Consequences

- **Examples 04 and 12 snapshots regenerate** across `kul-render`
  (no structural shape change but the test rerun confirms the fan
  is layout-only), `kul-layout`, and `kul-svg`. Both rerender from
  hub-and-flanks to fan-from-top-hub.
- **New example 15 (`15-polygamy-with-three-wives/`)** exercises
  N=3 polygamy with one child per marriage, demonstrating that
  half-siblings render in distinct sub-trees below their own bars.
- **Layout adapter gains a fractional sub-row mechanism.**
  `visual_row` is `f64`; `fan_drop_fraction` is a new
  `LayoutConfig` field defaulted to `0.5`. The visual-row cascade
  per ADR-0023 / ADR-0024 generalises naturally — the fan adds no
  special case to the descendant-pull arithmetic.
- **SVG visual vocabulary gains one element**: `kul-fan-connector`,
  with stroke-width matching the marriage-bar weight (~3-4px),
  distinct from `.kul-edge--birth` (1.5px solid) and
  `.kul-edge--adoption` (1.5px dashed). Surfaces theme the fan
  alongside the bar so the two read as one continuous "hub
  manifold."
- **Composition with P6 is unchanged** (ADR-0025). A co-spouse with
  a declared birth family composes via the existing bio-anchor
  ghost mechanism: ghost-{co-spouse} lives in the bio family's
  children row; the canonical card lives in the fan's joining slot;
  no cross-canvas edge (P10 mute). The fan is just the host
  context — bio anchoring is independent.
- **The `relocated_joining_bars` field never lands.** R14 (ADR-0026)
  eliminates the structural divergence that motivated the field;
  the fan primitive consumes clean input by language invariant.

## Anti-suggestions (do not re-propose)

- **"Hub-and-flanks for N=2 only; new primitive at N≥3."** Considered
  (Stance B); rejected because Stance A wants a uniform primitive so
  polygamy is visually distinct from monogamy at any N — including
  the common N=2 case. Two visual shapes for one category multiplies
  the user's pattern-recognition burden without proportional gain.
- **"Radial / star layout for N≥3."** Considered. Rejected — breaks
  the generation-row convention (P1), P14 scale-invariance, and the
  "children below" convention. A radial layout reads as a different
  kind of diagram, not as a continuation of the descendency tree.
- **"Vertically-extended hub (hub card spans multiple rows)."**
  Considered for any N. Rejected — at N=2 it looks like monogamy
  with a taller hub (no visual distinction); at N≥3 child routing
  from sibling bars collides at the hub's edges. The fan separates
  hub from co-spouses so each marriage's column stays clean.
- **"Compound polygamy block (decorative grouping rectangle around
  the hub + bars + co-spouses)."** Considered as a primitive.
  Rejected as a primitive — it's decoration, not geometry. The fan
  is the geometry; a future render-time toggle could add a
  decorative grouping element on top without committing the
  canonical pattern.
- **"Renderer-level relocation per the scrapped #163."** Considered
  before R14 was on the table; rejected because R14 (ADR-0026)
  eliminates the divergence that motivated relocation. The hub is
  the host by language invariant; the fan's semantics are
  unambiguous because the language guarantees clean input.
- **"Make children hang directly below their bar (snap-to-bar
  centering)."** Considered for tightening the visual association
  between bar and children. Rejected — the walker centres each
  child cluster below its parent cluster, and forcing children to
  snap below the bar would break the walker's collision-avoidance
  guarantees. The edge router already attaches each child's edge to
  its specific marriage bar (P9), so the bar-to-child association
  is unambiguous even when the child's column is offset from the
  bar's exact x-position.
- **"Vary `fan_drop_fraction` per cluster based on local density."**
  Speculative; deferred. A constant fraction (0.5) keeps every
  polygamy hub rendering the same way, which preserves the
  pattern's claim to scale-invariance (P14). Revisit only when a
  corpus example surfaces a density that the constant can't
  accommodate.

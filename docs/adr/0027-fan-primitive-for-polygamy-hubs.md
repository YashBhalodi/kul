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
  The hub card occupies its own cluster; nothing else shares its row.
- **Co-spouses at row R+1**, one per hosted marriage in declaration
  order. Each co-spouse is a single-card walker child of the hub on
  the standard child generation row. There is no marriage bar and no
  R+ε sub-row — the co-spouse is reached by a thick **marriage edge**
  (see below) the same way a birth edge reaches a child.
- **Marriage edge** — one routed `<path>` per hosted marriage, from
  the hub card's bottom-midpoint to the co-spouse, using the **same
  orthogonal hub-bottom → horizontal bus geometry as a birth edge**.
  The only difference from a birth edge is the stroke weight (~3-4px vs
  birth's 1.5px) and the CSS modifier (`kul-edge--marriage`). The fan
  visual emerges from N marriage edges fanning out of the single
  hub-bottom point — it is not a dedicated trunk-branch geometry. The
  edge's terminus depends on whether the marriage has children (below).
- **Children of each marriage** are walker children of the
  corresponding co-spouse, so each marriage's children hang in their
  own column at row R+2 (P9). Half-siblings render in distinct
  sub-trees, one per co-spouse. The geometry differs by whether the
  marriage bears children:
  - A **childless co-spouse** is centred at its cluster's walker centre
    X and the marriage edge drops onto its top-centre, exactly the way
    a birth edge reaches a child.
  - A **child-bearing co-spouse** uses a **junction** on the marriage-
    edge spine. The marriage edge drops a vertical spine at X (the
    co-spouse cluster's walker centre), the co-spouse card sits offset
    to the left of the spine (its cluster width inflated so the walker
    reserves room), connected by a short horizontal stub at the card's
    vertical mid-height — the *junction* `J = (X, row_top + ch/2)`. Each
    child's birth/adoption edge spawns from J on the spine and descends
    straight down, so the thick marriage edge and the thin child edge
    form **one continuous vertical line at X**, with the co-spouse
    hanging off to the left. A single child centred at X reads as a
    direct hub → spine → child lineage; multiple children fan from J
    like a normal bar. The couple → child lineage reads as one
    continuous chain hub → marriage edge → child, with the co-spouse
    card identifying the joining spouse beside the spine.

Monogamy (`hosted_marriages.len() == 1`) is unchanged: the classical
hub-and-flanks cluster (host card + bar + joining card on one row)
still applies, and the marriage still renders as a `<rect class="kul-bar">`
between the two adjacent spouse cards. **Bars are emitted only for
monogamy.**

`visual_row` in the layout adapter is `f64` (carried forward from the
first cut of this ADR; kept for future fractional-row primitives).
Every cluster in the v1 corpus — including every polygamy fan — lands
on an integer row, so the no-polygamy snapshots stay byte-identical.
Descendants of a polygamy hub are visually one row deeper than their
canonical-family `slot.generation` predicts (the co-spouse occupies
the row that, in a monogamy, would host the marriage's children); the
adapter threads a `min_visual_row` floor through the recursive build
so this single extra row cascades through the descendant-pull
arithmetic from ADR-0023 / ADR-0024 without special-casing.

`RenderShape` **does not change**. ADR-0021 already gave the polygamy
hub the right shape — one canonical `PersonCard` carrying `N` bars
in `hosted_marriages`. The fan is a layout-level concept;
`RENDER_SCHEMA_VERSION` stays at `2` (no additive shape change per
ADR-0017).

`PositionedShape` gains **no new field**. The marriage edge reuses the
existing `PositionedEdge` infrastructure: `EdgeKind` gains a `Marriage`
variant alongside `Birth` and `Adoption`, and the layout pass emits
one `PositionedEdge { kind: Marriage, .. }` per hosted marriage of a
polygamy hub. `kul-svg` picks the `kul-edge--marriage` CSS modifier in
its existing `write_edge`; the surface stylesheet sets the heavier
stroke weight.

## Consequences

- **Examples 04 and 12 snapshots regenerate** across `kul-layout` and
  `kul-svg` (the `kul-render` structural shape is unchanged). Both
  rerender from hub-and-flanks to fan-from-top-hub: the polygamy hub
  sits alone on its row, each co-spouse on the next row down reached
  by a thick `kul-edge--marriage` path, with no bar rect and no fan-
  connector segments. In both examples Meera is childless (her marriage
  edge lands on her top-centre, card centred under the spine) and Alice
  bears Priya (Alice's card sits offset left of the spine, the marriage
  edge stubs into a junction at her mid-height, and Priya's birth edge
  continues the spine straight down from that same junction).
- **Example 15 (`15-polygamy-with-three-wives/`)** exercises N=3
  polygamy with one child per marriage, demonstrating that
  half-siblings render in distinct sub-trees below their own
  co-spouse. Every wife bears a child, so all three co-spouses use the
  junction model: each card sits offset left of its spine and its
  child's birth edge continues that spine straight down. The three
  marriage edges fan out of one hub-bottom point (the middle spine to
  Alice routes straight down below the hub, the outer two jog left to
  Meera and right to Diana) — the fan shape with no dedicated trunk.
- **No bars for polygamy marriages.** `PositionedShape.bars` carries
  one rect per monogamy marriage only; polygamy marriages are pure
  edges. A polygamy hub that is *also* a monogamy host in a different
  generation (none in the corpus today) would still emit its
  monogamy bar normally.
- **SVG visual vocabulary unifies on the edge.** `kul-edge--marriage`
  is a sibling modifier of `kul-edge--birth` / `kul-edge--adoption`,
  sharing all the routing CSS (orthogonal right-angle, rounded
  corners, the `kul-edge--in-tree` routing class). The only delta is
  stroke weight (~3-4px) — polygamy reads as distinct from the thin
  birth (1.5px solid) and adoption (1.5px dashed) edges while staying
  inside one coherent edge vocabulary.
- **Composition with P6 is unchanged** (ADR-0025). A co-spouse with
  a declared birth family composes via the existing bio-anchor
  ghost mechanism: ghost-{co-spouse} lives in the bio family's
  children row; the canonical card lives in the fan's co-spouse slot;
  no cross-canvas edge (P10 mute). The fan is just the host
  context — bio anchoring is independent.
- **The `relocated_joining_bars` field never lands.** R14 (ADR-0026)
  eliminates the structural divergence that motivated the field;
  the fan primitive consumes clean input by language invariant.

## Anti-suggestions (do not re-propose)

- **"Trunk + branch + per-bar drop fan connector with a bar per
  co-spouse."** Considered, and *shipped in the first cut of #165*: the
  hub dropped a trunk to a horizontal branch, the branch spanned the
  per-marriage column centres, and a vertical drop landed on each
  marriage bar's top-midpoint; each co-spouse carried its own bar at an
  R+ε sub-row. Rejected on review because the edge-as-path treatment
  unifies polygamy visually with the rest of the edge vocabulary
  (birth, adoption) and removes a bespoke geometry: a polygamy marriage
  is now exactly one `kul-edge--marriage` path routed like a birth
  edge, no separate `PositionedFanConnector` type, no `kul-fan-connector`
  CSS class, no bar rect, no `fan_drop_fraction` sub-row. The fan visual
  emerges from N edges fanning out of the hub, not from a dedicated
  trunk. Do not re-introduce the trunk-branch-drop decomposition or the
  per-co-spouse bar at N≥2.
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
  from sibling co-spouses collides at the hub's edges. The fan
  separates hub from co-spouses so each marriage's column stays clean.
- **"Compound polygamy block (decorative grouping rectangle around
  the hub + co-spouses)."** Considered as a primitive. Rejected as a
  primitive — it's decoration, not geometry. The fan is the geometry;
  a future render-time toggle could add a decorative grouping element
  on top without committing the canonical pattern.
- **"Renderer-level relocation per the scrapped #163."** Considered
  before R14 was on the table; rejected because R14 (ADR-0026)
  eliminates the divergence that motivated relocation. The hub is
  the host by language invariant; the fan's semantics are
  unambiguous because the language guarantees clean input.
- **"Make children hang directly below the hub (snap-to-hub
  centering)."** Considered for tightening the visual association
  between a marriage and its children. Rejected — the walker centres
  each child cluster below its parent cluster (the co-spouse), and
  forcing children to snap below the hub would break the walker's
  collision-avoidance guarantees and re-merge half-siblings into one
  column. Each marriage's children hang below their own co-spouse;
  the birth edge spawns from the junction on the marriage-edge spine
  beside the co-spouse, so the marriage-to-child association is
  unambiguous (P9) and reads as one continuous vertical line.

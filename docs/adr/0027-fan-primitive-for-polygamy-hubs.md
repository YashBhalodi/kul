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

The decision adopts **Approach 1** — children centred at the marriage-
edge midpoint, co-spouse mirrored across that midpoint — governed by
the invariant

```text
children_center_i = (hub_cx + cospouse_cx_i) / 2
equivalently      cospouse_cx_i = 2 * children_center_i - hub_cx
```

so spouses splay toward the wings while each marriage's children gather
toward the centre, directly under the midpoint of that marriage's
(thick) marriage edge.

- **Hub on top, alone, at row R** (the host's data-level generation).
  The hub card occupies its own cluster; nothing else shares its row.
- **Co-spouses at row R+1**, one per hosted marriage in declaration
  order, at the **wing** position `cospouse_cx_i = 2 * children_center_i
  - hub_cx`. There is no marriage bar — the co-spouse is reached by a
  thick **marriage edge** (below) the same way a birth edge reaches a
  child.
- **Children of each marriage** at row R+2, **centred on the marriage-
  edge midpoint** `children_center_i = (hub_cx + cospouse_cx_i) / 2`.
  Each marriage's children forest keeps its full walker layout (nested
  P6 birth families, deeper generations, even recursive polygamy) — the
  fan only pins the forest's *block centre* to the midpoint. Half-
  siblings render in distinct columns, one per marriage, because the
  midpoints are spread apart.
- **Marriage edge** — one routed `<path>` per hosted marriage, from
  the hub card's bottom-midpoint to the co-spouse's top-centre, using
  the **same orthogonal hub-bottom → horizontal bus geometry as a birth
  edge**. The only difference from a birth edge is the stroke weight
  (markedly heavier than birth's 1.5px) and the CSS modifier (`kul-edge--marriage`).
  Its horizontal segment runs at a bus just below the hub; the
  segment's midpoint is `children_center_i`, so that marriage's child
  birth/adoption edges originate there and fan down. The fan visual
  emerges from N marriage edges fanning out of the single hub-bottom
  point — it is not a dedicated trunk-branch geometry. A **childless**
  co-spouse keeps the same wing/mirror placement with an empty children
  block; its marriage edge lands on its top-centre.

**Layout algorithm (children-centre space; hub-local x).** Because the
invariant ties each co-spouse to its marriage's children-centre
(`cospouse_cx_i = 2*C_i - hub_cx`), the fan is laid out by positioning
the **children-centres** directly and deriving the co-spouses from them.
Let `cw = card_width`, `gap = sibling_gap`, `clr = (cw + gap)/2`
(the half-clearance), and `CW_i` the packed width of marriage `i`'s
children forest (0 if childless):

1. Adjacent children-centre spacing: `spacing_i = max((CW_i +
   CW_{i+1})/2 + gap, clr)` (children blocks live half a co-spouse step
   apart, so this keeps neighbouring blocks — and co-spouse cards — from
   overlapping).
2. Cumulative placement `C_1 = 0`, `C_{i+1} = C_i + spacing_i`, then
   re-centre on the midpoint of the ends so the outer two centres are
   symmetric about the hub column (`hub_cx = (C_1 + C_N)/2`, taken as 0
   in the local frame).
3. **Child-drop clearance:** each child-bearing marriage's drop at `C_i`
   must clear that co-spouse's own card, i.e. `|C_i - hub_cx| >= clr`
   (equivalently `|cospouse_cx_i - hub_cx| >= cw + gap`). Any
   child-bearing centre inside the forbidden band `(hub_cx - clr,
   hub_cx + clr)` is nudged out to the nearer edge — for the lone middle
   marriage of an odd N (which lands exactly on the hub column) that is
   `hub_cx + clr` — and the fan re-packs outward from the centre to
   preserve spacing. The outer two centres are then mirrored so the hub
   stays at their midpoint; inner marriages may sit asymmetrically, since
   only the outer pair pins the hub. The N=2-with-one-childless-side case
   pushes the child-bearing co-spouse (and its mirror) out to `±clr`.
4. `cospouse_cx_i = 2*C_i - hub_cx`; shift marriage `i`'s already-laid-out
   children forest so its block centre lands on `C_i`.

Monogamy (`hosted_marriages.len() == 1`) keeps the classical
hub-and-flanks cluster (host card + joining card on one row).

> **Amendment ([#165](https://github.com/YashBhalodi/kul/issues/165) follow-up).**
> The thick marriage edge is now the **unified marriage connector for
> both monogamy and polygamy**, replacing the block-bar rendering. A
> monogamy marriage renders as a thick horizontal `EdgeKind::Marriage`
> edge spanning the inter-card gap between the two adjacent spouse cards
> at the cards' vertical mid-height — `[(left_card_right_edge, mid_y),
> (right_card_left_edge, mid_y)]` — and the couple's children drop from
> its midpoint, exactly where the bar's bottom-midpoint used to anchor
> them. An ended (divorced) monogamy marriage carries the `ended` flag
> through to a `kul-edge--ended` class (translucent), preserving the
> old `kul-bar--ended` treatment. The `MarriageBar` **data** type in
> `kul-render` is unchanged — only the rendered primitive changed from
> block to edge. The `PositionedBar` type, the `PositionedShape.bars`
> field, and the `<rect class="kul-bar">` SVG emission are removed;
> `PositionedEdge` gains an `ended: bool` field (default `false`;
> polygamy marriages are always un-ended per R14, and birth / adoption
> edges never carry it).

**Integration strategy (how the fan composes with Walker).** The hub
is a single Walker **leaf** whose width reserves the full wing-to-wing
extent (symmetric about the hub centre), so a fan packs cleanly against
sibling components and nests inside a larger tree (example 12) without
overlap. Each marriage's children forest is built through the usual
`build_person` recursion (so nested P6 / deeper generations / recursive
polygamy keep their tidy-tree treatment) and measured for its packed
width `CW_i` by a per-forest Walker pass. The forests are attached as
the hub leaf's Walker children purely so the global Walker positions
them (and reserves the contour); their natural positions are then
**overridden** in the projection pass — each forest is rigidly
translated so its block centre lands on its prescribed
`children_center_i`, measured relative to the hub's Walker-assigned x.
Because the prescribed midpoint spread is always ≥ the forests' natural
spacing and the reserved hub width covers the wider wings, the override
never pushes a forest outside the reserved footprint, so it cannot
collide with a neighbouring component.

`visual_row` in the layout adapter is `f64` (carried forward from the
first cut of this ADR; kept for future fractional-row primitives).
Every cluster in the v1 corpus — including every polygamy fan — lands
on an integer row, so the no-polygamy snapshots stay byte-identical.
A polygamy hub's children sit **two** rows below the hub (the co-spouse
row R+1 sits between the hub at R and the children at R+2), so the hub's
descendant-pull clause reads `min(child.visual_row) - 2.0`: a deep P6
sub-tree under a child forest pushes that child below R+2 and pulls the
whole fan down in lockstep through the same ADR-0023 / ADR-0024 cascade,
with no special-casing.

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
  rerender to the Approach-1 fan: the hub sits alone on its row, each
  co-spouse splays to a wing on the next row down, and each marriage's
  children gather under that marriage's edge midpoint two rows below the
  hub. No bar rect, no connector segments. With default metrics, in
  example 04 the hub (Devraj) centres at x = 296, Meera (childless)
  mirrors to 104 and Alice to 488 (both ±192); Priya sits at the
  Devraj–Alice midpoint 392 — exactly the midpoint of Alice's marriage-
  edge horizontal segment, where her birth edge originates — and the
  drop at x = 392 clears Alice's card (left edge 408) by 16px. Example
  12 is the same fan nested one generation down under Ramesh + Sita: the
  hub sits centred directly under the `m_ramesh_sita` bar, and the wide
  hub footprint reserves the wings so the fan doesn't overlap its parent
  cluster (which still renders its monogamy bar).
- **Example 15 (`15-polygamy-with-three-wives/`)** exercises N=3
  polygamy with one child per marriage, and the odd-N middle nudge. The
  hub (Devraj) centres at 680. The middle marriage (Alice) would land on
  the hub column, so its children-centre is nudged off to `hub + clr =
  680 + 96 = 776`, and the outer two marriages splay wider to keep the
  hub centred: co-spouses on row 1 sit at Meera 104 (`hub - 576`), Alice
  872 (`hub + 192`), Diana 1256 (`hub + 576`), and each child sits at its
  marriage-edge midpoint on row 2: Asha at `(680 + 104)/2 = 392`, Priya
  at `(680 + 872)/2 = 776`, Rohan at `(680 + 1256)/2 = 968`. Priya's drop
  at x = 776 now clears Alice's card (left edge 792) by 16px — the same
  `gap/2` clearance the wing co-spouses get — so the middle child no
  longer crosses its co-spouse. The cost is the wider splay of the outer
  co-spouses (±576 instead of the un-nudged ±384). Three thick edges fan
  from one hub-bottom point; the half-siblings render in distinct columns
  per P9.
- **No bars at all.** Per the #165 follow-up amendment above, monogamy
  marriages also render as thick marriage edges, so `PositionedBar` and
  `PositionedShape.bars` are removed entirely. Every marriage — monogamy
  or polygamy — is now a `PositionedEdge { kind: Marriage, .. }`.
- **SVG visual vocabulary unifies on the edge.** `kul-edge--marriage`
  is a sibling modifier of `kul-edge--birth` / `kul-edge--adoption`,
  sharing all the routing CSS (orthogonal right-angle, rounded
  corners, the `kul-edge--in-tree` routing class). The only delta is
  stroke weight (~8.75px in the default preview theme) — a marriage
  reads as distinct from the thin
  birth (1.5px solid) and adoption (1.5px dashed) edges while staying
  inside one coherent edge vocabulary. An ended monogamy marriage adds
  `kul-edge--ended` (translucent), replacing the old `kul-bar--ended`.
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
- **"Offset-spine / junction model (children below their own
  co-spouse)."** Considered, and *shipped in the second cut of #165*:
  the co-spouse was a walker child of the hub and the marriage's
  children were walker children of the co-spouse, so children hung
  below their co-spouse; a child-bearing co-spouse's card sat offset
  left of a vertical *spine* at the cluster centre, the marriage edge
  stubbed into a *junction* at the card's mid-height, and the child
  birth edge continued the spine straight down. Rejected in favour of
  Approach 1 (children centred at the marriage-edge midpoint, co-spouse
  mirrored across it): pulling children to the centre while spouses
  splay to the wings reads as a cleaner, more symmetric fan and removes
  the bespoke spine/junction/card-offset geometry — every child edge is
  again a plain birth edge originating at a point on the marriage edge.
  Do not re-introduce the spine, the junction stub, or the inflated
  co-spouse cluster that offsets the card left of its centre.
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
- **"Snap every marriage's children directly below the hub."**
  Considered as the extreme of centre-pulling. Rejected — distinct
  marriages would re-merge their half-siblings into one column under the
  hub, losing the per-marriage column separation P9 wants. Approach 1
  pulls children only as far as the *marriage-edge midpoint*
  `(hub_cx + cospouse_cx)/2`, halfway between the hub and the wing
  co-spouse, so each marriage keeps its own column while still gathering
  toward the centre. The midpoints are spread by construction (step 1),
  so half-siblings never collide.
- **"Spread children blocks by re-running Walker with inflated
  sibling widths."** Considered as a way to get the midpoint spacing
  for free from the global Walker. Rejected — the prescribed geometry
  (co-spouse mirror, midpoint clearance, outward scaling for odd-N
  clearance) is fully analytic and does not map onto Walker's sibling-
  separation rule. The fan is laid out in a hub-local x and projected
  against the hub's Walker x; the children forests keep Walker only for
  their *internal* tidy-tree layout. Do not try to encode the fan's
  inter-marriage spacing as Walker node widths.

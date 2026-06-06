# ADR 0020 — Polygamy: the hub-equals-host invariant and the fan primitive

**Status:** Accepted
**Date:** 2026-06-07
**Deciders:** owner

## Context

The `host` role is a **per-marriage** concept: the spec ([§4.2](../../spec/04-top-level-statements.md#42-marriage-statement)) names the first-listed spouse as host, and the canonical pattern uses host-ness to decide layout, ordering, and anchoring inside *that one* marriage. Polygamy introduces a **per-person** concept: a **polygamy hub**, a person with two or more un-ended marriages. The canonical pattern renders a hub as a single canonical card ([ADR-0017](./0017-render-shape-schema-and-versioning.md), the **one canonical card per person** principle) and must give it a deterministic visual layout (the **determinism** principle).

The two concepts coincide in **pure-host polygamy** — one person hosts every concurrent marriage — and diverge in two other shapes:

1. **Mixed-role concurrent polygamy** — a person hosts one un-ended marriage and joins another. They are the per-person hub but only the per-marriage host of one.
2. **Pure-join concurrent polygamy** — a person joins two un-ended marriages. They are the hub of both, the host of neither.

A deterministic fan layout needs the hub to be the visual anchor of every concurrent marriage; in the divergent shapes there is no single anchor, and there is no author-visible signal that the document is layout-ambiguous. This ADR settles both halves of polygamy together: the language invariant that removes the divergence, and the visual primitive that renders the result.

A separate question is *which* primitive. Two stances were weighed. **Stance A** uses one fan primitive for any N ≥ 2, including the common N=2 case, so polygamy reads as visually distinct from monogamy at every scale. **Stance B** keeps the monogamy-style hub-and-flanks shape for N=2 and introduces a new primitive only at N≥3. The decision takes Stance A: the shape of polygamy is a *category* the pattern recognises, not a continuous deformation of monogamy, and a single primitive that scales is preferable to graceful escalation — consistent with the **scale-invariance** principle and the project's preference for unified-distinct primitives.

## Decision

### R14 — the hub is the host, by language invariant

A validator rule, **R14 — polygamy hub must host all un-ended marriages** — rejects mixed-role and pure-join concurrent polygamy at check time. The author fixes a violation by swapping the two spouse identifiers in the offending marriage; there is no override field and no renderer-level escape hatch. The fan primitive therefore assumes clean input — every polygamy hub is the host of every concurrent marriage it participates in — and its layout is unambiguous because the language guarantees one canonical reading per hub.

Mutual hub-conflict (both spouses of one un-ended marriage are each hubs) is rejected unsatisfiably: no swap satisfies both spouses, so R14 fires on the marriage where the joining spouse is a hub, and the message points the author at the only authentic fix — ending one of the conflicting marriages so the document represents at most one current polygamy structure. R14 emits one diagnostic per offending marriage, anchored at the marriage id, in project-wide source order (cross-file marriages count toward the same hub per [ADR-0015](./0015-global-project-namespace.md)); the message reproduces `Currently:` and `Fix:` lines so the author sees the exact swap without re-reading the spec. R14 does not fire on a marriage whose spouse positions are unresolved or wrong-kind — R02 already condemns those, and the hub count excludes them so they neither trigger R14 nor inflate another person's count.

### The fan primitive (any N ≥ 2)

A polygamy hub renders as a **fan-from-top-hub**, governed by the invariant

```text
children_center_i = (hub_cx + cospouse_cx_i) / 2
equivalently      cospouse_cx_i = 2 * children_center_i - hub_cx
```

so co-spouses splay toward the wings while each marriage's children gather toward the centre, directly under the midpoint of that marriage's edge:

- **Hub alone at row R** (the host's data-level generation). Nothing else shares the hub's row.
- **Co-spouses at row R+1**, one per hosted marriage in declaration order, at the wing position `cospouse_cx_i = 2 * children_center_i - hub_cx`. There is no marriage bar; the co-spouse is reached by a thick marriage edge.
- **Children of each marriage at row R+2** (`children sit at hub.gen + 2`), centred on the marriage-edge midpoint `children_center_i = (hub_cx + cospouse_cx_i) / 2`. Each marriage's children forest keeps its full Walker layout (deeper generations, even recursive polygamy); the fan pins only the forest's block centre to the midpoint, so half-siblings render in distinct columns, one per marriage. This is the **children hang from the marriage-edge midpoint** principle applied to the fan.
- **Marriage edge** — one routed `<path>` per hosted marriage, from the hub card's bottom-midpoint to the co-spouse's top-centre, using the same orthogonal hub-bottom → horizontal-bus geometry as a birth edge. Its horizontal segment runs at a bus just below the hub and its midpoint is `children_center_i`, where that marriage's child edges originate and fan down. The fan visual emerges from N marriage edges fanning out of the single hub-bottom point; there is no dedicated trunk-branch geometry. A childless co-spouse keeps the same wing placement with an empty children block; its marriage edge lands on its top-centre.

**Layout algorithm (children-centre space, hub-local x).** Because the invariant ties each co-spouse to its marriage's children-centre, the fan is laid out by positioning the children-centres and deriving the co-spouses from them. With `cw = card_width`, `gap = sibling_gap`, `clr = (cw + gap) / 2`, and `CW_i` the packed width of marriage `i`'s children forest (0 if childless):

1. Adjacent children-centre spacing `spacing_i = max((CW_i + CW_{i+1}) / 2 + gap, clr)` — children blocks live half a co-spouse step apart, so this keeps neighbouring blocks and co-spouse cards from overlapping.
2. Cumulative placement `C_1 = 0`, `C_{i+1} = C_i + spacing_i`, then re-centre on the midpoint of the ends so the outer two centres are symmetric about the hub column.
3. **Child-drop clearance:** each child-bearing marriage's drop at `C_i` must clear that co-spouse's card, i.e. `|C_i - hub_cx| >= clr`. A child-bearing centre inside the forbidden band is nudged out to the nearer edge — for the lone middle marriage of an odd N (which lands exactly on the hub column) that is `hub_cx + clr` — and the fan re-packs outward to preserve spacing; the outer two centres are then mirrored so the hub stays at their midpoint.
4. `cospouse_cx_i = 2 * C_i - hub_cx`; shift marriage `i`'s already-laid-out children forest so its block centre lands on `C_i`.

**Integration with Walker.** The hub is a single Walker leaf whose width reserves the full wing-to-wing extent (symmetric about the hub centre), so a fan packs cleanly against sibling components and sits inside a larger host-lineage tree without overlap. Each marriage's children forest is built through the usual `build_person` recursion (so deeper generations and recursive polygamy keep their tidy-tree treatment) and measured for `CW_i` by a per-forest Walker pass; the forests are attached as the hub leaf's Walker children so the global Walker reserves their contour, then rigidly translated in the projection pass so each block centre lands on its prescribed `children_center_i`. Because the prescribed spread is always ≥ the forests' natural spacing and the reserved hub width covers the wider wings, the override never pushes a forest outside the reserved footprint. The hub's children sit at `hub.gen + 2` directly (the co-spouse row sits between), consistent with the shared global generation grid in [ADR-0018](./0018-canonical-layout-algorithm.md).

Monogamy (`hosted_marriages.len() == 1`) keeps the classical hub-and-flanks arrangement: host card and joining card adjacent on one row.

### The marriage edge is the unified connector

A marriage renders as a thick `EdgeKind::Marriage` edge for both monogamy and polygamy — there is no separate bar primitive. A monogamy marriage is a thick horizontal edge spanning the inter-card gap between the two adjacent spouse cards at their vertical mid-height (`[(left_card_right_edge, mid_y), (right_card_left_edge, mid_y)]`), and the couple's children drop from its midpoint. An ended (divorced) marriage carries an `is_ended` flag through to the `data-is-ended="true"` attribute (translucent). `kul-svg` emits `data-link-kind="marriage"` in its `write_edge`, sharing the base `kul-edge` class and all routing CSS with birth and adoption edges ([ADR-0021](./0021-language-properties-plumb-to-svg.md)); the only visual delta is stroke weight (markedly heavier than the 1.5px birth / adoption edges), so a marriage reads as distinct while staying inside one coherent edge vocabulary.

The `MarriageBar` **data** type in `kul-render` carries the marriage's structural metadata; the rendered primitive is an edge rather than a block. `RenderShape` gives a polygamy hub one canonical `PersonCard` carrying N bars ([ADR-0017](./0017-render-shape-schema-and-versioning.md)). In `PositionedShape`, `EdgeKind::Marriage` carries the marriage's properties (`host_id`, `joining_id`, `start`, `end`, `end_reason`, `is_ended`) alongside the `Birth` and `Adoption` parent-child variants (ADR-0021); polygamy marriages are always un-ended per R14. There is no `PositionedBar` type and no `bars` field.

## Consequences

- **A hub renders as a fan; monogamy is unchanged.** The hub sits alone on its row, each co-spouse splays to a wing on the next row, and each marriage's children gather under that marriage's edge midpoint two rows below the hub. A childless co-spouse's marriage edge lands on its top-centre with no children block.
- **The odd-N middle marriage is nudged off the hub column** so its child-drop clears the co-spouse's card by the same `gap/2` margin the wing co-spouses get; the outer co-spouses splay wider to keep the hub centred, and the child-at-midpoint invariant is preserved.
- **No bars at all.** Every marriage — monogamy or polygamy — is a `PositionedEdge { kind: Marriage { .. }, .. }`. The `<rect class="kul-bar">` emission and the `kul-bar--ended` class are gone, replaced by a `kul-edge` carrying `data-link-kind="marriage"` and `data-is-ended`.
- **Composition with the bio-anchor is automatic.** A co-spouse with a declared birth family anchors via the bio-anchor ghost ([ADR-0019](./0019-ghost-model-and-bio-anchor.md)): the ghost lives in the bio family's children row, the canonical card lives in the fan's co-spouse slot, and no edge crosses the canvas (ghosts are mute). The fan is only the host context; bio anchoring is independent.
- **No repair machinery ships.** R14 removes the structural divergence, so the renderer never needs a relocate-the-joining-bar field; the canonical card's children-set and edges resolve exactly per host-ness.

## Alternatives considered and rejected (R14)

1. **Renderer-level relocation.** Repair the divergence by re-anchoring the joining-side bar at the hub's card. Sneaks the hub concept into the rendering layer without naming it, and leaves the language permissive in exactly the cases the fan was meant to unify. Rejected.
2. **Leave admissible and do not render.** Treat mixed-role and pure-join concurrent polygamy as valid but layout-undefined. Incompatible with determinism — every document has one layout, and a class of valid documents with no layout breaks the contract the pipeline relies on. Rejected.
3. **A `host: true / false` override field on `marriage`.** Lets the author pick a host per marriage without reordering spouses. Adds a field and its maintenance burden for a problem swapping two identifiers already solves; revisit only if a future epic surfaces authoring patterns where swap-to-fix is awkward at scale. Rejected.

## Anti-suggestions (do not re-propose)

### The invariant

- **"Relax R14 to admit mixed-role for cultural or authorial reasons."** The fan's semantics depend on R14 holding; relaxing it re-opens the structural ambiguity. If a culture-specific pattern surfaces that R14 forbids, teach the spec to *accept* it in a controlled way (a new statement, with its own ADR), not weaken R14 globally.
- **"Resurrect a `relocated_joining_bars` field for the mutually-polygamous corner case."** That corner is rejected on purpose; mutual hubs is exactly the configuration with no deterministic fan layout, and a renderer-level repair would re-introduce the ambiguity R14 closes.
- **"Add a code-action that auto-swaps spouses."** The diagnostic already spells out the swap; a code action carries minimal value over the message and couples the validator's diagnostic to a quick-fix payload. Revisit only if the swap-edit workflow proves friction-heavy in practice.

### The primitive

- **"Trunk + branch + per-bar drop fan connector, with a bar per co-spouse."** A bespoke geometry: hub drops a trunk to a horizontal branch spanning the per-marriage columns, with a vertical drop onto each marriage bar. The edge-as-path treatment instead unifies a polygamy marriage with the rest of the edge vocabulary — one `kul-edge` path with `data-link-kind="marriage"` routed like a birth edge, no `PositionedFanConnector` type, no bar rect, no sub-row. Do not re-introduce the trunk-branch-drop decomposition or the per-co-spouse bar at N≥2.
- **"Offset-spine / junction model (children below their own co-spouse)."** A co-spouse offset left of a vertical spine, the marriage edge stubbing into a junction at the card's mid-height, the child edge continuing the spine down. Pulling children to the marriage-edge midpoint while co-spouses splay to the wings reads cleaner and removes the spine/junction/card-offset geometry — every child edge is again a plain birth edge originating on the marriage edge. Do not re-introduce the spine, the junction stub, or the offset co-spouse cluster.
- **"Hub-and-flanks for N=2 only; a new primitive at N≥3."** Two visual shapes for one category multiplies the reader's pattern-recognition burden without proportional gain. One primitive distinct from monogamy at every N — including the common N=2 — is the point.
- **"Radial / star layout for N≥3."** Breaks the generation-row convention, scale-invariance, and the children-below convention. A radial layout reads as a different kind of diagram, not a continuation of the descendency tree.
- **"Vertically-extended hub (the hub card spans multiple rows)."** At N=2 it looks like monogamy with a taller hub (no visual distinction); at N≥3 child routing from sibling co-spouses collides at the hub's edges.
- **"Compound polygamy block (a decorative grouping rectangle around hub + co-spouses)."** Decoration, not geometry. The fan is the geometry; a future render-time toggle could add a decorative grouping element on top without committing the canonical pattern.
- **"Snap every marriage's children directly below the hub."** Distinct marriages would re-merge their half-siblings into one column under the hub, losing the per-marriage column separation. The fan pulls children only to the marriage-edge midpoint, halfway between hub and wing co-spouse, so each marriage keeps its own column while still gathering toward the centre.
- **"Spread children blocks by re-running Walker with inflated sibling widths."** The prescribed geometry (co-spouse mirror, midpoint clearance, odd-N outward scaling) is fully analytic and does not map onto Walker's sibling-separation rule. The fan is laid out in hub-local x and projected against the hub's Walker x; the children forests keep Walker only for their internal tidy-tree layout. Do not encode the fan's inter-marriage spacing as Walker node widths.

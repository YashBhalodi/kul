# Canonical UI pattern

The visual language for kinship.

## Top-level positioning

- **Visual baseline: classical descendency family tree.** Generations as horizontal rows (older above, younger below); spouses adjacent with a marriage bar between them; children connected below the marriage bar. This is the only kinship visual everyone already knows; the pattern's job is to extend it honestly for the cases the classical tree doesn't natively handle.
- **No temporal axis.** Time is not a load-bearing layout axis. Dates may appear as content; the layout does not encode time. This was considered and rejected — the constraint that drove the rejection was "anyone-look-and-understand at low cognitive load," for which the familiar classical metaphor wins over temporal innovation.
- **Single surface for authors and readers.** The same canonical visual serves both audiences. Editor-mode overlays (selection, jump-to-source) are downstream presentational extensions.
- **Static and deterministic.** A Kul document renders to exactly one canonical layout. No interactivity, no view parameters required to produce the canonical view.

## Principles

### P1. Classical descendency tree baseline
The pattern's visual baseline is the classical descendency family tree.

*Alternatives considered:* marriage-centric (junction nodes, rejected as too unfamiliar); person-centric ego-style (fan chart, rejected for temporal implicitness and poor multi-family scaling); force-directed network (rejected for hiding kinship behind generic edges).

### P2. One canonical card per person
Each person renders as exactly one *canonical* card, which may host one or more marriage bars (P8). Visual duplicates (ghosts) exist only under the narrow conditions specified in P8 and P16.

### P3. First-declared spouse hosts
In a `marriage <id> <spouse_a> <spouse_b>` statement, `spouse_a` is the host; `spouse_b` joins the host's family tree. The host stays in their own canonical position; the joining spouse's canonical card sits adjacent to the host within the host's family. The joining spouse's birth-family connection becomes a cross-tree edge (P6).

The host role is **per-marriage** by definition — every marriage has its own host independent of other marriages either spouse is in. For a polygamy hub (a person with ≥2 un-ended marriages), the per-marriage host coincides with the per-person hub by language invariant: rule R14 ensures the hub is the declared host of every concurrent un-ended marriage they participate in ([ADR-0026](./adr/0026-polygamy-hub-equals-host.md)). Authors fix a violation by swapping the two spouse identifiers in the offending marriage; there is no override field.

*Rationale:* deterministic, gender-neutral, culturally neutral, author-controllable via declaration order. Alternatives considered and rejected: patrilineal default (gender-loaded); view-driven (incompatible with P7); explicit per-marriage tagging (heavier spec burden).

Defined normatively in [`spec/04-top-level-statements.md` §4.2](../spec/04-top-level-statements.md#42-marriage-statement); see also the `Host` and `Polygamy hub` glossary entries in [`CONTEXT.md`](../CONTEXT.md).

### P4. Adopted child lives in adoptive family
A child with both `birth` and `adoption` sub-statements lives — visually — in the adoptive family. The biological link renders as a cross-tree solid edge to the birth marriage (P5).

### P5. Connector style encodes link kind
Solid line = birth. Dashed line = adoption. Applies to every parent-child connector, whether it stays within a tree or crosses between them. Card border style is independent and encodes "is this a ghost?" — one dimension per visual axis.

### P6. Recursive nesting at inter-family connections
When a Kul document describes multiple intermarried families, each joining spouse's *birth family* sub-tree nests adjacent to the host tree at the joining spouse's connection point. Sub-trees nest further recursively.

Recursion terminates structurally: B's birth-family sub-tree nests at A's tree only if B's birth family is not already part of the current rendering context. If it is — cousin marriage (P11) is the simplest case, but it generalizes to any chain where a joining spouse's birth family is already being rendered — P11's within-family cross-edge applies instead, and no nested sub-tree is emitted. Combined with P2 (one canonical card per person), this guarantees finite, non-redundant nesting at any document size: the maximum depth equals the number of distinct birth families along the chain of joining spouses, never more.

Visual complexity at scale (renderer cost at 5,000 cards, level-of-detail, panning, virtualization) is renderer policy under P14, not a pattern concern.

*Rationale:* cross-tree edges stay short by construction; the layout makes inter-family structure visually obvious. Linear arrangement (trees side-by-side without nesting) was rejected because it produces long crossing edges.

*Worked example (`examples/13-inter-family-marriage`):* Alice (bio child of Rajesh+Saroj) hosts Bob (bio child of Mahesh+Lata); Alice's host tree carries Rajesh+Saroj at generation 0, the `m_host_parents` bar, Alice canonically in their children row, the `m_alice_bob` bar adjacent to Alice with Bob at the joining slot, and their child Kiran below. Bob's birth family is unrelated to Alice's, so per P6 it nests as a sub-tree adjacent to the host tree at the joining-spouse connection point: Mahesh + Lata at generation 0 and the `m_bob_parents` bar between them sit immediately to the right of Alice's host tree on the same generation-0 row. Bob himself appears in the nested children row as a **past-bio child-ghost** under `m_bob_parents` (P16) — his canonical card is the joining slot on `m_alice_bob`, and P8's chain selected that marriage over the bio family. Bob's bio-birth edge is therefore a short solid drop from `m_bob_parents` to ghost-Bob, using the same orthogonal bus-and-drop geometry every within-tree birth edge uses ([ADR-0018](./adr/0018-kul-layout-crate-boundary.md), [ADR-0022](./adr/0022-p6-layout-sibling-root-packing.md)). No P11 termination fires on the *nesting* — the two birth families share no people, so Bob's family is genuinely outside the main walk.

*Worked example (`examples/14-grand-nested-inter-family`):* extends example 13 with one more layer on Bob's side — Bob's mother Lata herself carries a birth family (Ramprasad + Sunita + `m_lata_parents`). P6 now recurses twice: Bob's birth family (Mahesh + Lata + `m_bob_parents`) nests as a sibling-root to the right of the host tree, and Lata's birth family in turn nests as a further sibling-root to the right of *that*. Row 0 holds Ramprasad + Sunita + `m_lata_parents` alone — the deepest ancestor in the canvas — packed rightmost via ADR-0022. Row 1 holds both Rajesh + Saroj + `m_host_parents` and Mahesh + Lata + `m_bob_parents` as kin-symmetric grandparents of Kiran, plus ghost-Lata in `m_lata_parents`'s children row (her canonical card is Bob's joining slot's mother in `m_bob_parents`; P16 emits a past-bio ghost there); row 2 holds Alice + `m_alice_bob` + Bob plus ghost-Bob in `m_bob_parents`'s children row (same rule); row 3 holds Kiran. The host tree's founders shift *down* to row 1 because `kul-layout` cascades `visual_row` from descendants up — a cluster sits one row above its closest descendant — so peer-relationship ancestors across an inter-family marriage align on the same visual row ([ADR-0024](./adr/0024-kul-layout-visual-row-descendant-pull.md), refining [ADR-0023](./adr/0023-kul-layout-visual-row-cascade.md) and distinct from `RenderShape`'s structural data-level generation per [ADR-0017](./adr/0017-render-shape-schema-and-versioning.md)). Both bio-birth edges are now short local drops — Lata's from `m_lata_parents` to ghost-Lata in the next row, and Bob's from `m_bob_parents` to ghost-Bob in the next row — rather than the long cross-tree edges they were before the P16 bio-anchor rule (ADR-0025).

### P7. Static, deterministic rendering
Given a Kul document, the canonical pattern produces exactly one layout. No interactivity, no view parameters, no user-selected focus. Surfaces may add interactivity on top of the canonical view; that is a presentational extension, not part of the pattern.

### P8. Canonical card sits at current intimacy
A person's canonical card sits at their **current intimacy**. "Current" is defined by a priority chain over the person's structural facts, applied in order:

1. **First-declared un-ended marriage.** If the person participates in any marriage (host or join) that does not carry an `end:` field, their canonical card sits at the first-declared such marriage by source order. The host's canonical card sits in their birth-family children row (or at the bar's host slot when the host has no birth family — a floating mini-component sortable per P12); the joining spouse's canonical card sits at the bar's joining slot.
2. **Most-recent adoption.** Otherwise, if the person has one or more `adoption` sub-statements, their canonical card sits in the children row of the most recent adoption (by `start:` date, declaration-order tiebreak).
3. **Bio family.** Otherwise, if the person has a `birth` sub-statement, their canonical card sits in the bio family's children row.
4. **Orphan.** Otherwise, the person renders as a lone card (P13).

A marriage is "ended" if it carries an `end:` field. In the current Kul spec, `end:` corresponds to divorce only — death is not marked on the marriage but on the deceased spouse's `died:` field. A widow's marriage therefore has no `end:` and she remains in the host family. If a future spec extension broadens what `end:` can carry, the rule applies uniformly without changes.

An **intimacy** in the priority chain's vocabulary is any link a person carries to a family unit — a marriage (host or join), an adoption, a birth. The "current" intimacy is whichever the chain selects; every link the chain *doesn't* select is a **past intimacy**. P16 governs how past intimacies surface visually.

A person may have multiple concurrent un-ended marriages (polygamy / polyamory) — a **polygamy hub**. If the person has ≥2 un-ended marriages, their canonical card sits at the **fan hub position**: the hub card occupies its own row at row R alone; each co-spouse appears canonically on the standard child row R+1 below, one per marriage in declaration order, and is reached by a thick **marriage edge** ([ADR-0027](./adr/0027-fan-primitive-for-polygamy-hubs.md)) — one routed `<path>` from the hub card's bottom edge to the co-spouse card's top edge, using the same orthogonal right-angle routing as a birth edge but with a heavier stroke. The fan visual emerges from the N marriage edges fanning out of the single hub-bottom point. No marriage bar is emitted for a polygamy marriage (bars are a monogamy-only primitive); a marriage's children hang below their own co-spouse at row R+2, with their birth/adoption edges anchored at the co-spouse card's bottom edge. No ghost is emitted for any current intimacy — the ghost vocabulary (P16) is reserved for *past* (ended / superseded) intimacies.

Rule R14 ([ADR-0026](./adr/0026-polygamy-hub-equals-host.md)) ensures the hub is the declared host of every concurrent un-ended marriage they participate in, so "hub" and "host" coincide by language invariant rather than by renderer repair. Mixed-role and pure-join concurrent polygamy are rejected at check time; authors fix violations by swapping spouse identifiers.

The marriage bar's canonical location is the host's birth-family slot at the position the host occupied within their birth family *at the time the marriage was declared*. This location is fixed at declaration and does not relocate due to later events (divorce, remarriage, death). The host's *canonical card* sits at that slot if the host hasn't moved on (no newer current intimacy); otherwise the host leaves a past-marriage *ghost* there per P16. The joining spouse occupies the adjacent slot — canonically if they haven't moved on, as a past-marriage ghost otherwise. If the host has no birth family (no `birth` sub-statement), the marriage bar becomes a *floating mini-component* sortable per P12.

*Worked example (`examples/03`):* Alice hosts Bob; Alice's marriage stays at Alice's slot in Ramesh+Sita's tree (Alice hasn't moved on, so her canonical card sits there per chain step 1); Bob has no birth family and the marriage has ended, so Bob's canonical card has no anchor and renders as an orphan component (chain step 4) — he leaves a past-marriage ghost adjacent to Alice (P16) to which Carol and Ravi's edges attach.

*Worked example (`examples/04`):* Devraj hosts Meera (`m_devraj_meera`, 1990) and Alice (`m_devraj_alice`, 1992), both ongoing. Devraj is the polygamy hub (two un-ended marriages); per ADR-0027 his canonical card renders alone at the fan hub position on row 0, with Meera and Alice as canonical co-spouses on row 1 below. Two thick marriage edges fan out of Devraj's bottom-midpoint — one routing down to Meera's top edge, one to Alice's — exactly as a birth edge routes a parent to a child, only heavier. No marriage bar is rendered. Neither marriage emits a past-marriage ghost (both are current). Priya hangs from Alice (`m_devraj_alice`'s co-spouse) per P9, in her own column on row 2, with her birth edge anchored at Alice's bottom edge.

*Worked example (`examples/12`):* Same polygamy hub as example 04 (Devraj concurrently married to Meera and Alice, Priya the bio child of `m_devraj_alice`), but Devraj himself is now a canonical child of Ramesh + Sita's marriage at row 1. The fan still applies: Devraj's card sits alone on row 1; Meera and Alice render as co-spouses on row 2 below him, each reached by a thick marriage edge fanning from Devraj's bottom; Priya hangs from Alice in her own column on row 3. Ramesh + Sita's classical hub-and-flanks cluster sits on row 0 (monogamy, unchanged — its marriage bar is still rendered), with Devraj's birth edge attaching to `m_ramesh_sita`'s bar and entering Devraj's hub card from above.

*Worked example (`examples/15`):* N=3 polygamy. Devraj hosts Meera, Alice, and Diana, all ongoing. Devraj's canonical card sits alone at the fan hub position on row 0; the three co-spouses render on row 1 in declaration order — Meera at the outer-left column, Alice in the middle column (directly below Devraj), Diana at the outer-right column. Three thick marriage edges fan out of Devraj's single bottom-midpoint: the middle edge to Alice routes straight down, the outer two jog left to Meera and right to Diana. No marriage bars are rendered. Each marriage has one child — Asha, Priya, Rohan — hanging in their own column below the corresponding co-spouse on row 2; the three half-siblings render in distinct sub-trees per P9.

*Worked example (`examples/09`):* Sam has a bio family (`m_ravi_priya`), a past adoption (`m_anita_bharat`, 1985), and a current adoption (`m_chen_dara`, 1992). Sam has no marriages, so chain step 1 doesn't fire; chain step 2 picks `m_chen_dara` (most recent) and Sam's canonical card sits in its children row. `m_anita_bharat` and `m_ravi_priya` are both past intimacies; P16 emits a child-ghost in each.

### P9. Birth/adoption edges connect to the marriage bar
A child's birth or adoption edge attaches to the marriage bar of their parents' marriage — not to either parent card individually. This matches Kul's data model (`birth m_xxx` references the marriage id, not the parent ids).

For polygamy hubs (P8 fan): a polygamy marriage has no bar, so each marriage's children hang from their own **co-spouse** (the spouse on the far end of the marriage edge), not from the hub. The child's birth/adoption edge anchors at the co-spouse card's bottom edge. Half-siblings render in distinct sub-trees, one per co-spouse — e.g. in example 04, Priya hangs from Alice only; in example 15 (N=3), Asha hangs from Meera, Priya from Alice, and Rohan from Diana, each in their own column below their co-spouse.

### P10. Ghosts are mute
A ghost connects only to the marriage/adoption bar it anchors. The person's other structural connections (their own birth family, other marriages, other adoptions) attach to the canonical card, never to any ghost.

### P11. Absorb rule applies uniformly at every scale
The absorb rule (first-declared spouse hosts; joining spouse's card moves adjacent; their birth-edge becomes a cross-edge) applies identically across families, within a single family (cousin marriages), and at any structural scale. There is no special case for within-family marriages — the same mechanism produces a within-family cross-edge instead of a cross-tree one.

Operationally for cousin marriage: the joining cousin's canonical card moves adjacent to the host (per P3); the cousin's original sibling-row position does *not* render — siblings re-pack just as they would for a cross-tree joining spouse, since P11 is "the same mechanism" without special cases. The cousin's birth-edge becomes a within-family cross-edge from the new adjacent-to-host position back to their parents' marriage bar; edge style stays solid (P5).

The sibling-marriage degenerate case (both spouses are children of the same marriage) is representable by the same mechanism: the host stays at their sibling-row position; the joining sibling moves adjacent to the host within the same row; both birth-edges attach to the same parents' marriage bar — the host's normally and the joining sibling's as a within-family cross-edge.

Cross-edge *routing* (geometry, collision avoidance) is renderer policy, not part of the pattern.

### P12. Multiple unrelated lineages arrange in source order
When a Kul document describes multiple lineages with no intermarriage between them (separate connected components in the graph), the components arrange left-to-right by the source position of the component's **first relevant declaration**: a marriage if the component has one, otherwise a person, otherwise — for floating ghost-marriage mini-components per P8's fallback — the underlying marriage. Components mix freely in source order; there is no "orphans-last" bucket.

The same ordering rule applies recursively within a component to nested sub-trees: when a host has multiple joining spouses each bringing a birth-family sub-tree (P6), those sub-trees arrange in joining-spouse declaration order.

*Rationale:* consistent with P3's source-order semantic. Author controls via declaration order — the same control mechanism used for the host rule.

### P13. Missing data renders as absence
Missing optional fields render as absence — no placeholders, no "Unknown" stubs, no allocated visual space. Required-field gaps are not a case the canonical pattern designs for (R03 ensures valid documents carry name and gender).

Orphan persons (declared with no edges of any kind) render as single-card components *and sort with all other components by the rule in P12*.

### P14. Scale-invariant pattern
The pattern is scale-invariant. The same rules produce coherent layouts at 5 persons, 50 persons, or 5,000. Level-of-detail, zoom, panning, virtualization, and aggregation are renderer-side policy and not part of the canonical pattern.

*Rationale:* pattern vs. presentation separation. The pattern produces a structural output; renderers innovate on level-of-detail using the pattern's natural hierarchy (family → branch → couple → child) and generational y-axis.

### P15. Uniform card; name minimum; gender not visually encoded
A person card is a uniform shape carrying at minimum the person's `name:`. Other Kul fields may appear per renderer policy.

Gender is **not** visually encoded by card shape, color, or icon in the canonical pattern. If a renderer chooses to surface gender, it does so via text label using Kul's three values (`male | female | other`).

The only canonical card-appearance variation is canonical (solid border, full opacity) vs. ghost (dotted border, faded fill, ↺ badge).

*Rationale:* cultural / political neutrality; visual uniformity focuses attention on structure; composability lets renderers opt into richer chrome without forking the pattern.

### P16. Past intimacies emit ghosts to anchor their edges
Every intimacy not selected by P8's chain is a *past intimacy*. Each past intimacy emits a **ghost** — a mute (P10), visually distinct (dotted border, faded fill, ↺ badge) duplicate of the person — slotted into the past family at the position that intimacy would have occupied if it were canonical. The ghost's only purpose is to anchor the edges that would otherwise traverse the canvas to the canonical card.

Three flavors apply the rule:

- **Past-marriage spouse-ghost.** A marriage carries `end:` and the moved-out spouse's canonical card now lives elsewhere (or the bar is a floating mini-component whose host has moved on). The ghost sits in the bar's host or joining slot (whichever the moved-out spouse occupied per P3) so the bar's children edges still attach. Past marriages without children leave no visual trace — the marriage stays in the data; the renderer omits it.
- **Past-adoption child-ghost.** A person has multiple `adoption` sub-statements, so P8's chain step 2 selects the most-recent and demotes the rest. Each past adoption's bar gets a child-ghost in its children row connected by a dashed edge — three past adoptions produce three ghosts.
- **Past-bio child-ghost.** A person has a `birth` sub-statement but P8's chain selects a different intimacy (a marriage, or an adoption — adoption demotes the bio family from current to past). The bio marriage gets a child-ghost in its children row connected by a solid edge.

Each ghost slots into the past family's children or spouse row at the **source-order position** the person would occupy if their canonical card were here — the same declaration-order key that drives sibling-row layout for canonical children (per P3 / P12's source-order semantic). This applies uniformly to all three flavors.

Each past family's bar is rendered at the canonical location defined by P8 (host's birth-family slot, or floating mini-component if the host has no birth family), **even when no other rule would surface it** — the ghost is the load-bearing reason that bar must exist.

*Worked example (`examples/03`):* `m_alice_bob` has ended and Bob has moved on (no birth family, but his canonical card is an orphan card per P8 step 4). Bob's past-marriage spouse-ghost sits in `m_alice_bob`'s joining slot adjacent to Alice; Carol and Ravi's birth edges attach to the bar.

*Worked example (`examples/09`):* Sam's canonical card sits in `m_chen_dara`'s children row (P8 step 2). Two past intimacies emit ghosts: a past-adoption child-ghost under `m_anita_bharat` (dashed edge), and a past-bio child-ghost under `m_ravi_priya` (solid edge). Sam's bio sibling Bro is a canonical child of `m_ravi_priya`; ghost-Sam and canonical-Bro sit side by side in source order (ghost-Sam first because Sam is declared before Bro).

*Rationale:* one emission rule, three applications, one visual primitive (ghost). Same purpose (anchor a past structural fact), same visual vocabulary. The principle's discriminator is *semantic* (derived from P8's canonical placement) rather than data-shape, so a future spec extension that introduces a new family-unit link kind would slot in as a fourth ghost flavor without restructuring the rule.

## Visual vocabulary

| Element | Convention |
| --- | --- |
| **Canonical person card** | Solid border, opaque fill. Carries `name:` at minimum. |
| **Ghost person card** | Dotted border, faded fill, ↺ badge in corner. Mute. Anchors a past structural fact. |
| **Marriage bar** | Small rectangle between two adjacent spouses. |
| **Adoption bar** | Same shape as marriage bar; semantically, the join-point for an adoption sub-statement. |
| **Birth edge** | Solid line. Routes within a tree (marriage bar → child below) or across trees (canonical card → past or different-tree birth marriage). |
| **Adoption edge** | Dashed line. Same routing rules; edge style alone distinguishes from birth. |
| **Marriage edge** | Solid line, thicker (~3-4px) than birth / adoption edges (1.5px). One per concurrent marriage of a polygamy hub (P8 fan, [ADR-0027](./adr/0027-fan-primitive-for-polygamy-hubs.md)), routed from the hub card's bottom edge to the co-spouse card's top edge with the same orthogonal right-angle geometry as a birth edge. The fan emerges from N marriage edges fanning out of the single hub-bottom point; there is no separate connector primitive. Emitted only at N ≥ 2 — monogamy renders the marriage as a bar between adjacent spouses instead. |

## Amending this document

This pattern co-evolves with the Kul language specification. When [`spec/`](../spec/README.md) gains a new construct (a new sub-statement, a new field that affects layout, a broadened semantic on an existing field, a new top-level statement), the responsible PR updates this document in the same change — deciding how the new construct renders is part of shipping it. Amend the principles; don't restart from scratch.

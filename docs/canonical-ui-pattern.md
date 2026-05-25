# Canonical UI pattern

The visual language for kinship.

## What the pattern is

The canonical pattern is a **pure function from a Kul document to one layout**. Given a document it produces exactly one canonical visual; there are no view parameters, no interactivity, and no user-selected focus inside the pattern itself. Surfaces (the VSCode preview, a web app, a CLI export) add theming and interactivity on top — that is downstream chrome, not part of the pattern (see [ADR-0016](./adr/0016-visualization-pipeline-crate-boundaries.md)).

Its visual baseline is the **classical descendency family tree**: generations as horizontal rows (older above, younger below); spouses adjacent with a thick marriage edge between them; children connected below the midpoint of that edge. This is the only kinship visual everyone already knows, and the pattern's job is to extend it honestly for the cases the classical tree does not natively handle — adoption, divorce and remarriage, polygamy, and marriages that join unrelated families. Time is deliberately **not** a layout axis: dates may appear as content, but the layout encodes kinship structure, not chronology, because "anyone can look and understand at low cognitive load" is better served by the familiar spatial metaphor than by temporal innovation.

The same canonical visual serves both authors and readers. Editor overlays (selection, jump-to-source) are presentational extensions on top of it.

## Principles

The principles below are the minimal set that generates the canonical layout. They are stated so a reader understands *what* each one requires; the architectural decisions in [`docs/adr/`](./adr/) carry the *why* and the rejected alternatives.

### The classical descendency tree

The pattern's visual baseline is the classical descendency family tree — generation rows, adjacent spouses joined by a marriage edge, children below the marriage-edge midpoint. Every other principle extends this baseline rather than replacing it.

*Alternatives considered:* marriage-centric junction nodes (rejected as too unfamiliar); person-centric ego/fan charts (rejected for temporal implicitness and poor multi-family scaling); force-directed networks (rejected for hiding kinship behind generic edges).

### Determinism and scale-invariance

A Kul document renders to exactly one canonical layout — no interactivity, no view parameters, no user-selected focus are required to produce it. The same rules produce a coherent layout at five persons, fifty, or five thousand. Level-of-detail, zoom, panning, virtualization, and aggregation are renderer-side policy that build on the pattern's natural hierarchy (family → branch → couple → child) and its generational rows; they are not part of the pattern.

### One canonical card per person

Each person renders as exactly one *canonical* card, which may host one or more marriages. Visual duplicates of a person — **ghosts** — exist only to anchor a past structural fact, under the narrow conditions of *past intimacies emit ghosts*.

### The uniform card

A person card is a uniform shape carrying at minimum the person's `name:`; other Kul fields may appear per renderer policy. Gender is **not** encoded by card shape, colour, or icon — a renderer that surfaces gender does so via a text label using Kul's three values (`male | female | other`). The only canonical card-appearance variation is canonical (solid border, full opacity) versus ghost (dotted border, faded fill, ↺ badge). Visual uniformity keeps attention on structure and stays culturally neutral. Consuming surfaces MAY opt out by selecting on the structural `data-gender` attribute the SVG carries (every declared property plumbs through as a `data-*` attribute, per [ADR-0021](./adr/0021-language-properties-plumb-to-svg.md)), but the canonical pattern does not.

### Absence, not placeholders

Missing optional fields render as absence — no placeholders, no "Unknown" stubs, no allocated space. (Required-field gaps are not a case the pattern designs for; R03 ensures valid documents carry name and gender.) A person declared with no edges of any kind renders as a single-card component.

### The absorb rule

In a `marriage <id> <spouse_a> <spouse_b>` statement, `spouse_a` is the **host** and `spouse_b` **joins** the host's family. The host stays in their own canonical position; the joining spouse's canonical card sits adjacent to the host, and the joining spouse's birth-family connection becomes a **cross-edge** rather than relocating the host. This one rule applies **uniformly at every scale**, with no special case for who the two spouses are:

- **Across unrelated families** — the joining spouse's *birth family* nests as a sub-tree adjacent to the host tree at the connection point, and the rule recurses: a joining spouse inside that nested sub-tree brings *their* birth family as a further nested sub-tree. Recursion terminates structurally — a birth family that is already in the current rendering context does not nest again — so nesting depth never exceeds the number of distinct birth families along the chain of joining spouses.
- **Within one family** (cousin marriage, or the degenerate sibling marriage where both spouses descend from the same marriage) — the same mechanism produces a *within-family* cross-edge instead of a nested sub-tree. The joining cousin's card moves adjacent to the host; their original sibling-row slot does not render; siblings re-pack exactly as they would for any joining spouse; and their birth edge becomes a within-family cross-edge back to their parents' marriage-edge midpoint.

The host role is **per-marriage** by definition — every marriage has its own host, independent of any other marriage either spouse is in. Authors control host-ness through declaration order; there is no override field, and a violation is fixed by swapping the two spouse identifiers. (For a polygamy hub the per-marriage host coincides with the per-person hub by language invariant; see *current-intimacy placement* and [ADR-0020](./adr/0020-polygamy-hub-and-fan.md).) Cross-edge routing — geometry, collision avoidance, how far a nested sub-tree sits from the host tree — is layout policy ([ADR-0018](./adr/0018-canonical-layout-algorithm.md)), not part of the pattern.

Defined normatively in [`spec/04-top-level-statements.md` §4.2](../spec/04-top-level-statements.md#42-marriage-statement); see the `Host` and `Polygamy hub` entries in [`CONTEXT.md`](../CONTEXT.md).

*Worked example (`examples/05-cousins-and-in-laws`):* the same rule produces both outcomes in one family. Marco Rossi hosts Elena, who was born into the unrelated Bianchi family — so the Bianchis nest as a sub-tree adjacent to the Rossi tree at Elena's connection point, and Elena's bio family becomes a past intimacy that surfaces a past-bio child-ghost in her parents' children row, reached by a short local edge. Later, Marco's son Matteo marries his cousin Giulia — both already in the Rossi tree — so within-family termination fires: nothing new nests, Giulia's tie is drawn as a within-family cross-edge back to her parents' marriage, and a ghost marks the slot her card left.

*Worked example (`examples/09-family-across-a-century`):* the absorb rule recurses. Adunni hosts Chukwu, born into the Okafor family, so the Okafors nest beside the Adeyemi tree; Chukwu's mother Chiamaka was herself a joining spouse who carried her own birth family (the Eze line), so that family nests as a further sub-tree one level deeper. Across such an inter-family marriage the deepest ancestor stack and the host-tree founders are aligned onto shared rows so kin-symmetric grandparents read as the same generation — a layout concern handled by the visual-row cascade ([ADR-0018](./adr/0018-canonical-layout-algorithm.md)), not by the pattern.

### Current-intimacy placement

A person's canonical card sits at their **current intimacy**, chosen by a priority chain over the person's structural facts, applied in order:

1. **First-declared un-ended marriage.** If the person participates in any marriage (as host or joining spouse) that carries no `end:` field, their canonical card sits at the first such marriage by source order.
2. **Most-recent adoption.** Otherwise, if the person has one or more `adoption` sub-statements, their card sits in the children row of the most recent adoption (by `start:` date, declaration-order tiebreak). An adopted child thus lives — visually — in their adoptive family; the biological link becomes a cross-edge.
3. **Bio family.** Otherwise, if the person has a `birth` sub-statement, their card sits in the bio family's children row.
4. **Orphan.** Otherwise, the person renders as a lone card.

An **intimacy** is any link a person carries to a family unit — a marriage, an adoption, a birth. The chain selects the *current* one; every link it does not select is a **past intimacy** (see *past intimacies emit ghosts*). A marriage is "ended" only if it carries `end:` — death is marked on the deceased spouse's `died:` field, not on the marriage, so a widow's marriage has no `end:` and she stays in the host family. If a future spec broadens what `end:` can carry, the rule applies unchanged.

A marriage's canonical location is the host's birth-family slot at the position the host occupied *when the marriage was declared*; it does not relocate due to later events. The host's canonical card sits there if the host has not moved on; otherwise the host leaves a past-marriage ghost there. The joining spouse occupies the adjacent slot on the same terms. If the host has no birth family, the marriage becomes a **floating mini-component** ordered with the other components by *source order*.

A person with two or more concurrent un-ended marriages is a **polygamy hub**. By language invariant (rule R14) the hub is the declared host of every concurrent un-ended marriage, so "hub" and "host" coincide and the layout is unambiguous — mixed-role and pure-join concurrent polygamy are rejected at check time, and authors fix a violation by swapping spouse identifiers. The hub's single canonical card sits at the **fan hub position**: the hub occupies its own row alone, each co-spouse sits one row below (one per marriage, in declaration order), and each marriage's children sit a further row below, gathered under that marriage's edge (see *children hang from the marriage-edge midpoint*). No ghost is emitted for any current intimacy. The full fan geometry — wing placement, the odd-N middle nudge, the marriage edge as a routed path — is layout policy in [ADR-0020](./adr/0020-polygamy-hub-and-fan.md).

*Worked example (`examples/03-divorce-and-remarriage`):* Erik and Astrid divorce and each remarries, so chain step 1 places each of their canonical cards at their *current* marriage. Their ended marriage keeps neither as canonical — both leave a past-marriage ghost there — while their daughter Linnea, who has only a `birth`, is canonical in that bio family (chain step 3). A person who has moved on with no anchor at all would fall through to a lone card (chain step 4) — as the orphan Nadia does in `examples/07-disconnected-lineages`.

*Worked example (`examples/06-polygamous-household`):* Khalid is concurrently married to Aisha, Layla, and Mariam, so he is a polygamy hub and his single card sits at the fan hub position; the three co-spouses are canonical, one per marriage. None emits a ghost, since every marriage is current.

*Worked example (`examples/04-adoption-and-belonging`):* Dalisay was born to one couple, adopted by a second whose adoption later ended, and finally adopted by the Mendozas — and has no marriage. Chain step 2 selects her most-recent adoption (the Mendozas), so her canonical card sits in their children row; her birth family and the earlier adoption are both past intimacies.

### Edges encode link kind

A parent-child connector's *line style* encodes the link kind: **solid for birth, dashed for adoption**. This holds for every connector, whether it stays within a tree or crosses between them. Line style is one visual axis; card border style (canonical versus ghost) is an independent axis. One dimension per axis.

### Children hang from the marriage-edge midpoint

A child's birth or adoption edge attaches to the **midpoint of their parents' marriage edge** — not to either parent card individually. This matches Kul's data model, where `birth m_xxx` references the marriage id, not the parent ids. For a monogamous couple that midpoint is the centre of the horizontal marriage edge spanning the gap between the two adjacent spouse cards, and the child's edge drops from there.

For a polygamy hub the same rule holds per marriage: each marriage's children hang from the midpoint of *that* marriage's edge. The hub and each co-spouse sit symmetrically about that midpoint, captured by the invariant

> `children_center = (hub_center + cospouse_center) / 2`

so the co-spouse splays out toward the wing while the marriage's children gather toward the centre, under the marriage-edge midpoint. Because each marriage has its own midpoint, every marriage's children occupy their own column, distinct from the half-siblings of every other marriage.

### Past intimacies emit ghosts

Every intimacy the priority chain does *not* select is a past intimacy, and each one emits a **ghost** — a mute, visually distinct (dotted border, faded fill, ↺ badge) duplicate of the person, slotted into the past family at the position that intimacy would occupy if it were current. The ghost's only purpose is to anchor the edges that would otherwise traverse the canvas to the canonical card. One emission rule, three applications:

- **Past-marriage spouse-ghost** — an ended marriage whose moved-out spouse lives elsewhere (or a floating bar whose host has moved on). The ghost sits in the slot that spouse occupied, so the marriage's children edges still attach. A childless past marriage leaves no visual trace.
- **Past-adoption child-ghost** — a demoted adoption (chain step 2 selected a more recent one). The bar gets a child-ghost connected by a dashed edge.
- **Past-bio child-ghost** — a `birth` link the chain did not select (because a marriage or adoption is current). The bio marriage gets a child-ghost connected by a solid edge.

Each ghost slots into its past family's children or spouse row at the **source-order position** the person would occupy if canonical there — the same declaration-order key the children row uses for canonical siblings. A past family's bar is rendered at its canonical location even when nothing else would surface it; the ghost is the reason that bar must exist. The discriminator between the three flavors is semantic (derived from where the chain placed the canonical card), so a future link kind would slot in as a fourth flavor without restructuring the rule (see [ADR-0019](./adr/0019-ghost-model-and-bio-anchor.md)).

*Worked example (`examples/03-divorce-and-remarriage`):* Erik and Astrid's marriage has ended and both have moved on, so each leaves a past-marriage spouse-ghost in the slot they occupied; their daughter Linnea's birth edge attaches to that marriage between the two ghosts.

*Worked example (`examples/04-adoption-and-belonging`):* Dalisay's canonical card sits in her current adoptive family, and her two past intimacies each emit a child-ghost: a past-bio child-ghost (solid edge) in her birth family's children row, and a past-adoption child-ghost (dashed edge) at the earlier, ended adoption.

### Ghosts are mute

A ghost connects only to the marriage or adoption bar it anchors. The person's other structural connections — their own birth family, other marriages, other adoptions — attach to the canonical card, never to any ghost.

### Source order

When a document describes multiple lineages with no intermarriage between them (separate connected components), the components arrange left-to-right by the source position of each component's first relevant declaration — a marriage if it has one, otherwise a person, otherwise the underlying marriage of a floating mini-component. Components mix freely in source order; there is no "orphans-last" bucket. The same rule applies recursively: nested birth-family sub-trees under one host arrange in joining-spouse declaration order, and siblings (canonical and ghost alike) within a children row arrange in declaration order. Author control is through declaration order — the same mechanism that controls the host rule.

## Visual vocabulary

| Element | Convention |
| --- | --- |
| **Canonical person card** | Solid border, opaque fill. Carries `name:` at minimum. |
| **Ghost person card** | Dotted border, faded fill, ↺ badge in a corner. Mute; anchors a past structural fact. |
| **Marriage edge** | The unified marriage connector: a thick stroke (~8.75px in the default preview theme), distinct from the thin birth / adoption edges (1.5px). For **monogamy**, a horizontal segment between the two adjacent spouse cards at their vertical mid-height; the couple's children drop from its midpoint. For a **polygamy hub**, one edge per concurrent marriage, routed from the hub card's bottom to the co-spouse card's top with the same orthogonal right-angle geometry as a birth edge, the marriages fanning out of the single hub-bottom point ([ADR-0020](./adr/0020-polygamy-hub-and-fan.md)). An **ended** (divorced) marriage renders translucent. |
| **Birth edge** | Solid line. Routes within a tree (marriage-edge midpoint → child below) or across trees (canonical card → a past or different-tree birth marriage). |
| **Adoption edge** | Dashed line. Same routing; line style alone distinguishes it from birth. |

## Evolving this document

This pattern co-evolves with the Kul language specification. When [`spec/`](../spec/README.md) gains a new construct — a new sub-statement, a new field that affects layout, a broadened semantic on an existing field, a new top-level statement — the responsible PR updates this document in the same change, because deciding how the new construct renders is part of shipping it. Extend the principles in place; do not restart from scratch. A new construct should slot in as a new application of an existing principle wherever possible (the way a future family-unit link would become a fourth ghost flavor), rather than as a parallel rule.

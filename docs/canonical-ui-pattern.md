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
Each person renders as exactly one *canonical* card. Visual duplicates (ghosts) exist only under the narrow conditions specified in P8 and P16.

### P3. First-declared spouse hosts
In a `marriage <id> <spouse_a> <spouse_b>` statement, `spouse_a` is the host; `spouse_b` joins the host's family tree. The host stays in their own canonical position; the joining spouse's canonical card sits adjacent to the host within the host's family. The joining spouse's birth-family connection becomes a cross-tree edge (P6).

*Rationale:* deterministic, gender-neutral, culturally neutral, author-controllable via declaration order. Alternatives considered and rejected: patrilineal default (gender-loaded); view-driven (incompatible with P7); explicit per-marriage tagging (heavier spec burden).

Defined normatively in [`spec/04-top-level-statements.md` §4.2](../spec/04-top-level-statements.md#42-marriage-statement); see also the `Host` glossary entry in [`CONTEXT.md`](../CONTEXT.md).

### P4. Adopted child lives in adoptive family
A child with both `birth` and `adoption` sub-statements lives — visually — in the adoptive family. The biological link renders as a cross-tree solid edge to the birth marriage (P5).

### P5. Connector style encodes link kind
Solid line = birth. Dashed line = adoption. Applies to every parent-child connector, whether it stays within a tree or crosses between them. Card border style is independent and encodes "is this a ghost?" — one dimension per visual axis.

### P6. Recursive nesting at inter-family connections
When a Kul document describes multiple intermarried families, each joining spouse's *birth family* sub-tree nests adjacent to the host tree at the joining spouse's connection point. Sub-trees nest further recursively.

*Rationale:* cross-tree edges stay short by construction; the layout makes inter-family structure visually obvious. Linear arrangement (trees side-by-side without nesting) was rejected because it produces long crossing edges.

### P7. Static, deterministic rendering
Given a Kul document, the canonical pattern produces exactly one layout. No interactivity, no view parameters, no user-selected focus. Surfaces may add interactivity on top of the canonical view; that is a presentational extension, not part of the pattern.

### P8. Canonical card at current intimacy; child-anchoring ghost for past marriages
A person's canonical card is positioned at their *current intimacy*: the host family of their most-recent un-ended marriage, or their birth family if no current marriage exists. A marriage is "ended" if it carries an `end:` field.

In the current Kul spec, `end:` corresponds to divorce only — death is not marked on the marriage but on the deceased spouse's `died:` field. A widow's marriage therefore has no `end:` and she remains in the host family. If a future spec extension broadens what `end:` can carry, the rule applies uniformly without changes.

When a past ended marriage produced children, the moved-out spouse leaves a **ghost** at the historical marriage location in the host family. The ghost is visually distinct (dotted border, faded fill, ↺ badge); its only purpose is to anchor the children's birth edges. Past marriages without children leave no visual trace (the marriage stays in the data; the renderer omits it).

### P9. Birth/adoption edges connect to the marriage bar
A child's birth or adoption edge attaches to the marriage bar of their parents' marriage — not to either parent card individually. This matches Kul's data model (`birth m_xxx` references the marriage id, not the parent ids).

### P10. Ghosts are mute
A ghost connects only to the marriage/adoption bar it anchors. The person's other structural connections (their own birth family, other marriages, other adoptions) attach to the canonical card, never to any ghost.

### P11. Absorb rule applies uniformly at every scale
The absorb rule (first-declared spouse hosts; joining spouse's card moves adjacent; their birth-edge becomes a cross-edge) applies identically across families, within a single family (cousin marriages), and at any structural scale. There is no special case for within-family marriages — the same mechanism produces a within-family cross-edge instead of a cross-tree one.

### P12. Multiple unrelated lineages arrange in source order
When a Kul document describes multiple lineages with no intermarriage between them (separate connected components in the graph), the components arrange left-to-right by the source position of each component's first-declared marriage.

*Rationale:* consistent with P3's source-order semantic. Author controls via declaration order — the same control mechanism used for the host rule.

### P13. Missing data renders as absence
Missing optional fields render as absence — no placeholders, no "Unknown" stubs, no allocated visual space. Required-field gaps are not a case the canonical pattern designs for (R03 ensures valid documents carry name and gender).

Orphan persons (declared with no edges of any kind) render as single-card components and arrange with other components per P12.

### P14. Scale-invariant pattern
The pattern is scale-invariant. The same rules produce coherent layouts at 5 persons, 50 persons, or 5,000. Level-of-detail, zoom, panning, virtualization, and aggregation are renderer-side policy and not part of the canonical pattern.

*Rationale:* pattern vs. presentation separation. The pattern produces a structural output; renderers innovate on level-of-detail using the pattern's natural hierarchy (family → branch → couple → child) and generational y-axis.

### P15. Uniform card; name minimum; gender not visually encoded
A person card is a uniform shape carrying at minimum the person's `name:`. Other Kul fields may appear per renderer policy.

Gender is **not** visually encoded by card shape, color, or icon in the canonical pattern. If a renderer chooses to surface gender, it does so via text label using Kul's three values (`male | female | other`).

The only canonical card-appearance variation is canonical (solid border, full opacity) vs. ghost (dotted border, faded fill, ↺ badge).

*Rationale:* cultural / political neutrality; visual uniformity focuses attention on structure; composability lets renderers opt into richer chrome without forking the pattern.

### P16. Most-recent adoption is canonical; past adoptive families get a child-anchoring ghost
When a person has multiple `adoption` sub-statements, the most recent (by `start:` date, falling back to source order) determines the canonical adoptive family; the child's canonical card lives there. Past adoptive families display a *ghost of the child* connected to the past adoption marriage by a dashed edge. The ghost is mute (P10).

A biological `birth` link, if present, renders as a solid cross-edge from the canonical card to the birth marriage (P5).

*Rationale:* mirror of P8 with the adoption side instead of the marriage side. Same primitive (ghost), same purpose (anchor a past structural fact), same visual vocabulary.

## Visual vocabulary

| Element | Convention |
| --- | --- |
| **Canonical person card** | Solid border, opaque fill. Carries `name:` at minimum. |
| **Ghost person card** | Dotted border, faded fill, ↺ badge in corner. Mute. Anchors a past structural fact. |
| **Marriage bar** | Small rectangle between two adjacent spouses. |
| **Adoption bar** | Same shape as marriage bar; semantically, the join-point for an adoption sub-statement. |
| **Birth edge** | Solid line. Routes within a tree (marriage bar → child below) or across trees (canonical card → past or different-tree birth marriage). |
| **Adoption edge** | Dashed line. Same routing rules; edge style alone distinguishes from birth. |

## Amending this document

This pattern co-evolves with the Kul language specification. When [`spec/`](../spec/README.md) gains a new construct (a new sub-statement, a new field that affects layout, a broadened semantic on an existing field, a new top-level statement), the responsible PR updates this document in the same change — deciding how the new construct renders is part of shipping it. Amend the principles; don't restart from scratch.

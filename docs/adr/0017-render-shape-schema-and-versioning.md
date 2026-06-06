# ADR 0017 — `RenderShape` schema and versioning

**Status:** Accepted
**Date:** 2026-06-07
**Deciders:** owner

## Context

[ADR-0016](./0016-visualization-pipeline-crate-boundaries.md) places the canonical-UI-pattern projection in `kul-render` and pins its two public surfaces. The value those surfaces emit — `RenderShape` — is this ADR's subject.

Three shapes were on the table:

1. **Flat nodes + flat edges (Cytoscape-style).** One bag of cards, one bag of edges, type discriminators on each. Maximum interop, minimum structure — a consumer re-derives every layout-meaningful fact (component grouping, generation row, ghost vs canonical, host vs child) by walking discriminators.
2. **Fully positioned.** Cards with `x/y`, edges with routed polylines. Smallest contract, but bakes layout policy into the projection and forecloses renderer innovation.
3. **Hierarchical card slots + flat edge list (chosen).** A tree whose shape *is* the canonical pattern's spatial hierarchy (family → couple → child), with edges flat because they cross the hierarchy freely.

Two facts shaped the choice. First, the canonical pattern has a natural hierarchy (family → branch → couple → child); encoding it in the data lets a consumer walk it directly rather than rebuild it from a flat node list — which matters for the scale-invariance principle's level-of-detail story. Second, every ghost-and-canonical decision the projection makes must be a *field* in the shape, not something a consumer re-derives — otherwise the algorithm has merely moved one layer up.

A third fact shapes the component root specifically. In a marriage the host is the conceptual root of the marriage's placement: the canonical pattern's **current-intimacy placement** anchors a marriage at the host's birth-family slot. The two spouses are not peers of the bar. And **one canonical card per person** must hold for a person with two or more concurrent un-ended marriages (a polygamy hub) — one card, N marriages — which a marriage-rooted component cannot express without inventing a parallel primitive.

## Decision

`RenderShape` is the top-level value `kul-render` emits: untagged success/failure variants discriminated by an `ok` boolean, matching the [`ExportEnvelope`](../../crates/kul-core/src/export.rs) precedent ([ADR-0009](./0009-export-strict-on-diagnostics.md)). On success:

```text
RenderShape::Success {
    ok: true,
    schema: u32,                // RENDER_SCHEMA_VERSION
    kul: String,                // language version, mirrored from input envelope
    components: Vec<Component>,  // top-level layout components in source order
    edges: Vec<Edge>,           // every birth + adoption parent-child edge
}
```

### Components are rooted at a person, not a marriage

A `Component` is one of:

- `FamilyTree { root: Box<PersonCard> }` — a person and their descendants. The root `PersonCard` is the outermost canonical host of the component.
- `OrphanPerson { card: Box<CardSlot> }` — a single canonical card (a person declared with no edges, plus the lone-card fallback of the current-intimacy chain).

A `PersonCard` carries `slot: CardSlot` plus `hosted_marriages: Vec<MarriageBranch>`. A monogamous host carries a `Vec` of length one; a polygamy hub carries N — a length-one `Vec` is not a special case, so there is exactly one structural primitive for "a person with hosted marriages" regardless of N.

A `MarriageBranch` holds a `MarriageBar` plus a flat `Vec<PersonCard>` of children; each child `PersonCard` may itself host marriages, recursing through one host-lineage tree. A `MarriageBar` carries the bar metadata — `id`, `host_id`, `joining_id`, dates, end-reason, and an `ended` boolean reified from `end:` presence — plus the joining slot. The bar carries **no `host_slot`**: the host face of every bar is the parent `PersonCard.slot` in the tree, implicit by position rather than duplicated on the bar. (`host_id` stays, for consumers cross-referencing by id.) A joining spouse's bio family is rendered as its own component reached through past-bio child-ghost + name pairing ([ADR-0019](./0019-ghost-model-and-bio-anchor.md)), not inlined under the bar.

The root `PersonCard.slot.kind` is normally `Canonical`. A past-ended floating bar — one whose host and joining spouse have both moved on, so the bar exists only to anchor a child's edge — roots its component at a **ghost** `PersonCard` whose `slot.kind = Ghost(PastMarriage)` and whose `hosted_marriages` carries the past-ended bar. Every `FamilyTree` is thus rooted at a `PersonCard`, canonical or ghost; the data permits the same person to appear as a canonical `PersonCard` in one component and a ghost `PersonCard` rooting another, mirroring `CardSlot`-level canonical/ghost duplication.

### Slots, ghosts, and edges

A `CardSlot` carries `personId`, `kind` (`Canonical` or `Ghost { reason }`), the generation index, and the person's display fields (`name`, `gender`, `family`, `given`, `born`, `died`, mirrored from the input envelope). `GhostReason` is one of `PastMarriage`, `PastAdoption`, `PastBirth`; which one a ghost carries, and when a ghost is emitted at all, is the ghost model in [ADR-0019](./0019-ghost-model-and-bio-anchor.md). An `Edge` carries `kind` (`birth` / `adoption`), `childId`, `marriageId`, and (for adoptions) the `start:` / `end:` dates. Edges are flat because cross-tree links cross the hierarchy freely; a router walks `shape.edges`, not the tree.

### Generation and canonical selection

Generation indices are pre-computed by fixpoint relaxation over the canonical-family graph (bio parents, or adoptive parents when a canonical adoption exists): roots at 0, `child = max(canonical-family spouses' generations) + 1`. The export envelope is acyclic per [R13](../../spec/07-validation-rules.md), so the fixpoint converges in at most `persons.len()` iterations. This generation is **structural** — it describes data-level kinship depth, not canvas placement; where the generations land on a canvas is layout policy ([ADR-0018](./0018-canonical-layout-algorithm.md)).

Which un-ended marriage anchors a person's canonical card is decided by **first-declared un-ended participation** — the first marriage by source order across the union of the marriages the person hosts and joins. This matches the current-intimacy placement principle.

### Versioning

`RENDER_SCHEMA_VERSION` is a `pub const u32` exported from `kul-render`; the current value is `3`. Bumping follows [ADR-0010](./0010-export-schema-versioning.md)'s discipline transposed to the render shape:

- **Bump** when a consumer might silently mis-represent data by ignoring a new construct — a new top-level layout primitive, or an existing field's semantics changing incompatibly.
- **No bump** for forward-compatible additions: a new optional slot field, a new `GhostReason` value, a new component-kind variant a consumer can treat as opaque. The types use `#[serde(skip_serializing_if = "Option::is_none")]` on additions so older consumers keep parsing.

The `kul` language-version string passes through verbatim from the input envelope, purely informational, same role as in [ADR-0010](./0010-export-schema-versioning.md).

## Consequences

- **Consumers read, don't re-derive.** Which spouse is canonical, which slot is a ghost and why, which component a card belongs to, which generation hosts a bar — each is a field. A surface renderer is a walker, not a re-implementer of the pattern.
- **The hierarchy supports level-of-detail.** A renderer collapsing a sub-tree or virtualizing an off-screen branch walks the existing tree; it does not first rebuild it from discriminators on a flat list.
- **One root shape, every N.** Monogamy, polygamy, and the past-ended floating bar all flow through `FamilyTree { root: PersonCard }`. The consumer pattern-matches one component shape; the polygamy adapter ([ADR-0020](./0020-polygamy-hub-and-fan.md)) reads N bars off one card.
- **`ended` is reified, not recomputed.** Death does not end a marriage — only `end:` does — so the `ended` predicate that current-intimacy placement depends on is computed once in the projection and tested at the pattern boundary, not in every consumer.
- **`Box` discipline matches `clippy::large_enum_variant`.** The boxed `root`/`card` keep `Component` compact; serde flattens both, so the wire shape is unchanged.

## Anti-suggestions (do not re-propose)

- **"Flatten the tree into a `Vec<Card>` plus a `Vec<MarriageBar>`."** Loses the spatial hierarchy level-of-detail relies on, and forces consumers to re-derive parent / sibling / child relationships from cross-id lookups.
- **"Include `x` and `y` on every card."** Pre-computed positions foreclose downstream layout innovation (level-of-detail, virtualization, alternative algorithms). The pattern is structural, not positional; positions are layout policy in `kul-layout`.
- **"Add a `PolygamyHub` parallel variant alongside `FamilyTree`."** Two structural primitives for "a person with hosted marriages" — one for N=1, one for N>1 — doubles the consumer's pattern-match surface for no benefit. `PersonCard` already carries `hosted_marriages: Vec<MarriageBranch>`, and a `Vec` of length 1 is not a special case.
- **"Use a second root shape for past-ended floating bars (a `MarriageBranch` root, or a synthesized `GhostBarRoot` primitive)."** Two root shapes for one `ComponentKind` push the discriminator into consumer code instead of the data. The ghost-rooted `PersonCard` gives one uniform shape and reuses the `Ghost { reason: PastMarriage }` vocabulary that already exists.
- **"Drop the `ended` boolean — let consumers compute it from `end`."** `ended` is a load-bearing placement predicate; reifying it keeps the mechanic readable and tested at the pattern boundary.
- **"Replace the `Ghost { reason }` discriminator with a flat `Faded` boolean," or "generalise the reasons into `Past { intimacy }`."** The three reasons carry different downstream semantics (which edge anchors at the ghost; whether the bar is mute) and do not share a `Past`-shaped payload. Three flat variants stay pattern-matchable in one line; a nested form needs an extra match arm at every consumer for zero structural benefit; a flat boolean loses the distinction entirely.
- **"Inline a joining spouse's bio family into the `MarriageBar` (as a nested sub-tree, child `PersonCard`, or any other shape)."** A joining spouse's bio family is a distinct host-lineage tree and renders as its own component. Cross-family kinship reads through past-bio child-ghost + shared name identity ([ADR-0019](./0019-ghost-model-and-bio-anchor.md)); spatial inlining loses the symmetry and re-introduces the multi-rooted-component problem that motivated this shape.
- **"Use semantic versioning (`schema: \"1.0.0\"` or `schema: 1.1`)."** Per [ADR-0010](./0010-export-schema-versioning.md), the schema integer is a discriminator at the consumer boundary, not a release identifier.
- **"Add a Visitor trait over the component shape."** As with [ADR-0001](./0001-resolved-document-as-query-seam.md), `Component` is a two-variant enum; pattern matches stay clearer than a visitor at this scale.

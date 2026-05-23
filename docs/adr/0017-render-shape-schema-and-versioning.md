# ADR 0017 — `RenderShape` schema and versioning

**Status:** Accepted
**Date:** 2026-05-23
**Deciders:** owner

## Context

[ADR-0016](./0016-kul-render-crate-boundary.md) places the canonical-UI-pattern projection in a new `kul-render` crate and pins the two public surfaces (`compute(&CheckResult)`, `transform(&ExportEnvelope)`). It does not say what the output value looks like. That contract is this ADR's subject.

Several shapes were on the table:

1. **Flat nodes + flat edges (Cytoscape-style).** One bag of cards, one bag of edges, type discriminators on each. Maximum interop, minimum structural information — a surface renderer has to re-derive every layout-meaningful fact (component grouping, generation row, ghost vs canonical, host vs child) by walking discriminators.
2. **Fully positioned (layout already done).** Cards with `x/y` coordinates, edges with routed polylines. Smallest contract but bakes layout policy into the projection and forecloses renderer innovation (level-of-detail, virtualization, alternative layout algorithms per [P14](../canonical-ui-pattern.md#p14-scale-invariant-pattern)).
3. **Hierarchical card slots + flat edge list (chosen).** A tree of `Component → MarriageBranch → PersonCard → (hosted) MarriageBranch → …` so the spatial hierarchy the canonical pattern produces is the data shape. Edges stay flat because they cross the hierarchy freely (cross-tree birth/adoption edges).

Two further inputs shaped the decision. First, the canonical UI pattern has a natural hierarchy — family → branch → couple → child (P14's "renderers innovate on level-of-detail using the pattern's natural hierarchy"). Encoding that hierarchy in the data lets renderers walk it directly. Second, every ghost-and-canonical decision the projection makes (P8 ended-marriage anchors, P16 past-adoption child-ghosts, P6 recursive nesting termination per P11) must be visible without consumers re-deriving — otherwise we've just moved the algorithm one layer up.

## Decision

`RenderShape` is the top-level value `kul-render` emits. Untagged success/failure variants discriminated by an `ok` boolean, matching the [`ExportEnvelope`](../../crates/kul-core/src/export.rs) precedent ([ADR-0009](./0009-export-strict-on-diagnostics.md)). On success:

```text
RenderShape::Success {
    ok: true,
    schema: u32,                   // RENDER_SCHEMA_VERSION
    kul: String,                   // language version, mirrored from input envelope
    components: Vec<Component>,    // top-level layout components in P12 source order
    edges: Vec<Edge>,              // every birth + adoption parent-child edge
}
```

A `Component` is one of:

- `FamilyTree { root: MarriageBranch }` — a marriage and its descendants. The root marriage is the outer-most layer of the component's nesting (either a P8 floating mini-comp whose host has no birth family, or a marriage whose host's canonical family is reached through a P6 nest).
- `OrphanPerson { card: CardSlot }` — a single canonical card (P13 declared-with-no-edges orphans plus the P8 fallback case from `examples/03-three-generations/`).

A `MarriageBranch` holds a `MarriageBar` plus a flat list of `PersonCard` children. Each `PersonCard` carries one `CardSlot` (canonical or ghost) plus any marriages that branch from its slot (P11 absorb rule applied uniformly). A `MarriageBar` carries the bar metadata (id, host/joining ids, dates, end-reason, an `ended` boolean reified from `end:` presence) plus the two slot positions and an optional `joining_nested_birth_family: Box<MarriageBranch>` for the P6 cross-component case.

A `CardSlot` carries `personId`, `kind` (`canonical` or `ghost { reason }` where `reason` is `pastMarriage` or `pastAdoption`), the generation index, and the person's display fields (`name`, `gender`, `family`, `given`, `born`, `died` — mirrored from the input envelope). An `Edge` carries `kind` (`birth` / `adoption`), `childId`, `marriageId`, and (for adoptions) the `start:` / `end:` dates.

Generation indices are pre-computed by fixpoint relaxation over the canonical-family graph (bio parents, or adoptive parents if a canonical adoption exists) — roots at 0, child = max(canonical-family-spouses' gens) + 1. The export envelope is acyclic per [R13](../../spec/07-validation-rules.md), so the fixpoint converges in at most `persons.len()` iterations.

`RENDER_SCHEMA_VERSION` is a `pub const u32` exported from `kul-render`. The current value is `1`. Bumping follows [ADR-0010](./0010-export-schema-versioning.md)'s discipline transposed to the render shape:

- **Schema bump** when downstream consumers might silently mis-represent data by ignoring a new construct. Examples: a new top-level layout primitive (e.g. a `Cluster` variant alongside `Component`); semantics of an existing field change incompatibly (e.g. `host_slot` becomes a list).
- **No bump** for forward-compatible additions: new optional field on a slot, new ghost-reason value, new component-kind variant that consumers can fall back to as opaque. The `RenderShape` types use `#[serde(skip_serializing_if = "Option::is_none")]` on additions so older consumers keep parsing.

The `kul` language-version string is passed through verbatim from the input envelope — same role as in [ADR-0010](./0010-export-schema-versioning.md), purely informational.

## Consequences

- **Consumers read, don't re-derive.** Every load-bearing decision the canonical UI pattern makes — which spouse is canonical, which slot is a ghost and why, which component a card belongs to, which generation row hosts the bar — is a field in the shape. A surface renderer becomes a walker, not a re-implementer of the pattern.
- **The hierarchy matches P14's "natural hierarchy" rationale.** A renderer doing level-of-detail (collapse a sub-tree, virtualize an off-screen branch) walks the existing tree; it doesn't have to first re-build it from discriminators on a flat node list.
- **Flat edges keep cross-tree links cheap to enumerate.** A cross-tree edge router walks `shape.edges`, not the tree. Routing geometry is renderer policy ([ADR-0008](./0008-export-kinship-native-shape.md)'s consequence for cross-references).
- **Two schema-version axes, two independent versions.** The export envelope's `schema` and the render shape's `schema` are independent — a schema-1 export feeding a schema-1 render is the current contract; either can bump without forcing the other to.
- **Box discipline matches `clippy::large_enum_variant`.** `Component::FamilyTree`'s `root: Box<MarriageBranch>` and `Component::OrphanPerson`'s `card: Box<CardSlot>` keep the enum compact; serde flattens both transparently so the JSON wire shape is unchanged.

## Anti-suggestions (do not re-propose)

- **"Flatten the tree into a `Vec<Card>` plus a `Vec<MarriageBar>`."** Loses the spatial hierarchy P14 explicitly relies on, and forces consumers to re-derive parent / sibling / child relationships from cross-id lookups.
- **"Include `x` and `y` on every card."** Pre-computed positions would foreclose downstream layout innovation (level-of-detail, virtualization, alternative algorithms per [P14](../canonical-ui-pattern.md#p14-scale-invariant-pattern)). The pattern is structural, not positional; positions are renderer policy.
- **"Drop the `ended` boolean on `MarriageBar` — let consumers compute it from `end`."** `ended` is a load-bearing P8 predicate (death does not end a marriage, only `end:` does); reifying it in the projection keeps the P8 mechanic readable and tested at the canonical-UI-pattern boundary instead of in every consumer.
- **"Replace the `Ghost { reason }` discriminator with a flat `Faded` boolean."** `pastMarriage` and `pastAdoption` ghosts carry different downstream semantics (which edge anchors at the ghost; whether the bar is mute per P10) — collapsing them loses that.
- **"Inline the joining spouse's birth family directly into `MarriageBar.joining_slot`."** Mixes "slot" (the card position) with "sub-tree" (an entire MarriageBranch). The current layout — slot at one level, optional nested sub-tree at the same level but a sibling field — keeps the two concepts separable and serde-renamable.
- **"Use semantic versioning (`schema: \"1.0.0\"`)."** Same reason as [ADR-0010](./0010-export-schema-versioning.md): the schema integer is a discriminator at the consumer boundary, not a release identifier.

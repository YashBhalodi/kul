# ADR 0008 — Export uses a kinship-native graph shape

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

The export's job is to project a Kula document into a JSON value a downstream consumer can render. That projection has many shapes to choose from — and the choice is load-bearing because every consumer (web visualizer, VSCode webview, scripts, generators) inherits it. Three shapes were on the table:

1. **Kinship-native.** Three flat collections — `persons`, `marriages`, `parenthood_links` — that mirror the language's primitives one-to-one. Cross-references are by id; nothing is embedded; nothing is derived. A consumer indexing by id gets a dictionary lookup; a consumer rendering the family tree composes derived views (children-of, siblings-of, descendants-of) on top.
2. **Generic-graph (nodes + edges).** A flat `nodes` array plus a flat `edges` array, with type tags on each. Loadable into Cytoscape, Sigma.js, vis-network, Gephi without writing a custom adapter. Pays for that interop with a layer of indirection — all kinship-meaningful structure is encoded as `type` discriminators on opaque nodes/edges.
3. **Embedded-tree.** Persons carry their spouses inline; marriages carry their children inline. One round-trip gives the consumer the whole family. Pays for that convenience with cycles (a child appears under both parents), deep nesting, and mandatory choices about which relationship "owns" which entity.

Two further pressures shaped the decision. First, the language is single-context and small — every primitive (Person, Marriage, Birth, Adoption, Parenthood Link) has a name in [`CONTEXT.md`](../../CONTEXT.md), and the spec already defines what derives from what. The export should preserve those names so the JSON is read with the same vocabulary as the source. Second, a separate Cytoscape transformer is wanted anyway as an opt-in `--format cytoscape` mode — the existence of that transformer makes shape (1) strictly more general than shape (2), since (2) can be derived from (1) but not vice versa.

## Decision

The canonical export shape is **kinship-native**. Three flat top-level collections:

- `persons` — every declared person.
- `marriages` — every declared marriage.
- `parenthood_links` — one entry per `birth` and `adoption` sub-statement, carrying `marriage_id`, `child_id`, and a `kind` tag (`"biological"` or `"adoptive"`).

Cross-references are id-only. Embedded objects do not appear (the marriage carries spouse ids, not spouse objects). Derived projections do not appear (no `person.children`, no `person.siblings`, no `marriage.duration`). Consumers compose those views over the flat collections.

The Cytoscape format ships as a secondary, opt-in projection (`--format cytoscape`) — a pure transformer over the kinship-native graph. It cannot drift from the canonical shape because it is derived from it.

## Consequences

- The schema vocabulary (`persons`, `marriages`, `parenthood_links`, `marriage_id`, `child_id`, `kind`) matches the project's domain glossary in [`CONTEXT.md`](../../CONTEXT.md). A reader who has read the spec recognises every key.
- The schema is stable under additive language change. New person fields (e.g. a future `nickname:`) land as new optional keys on the person object; new sub-statement kinds (e.g. surrogacy) land as a new `kind` value on the parenthood-link object. No existing field changes shape — see ADR-0010 for the schema-versioning policy.
- The id-only cross-reference rule means the JSON is acyclic, which sidesteps one whole class of consumer bugs (cycle detection in serializers, stack-overflowing pretty-printers) and lets consumers index-by-id with a one-liner.
- Promoting parenthood links to a first-class collection makes the queries downstream consumers want most ("all children of marriage M", "all parents of person P") into one-line filters. The alternative (children inlined under marriage, parents not stored) forces consumers to either index manually or denormalize.
- Marriage-as-relationship (with two spouse ids) rather than marriage-as-edge keeps the marriage a typed kinship entity that can carry its own fields (`start`, `end`, `end_reason`). The Cytoscape transformer promotes it to a node specifically because it has those fields.

## Anti-suggestions (do not re-propose)

- **"Use a generic nodes/edges shape as the canonical form."** A Cytoscape-shaped canonical export forces every consumer that wants to ask kinship questions to first reverse the transform. The kinship-native shape is more general and more readable. The Cytoscape mode lives as a derived transformer for exactly the consumers who want it.
- **"Embed children inside marriage objects."** Creates an asymmetry between bio and adoptive parents (whose marriage do you nest under?), pushes a redundancy-vs-cycle choice onto every consumer, and makes the schema sensitive to how the source happens to be ordered. The flat parenthood-link collection sidesteps all of this.
- **"Inline spouse objects inside the marriage object."** Same problem as embedded children. Consumers that want a denormalized view build it once over the flat collections; consumers that don't want it pay nothing.
- **"Add a `person.children` derived collection 'as a convenience'."** Drags every kinship question down a one-way path. Once a consumer reads `person.children`, the schema owes them every other derived view (`siblings`, `descendants`, `cousins`, `in_laws`). The single answer this ADR commits to is: derived views are consumer-side. The foundation does not freeze any kinship-derivation semantics.
- **"Make the schema GEDCOM-shaped for interop."** [`docs/vision.md`](../vision.md) is explicit that Kula is intentionally not GEDCOM-compatible. A GEDCOM bridge is a separate downstream transformer if it ever happens.

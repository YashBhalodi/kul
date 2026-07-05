# ADR 0026 — The relationship descriptor: terminology-neutral, maximally discriminating, path-identity, set-shaped

**Status:** Accepted
**Date:** 2026-07-05
**Deciders:** owner

## Context

The kinship query engine ([ADR-0025](./0025-kinship-query-engine-contract-and-traversal.md)) answers "who is this person's …?" and "how are X and Y related?". The *answer* to both is a structured record of a relationship. This ADR pins that record — the **relationship descriptor** — and the rule for how many of them an answer contains.

The forcing function is the deliberately-thin future layer above the descriptor: a community-growable, additive map from `RelationshipDescriptor → culture-specific term` ("sister-in-law" | "bhabhi" | *chacha* | *tau* | …). That layer must be able to grow as **pure data**, without ever changing the engine. That is only possible if the descriptor already carries every distinction any target culture's terminology could key on. Under-modeling the descriptor now forces an engine change later and breaks the additivity promise — so the descriptor is designed richer than English needs.

## Decision

### The descriptor is terminology-neutral and maximally discriminating

The engine **never emits a kinship word.** It emits a structured descriptor; the app (via the future terminology layer) renders the word. The descriptor carries: endpoint ids and genders; `classification` (`self` | lineal `{role, generations}` | collateral `{up, down, cousinDegree, removed}`); `edgeNature` (`blood | adoptive`); `affinity` (`blood | step | inLaw`); `sharing` (`full | half | notApplicable`); `side` (`maternal | paternal | other | both | notApplicable`); two seniorities; and the lossless [path backbone](../../CONTEXT.md).

Two design choices make it *maximally* discriminating:

- **`edgeNature` × `affinity` are split**, replacing an earlier flat `full|half|adoptive|step` enum that conflated independent dimensions and could not express a half-adoptive or full-adoptive sibling. `cousinDegree` / `removed` are **materialized numbers** in the serialized form — the formulas (`min(up,down)−1`, `|up−down|`) are exactly the off-by-one traps a consumer would fumble.
- **Two seniority fields, not one.** `seniority` compares the alter to the ego; `apexSeniority` compares the two branch-siblings at a collateral apex. A single ego-relative field cannot express *chacha* vs *tau* (which compare the uncle to ego's *father*), so the descriptor carries both — even though this lineal slice populates only the endpoint field and leaves `apexSeniority` at `notApplicable`.

Every dimension is **derived, never guessed**. `side` is derived from the path's routing (with an explicit `other` value when the linking parent's gender is `other`, because the grammar permits it). Seniority rides the toolchain's single `before_strict` interval comparison, so a missing or too-coarse date yields `unknown`, never an invented order.

### `unknown` and `notApplicable` are explicit, and never conflated

Both are **explicit enum values, never `null` or an absent field.** `unknown` = the data is insufficient to decide (missing dates, overlapping intervals). `notApplicable` = the dimension does not apply to this path shape (no sibling junction on a lineal path; no "side" for a direct parent). JS collapses `null` and `undefined` too easily, and the distinction is load-bearing, so it is spelled out in the wire form. Serialization is pinned overall: camelCase, internally-tagged unions (so TS consumers `switch` on a discriminant), materialized derived numbers, and path hops carrying ids not payloads.

### Descriptor identity is path identity; output is set-shaped; no collapsing

**One descriptor per distinct relationship path.** The result set is not collapsed by classification, and the core never picks a "primary" relationship. Two consequences:

- A person reachable two ways — e.g. as both a bio and an adoptive ancestor via different marriages — yields **two members with distinct backbones**. Collapsing would hide a true relationship; picking a winner would embed a UX/cultural ranking in the core. Both are forbidden.
- The full kinship graph is cyclic in practice (consanguineous marriage, adoption-into-relatives), so two people are often related several ways at once. Path identity extends the same honesty to same-classification multiplicity (double cousins, in a later slice).

Consumers who want "just first cousins" collapse on the normalized fields themselves — a UX decision, kept out of the core. Because the set is the contract, **member order is deterministic** (snapshots depend on it): by alter person id, then path hop count, then serialized backbone.

### The path backbone is lossless ground truth

Every descriptor carries the ordered hop sequence it was derived from — direction, edge tag, the person id landed on, and that person's gender (and, for marriage hops, the marriage id + status + end reason). It is the escape hatch: a distinction nobody anticipated is still recoverable from the backbone without an engine change. Hops carry **ids, never entity payloads** — consumers hydrate on demand via the detail lookups ([ADR-0024](./0024-query-seam-and-envelope.md)), so query results never duplicate the export's person shape.

## Consequences

- The future terminology layer can ship as pure additive data over a stable descriptor; adding a culture pack never touches the engine.
- The descriptor pays for full discrimination (two seniorities, edge/affinity split, `other` side) even though this slice ships no culture pack and populates only the lineal subset — that cost is deliberate, because retrofitting a distinction later would break additivity.
- Set-shaped, path-identity output means a consumer building a consanguinity-aware or double-relationship view never loses a true tie to engine-side collapsing.
- Deterministic ordering makes the descriptor serialization snapshot-testable, which is how kinship correctness is pinned once at the core seam.

## Anti-suggestions (do not re-propose)

- **"Collapse same-classification descriptors and return one row per person — apps don't want duplicates."** That hides real relationships (double cousins; bio-and-adoptive ancestor) and forces the core to rank. Path identity is the contract; collapsing is the consumer's UX choice.
- **"Model side/seniority as `null` when they don't apply — it's less enum noise."** `null` conflates "unknown" with "not applicable", and JS erases the difference further. Both are explicit enum values.
- **"Ship a flat `full | half | adoptive | step` consanguinity enum — it's what English needs."** It cannot express half-adoptive or full-adoptive siblings, and English is not the target ceiling. `edgeNature` and `affinity` are independent dimensions.
- **"One ego-relative seniority field is enough."** It cannot express *chacha* vs *tau*, which compare the uncle to ego's father. The descriptor carries the apex-junction seniority too, reserved `notApplicable` until collateral paths populate it.
- **"Have the engine emit the kinship term — it already knows the relationship."** The moment the core emits 'grandmother' it becomes the first culture pack, shipped by accident, and the additive-terminology promise is dead. The engine emits the descriptor; the app renders the word — including in `kul query` human output.

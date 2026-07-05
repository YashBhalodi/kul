# ADR 0028 — Relationship resolution: nearest-common-ancestor bounding, unbounded lineal detection, and honest emptiness

**Status:** Accepted
**Date:** 2026-07-05
**Deciders:** owner

## Context

The kinship query engine ([ADR-0025](./0025-kinship-query-engine-contract-and-traversal.md)), the relationship descriptor ([ADR-0026](./0026-relationship-descriptor-and-path-identity.md)), and the affinal traversal ([ADR-0027](./0027-affinal-traversal-ceiling-and-step-subsumption.md)) built the kin-set half of the engine: from one anchor + a pattern, the *set* of related persons. This ADR pins the second half — **relationship resolution**, the two-anchor question (issue [#259](https://github.com/YashBhalodi/kul/issues/259), PRD 0005): given persons `x` and `y`, return *all* the ways they are related.

Three decisions here are non-obvious enough to outlive the code a future agent might otherwise "simplify" away.

## Decision

### Resolution shares the kin-set engine — a separate call, never a forked traversal

`resolve(x, y, config)` is a **separate two-anchor entry point** (it answers a different question than the kin-set [Query value](../../CONTEXT.md#query-value), so it is not a pipeline stage), but it is **not a second engine**. It reuses the same `AffinalWalk` (generalized with a pluggable emit-gate and a per-segment cap), the same couple-apex canonicalization and backbone de-duplication, the same [step subsumption](../../CONTEXT.md#step-subsumption), and the same `RelationshipDescriptor::derive`. Resolution and kin-sets therefore share one vocabulary and one derivation: a tie resolution reports is byte-for-byte the descriptor the corresponding kin-set query would report. Forking the traversal would have been two engines to keep in agreement forever.

`x == y` resolves to a single `self` descriptor with an empty path — derived through the same `derive`, so `self` never forks the vocabulary either. (The reflexive `self` is the *only* empty-path descriptor: no kin-set query emits one, because the anchor excludes itself. Its `seniority` is `notApplicable`, distinguishing it from a `self`-*classification* spouse — reached by `across` hops — whose seniority is real.)

### Nearest-common-ancestor bounding is per **blood segment**, and it is a budget, not semantics

Resolution's one knob is `maxApexGenerations` (default **5**), bounding **each blood segment's** ascent and descent — a nearest-common-ancestor bound, not a total-path-length bound. Five reaches through fourth cousins, a strict superset of every lexicalized kinship term (cultures run out by third cousins), while cutting off the remote-ancestor haystack.

It is a *search budget* on the [semantics-vs-budget line](./0027-affinal-traversal-ceiling-and-step-subsumption.md): two apps may legitimately look differently far, so it is a caller knob. The 2-affinal-hop **ceiling** (ADR-0027) is the opposite — *semantics*, fixed, never configurable, because no culture names a three-affinal-hop tie. A bigger budget can reveal more of the *same* relationship space; it can never change its definition. The two apps must never disagree about *what relationships exist*, only about how far they chose to look.

A consequence of the *per-segment* (rather than *total*) budget: an affinal hop opens a fresh blood segment, so a marriage detour can "reset" the ascent counter. First cousins (apex up 2 / down 2) are reachable even at budget 1 by routing one affinal hop through a co-parent's marriage on each branch — a real, if convoluted, tie the tool reports honestly (never lying by omission). Second cousins (up 3 / down 3) would need four such resets, beyond the two-hop ceiling, so they are genuinely `noneWithinBounds` at budget 1.

### Pure-lineal ties are detected unbounded, regardless of the budget

A direct ancestor/descendant tie — a single blood segment of pure `up+` or `down+`, zero affinal hops — is detected by a **separate, uncapped** cycle-guarded parent-chain walk over both edge kinds, *in addition to* the budgeted general enumeration. The remote-ancestor haystack the budget guards against is a *collateral* phenomenon; a direct-line check is cheap, and `noneWithinBounds` must **never** hide a recorded direct-line tie. So `resolve(great-great-grandchild, ancestor)` finds the lineal descriptor even at budget 1.

### Honest emptiness: `disconnected` vs `noneWithinBounds`

Resolution returns a **result object, not a bare set**: the descriptor list, plus — **only when the list is empty** — an explicit [emptiness reason](../../CONTEXT.md#emptiness-reason):

- `disconnected` — `x` and `y` lie in different connected components of the **full relation graph** (undirected reachability over every parent-child edge of both kinds plus every spouse edge). Raising the budget can never help.
- `noneWithinBounds` — same component, but nothing is derivable under the semantics (the affinal ceiling, step subsumption) and the current budget. A bigger budget might reveal a tie.

Collapsing the two into a bare empty list would invite an app to render "not related" when the truth is "not related as far as we looked". The distinction is the product.

## Consequences

- `AffinalWalk` gains a `segment_cap` (per-segment budget, resolution-only; kin-set leaves it `None` and keeps bounding the *total* ascent/descent) and a pluggable `emit_gate` closure (the kin-set classification gate, or an accept-all gate with an alter-identity filter for resolution). Kin-set behaviour and snapshots are unchanged.
- `RelationshipDescriptor::derive` sets `seniority = notApplicable` for the empty path (the reflexive `self`), leaving every non-empty `self`-classification (spouse, co-spouse) with a real seniority. No kin-set snapshot changes (none emits an empty path).
- Resolution's deterministic order is (path hop count) → (serialized backbone); there is no alter-id key (every descriptor shares the alter `y`), unlike the kin-set member order.
- The surface is additive: WASM `queryResolve(files, manifest, xId, yId, config?)` → `QueryEnvelope<ResolveResult>` (omitted config = default budget), and CLI `kul query rel <x> <y> [--max-generations N] [--format human|json]`. An empty result exits 0 (an empty result is an answer); a bad id is a typed error / nonzero. `ResolveResult`, `ResolveConfig`, and `EmptyReason` ride the committed-tsify discipline ([ADR-0012](./0012-tsify-derived-types-committed-and-diffed.md)).
- CLI human output stays terminology-neutral — the descriptor's structured facts plus the hop-by-hop path with ids and display names, marriage status on `across` hops; never a kinship word (that would ship a culture pack by accident).

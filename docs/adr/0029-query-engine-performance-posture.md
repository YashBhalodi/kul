# ADR 0029 — Query-engine performance posture: on-demand, no indices, no caching, budget-as-test

**Status:** Accepted
**Date:** 2026-07-05
**Deciders:** owner

## Context

The kinship query engine ([ADR-0024](./0024-query-seam-and-envelope.md) through [ADR-0028](./0028-relationship-resolution-and-honest-emptiness.md)) runs on-demand over a checked project's `ResolvedDocument`. Its performance *posture* — how the engine is allowed to reach the interactive target, and how that target is defended — was pinned in PRD 0005's Performance section, not in an ADR. This is the epic's closing slice (issue [#261](https://github.com/YashBhalodi/kul/issues/261)), and the PRD is transient — it is deleted in the same PR that ships this ADR. A decision a future agent might re-propose changing must not evaporate with the PRD; the no-indices / no-caching constraint is exactly that kind of decision (the obvious "make `resolve` faster" instinct is to add a cache), so it needs a durable home with its rationale. This ADR is that home.

Two of the earlier ADRs already touch the posture in passing — [ADR-0025](./0025-kinship-query-engine-contract-and-traversal.md) notes the engine "builds its own in-memory adjacency per invocation and caches nothing across queries", and [ADR-0028](./0028-relationship-resolution-and-honest-emptiness.md) sizes the resolution budget against the remote-ancestor haystack. This ADR states the constraint as a first-class decision and pins how it is *guarded*.

## Decision

### On-demand over `ResolvedDocument`, per-invocation structures only — no query indices, no cross-query caching

Every query — kin-set, resolution, attribute filter, detail lookup — runs on demand against the same checked `ResolvedDocument` the validator and the renderer already read, reusing the resolver's in-memory id index. A query that needs an adjacency (kin-set traversal, resolution) **builds its own, per invocation, and throws it away** ([ADR-0025](./0025-kinship-query-engine-contract-and-traversal.md)). There are **no dedicated query indices** (no precomputed ancestor tables, no cousin maps) and **no cross-query result caching** (no memoisation of `resolve(x, y)`, no warm descriptor set).

The reason this holds is the [no-mega-import boundary](../../CONTEXT.md): large-corpus import (GEDCOM trees of 100k+ persons) is **never a target**, by paradigm choice — the durable ceiling is ~10k persons. At that scale an on-demand traversal is interactive without precomputation, so an index would be complexity guarding a scale the product deliberately refuses. The two-tier product story stays clean: the engine owns *kinship correctness* computed fresh; anyone who needs precomputed analytics at scale falls back to the exported JSON.

### The interactive target is `< ~50 ms/query` on ~10k persons, and it is defended by a test, not a benchmark

The target is interactive latency — a UI calls the engine on every click without a spinner (PRD story #50) — quantified as **< ~50 ms per query on a project of up to ~10,000 persons**. It is guarded the way every other performance-sensitive path in this workspace is: a **perf budget that is a test, not a benchmark** ([docs/testing.md](../testing.md)), so it runs in every `cargo nextest` and a regression fails at PR time rather than waiting for someone to remember a bench.

The gate lives at [`crates/kul-core/tests/perf.rs`](../../crates/kul-core/tests/perf.rs) — the engine is pure `kul-core`, so its budget is a core-crate test, the per-crate mirror of the render/LSP gate at `crates/kul-lsp/tests/perf.rs`. It exercises the seven representative operations (bounded ancestors/descendants, second cousins, resolution of a distant pair and of a cross-component pair, a filtered-sorted-counted `allPersons`, and a detail lookup) over a **deterministic, constructed ~10k-person synthetic corpus** — ~12 generations grown breadth-first at the branching factor a 10k / 12-generation history actually implies, carrying the structural hazards (polygamy, adoption-into-relatives, divorce-and-remarriage) plus a second disconnected component. Constructed, never randomised, so the budget is reproducible. Following the workspace convention, the assertion ceiling is generous (~5× the real target, higher in debug) so runner variance never flakes it while a 2× regression still fires; the real 50 ms target lives in a comment.

### If the budget cannot be met within this constraint, that is a human decision — never a silent cache

The no-caching constraint is load-bearing precisely because the fast fix for a missed budget is always "add a cache". So the rule is explicit: if a future change cannot meet the interactive target **within** the on-demand / no-index / no-cache constraint, the engine does **not** silently grow a cache or an index to paper over it. The trade-off (accept the latency, optimise the on-demand traversal, or relax the constraint) is surfaced for a human decision — a comment on the issue — because introducing caching changes the engine's architecture and its staleness guarantees, and that is not a call an implementer makes unilaterally.

## Consequences

- The query perf gate is added at `crates/kul-core/tests/perf.rs`, alongside the existing render/LSP budgets, and runs as part of `just check`. On a dev laptop the whole representative set lands under the target in the shipped (release) profile — kin-set / filter / lookup ≈ 1–3 ms, and the heaviest operation, `resolve` (which enumerates *every* way two people are related over the cap-bounded neighbourhood), ≈ 15–17 ms — all well inside 50 ms, with no cache or index introduced to get there.
- The synthetic corpus is built in-test and never added to `examples/`, so the example-corpus snapshots stay untouched by a workload that exists only to measure latency.
- Because every query rebuilds its adjacency, the cost model is "one query = one adjacency build + one bounded traversal". A regression that makes a bounded kin-set walk fall back to O(corpus), or that lets the resolution neighbourhood blow past the cap, trips the gate.
- The constraint is now recoverable without the PRD: a future proposal to add a query index or a `resolve` cache must argue against this ADR, not merely fail to find the prohibition.

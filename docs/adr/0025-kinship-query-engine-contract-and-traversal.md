# ADR 0025 — Kinship correctness lives in the core; one Query value, one cycle-guarded traversal

**Status:** Accepted
**Date:** 2026-07-05
**Deciders:** owner

## Context

PRD 0005 (epic [#253](https://github.com/YashBhalodi/kul/issues/253)) ships a kinship query engine. [ADR-0024](./0024-query-seam-and-envelope.md) established the seam (the `query` module over `ResolvedDocument`), the `QueryEnvelope`, and the two detail lookups. This slice ([#256](https://github.com/YashBhalodi/kul/issues/256)) lands the first *computed* capability — the lineal kin-set family (parents, children, ancestors, descendants) — and with it three architectural decisions that every later capability (collateral kin, relationship resolution, attribute filtering) builds on:

1. **Where does kinship correctness live, and who is allowed to re-derive it?**
2. **What is the contract a consumer constructs, and how many ways are there to evaluate it?**
3. **How does traversal behave on a graph that is not acyclic?**

The companion [ADR-0026](./0026-relationship-descriptor-and-path-identity.md) pins the *shape of the answer* (the relationship descriptor and path identity); this ADR pins the *engine*.

## Decision

### Kinship correctness lives in the core offering and is single-sourced

The governing constraint of the epic: **the correctness of kinship querying lives in the core, computed once, never re-implemented by a downstream consumer.** Consumers own only the *UX* of querying. The engine is a deep module in `kul-core` layered over the primitive one-hop derivations `ResolvedDocument` already exposes (`parents_of`, `spouses_of`) — it consumes those plus `persons()`, and never re-walks the AST and never consumes the `ExportedGraph`.

This rules out "hand consumers the three exported tables plus a generic query engine (SQL/GraphQL)": that re-creates the exact "compute the sibling yourself" trap the engine exists to remove. The core exposes **computed relations as first-class**, not raw tables. The two adapters (WASM, CLI) are thin wiring over the same core path; native consumers use the Rust crate directly.

### The Query value is the single contract, with exactly one evaluation path

The contract artifact is a **declarative, serializable `Query` value** — a `source` plus a `projection` — evaluated by one core entry point, `query::evaluate`. Every surface constructs this value: the Rust named sugar (`parents_of`, `ancestors_of(depth)`, …), the WASM `queryKin`, and the CLI `kul query kin` all desugar to a `Query` and call the same evaluator. **There is no second evaluation path.**

- **Named sugar is defined *as* its Query-value expansion**, not as a parallel implementation. `parents_of(x) ≡ kinOf(x, lineal ancestor, generations {1,1})`. This makes parameterized queries ("second cousins once removed") expressible by construction in later slices, with no dedicated API.
- **Raw up/down/across step composition stays internal.** Exposing it would re-create the trap: self-exclusion, cycle guarding, and subsumption are engine-owned and must stay unreachable by consumers.
- The value's enums are **additive**: `allPersons` sources, `collateral` patterns, `count` projections, and attribute filters are new variants, never a reshape. Committing the value's shape now (even where only the lineal subset is populated) is what lets later slices extend without re-litigating the contract.
- Rust-native ergonomics (free functions here; a builder may be added later) are an implementer's call under the one pinned constraint — any sugar desugars to the Query value.

An unknown anchor is a **typed `UnknownPerson`-class error, never an empty set.** An anchor is an *input* to the question (unlike a detail lookup's id, which is the *subject* — [ADR-0024](./0024-query-seam-and-envelope.md) draws that line deliberately), so a bad anchor is a caller bug. An empty set therefore always means "no kin matched". The adapters fold that typed error into the `QueryEnvelope` error arm as a diagnostic naming the id.

### Traversal walks the full relation graph and is unconditionally cycle-guarded

The engine builds **its own in-memory adjacency per invocation** — a children index that is the inverse of the resolved parent links — and caches nothing across queries (the ~10k-person budget makes precomputed indices unnecessary). Parent-child edges are tagged `bio` / `adoptive` from the resolved parent-link kinds.

**Cycle guarding is unconditional.** Biological parenthood is a guaranteed DAG (rule R13), but following adoption edges forfeits that guarantee — adoption-into-relatives is a real corpus case, not an edge case. The engine never assumes acyclicity: the **simple-path rule** (no person appears twice on a path) is the guard, and traversal terminates on every input.

The path grammar is stated in full for the record — a relationship path is 1–3 blood segments joined by at most two marriage hops, each blood segment `up* down*` through a single apex — but **this slice implements the vertical-only subset**: exactly one blood segment, zero marriage hops, so paths are `up+` (ancestors) or `down+` (descendants). Deriving classification mechanically from hop counts now means the collateral and affinal slices extend rather than rewrite.

## Consequences

- Every consumer that needs kinship answers gets them from one place; a correctness fix lands once and every surface inherits it.
- The Query value is the stable contract; adapters stay thin wiring, and the CLI `--format json` bytes are the WASM bytes (both serialize the one core envelope), so the CLI snapshot suite pins the whole epic's contract serialization.
- Later capabilities are additive variants on the Query value and new descriptor derivations — not new engines.
- Because traversal never assumes acyclicity, adoption-into-relatives and (later) cousin-marriage corpora are correct by construction, not by a caller remembering to bound depth.

## Anti-suggestions (do not re-propose)

- **"Ship SQL/GraphQL over the three exported tables and let consumers write kinship queries."** That hands consumers the raw graph and the re-derivation burden — the precise trap the engine removes. The core exposes computed relations; the export stays the escape hatch for analytics, never the engine's input.
- **"Give the CLI (or WASM) its own little traversal for the simple cases."** A second evaluation path is a second thing to keep correct forever. Every surface desugars to the one Query value and the one `evaluate`.
- **"Expose raw up/down/across steps so power users compose their own relations."** That re-exposes self-exclusion, cycle guarding, and subsumption to consumers. Sugar over the Query value is the surface; step composition stays internal.
- **"Bio-parenthood is a DAG (R13), so skip the visited-set on the common path for speed."** Adoption edges break acyclicity, and the guard is what guarantees termination. Cycle guarding is unconditional, not a mode.
- **"Return an empty set for an unknown anchor — it's simpler than a typed error."** Then a caller typo is indistinguishable from "no relatives". The anchor is an input to the question; a bad one is a typed error, and an empty set always means "no kin matched".

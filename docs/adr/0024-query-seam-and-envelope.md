# ADR 0024 — The `query` seam over `ResolvedDocument`, and the `QueryEnvelope` fourth surface

**Status:** Accepted
**Date:** 2026-07-05
**Deciders:** owner

## Context

PRD 0005 (epic [#253](https://github.com/YashBhalodi/kul/issues/253)) ships a kinship query engine: kin-set queries, relationship resolution, and attribute filtering over a checked Kul project, surfaced to the WASM and CLI adapters. That is a large body of work. Issue #255 is its first tracer-bullet slice — establish the seams and thread the two simplest operations (the id → detail lookups `person(id)` / `marriage(id)`) end-to-end through core, WASM, and CLI — so that every layer exists and agrees on conventions before the harder capabilities land.

Several structural questions had to be pinned by this first slice, because everything later builds on the answers:

1. **Where does query logic live, and what is its substrate?** A new module, or bolted onto `ResolvedDocument`? Layered over the resolved view, or over the exported graph?
2. **What shape do the lookups return, and how is "not found" reported?** A new person/marriage shape, or the export's? An error, an empty value, or an absent one?
3. **What is the adapter surface?** How does a never-throwing JS boundary and a strict CLI both carry a result-or-diagnostics outcome, consistently with the surface that already exists?

## Decision

### One new logic seam: the `query` module in `kul-core`, layered over `ResolvedDocument`

The query engine is a **deep module in `crates/kul-core/src/query.rs`** — the single public seam where all query capabilities and their result types live. It consumes `ResolvedDocument`'s public accessors ([ADR-0001](./0001-resolved-document-as-query-seam.md)) and never re-walks the AST. `ResolvedDocument` remains the seam for *primitive* one-hop derivations (`parents_of`, `spouses_of`); the query engine layers the *composite* capabilities on top.

**The engine's substrate is `ResolvedDocument`, never the `ExportedGraph`.** The export stays the deliberate escape hatch for consumers who don't use the engine (analytics, foreign tooling); it is never the engine's own input. Feeding the engine its own export would couple two contracts that must be free to evolve independently.

### Detail lookups return the export shapes, single-sourced; absence is the answer

`query::person(id)` returns `Option<ExportedPerson>` and `query::marriage(id)` returns `Option<ExportedMarriage>` — **the same serialized shapes the export produces.** There is no second person/marriage shape. To make single-sourcing structural rather than aspirational, the export's per-entity construction is prefactored into `build_one_person` / `build_one_marriage`, which both the whole-graph export loop and the lookups call. A shape drift is now impossible without breaking both call sites at once.

**For a lookup, absence is the answer.** An unknown id, or an id that names a marriage when a person was asked for (and vice versa), yields `None`. There is no error type at the lookup layer: a lookup asks "is there a person with this id?", and "no" is a complete, honest answer. This is deliberately *different* from later slices, where an id is an *input anchor* to a relationship question — passing an unknown id there is a caller bug and will be a typed error. The distinction (subject-of-the-question vs anchor-of-the-question) is what makes `Option` right here and an error right there.

### The `QueryEnvelope` — the fourth WASM shape, single-sourced for the CLI too

Query operations return a `QueryEnvelope<T>`: an untagged union discriminated by an `ok` boolean, whose ok arm carries the query `result` and whose error arm carries structured `diagnostics`. This **extends the surface of [ADR-0011](./0011-wasm-surface-three-shapes-no-wrappers.md)** with a fourth operation-specific shape, exactly as `renderSvg` did. It is gated on the project passing its checks (strict-on-errors, [ADR-0009](./0009-export-strict-on-diagnostics.md)): a failing project yields the error arm, never a partial answer, and — on the WASM boundary — the function never throws.

Two decisions keep the surface consistent:

- **The envelope mirrors the existing `ok`-boolean discriminator, not a new `status` tag.** PRD 0005's sketch spelled the tag `status: "ok" | "error"`, but the existing envelopes (`ExportEnvelope`, `RenderEnvelope`) discriminate on an untagged `ok` boolean. Consistency with the surface that already ships wins; the envelope uses `{ ok: true, result }` / `{ ok: false, diagnostics }`.
- **The envelope lives in `kul-core`, not the adapters.** Like `ExportEnvelope`, it is defined once (with a `cfg`-gated `Tsify` derive) so the CLI `kul query --format json` bytes are byte-identical to what WASM returns. The CLI JSON path is the epic's contract-snapshot harness; a second definition would let the two drift.

`QueryEnvelope<T>` is **generic over the payload** so later slices (kin-set queries, relationship resolution) reuse it without reshaping. Its TypeScript types ship under the committed-tsify discipline of [ADR-0012](./0012-tsify-derived-types-committed-and-diffed.md).

**Explicit `null`, not absent, for a not-found result.** tsify's default WASM serializer omits a `None` field, which would make a not-found lookup cross the boundary as `{ ok: true }` — diverging from the CLI's `serde_json` `{ ok: true, result: null }` and from the committed TS type (`result` is a required `T | null`). `QueryEnvelope` carries `#[tsify(missing_as_null)]` so `None` serializes as an explicit `null`; the export shape's `skip_serializing_if` optionals still stay absent. `null` is load-bearing here — it is the "no such entity" answer — so it must be present, and it must be the same byte on both surfaces.

### CLI surface

`kul query person <id>` and `kul query marriage <id>`, modelled on `kul export`: CWD-rooted project discovery, the same strict load-and-check gate, and `--format human|json`. `--format json` emits the `QueryEnvelope` serialization. `--format human` renders the entity's recorded fields in a readable, **terminology-neutral** layout — presentation owned by the adapter, not a third contract shape, and never a kinship word (rendering "sister-in-law" would make the CLI the first culture pack, shipped by accident). Not-found prints a diagnostic naming the id to stderr and exits nonzero; under `--format json` it *also* emits the ok envelope with the `null` payload on stdout, because that null is the contract answer.

## Consequences

- Every layer of the query engine now exists and agrees on conventions. Later slices add capabilities to the `query` module and variants to the surfaces without re-litigating where things live or how results are shaped.
- The person/marriage serialized shape has exactly one producer path. Adding a field to `ExportedPerson` updates the export *and* the lookup in one edit.
- The CLI `--format json` output and the WASM envelope are byte-identical by construction (one core-side serialization), so the CLI snapshot suite pins the whole epic's contract serialization.
- The `ok`-boolean discriminator means JS consumers narrow the envelope structurally (`'result' in env`), the same pattern `exportGraph` / `renderSvg` already use.

## Anti-suggestions (do not re-propose)

- **"Run the engine over the `ExportedGraph` — it already flattens the data."** The export is the engine's *output-side sibling*, not its input. Coupling them ties two contracts that must evolve independently, and re-imports the "compute the derivation yourself over raw tables" trap the engine exists to remove. The substrate is `ResolvedDocument`.
- **"Return a typed `UnknownEntity` error from the lookups for symmetry with the future relationship queries."** A lookup's subject *is* the id; absence is the honest answer, so `Option`/`null` is correct. The typed error belongs where an id is an *anchor* to a different question — a distinction the two slices deliberately draw differently.
- **"Define a second, leaner person shape for query results."** Single-sourcing the export shapes is the pinned decision. A parallel shape is one more thing to keep in sync forever, for no consumer benefit.
- **"Give `QueryEnvelope` a `status: 'ok' | 'error'` string tag as the PRD sketched."** The shipped surface discriminates on an `ok` boolean; matching it is worth more than matching a sketch. Consistency with the existing envelopes wins.
- **"Let a not-found result serialize as an absent key — JS treats `undefined` and `null` the same."** The committed TS type makes `result` required, and the CLI writes `null`; an absent key on WASM breaks byte-identity and the type contract. `#[tsify(missing_as_null)]` keeps the two surfaces honest.

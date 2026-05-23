# ADR 0016 — `kul-render` is a separate crate

**Status:** Accepted
**Date:** 2026-05-23
**Deciders:** owner

## Context

A canonical Kul document has three named data forms on its path from source to pixels. `kul-core` produces the first: a checked project plus the kinship-native [`ExportEnvelope`](../../crates/kul-core/src/export.rs) that mirrors the language's primitives one-to-one. Surface renderers (VSCode preview panel, web visualizer, future native preview) produce the third: pixels. Between them sits the canonical UI pattern ([`docs/canonical-ui-pattern.md`](../canonical-ui-pattern.md), issue #110) — the design language that names cards, ghosts, marriage bars, components, generations, and the recursive nesting that makes inter-family structure visually obvious. The pattern has data form too: layout-meaningful facts in a hierarchy that matches the pattern's principles. That data form needs an owner.

Three placements were on the table for that owner:

1. **Inside `kul-core`.** Keep everything in one library and re-use `ResolvedDocument` directly without going through `ExportEnvelope`.
2. **Inside `kul-wasm`.** Bind the pattern's data form to JS-shaped types and avoid the extra crate.
3. **As a new `kul-render` crate sibling to `kul-core`.** The pattern's vocabulary, principles, and projection algorithm live together; consumers of either the kinship-native graph or the pattern-shaped data pick the one they need.

Two pressures shaped the decision. First, [ADR-0008](./0008-export-kinship-native-shape.md)'s audit (#117) confirmed the kinship-native shape carries every fact the canonical UI pattern needs — there is no reason for the projection to reach back into the AST or resolver state, and doing so would tie the pattern's semantics to `ResolvedDocument` internals. Second, the canonical UI pattern is meant to co-evolve with [`docs/canonical-ui-pattern.md`](../canonical-ui-pattern.md) as the language grows; it should be possible to amend the pattern (e.g. handle a new sub-statement) without touching `kul-core`.

## Decision

A new crate at `crates/kul-render/`, sibling to `kul-core`. It depends on `kul-core` (one direction) and is depended on by future surface-renderer adapters (the WASM render bridge, the VSCode preview panel). It does **not** depend on `kul-loader`, `kul-cli`, or `kul-lsp`.

Two public functions:

- `pub fn transform(envelope: &ExportEnvelope) -> RenderShape` — pure transformer. Reads only the kinship-native graph; never the AST.
- `pub fn compute(check: &CheckResult) -> RenderShape` — convenience wrapper that calls `kul_core::export::export` with positions enabled, then `transform`.

Both are public so test surfaces are independent: `compute` is exercised against the `examples/` corpus; `transform` is exercised against fabricated `ExportEnvelope` fixtures for edge cases the corpus doesn't naturally surface (P6 cross-component nesting, P16 with three-plus adoptions, failure-envelope passthrough).

Internally the crate is two layers:

- `shape.rs` — the `RenderShape` types (see [ADR-0017](./0017-render-shape-schema-and-versioning.md)). Pure data plus a schema-version const. No projection logic.
- `build.rs` — the kinship-native → canonical-UI-pattern projection algorithm. One linear pre-computation pass over the export's flat collections to derive each person's canonical-card location, generation index, primary marriage, and canonical adoption; one union-find pass to discover components; one recursive build per component.

## Consequences

- **Layer separation is explicit.** `kul-core` owns lexer / parser / AST / semantic / validator / export; `kul-render` owns the canonical UI pattern's data form and the projection that produces it. The pattern's vocabulary (card slot, ghost, component, marriage bar, P6 nested birth family) lives in `crates/kul-render/src/shape.rs` and the principles in `docs/canonical-ui-pattern.md`. Amending the pattern is a `kul-render` change, not a `kul-core` change.
- **Independently testable projection.** The `transform(envelope)` surface lets us drive the projection with hand-constructed envelopes — no need to write a `.kul` source for every edge case (and many edge cases, like fabricating an envelope that mixes ghost-only marriages with custom date precisions, would be awkward to express in source).
- **Schema versioning is independent of language and export.** `RENDER_SCHEMA_VERSION` follows the same per-[ADR-0010](./0010-export-schema-versioning.md) discipline as `export::SCHEMA_VERSION` but bumps under different conditions — see [ADR-0017](./0017-render-shape-schema-and-versioning.md).
- **Surface-renderer binding moves out of `kul-wasm`.** The WASM bridge for the render shape is a separate follow-up issue; when it lands it depends on `kul-render` and exposes a thin `exportRenderShape` function, the same way `exportGraph` thinly wraps `kul_core::export::export`. The wasm crate stays small and predictable per [ADR-0011](./0011-wasm-surface-three-shapes-no-wrappers.md).
- **`kul-render` is filesystem-free.** Like `kul-core`, it never touches the disk; the convenience `compute(&CheckResult)` takes an already-checked project. Filesystem concerns stay in `kul-loader`.
- **The projection reads only the kinship-native graph.** `compute` is the one place in the crate that calls `kul-core::export::export`; everything downstream of that reads the envelope. If the pattern's needs ever outgrow what the kinship-native shape carries, the audit at #117 will need to be re-run and the export schema bumped — not `kul-render`'s internal seams loosened.

## Anti-suggestions (do not re-propose)

- **"Inline `kul-render` into `kul-wasm`."** The pattern's data form is a logic concern; binding it to one adapter (the JS host) forecloses the in-process renderer (VSCode panel today, native preview tomorrow). The wasm surface remains exactly the three shapes [ADR-0011](./0011-wasm-surface-three-shapes-no-wrappers.md) committed to.
- **"Pass `&ResolvedDocument` to `transform` instead of an `ExportEnvelope`."** That would let the projection re-read AST detail the export doesn't carry — which is exactly the violation #117 audited against. The kinship-native graph is the contract.
- **"Add a layout / pixels layer to this crate."** Layout (card positions, edge routing geometry, level-of-detail, virtualization) is a multi-quarter epic with its own design space (SVG vs Canvas vs HTML, layout algorithm, panning, zoom). Bundling it with the projection would conflate "what does the pattern look like as data" with "how do we draw it" — two questions with very different rates of change. Layout lives in surface renderers downstream of `kul-render`.
- **"Re-export `ExportEnvelope` from `kul-render`."** Adds two import paths for the same type. Consumers that need both pull `kul_core::export::ExportEnvelope` directly; the type is part of the published `kul-core` surface.

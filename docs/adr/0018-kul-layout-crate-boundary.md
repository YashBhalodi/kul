# ADR 0018 — `kul-layout` is a separate crate

**Status:** Accepted
**Date:** 2026-05-23
**Deciders:** owner

## Context

[ADR-0016](./0016-kul-render-crate-boundary.md) places the canonical-UI-pattern projection in `kul-render` and pins the two public surfaces (`compute(&CheckResult)`, `transform(&ExportEnvelope) -> RenderShape`). It also bans a layout / pixels layer from `kul-render` — that anti-suggestion is one of ADR-0016's "do not re-propose" items, because layout is "a multi-quarter epic with its own design space (SVG vs Canvas vs HTML, layout algorithm, panning, zoom)" and bundling it would conflate "what does the pattern look like as data" with "how do we draw it".

[`RenderShape`](../../crates/kul-render/src/shape.rs) is structural, not positional ([ADR-0017](./0017-render-shape-schema-and-versioning.md) §"Anti-suggestions"). A surface renderer that wants to display a Kul document still needs to decide, given a `RenderShape`, *where* every card, bar, and edge segment goes. That step — the positioning algorithm plus the canonical-pattern adapter that wraps it — needs an owner.

Three placements were on the table:

1. **Inside the surface renderer (e.g. the VSCode webview, in TypeScript).** Each surface owns its own layout. Simplest at first; forces every future surface (web app, native preview, CLI export) to reproduce the algorithm.
2. **Folded back into `kul-render`.** Reverses ADR-0016's anti-suggestion. Bundles layout with projection.
3. **As a new `kul-layout` crate sibling to `kul-render`.** Walker's algorithm plus the canonical-pattern adapter live together; surface renderers walk a positioned shape instead of running the algorithm themselves.

Two pressures shaped the decision. First, the canonical UI pattern's adapter logic (marriage bars positioned between adjacent spouses, ghost slots at the host's birth-family position per P8, generation rows from generation indices, P6 recursive nesting) is **the canonical pattern's positioning semantics, not any one surface's UI**. Re-deriving it in TypeScript inside the VSCode webview, then again in JS inside a future web app, then again in Rust inside a future CLI export path, would put three implementations of the same canonical decision in three different codebases. Second, Walker's algorithm (Reingold–Tilford–Walker, O(n)) is small (~200 lines) but non-trivial; owning it once in Rust beats porting it.

## Decision

A new crate at `crates/kul-layout/`, sibling to `kul-render`. It depends on `kul-render` (one direction) and is depended on by `kul-svg` (which renders the positioned output to SVG) and by future surface adapters that want positioned data without going through SVG. It does **not** depend on `kul-loader`, `kul-cli`, or `kul-lsp`.

One public function:

```rust
pub fn layout(shape: &RenderShape, config: &LayoutConfig) -> PositionedShape;
```

`LayoutConfig` is a struct with `Default`; only `LayoutConfig::default()` is constructed by any consumer in v1. The struct exists as a forward-compatibility seam — future configurable density, font-metric tweaks, alternative-algorithm dispatch all add fields here without changing the function's signature.

Internally the crate is two deep modules:

- **`kul_layout::walker`** — the canonical Reingold–Tilford–Walker port. Takes a tree (kul-layout's internal representation derived from `RenderShape`) and emits positions. Small input/output surface; encapsulates the algorithm's state (preliminary x, modifier, threads, ancestors).
- **`kul_layout::adapter`** — wraps Walker's for kul's pattern: marriage bar between adjacent spouses, ghost slots at the host's birth-family position per P8, generation rows from generation indices, orthogonal right-angle edge routing for `InTree` edges, future recursive P6 nesting. Hides Walker's complexity from `kul_layout::layout`.

`PositionedShape` is an **internal Rust seam**, not a wire shape. It is **not** `Serialize`, not schema-versioned, not part of any cross-process contract. The crate exposes the type publicly so `kul-svg` and future Rust consumers can read it; the JSON wire shapes the project versions and pins are `ExportEnvelope` (kul-core) and `RenderShape` (kul-render) on the input side, and the SVG string on the output side. A third versioned shape between them has no external consumer.

`PositionedEdge::routing` is an extensible discriminator with `InTree` (v1) and `CrossTree` (future) variants. v1 only constructs `InTree`; the type's shape anticipates the cross-tree follow-up so adding it is one match-arm, not a refactor of the shape.

Walker's algorithm is implemented from day one even though `examples/03-three-generations/` does not exercise sibling-subtree collisions. The hand-fabricated `tests/walker.rs` tests cover the algorithm's contract independent of corpus content; future examples that do trigger collisions land without needing a new algorithm.

## Consequences

- **Layout lives once, in Rust.** Future surface renderers (VSCode preview today, web app + native preview tomorrow, CLI export later) share one implementation of the canonical pattern's positioning semantics. No drift across surfaces.
- **The kul-render anti-suggestion holds.** Layout did not get folded back into `kul-render`. The projection (what does the pattern look like as data) and the positioning (where does the data go on a 2D plane) are separate concerns in separate crates, even though both are downstream of `ExportEnvelope`.
- **`PositionedShape` is free to evolve.** Because it is not a wire contract, internal refactors — adding a field, splitting a struct, renaming a variant — do not require a schema bump or migration policy. The cost of pinning `PositionedShape` as a versioned shape (matching ADR-0010's discipline transposed once more) has no benefit in v1, because no out-of-process consumer reads it.
- **Walker's is in place for follow-ups.** F2 (`examples/02` + `/04`), F4 (P12 multi-component), and F5 (P11 cross-edges) land without algorithmic work — they extend the adapter, not the positioning core.
- **The dependency graph stays unidirectional.** `kul-svg → kul-layout → kul-render → kul-core`, all one-direction.

## Anti-suggestions (do not re-propose)

- **"Inline `kul-layout` into `kul-svg`."** Then the positioning algorithm is bound to one output format. A future CLI consumer that wants positioned data without SVG (for, say, a custom Canvas renderer or a tooling integration that needs card centroids for layout metrics) would have to depend on `kul-svg` and discard the SVG string. The split keeps the positioning concern reusable across output formats.
- **"Make `PositionedShape` `Serialize` and schema-version it."** v1 has no out-of-process consumer of positioned data. The wire contracts are already pinned at `RenderShape` (input) and the SVG string (output). Reify the contract only when an out-of-process consumer appears — then the policy is the same as ADR-0010/0017's transposed once more.
- **"Use an external layout library (dagre, elkjs, cytoscape, react-flow)."** None of them speak the canonical pattern's vocabulary — marriage bars between adjacent spouses, ghost slots, P6 recursive nesting. A custom Walker's port is small enough (~200 lines) to own. Adding a dependency to avoid 200 lines of standard algorithm code is poor value.
- **"Compute positions inside `kul-render::compute`."** That re-opens ADR-0016's "Add a layout / pixels layer to this crate" anti-suggestion. The data form and the positioning are different rates of change — `RenderShape` is the canonical pattern as data (slow); `PositionedShape` is layout policy (fast, may grow alternative algorithms). Bundling them would force a `RenderShape` consumer who didn't want positions (e.g. a future text-mode dump tool) to also pull in the positioning code.
- **"Reach into `kul-core::semantic::ResolvedDocument` from kul-layout."** Same anti-pattern ADR-0016 closed off for `kul-render`. `kul-layout` reads `RenderShape` only; `RenderShape` carries every layout-meaningful fact (ADR-0017).

# ADR 0032 — Lineage-depth cap on the visualization pipeline

**Status:** Accepted
**Date:** 2026-07-06
**Deciders:** owner

## Context

The visualization pipeline turns a checked project into pixels through recursion that descends one level per generation of a lineage:

- `kul-render`'s builder chain (`build_person_root` → `build_hosted_marriages` → `build_marriage_branch` → `build_children` → …) recurses down the host-lineage tree.
- `kul-layout`'s adapter `build_person` recurses to assemble the walker input tree.
- `kul-layout`'s Walker port recurses twice more — `first_walk` (post-order) and `second_walk` (pre-order).

None of these passes bounded recursion depth. Lineage depth is **attacker-controlled**: a `.kul` document is untrusted input, and a single unbroken lineage thousands of generations deep is trivially machine-generatable. Such a document drove recursion that deep and aborted the process on stack overflow — a panic reachable from untrusted input. It hit the surfaces with the smallest stacks hardest: the WASM `renderSvg` bridge (a panic unwinds/traps across the JS boundary) and the LSP `kul/render` + `kul/exportSvg` requests (worker-thread stacks). In-budget documents — real genealogies run to roughly 100 generations, and the corpus is a handful deep — were never at risk.

The upstream stages are already iterative and safe: `kul-core`'s generation computation is a flat fixpoint relaxation, cycle detection uses an explicit `frames` stack, and graph export is flat iteration. The unbounded recursion begins only once the render builder starts descending the tree.

## Decision

Enforce a documented maximum lineage depth at the single choke point where recursion begins — `kul_render::build::build` — and downgrade an over-limit document to a `RenderShape::Failure` carrying a `KUL-V01` diagnostic instead of recursing into it.

```rust
pub(crate) const MAX_LINEAGE_DEPTH: u32 = 512;
```

The check is cheap and iterative: `Index::new` already precomputes every person's generation, so the deepest lineage depth is a `max` over a flat slice, decided **before** any recursive build runs. When it exceeds the cap, `build` returns the diagnostic and `transform` projects it into `RenderShape::Failure`.

**One guard bounds every pass.** `kul_layout::layout` takes `&SuccessRender`, so it only ever runs on the success arm. An over-limit document never becomes a `SuccessRender`, so the adapter and both Walker walks are never reached with pathological depth. Guarding the render boundary transitively guards the entire downstream pipeline — the WASM, CLI, and LSP surfaces all route through `kul_visual::render_from_check`, which returns the failure diagnostics rather than trapping.

**Depth cap over iterative conversion.** The issue permitted either converting the recursive passes to explicit-stack iteration (the precedent being `forest_extent`/`translate_forest`) or a documented depth cap. A single depth cap was chosen deliberately:

- **One mechanism, one choke point.** The cap bounds all four recursive passes at once. Piecemeal iterative conversion would leave the render builder and the Walker's post-order-with-`apportion` pass — neither of which the simple pre-order precedent shows how to convert — still needing a cap, yielding two mechanisms for one hazard.
- **Byte-identical in-budget output, guaranteed structurally.** Below the cap the recursive code paths run unchanged, so every in-budget document lays out identically and no `insta` snapshot shifts. A hand-rolled iterative Walker risks a subtle ordering or floating-point difference that would churn snapshots for zero user-visible benefit.

`MAX_LINEAGE_DEPTH = 512` sits well above any realistic document (recorded human lineages run to ~100 generations; the "hundreds deep" band is already known-safe on every surface) yet far below the "thousands" depth at which the smallest surface stacks overflow — a document exactly at the cap still recurses safely.

`KUL-V01` is a new diagnostic namespace (`V` for the visualization pipeline), distinct from the language-level `KUL-L`/`KUL-P`/`KUL-R`/`KUL-M`/`KUL-Q` codes: it is a rendering resource limit, not a language-validation rule, so it is not part of the normative `spec/` and does not appear from `kul validate` or the non-visual `kul export` (whose graph build is flat). It is unanchored (`primary: None`, like `KUL-M01`) — depth is a whole-document property, not a single declaration.

## Consequences

- **No untrusted input aborts the process.** The WASM bridge returns a failure envelope, the LSP returns anchored diagnostics, and the CLI SVG export prints diagnostics — none trap or overflow.
- **In-budget documents are unaffected.** Any lineage at or below 512 generations projects and lays out exactly as before; no snapshot changed.
- **The cap is the single source of truth for the limit.** The constant and its `KUL-V01` diagnostic live in `kul-render`; the rationale lives here. There is no second copy in a surface crate.
- **A raised ceiling is a one-line change.** If a real document ever approaches the cap, the constant moves — but the smallest-stack safety budget is the real ceiling, not the constant, and that is what the ADR pins.

## Anti-suggestions (do not re-propose)

- **"Convert the Walker walks and the render builder to explicit-stack iteration instead."** The simple pre-order precedent (`forest_extent`/`translate_forest`) does not cover the Walker's post-order-with-`apportion` pass or the mutually-recursive render builder; converting them risks churning in-budget layout for no user-visible gain, and would still leave the same hazard partially unguarded. The cap bounds all passes at one choke point with guaranteed-identical in-budget output.
- **"Put the cap in `kul-layout` (or check it per pass)."** Layout only runs on the success arm, so a cap there would be redundant and would fragment one limit across crates. The render boundary is the earliest point recursion begins and the single place every surface funnels through.
- **"Add `KUL-V01` to `spec/` as a validation rule."** It is a tooling resource limit, not language semantics. `kul validate` and non-visual `kul export` do not and should not emit it.
- **"Make the cap configurable per surface."** The limit exists to keep the *smallest* stack safe; a surface raising it would reintroduce the overflow it is meant to prevent. One conservative constant serves every surface.

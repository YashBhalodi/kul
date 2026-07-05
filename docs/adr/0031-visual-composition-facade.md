# ADR 0031 — `kul-visual`: the composition facade above the pinned pipeline crates

**Status:** Accepted
**Date:** 2026-07-06
**Deciders:** owner

## Context

[ADR-0016](./0016-visualization-pipeline-crate-boundaries.md) split the source-to-pixels journey into four crates, each pinned to a single public function:

```
ExportEnvelope ──▶ RenderShape ──▶ PositionedShape ──▶ SVG string
  (kul-core)        (kul-render)     (kul-layout)        (kul-svg)
 compute/transform     layout            render
```

The pinning is deliberate: a consumer pulls only the layers it needs, and no crate reaches *up* the stack to orchestrate the ones above it. But the four crates only describe the *stages*; they say nothing about the *composition* that drives a checked project all the way to an SVG string. That composition — `compute(check) → match on the render shape → layout(success) → render(positioned)` — was re-implemented once per SVG-producing surface:

- the WASM `renderSvg` path (`crates/kul-wasm/src/lib.rs`),
- the CLI `kul export --format=svg` path (`crates/kul-cli/src/commands/export.rs`),
- the LSP `kul/render` + `kul/exportSvg` path (`crates/kul-lsp/src/features/svg_envelope.rs`, itself already shared between those two requests after #269).

Every copy ran the identical success sequence and differed only in three genuinely surface-specific ways: the **theme** (`ThemeConfig::default()` for the theme-agnostic preview vs. `ThemeConfig::for_file_export()` for the self-contained file export), the **failure projection** (raw `ExportedDiagnostic`s for WASM, miette-to-stderr for the CLI, URI/range-anchored diagnostics for the LSP), and the **output sink** (a JS envelope, stdout, an LSP response envelope). The `RenderEnvelope` comment in `crates/kul-wasm/src/lib.rs` recorded the intent explicitly: *"Rule-of-three: a shared crate emerges only when a third independent consumer materializes."* With WASM, CLI, and LSP now all consuming the pipeline, the rule-of-three threshold is reached.

## Decision

Introduce a new thin composition crate `kul-visual` that depends on the four pinned crates (`kul-core`, `kul-render`, `kul-layout`, `kul-svg`) and exposes one function:

```rust
pub fn render_from_check(
    check: &CheckResult,
    theme: &ThemeConfig,
) -> Result<String, Vec<ExportedDiagnostic>>;
```

It owns the invariant success half of the pipeline — `compute → layout → render`, with the default `LayoutConfig` — and returns either the SVG string or the project's diagnostics. The three adapters route through it and keep only their own theme choice, failure wrapping, and output sink; none repeats the pipeline skeleton.

**The facade sits *above* the pinned crates, not *inside* one of them.** This is the load-bearing constraint. `render_from_check` must **not** be added to `kul-svg`: ADR-0016 pins `kul-svg::render(&PositionedShape, &ThemeConfig) -> String` to emission only, and giving it a `check`-to-SVG entrypoint would make it reach up the stack to orchestrate `compute` and `layout` — the exact crate-separation inversion ADR-0016's thesis forbids. `kul-visual` adds the composition layer as a new node above the four, extending the one-directional dependency graph:

```
kul-visual ──▶ kul-svg ──▶ kul-layout ──▶ kul-render ──▶ kul-core
```

No ADR-0016-pinned surface changes: `kul-render::{transform, compute}`, `kul-layout::layout`, and `kul-svg::render` are byte-for-byte the same. The four crates remain independently consumable — a tooling integration that wants card centroids still depends on `kul-layout` alone; `kul-visual` is for consumers that want the whole check-to-SVG composition and nothing less.

`ThemeConfig` is re-exported from `kul-visual` so a surface that only composes SVGs depends on the facade alone, not on `kul-svg` directly. The surfaces' direct dependencies on `kul-render`/`kul-layout`/`kul-svg` are dropped where the only use was the composition (they remain as dev-dependencies where a test still exercises the stages directly, e.g. the LSP perf gate times `compute` / `layout` / `render` separately).

## Consequences

- **The composition lives once.** A change to the success pipeline's shape (a new pipeline stage, a config that must thread through) is a one-file edit in `kul-visual`, not a three-surface sweep. Surface drift in the success arm becomes structurally impossible.
- **The three surface-specific concerns stay at the surface.** Theme choice, failure projection, and output sink are exactly what each adapter keeps; the facade is parameterized by theme and returns a plain `Result`, so each surface projects the failure arm into its own sink. The CLI and LSP discard the returned diagnostic list and re-derive their own (miette / anchored) projection from `check`; only WASM forwards it verbatim — all three unchanged in output.
- **Pure refactor, no behavioural change.** Every surface's success SVG and failure envelope is byte-identical to before; no `insta` snapshot shifted.
- **The pinned-surface pin holds.** Because the facade composes rather than extends, ADR-0016's "a consumer pulls only the layers it needs" property is preserved. `kul-visual` is one more consumer that happens to want all four.

## Anti-suggestions (do not re-propose)

- **"Put `render_from_check` in `kul-svg` (or `kul-layout`)."** That broadens a pinned single-function surface and inverts the layering — the emitter would orchestrate the stages above it. The facade's whole reason to be a separate crate is to sit above the pinned four without touching them.
- **"Fold the surface-specific failure projection into the facade."** The three projections (raw diagnostics, miette-to-stderr, anchored LSP diagnostics) are genuinely different and genuinely surface-owned. The facade returns the raw diagnostic list; the shaping stays at the sink. Pushing a projection strategy into `kul-visual` would drag surface concerns (miette, LSP `Range`, `ProjectEntry`) into a crate that must not know about any surface.
- **"Have the facade also own `LayoutConfig` / theme *selection*."** The theme is a per-surface choice (preview vs. file export); the facade takes it as a parameter and never decides it. `LayoutConfig` stays at its default inside the facade because no surface varies it today; a surface that needs to would add a parameter, not a second entrypoint.

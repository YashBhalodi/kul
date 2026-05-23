# PRD 0002 — Visual rendering pipeline for the canonical UI pattern

**Status:** Draft
**Date:** 2026-05-23
**Tracks:** issue #125
**Supersedes:** issue #122 (closed when this PRD lands)

## Problem Statement

Kul exists to make kinship readable. Today's pipeline produces structured data at every stage — tokens, AST, resolved documents, kinship-native `ExportEnvelope`, and the canonical-UI-pattern-shaped `RenderShape` (kul-render, ADR-0016/0017). What no Kul author can do today is **look at their family tree**. The data exists in increasing levels of refinement, but the pixels never materialise. The promise of "anyone-look-and-understand at low cognitive load" (per `docs/canonical-ui-pattern.md`) is unfulfilled until something turns `RenderShape` into a visual artifact.

The product cost is concrete: every author writing `.kul` today is editing structured prose with no visual feedback. The LSP catches errors and the export tooling produces machine-readable graphs, but the pattern's whole reason for existing — "anyone can look at it and understand" — sits dormant.

The remaining gap is the bridge from `RenderShape` (data) to SVG (pixels), the surface that consumes that SVG (a VSCode preview panel as the first surface; web app, native preview, CLI export in the future), and the toolchain seams that make those surfaces possible.

## Solution

Two new Rust crates, sibling to `kul-render`:

- **`kul-layout`** consumes `RenderShape` and produces an internal `PositionedShape` (cards positioned in 2D; edges with computed polyline geometry). Walker's algorithm (Reingold–Tilford–Walker, O(n)) handles tree positioning; the canonical pattern's marriage-bars, ghost slots, and recursive nesting layer on top.
- **`kul-svg`** consumes `PositionedShape` and produces an SVG string. The SVG is **theme-agnostic** — it uses semantic CSS class names (`kul-card`, `kul-card--canonical`, `kul-card--ghost`, `kul-bar`, `kul-edge--birth`, `kul-edge--adoption`, etc.) and emits no inline colours. Theming is applied by each consuming surface.

The pipeline composes: `ExportEnvelope` → kul-render → `RenderShape` → kul-layout → `PositionedShape` → kul-svg → SVG string.

A new LSP custom request **`kul/render`** mirrors the existing `kul/export` pattern: the VSCode extension passes a document URI, the LSP runs the pipeline against the cached document, and returns the rendered SVG (or a failure envelope if the document has diagnostics).

The **VSCode preview panel** is the first consumer surface. A `Kul: Show Preview` command opens a webview alongside the active editor; the extension subscribes to `onDidChangeTextDocument`, debounces 300 ms, fires `kul/render`, and posts the SVG to the webview. A small stylesheet (`preview.css`, ~30 lines) maps the kul-svg semantic classes to VSCode CSS variables (`var(--vscode-editor-foreground)` etc.), so the preview auto-tracks the user's editor theme (light, dark, high-contrast) without re-rendering. The webview is **static-only** in v1 — no interactivity at all; native browser scroll handles oversized SVGs.

The v1 success criterion is narrow: **`examples/03-three-generations/` renders correctly in the VSCode preview panel.** Every other corpus example, every interactivity feature, and every other consumer surface (CLI, web app, wasm bridge) becomes a focused follow-up issue.

## User Stories

### Kul author using the VSCode preview

1. As a Kul author, I want to run a `Kul: Show Preview` command on a `.kul` file, so that a preview panel opens alongside my editor showing the canonical visual.
2. As a Kul author, I want the preview to re-render automatically (within ~300 ms) as I edit the source, so that I see my changes appear without explicitly saving.
3. As a Kul author working in a dark VSCode theme, I want the preview to render with dark-theme-appropriate colours, so that the preview does not blind me with a bright white SVG.
4. As a Kul author working in a light VSCode theme, I want the preview to render with light-theme-appropriate colours.
5. As a Kul author working in a high-contrast VSCode theme, I want the preview to honour the high-contrast palette, so that the visual remains accessible.
6. As a Kul author whose document has errors, I want the preview panel to show a clear "Document has errors — see Problems panel" banner with the diagnostic count, so that I am not surprised when the visual does not update.
7. As a Kul author whose document has just become valid again, I want the preview to re-render the canonical visual immediately, so that fixing an error gives me visual feedback.
8. As a Kul author with a large `.kul` file that renders to an SVG bigger than the viewport, I want native browser scroll inside the preview panel, so that I can scroll to see off-screen parts.
9. As a Kul author closing the preview panel tab, I want re-running `Kul: Show Preview` to reopen the panel cleanly, so that the panel's lifecycle is predictable.

### Kul author reading `examples/03` specifically (v1 tracer)

10. As a Kul author opening `examples/03-three-generations/`, I want to see Ramesh + Sita as a marriage bar in the top generation row, with their daughter Alice's canonical card below.
11. As a Kul author opening the same example, I want to see Alice + Bob as a marriage bar in the middle row, with Bob rendered as a ghost (dotted border, faded fill, ↺ badge) because the marriage has `end:`.
12. As a Kul author opening the same example, I want to see Carol and Ravi in the bottom row, with Carol's edge to the `m_alice_bob` bar drawn solid (birth) and Ravi's drawn dashed (adoption).
13. As a Kul author opening the same example, I want to see Bob's canonical card rendered as a separate orphan-component card (per P8 + P12), because Bob has no birth family declared.

### Future consumer surfaces (out of scope for v1; motivate the architecture)

14. As a future web-app integrator, I want the same kul-layout + kul-svg pipeline available via the wasm bridge, so that the web app produces the exact same canonical visual as the VSCode preview.
15. As a future CLI user, I want to run `kul export --format=svg` to write the canonical SVG to disk, so that I can include a snapshot of my family tree in documentation, slides, or a static website.
16. As a future Kul author using a CLI-exported SVG, I want the file to be self-contained (with its own embedded CSS), so that the SVG renders correctly when opened in any browser without external dependencies.
17. As a future Kul author wanting raster output (PNG, PDF), I want to run the CLI-exported SVG through any standard SVG-to-raster tool (resvg, Inkscape, browser print-to-PDF), so that Kul itself does not have to ship a raster pipeline.

### Toolchain maintainer

18. As a toolchain maintainer, I want kul-layout and kul-svg as separate crates with a clean `PositionedShape` seam between them, so that snapshot tests at each layer fingerprint a focused concern (geometry vs. format).
19. As a toolchain maintainer, I want Walker's algorithm implemented in kul-layout from day one, so that the algorithm is already in place when follow-up issues introduce examples with sibling-subtree overlap.
20. As a toolchain maintainer, I want kul-svg's SVG output to be theme-agnostic (no inline colours; semantic CSS class names), so that theming is a presentational concern owned by each consuming surface rather than a Rust dependency.
21. As a toolchain maintainer, I want `PositionedShape` to remain an internal Rust seam (not `Serialize`, not schema-versioned), so that the seam can evolve freely; the public wire contracts stay anchored at `RenderShape` (input) and the SVG string (output).
22. As a toolchain maintainer, I want the new `kul/render` LSP request to mirror the existing `kul/export` shape (Backend `custom_method` + `features` module + integration test), so that the request fits the established kul-lsp pattern.
23. As a toolchain maintainer, I want every architectural decision in this PRD recorded as an ADR alongside the implementation, so that future contributors can read why the canonical visual is theme-agnostic, why kul-svg ships SVG-only forever, and why layout lives in Rust rather than TS.
24. As a toolchain maintainer, I want the example corpus snapshot scope grown one example (or one pattern primitive) per follow-up issue, so that each follow-up's exit criterion is "examples/NN's snapshot lands" and progress is visible per-PR.

### Pattern decision visibility

25. As a future contributor wondering "why does kul-svg emit theme-agnostic SVG instead of having a theme parameter?", I want an ADR that frames the canonical-visual-vs-interaction-chrome separation principle, so that I can read the rationale rather than re-discover it from code.
26. As a future contributor wondering "why is kul-layout a separate crate when ADR-0016 said layout lives in surface renderers?", I want an ADR documenting the kul-layout crate boundary, so that the seeming conflict is reconciled in writing.
27. As a future contributor wondering "why does kul-svg refuse to ever emit PNG or PDF?", I want kul-svg's boundary ADR to explicitly capture the SVG-only-forever stance, so that no one re-proposes adding a raster path.

## Implementation Decisions

### Crate boundaries

- **`crates/kul-layout/` (new).** Public surface: `pub fn layout(shape: &RenderShape, config: &LayoutConfig) -> PositionedShape`. Depends on `kul-render`. No serde on `PositionedShape`; no schema version. Only `LayoutConfig::default()` is constructed by any consumer in v1.
- **`crates/kul-svg/` (new).** Public surface: `pub fn render(positioned: &PositionedShape, config: &ThemeConfig) -> String`. Depends on `kul-layout`. Returns an SVG string with theme-agnostic structure and semantic CSS class names. Only `ThemeConfig::default()` is constructed by any consumer in v1.
- **No changes to `kul-render`** beyond what is already shipped. ADR-0016 stands.
- **kul-cli and kul-wasm do not gain rendering surfaces in v1.** Their rendering paths are tracked as follow-up issues.

### Layout algorithm

- **Walker's algorithm (Reingold–Tilford–Walker, O(n)) from day one.** Implemented in `kul-layout::walker`. Handles arbitrary trees with sibling-subtree collision avoidance; v1 examples do not trigger collisions, but the algorithm is in place when P6 / P11 follow-ups land.
- **Adapter wraps Walker's** for kul-specific concerns: marriage bars positioned between adjacent spouses; ghost slots at the host's birth-family position per P8; recursive P6 nesting (deferred to follow-up; the data shape anticipates it).
- **No external layout-library dependency.** General-purpose graph layout libraries (dagre, elkjs, cytoscape, react-flow) do not speak the canonical pattern's vocabulary (marriage bars, ghost slots, P6 nesting); a custom Walker's port is small enough (~200 lines) to own.
- **Uniform card dimensions** (monospace font, fixed width × height) per P15's "uniform card" constraint. No DOM text-measurement required; layout is fully deterministic in Rust.

### PositionedShape contract

- **Internal Rust seam between kul-layout and kul-svg.** Public types in kul-layout; not serialisable; not schema-versioned. Snapshot tests target the SVG string (the actual artifact consumers see).
- **Coordinates in absolute pixels.** Cards, bars, edges all carry concrete (x, y) and dimensions.
- **Edges carry computed polyline geometry.** kul-layout decides where each edge's segments go; kul-svg just emits `<polyline points="..." />`.
- **`PositionedEdge` has an extensible `routing` discriminator** with `InTree` (v1) and `CrossTree` (follow-up) variants, so the cross-tree follow-up adds a single match-arm without changing the type's shape.
- **Bounding-box / canvas dimensions** emitted by kul-layout so kul-svg can set the `<svg viewBox>` correctly.

### SVG output (kul-svg)

- **Theme-agnostic.** No inline `fill="..."`. Every visual element carries a semantic CSS class.
- **Class vocabulary** (stable seam, considered public-by-construction): `kul-card`, `kul-card--canonical`, `kul-card--ghost`, `kul-bar`, `kul-bar--ended`, `kul-edge`, `kul-edge--birth`, `kul-edge--adoption`, `kul-label-name`, `kul-ghost-badge`.
- **Edge styles encoded structurally.** `kul-edge--birth` has no dasharray; `kul-edge--adoption` ships with `stroke-dasharray: 6,4` in the kul-svg base styles. Birth-vs-adoption is structural (P5), so the dasharray belongs in the base stylesheet (consuming surface can override).
- **Edge routing in v1 is orthogonal right-angle** for `InTree` edges: bar-midpoint drops to a horizontal bus mid-row, then drops to each child's card top. Sibling children share the bus. Matches classical descendency-tree convention (P1).
- **Ghost visual treatment** (P15): `stroke-dasharray="3,2"` for dotted border; `fill-opacity: 0.4` for faded fill; a small `<text>` element at top-right with the ↺ glyph.
- **No source-span data attributes in v1.** Spans are not propagated through `RenderShape` or `PositionedShape`. The click-to-jump follow-up (F10) adds them additively per ADR-0017.

### `kul/render` LSP request

- **New `crates/kul-lsp/src/features/render.rs`** mirroring `features/export.rs`'s shape: `pub fn render_for(entry: &ProjectEntry, params: &RenderParams) -> Result<RenderResponse, RenderRequestError>` + `Backend::render` handler.
- **Request shape:** `{ uri: Url }`. No format / config params in v1.
- **Response shape:** success → `{ ok: true, svg: string }`; failure → `{ ok: false, diagnostics: [...] }` (same diagnostic shape the `ExportEnvelope` failure variant uses).
- **Custom-method registration** in `crates/kul-lsp/src/lib.rs` alongside `kul/export`.

### VSCode extension

- **New command `kul.preview.show`** registered in `package.json` as `Kul: Show Preview`. Opens a `WebviewPanel` beside the active editor.
- **Panel lifecycle.** One panel per extension session; reopening focuses the existing panel rather than spawning duplicates. `panel.onDidDispose` clears the reference.
- **Render trigger.** Extension listens for `vscode.workspace.onDidChangeTextDocument`. A 300 ms debounce timer fires `kul/render` once changes settle, then posts the SVG to the webview.
- **Initial render.** When the panel opens, an immediate `kul/render` runs for the currently-active `.kul` document.
- **Webview HTML.** Minimal `<body><div id="root"></div><style>…preview.css…</style></body>`; a tiny inline script handles the typed message protocol (`addEventListener('message', …)`).
- **Webview stylesheet** at `editor/vscode/media/preview.css`, ~30 lines. Maps the kul-svg semantic classes to VSCode CSS variables (`var(--vscode-editor-foreground)`, `var(--vscode-editor-background)`, etc.).
- **No svg-pan-zoom, no click handlers, no hover effects, no selection sync** in v1.

### Webview ↔ extension message protocol

- **Typed messages.** Extension → webview: `{ type: 'render', svg: string }` (success) or `{ type: 'renderError', message: string, diagnosticCount: number }` (failure). Webview → extension: nothing in v1.
- **Failure rendering.** The webview shows a styled HTML banner ("Document has N errors — see Problems panel") on `renderError`. Not a placeholder SVG; HTML so the link, copy, and iconography are unconstrained.
- **No state persistence** across VSCode reloads (no `WebviewPanelSerializer`).
- **No bidirectional protocol** in v1.

### Scope deferrals (each tracked as its own follow-up issue)

- **Interactivity** (pan/zoom, click-to-jump, hover, selection sync) — separate issues.
- **Cross-tree edges** (P4, P10, P11) — deferred per pattern primitive. `PositionedEdge::routing::CrossTree` lands with the first follow-up that needs it.
- **P6 recursive nesting** — deferred. May need a new corpus example.
- **CLI `kul export --format=svg`** — separate issue.
- **kul-wasm exposure** of the render pipeline — separate issue (amends/follows ADR-0011).
- **Performance / virtualisation** for 5,000-card docs — separate issue. v1 ships unoptimised; throttled didChange provides natural backpressure.

### ADR coverage

Three ADRs land alongside the implementation:

- **kul-layout crate boundary ADR.** Why a fourth Rust crate sibling to kul-render. Walker-from-day-one rationale. Internal-seam (not `Serialize`) nature of `PositionedShape` and the rationale (wire contracts are already pinned at `RenderShape` and the SVG string; a third versioned shape has no external consumer).
- **kul-svg crate boundary ADR.** SVG-only-forever stance (downstream consumers convert to raster themselves). CSS-class vocabulary as a stable seam. Theming-at-consumption rationale.
- **Canonical-visual-vs-interaction-chrome separation principle ADR.** Standalone cross-cutting ADR. Future surface renderers (web app, native preview) cite this when deciding what is "canonical" vs. "their UI." Theming is chrome; interactivity is chrome; the canonical SVG structure is theme-and-interaction-agnostic.

A `kul/render` LSP request ADR is **not** needed — the request is a small additive feature mirroring `kul/export` and follows the kul-lsp idiom directly.

### Module sketch — deep modules to build or deepen

- **`kul-layout::walker` (new deep module).** The canonical Reingold–Tilford–Walker port. Takes a tree (kul-layout's internal representation derived from `RenderShape`) and emits positions. Small input/output surface; encapsulates the algorithm's state.
- **`kul-layout::adapter` (new deep module).** Wraps Walker's for kul's pattern: marriage bar between spouses, ghost slots, generation rows from generation indices. Recursive composition; hides Walker's complexity from the public surface.
- **`kul-svg::emit` (new deep module).** String templating for SVG output. Stateless; maps `PositionedShape` elements to SVG elements without state-machine complexity.
- **`kul-lsp::features::render` (new).** Pure function `render_for(entry, params) -> Result<RenderResponse, RenderRequestError>` mirroring `features::export::export_for`. Backend handler is thin.
- **VSCode webview controller (new, small).** Panel lifecycle + debounce + message dispatch. Not deep — too thin to abstract.

## Testing Decisions

External-observable behaviour only. The existing project conventions (snapshot tests via `insta` per ADR-0003; positive corpus in `examples/`; fabricated edge cases in test files; manual verification for thin UI wiring) carry over without modification.

### Modules to test

1. **kul-layout — corpus snapshot test.** `tests/corpus.rs` runs the full pipeline `kul_core::export::export` → `kul_render::transform` → `kul_layout::layout` against `examples/03-three-generations/` only in v1, snapshotting the YAML-serialised `PositionedShape`. Each follow-up issue extends the example set by one (or one pattern primitive's worth). Snapshots committed; accepted via `cargo insta review` per ADR-0003.

2. **kul-layout — Walker's algorithm unit tests.** `tests/walker.rs` exercises Walker's with hand-fabricated trees the corpus does not naturally surface: sibling-subtree collisions, deeply-nested branches, degenerate single-child paths, empty trees. Each scenario snapshots the positioned tree. These tests catch Walker's-specific regressions independent of canonical-pattern adapter logic.

3. **kul-svg — corpus snapshot test.** `tests/corpus.rs` runs the full pipeline against `examples/03-three-generations/` and snapshots the SVG string via insta's default text snapshot. Snapshots are diff-readable as raw SVG. Each follow-up issue extends the example set.

4. **kul-svg — visual vocabulary tests.** `tests/visual.rs` exercises the SVG emitter with hand-fabricated `PositionedShape` inputs that pin specific class-vocabulary decisions: ghost cards emit `kul-card--ghost`, adoption edges emit `kul-edge--adoption`, marriages with `end:` emit `kul-bar--ended`, etc. Each scenario snapshots the relevant SVG fragment.

5. **kul-lsp — `kul/render` integration test.** `tests/render.rs` mirrors `tests/export.rs`: open a document via the test harness, fire `kul/render`, assert the response shape. One success case + one failure case (document with diagnostics → `ok: false, diagnostics: [...]`).

6. **VSCode extension — manual verification only.** Test plan documented in the implementation PR description:
   - Open `examples/03-three-generations/threegens.kul`.
   - Run `Kul: Show Preview`. Verify panel opens with the three-generation diagram.
   - Edit Alice's name. Verify preview updates within ~300 ms.
   - Toggle VSCode between light and dark theme. Verify preview re-themes immediately (no re-render needed).
   - Introduce a deliberate parse error. Verify error banner appears with the diagnostic count.
   - Fix the error. Verify the diagram returns.
   - Close the panel tab. Re-run `Kul: Show Preview`. Verify the panel reopens cleanly.

### Prior art

- Snapshot tests via `insta`: existing pattern in `crates/kul-render/tests/`, `crates/kul-core/tests/`.
- LSP integration tests with hand-rolled stdio client: `crates/kul-lsp/tests/export.rs` is the direct template for `tests/render.rs`.
- Examples corpus loop pattern: `crates/kul-render/tests/corpus.rs` is the direct template for kul-layout's and kul-svg's `corpus.rs`.
- No prior art for VSCode-extension TS testing in this repo (existing extension has no tests). Manual verification is the established convention.

## Out of Scope

- **Any interactivity in v1.** Pan, zoom, click-to-jump-to-source, hover, selection sync — all separate follow-up issues.
- **Examples beyond `examples/03`.** The other ten corpus examples are deferred, one follow-up issue per pattern primitive (or pattern-primitive group). Each follow-up's exit criterion is "examples/NN's snapshot lands and renders correctly in the preview."
- **Cross-tree edges** (P4, P10, P11) — deferred. `PositionedEdge` anticipates them via the `routing` discriminator but only `InTree` is constructed in v1.
- **P6 recursive nesting** — deferred. May require a new corpus example.
- **CLI `kul export --format=svg`** — separate issue.
- **kul-wasm exposure** of the render pipeline — separate issue.
- **PNG, PDF, or any raster output — ever.** kul-svg emits SVG only. Downstream consumers run resvg, Inkscape, browser print-to-PDF, or any other tool. The kul-svg boundary ADR pins this.
- **Web app, native preview, mobile app.** Future surfaces, not in this epic.
- **Performance optimisation for large documents.** Throttled didChange provides natural backpressure for v1. Virtualisation, culling, level-of-detail are a separate follow-up.
- **User-facing configuration.** Themes are CSS-only (surface owns). Layout density, alternative algorithms, font choices, etc. are deferred. The internal `LayoutConfig` and `ThemeConfig` types exist as seams but only `::default()` is constructed in v1.
- **Editor selection sync.** Editor cursor → highlight matching card. Bidirectional protocol; multi-quarter feature; explicit follow-up.
- **Failure visualisation inside the SVG.** Failure case shows as an HTML banner in the webview, not as a placeholder SVG. kul-svg's job is canonical rendering of valid `PositionedShape`; failure paths bail to the surface.

## Further Notes

### Issue breakdown

This PRD fans out into one implementation issue (v1) plus a series of follow-up issues, all decomposed using the to-issues tracer-bullet pattern:

| Issue | Scope | Type | Blocked by |
|---|---|---|---|
| **v1 implementation** | kul-layout + kul-svg + `kul/render` + VSCode preview; `examples/03` renders end-to-end | AFK | none |
| F1: kul-wasm exposure | Expose kul-layout + kul-svg surfaces to the JS host | AFK | v1 |
| F2: `examples/02` + `/04` | Extend corpus to nuclear family and polygamous family | AFK | v1 |
| F3: `examples/08` | Deeper P8 ghost exercise (divorce + remarriage) | AFK | v1 |
| F4: `examples/06` + `/10` | P12 multi-component ordering and orphan packing | AFK | v1 |
| F5: `examples/05` + `/11` | P11 within-family cross-edges (first `CrossTree` variant) | AFK | v1 |
| F6: `examples/09` | P16 multi-adoption child-ghosts | AFK | v1 |
| F7: `examples/07` | Multi-file project rendering (URI handling) | AFK | v1 |
| F8: P6 recursive nesting | Joining-spouse birth-family nesting (may need new example) | HITL | v1, F5 |
| F9: webview pan/zoom | First interactivity; svg-pan-zoom OSS dep | AFK | v1 |
| F10: webview click-to-jump | Span propagation through `RenderShape` + `PositionedShape`; webview ↔ extension protocol | HITL | v1 |
| F11: webview hover effects | Pure CSS | AFK | v1 |
| F12: webview selection sync | Editor cursor → highlight; bidirectional protocol | HITL | v1, F10 |
| F13: CLI `kul export --format=svg` | Wire kul-svg into kul-cli; self-contained SVG | AFK | v1 |
| F14: performance / virtualisation | Large-doc support; design TBD | HITL | v1 |

The v1 implementation issue is expected to land as **two PRs** for review hygiene:

- **PR1 (Rust):** kul-layout + kul-svg + `kul/render` LSP. Snapshot-tested in isolation; ships "dead" with no UI consumer. Mirrors how kul-render (#121) landed.
- **PR2 (TS):** VSCode extension wiring — command, panel lifecycle, debounce listener, message protocol, error banner, `preview.css`. Thin layer over PR1; reviews on UX merits.

### Relationship to `canonical-ui-pattern.md` and ADR-0016 / ADR-0017

- `docs/canonical-ui-pattern.md` is the **pattern spec**. It defines what the renderer must draw and constrains acceptable visual treatments. This PRD does not modify it.
- ADR-0016 (kul-render crate boundary) stays intact. Its anti-suggestion "do not add a layout / pixels layer to kul-render" is honoured — layout and SVG live in *new sibling crates*, not amendments to kul-render.
- ADR-0017 (`RenderShape` schema) stays intact. The render shape is unchanged in v1; the click-to-jump follow-up (F10) will add `span` fields additively per ADR-0017's no-bump rule.
- The new kul-layout and kul-svg crate-boundary ADRs document architectural decisions made *outside* what ADR-0016 covered. They are peer ADRs, not amendments.

### Naming retirement

The "Stage 3" framing inherited from the kul-render epic (#110) is retired with this PRD. The pipeline is named by its data shapes (`RenderShape`, `PositionedShape`, SVG string) and the crates that produce them (kul-render, kul-layout, kul-svg). New contributors do not need to learn the Stage 1 / Stage 2 / Stage 3 decomposition.

### Risks

- **Walker's algorithm correctness.** The algorithm is well-documented but kul-layout's adapter (marriage bars, ghost slots, recursive nesting) is novel composition. Snapshot tests on `examples/03` catch the v1 regression risk; the `tests/walker.rs` unit tests with fabricated collision scenarios catch the algorithm's own behaviour independent of the adapter. P6 nesting risk is deferred along with F8.
- **CSS-variable theming coverage.** The mapping from kul-svg semantic classes to VSCode CSS variables is a hand-curated stylesheet. The risk is missing or wrong-named variables producing unreadable output in some themes. Mitigation: manual verification across all three VSCode theme kinds (light, dark, high-contrast) before the TS PR merges.
- **300 ms debounce calibration.** May feel sluggish on small documents and overwhelmed on large ones. Tunable; F14 (performance) is the right place to revisit.
- **`kul/render` adds a per-keystroke pipeline pass.** For v1's tracer slice (~7 visible elements) the pipeline runs in single-digit milliseconds; large-document pressure is a follow-up concern handled by F14.

### Grilling provenance

This PRD is the synthesised output of a multi-turn grilling session against issue #122 conducted 2026-05-23. Thirteen load-bearing decisions were resolved in that session (tracer slice, crate decomposition, render pipeline, layout algorithm, PositionedShape role, edge routing, theming strategy, interaction scope, render trigger, messaging contract, testing approach, PR decomposition, ADR coverage). Issue #122 closes when this PRD lands; the v1 implementation issue and the 14 follow-ups become the active trackers.

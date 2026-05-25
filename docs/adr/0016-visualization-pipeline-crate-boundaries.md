# ADR 0016 — The visualization pipeline: four crates and the structural/chrome line

**Status:** Accepted
**Date:** 2026-05-25
**Deciders:** owner

## Context

A Kul document travels from source to pixels through four named data forms:

```
ExportEnvelope ──▶ RenderShape ──▶ PositionedShape ──▶ SVG string
  (kul-core)        (kul-render)     (kul-layout)        (kul-svg)
```

`kul-core` produces the first: the kinship-native [`ExportEnvelope`](../../crates/kul-core/src/export.rs) that mirrors the language's primitives one-to-one. Surface renderers (the VSCode preview panel today; a web app, a native preview, a CLI export tomorrow) consume the last. Between them sit three questions, each with a different rate of change:

1. **What does the canonical UI pattern ([`docs/canonical-ui-pattern.md`](../canonical-ui-pattern.md)) look like *as data*?** — cards, ghosts, marriages, components, generations, recursive nesting. This co-evolves with the language and changes slowly.
2. **Where does that data sit on a 2D plane?** — the positioning algorithm plus the canonical-pattern adapter. This is layout policy and may grow alternative algorithms, density controls, or level-of-detail.
3. **How is the positioned diagram emitted for a viewer?** — the final serialization to a format a browser or file viewer can display.

Bundling any two of these into one owner conflates concerns with different rates of change and forces a consumer who wants one to pull in the others. A fourth recurring question cuts across all three: **what is part of the canonical visual, and what is a particular surface's UI?** Answered case by case, that boundary drifts — theming, click-to-jump, and pan/zoom each look innocuous to bake in for one surface's convenience, and the cumulative result is a renderer that knows about every surface that ever consumed it.

## Decision

Three sibling crates, each owning one question, plus one cross-cutting principle governing the fourth. The dependency graph is strictly one-directional: `kul-svg → kul-layout → kul-render → kul-core`. None of the three depend on `kul-loader`, `kul-cli`, or `kul-lsp`.

### `kul-render` — the canonical pattern as data

Owns the pattern's vocabulary and the projection that produces it. Two public functions:

- `pub fn transform(envelope: &ExportEnvelope) -> RenderShape` — a pure transformer reading only the kinship-native graph, never the AST or resolver state. The [#117 audit](./0008-export-kinship-native-shape.md) confirmed the kinship-native shape carries every fact the pattern needs.
- `pub fn compute(check: &CheckResult) -> RenderShape` — a convenience wrapper that calls `kul_core::export::export` with positions enabled, then `transform`. This is the one place in the crate that touches `kul-core::export`.

Both surfaces are public so tests are independent: `compute` runs against the `examples/` corpus; `transform` runs against fabricated envelopes for edge cases the corpus does not naturally surface. The crate is filesystem-free and splits into `shape.rs` (the `RenderShape` types and schema-version constant; [ADR-0017](./0017-render-shape-schema-and-versioning.md)) and `build.rs` (the projection).

### `kul-layout` — positioning

Owns the positioning algorithm (a Reingold–Tilford–Walker port) and the canonical-pattern adapter that wraps it ([ADR-0018](./0018-canonical-layout-algorithm.md)). One public function:

```rust
pub fn layout(shape: &RenderShape, config: &LayoutConfig) -> PositionedShape;
```

`PositionedShape` is an **internal Rust seam**, not a wire shape: not `Serialize`, not schema-versioned, not part of any cross-process contract. The crate exposes the type publicly so `kul-svg` and future Rust consumers can read it, but the versioned wire contracts are pinned at `RenderShape` (input) and the SVG string (output). A third versioned shape between them has no external consumer; pinning one would cost a migration policy for no benefit.

### `kul-svg` — emission, SVG-only forever

Owns the final emission step. One public function:

```rust
pub fn render(positioned: &PositionedShape, config: &ThemeConfig) -> String;
```

The output is SVG and only SVG, forever. SVG expresses every primitive the canonical pattern uses — cards (`<rect>`), edges (`<path>`), labels (`<text>`), grouping (`<g>`) — directly, so it is the format closest to the pattern's vocabulary. Consumers that need raster output (PNG, PDF, JPG) run a standard SVG-to-raster tool (`resvg`, Inkscape, browser print-to-PDF) on the string; the Rust toolchain ships no raster pipeline.

The emitted SVG is **theme-agnostic**: no inline `fill`, `stroke`, or `color`. Every element carries a semantic CSS class; theming is applied by the consuming surface via a stylesheet. The class + attribute vocabulary is a stable, public-by-construction seam.

**A CSS class names the entity *type*; every *property* of that entity is a `data-*` attribute.** This keeps the class set small and closed (one class per primitive) while the property surface grows additively as the language does — every declared Person / Marriage / birth / adoption property plumbs through to a `data-*` attribute ([ADR-0021](./0021-language-properties-plumb-to-svg.md)). Two attribute conventions:

- **Booleans** use `data-is-<adjective>="true|false"` — `data-is-alive`, `data-is-ended`, `data-is-past`. Always emitted (both truth values are meaningful).
- **Enumerations** use explicit enum strings — `data-kind="canonical|ghost"`, `data-gender="male|female|other"`, `data-ghost-reason`, `data-link-kind="birth|adoption|marriage"`, `data-end-reason`.
- A **missing / unknown** optional value omits the attribute entirely (no empty strings) — the pattern's "absence, not placeholders".

The classes are:

- `kul-card` — a person card. Properties: `data-person-id`, `data-kind`, `data-ghost-reason` (ghost only), `data-gender`, `data-is-alive`, `data-born`, `data-died`, `data-family`, `data-given`, `data-generation`.
- `kul-edge` — a birth / adoption / marriage connector. Properties: `data-link-kind`, `data-marriage-id`; for birth / adoption `data-child-id`, `data-is-past`, plus adoption's `data-adoption-start` / `data-adoption-end`; for the unified marriage connector (ADR-0020) `data-host-id`, `data-joining-id`, `data-start`, `data-end`, `data-end-reason`, `data-is-ended`.
- `kul-label-name`, `kul-ghost-badge` — the card's name `<text>` and the ghost `↺` badge.

Distinctions that are properties of *what an element is*, rather than of theme, ship structurally in the base SVG: an adoption edge carries `stroke-dasharray="6 4"` (birth is solid); a ghost card carries `stroke-dasharray="3 2"` and a `<text>` ↺ badge. These are inline because they are structural, not theming; a consuming surface that wants the property programmatically reads the corresponding `data-*` attribute (`data-link-kind`, `data-kind`). Consuming surfaces override colours, stroke widths, and opacities via CSS — selecting on the entity class and `data-*` attributes (e.g. `.kul-card[data-kind="ghost"]`, `.kul-edge[data-link-kind="marriage"]`) — without disturbing these structural marks.

`ThemeConfig` and `LayoutConfig` are `Default`-only structs in v1 — forward-compatibility seams for future per-emission or per-layout tweaks (opt-in source-span attributes, self-contained inline CSS, density, font metrics) that add fields without changing a function signature.

### The structural/chrome line

**The canonical visual is theme- and interaction-agnostic. Theming and interactivity are chrome, owned by each consuming surface.** The test is a single question: *does the choice change how the kinship reads?*

- **Structural (lives in Rust):** card position relative to other cards, card kind (canonical vs ghost), edge kind (birth vs adoption vs marriage), generation row, the entity-class + `data-*` attribute vocabulary that lets chrome hook in. A wrong position misrepresents kinship; a wrong edge style conflates adoption and birth; a missing ghost loses a past structural fact.
- **Chrome (lives in the surface):** light/dark/high-contrast theme (CSS variable → CSS rule), pan/zoom (a webview JS library), hover effects (`:hover` over the existing classes), click-to-jump and editor-cursor sync (a webview ↔ extension message protocol). A theme does not change who is married to whom; a pan does not move Carol's parents to a different bar.

Where a feature has a structural enabler and a chrome consumer — source spans enabling click-to-jump — the enabler lives in Rust and the handler lives in the surface.

## Consequences

- **Each concern lives once, in Rust, shared across surfaces.** The VSCode preview, a future web app, a native preview, and a CLI export all consume the same `RenderShape`, `PositionedShape`, and `render`. Visual drift across surfaces becomes a deliberate per-surface CSS choice, not an emergent property of three independent reimplementations.
- **The dependency graph stays acyclic and one-directional.** A consumer pulls exactly the layers it needs: a tooling integration wanting card centroids depends on `kul-layout` and ignores `kul-svg`; a text-mode dump tool depends on `kul-render` and ignores positions.
- **Theming is a CSS concern.** The VSCode preview maps the class vocabulary to VSCode CSS variables in a ~30-line stylesheet (`editor/vscode/media/preview.css`); a web app maps the same classes to its brand palette. Adding a theme is a stylesheet change, not a Rust release.
- **Schema versioning is per-shape and independent.** `RENDER_SCHEMA_VERSION` follows the [ADR-0010](./0010-export-schema-versioning.md) discipline but bumps under its own conditions ([ADR-0017](./0017-render-shape-schema-and-versioning.md)); `PositionedShape` is unversioned by design.
- **Future cross-cutting decisions cite the structural/chrome line.** "Should X live in `kul-svg` or in the webview?" resolves to: does X change how the kinship reads? Structural → Rust; chrome → surface.

## Anti-suggestions (do not re-propose)

### Crate ownership

- **"Inline `kul-render` into `kul-wasm`."** The pattern's data form is a logic concern; binding it to one adapter (the JS host) forecloses the in-process renderer. The wasm surface stays exactly the three shapes [ADR-0011](./0011-wasm-surface-three-shapes-no-wrappers.md) committed to.
- **"Pass `&ResolvedDocument` to `transform` instead of an `ExportEnvelope`."** That lets the projection re-read AST detail the export does not carry — the exact violation [#117](./0008-export-kinship-native-shape.md) audited against. The kinship-native graph is the contract. The same ban applies to `kul-layout`: it reads `RenderShape` only.
- **"Fold positioning back into `kul-render` (compute positions inside `compute`)."** The data form and the positioning have different rates of change — `RenderShape` is the canonical pattern as data (slow); `PositionedShape` is layout policy (fast, may grow alternative algorithms). Bundling them forces a consumer who wants the data but not the positions to pull in the positioning code.
- **"Inline `kul-layout` into `kul-svg`."** That binds the positioning algorithm to one output format; a consumer wanting positioned data without SVG would have to depend on `kul-svg` and discard the string.
- **"Re-export `ExportEnvelope` from `kul-render` (or `PositionedShape` from `kul-svg`)."** Two import paths for one type. Consumers that need both pull the type from its owning crate directly.

### `PositionedShape` and external dependencies

- **"Make `PositionedShape` `Serialize` and schema-version it."** No v1 consumer reads positioned data out of process. Reify the contract only when an out-of-process consumer appears — then the policy is the same as [ADR-0010](./0010-export-schema-versioning.md) / [ADR-0017](./0017-render-shape-schema-and-versioning.md) transposed once more.
- **"Use an external layout library (dagre, elkjs, cytoscape, react-flow)."** None speak the canonical pattern's vocabulary — the absorb rule, ghost slots, recursive nesting, the polygamy fan. A custom Walker's port is small enough (~200 lines) to own; adding a dependency to avoid it is poor value.

### SVG scope and theming

- **"Add a PNG / PDF / Canvas / raster output to `kul-svg`." Ever.** SVG plus standard external tools covers every raster need; bundling a raster pipeline multiplies the dependency surface (image crates, font rasteriser, colour management) for a feature consumers satisfy themselves. This is the load-bearing reason `kul-svg`'s scope is pinned.
- **"Emit HTML+CSS instead of SVG."** HTML+CSS positioning of card grids plus polyline edges needs either absolute positioning + DOM measurement or grid layout that does not express polylines naturally. SVG expresses every primitive directly; HTML wrappers are a surface concern.
- **"Add a theme parameter to `kul-svg::render`," or "bake themes (or the VSCode CSS variables) into the SVG."** Pushes the theme catalogue into Rust — every theme a Rust release, every custom theme a fork — and couples the renderer to one surface's variable namespace. The CSS-class seam already gives consumers full theming control.
- **"Inline source spans on every element by default."** No v1 consumer reads them; they would inflate every SVG and bind the format to a future click-to-jump wire shape before it is designed. They arrive via a `ThemeConfig` opt-in when the consumer does.

### The structural/chrome line

- **"Put pan/zoom or click handlers into `kul-svg::render`."** Pan/zoom and event handling are runtime behaviour, not SVG structure. Their home is a per-surface JS library and message protocol. The structural enabler (source spans) lives in Rust; the handler that consumes it lives in the surface.
- **"Make `RenderShape` carry positions so the SVG is 'fully self-describing.'"** `RenderShape` is structural on the pattern's hierarchy axis; positions are a different rate of change. Different rates of change → different crates.

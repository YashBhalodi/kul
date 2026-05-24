# ADR 0019 — `kul-svg` is a separate crate, SVG-only forever

**Status:** Accepted
**Date:** 2026-05-23
**Deciders:** owner

## Context

[ADR-0018](./0018-kul-layout-crate-boundary.md) places positioning in `kul-layout` and pins `PositionedShape` as an internal Rust seam. Surface renderers (VSCode preview, web app, future CLI export) still need to turn `PositionedShape` into something a browser, an `<img>` tag, or a file viewer can display.

Three placements were on the table for that final emission step:

1. **Each surface emits its own format.** VSCode webview gets HTML+CSS; web app gets HTML+CSS+React; CLI gets … something. Three implementations of the same visual vocabulary (cards, marriage bars, edges, ghosts) drift across surfaces.
2. **One emission layer with format dispatch (SVG / PNG / PDF / Canvas).** Adapts the canonical visual to many output formats.
3. **One emission layer, SVG-only.** Downstream consumers convert SVG to other formats themselves via standard tools (resvg, Inkscape, browser print-to-PDF).

Two pressures shaped the decision. First, the canonical UI pattern is **structural**: cards, bars, edges, generation rows. SVG expresses every one of these directly with `<rect>`, `<line>`, `<polyline>`, `<text>`, `<g>`. Picking the format closest to the pattern's vocabulary minimises adapter cost. Second, surface theming (light, dark, high-contrast in VSCode; brand colours in a web app; print monochrome in a CLI export) is a **consumer concern**, not a renderer concern. A theme-baking renderer either (a) ships every theme the project ever needs as a parameter, or (b) ships no themes and pushes the choice out. (b) keeps the renderer simple and the theming free.

Option 2 (multi-format dispatch) was rejected. Every additional output format multiplies the test surface, the maintenance cost, and the chances of inconsistent canonical visuals across formats — and downstream consumers can already get raster output from SVG via standard tools.

## Decision

A new crate at `crates/kul-svg/`, sibling to `kul-layout`. It depends on `kul-layout` (one direction) and is depended on by `kul-lsp` (for the `kul/render` request) and future CLI / web-app / native-preview adapters. It does **not** depend on `kul-loader`, `kul-cli`, or `kul-render` directly (consumers that want both `RenderShape` and SVG pull both crates).

One public function:

```rust
pub fn render(positioned: &PositionedShape, config: &ThemeConfig) -> String;
```

`ThemeConfig` is a struct with `Default`; only `ThemeConfig::default()` is constructed by any consumer in v1. The struct exists as a forward-compatibility seam — future per-emission tweaks (e.g. opt-in source-span data attributes for click-to-jump, opt-in inline CSS for self-contained CLI export) add fields here without changing the function's signature.

Internally the crate is one deep module:

- **`kul_svg::emit`** — string templating from `PositionedShape` to SVG. Stateless; maps `PositionedShape` elements to SVG elements without state-machine complexity. Writes into a `String` buffer; returns the assembled SVG.

The emitted SVG is **theme-agnostic**:

- No inline `fill="…"`, no inline `stroke="…"`, no inline `color="…"`. Every visual element carries a semantic CSS class name; theming is applied by the consuming surface via a stylesheet.
- **Class vocabulary (stable seam, public-by-construction):**
  - `kul-card`, `kul-card--canonical`, `kul-card--ghost`
  - `kul-bar`, `kul-bar--ended`
  - `kul-edge`, `kul-edge--birth`, `kul-edge--adoption`
  - `kul-label-name`, `kul-ghost-badge`
- **Edge dasharrays are structural and ship in the base SVG.** `kul-edge--birth` carries no dasharray (solid line). `kul-edge--adoption` ships with `stroke-dasharray="6 4"` directly in the `<polyline>` element. Birth-vs-adoption is structural (P5) — the visual distinction is a property of *what the edge is*, not of theme — so it belongs in the SVG itself. Consuming surfaces can override colours, stroke widths, and opacities via CSS without disturbing the dash pattern.
- **Ghost visual treatment (P15) is structural.** Cards carrying `kul-card--ghost` ship with `stroke-dasharray="3 2"` and a `<text>` ↺ badge. Consuming surfaces theme via CSS.
- **Edge routing is orthogonal right-angle for *every* edge** (`InTree` and `CrossTree` alike): bar-midpoint drops to a horizontal bus mid-row, then drops to each child's card top. Matches classical descendency-tree convention (P1). The `kul-edge--in-tree` / `kul-edge--cross-tree` class distinction is a future re-theming hook only — geometry, attachment points, dashes, and stroke are identical between the two. See [ADR-0018](./0018-kul-layout-crate-boundary.md) for the routing decision.
- **No source-span data attributes in v1.** The click-to-jump follow-up (F10) adds them additively per ADR-0017.

The emitted SVG is **self-contained except for theming**: it carries no `<defs>` referencing external stylesheets, no external `<link>`s, no script. A consumer can drop it into any HTML document; a consumer that wants a fully-self-contained SVG file (for offline viewing without a host stylesheet) wraps a default `<style>` block around it. F13 (CLI export) does this.

## Consequences

- **One visual emission layer for every surface.** VSCode preview, future web app, future native preview, future CLI export all consume `render(&PositionedShape, &ThemeConfig)`. Visual drift across surfaces becomes a deliberate per-surface choice (overriding the base SVG via CSS) rather than an emergent property of independent reimplementations.
- **Theming is consumer policy.** The VSCode preview maps `kul-card` → `var(--vscode-editor-foreground)` for the stroke, `var(--vscode-editor-background)` for the fill, in a ~30-line stylesheet (`editor/vscode/media/preview.css`). A web app maps the same classes to its brand palette; a CLI export wraps a default light-theme stylesheet inside the SVG. None of those decisions live in `kul-svg`.
- **Class vocabulary is a stable seam.** Adding a new class (e.g. `kul-card--multi-adopted` if F6 wants per-multi-adoption emphasis) is additive — no consumer breaks. Removing or renaming a class is a breaking change to every theming surface; treat it accordingly.
- **No raster path inside kul-svg.** Consumers who need PNG, PDF, JPG run `resvg`, Inkscape, browser print-to-PDF, or any other SVG-to-raster tool on the emitted string. The Rust toolchain does not ship a raster pipeline.
- **The dependency graph stays unidirectional.** `kul-svg → kul-layout → kul-render → kul-core`. The LSP depends on `kul-svg` to fulfil `kul/render`.

## Anti-suggestions (do not re-propose)

- **"Add a PNG / PDF / Canvas / raster output to kul-svg."** **Ever.** SVG plus standard external tools already covers every raster need; bundling a raster pipeline multiplies the dependency surface (image crates, font rasteriser, colour management) for a feature consumers can satisfy themselves. This anti-suggestion is the load-bearing one for kul-svg's scope and is the reason this ADR exists.
- **"Bake themes into the SVG (light / dark / hc as a parameter)."** Pushes the theme catalogue into Rust. Every new theme is a Rust release; every consumer that wants a custom theme has to fork. The CSS-class seam moves that cost out of Rust into a per-consumer stylesheet, where it belongs (theming is chrome — see [ADR-0020](./0020-canonical-visual-vs-interaction-chrome.md)).
- **"Emit HTML+CSS instead of SVG."** HTML+CSS positioning of arbitrary card grids plus polyline edges requires either absolute positioning + DOM measurement or grid layout that doesn't naturally express polylines. SVG expresses every primitive directly. HTML wrappers are a surface concern (the VSCode webview wraps the SVG in `<body>`, the web app may wrap it in a React component) — `kul-svg` emits the SVG and stops.
- **"Inline source spans on every element by default."** v1 has no consumer that needs them; F10 (click-to-jump) is the first. Adding them now would pollute every SVG with attributes no v1 consumer reads, inflate snapshot tests, and bind the format to a follow-up's wire shape before the follow-up is designed. F10 adds them via `ThemeConfig` opt-in.
- **"Re-export `PositionedShape` from `kul-svg`."** Adds two import paths for the same type. Consumers that need both pull `kul_layout::PositionedShape` directly; the type is part of the published `kul-layout` surface.

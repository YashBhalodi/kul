# ADR 0022 — Diagram legend: CLI baked-SVG band + preview chrome overlay

**Status:** Accepted
**Date:** 2026-05-30
**Deciders:** owner

## Context

A reader who opens a Kul diagram for the first time sees gender-tinted card strokes, ghost cards, three edge kinds, and an ended-marriage treatment, without a key. The canonical pattern's *visual vocabulary* is well-defined in [`docs/canonical-ui-pattern.md`](../canonical-ui-pattern.md), but neither rendering surface surfaces it inline: a standalone CLI-exported SVG is silent about its own conventions, and the VSCode preview reader has nothing to glance at to confirm "is the dashed border a ghost or an adoption edge?". A legend — a small key keyed to the diagram itself — closes that gap.

Two surfaces consume the canonical visual today (`kul export --format=svg` and the VSCode preview), with one rendering pipeline behind them (`kul-render → kul-layout → kul-svg`), and they need the same legend information on different terms:

- A standalone exported SVG must be **self-explanatory** when opened in a browser or dropped into an `<img>` — the legend has to be *part of the file*.
- The VSCode preview is interactive chrome over a live SVG, with pan/zoom controls already drawn over the canvas as an HTML overlay. Its legend should be **chrome of the same kind**, sitting in the same overlay layer alongside `#kul-controls` and the ghost `↺` badge — emitted into the SVG it would couple the always-emitted SVG (and the wasm/LSP-served SVG) to a particular surface's UI.

The structural/chrome line ([ADR-0016](./0016-visualization-pipeline-crate-boundaries.md), "The structural/chrome line") already settles which side a feature sits on by asking *does the choice change how the kinship reads?*. A legend does not change the kinship; it labels the existing conventions. So the legend is **chrome**, and the right place for chrome differs by surface — emitted into the standalone file when the file *is* the surface, drawn as an overlay when there is a chrome layer.

Two cross-cutting risks govern the design:

1. **Colour drift.** Every swatch is a coloured shape, and a naïve implementation would either (a) hardcode swatch colours per surface or (b) introduce new "legend" colour tokens that mirror the diagram tokens. Either path creates two sources of truth — change the diagram's marriage edge colour and the legend's marriage swatch silently keeps the old hex. The seam already exists: the diagram's stroke colour is reached via a CSS rule keyed on the production class + `data-*` attribute. If the legend's swatch is a miniature of the real glyph carrying the same class + `data-*`, the same rule paints it. **Colour stays a single source of truth on each surface.**
2. **Implementation drift.** Two surfaces, one normative spec. If the row table (canonical order, exact label strings, presence rules) lives only in code, the two implementations can quietly disagree — different label, different order, one shows "Other" the other does not. The spec lives in `docs/canonical-ui-pattern.md` as the single normative source.

A separate sub-question is whether the legend's *content* is static or dynamic. Static would mean every legend lists every category — a reader of a nuclear-family diagram still sees "Past record", "Adoption", "Ended marriage" rows for shapes that do not appear in the diagram. Dynamic means rows appear only for categories actually present. The latter keeps the legend tight against the diagram it keys; the cost is a per-diagram presence check, trivial on either surface.

## Decision

### Architecture: one normative table, two emitters

The canonical legend table — its row order, exact label strings, presence rules, and the production class / `data-*` attribute each swatch reuses — is **normative in [`docs/canonical-ui-pattern.md`](../canonical-ui-pattern.md)**. Both implementations conform to it:

- **CLI surface** — `kul-svg` owns the legend end-to-end as an emission concern. `ThemeConfig` gains a `legend: bool` (default `false`) and a chainable `with_legend(self, bool) -> Self`. When opted in, `render()` walks the `PositionedShape` to determine which categories are present, grows the SVG `viewBox` height by exactly `(legend gap + rows × row height + bottom pad)`, keeps the diagram in place at the top, and emits the legend in a reserved bottom-left band. **Nothing is added to `PositionedShape` or `kul-layout`** — the legend is a render-time concern, not a layout concern, and the layout crate is unaware it exists. `kul-cli`'s `export --format=svg` is the one in-tree consumer that opts in: `render(&positioned, &ThemeConfig::with_self_contained(true).with_legend(true))`.
- **Preview surface** — `editor/vscode/` owns the legend as webview chrome. A `<div id="kul-legend">` lives as a sibling of `#kul-controls` and `#root`, so the per-render `root.innerHTML = …` swap never wipes it. On each successful render the bootstrap walks the rendered SVG DOM with `querySelectorAll`, builds rows for the present categories, and shows the overlay; on `renderError` it hides. The lifecycle mirrors `#kul-controls`.

The two emitters share **no source code** — one is Rust string templating, the other is TypeScript DOM. They are kept in lockstep by a shared normative source (the canonical-pattern doc) and by tests on each side that pin the row order and label strings.

### Colour: reuse production classes + `data-*`, never redeclare

A swatch is a **miniature of the real glyph carrying the production class + `data-*` attributes**. The adoption swatch is `<path class="kul-edge" data-link-kind="adoption">`; the ghost swatch is a `<rect>` inside `<g class="kul-card" data-kind="ghost">`; a gender swatch is `<g class="kul-card" data-kind="canonical" data-gender="male">`; an ended-marriage swatch carries both `data-link-kind="marriage"` and `data-is-ended="true"`. The existing stylesheets — the baked `SELF_CONTAINED_STYLE` (CLI) and `preview-themes.css` + `preview.css` (preview) — paint them automatically through the same rules that paint the diagram. **Zero new colour tokens, zero hardcoded hex; colours track the active theme automatically.**

Only **size, stroke-width, and dash** are tuned for legibility at swatch scale (the production 8.75px marriage stroke fills nearly half a 22px row). Each stylesheet gains a small `.kul-legend …` / `.kul-legend-swatch …` size-override block that touches only those properties:

- A `.kul-legend-label` rule (CLI) / `--kul-legend-*` token family (preview) sets the label font size.
- A `.kul-legend …[data-link-kind="marriage"]` rule clamps the marriage stroke-width down to a swatch-scale block.

**Swatch colour is never overridden.** Adding a theme is unchanged: re-map the `--kul-*` tokens, and both diagram and legend re-theme together.

### Panel: a rounded background frames the rows

Both surfaces draw the legend inside a **panel** — a rounded background that visually groups the rows into one chrome block. The CLI bakes a `<rect class="kul-legend-bg">` as the first child of the legend group (so the rows render on top); the preview's HTML overlay already has its own `background-color` + `border-radius` on the `.kul-preview-legend` container. The panel is **a new structural element**, not a swatch override — it carries its own `--kul-legend-panel-bg` / `--kul-legend-panel-border` tokens, distinct from the swatch tokens. The "swatch colour never overridden" rule continues to hold; the panel rect is outside its scope.

### Dynamic, present-only rows

Rows appear only for categories actually present in this diagram. A diagram with no adoption edges renders no adoption row; a diagram with no `other`-gender person renders no `other` row. Each surface derives presence from its own source:

- The CLI walks `PositionedShape` (`shape.cards` for gender / ghost presence, `shape.edges` for the three edge kinds and the ended-marriage variant).
- The preview chrome runs `querySelector` against the rendered SVG DOM after each `innerHTML` swap (alongside `injectGhostBadges`), keyed on the same `data-*` attributes the row table declares.

Canonical row order (normative in [`canonical-ui-pattern.md`](../canonical-ui-pattern.md)):

1. **Male** — gender card-stroke tint
2. **Female** — gender card-stroke tint
3. **Other** — gender card-stroke tint
4. **Past record** — ghost (dashed border, faded fill)
5. **Birth** — solid edge
6. **Adoption** — dashed edge
7. **Marriage** — un-ended thick edge
8. **Ended marriage** — faded thick edge

Marriage (#7) and Ended marriage (#8) are independently dynamic: a diagram with only ended marriages shows just #8, since #7's "un-ended marriage" category is empty.

### Labels are hardcoded English

The label table is a hardcoded `&str` table inside `kul-svg::emit` (CLI) and a `LEGEND_ROWS` constant in `preview-html.ts` (preview). **There is no caller-supplied label seam.** Two reasons:

- The legend ships only on the opt-in path (`legend = true`). The default `ThemeConfig` and `with_self_contained(true)` alone keep emitting the same theme-agnostic SVG, byte-for-byte unchanged — so **no English ships on the always-emitted (preview / wasm / LSP) SVG**. i18n is therefore not a regression for any existing consumer.
- An i18n seam (a caller-supplied label table or an enum the consumer interprets) would commit the contract to a particular shape before any localisation requirement exists. When and if i18n is required, adding a `labels: LegendLabels` field to `ThemeConfig` and a parallel chrome path is a purely additive change — and one that does not freeze a wrong shape today.

### CLI baked path stays on the self-contained side of every line

The legend resolves colour from the surrounding stylesheet, so it is only *meaningful* with `self_contained = true` (the baked `<style>` is what paints the swatches). The emitter does not couple the two flags — `with_legend(true)` without `with_self_contained(true)` renders the legend's structure colour-less, leaving the consumer's CSS to paint it. The CLI export sets both: `ThemeConfig::with_self_contained(true).with_legend(true)`. **No new CLI flag** — `--no-legend` (and a preview show/hide toggle) are clean additive follow-ups.

The CLI legend stays on the self-contained side of every line the [ADR-0016 2026-05-30 amendment](./0016-visualization-pipeline-crate-boundaries.md#amendment-2026-05-30--one-neutral-default-theme-behind-the-self_contained-opt-in) drew: no new colour bake (the existing `--kul-*` token vocabulary is reused), no theme catalogue, no VSCode variables. The amendment's "one neutral default theme behind the `self_contained` opt-in" carves the seam; the legend rides it.

## Consequences

- **A new visual category lands in one place per surface, plus the normative doc.** Adding (say) "step-relation" would mean: extend the canonical-pattern doc's normative table, add a `LegendRow` variant to `kul-svg::emit`, add an entry to `LEGEND_ROWS` in `preview-html.ts`. No new colour tokens; the existing token vocabulary continues to paint everything.
- **Colour stays a single source of truth.** Changing the marriage edge colour means changing one `--kul-marriage-edge-stroke` value — the diagram and the legend's marriage swatch re-paint together. There is no swatch hex hiding in either implementation that could silently drift.
- **The two emitters are not abstracted into a shared module.** They share a *spec* (the doc) and a *contract* (the data-* attribute seam), not code. A shared Rust module would be useless to the TypeScript chrome; a shared declarative table compiled into both would couple two release cycles together for a one-screen feature. The cost of duplication is bounded: ~150 LOC each, both pinned by tests.
- **The legend is structurally invisible to the layout crate.** `kul-layout` does not know the legend exists. The viewBox growth happens inside `kul-svg::emit` from `LegendRow` constants alone; the diagram's `PositionedShape` is unmodified. A future layout algorithm change does not have to consider the legend.
- **The preview's always-emitted SVG is byte-unchanged.** `ThemeConfig::default()` and the LSP `kul/render` (which uses it) emit exactly what they did before this change. The legend is *added* via opt-in, never substituted.
- **A non-JS consumer of the preview-served SVG still has no legend.** The chrome legend is a webview overlay; a consumer that scrapes the LSP-served SVG and renders it in a non-webview context gets the bare diagram. That is the same consequence the structural/chrome line already accepted for the ghost `↺` badge.

## Anti-suggestions (do not re-propose)

### Architecture and ownership

- **"Make the legend a layout concern — add `PositionedLegend` to `PositionedShape`."** The legend is a labelled key over the canonical pattern's visual vocabulary, not a layout primitive. It has no structural relationship with cards or edges; it sits beside them. Adding a layout type for it would force `kul-layout` to know about emission-time concerns (label strings, swatch dimensions, theming) and couple every future layout algorithm to legend metrics. The legend is render-time only.
- **"Share a legend module between Rust and TypeScript via a code generator."** Two ~150-LOC emitters, one normative-doc spec, and a test on each side is cheaper to maintain than the build-time generator that keeps them in sync — and the generator would freeze the table shape, blocking the next additive change. The doc is the single source of truth; the two emitters conform.
- **"Build the legend in `kul-render` so it sits next to `RenderShape`."** Same problem as `PositionedShape`: the render shape is the *canonical pattern as data*, not surface chrome. A legend is a labelled key over the rendered visual; it has no place in the pattern's data form.

### Colour and theming

- **"Introduce `--kul-legend-card-stroke-male` (etc.) for swatch colours so the legend is independently themable."** That recreates the drift problem the swatch-class reuse solves: changing the diagram's male stroke without also changing the legend's leaves the two out of sync. The same per-gender stroke colour goes on the same `data-gender="male"` selector for both; one token, one source of truth.
- **"Hardcode swatch hex values inline so the legend works even when the consumer's CSS doesn't define the tokens."** That hardcodes colour into the emitter, in direct violation of [ADR-0016](./0016-visualization-pipeline-crate-boundaries.md)'s "theme-agnostic SVG" rule on the default path. The opt-in `legend = true` plus `self_contained = true` combo *already* covers the standalone-file case via the baked `<style>`. A consumer with a custom stylesheet gets the legend re-themed by their tokens.

### Labels and i18n

- **"Add a `LegendLabels` struct to `ThemeConfig` so consumers can supply their own strings."** A speculative i18n seam ahead of any localisation requirement. The legend ships only on the opt-in path; no English reaches the always-emitted SVG; adding the seam later is purely additive. The cost of a wrong i18n shape now exceeds the cost of adding the right one later.
- **"Switch the labels to glyphs / icons so the legend is language-free."** Glyphs are not self-evident for "Past record" or "Ended marriage" — the legend's job is to *explain* the conventions, not introduce a second visual vocabulary the reader must learn first. English is the v1 language of the spec and tooling.

### CLI ergonomics

- **"Add a `--no-legend` flag to `kul export`."** A clean additive follow-up — once the default-on behaviour has a real consumer asking to turn it off. Shipping the flag now would freeze the contract before the use case exists.
- **"Bake the legend into the preview-served SVG too, so a downstream non-webview consumer gets it."** That re-emits English into the always-emitted SVG (`ThemeConfig::default()` would no longer be byte-unchanged) and couples every other surface — wasm `renderSvg`, the LSP `kul/render` over the wire — to a particular surface's chrome decisions. The chrome line is firm: a surface that wants a legend opts in, or supplies its own overlay.

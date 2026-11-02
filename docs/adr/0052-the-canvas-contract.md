# ADR 0052 — The canvas contract: marks and camera survive re-renders, two-way linking, theming through app tokens

**Status:** Accepted
**Date:** 2026-11-02
**Deciders:** owner

Supersedes [ADR-0036](./0036-two-tier-theming-and-the-accessibility-non-goal.md) (its token mapping), [ADR-0038](./0038-tier-1-register-overlay-stack-and-the-reserved-carve-out.md), [ADR-0042](./0042-the-selection-seam-and-occlusion-aware-centring.md) (its centring rule) and [ADR-0046](./0046-the-mode-boundarys-render-paths-and-what-the-retired-documents-leave-behind.md). Amends [ADR-0016](./0016-visualization-pipeline-crate-boundaries.md) and [ADR-0022](./0022-svg-legend.md) (the chrome is the app's). Issue [#351](https://github.com/YashBhalodi/kul/issues/351).

## Context

The canvas draws the engine's SVG under the editor window and re-draws it after every edit. A reader who is looking at one person and fixes a typo two files away should not lose the person, the zoom or the place in the tree. The tree's chrome — pan and zoom, selection, the legend, the ghost badge — is the surface's by the structural/chrome line (ADR-0016), and the surface is now the app, with the app's own design tokens in two themes.

## Decision

### The SVG is the contract; the canvas reads only that

Each analysis that can draw the project carries the theme-agnostic SVG (`analyze().svg`, ADR-0049). The canvas (`apps/web/src/app/canvas/`) swaps it into the page under a CSS-transform camera and reads only the `kul-svg` contract: the `kul-*` classes and the `data-*` attributes (`data-person-id`, `data-kind`, `data-ghost-reason`, …; ADR-0016, ADR-0021). An analysis with errors carries no SVG, so the canvas keeps the last tree, dimmed, with a banner that opens the problems strip.

The canvas's decisions are pure modules in `apps/web/src/canvas/` (camera maths, reveal, selection, details model, legend rows), tested in Node.

### Selection and camera survive re-renders

- **Marks are re-applied by id after every swap.** The canvas holds two marks — the **selection** (the entity the reader chose) and the **linked entity** (the one under the editor's cursor) — each an `EntityRef { kind, id }`. After a swap, `reapply` keeps a mark only if the new tree still draws that entity. An edit therefore ends a selection only by deleting what it selected. A stale tree (errors) keeps its marks.
- **The camera is the tab's own view.** It fits the first tree that draws someone (never enlarging past 100%), then stays put across edits. It is saved with the project's window state (debounced) and restored on open; a stored camera wins over the first fit. Another tab's later camera writes are not re-read.
- Details while errors exist: `details(id)` answers nothing, so the panel keeps its last answer; a new selection on a stale tree says to fix the errors first.

This reverses [ADR-0046](./0046-the-mode-boundarys-render-paths-and-what-the-retired-documents-leave-behind.md)'s "an edit ends query mode": that rule served a read-only query surface whose answers were computed from a snapshot. The canvas computes nothing it cannot recompute from the new tree, so keeping the reader's place costs nothing and is what an author editing beside the tree needs.

### Two-way linking

- **Canvas → editor.** Clicking a card or a marriage line (an invisible 18 px `kul-hit` path laid along each line) selects it and asks `locate(id)`; the editor opens that file and puts the cursor on the declaration **without taking focus**. "Go to source" in the details panel does the same and focuses the editor. A ghost's return badge jumps to the person's own card.
- **Editor → canvas.** The editor publishes its cursor; 150 ms after it rests, the canvas asks `entityAt(file, offset)` (latest request wins) and marks the answer as the linked entity. An `undefined` answer clears the link. The canvas reveals the linked entity only when it changes.

### Reveal moves the least distance; jumps centre

Revealing an entity (a selection or a new linked entity that is off screen or hidden) moves the camera the **least distance** that brings it inside the **visible region**: the widest full-height strip no occluder covers — the editor window unless minimized, and the details panel — kept 24 px clear of each occluder, with the revealed box 48 px inside the canvas's edges. A card touching an occluder counts as hidden. The occluders' boxes come from the same layout functions that place them. A ghost badge's jump is navigation, so it **centres** the card in that region. This replaces [ADR-0042](./0042-the-selection-seam-and-occlusion-aware-centring.md)'s centre-on-every-reveal, which, with the editor as a second occluder, threw the tree around on every selection and cursor move.

### The details panel is an `aside`

The details panel is a side panel along the canvas's right edge, built from `details(id)` through a pure model (`apps/web/src/canvas/details.ts`), never by reading the drawing. It is an `aside` (a complementary landmark), not shadcn's Sheet or a Dialog: those are modal — they move focus into themselves, trap it, and make the rest of the page inert — and the panel must never take the keyboard from the editor or the canvas, nor block either while it is open. It looks like a sheet (right edge, border, shadow, slide-in) without behaving like one, and it sits beside a right-docked editor or under a floating one.

### Theming: app tokens mapped onto `kul-svg` classes

- The app paints the agnostic SVG with `.kul-tree` rules in `apps/web/src/styles.css`, all from tokens in `apps/web/src/styles/tokens.css`, in `:root` (light) and `.dark`: `--tree-hue-{male,female,other,birth,adoption,marriage,past}`, `--tree-selection(-fill)`, `--tree-linked(-fill)`. Nothing else in `src/` names a colour (`scripts/lint-colors.mjs`).
- A test reads the palette baked into a real `renderSvg("fileExport")` and holds the hue tokens equal to it in both themes, so the canvas and the downloaded SVG show the same categories in the same colours.
- **The downloaded SVG** is the CLI's `kul export --format=svg` output: self-contained, light, with the legend baked in (ADR-0022). Its `<style>` keeps two layers — **roles** (colour roles, the diagram hues, the diagram font) and **per-site aliases** that point at a role — and every name and value in it is part of the export, which stays byte-stable.
- **`--kul-preview-bg` is a kept name.** The self-contained SVG's background alias is called `--kul-preview-bg`, a name from the editor preview that first defined the token layer. Every exported SVG contains it, and the export must stay byte-identical (it is a contract: files people have downloaded and tools that read them), so it is not renamed. Nothing in the app reads it; it lives only inside the exported `<style>`.
- The canvas's legend is chrome: a collapsible panel listing the categories the current tree draws, its rows and labels held equal to the baked legend by a browser test.

## Consequences

- The tree can be re-drawn on every edit without the reader noticing anything but the change.
- The canvas never infers kinship from the picture; everything it says comes from the engine.
- Measured (CI medians): keystroke to canvas swap on `examples/09` is 63–89 ms typing and 161–179 ms deleting across Chromium, Firefox and WebKit (ceiling 300 ms). A 2,000-person tree's DOM swap costs 196–625 ms of main thread on CI; making the swap incremental (keyed by `data-person-id`) or culling off-screen cards is follow-up [#358](https://github.com/YashBhalodi/kul/issues/358).

## Anti-suggestions (do not re-propose)

- **"Clear the selection on every edit."** Re-applying by id is free and keeps the reader's place.
- **"Centre on every reveal."** It moves the whole tree for a small correction.
- **"Make the details panel a sheet or dialog."** Focus and modality are wrong for a panel beside a live editor.
- **"Theme the SVG in Rust."** Themes are the surface's; the baked theme exists only for the self-contained download.
- **"Edit from the canvas."** The source is the document; the canvas reads it.

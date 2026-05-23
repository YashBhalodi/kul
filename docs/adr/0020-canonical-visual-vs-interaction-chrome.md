# ADR 0020 — Canonical visual vs. interaction chrome

**Status:** Accepted
**Date:** 2026-05-23
**Deciders:** owner

## Context

The KulLang toolchain is starting to grow surface renderers. The VSCode preview lands in this epic; a web app, a native preview, a CLI export, and a future mobile app are all on the roadmap. Each surface has its own affordances — VSCode has light/dark/high-contrast themes and editor-cursor sync; a web app has brand colours and link-out URLs; a CLI export has print-mode and self-contained SVGs.

This raises a recurring design question: **what is part of the canonical visual, and what is part of a particular surface's UI?** Answered case-by-case, the boundary drifts. Theming might end up baked into `kul-svg` for the VSCode preview's convenience; click-to-jump might end up baked into `kul-svg` for the editor's convenience; pan/zoom might end up baked into `kul-svg` for the web app's convenience. Each individual case looks innocuous; the cumulative result is a "render" crate that knows about every surface that ever consumed it.

The PRD's three crates ([ADR-0016](./0016-kul-render-crate-boundary.md), [ADR-0018](./0018-kul-layout-crate-boundary.md), [ADR-0019](./0019-kul-svg-crate-boundary.md)) each made one local decision pointing in the same direction. Pulling the principle out as a standalone ADR makes it citeable for the cases that haven't come up yet.

## Decision

**The canonical visual is theme- and interaction-agnostic. Theming and interactivity are interaction chrome, owned by each consuming surface.**

Concretely:

- **`kul-render` emits structural data only.** `RenderShape` carries components, marriage branches, card slots, ghosts, P6 nested sub-trees. It does not carry positions, themes, or interaction state. [ADR-0017](./0017-render-shape-schema-and-versioning.md).
- **`kul-layout` emits structural positions only.** `PositionedShape` carries absolute pixel coordinates for cards, bars, and edge polylines. It does not carry colours, fonts, hover/click metadata, or interactivity state. [ADR-0018](./0018-kul-layout-crate-boundary.md).
- **`kul-svg` emits theme-agnostic SVG.** The output uses semantic CSS classes (`kul-card`, `kul-card--canonical`, `kul-card--ghost`, `kul-bar`, `kul-bar--ended`, `kul-edge`, `kul-edge--birth`, `kul-edge--adoption`, `kul-label-name`, `kul-ghost-badge`). No inline colours. No event handlers. No JavaScript. [ADR-0019](./0019-kul-svg-crate-boundary.md).
- **Surfaces own their chrome.** The VSCode preview ships `editor/vscode/media/preview.css` mapping the kul-svg classes to VSCode CSS variables (light/dark/high-contrast auto-tracking is a CSS concern, not a Rust release). A future web app ships its own stylesheet mapping the same classes to its brand palette. A future CLI export wraps a default stylesheet inside the emitted SVG to make it self-contained.

**Theming, interactivity, and surface affordances are chrome:**

- Light/dark/high-contrast theme: chrome (CSS variable → CSS rule).
- Pan / zoom: chrome (webview JS library, e.g. `svg-pan-zoom`).
- Click-to-jump-to-source: chrome (webview ↔ extension message protocol). The data path that *enables* it (source spans on `PositionedShape` elements) is structural and lives in `kul-layout`/`kul-svg`; the click handler that consumes it is chrome.
- Hover effects: chrome (pure CSS, `:hover` selectors over the existing class vocabulary).
- Editor-cursor → highlight-matching-card: chrome (bidirectional protocol; matching-card lookup uses the structural `personId` already in the SVG).
- Selection of a card → "Reveal in editor" UI: chrome.

**The canonical visual is structural:**

- Card position relative to other cards and bars (P14's "natural hierarchy"): structural.
- Card kind (canonical vs ghost): structural.
- Edge kind (birth vs adoption — solid vs dashed per P5): structural.
- Bar position between adjacent spouses: structural.
- Generation row: structural.
- Class vocabulary on emitted SVG (so chrome can hook in): structural seam, owned by `kul-svg`.

The line is **whether the choice changes how the kinship reads**. A theme change does not change "who is married to whom" or "who is adopted vs born." A pan/zoom does not change "Carol's parents are at this bar." A click-to-jump moves the editor cursor — it does not change the rendered family tree. By contrast, a wrong card position would mis-represent kinship; a wrong edge style would conflate adoption and birth; a missing ghost would lose a load-bearing past structural fact.

## Consequences

- **One canonical visual across every surface.** A `.kul` document rendered in VSCode, in a web app, in a native preview, and as a CLI export shows the same families in the same arrangement with the same kinship relationships visible. Surface differences live in colour, density, and interactivity — not in *what the diagram says*.
- **Surfaces innovate in chrome, not in pattern.** Editor-cursor sync, pan/zoom, link-out URLs, print-mode CSS, mobile touch gestures — all of these are surface-local. Adding one to the VSCode preview does not require shipping a kul-svg release or co-ordinating with the web app.
- **Theming is a CSS concern.** The CSS-class vocabulary (`kul-card`, `kul-bar`, etc.) is the seam every theming surface hooks into. Adding a theme is a stylesheet change; adding a new card variant (e.g. `kul-card--multi-adopted`) is a kul-svg release that every theming surface picks up on next update — but old themes keep working because the additive variant degrades to the base class.
- **Future cross-cutting decisions cite this ADR.** When a future contributor asks "should X live in `kul-svg` or in the VSCode webview?", the answer phrases as: does X change how the kinship reads? If yes, it's structural — into Rust. If no, it's chrome — into the surface. This ADR is the canonical reference for that decision.

## Anti-suggestions (do not re-propose)

- **"Add a theme parameter to `kul-svg::render`."** Moves the theme catalogue into Rust. Every new theme is a Rust release; every consumer that wants a custom theme has to fork or wait. The CSS-class seam already gives consumers full theming control — adding the parameter would only duplicate that surface.
- **"Bake the VSCode CSS variables into `kul-svg` directly."** Couples the renderer to one surface's variable namespace (`var(--vscode-editor-foreground)`). The web app's CSS variables are not `--vscode-*`; the CLI export's defaults aren't either. Per-surface stylesheets are the right place for per-surface variable bindings.
- **"Put pan/zoom into `kul-svg::render` via an opt-in flag."** Pan/zoom is JS at runtime, not SVG structure. The right home is a tiny JS library inside the webview (or the web app, or the native preview). Bundling it into kul-svg would force every surface that doesn't want pan/zoom (e.g. a print-mode CLI export) to strip it back out.
- **"Add click handlers to the SVG inside `kul-svg`."** Same shape — event handling is runtime, not structure. Per-surface message protocols and event routing belong in per-surface code. The structural enabler (source spans on elements) lives in Rust; the click handler that consumes it lives in the surface.
- **"Make `RenderShape` carry positions so the SVG can be 'fully self-describing.'"** This is the inverse of the ADR-0018 anti-suggestion. `RenderShape` is structural in a different axis (the canonical pattern's hierarchy); positions are a different rate of change (level-of-detail, virtualisation, alternative algorithms per P14). Different rates of change → different crates.

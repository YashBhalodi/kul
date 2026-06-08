# Changelog

All notable changes to the **KulLang** VSCode extension are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.4.2] — 2026-06-09

### Added

- **New command `Kul: Export SVG`** (`kul.export.svg`) — emits a self-contained SVG to a user-chosen file. Defaults the save dialog to `<project-dir-basename>.svg` in the project directory; bytes are byte-identical to `kul export --format=svg`. On failure, surfaces a counted warning toast pointing at the Problems panel — no save dialog, no file (#217).
- **Ghost `↺` badge becomes a clickable jump-to-canonical action.** Clicking a ghost card's `↺` badge pans the preview viewport to the person's canonical card with a brief border-glow pulse on arrival. Strictly viewport navigation; the editor cursor is not moved. The badge gains a native `Jump to canonical card` tooltip and a 24×24 hit target (#211).

### Changed

- **Preview no longer flickers between diagram and error banner during live edits.** A partial / invalid intermediate state used to wipe the rendered SVG and replace it with a full-pane banner, yanking away the diagram the author was reasoning about. The last successful render now stays mounted (with its pan/zoom state preserved) and dims via a `kul-render-stale` overlay so the staleness is visible. Errors surface through a new red triangle icon in the bottom-left control panel — a count badge shows the active error count, clicking expands a popover listing each diagnostic, and clicking a row jumps the editor to the diagnostic's source range. Warnings continue to live in the Problems pane; the popover is errors-only (#203).
- The `marriage` snippet no longer pre-fills `start:`. After #200 made marriage `start:` optional in the spec and validator (R03 retired), the snippet pushed authors toward writing a field that's now optional. The body now expands to `marriage <id> <spouse1> <spouse2>`; authors who want a start date can type `start:` themselves, and the LSP's field completion still offers it after the third positional argument (#202).
- **Preview chrome moved into a shared `@kullang/preview` workspace package.** The webview HTML shell, bootstrap, tooltip, legend, pan/zoom controls, error popover, and theme tokens now live in `packages/preview/` so a future webapp can reuse the same chrome against its own `HostAdapter`. User-visible preview behaviour is unchanged (#220).

### Fixed

- **Preview re-renders when its tab regains focus.** VSCode destroys a webview's DOM and JS context when its tab moves to the background; on restore, the panel used to stay blank until the next save. The extension now subscribes to `onDidChangeViewState` and re-renders on every hidden → visible transition (#206).
- **Language server no longer crashes on a render edge whose marriage has no positioned anchor.** Two layout invariants that previously panicked the LSP — a polygamy hub with a render edge but no card-emitting children, and a render edge whose marriage anchor went missing in a future regression — now degrade to a silent skip in release builds (debug builds still surface the violation) (#208, #209).

## [0.4.1] — 2026-06-06

### Fixed

- The outline pane no longer goes silent mid-edit. While typing a person's name, the buffer transiently contains `name:""`, and VSCode's language-client rejects any `DocumentSymbol` with an empty `name`. The language server now treats an empty or whitespace-only `name:` literal as "no usable name" and falls back to the person id (and per-spouse for marriage labels), so the outline keeps refreshing on every keystroke (#199).

## [0.3.4] — 2026-05-30

### Added

- **Click a card or marriage bar to jump to its source.** Clicking a person card or a marriage bar in the preview opens the declaration and selects its id token (cursor swaps to a pointer to surface the affordance). Birth/adoption edges stay inert — they keep the pan cursor and have no click action (#135).
- **Editor cursor highlights the matching preview element.** Moving the cursor onto a `person` or `marriage` declaration (or a reference to one) wraps the matching card / marriage bar in a magenta selection ring — a hue reserved for selection so it stays equally prominent across light, dark, and high-contrast themes. The viewport eases over to centre the match; a new highlight cancels any in-flight pan and re-eases (#137).
- **Hover tooltip surfaces an entity's details.** Hovering a person card or a marriage / adoption edge opens a tooltip after a hover-intent delay: a typed header (person name, marriage spouses, or adoption child) plus a two-column field grid built from the entity's properties. Scales with the diagram, anchored at the cursor, viewport-clamped, dismissed on `mouseleave` / re-render / pan / zoom (#156).
- **Hover affordance for clickable elements.** Person cards and marriage bars bump their stroke width on hover, reinforcing the click target while preserving the gender colour encoding (#136).
- **Keyboard pan/zoom for the preview viewport.** Arrow keys pan (smooth `requestAnimationFrame` motion), `+` / `=` zoom in, `-` zooms out, `0` resets. Modifier guards keep `Cmd+0` / `Ctrl+=` passing through to VSCode (#180).
- **Diagram legend.** A compact bottom-left reference card keys the diagram's visual vocabulary (card kinds and edge kinds), built from the rendered SVG so swatch colours can never drift from the diagram. Opt-in via a new `ⓘ` "Show legend" toggle in the control cluster — hidden by default to keep the canvas uncluttered. Companion to the CLI baked legend; both conform to the same normative key (#157).

### Changed

- The ghost ↺ badge is now drawn by the preview rather than baked into the SVG, since it's an element-marker that CSS cannot generate from a presentation attribute (#182).

## [0.3.3] — 2026-05-26

### Added

- The preview panel is now an interactive **pan/zoom** surface (vendored `svg-pan-zoom`): drag to pan, wheel to zoom, and on-screen controls for zoom-in / zoom-out / reset-view. The first render fits-and-centers; subsequent debounced re-renders while editing preserve the current viewport, and the reset control returns to fit-and-center on demand (#134).
- The default preview theme now tints each person card's outline by gender (male / female / other), for canonical and ghost cards alike — opting into the gender data-* seam the renderer already emits (#134).

## [0.3.2] — 2026-05-26

### Added

- The preview panel now **colour-codes element kinds** from your active VSCode theme — cards blue, birth edges green, adoption edges orange, marriage edges purple (atop their existing line styles) — so what an element *is* reads by hue as well as by shape. A theme without a charts palette degrades gracefully to the previous monochrome appearance (#179).

## [0.3.0] — 2026-05-26

### Added

- New command **`Kul: Show Preview`** (`kul.preview.show`) opens a canonical-visual preview panel beside the active editor. Renders the active `.kul` document as a family tree in theme-tracking SVG (light, dark, high-contrast). Debounced re-render (~300 ms) when the document or any sibling `.kul` in the same project changes; an error banner for documents with diagnostics. Backed by the language server's new `kul/render` request, so the preview matches the canonical visual every other Kul surface produces (#125).

### Changed

- The two export commands drop their redundant `Kul:` title prefix — the command palette already groups them under the **Kul** category.

## [0.1.3] — 2026-05-07

The extension is now published to the **VS Code Marketplace** in addition to Open VSX. Upstream VSCode users can install with `code --install-extension YashBhalodi.kul` directly, instead of sideloading a `.vsix` from the GitHub Release. Open-VSX-consuming editors (VSCodium, Cursor, Windsurf, Theia/Che, Gitpod) continue to install from Open VSX as before. No extension behavior changes (#62).

## [0.1.2] — 2026-05-07

CI lockstep bump. No extension behavior changes.

## [0.1.1] — 2026-05-07

Hotfix for marketplace install. `v0.1.0` shipped a single un-targeted `.vsix`; Cursor's marketplace install path treats untagged extensions as platform-independent and strips bundled platform binaries on install, leaving the extension with no language server. Fixed by publishing four `--target`-tagged `.vsix` files (one per platform) and chmoding the bundled binary on activation as a belt-and-suspenders against vsce's zip layer dropping the execute bit (#59).

## [0.1.0] — 2026-05-07

First public release. The extension ships with a bundled `kul-lsp` language server — no setup required beyond installing the extension.

### Editing

- File association and file-tree icon for `.kul`.
- Syntax highlighting for keywords, strings (with escapes), date literals (with `~` circa marker), field names, enum values (`male` / `female` / `other` / `divorce`), declared identifiers, and id references.
- Snippets for the common shapes: `kul`, `person`, `marriage`, `birth`, `adoption`.
- Line-comment toggling (`#`) and auto-closing string quotes.
- Format-on-save by default — `.kul` files are canonicalized whenever you save (override per workspace if you prefer manual formatting).

### Language-server features

The bundled language server gives you these features automatically — no `kul.serverPath` configuration needed.

- **Live diagnostics** — red squiggles under errors as you type, surfacing all 13 Kul validation rules (missing required fields, unresolved references, self-marriages, temporal contradictions, parenthood cycles, and more).
- **Hover panels** — markdown documentation on keywords, identifiers, field names, and references.
- **Go to definition** — `Cmd+Click` (or `F12`) on a person or marriage reference jumps to its declaration.
- **Find all references** — locate every use of a person or marriage id across the document.
- **Rename symbol** — rename a person or marriage id everywhere at once. Rejects collisions with existing ids and reserved keywords with a clear error.
- **Code actions** — quick fixes for missing-required-field (`KUL-R03`) and end-consistency (`KUL-R05`) diagnostics.
- **Completion** — context-aware suggestions for keywords, field names, enum values, and existing person / marriage ids.
- **Document outline** — persons, marriages, and their nested `birth` / `adoption` sub-statements in the outline view and breadcrumbs.
- **Semantic highlighting** — declaration-vs-reference distinction for ids, plus token-level coloring that follows your theme.
- **Document formatting** — the `Format Document` command and format-on-save both run the canonical `kul format` pass.

### Commands

- **Kul: Export to JSON** — projects the current document (including unsaved edits) to the canonical kinship-native JSON envelope and prompts for a save location. Schema is normative; see [spec §15](https://github.com/YashBhalodi/kul/tree/main/spec/16-export-schema.md).
- **Kul: Export to Cytoscape JSON** — the same data projected into the `nodes` / `edges` shape loadable by Cytoscape.js, Sigma.js, vis-network, and similar tools.

Both commands appear in the command palette only on `.kul` files. They surface a notification and point you at the Problems panel if the document has errors.

### Settings

- `kul.serverPath` — absolute path to a `kul-lsp` binary. Leave empty to use the bundled binary.
- `kul.trace.server` — `off` / `messages` / `verbose`. Controls LSP message tracing in the **Kul LSP** output channel.

# Changelog

All notable changes to the **KulLang** VSCode extension are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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

- **Kul: Export to JSON** — projects the current document (including unsaved edits) to the canonical kinship-native JSON envelope and prompts for a save location. Schema is normative; see [spec §15](https://github.com/YashBhalodi/kul/tree/main/spec/15-export-schema.md).
- **Kul: Export to Cytoscape JSON** — the same data projected into the `nodes` / `edges` shape loadable by Cytoscape.js, Sigma.js, vis-network, and similar tools.

Both commands appear in the command palette only on `.kul` files. They surface a notification and point you at the Problems panel if the document has errors.

### Settings

- `kul.serverPath` — absolute path to a `kul-lsp` binary. Leave empty to use the bundled binary.
- `kul.trace.server` — `off` / `messages` / `verbose`. Controls LSP message tracing in the **Kul LSP** output channel.

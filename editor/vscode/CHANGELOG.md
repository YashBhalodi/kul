# Changelog

All notable changes to the **KulaLang** VSCode extension are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — Unreleased

First public release. The extension ships with a bundled `kula-lsp` language server — no setup required beyond installing the extension.

### Editing

- File association and file-tree icon for `.kula`.
- Syntax highlighting for keywords, strings (with escapes), date literals (with `~` circa marker), field names, enum values (`male` / `female` / `other` / `divorce`), declared identifiers, and id references.
- Snippets for the common shapes: `kula`, `person`, `marriage`, `birth`, `adoption`.
- Line-comment toggling (`#`) and auto-closing string quotes.
- Format-on-save by default — `.kula` files are canonicalized whenever you save (override per workspace if you prefer manual formatting).

### Language-server features

The bundled language server gives you these features automatically — no `kula.serverPath` configuration needed.

- **Live diagnostics** — red squiggles under errors as you type, surfacing all 13 Kula validation rules (missing required fields, unresolved references, self-marriages, temporal contradictions, parenthood cycles, and more).
- **Hover panels** — markdown documentation on keywords, identifiers, field names, and references.
- **Go to definition** — `Cmd+Click` (or `F12`) on a person or marriage reference jumps to its declaration.
- **Find all references** — locate every use of a person or marriage id across the document.
- **Rename symbol** — rename a person or marriage id everywhere at once. Rejects collisions with existing ids and reserved keywords with a clear error.
- **Code actions** — quick fixes for missing-required-field (`KULA-R03`) and end-consistency (`KULA-R05`) diagnostics.
- **Completion** — context-aware suggestions for keywords, field names, enum values, and existing person / marriage ids.
- **Document outline** — persons, marriages, and their nested `birth` / `adoption` sub-statements in the outline view and breadcrumbs.
- **Semantic highlighting** — declaration-vs-reference distinction for ids, plus token-level coloring that follows your theme.
- **Document formatting** — the `Format Document` command and format-on-save both run the canonical `kula format` pass.

### Commands

- **Kula: Export to JSON** — projects the current document (including unsaved edits) to the canonical kinship-native JSON envelope and prompts for a save location. Schema is normative; see [spec §15](https://github.com/YashBhalodi/kulalang/tree/main/spec/15-export-schema.md).
- **Kula: Export to Cytoscape JSON** — the same data projected into the `nodes` / `edges` shape loadable by Cytoscape.js, Sigma.js, vis-network, and similar tools.

Both commands appear in the command palette only on `.kula` files. They surface a notification and point you at the Problems panel if the document has errors.

### Settings

- `kula.serverPath` — absolute path to a `kula-lsp` binary. Leave empty to use the bundled binary.
- `kula.trace.server` — `off` / `messages` / `verbose`. Controls LSP message tracing in the **Kula LSP** output channel.

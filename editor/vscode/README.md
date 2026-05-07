# KulLang for VSCode

Syntax highlighting and editor support for [Kul](https://github.com/YashBhalodi/kul) (`.kul`) kinship-description files.

![Highlighting preview](images/screenshot.png)

This extension lives inside the [kul](https://github.com/YashBhalodi/kul) repo. See the [language specification](https://github.com/YashBhalodi/kul/tree/main/spec) and [examples](https://github.com/YashBhalodi/kul/tree/main/examples).

## Features

- File association and file-tree icon for `.kul`
- Line-comment toggling (`#`) and auto-closing string quotes
- Syntax highlighting for keywords, strings (with escapes), date literals (with `~` circa marker), field names, enum values (`male`/`female`/`other`/`divorce`), declared identifiers, and id references
- Snippets for the common shapes: `kul`, `person`, `marriage`, `birth`, `adoption`
- Format-on-save: `.kul` files are canonicalized via `kul format` whenever you save (override per workspace if you prefer manual formatting)
- **Language-server integration** when `kul-lsp` is available (pointed at via `kul.serverPath` for development; bundled in the published Open VSX release): live diagnostics, hover panels, go-to-definition, basic completion, document outline, find-references, rename, code actions, and document formatting
- **Export commands** — `Kul: Export to JSON` and `Kul: Export to Cytoscape JSON` (run from the command palette on any `.kul` file): projects the current document — *including unsaved edits* — through the language server's `kul/export` request and prompts for a save location. The JSON form is the canonical kinship-native shape ([spec §15](https://github.com/YashBhalodi/kul/tree/main/spec/15-export-schema.md)); the Cytoscape form is a `nodes`/`edges` projection loadable into Cytoscape.js, Sigma.js, vis-network, etc. If the document has errors the command surfaces a notification and points you at the Problems panel

## Settings

- `kul.serverPath` — absolute path to a `kul-lsp` binary. When set, overrides the bundled binary; useful for pointing at a locally-built `target/debug/kul-lsp`. Leave empty to use the bundled binary (when the extension ships with one).
- `kul.trace.server` — `off` / `messages` / `verbose`. Enables LSP message tracing in the `Kul LSP` output channel.

## Local development

### Install for development

A single command builds the language server, packages the extension, and installs the `.vsix` into your system VSCode:

```sh
just vscode            # debug LSP build (fast)
just vscode release    # optimized LSP build
```

Re-run after every code change. Reload the VSCode window once it finishes (`Cmd+Shift+P` → `Developer: Reload Window`) to pick up the new bundle. The recipe is idempotent and uses `--force` to overwrite the previously-installed extension.

**One-time setup:**

```sh
cd editor/vscode && npm install
```

**One-time `kul.serverPath`:** point at the locally-built LSP so the extension uses your code, not a bundled binary. Open Settings (`Cmd+,`) → search `kul.serverPath` → paste the absolute path printed by `just vscode`. To switch between debug and release, edit this setting and reload the window.

**Uninstall:**

```sh
code --uninstall-extension YashBhalodi.kul
```

The generated `*.vsix` is gitignored.

### Iterate inside an Extension Development Host (no install)

For TypeScript-only changes in `src/extension.ts`, opening `editor/vscode/` in VSCode and pressing `F5` launches an Extension Development Host with the extension loaded. Faster than `just vscode` because it skips packaging and global install — but only the dev-host window sees the extension, and language-server changes still require `cargo build -p kul-lsp` and a reload.

### Build a fully-bundled `.vsix` (production-style)

The published `.vsix` (Open VSX, GitHub Releases) bundles pre-built `kul-lsp` binaries for all four target platforms (`linux-x64`, `darwin-x64`, `darwin-arm64`, `win32-x64`); end users don't need `kul.serverPath`. To produce that artifact locally:

```sh
cd editor/vscode
npm install
npm run package:bundled    # downloads binaries from the v<version> GitHub Release, then vsce package
code --install-extension kul-<version>.vsix --force
```

Requires a published GitHub Release at tag `v<version>` (the release pipeline produces all binaries under one tag). For day-to-day development, `just vscode` is faster — only this flow is needed when validating the bundled-binary auto-locator.

Override with `LSP_VERSION=<x.y.z> npm run fetch-server` if you need a release other than the one that matches `package.json`.

### Tracing LSP traffic

To debug the language server itself, set `kul.trace.server` to `messages` or `verbose` and watch the `Kul LSP` output channel (`View → Output → Kul LSP`).

## Requirements

VSCode 1.85 or later. The extension targets Node 18+ via the bundled extension host.

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

### Option A — Dev-host (fastest iteration)

1. Open this directory (`editor/vscode/`) in VSCode.
2. Run `npm install` once.
3. Press `F5` to launch an Extension Development Host window with the extension loaded. The pre-launch task compiles the TypeScript bundle.
4. In the dev host, open any file from the repo's [`examples/`](https://github.com/YashBhalodi/kul/tree/main/examples) directory.

Closing the dev-host window unloads the extension. Best for iterating on the activation script — edits to `src/extension.ts` take effect on relaunch.

### Option B — Install a local `.vsix` into your real VSCode

Use this when you want the extension active across all your VSCode windows (not just the dev host) without publishing to Open VSX.

**One-time setup:**

```sh
npm i -g @vscode/vsce
cd editor/vscode
npm install
```

**Package and install:**

```sh
cd editor/vscode
npm run package                                       # produces kul-<version>.vsix
code --install-extension kul-<version>.vsix      # use --force to overwrite an existing install
```

`npm run package` invokes `vsce package`, which runs the `vscode:prepublish` script first (typecheck + esbuild bundle).

Reload VSCode (`Cmd+Shift+P` → `Developer: Reload Window`) for the change to take effect.

**Re-package after edits:**

```sh
npm run package && code --install-extension kul-<version>.vsix --force
```

**Uninstall:**

```sh
code --uninstall-extension YashBhalodi.kul
```

The generated `*.vsix` file is gitignored.

### Option C — Test the language server locally

The extension's LSP client looks for `kul-lsp` first via the `kul.serverPath` setting and then falls back to a bundled binary. The default `npm run package` produces an **unbundled** `.vsix` (fast, no network) — perfect for local-dev install. For development you'll want to point at your locally-built binary:

1. Build the language server from the repo root:

   ```sh
   cargo build -p kul-lsp
   ```

2. Note the absolute path of the produced binary (`<repo>/target/debug/kul-lsp`).

3. Install the extension via Option A or Option B.

4. In VSCode, open Settings (`Cmd+,`) → search `kul.serverPath` → paste the absolute path. (Or edit `settings.json` directly with `"kul.serverPath": "/absolute/path/to/target/debug/kul-lsp"`.)

5. Reload the window (`Cmd+Shift+P` → `Developer: Reload Window`).

6. Open any `examples/*.kul` file. You should see:

   - Red squiggles under errors as you type (live diagnostics)
   - Hover panels on keywords, identifiers, field names, and references
   - Cmd+click on a person ref or marriage ref jumps to the declaration
   - Autocomplete for keywords, field names, and enum values

To debug the language server itself, set `kul.trace.server` to `messages` or `verbose` and watch the `Kul LSP` output channel (`View → Output → Kul LSP`).

### Option D — Build a fully-bundled `.vsix` (production-style)

Use this to package an extension that ships pre-built `kul-lsp` binaries for all four target platforms (`linux-x64`, `darwin-x64`, `darwin-arm64`, `win32-x64`) — the form that goes to Open VSX (and that ships as `kul-<version>.vsix` on every GitHub Release).

This requires a published GitHub Release at tag `v<version>` (the unified release pipeline produces all binaries under one tag). For day-to-day development you don't need this — Option C with `kul.serverPath` is faster.

```sh
cd editor/vscode
npm install
npm run package:bundled                                # downloads binaries from the v<version> release, then vsce package
code --install-extension kul-<version>.vsix --force
```

End users installing the bundled `.vsix` don't need to set `kul.serverPath` — the extension auto-locates the right platform binary.

Override with `LSP_VERSION=<x.y.z> npm run fetch-server` if you need a release other than the one that matches `package.json`.

## Requirements

VSCode 1.85 or later. The extension targets Node 18+ via the bundled extension host.

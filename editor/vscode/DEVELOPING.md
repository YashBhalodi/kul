# Developing the KulLang VSCode extension

Local-development guide for the extension at `editor/vscode/`. Consumer-facing documentation lives in [`README.md`](./README.md); release-pipeline details live in [`docs/release.md`](../../docs/release.md).

## Install for development

A single command builds the language server, packages the extension, installs the `.vsix` into VSCode, and points `kul.serverPath` at the just-built binary:

```sh
just vscode            # debug LSP build (fast)
just vscode release    # optimized LSP build
```

Re-run after every code change. Reload the VSCode window once it finishes (`Cmd+Shift+P` → `Developer: Reload Window`) to pick up the new bundle. The recipe is idempotent — it `--force`-overwrites the installed extension and surgically updates `kul.serverPath` in both Cursor's and VSCode's user-level `settings.json` (whichever exist). Switching between debug and release just means re-running with the other mode.

**One-time setup:**

```sh
cd editor/vscode && npm install
```

**Uninstall:**

```sh
code --uninstall-extension YashBhalodi.kul
```

The generated `*.vsix` is gitignored.

## Iterate inside an Extension Development Host (no install)

For TypeScript-only changes in `src/extension.ts`, opening `editor/vscode/` in VSCode and pressing `F5` launches an Extension Development Host with the extension loaded. Faster than `just vscode` because it skips packaging and global install — but only the dev-host window sees the extension, and language-server changes still require `cargo build -p kul-lsp` and a reload.

## Build a fully-bundled `.vsix` (production-style)

The published `.vsix` files (Open VSX, GitHub Releases) are produced as four platform-specific bundles by `.github/workflows/release.yml`. To produce a single-platform bundle locally:

```sh
cd editor/vscode
npm install
npm run package:bundled    # downloads binaries from the v<version> GitHub Release, then vsce package
code --install-extension kul-<version>.vsix --force
```

Requires a published GitHub Release at tag `v<version>` (the release pipeline produces all binaries under one tag). For day-to-day development, `just vscode` is faster — only this flow is needed when validating the bundled-binary auto-locator.

Override with `LSP_VERSION=<x.y.z> npm run fetch-server` if you need a release other than the one that matches `package.json`.

## Tracing LSP traffic

To debug the language server itself, set `kul.trace.server` to `messages` or `verbose` and watch the **Kul LSP** output channel (`View → Output → Kul LSP`).

# KulaLang for VSCode

Syntax highlighting and editor support for [Kula](https://github.com/YashBhalodi/kulalang) (`.kula`) kinship-description files.

This extension lives inside the [kulalang](https://github.com/YashBhalodi/kulalang) repo. See the repo root for the language specification, examples, and roadmap.

## Features (v0.0.1)

- File association and file-tree icon for `.kula`
- Line-comment toggling (`#`) and auto-closing string quotes
- Syntax highlighting for keywords, strings (with escapes), date literals (with `~` circa marker), field names, enum values (`male`/`female`/`other`/`divorce`), declared identifiers, and id references
- Snippets for the common shapes: `kula`, `person`, `marriage`, `birth`, `adoption`

Language-server support (live diagnostics, hover, go-to-definition, completion) lands in subsequent phases — see the [roadmap](https://github.com/YashBhalodi/kulalang/tree/main/docs/roadmap).

## Local development

### Option A — Dev-host (fastest iteration)

1. Open this directory (`editor/vscode/`) in VSCode.
2. Press `F5` to launch an Extension Development Host window with the extension loaded.
3. In the dev host, open any file from the repo's [`examples/`](https://github.com/YashBhalodi/kulalang/tree/main/examples) directory.

Closing the dev-host window unloads the extension. Best for iterating on the grammar or snippets — edits to the source files take effect when you re-launch.

### Option B — Install a local `.vsix` into your real VSCode

Use this when you want the extension active across all your VSCode windows (not just the dev host) without publishing to the marketplace.

**One-time setup:**

```sh
npm i -g @vscode/vsce
```

**Package and install:**

```sh
cd editor/vscode
vsce package                                          # produces kulalang-<version>.vsix
code --install-extension kulalang-<version>.vsix      # use --force to overwrite an existing install
```

Reload VSCode (`Cmd+Shift+P` → `Developer: Reload Window`) for the change to take effect.

**Re-package after edits:**

```sh
vsce package && code --install-extension kulalang-<version>.vsix --force
```

**Uninstall:**

```sh
code --uninstall-extension YashBhalodi.kulalang
```

The generated `*.vsix` file is gitignored at the repo root.

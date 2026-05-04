# KulaLang for VSCode

Syntax highlighting and editor support for [Kula](https://github.com/YashBhalodi/kulalang) (`.kula`) kinship-description files.

This extension lives inside the [kulalang](https://github.com/YashBhalodi/kulalang) repo. See the repo root for the language specification, examples, and roadmap.

## Features (v0.0.1)

- File association for `.kula`
- Line-comment toggling (`#`)
- Auto-closing string quotes
- Syntax highlighting for top-level keywords (`kula`, `person`, `marriage`, `birth`, `adoption`)

More highlighting (string literals, dates, fields, enum values, identifiers) and language-server support land in subsequent releases — see the [roadmap](https://github.com/YashBhalodi/kulalang/tree/main/docs/roadmap).

## Local development

1. Open this directory (`editor/vscode/`) in VSCode.
2. Press `F5` to launch a dev-host instance with the extension loaded.
3. In the dev host, open any file from the repo's [`examples/`](https://github.com/YashBhalodi/kulalang/tree/main/examples) directory.

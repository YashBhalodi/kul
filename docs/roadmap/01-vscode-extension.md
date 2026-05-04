# PRD 01 — VSCode extension (TextMate-only)

**Phase:** 1 of 4
**Headline deliverable:** Published `.vsix` extension with syntax highlighting, snippets, file icon. No LSP.
**Target outcome version:** Extension `0.0.1`.

## Problem Statement

A user who wants to author a `.kula` file opens it in VSCode and gets a blank, unstyled text document. There is no syntax highlighting, no snippet templates, no comment-toggle support, no file-icon recognition. Authoring a structured kinship document this way is hostile — the user is asked to remember every keyword and produce well-formed syntax purely from memory while staring at monochrome text.

## Solution

Publish an official Kula VSCode extension that registers `.kula` as a known language and ships:

- TextMate grammar so keywords (`person`, `marriage`, `birth`, `adoption`, etc.), identifiers, strings, dates, circa markers, and comments render in the user's chosen color theme.
- Snippets for the common shapes (a `person` skeleton, a `marriage` skeleton, an `adoption` sub-statement) so the user can scaffold by typing a prefix.
- A language configuration so `Cmd+/` toggles `#` comments and so common bracket pairs auto-close.
- A file icon for `.kula` documents so they're recognizable in the VSCode file tree.
- A README and marketplace listing pointing back to the spec.

The extension is fully declarative — JSON files and TextMate rules. No language server, no Rust binary, no runtime activation logic beyond what VSCode does for declarative extensions. This phase is deliberately lightweight: ship something useful in a couple of days and clear the marketplace publishing ceremony while the artifact is low-stakes.

## User Stories

1. As a Kula author, I want `.kula` files to display colored syntax in VSCode so that I can read structure at a glance.
2. As a Kula author, I want keywords (`person`, `marriage`, `birth`, `adoption`, `gender`, etc.) to be visually distinct from identifiers and values so that I can spot the shape of each statement.
3. As a Kula author, I want string literals to be highlighted distinctly from bare values so that I can see at a glance which fields use quoted strings.
4. As a Kula author, I want date literals and circa markers (`~`) to be highlighted distinctly from strings and identifiers so that dates pop visually.
5. As a Kula author, I want `#` comments to be styled as comments (typically dimmed) so that they don't compete with the actual content.
6. As a Kula author, I want `Cmd+/` (or `Ctrl+/`) to toggle a `#` line comment so that I get the muscle memory I expect from other languages.
7. As a Kula author, I want to type `person` and tab-expand into a fully-templated person skeleton with placeholders for id, name, gender so that I don't have to remember the field syntax.
8. As a Kula author, I want a similar snippet for `marriage` so that I can scaffold a marriage statement from a prefix.
9. As a Kula author, I want a snippet for the `adoption` sub-statement so that adding adoption events is one keystroke away.
10. As a Kula author, I want a recognizable file icon for `.kula` files so that they stand out in the file tree.
11. As a Kula author, I want to install the extension from the official VSCode marketplace by searching "Kula" or "KulaLang" so that I don't need to sideload a `.vsix` manually.
12. As a Kula author, I want the marketplace listing to link to the language spec so that I can learn the full language from one click.
13. As a Kula author, the extension should activate quickly when I open a `.kula` file so that it never feels laggy.
14. As a Kula author, on opening a `.kula` file the extension should NOT show me errors I don't want — no warnings about absent fields, no nags about style. (Live error reporting is Phase 3, not this one.)
15. As a project maintainer, I want the extension source to live inside the `kulalang` monorepo so that it's versioned and reviewed alongside the spec.
16. As a project maintainer, I want a single command (e.g. `pnpm package` or `npm run package`) that produces a publishable `.vsix` so that releasing is friction-free.
17. As a project maintainer, I want the extension to publish under a stable publisher account (`YashBhalodi` or a `kulalang` org) so that it's discoverable and trusted.
18. As an AI agent developer, I want the extension's TextMate grammar in a single human-editable JSON file with comments where rules cover non-obvious cases so that I can extend it confidently.
19. As an AI agent developer, when I add a new keyword to the language spec, I want one obvious place in the extension to register it for highlighting.
20. As a non-Kula user who happens to open a `.kula` file, the extension should not break my VSCode or hijack other file types.
21. As a Kula author on Windows, I want the extension to behave identically to mac/linux (same colors, same snippets) — declarative-only means no platform-specific code.

## Implementation Decisions

### Modules and structure

- All extension source lives under `editor/vscode/` in the existing `kulalang` repo.
- The extension is built and published as a single npm package using the conventional `vsce` (Visual Studio Code Extension manager) toolchain.
- The package is `commonjs` / typescript-free — no transpile step in this phase. All artifacts are JSON.
- Subdirectories under `editor/vscode/`:
  - `syntaxes/` — the TextMate grammar JSON.
  - `snippets/` — snippet definition JSON.
  - `images/` — extension icon and the `.kula` file-type icon.
  - The root holds `package.json`, `language-configuration.json`, `README.md`, `LICENSE` (a copy of the repo MIT license), and `.vscodeignore`.

### Extension manifest

- Publisher: `YashBhalodi` (or a `kulalang` organization if registered before publish).
- Extension id: `kulalang` or `kula`.
- Display name: "Kula".
- Description: "Syntax highlighting and snippets for the Kula kinship description language."
- Categories: `Programming Languages`, `Snippets`.
- Engines: VSCode `^1.70.0` or whatever is the project floor at publish time.
- `contributes.languages`: registers `kula` with extensions `[".kula"]` and the `language-configuration.json`.
- `contributes.grammars`: maps language `kula` to `syntaxes/kula.tmLanguage.json` with scope name `source.kula`.
- `contributes.snippets`: maps language `kula` to `snippets/kula.json`.
- `contributes.iconThemes` (optional): provides a file icon for `.kula`.

### TextMate grammar

- Single file: `syntaxes/kula.tmLanguage.json`.
- Scope name `source.kula`.
- Token classes:
  - `keyword.control.kula` — `person`, `marriage`, `birth`, `adoption`, `kula`.
  - `variable.parameter.kula` (or `keyword.other.kula`) — field names: `name`, `family`, `given`, `born`, `died`, `gender`, `start`, `end`, `end_reason`.
  - `constant.language.kula` — enum values: `male`, `female`, `other`, `divorce`.
  - `string.quoted.double.kula` — string literals with escape support for `\"` and `\\`.
  - `constant.numeric.date.kula` — date literals matching the three granularities (`YYYY-MM-DD`, `YYYY-MM`, `YYYY`), with the `~` prefix highlighted distinctly.
  - `comment.line.number-sign.kula` — `#` comments to end of line.
  - `entity.name.kula` — bare identifiers in declaration position (after `person` / `marriage` keyword).
- The grammar errs on the side of generous matching — TextMate cannot do deep semantic validation, that's the LSP's job in Phase 3. Misclassification here is acceptable for the value-add of basic colors.

### Language configuration

- `comments.lineComment`: `#`
- `comments.blockComment`: not set (block comments are not in the v0.1 spec)
- `brackets`: empty (no syntactic brackets in the language)
- `autoClosingPairs`: `[ "\"", "\"" ]` only (auto-close double-quoted strings)
- `surroundingPairs`: same as auto-closing
- `wordPattern`: matches the spec's identifier production (`[A-Za-z_][A-Za-z0-9_-]*`)

### Snippets

At minimum:

- `person` → expands to a person statement with placeholders for id, display name, gender (with choice between male/female/other).
- `personf` / `personm` (optional shorthand) → same but with gender preset.
- `marriage` → expands to a marriage statement with placeholders for id, two spouses, start date.
- `birth` → expands to a `birth` sub-statement (indented one level) with a marriage-id placeholder.
- `adoption` → expands to an `adoption` sub-statement with marriage-id and start-date placeholders.
- `kula` → expands to `kula 0.1` (the version header).

### Build and release

- `package.json` scripts:
  - `package` — runs `vsce package` to produce the `.vsix`.
  - `publish` — runs `vsce publish` (requires PAT in env).
  - `lint` — runs a JSON validator on the grammar and snippets files (e.g. `ajv` against the TextMate JSON schema, plus the VSCode snippet schema).
- GitHub Actions workflow `.github/workflows/vscode-extension.yml`: on push to `main` affecting `editor/vscode/**`, run lint; on git tag matching `vscode-v*`, run publish.

### Marketplace metadata

- README inside `editor/vscode/` with a screenshot of a highlighted `.kula` file (taken from one of the example documents).
- Marketplace categories, keywords, and a link back to the repo and spec.
- License: MIT (same as repo).

## Testing Decisions

A "good test" for this phase tests the *external behavior* of the extension — does opening a `.kula` file produce the expected highlighting? does the snippet expand to the expected output? — not the implementation details of the TextMate JSON.

Because this phase has no Rust code and no executable logic beyond what VSCode interprets from JSON, the testing surface is small:

### Automated

- **JSON schema validation** for the TextMate grammar (against the standard `tmLanguage` schema) and for the snippets file. Catches typos and structural errors at build time. Runs in CI.
- **Manifest lint** via `vsce ls --tree` (does the package pack correctly with the expected files?) in CI.

### Manual / golden

- **Visual smoke test:** open each file in `examples/` in a dev-host VSCode (`F5`) and visually verify highlighting matches expectations. A short `editor/vscode/TESTING.md` documents the checklist.
- **Snippet expansion verification:** for each snippet, type the prefix in a `.kula` file in dev-host VSCode and confirm the expansion matches the snippet definition.
- **Marketplace install dry-run:** `vsce package` produces a `.vsix`; install it into a fresh VSCode and re-run the visual smoke test.

### Why no automated end-to-end tests in this phase

VSCode extension testing frameworks (`@vscode/test-electron`) exist but their setup cost is non-trivial and the test surface here is mostly visual. End-to-end automated tests are deferred to Phase 3 when the LSP introduces real testable logic. For Phase 1, schema validation + manual visual checks are proportionate to the artifact's size.

### Prior art

- The `vscode-toml`, `vscode-yaml`, and `taplo` extensions are reasonable references for grammar structure, snippet conventions, and `package.json` shape.

## Out of Scope

- Language Server Protocol, autocomplete beyond static snippets, hover, go-to-definition, find-references, rename, code actions, formatting — all deferred to Phases 3 and 4.
- Diagnostics / error squiggles — none in this phase. Until Phase 2 produces a validator and Phase 3 wires it through LSP, the user gets no live error feedback in the editor.
- Cross-platform binary distribution — there is no binary to distribute in this phase.
- Semantic highlighting (e.g. distinguishing person-id-decl from person-ref by color). TextMate grammars cannot do this; semantic tokens are a Phase 4 deliverable via the LSP.
- Other editors (vim, emacs, helix, zed). They are not addressed by this phase. They will be addressable from Phase 3 onward via the LSP.
- Multi-file projects, workspace-level features.
- Telemetry of any kind.

## Further Notes

- The extension's `0.0.1` version intentionally signals "early, not yet feature-complete." The headline release is `0.1.0`, which lands with the basic LSP in Phase 3.
- Marketplace publishing requires a Microsoft Personal Access Token (PAT) for the publisher account. Setting this up is part of Phase 1 and is a forcing function we want to clear early — better to stub a PAT against an empty extension than to discover the publisher-account paperwork mid-LSP-launch.
- The extension's icon and the `.kula` file icon are minor design tasks. A simple visual identity (a stylized कुल character or a family-tree glyph) is sufficient for v0.0.1.

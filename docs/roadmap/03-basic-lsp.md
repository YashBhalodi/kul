# PRD 03 — Basic LSP

**Phase:** 3 of 4
**Headline deliverable:** A `kula-lsp` binary implementing the Language Server Protocol over stdio, plus a VSCode extension update that activates it. Real-time diagnostics, hover, go-to-definition, and basic completion (keywords, field names, enum values) work in VSCode as the user types.
**Target outcome version:** Extension `0.1.0`, `kula-lsp` `0.1.0`.

## Problem Statement

After Phase 2 the user has a working `kula validate` CLI but the editor experience is still primitive: they get TextMate highlighting (Phase 1) but no live errors, no hover tooltips, no jump-to-definition, no autocomplete beyond static snippets. Authoring a real family file means a constant cycle of edit → save → run `kula validate` → read errors → re-edit. This is the same workflow as authoring HTML before browsers had inspectors. Modern language tooling collapses this loop into an in-editor experience, and Kula needs the same to feel like a language a person would actually pick up.

## Solution

Build a Language Server in Rust (`kula-lsp` crate) that:

- Implements the LSP over stdio using `tower-lsp`.
- Wraps `kula-core` as a thin adapter — no business logic is duplicated. Diagnostics, AST, and resolved-document data all come from the same library Phase 2 built.
- Exposes the four foundational LSP capabilities: `publishDiagnostics`, `hover`, `definition`, `completion`. (Other capabilities like `references`, `rename`, `formatting`, `documentSymbol`, `codeAction`, `semanticTokens` land in Phase 4.)

Update the VSCode extension (Phase 1's product) to:

- Bundle the `kula-lsp` binary for the four supported platforms (linux-x64, darwin-x64, darwin-arm64, win32-x64).
- Spawn the binary on `.kula` document activation and wire the LSP client.
- Bump version to `0.1.0` to mark the headline release.

After this phase: the user opens `family.kula` in VSCode, types, and red squiggles appear under the actual problems with hover-over explanations, hovers over a marriage ID and sees the marriage details, Cmd+clicks a person reference to jump to the declaration, and gets autocomplete for keywords / field names / enum values. This is the milestone where Kula "feels like a real language" in the editor.

## User Stories

### As the Kula author (in-editor experience)

1. As a Kula author, I want red squiggles to appear under errors as I type so that I see problems immediately, not on save.
2. As a Kula author, I want hovering over a squiggled token to show the diagnostic message so that I understand what's wrong without leaving the editor.
3. As a Kula author, I want each diagnostic to show the spec rule code (e.g. `KULA-R04`) and message so that I can look up the full rule definition.
4. As a Kula author, I want hover on a person ID to show the person's display name, gender, birth/death dates so that I can verify references without scrolling.
5. As a Kula author, I want hover on a marriage ID to show the marriage's spouses (with display names), start, end, and reason so that I understand at a glance what marriage is being referenced.
6. As a Kula author, I want Cmd+click (Ctrl+click on Windows/Linux) on a person reference to jump to the `person` declaration so that I can navigate the document.
7. As a Kula author, I want the same jump-to-definition behavior on marriage references so that I can navigate from `birth m_alice_bob` to the `marriage` line.
8. As a Kula author, I want autocomplete for top-level keywords (`person`, `marriage`) when I'm at the start of a line so that I don't have to remember spelling.
9. As a Kula author, I want autocomplete for sub-statement keywords (`birth`, `adoption`) when I'm inside an indented continuation of a person so that the right options appear in the right context.
10. As a Kula author, I want autocomplete for field names (`name:`, `gender:`, `family:`, `given:`, `born:`, `died:`) inside a `person` statement so that I see what's available.
11. As a Kula author, I want autocomplete for marriage-statement fields (`start:`, `end:`, `end_reason:`) inside a `marriage` statement so that I see only the contextually-valid options.
12. As a Kula author, I want autocomplete for enum values after `gender:` (suggesting `male`, `female`, `other`) and after `end_reason:` (suggesting `divorce`) so that I don't have to recall the vocabulary.
13. As a Kula author, I want diagnostics to update within ~100ms of stopping typing so that the feedback feels live, not laggy.
14. As a Kula author, I want the LSP to handle a partially-typed (broken) document gracefully — still show diagnostics, still serve hover and completion where the parse succeeds — so that the editor doesn't go dark when the document is mid-edit.
15. As a Kula author, I want the extension to start the LSP automatically the first time I open a `.kula` file so that there's zero configuration.
16. As a Kula author, I should not need to install Rust or any other runtime — the extension bundles the LSP binary for my platform.
17. As a Kula author on Windows, macOS (Intel), macOS (Apple Silicon), or Linux, the LSP should work identically.
18. As a Kula author, when I have many `.kula` files open in one workspace, each should be diagnosed independently and editing one shouldn't slow editing another.
19. As a Kula author, if the LSP crashes, VSCode should restart it gracefully and the user should see at most a brief gap in features, not lose work.

### As a non-VSCode editor user

20. As a Neovim/Helix/Zed/Emacs user, I want the `kula-lsp` binary published as a separate GitHub Release artifact so that I can configure my editor's LSP client to point at it.
21. As a Neovim user, I want a brief documentation snippet showing how to register `kula-lsp` with `nvim-lspconfig` so that I can get the same in-editor features without a Kula-specific extension.

### As an AI agent developer

22. As an AI agent developer, I want the LSP layer to be a thin adapter (no business logic) so that all language semantics live in `kula-core` and are testable without LSP plumbing.
23. As an AI agent developer, I want LSP integration tests that send LSP messages to a child process and assert responses so that I can verify protocol behavior without launching a full editor.
24. As an AI agent developer, I want each LSP capability handler to be its own small module so that I can change one without touching others.
25. As an AI agent developer, I want completion logic to be exposed as a pure function over (resolved-document, position) so that I can unit-test it without LSP plumbing.
26. As an AI agent developer, I want the test command (`just check`) to include LSP integration tests so that one command verifies all layers.
27. As an AI agent developer, I want clear error logging from the LSP server (configurable level via `RUST_LOG`) so that I can diagnose issues during development.

### As the project maintainer

28. As the project maintainer, I want the extension's release process to bundle the right LSP binary for each platform automatically so that I don't manually pack three binaries.
29. As the project maintainer, I want LSP releases tagged `lsp-v0.1.0` and extension releases tagged `vscode-v0.1.0` so that the two versioning streams are independent but coordinated.
30. As the project maintainer, I want the LSP binary name (`kula-lsp`) to be distinct from the CLI binary (`kula`) so that they don't conflict on PATH.
31. As the project maintainer, I want the LSP binary's startup time to be under 100ms so that VSCode doesn't show a long activation spinner.

### As a Rust beginner (the repo owner)

32. As a Rust beginner, I want the LSP code to follow the same module conventions as the parser/validator so that I can navigate by analogy.
33. As a Rust beginner, I want the async parts (LSP requires `tokio`) confined to the outermost layer so that the inner pure logic is sync and easy to reason about.

## Implementation Decisions

### Workspace addition

A new crate `crates/kula-lsp/` joins the workspace. It depends on `kula-core` and on `tower-lsp` + `tokio`.

```
crates/
  kula-core/    # unchanged from Phase 2
  kula-cli/     # unchanged from Phase 2
  kula-lsp/     # NEW — language server binary
```

The `kula-cli` crate gains a `kula lsp` subcommand that simply launches the language server (so operators can run `kula lsp` instead of needing a separate binary on PATH). The standalone `kula-lsp` binary is also produced for editor integrations that don't go through the CLI.

### `kula-lsp` modules

- **`main`** — sets up `tokio` runtime, instantiates the server, runs over stdio.
- **`server`** — the `tower-lsp::LanguageServer` implementation. One method per LSP capability we serve. Each method delegates to a feature module.
- **`state`** — the server's open-document state. Maps URI → `Document` (source text + parsed AST + resolved doc + cached diagnostics). Updated on `didOpen`, `didChange`, `didClose`. All access through this module — no shared mutable state elsewhere.
- **`features::diagnostics`** — converts `kula-core` diagnostics into LSP diagnostics. Triggered after every parse on document change.
- **`features::hover`** — implements `textDocument/hover`. Given a position, finds the AST node under the cursor, dispatches by node type to a hover-content builder.
- **`features::definition`** — implements `textDocument/definition`. Given a position on a reference, looks up the declaration in the resolved doc and returns its location.
- **`features::completion`** — implements `textDocument/completion`. Given a position, classifies the completion context (top-level start? after `person <id>`? inside a value?) and returns the appropriate completion items.
- **`convert`** — utility module: byte spans ↔ LSP positions (LSP uses 0-indexed UTF-16 code units; `kula-core` uses byte offsets; we need a clean conversion).

`server` and `state` are LSP-specific glue. `features::*` modules are mostly pure functions called from `server`, which makes them easy to unit-test without LSP plumbing.

### Document sync strategy

Full sync (`TextDocumentSyncKind::FULL`) for v0.1.0. Incremental sync is more efficient but adds complexity that's not worth it at the corpus size we expect. Revisit in Phase 4 if profiling shows a need.

### Reparse-on-change cadence

- On every `didChange` notification, reparse the entire document synchronously, recompute resolved-doc and diagnostics, and publish diagnostics.
- For documents up to ~1000 statements, this should be well under 100ms. We measure and revisit if it isn't.
- No debouncing — `tower-lsp` queues requests so the user-perceived latency is whatever VSCode's natural change-firing cadence is.

### Completion context classification

Completion is the trickiest of the four basic capabilities. We classify the cursor's context using the parser's partial AST (or the token stream when the parse failed at this location):

| Context | Items returned |
| --- | --- |
| Start of a top-level line | `person`, `marriage`, `kula` (and version) |
| After `person <id>` or in a person field-list | Person field names with `:` suffix (`name:`, `gender:`, etc.), filtered to omit fields already present |
| Inside an indented continuation of a person | `birth`, `adoption` |
| After `marriage <id> <person> <person>` or in marriage field-list | Marriage field names (`start:`, `end:`, `end_reason:`), filtered to omit fields already present |
| Right after `gender:` | `male`, `female`, `other` |
| Right after `end_reason:` | `divorce` |
| Other positions | empty (no completion served) |

ID-reference completion (after `birth `, suggest declared marriage IDs) is **out of scope for this phase** — it requires more intricate context detection and richer completion-item formatting. Defers to Phase 4.

### Hover content

| Cursor on... | Hover shows |
| --- | --- |
| A keyword (`person`, `marriage`, `birth`, `adoption`) | A one-line description and a link to the relevant spec section |
| A person's declared ID | The person's display name, gender, born, died, with each field formatted nicely |
| A marriage's declared ID | The marriage's spouses (with display names), start, end, end_reason |
| A reference to a person ID | Same content as the declaration would show |
| A reference to a marriage ID | Same content as the declaration would show |
| A field name in a `person` or `marriage` | A one-line description of the field (e.g. "Display name; full UTF-8") |
| Anywhere else | nothing |

### Distribution

- The `kula-lsp` binary is built per-platform via the same release CI that builds `kula-cli`.
- The VSCode extension's release pipeline downloads the latest `kula-lsp` binaries (matched by version) and bundles them into the `.vsix` under `editor/vscode/server/<platform>/kula-lsp[.exe]`.
- The extension activates on language `kula`, locates the right binary based on `process.platform` and `process.arch`, and spawns it via `vscode-languageclient`.
- For users who want to point at a custom-built `kula-lsp` (e.g. for development), the extension exposes a setting `kula.serverPath` that overrides the bundled binary.

### Logging

- `kula-lsp` uses `tracing` for structured logging.
- Logs go to stderr (LSP convention; stdin/stdout are reserved for the protocol).
- `RUST_LOG=kula_lsp=debug` enables verbose logging during development.
- The VSCode extension exposes the LSP's stderr in a "Kula LSP" output channel so developers can see logs without checking files.

### Versioning

- `kula-lsp 0.1.0` ships with `kula-core 0.2.0` (which gets a minor bump for any API additions Phase 3 needs from `kula-core` — likely span-to-position helpers and AST node-at-position queries).
- The VSCode extension bumps to `0.1.0` to mark the LSP-backed headline release.
- Extension settings live under the `kula.*` namespace.

## Testing Decisions

### What makes a good test (in this phase)

- **LSP integration tests** spawn the `kula-lsp` binary as a subprocess, send LSP messages, and assert responses. They exercise the full stack: server ↔ state ↔ features ↔ kula-core. They are slower than unit tests but indispensable — they catch protocol-level mistakes (wrong response shape, off-by-one in position conversion) that unit tests can't.
- **Unit tests for `features::*`** call the feature functions directly with constructed inputs (e.g. a parsed document and a cursor position) and assert outputs (e.g. completion items). These are fast and cover the bulk of edge cases.
- **Snapshot tests for completion** capture the full completion list at specific positions in canned `.kula` snippets — when the list changes, the snapshot diff is reviewed.

### Per-module test plan

- **`state`** — direct tests on document open/change/close lifecycle. Verify that diagnostics are recomputed on change and stale data isn't served.
- **`features::diagnostics`** — for each spec rule, verify the LSP diagnostic shape (message, range, severity, code). Reuses Phase 2's invalid-corpus files.
- **`features::hover`** — for each hoverable node type (keyword, person decl, marriage decl, person ref, marriage ref, field name), verify the hover content. Snapshot-tested.
- **`features::definition`** — verify `textDocument/definition` returns the correct location for each of: spouse refs in a marriage, marriage refs in `birth`/`adoption`. Verify it returns nothing on a declaration (you don't go to the definition of a declaration).
- **`features::completion`** — exhaustive tests for each of the seven context categories listed above. Snapshot tests of the full completion list.
- **`convert`** — round-trip tests: byte_offset → LSP_position → byte_offset. UTF-8 multi-byte characters and CRLF line endings get explicit coverage.

### LSP integration test corpus

```
crates/kula-lsp/tests/
  integration/
    diagnostics.rs    # opens a doc with errors, verifies publishDiagnostics
    hover.rs          # opens a doc, sends hover at position, asserts response
    definition.rs     # same pattern for go-to-def
    completion.rs     # same pattern for completion at specific positions
  scripted/           # JSON-RPC scripts for manual replay (debugging aid)
```

### What we don't test in this phase

- We don't test VSCode-extension UI behavior. The extension is a thin client — `vscode-languageclient` does the heavy lifting and is a trusted dependency. Manual smoke testing in dev-host VSCode covers regressions.
- We don't load-test the LSP. Performance work waits for Phase 4 if profiling reveals a need.
- We don't test the bundled-binary distribution format. CI builds and `vsce` packaging together cover that the binary lands in the right place.

### Prior art

- `taplo` (TOML LSP): clean separation of `core` ↔ `lsp` ↔ `cli` in one workspace. Reference for our crate boundaries.
- `tinymist` (Typst LSP): tower-lsp idioms, including how to organize feature handlers.
- `rust-analyzer`: how it manages document state and reparses on change. We're far simpler but the patterns transfer.

## Out of Scope

- ID-reference completion (after `birth `, suggesting declared marriage IDs with display info). Phase 4.
- `textDocument/references` (find all references to a person or marriage ID). Phase 4.
- `textDocument/rename` (workspace edit refactoring). Phase 4.
- `textDocument/documentSymbol` (outline view). Phase 4.
- `textDocument/codeAction` (quick-fixes for diagnostics). Phase 4.
- `textDocument/formatting` (formatter). Phase 4.
- `textDocument/semanticTokens` (richer than TextMate highlighting). Phase 4.
- Workspace-wide features (`workspace/symbol`, multi-file references). Out of v1 — there are no multi-file projects in v1.
- Live Family-tree visualization in a VSCode webview. Out of v1 (downstream).
- Configuration UI in VSCode (settings page). For v0.1.0 the only setting is `kula.serverPath`; that's exposed via the standard `package.json` contributes mechanism.
- Publishing the LSP binary to package managers (Homebrew, AUR, etc.). GitHub Releases is sufficient for v0.1.0.

## Further Notes

- **Why `tower-lsp` over hand-rolling LSP plumbing.** It handles all the protocol-message routing and JSON-RPC framing. Trying to hand-roll is a multi-week distraction with no payoff.
- **Why Tokio as the async runtime.** `tower-lsp` requires it. We don't make our own async-runtime choice.
- **Sync logic inside async handlers.** Each LSP request handler's first line should be to call into `kula-core` synchronously (parse + validate) — the language work itself is sync and fast. The async layer is purely for protocol concurrency.
- **Position conversion is the most error-prone glue layer.** LSP uses UTF-16 code units (a quirk inherited from VS Code's TypeScript origins); we use UTF-8 byte offsets. The `convert` module is small but gets dedicated tests because off-by-one here causes invisible bugs (highlighting jumps to the wrong character on documents with non-ASCII names).
- **Diagnostics latency budget.** For a 1000-line document, parse + validate + LSP serialization should fit in 50ms on a modern laptop. We measure on the example corpus during CI, not just functional correctness.
- **The headline version bump.** Phase 1 ships extension `0.0.1` (preview-quality). Phase 3 ships `0.1.0` because LSP support is the first version a user might *recommend* to a friend.
- **Risk: VSCode marketplace review.** Bundled native binaries occasionally get extra scrutiny in marketplace review. We mitigate by having a clean `vsce package` that includes the binaries, signing the binaries on macOS (codesign) and Windows (signtool) when we have certificates available, and being patient if a review takes a few days.

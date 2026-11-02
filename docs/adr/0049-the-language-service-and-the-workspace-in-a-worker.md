# ADR 0049 — The language service: a transport-free crate, a stateful `Workspace` in a worker, UTF-16 at the boundary

**Status:** Accepted
**Date:** 2026-11-02
**Deciders:** owner

Supersedes [ADR-0034](./0034-query-transport-and-result-node-identity.md). Amends [ADR-0002](./0002-token-stream-first-completion-classifier.md) (where the classifier lives), [ADR-0011](./0011-wasm-surface-three-shapes-no-wrappers.md) (the `Workspace`), [ADR-0012](./0012-tsify-derived-types-committed-and-diffed.md) (whose types are committed) and [ADR-0037](./0037-batched-detail-lookup-shape.md) (who calls the batched lookup). Issues [#346](https://github.com/YashBhalodi/kul/issues/346), [#350](https://github.com/YashBhalodi/kul/issues/350).

## Context

The editor features — hover, completion, definition, references, rename, symbols, quick fixes, highlighting, formatting — were written against an editor protocol: line and UTF-16 character positions, URIs, protocol result types, an async runtime. The browser app (ADR-0047) needs the same features in a Web Worker, with no protocol between the editor and the engine. The features' logic was sound and snapshot-tested; their types were the protocol's.

## Decision

### `kul-language-service`: pure functions, Kul-native types

The crate `crates/kul-language-service` holds every editor feature as a pure function over a `&CheckResult`, a file **name** and a **byte** offset: `diagnostics`, `hover`, `complete`, `definition`, `references`, `prepare_rename`, `rename`, `symbols`, `quick_fixes`, `semantic_tokens`, `lexical_tokens`, `format`, `entity_at`, `locate` and `details`. No IO, no protocol types, no async runtime.

Its types are the language's own: `Span { start, end }`, `Location { file, span }`, `TextEdit { span, text }`, `FileEdit { file, edits }` (a rename is one per affected file, in file order), and enums for completion, symbol, token, severity and entity kinds. A completion's snippet is `insert: { text, cursor }` rather than a snippet syntax. `format` returns `None` when the file does not parse and `[]` when it is already canonical. `details(id)` answers through `kul_core::query::details` and is strict on errors: a project with errors answers nothing. `OutlineSymbol` is so named because `Symbol` would shadow the JavaScript global in the generated `.d.ts`.

The feature logic, the completion classifier (ADR-0002) and the semantic-token walk moved without being rewritten; every moved snapshot maps onto its predecessor exactly. Features that key on the entity under the cursor share one helper, `entity_under`.

Diagnostics are **one project-wide list**, each with `location: Option<Location>`; a consumer filters per file. Render-only failures (KUL-V01, KUL-V03) join that list when the check itself is clean.

### `kul-wasm`: one stateful `Workspace`

The WASM surface is one class, `Workspace` (ADR-0011's amendment), plus `KUL_CORE_VERSION()`, `KUL_LANGUAGE_VERSION()` and `diagnosticCodes()`:

- `setFile(name, text, version)`, `renameFile(from, to) -> bool`, `deleteFile(name) -> bool` change the file set and drop the cached check.
- The first request after a change runs `kul_core::check` once over every file and caches the `CheckResult` in a `OnceCell`; every request until the next change reads it.
- `analyze()` returns `{ files: [{ name, version }], diagnostics, svg? }`. `svg` is absent exactly when the project cannot be drawn.
- One method per language feature, plus `renderSvg("agnostic" | "fileExport")`. `rename` returns `{ ok: true, edits } | { ok: false, message }`.

**Versions travel with the files.** The caller tags each text with a version and `analyze()` echoes them, so the caller can tell which texts an answer was computed from without bookkeeping of its own.

### UTF-16 offsets at the boundary, in one place

JavaScript strings are indexed in UTF-16 code units; Rust spans are UTF-8 bytes. Every offset that crosses the boundary goes through one converter, `Utf16Index` (`crates/kul-wasm/src/utf16.rs`), one per file. The service never sees UTF-16 and JavaScript never sees bytes. Results are converted with exhaustive destructuring, so a new span field cannot slip through unconverted. The service's types cross the boundary as themselves (their Tsify derive sits behind a `tsify` feature), documented as UTF-16 on the JS side, rather than as WASM-only mirror types.

### The language worker and its client

The `Workspace` lives in a module Web Worker (`apps/web/src/language/worker.ts`) behind Comlink; the main thread holds a **language client** (`apps/web/src/language/client.ts`):

- Every edit sends the file's whole text with a new version. Versions are client-global and monotonic. An answer is current only if **every** file's version matches the client's latest; otherwise it is dropped. (Hovering while another file changes drops one hover — the simplest correct rule.)
- An edit answers at once with that file's lexical tokens. Analysis waits for edits to pause (200 ms, `ANALYSIS_WAIT_MS`), a newer edit replacing the waiting one; a feature that needs the check (hover, completion, rename, …) runs the waiting analysis first. The analysis report carries every file's semantic tokens.
- **A crash is a crash however it shows**: a worker `error` or `messageerror`, or any call rejecting with a WASM `RuntimeError` (a panic poisons the module). The client terminates the worker, spawns a new one, gives it the file set again, and settles in-flight calls with `undefined`. After four crashes in a minute it stops and says so.

### The editor adapter

CodeMirror 6 is the editor (`apps/web/src/editor/` for the app-agnostic parts, `apps/web/src/app/editor/` for the window). The adapter maps the service's answers onto CodeMirror's extension points: completion source, hover tooltip, lint diagnostics, decorations from tokens (ADR-0050), and transactions for edits.

- A rename or a "format all" is applied **all or nothing** across files: each open file gets one isolated transaction, closed files are written through the project store, and the change is one linked undo step across files. Undo restores a closed file only if it is still as the change left it.
- The editor's own features match keys through TanStack Hotkeys (user-agent Mod); undo and redo stay CodeMirror's (platform Mod). Every shortcut is declared in one table (`apps/web/src/app/shortcuts.ts`), so the shortcuts dialog and the docs list them all.
- Tabs are a `nav` list with `aria-current`, not a tablist, because a tablist with close buttons fails axe.

## Consequences

- One feature, one adapter: a new feature is a service function, a `Workspace` method and a CodeMirror mapping (the recipe is in `docs/architecture.md`).
- The service is tested as plain Rust with byte offsets; the boundary is tested once for UTF-16 (`crates/kul-wasm/tests/utf16.rs`, Gujarati, Devanagari and emoji before every feature's inputs and outputs).
- Measured: `analyze()` on `examples/09-family-across-a-century` after an edit takes 0.17 ms in release; keystroke to squiggle is about 60 ms (median, CI) because typing asks for completions, which flush the waiting analysis.
- The gzipped WASM went from about 208 KB to about 170 KB when the stateless and query exports left (ADR-0011's amendment).

## Anti-suggestions (do not re-propose)

- **"Speak an editor protocol inside the worker."** The protocol's types and positions are a translation layer with one consumer.
- **"Mirror the service's types in WASM-only structs."** Two definitions drift; convert in place at the boundary.
- **"Re-check per request."** The check dominates every feature; the workspace caches it per edit.
- **"Compare versions per file only."** Cross-file answers (references, rename, diagnostics) depend on every file.

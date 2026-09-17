# Editor intelligence in the browser: WASM operations versus a worker language server

**Ticket:** [Research: exposing editor intelligence on kul-wasm versus a worker language server](https://github.com/YashBhalodi/kul/issues/332)
**Feeds:** [Decide: how editor intelligence reaches the browser](https://github.com/YashBhalodi/kul/issues/336)
**Does not decide** the architecture. Standing preference (not reopened here): `check` / `format` / `renderSvg` / `query*` stay in-page WASM.

## Question

Kul's editor floor beyond `check` / `format` — completion, hover, go-to-definition, find-references, rename, and editor↔tree identity (`node_at`, `kul/locate`, `kul/entityAt`) — exists only in `kul-lsp` today. `kul-wasm` has no wrapper for any of it. What are the real ways to put that intelligence in the browser?

## Sources

This file cites only this repo and first-party docs of the third-party paths it names.

- Repo: [`crates/kul-core/src/node_at.rs`](../../crates/kul-core/src/node_at.rs), [`crates/kul-core/src/semantic.rs`](../../crates/kul-core/src/semantic.rs), [`crates/kul-core/src/format.rs`](../../crates/kul-core/src/format.rs), [`crates/kul-core/src/export.rs`](../../crates/kul-core/src/export.rs), [`crates/kul-core/src/span.rs`](../../crates/kul-core/src/span.rs), [`crates/kul-lsp/src/lib.rs`](../../crates/kul-lsp/src/lib.rs), [`crates/kul-lsp/src/server.rs`](../../crates/kul-lsp/src/server.rs), [`crates/kul-lsp/src/state.rs`](../../crates/kul-lsp/src/state.rs), [`crates/kul-lsp/src/convert.rs`](../../crates/kul-lsp/src/convert.rs), [`crates/kul-lsp/src/features/*`](../../crates/kul-lsp/src/features), [`crates/kul-wasm/src/lib.rs`](../../crates/kul-wasm/src/lib.rs), [`docs/architecture.md`](../architecture.md), [ADR-0001](../adr/0001-resolved-document-as-query-seam.md), [ADR-0002](../adr/0002-token-stream-first-completion-classifier.md), [ADR-0007](../adr/0007-resolved-document-owns-document.md), [ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md), [ADR-0015](../adr/0015-global-project-namespace.md), [ADR-0034](../adr/0034-query-transport-and-result-node-identity.md).
- wasm-bindgen: [exporting a function](https://rustwasm.github.io/docs/wasm-bindgen/contributing/design/exporting-rust.html), [`js_name`](https://wasm-bindgen.github.io/wasm-bindgen/reference/attributes/on-rust-exports/js_name.html), [supported types](https://wasm-bindgen.github.io/wasm-bindgen/reference/types.html).
- monaco-languageclient: [docs index](https://github.com/TypeFox/monaco-languageclient/blob/main/docs/index.md), [configuration](https://github.com/TypeFox/monaco-languageclient/blob/main/docs/guides/configuration.md), [examples](https://github.com/TypeFox/monaco-languageclient/blob/main/docs/guides/examples.md).
- vscode-languageserver-node: [README (common / node / browser split)](https://github.com/microsoft/vscode-languageserver-node/blob/main/README.md), [`jsonrpc/src/browser/main.ts`](https://github.com/microsoft/vscode-languageserver-node/blob/main/jsonrpc/src/browser/main.ts), [`server/src/browser/main.ts`](https://github.com/microsoft/vscode-languageserver-node/blob/main/server/src/browser/main.ts).
- tower-lsp (the crate `kul-lsp` actually runs): [`Server`](https://docs.rs/tower-lsp/latest/tower_lsp/struct.Server.html).

## 1. What each LSP feature actually calls

`kul-lsp` is a thin adapter: "every diagnostic, hover, definition, and completion answer comes from the core query seam (ADR-0001). The async layer here is purely for LSP-protocol concurrency" ([`crates/kul-lsp/src/lib.rs`](../../crates/kul-lsp/src/lib.rs)). Feature modules are "pure functions over the cached document state" ([`crates/kul-lsp/src/features/mod.rs`](../../crates/kul-lsp/src/features/mod.rs)). The request flow is: URI + LSP `Position` → `ProjectEntry::cursor_for_uri` → `LineIndex::byte_offset` → `resolved.node_at(file, offset)` → feature builder → LSP types ([`docs/architecture.md`](../architecture.md) "LSP request flow"; [`state.rs`](../../crates/kul-lsp/src/state.rs) `cursor_for_uri` / `Cursor::entity`).

`ResolvedDocument` is the kinship-query seam ([ADR-0001](../adr/0001-resolved-document-as-query-seam.md)). `node_at(file, byte_offset)` is "What's at this byte offset? — the foundation for hover, goto-def, and completion" ([`node_at.rs`](../../crates/kul-core/src/node_at.rs)). It returns a typed `Node` (keyword, decl id, person/marriage ref with optional resolved target, field name/value). Whitespace, comments, and out-of-range positions yield `None`. Smallest enclosing span wins. Reference targets resolve project-wide ([ADR-0015](../adr/0015-global-project-namespace.md); [`node_at.rs`](../../crates/kul-core/src/node_at.rs) module docs). `Node::entity_reference` collapses the four id variants into an `EntityNode` (kind, name, ident span, is_decl, optional `EntityTarget`). `ResolvedDocument::references_to(id, kind)` walks every file and returns `FileSpan`s of reference sites, **excluding** the declaration; unresolved refs with a matching name are still returned "so rename/find-references work on partial documents" ([`semantic.rs`](../../crates/kul-core/src/semantic.rs)).

`node_at` / `statement_at` keep a `FileId` because "byte offsets are inherently per-file" ([ADR-0015](../adr/0015-global-project-namespace.md)). Per-id lookups (`entity`, `person`, `marriage`) take a bare id — the project is one namespace.

The table below is the floor named by the ticket. Semantic tokens, document symbols, and code actions exist in `kul-lsp` and are out of this floor ([issue #328](https://github.com/YashBhalodi/kul/issues/328) standing preference).

| Feature | Language seam | Protocol machinery |
| --- | --- | --- |
| **Hover** (`textDocument/hover`) | `resolved.node_at` → match `Node`; person/marriage panels read AST fields; field hover reads `field_meta::meta(…).hover_md`; spouse-role line uses `statement_at` ([`hover.rs`](../../crates/kul-lsp/src/features/hover.rs)) | Wrap Markdown in `lsp_types::Hover` / `MarkupContent`; `line_index.range(span)` for the highlight ([`hover.rs`](../../crates/kul-lsp/src/features/hover.rs); [`convert.rs`](../../crates/kul-lsp/src/convert.rs)) |
| **Go-to-definition** | `Cursor::entity()` → `entity.decl_span()`; no-op on a decl or unresolved ref ([`definition.rs`](../../crates/kul-lsp/src/features/definition.rs); [`Cursor::entity`](../../crates/kul-lsp/src/state.rs)) | `ProjectEntry::location_for(FileSpan)` → `lsp_types::Location` (URI + UTF-16 `Range`) ([`state.rs`](../../crates/kul-lsp/src/state.rs) `location_for`) |
| **Find-references** | `entity` + `resolved.references_to(name, kind)`; optional `decl_span` when `include_declaration` ([`references.rs`](../../crates/kul-lsp/src/features/references.rs)) | Sort/dedup `FileSpan`s, map each through `location_for` ([`references.rs`](../../crates/kul-lsp/src/features/references.rs)) |
| **Rename** | `entity` + `decl_span` + `references_to`; validate with `kul_core::lexer::{is_identifier, is_reserved_word}` and `resolved.entity(new_name)` collision ([`rename.rs`](../../crates/kul-lsp/src/features/rename.rs)) | `prepare_rename` → `PrepareRenameResponse::Range(line_index.range(ident_span))`; `rename` groups spans by `FileId`, looks up each file's `Url` + `LineIndex`, emits `WorkspaceEdit { changes: HashMap<Url, Vec<TextEdit>> }` ([`rename.rs`](../../crates/kul-lsp/src/features/rename.rs)) |
| **Completion** | Token-stream-first `classify` ([ADR-0002](../adr/0002-token-stream-first-completion-classifier.md); [`classify.rs`](../../crates/kul-lsp/src/features/completion/classify.rs)). `tokenize(source)` then classify; `ResolvedDocument` / `statement_at` / `node_at` are a **secondary** signal (enclosing statement, already-declared fields, project-wide person/marriage ids). Item lists for `MarriageRefPosition` / `SpousePosition` call `resolved.marriages()` / `resolved.persons()` ([`items.rs`](../../crates/kul-lsp/src/features/completion/items.rs)) | Each item is an `lsp_types::CompletionItem` (`kind`, `detail`, snippet `insert_text` / `InsertTextFormat::SNIPPET`) ([`items.rs`](../../crates/kul-lsp/src/features/completion/items.rs)). Classifier lives in `kul-lsp`, not `kul-core` ([ADR-0002](../adr/0002-token-stream-first-completion-classifier.md) anti-suggestion: "the classifier is editor-protocol concern; it stays in kul-lsp") |
| **`kul/locate`** | `check.resolved().entity(id)` → `EntityRef::span()` ([`locate.rs`](../../crates/kul-lsp/src/features/locate.rs); [`semantic.rs`](../../crates/kul-core/src/semantic.rs) `EntityRef::span`) | `location_for` → `LocateResponse { location: Option<Location> }`. Request identifies the project by any open URI; the id may live in a sibling file ([`locate.rs`](../../crates/kul-lsp/src/features/locate.rs)) |
| **`kul/entityAt`** | `cursor.entity()` filtered to `target.is_some()` (unresolved refs report `null`) ([`entity_at.rs`](../../crates/kul-lsp/src/features/entity_at.rs)) | URI + LSP `Position` in, `{ id, kind }` or `null` out. Inverse of locate ([`entity_at.rs`](../../crates/kul-lsp/src/features/entity_at.rs)) |
| **Diagnostics** (not a request; a publish) | `kul_core::check` → `CheckResult.diagnostics` ([`architecture.md`](../architecture.md); [`state.rs`](../../crates/kul-lsp/src/state.rs) `build_entry`) | `diagnostics::to_lsp` filters by `FileId`, maps primary/related spans through each file's `LineIndex` and `Url`, then `client.publish_diagnostics` per URI ([`diagnostics.rs`](../../crates/kul-lsp/src/features/diagnostics.rs); [`server.rs`](../../crates/kul-lsp/src/server.rs) `publish_project`) |
| **Format** (`textDocument/formatting`) | `kul_core::format::format_source(source)` ([`formatting.rs`](../../crates/kul-lsp/src/features/formatting.rs); [`format.rs`](../../crates/kul-core/src/format.rs)) | Refuse when this file has `KUL-L*` / `KUL-P*` errors; else one full-document `TextEdit` (or empty list if already canonical) ([`formatting.rs`](../../crates/kul-lsp/src/features/formatting.rs)) |

Protocol machinery that every cursor-shaped request shares, and that is **not** in `kul-core`:

- **`LineIndex`** — UTF-8 byte offset ↔ LSP `Position` (0-based line, 0-based UTF-16 code units). CRLF-aware. Lives in [`crates/kul-lsp/src/convert.rs`](../../crates/kul-lsp/src/convert.rs). Architecture names it as the "Byte ↔ LSP-position" seam ([`architecture.md`](../architecture.md)).
- **URIs** — `ProjectEntry` keeps `files: Vec<FileMeta { url, line_index }>` in `FileId` order and maps `FileSpan` → `Location` ([`state.rs`](../../crates/kul-lsp/src/state.rs) `location_for` / `url_for` / `file_id_for`).
- **`WorkspaceEdit`** — URI-keyed `TextEdit` lists with UTF-16 ranges ([`rename.rs`](../../crates/kul-lsp/src/features/rename.rs)).
- **`publishDiagnostics`** — server-push, per URI, with version on the active document ([`server.rs`](../../crates/kul-lsp/src/server.rs) `publish_project`).
- **Document cache + overlay + disk discovery** — one `CheckResult` per project root; editor buffers win over disk; `kul_loader::discover` reads the directory ([`state.rs`](../../crates/kul-lsp/src/state.rs) `Documents` / `build_entry` / `discover_disk`). Cached so hover/completion/definition do not re-resolve per keystroke ([ADR-0007](../adr/0007-resolved-document-owns-document.md)).
- **Stdio JSON-RPC** — `kul_lsp::run` binds `tokio::io::stdin` / `stdout` to `tower_lsp::Server` and registers the five custom methods ([`lib.rs`](../../crates/kul-lsp/src/lib.rs)).

`kul-core` locators stay URI-less: `ByteSpan` is a half-open UTF-8 range; `FileSpan` is `(FileId, ByteSpan)` ([`span.rs`](../../crates/kul-core/src/span.rs)).

## 2. What `check` and `format` already prove about the WASM path

`kul-wasm` is a wasm-bindgen adapter over `kul-core` with **no** `kul-lsp` dependency ([`crates/kul-wasm/Cargo.toml`](../../crates/kul-wasm/Cargo.toml); crate graph in [`architecture.md`](../architecture.md): `kul-wasm → kul-core`). [ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md) pins operation-specific shapes, no uniform `{ ok }` envelope, no convenience layer. Each `#[wasm_bindgen]` function is a short translation of one `kul-core` call ([`lib.rs`](../../crates/kul-wasm/src/lib.rs)). wasm-bindgen's first-party contract for that shape: `#[wasm_bindgen]` on a Rust `pub fn` generates JS/Rust shims so richer types than numbers (strings, structs) cross the ABI ([exporting a function](https://rustwasm.github.io/docs/wasm-bindgen/contributing/design/exporting-rust.html)); `js_name` is how this crate exposes camelCase (`format`, `check`, `queryPerson`, …) ([`js_name`](https://wasm-bindgen.github.io/wasm-bindgen/reference/attributes/on-rust-exports/js_name.html); [`lib.rs`](../../crates/kul-wasm/src/lib.rs)).

Proven already:

1. **Project-shaped, stateless, files + typed manifest.** `check(files, manifest)`, `exportGraph`, `renderSvg`, and every `query*` take `Vec<WasmInputFile>` (`{ name, source }`) plus a typed `Manifest`. The host enumerates files; the bridge does not read disk and does not enable `kul-core`'s `yaml` feature ([`lib.rs`](../../crates/kul-wasm/src/lib.rs); [`Cargo.toml`](../../crates/kul-wasm/Cargo.toml); [ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md) 2026-05-23 `renderSvg` amendment; [ADR-0015](../adr/0015-global-project-namespace.md) slice 4). The TypeScript compile-test writes this down as the consumer contract ([`usage.ts`](../../crates/kul-wasm/tests/typescript/usage.ts)).
2. **Per-file `format(source) -> string`.** No manifest, no sibling files. "the underlying formatter has no cross-file interaction" ([`lib.rs`](../../crates/kul-wasm/src/lib.rs) crate docs; [`usage.ts`](../../crates/kul-wasm/tests/typescript/usage.ts)).
3. **Never-throwing, operation-specific failure.** `check` always returns `{ diagnostics }` — empty means clean, no `ok` ([ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md); [`CheckEnvelope`](../../crates/kul-wasm/src/lib.rs)). Query ops return `QueryEnvelope` error arms. Panics are reserved for `kul-core` bugs via `console_error_panic_hook` ([ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md) anti-suggestion "Throw JS exceptions").
4. **URI-less spans.** `check` projects diagnostics to `ExportedDiagnostic` / `ExportedSpan`: `file` is `InputFile.name` (or the manifest name), plus `byte_start` / `byte_end` / `line` / `column` ([`export.rs`](../../crates/kul-core/src/export.rs)). No `file://` URI, no UTF-16 `Range`.
5. **Stateless re-check.** Every project-scoped WASM op calls `check_with_manifest` on the way in ([`lib.rs`](../../crates/kul-wasm/src/lib.rs)). [ADR-0034](../adr/0034-query-transport-and-result-node-identity.md) records this as deliberate: "there is no persistent document handle, so each call re-checks the project." Measured on the 10k-person corpus: WASM `check` ≈ 13.72 ms, 42–100% of every query call, ~1.35 µs per declared person; WASM-over-native multiplier 1.0–1.14× at the ceiling ([ADR-0034](../adr/0034-query-transport-and-result-node-identity.md) #292 table). A stateful WASM handle is named there as "the obvious first move if the epic's measurements come back badly" and is rejected as out of scope, not as wrong.

Where the shipped WASM path **diverges** from LSP, on the two operations that already exist on both sides:

| | WASM today | LSP today |
| --- | --- | --- |
| **Format on parse error** | `format_source` "Returns partial output on recoverable parse errors" ([`format.rs`](../../crates/kul-core/src/format.rs)). The WASM bridge forwards that string unconditionally ([ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md); [`format_source`](../../crates/kul-wasm/src/lib.rs)). Tests call `format("person")` and `format("@@@ not kul @@@")` and only require "must not panic" ([`crates/kul-wasm/tests/format.rs`](../../crates/kul-wasm/tests/format.rs) `format_returns_string_for_partial_parse_input`). | `formatting` returns `None` when this file has error-severity `KUL-L*` or `KUL-P*` diagnostics, so "the editor falls back to user input instead of mangling broken source"; `KUL-R*` still formats ([`formatting.rs`](../../crates/kul-lsp/src/features/formatting.rs)). |
| **Diagnostic / span identity** | `ExportedSpan.file` is a host-supplied name; offsets are UTF-8 bytes ([`export.rs`](../../crates/kul-core/src/export.rs)). | `publishDiagnostics` on a `Url`; ranges are UTF-16 via `LineIndex` ([`diagnostics.rs`](../../crates/kul-lsp/src/features/diagnostics.rs); [`convert.rs`](../../crates/kul-lsp/src/convert.rs)). Manifest-anchored diagnostics are filtered off `.kul` URIs. |
| **When the project is checked** | Every call ([ADR-0034](../adr/0034-query-transport-and-result-node-identity.md)). | Once per overlay mutation; feature handlers read `entry.check.resolved()` ([ADR-0007](../adr/0007-resolved-document-owns-document.md); [`state.rs`](../../crates/kul-lsp/src/state.rs)). |
| **How files arrive** | Host-built `WasmInputFile[]` + typed `Manifest` ([`lib.rs`](../../crates/kul-wasm/src/lib.rs)). | `did_open` / overlay + `kul_loader::discover` on a directory ([`state.rs`](../../crates/kul-lsp/src/state.rs) `discover_disk`). |

A new editor-intelligence export on `kul-wasm` would inherit the left column unless someone adds a handle or a format-strictness flag. That is a cost, not a recommendation.

## 3. Three real ways to put the floor in the browser

### 3.1 Operation-shaped WASM exports

One `#[wasm_bindgen]` function per floor feature, same posture as `check` / `query*`: **stateless**, arguments `files + manifest + cursor` (file name + byte offset, or id for locate), return an operation-specific shape ([ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md); wasm-bindgen [exporting a function](https://rustwasm.github.io/docs/wasm-bindgen/contributing/design/exporting-rust.html)). Architecture's "add a WASM operation" recipe is exactly this: a `pub fn` that calls one `kul-core` function, Tsify the return type, snapshot + `usage.ts` + `just wasm` ([`architecture.md`](../architecture.md) "Where to add X" / WASM).

What is cheap, because it is already a `kul-core` method:

- **`nodeAt` / `entityAt`** — `check_with_manifest` → `resolved.node_at(file, offset)` → project `Node` / `EntityNode` (need a serializable projection; `Node` itself borrows the AST ([`node_at.rs`](../../crates/kul-core/src/node_at.rs))).
- **`locate`** — `resolved.entity(id).map(EntityRef::span)` → `{ file: InputFile.name, byte_start, byte_end }` or `null` ([`locate.rs`](../../crates/kul-lsp/src/features/locate.rs) language half; [`export.rs`](../../crates/kul-core/src/export.rs) `ExportedSpan` is the existing name-and-bytes shape).
- **`definition` / `references`** — `entity_reference` + `decl_span` / `references_to` → lists of name-and-bytes spans ([`definition.rs`](../../crates/kul-lsp/src/features/definition.rs); [`references.rs`](../../crates/kul-lsp/src/features/references.rs); [`semantic.rs`](../../crates/kul-core/src/semantic.rs)).
- **Rename validation + edit list** — `is_identifier` / `is_reserved_word` / `entity` collision + `references_to` + decl span. The **edits** are not a `WorkspaceEdit` (see §4).

What is not already a `kul-core` export, and sits in `kul-lsp` returning `lsp_types`:

- **Hover Markdown builders** ([`hover.rs`](../../crates/kul-lsp/src/features/hover.rs)).
- **Completion classifier + item lists** ([`classify.rs`](../../crates/kul-lsp/src/features/completion/classify.rs); [`items.rs`](../../crates/kul-lsp/src/features/completion/items.rs); [ADR-0002](../adr/0002-token-stream-first-completion-classifier.md) parks them in `kul-lsp` and lists "Move classification into kul-core so the CLI could use it too" as an anti-suggestion — the stated reason is that the CLI does not need completion).

`kul-wasm` cannot take a `kul-lsp` dependency without pulling `tower-lsp`, `tokio`, and `kul-loader` into the wasm graph ([`kul-lsp/Cargo.toml`](../../crates/kul-lsp/Cargo.toml); [`architecture.md`](../architecture.md) unidirectional graph). A WASM completion/hover export therefore either (a) extracts those pure functions off `lsp_types` into a crate both adapters can call, or (b) re-implements the builders next to the wasm functions, still calling the same `kul-core` seams. This note does not pick.

Cost that is already measured for this posture: **every keystroke that calls an op re-checks the project** ([ADR-0034](../adr/0034-query-transport-and-result-node-identity.md)). Hover and completion are the hot path; query-in-the-webview accepted the re-check because a hover-lens query at 10k persons stayed inside ~50 ms. Completion fires more often than a preview query. The LSP side does not pay this: `did_change` rebuilds once, handlers read the cache ([ADR-0007](../adr/0007-resolved-document-owns-document.md)).

The editor, not the bridge, would: map byte spans onto its buffers; apply multi-file edits; paint diagnostics from `check` (already URI-less); decide whether to format on parse error (today's WASM contract says yes). That matches ADR-0011's "no convenience layer" — the bridge does not become an editor.

### 3.2 Hosting `kul-lsp` in a worker

`kul-lsp` as shipped is not a worker. `run()` "Run the language server over stdio. Blocks until the client disconnects" and passes `tokio::io::stdin()` / `stdout()` to `tower_lsp::Server` ([`lib.rs`](../../crates/kul-lsp/src/lib.rs)). The crate description is "Language server for the Kul language (LSP over stdio)" ([`Cargo.toml`](../../crates/kul-lsp/Cargo.toml)). Architecture: "client (VSCode) → JSON-RPC over stdio" ([`architecture.md`](../architecture.md)). Disk discovery is `kul_loader::discover` on a filesystem path ([`state.rs`](../../crates/kul-lsp/src/state.rs) `discover_disk`). Overlay keys are `Url`s, typically `file://`.

tower-lsp's first-party `Server` is "Server for processing requests and responses on standard I/O or TCP"; `new` "Creates a new Server with the given stdin and stdout handles"; `serve` "Spawns the service with messages read through stdin and responses written to stdout" ([`Server`](https://docs.rs/tower-lsp/latest/tower_lsp/struct.Server.html)). The same page says the crate's own concurrency limiter exists so it can "support exotic targets currently incompatible with tokio, such as WASM" — that is about **not requiring `tokio::spawn`**, not about a Worker `postMessage` transport. There is no documented worker reader/writer on that type.

So "put `kul-lsp` in a worker" is not a compile flag. It is a rewrite of the adapter's I/O and discovery:

- Replace stdio with something a worker can read/write (tower-lsp still wants `AsyncRead` / `AsyncWrite` stdin/stdout, not `BrowserMessageReader`).
- Replace `kul_loader::discover` / `file://` with the vault's in-memory files (the WASM host already does this by construction).
- Compile `tokio` + `tower-lsp` + `kul-loader` + the feature modules to `wasm32`, or drop the async runtime.
- Keep `LineIndex`, `WorkspaceEdit`, `publishDiagnostics`, and URI identity — the worker would invent URIs for vault files.

The feature functions themselves (`hover::hover`, `rename::rename`, …) are then reusable. The cache (`Documents` / one `CheckResult` per project) comes along, which is the property WASM ops do not have ([ADR-0007](../adr/0007-resolved-document-owns-document.md)). The cost is the transport/discovery rewrite plus a much larger wasm graph than `@kullang/wasm` today ([`kul-wasm/Cargo.toml`](../../crates/kul-wasm/Cargo.toml) vs [`kul-lsp/Cargo.toml`](../../crates/kul-lsp/Cargo.toml)).

A browser cannot spawn the native `kul-lsp` binary. That path is desktop-only (the VSCode host already does it).

### 3.3 The editor speaks LSP to a WASM language server

This path is first-party on the **editor** side and does not require `kul-lsp`'s stdio binary.

monaco-languageclient's job is "Integrate the Monaco Editor with language clients and language servers utilizing the Language Server Protocol"; it names "WebSocket or Web Worker connections" as supported ([docs index](https://github.com/TypeFox/monaco-languageclient/blob/main/docs/index.md)). Connection types include `WorkerConfig` (URL + `type: 'module'`) and `WorkerDirect` (`new Worker('./language-server.js', { type: 'module' })`) ([configuration](https://github.com/TypeFox/monaco-languageclient/blob/main/docs/guides/configuration.md) "Connection Types"). Example 5 ("Web Worker Language Server") wires `BrowserMessageReader` / `BrowserMessageWriter` from `vscode-languageclient/browser` to a `LanguageClientWrapper` with `documentSelector` ([examples](https://github.com/TypeFox/monaco-languageclient/blob/main/docs/guides/examples.md)). The same guide documents an in-memory filesystem overlay (`RegisteredFileSystemProvider` / `RegisteredMemoryFile`) for "browser-only applications."

On the **server** side, vscode-languageserver-node split every module into `common` / `node` / `browser` so "the LSP client and server npm modules" can run "in a Web browser via webpack"; `vscode-jsonrpc/browser` is "the common and browser code" ([README](https://github.com/microsoft/vscode-languageserver-node/blob/main/README.md)). `BrowserMessageReader` / `BrowserMessageWriter` take `MessagePort | Worker | DedicatedWorkerGlobalScope` and `postMessage` JSON-RPC messages ([`jsonrpc/src/browser/main.ts`](https://github.com/microsoft/vscode-languageserver-node/blob/main/jsonrpc/src/browser/main.ts)). `vscode-languageserver` browser `createConnection(reader, writer)` builds a `Connection` over those transports ([`server/src/browser/main.ts`](https://github.com/microsoft/vscode-languageserver-node/blob/main/server/src/browser/main.ts)).

That is a real, documented way to speak LSP entirely in-page. It does **not** implement Kul. The worker still has to answer `textDocument/hover` etc. Two implementations sit under this transport:

- A JS/TS server (`createConnection` in the worker) that calls **operation-shaped WASM** (§3.1) and translates name-and-bytes spans into LSP `Location` / `WorkspaceEdit` / `publishDiagnostics` (so `LineIndex` and URI mapping move to JS).
- A new Rust wasm crate that speaks JSON-RPC over the same `postMessage` pipes. That is closer to §3.2, minus the fiction that today's `kul_lsp::run` is that crate.

The editor then gets completion, hover, definition, references, rename, and diagnostics from the protocol for free — **if** the chosen editor stack is the monaco-languageclient / VS Code API one. Issue #336 is blocked on [Decide: in-browser editor stack](https://github.com/YashBhalodi/kul/issues/334); a stack that is not LSP-shaped would not spend this path.

Custom Kul methods (`kul/locate`, `kul/entityAt`) are not standard LSP. monaco-languageclient's `LanguageClientWrapper` is a VS Code-style client; extra methods are possible the way `kul-lsp` already registers them (`LspService::build().custom_method(...)` in [`lib.rs`](../../crates/kul-lsp/src/lib.rs)), but they are extra client code either way. `node_at` itself is not an LSP request.

## 4. Cross-file rename: what a WASM op returns instead of a `WorkspaceEdit`

LSP rename, project-wide ([ADR-0015](../adr/0015-global-project-namespace.md)): collect `references_to` + decl `FileSpan`, group by `FileId`, emit

```text
WorkspaceEdit {
  changes: Some({
    Url(file_a) → [TextEdit { range: LineIndex.range(span), new_text }],
    Url(file_b) → [TextEdit { … }],
  })
}
```

([`rename.rs`](../../crates/kul-lsp/src/features/rename.rs); snapshot `snapshot_rename_workspace_edit`; test `rename_spans_every_project_file`). Validation failures are `RenameError` variants (not renameable, unresolved ref, invalid identifier, reserved keyword, collision), mapped to JSON-RPC `InvalidRequest` by the server ([`server.rs`](../../crates/kul-lsp/src/server.rs) `rename`). No-op (`new_name == current`) is an empty `WorkspaceEdit`.

A WASM op has no `Url` and no `LineIndex`. The identity it already uses for every other project-wide fact is `InputFile.name` + UTF-8 bytes ([`ExportedSpan`](../../crates/kul-core/src/export.rs); [`WasmInputFile.name`](../../crates/kul-wasm/src/lib.rs)). The language result of rename is the same list of `FileSpan`s plus `new_name`. Projected the way `check` already projects diagnostics, that is:

```text
{ file: string, byteStart: number, byteEnd: number, newText: string }[]
```

grouped by `file` if the host wants a map, or left flat. The host applies the ranges to the vault buffers it passed in. Failure stays a structured error arm (identifier / reserved / collision / unresolved / not-renameable) — the same `RenameError` cases, not a `WorkspaceEdit`. Prepare-rename is the ident `ExportedSpan` (or `null`), not `PrepareRenameResponse::Range`.

This is the shape #336 must name if it picks WASM ops. It is not a `WorkspaceEdit`. If #336 picks an LSP worker, the worker (Rust or JS) is the place that builds the `WorkspaceEdit` from those same spans.

## 5. Honest leftover architecture tradeoff for #336

#336 asks how completion, hover, go-to-definition, find-references, rename, and editor↔tree identity "actually run in the page." This research does not pick. The leftover is one cut:

**Does the browser editor consume Kul's language seams as stateless, name-and-bytes WASM operations (the shape `check` / `query*` already proved), or does it consume them as LSP (`Position` / `Url` / `WorkspaceEdit` / `publishDiagnostics`) through a worker language server?**

Facts that cut must sit on:

1. **The language work is already in `kul-core`** (`node_at`, `entity_reference`, `references_to`, `entity`, `format_source`, identifier rules) and in two `kul-lsp` modules that are language-aware but protocol-typed (hover Markdown, completion classifier). Protocol work (`LineIndex`, URIs, `WorkspaceEdit`, `publishDiagnostics`, stdio, disk overlay) is `kul-lsp` only (§1).
2. **`check` / `format` already prove the WASM op path and already diverge from LSP** on format-on-parse-error and URI-less spans (§2). A WASM rename returns name-and-bytes edits, not a `WorkspaceEdit` (§4). Completing the floor that way is more of ADR-0011's surface, plus extracting or copying hover/completion out of `kul-lsp` without taking a `kul-lsp` dependency.
3. **Today's `kul-lsp` is stdio + tokio + `kul-loader` disk.** tower-lsp `Server` is stdin/stdout (or TCP), not a worker ([`lib.rs`](../../crates/kul-lsp/src/lib.rs); [`Server`](https://docs.rs/tower-lsp/latest/tower_lsp/struct.Server.html)). Hosting it in a worker is an adapter rewrite, not a retarget (§3.2).
4. **Speaking LSP in the page is a documented third-party path** (monaco-languageclient worker connection + `vscode-languageserver/browser` `createConnection`) that still needs a Kul implementation behind the pipes — either WASM ops plus a JS LSP façade, or a new wasm language server (§3.3). It couples to an LSP-shaped editor stack (#334).
5. **Cache versus re-check.** LSP handlers read a cached `ResolvedDocument` ([ADR-0007](../adr/0007-resolved-document-owns-document.md)). WASM ops re-check ([ADR-0034](../adr/0034-query-transport-and-result-node-identity.md)). Completion is the place that tension is loudest. A stateful WASM handle is recorded in ADR-0034 as available and out of scope, not as chosen.
6. **`check` / `format` / `renderSvg` / `query*` stay in-page WASM** regardless. The cut is only the editor-intelligence floor. Two transports for two floors is allowed by the standing preference; one transport for everything is not required and is not forbidden.

#336 also owes the concrete surface (function names / LSP methods) and the rename return type. Those are consequences of the cut, not a second cut.

# In-browser editor stacks for a custom DSL (Monaco vs CodeMirror 6)

Retrieved 2026-09-17. Resolves [Research: in-browser editor stacks for a custom DSL (Monaco vs CodeMirror 6)](https://github.com/YashBhalodi/kul/issues/330). Does **not** pick a stack. The leftover tradeoff belongs to [Decide: in-browser editor stack](https://github.com/YashBhalodi/kul/issues/334).

## Question

Which in-browser editor, as of this retrieval, can carry Kul's editor floor without a VS Code host — diagnostics, completion, format, hover, go-to-definition, find-references, rename, and editor↔tree identity — over a custom language?

This note compares **Monaco** and **CodeMirror 6** from official docs and first-party APIs. A third stack is named only if a primary source makes it a real contender for that floor. Tokens, symbols, and code actions are out of scope.

Repo facts used as the floor's counterpart (not as editor documentation):

- The repo has no Monaco or CodeMirror dependency today.
- [`kul-lsp`](../../crates/kul-lsp/) already implements the language-intelligence half of the floor as LSP feature modules: diagnostics, completion, hover, definition, references, rename, formatting ([`crates/kul-lsp/src/features/mod.rs`](../../crates/kul-lsp/src/features/mod.rs); [architecture map](../architecture.md)).
- [`@kullang/wasm`](../../crates/kul-wasm/) exposes `check` (diagnostics), `format` (per-file string), `exportGraph`, `renderSvg`, and query operations — not hover, definition, references, or rename ([ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md); [`crates/kul-wasm/src/lib.rs`](../../crates/kul-wasm/src/lib.rs)).
- Cross-file identity in Kul is [`FileId`](../../CONTEXT.md) + [`FileSpan`](../../CONTEXT.md). Adapters map those onto editor URIs ([ADR-0014](../adr/0014-file-identity-and-per-file-namespaces.md), [ADR-0015](../adr/0015-global-project-namespace.md)).

## Versions consulted

| Artifact | Version / date | Source |
| --- | --- | --- |
| `monaco-editor` (npm latest) | 0.56.0, MIT, unpacked 97 911 464 bytes / 1909 files | [npm registry](https://registry.npmjs.org/monaco-editor/latest) retrieved 2026-09-17 |
| Monaco Editor README | `main` as of retrieval | [microsoft/monaco-editor README](https://github.com/microsoft/monaco-editor/blob/main/README.md) |
| Monaco public API | `monaco.d.ts` (the versioned surface) | [Monaco Editor API](https://microsoft.github.io/monaco-editor/typedoc/index.html), [languages namespace](https://microsoft.github.io/monaco-editor/typedoc/modules/editor_editor_api.languages.html) |
| `@vscode/monaco-lsp-client` (in-repo) | 0.1.0, README: alpha | [monaco-lsp-client/package.json](https://github.com/microsoft/monaco-editor/blob/main/monaco-lsp-client/package.json), [monaco-lsp-client/README.md](https://github.com/microsoft/monaco-editor/blob/main/monaco-lsp-client/README.md). **Not** on the npm registry (`@vscode/monaco-lsp-client` → HTTP 404 on 2026-09-17) |
| CodeMirror system guide / reference | current docs site | [System Guide](https://codemirror.net/docs/guide/), [Reference Manual](https://codemirror.net/docs/ref/) |
| `codemirror` (basic setup) | 6.0.2, MIT, unpacked 21 258 bytes | [npm registry](https://registry.npmjs.org/codemirror/latest) |
| `@codemirror/view` | 6.43.12, unpacked 1 256 059 bytes | [npm registry](https://registry.npmjs.org/@codemirror/view/latest) |
| `@codemirror/state` | 6.7.5, unpacked 440 230 bytes | [npm registry](https://registry.npmjs.org/@codemirror/state/latest) |
| `@codemirror/autocomplete` | 6.20.3, unpacked 256 089 bytes | [npm registry](https://registry.npmjs.org/@codemirror/autocomplete/latest) |
| `@codemirror/lint` | 6.9.7, unpacked 97 441 bytes | [npm registry](https://registry.npmjs.org/@codemirror/lint/latest) |
| `@codemirror/lsp-client` | 6.3.0, MIT, unpacked 291 612 bytes (published 2026-09-17) | [npm registry](https://registry.npmjs.org/@codemirror/lsp-client/latest) |
| TanStack Start bundler | Vite or Rsbuild | [Build a Project from Scratch](https://tanstack.com/start/latest/docs/framework/react/build-from-scratch) |

Unpacked npm size is the published package on disk, not a production bundle. Monaco's official ESM guide documents a "webpack-small" sample for shipping a subset ([integrate-esm.md](https://github.com/microsoft/monaco-editor/blob/main/docs/integrate-esm.md)). CodeMirror is published as separately installable packages ([Reference Manual](https://codemirror.net/docs/ref/): "published as a set of NPM packages under the `@codemirror` scope").

## How each stack thinks

**Monaco** is "the fully featured code editor from VS Code", generated from VS Code's sources with shims so it runs in a browser ([README](https://github.com/microsoft/monaco-editor/blob/main/README.md)). A **model** is a file (text + language + undo); a **URI** identifies the model; an **editor** is a view of one model; **providers** supply smart features and "often map to" LSP methods but are "not the same as" LSP ([README — Concepts](https://github.com/microsoft/monaco-editor/blob/main/README.md)). VS Code extensions do not run in Monaco ([README FAQ](https://github.com/microsoft/monaco-editor/blob/main/README.md)).

**CodeMirror 6** is a modular editor: immutable `EditorState` + `EditorView`, configured by extensions. Official core packages do not include a language server. The first-party [`@codemirror/lsp-client`](https://www.npmjs.com/package/@codemirror/lsp-client) package adds an `LSPClient` that talks JSON-RPC over a host-supplied `Transport` ([Reference — Client](https://codemirror.net/docs/ref/), [package README](https://www.npmjs.com/package/@codemirror/lsp-client)). The packages are ES modules; the system guide names Vite as a bundler to look at ([System Guide](https://codemirror.net/docs/guide/)).

Neither stack ships a file tree. Neither knows Kul. Intelligence is whatever the host registers (Monaco providers / CodeMirror extensions) or whatever language server the host connects (CodeMirror's published client; Monaco's in-repo alpha client).

## Floor item by item

### Diagnostics / markers

**Monaco — built-in marker surface; host pushes markers.** `monaco.editor.setModelMarkers(model, owner, markers)` sets problems on one model. `IMarkerData` carries `severity`, `message`, 1-based `startLineNumber` / `startColumn` / `endLineNumber` / `endColumn`, optional `code`, `source`, `tags`, and `relatedInformation` (each related item has its own `resource: Uri`) ([monaco.d.ts `setModelMarkers` / `IMarkerData` / `IRelatedInformation`](https://microsoft.github.io/monaco-editor/typedoc/index.html)). `getModelMarkers`, `removeAllMarkers`, and `onDidChangeMarkers` are the rest of the surface. There is no `registerDiagnosticProvider`. The host computes diagnostics (WASM `check`, an LSP `publishDiagnostics` adapter, or anything else) and writes them onto models.

**CodeMirror 6 — first-class lint package; host supplies a source or pushes.** `@codemirror/lint` defines `Diagnostic` (`from`, `to`, `severity: "error" | "hint" | "info" | "warning"`, `message`, optional `source`, `actions`) ([Reference — @codemirror/lint](https://codemirror.net/docs/ref/), [Lint example](https://codemirror.net/examples/lint/)). Two intake paths:

- Pull: `linter(source)` calls a `LintSource` when the editor is idle (default delay 750 ms).
- Push: `setDiagnostics(state, diagnostics)` returns a transaction that updates the current set.

`lintGutter` and `openLintPanel` are optional UI. The library "does not come with a collection of lint sources" ([Lint example](https://codemirror.net/examples/lint/)). Cross-file diagnostics via LSP use `serverDiagnostics()`, which "makes the client receive diagnostics from the server, and show them via the CodeMirror linter" ([Changelog](https://codemirror.net/docs/changelog/), [Reference — serverDiagnostics](https://codemirror.net/docs/ref/)).

**Gap vs Kul.** Kul diagnostics carry a code, a primary `FileSpan`, and related spans that may sit in a sibling file ([CONTEXT.md — Diagnostic](../../CONTEXT.md)). Monaco's `IRelatedInformation.resource` can name another model URI. CodeMirror's core `Diagnostic` is one document; sibling-file related info is an LSP-client / host concern.

### Completion

**Monaco — built-in suggest UI; host implements `CompletionItemProvider`.** `monaco.languages.registerCompletionItemProvider(languageSelector, provider)` ([API](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerCompletionItemProvider.html)). `provideCompletionItems(model, position, context, token)` returns the list; optional `resolveCompletionItem` fills documentation later ([`CompletionItemProvider`](https://microsoft.github.io/monaco-editor/typedoc/index.html)). The homepage: "write your own completion providers in JavaScript" ([Monaco homepage](https://microsoft.github.io/monaco-editor/)).

**CodeMirror 6 — first-class autocomplete package; host implements `CompletionSource`.** `autocompletion` is a core extension ([List of core extensions](https://codemirror.net/docs/extensions/), [Autocompletion example](https://codemirror.net/examples/autocompletion/)). A `CompletionSource` is `(context) => CompletionResult | Promise<…> | null` ([Reference](https://codemirror.net/docs/ref/)). Language packages register a source on `Language.data` under `"autocomplete"` ([Language package example](https://codemirror.net/examples/lang-package/)). Via LSP: `serverCompletion` / `serverCompletionSource` ([Reference — LSP extensions](https://codemirror.net/docs/ref/)).

### Hover

**Monaco — built-in hover widget; host implements `HoverProvider`.** `registerHoverProvider` ([API](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerHoverProvider.html)). `provideHover` returns `contents: IMarkdownString[]` and an optional `range`. Multiple hovers at the same position are merged ([`HoverProvider`](https://microsoft.github.io/monaco-editor/typedoc/index.html)).

**CodeMirror 6 — first-class `hoverTooltip`; host implements `HoverTooltipSource`.** The callback receives `(view, pos, side)` and returns a `Tooltip` or a promise ([Reference — hoverTooltip](https://codemirror.net/docs/ref/)). Via LSP: `hoverTooltips()` "queries the language server for hover tooltips" and renders the result ([Reference](https://codemirror.net/docs/ref/)). The LSP client warns that server Markdown is rendered to HTML and is an XSS channel unless `sanitizeHTML` is supplied ([`LSPClientConfig.sanitizeHTML`](https://codemirror.net/docs/ref/)).

### Go-to-definition

**Monaco — built-in go-to and peek; host implements `DefinitionProvider`.** `registerDefinitionProvider` is "used by e.g. go to definition" ([API](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerDefinitionProvider.html)). `DefinitionProvider` "defines the contract between extensions and the go to definition and peek definition features"; `provideDefinition` returns a `Definition` = `Location | Location[] | LocationLink[]`, and each `Location` has `uri` + `range` ([monaco.d.ts](https://microsoft.github.io/monaco-editor/typedoc/index.html)). Cross-file jumps are URI-shaped. Revealing another file is `editor.setModel(getModel(uri))` plus a reveal call (the editor is a view of one model; [README — Editors](https://github.com/microsoft/monaco-editor/blob/main/README.md), [`setModel`](https://microsoft.github.io/monaco-editor/typedoc/index.html)).

**CodeMirror 6 — not a core command; first-party only on the LSP client.** The reference's `jumpToDefinition` lives under the LSP client extensions: "Jump to the definition of the symbol at the cursor. To support cross-file jumps, you'll need to implement `Workspace.displayFile`." F12 is bound by `jumpToDefinitionKeymap` ([Reference](https://codemirror.net/docs/ref/)). Without that package, the host writes its own command that looks up a span and calls `setState` / `dispatch`.

### Find-references

**Monaco — built-in reference search; host implements `ReferenceProvider`.** `registerReferenceProvider` is "used by e.g. reference search" ([API](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerReferenceProvider.html)). `provideReferences` returns `Location[]` (each `uri` + `range`) and receives `ReferenceContext.includeDeclaration` ([monaco.d.ts](https://microsoft.github.io/monaco-editor/typedoc/index.html)).

**CodeMirror 6 — not a core command; first-party only on the LSP client.** `findReferences` "asks the server to locate all references… show them as a list in a panel." Shift-F12 via `findReferencesKeymap` ([Reference](https://codemirror.net/docs/ref/)). Without the LSP client, the host owns lookup and panel.

### Rename

**Monaco — built-in rename UI; host returns a `WorkspaceEdit`, not a single replace.** `registerRenameProvider` is "used by e.g. rename symbol" ([API](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerRenameProvider.html)). `RenameProvider.provideRenameEdits(model, position, newName, token)` returns a `WorkspaceEdit`; optional `resolveRenameLocation` prepares the range ([monaco.d.ts](https://microsoft.github.io/monaco-editor/typedoc/index.html)). `WorkspaceEdit.edits` is an array of `IWorkspaceTextEdit` (`resource: Uri`, `textEdit`, `versionId`) and `IWorkspaceFileEdit` (optional `oldResource` / `newResource`). Cross-file rename is therefore a **protocol**: the provider names every URI to rewrite. Those URIs only apply if the corresponding models exist (`createModel` / `getModel`; [README — Models / URIs](https://github.com/microsoft/monaco-editor/blob/main/README.md)). `ITextModel.applyEdits` exists for host-applied edits that skip the undo stack; `pushEditOperations` is the undo-safe path ([monaco.d.ts](https://microsoft.github.io/monaco-editor/typedoc/index.html)).

**CodeMirror 6 — not a core command; first-party only on the LSP client.** `renameSymbol` "prompts the user for a new name… and asks the language server to perform a rename." The docs are explicit: "this may affect files other than the one loaded into this view. See the `Workspace.updateFile` method." F2 via `renameKeymap` ([Reference](https://codemirror.net/docs/ref/)). Without the LSP client, a host `dispatch({changes: {from, to, insert}})` is a **text replace in one document**, not project-wide rename.

Kul's rename is project-wide over `references_to` ([CONTEXT.md — ResolvedDocument](../../CONTEXT.md); [`crates/kul-lsp/src/features/rename.rs`](../../crates/kul-lsp/src/features/rename.rs)). A host that only replaces the word under the cursor does not meet the floor.

### Document formatting

**Monaco — built-in format command; host implements `DocumentFormattingEditProvider`.** `registerDocumentFormattingEditProvider` "registers a formatter that can handle only entire models"; `provideDocumentFormattingEdits` returns `TextEdit[]` for that model ([API](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerDocumentFormattingEditProvider.html)). Range and on-type formatters are separate registrations (out of the floor's "format" item unless a later decide ticket expands it).

**CodeMirror 6 — no core formatter; first-party format is an LSP command.** `formatDocument` "asks the language server to reformat the document, and then applies the changes it returns." Shift-Alt-f via `formatKeymap` ([Reference](https://codemirror.net/docs/ref/)). Without the LSP client, the host calls whatever formatter it has (Kul: WASM `format` → string; [ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md)) and replaces the document through a transaction or `setState`.

### Editor↔tree identity

Neither editor ships a file tree. Both expose the hooks a host tree needs.

**Monaco.** Each model has `uri` ([`ITextModel.uri`](https://microsoft.github.io/monaco-editor/typedoc/index.html)). `createModel(value, language?, uri?)` creates it; `getModel(uri)` / `getModels()` look it up; two models cannot share a URI ([README — URIs](https://github.com/microsoft/monaco-editor/blob/main/README.md)). The editor emits `onDidChangeCursorPosition`, `onDidChangeCursorSelection`, and `onDidChangeModel` ([monaco.d.ts](https://microsoft.github.io/monaco-editor/typedoc/index.html)). Tree → editor is `setModel` + reveal. Editor → tree is those events plus the current model's `uri`.

**CodeMirror 6.** One `EditorView` holds one `EditorState` / document. `EditorView.setState(newState)` resets the view for a new document; the guide says to create a new state when loading a new document so undo history does not leak ([System Guide](https://codemirror.net/docs/guide/), [Reference — setState](https://codemirror.net/docs/ref/)). `EditorView.updateListener` fires on every view update ([Reference](https://codemirror.net/docs/ref/)). With the LSP client, identity is `WorkspaceFile.uri`; `Workspace.displayFile(uri)` is what the client calls when it must show a file that is not in the current view ([Reference — Workspace](https://codemirror.net/docs/ref/)).

The host owns the tree and the mapping `tree node ↔ URI ↔ FileId`. Kul already has the cursor-shaped query (`Node::entity_reference`, `Cursor::entity`) that turns a URI + offset into a person/marriage ([CONTEXT.md](../../CONTEXT.md)). That query is independent of which editor widget is on screen.

### Custom language registration (needed so the floor attaches)

**Monaco.** `monaco.languages.register({ id, … })` registers the language; providers take a `LanguageSelector` ([`register`](https://microsoft.github.io/monaco-editor/typedoc/index.html)). Tokenizer / Monarch APIs exist and are out of this floor (tokens are out of scope).

**CodeMirror 6.** A language package wraps a parser in a `Language`, optionally attaches `data` (e.g. autocomplete), and exports a `LanguageSupport` ([Language package example](https://codemirror.net/examples/lang-package/)). Highlighting / Lezer are out of this floor.

## Built-in vs host, and whether the host is an LSP client

| Responsibility | Monaco | CodeMirror 6 |
| --- | --- | --- |
| Text editing, selection, undo | Built-in (`ITextModel` / editor) | Built-in (`EditorState` / `EditorView`) |
| Marker / lint **UI** | Built-in once markers are set | Built-in once `@codemirror/lint` is added |
| Completion / hover **UI** | Built-in once providers are registered | Built-in once the matching extensions are added |
| Go-to / peek definition **UI** | Built-in once `DefinitionProvider` is registered | Built-in **only** via `@codemirror/lsp-client` (`jumpToDefinition`) |
| References **UI** | Built-in once `ReferenceProvider` is registered | Built-in **only** via `@codemirror/lsp-client` (`findReferences` panel) |
| Rename **UI** | Built-in once `RenameProvider` is registered | Built-in **only** via `@codemirror/lsp-client` (`renameSymbol`) |
| Format **command** | Built-in once a formatting provider is registered | Built-in **only** via `@codemirror/lsp-client` (`formatDocument`) |
| Language intelligence | Host implements providers, **or** wires an LSP client that implements those providers | Host implements extension sources, **or** connects `@codemirror/lsp-client` |
| First-party LSP client | In-repo `@vscode/monaco-lsp-client` 0.1.0, README: "alpha… might contain many bugs"; **not published on npm** as of this retrieval ([package.json](https://github.com/microsoft/monaco-editor/blob/main/monaco-lsp-client/package.json), [README](https://github.com/microsoft/monaco-editor/blob/main/monaco-lsp-client/README.md), registry 404) | Published `@codemirror/lsp-client` 6.3.0 ([npm](https://www.npmjs.com/package/@codemirror/lsp-client), [Reference](https://codemirror.net/docs/ref/)) |
| File tree | Host | Host |
| Transport to a language server | Host (no published first-party client) | Host implements `Transport.send` / `subscribe` / `unsubscribe` ([Reference — Transport](https://codemirror.net/docs/ref/); [package README](https://www.npmjs.com/package/@codemirror/lsp-client)) |

Monaco's README states providers "often map to" LSP but are not LSP, and that a VS Code extension will not run in the browser editor; a parenthetical notes that an extension that is "fully based on the LSP" with a JavaScript language server "would be possible" ([README FAQ](https://github.com/microsoft/monaco-editor/blob/main/README.md)). That is a possibility statement, not a shipped client.

CodeMirror's published client **is** an LSP client. `languageServerSupport(client, uri, languageID?)` "enables the LSP plugin as well as LSP based autocompletion, hover tooltips, and signature help, along with the keymaps for reformatting, renaming symbols, jumping to definition, and finding references" (deprecated in favor of composing `LSPClient.plugin` + `languageServerExtensions`) ([Reference](https://codemirror.net/docs/ref/)).

The host is **not** required to be an LSP client on either stack: both accept host-implemented functions. The host **is** expected to be an LSP client if it wants the first-party go-to / references / rename / format **protocol** on CodeMirror, or if it uses Monaco's unpublished alpha client. Kul already has the server (`kul-lsp`). Kul's WASM surface cannot answer hover / definition / references / rename today ([ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md)).

## Multi-file: one editor, many models, host-owned tree

**Monaco's first-party story is one editor, many models.**

- A model "represents a file that has been opened" and "could represent a file that exists on a file system, but it doesn't have to" ([README — Models](https://github.com/microsoft/monaco-editor/blob/main/README.md)).
- "Ideally when you represent content in Monaco editor, you should think of a virtual file system that matches the files your users are editing" ([README — URIs](https://github.com/microsoft/monaco-editor/blob/main/README.md)).
- `createModel` / `getModel` / `getModels` / `onDidCreateModel` / `onWillDisposeModel` are the registry ([monaco.d.ts](https://microsoft.github.io/monaco-editor/typedoc/index.html)).
- `editor.setModel(model | null)` attaches one model to the visible editor; the previous model is not destroyed if the host created it ([`setModel`](https://microsoft.github.io/monaco-editor/typedoc/index.html)).
- Providers receive the active `ITextModel` and return `Location`s / `WorkspaceEdit`s keyed by `Uri`. The host must have created the target models before a cross-file jump or rename can land.

There is no Monaco API for a file-tree widget. The tree is host chrome that calls `setModel(getModel(uri))`.

**CodeMirror 6's first-party story is one view, one state; many files are a workspace the host implements.**

- Loading a new document: create a new `EditorState` and `setState` ([System Guide](https://codemirror.net/docs/guide/), [Reference — setState](https://codemirror.net/docs/ref/)).
- The LSP client's default workspace "only opens files that have an active editor, and only allows one editor per file" ([`LSPClientConfig.workspace`](https://codemirror.net/docs/ref/)).
- A custom `Workspace` is the documented way to "provide more control over the way files are loaded and managed": `files`, `openFile`, `closeFile`, `syncFiles`, `requestFile`, `updateFile`, `displayFile` ([Reference — Workspace](https://codemirror.net/docs/ref/)).
- Cross-file go-to requires `displayFile`. Cross-file rename goes through `updateFile`.

There is no CodeMirror API for a file-tree widget. The tree is host chrome that constructs or selects an `EditorState` (and, if using the LSP client, opens the matching `WorkspaceFile`).

Kul's project is already multi-file: one `Document` of `KulFile`s, project-wide ids, `FileSpan`s that may point at a sibling ([ADR-0014](../adr/0014-file-identity-and-per-file-namespaces.md), [ADR-0015](../adr/0015-global-project-namespace.md)). Any stack that only keeps the active buffer will drop cross-file definition, references, and rename on the floor.

## Weight, Vite / TanStack Start, WASM, license

### Weight

| Package | Unpacked size (npm, 2026-09-17) |
| --- | --- |
| `monaco-editor@0.56.0` | 97 911 464 bytes, 1909 files |
| `@codemirror/view@6.43.12` | 1 256 059 bytes |
| `@codemirror/state@6.7.5` | 440 230 bytes |
| `@codemirror/autocomplete@6.20.3` | 256 089 bytes |
| `@codemirror/lint@6.9.7` | 97 441 bytes |
| `@codemirror/lsp-client@6.3.0` | 291 612 bytes |
| `codemirror@6.0.2` (basic-setup re-export) | 21 258 bytes |

Monaco's published tarball includes the editor, built-in language services, and workers. Official integration still requires wiring `editor.worker` plus per-language workers if those services are used ([README FAQ — web workers](https://github.com/microsoft/monaco-editor/blob/main/README.md), [integrate-esm.md](https://github.com/microsoft/monaco-editor/blob/main/docs/integrate-esm.md)). A custom-language-only host can ignore the TS/JSON/HTML/CSS workers and still must satisfy `MonacoEnvironment.getWorker` / `getWorkerUrl` for the editor worker.

CodeMirror's official posture is "install and import the pieces you need" ([System Guide](https://codemirror.net/docs/guide/)). The floor's non-LSP pieces are `@codemirror/lint` + `@codemirror/autocomplete` + `hoverTooltip` from `@codemirror/view`. The floor's protocol pieces add `@codemirror/lsp-client`.

### Vite / TanStack Start

**CodeMirror** names Vite in the system guide: packages are ES modules and "it is not currently practical to run the library without some kind of bundler… I recommend looking into rollup or Vite" ([System Guide](https://codemirror.net/docs/guide/)).

**Monaco** has a first-party Vite section: implement `MonacoEnvironment.getWorker` (not `getWorkerUrl`) and load `*.worker?worker` modules ([integrate-esm.md — Using Vite](https://github.com/microsoft/monaco-editor/blob/main/docs/integrate-esm.md)). The repo also ships [`samples/browser-esm-vite-react`](https://github.com/microsoft/monaco-editor/blob/main/samples/browser-esm-vite-react/src/userWorker.ts) using Vite `?worker` imports for the editor / json / css / html / ts workers.

**TanStack Start** "supports Vite or Rsbuild as the build tool" and documents a Vite plugin (`tanstackStart` from `@tanstack/react-start/plugin/vite`) ([Build from scratch](https://tanstack.com/start/latest/docs/framework/react/build-from-scratch)). There is no first-party TanStack documentation for Monaco or CodeMirror. A Start+Vite host can follow each editor's official Vite recipe; a Start+Rsbuild host has no first-party editor recipe from either project.

### WASM coexistence

Neither editor's official docs describe a conflict with an application-owned WASM module.

- Monaco language services "create web workers to compute heavy stuff outside of the UI thread" ([README FAQ](https://github.com/microsoft/monaco-editor/blob/main/README.md)). `monaco.editor.createWebWorker` builds a worker with model syncing ([API](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.editor.createWebWorker.html)). Custom-language providers run in the host's JS unless the host puts them in a worker. Official docs do not mention WASM.
- `@codemirror/lsp-client` states there are "various ways to run a language server and connect it to a web page": proxy over a WebSocket, **or**, "if it is written in JavaScript or can be compiled to WASM, run it directly in the client." The package talks JSON messages over a `Transport`; "responsibility for how to actually talk to the server… is left to the code that implements the transport" ([package README](https://www.npmjs.com/package/@codemirror/lsp-client)).

Kul already ships `@kullang/wasm` as a build asset for the existing preview ([ADR-0040](../adr/0040-engine-provenance-a-build-asset-not-a-registry-dependency.md)). That module does not speak LSP and does not implement hover / definition / references / rename.

### License

Both are MIT.

- Monaco: [LICENSE.txt](https://github.com/microsoft/monaco-editor/blob/main/LICENSE.txt), Copyright Microsoft Corporation; npm `license: MIT`.
- CodeMirror: [codemirror/dev LICENSE](https://github.com/codemirror/dev/blob/master/LICENSE), Copyright Marijn Haverbeke, Adrian Heine, and others; `@codemirror/*` packages declare MIT on npm.

### Mobile (not a floor item; recorded because the primary source is explicit)

Monaco: "Is the editor supported in mobile browsers or mobile web app frameworks? **No.**" ([README FAQ](https://github.com/microsoft/monaco-editor/blob/main/README.md); same claim on the [homepage](https://microsoft.github.io/monaco-editor/)). CodeMirror's tooltip docs mention an iOS `position: fixed` workaround ([Reference — tooltips](https://codemirror.net/docs/ref/)); they do not declare a mobile support bar.

## Honest gaps versus the floor

1. **Neither editor carries Kul.** A custom language gets a text box until the host registers providers / extensions or connects `kul-lsp` (or grows a new WASM language-intelligence surface). The current WASM surface covers diagnostics and format only ([ADR-0011](../adr/0011-wasm-surface-three-shapes-no-wrappers.md)).

2. **Rename is a workspace protocol, not a buffer replace.** Monaco's `RenameProvider` returns `WorkspaceEdit` keyed by `Uri`. CodeMirror's first-party rename asks an LSP server and may call `Workspace.updateFile` on other files. A host that `applyEdits`s / `dispatch`es only the current selection has not implemented the floor.

3. **Go-to-definition and find-references are URI lists.** Both stacks' first-party results are multi-location. CodeMirror's first-party implementation exists only on the LSP client and needs `Workspace.displayFile` for a sibling `.kul` file. Monaco needs the target `ITextModel` already created.

4. **File tree and editor↔tree identity are 100% host.** The editors give URI + cursor events. Kul gives `FileId` / `FileSpan` / `EntityNode`. The mapping layer is new work on either pick.

5. **CodeMirror core does not implement four of the eight floor items as language features.** Lint, completion, and hover are core packages. Definition, references, rename, and format-as-protocol are `@codemirror/lsp-client` (or host-written substitutes). Treating "CodeMirror 6" as sufficient without deciding the LSP-client vs hand-rolled-command question leaves those four items unspecified.

6. **Monaco's first-party LSP client is not a product you can install today.** `@vscode/monaco-lsp-client` lives in the monaco-editor repo, is labeled alpha, and was not on the npm registry at retrieval. The supported public surface is `monaco.languages.register*Provider`. Using LSP against `kul-lsp` from Monaco means a host-written client (or that alpha tree), not a documented published package.

7. **Encoding.** CodeMirror positions are UTF-16 code units ([System Guide](https://codemirror.net/docs/guide/)). Monaco positions are 1-based line/column in the VS Code tradition (`IMarkerData`, `Position` in monaco.d.ts). Kul spans are UTF-8 byte offsets on a `FileSpan` ([CONTEXT.md](../../CONTEXT.md)). `kul-lsp` already performs that conversion for LSP; a non-LSP host must do it again.

8. **Monaco is not a mobile editor** ([README FAQ](https://github.com/microsoft/monaco-editor/blob/main/README.md)). Irrelevant if the surface is desktop-web only; blocking if the decide ticket later includes phones.

9. **Weight and workers.** Monaco's published package is ~93 MB unpacked and requires worker wiring on Vite ([integrate-esm.md](https://github.com/microsoft/monaco-editor/blob/main/docs/integrate-esm.md)). CodeMirror's floor-relevant packages sum to a few megabytes unpacked and are ESM-native. That is a packaging fact, not a capability fact.

## Third stack

No third first-party stack is named. vscode.dev / VS Code for the Web is a VS Code **host**, which this question excludes. Community wrappers (TypeFox `monaco-languageclient`, third-party CodeMirror LSP adapters predating `@codemirror/lsp-client`) are not first-party and are not in scope.

## Leftover tradeoff (do not decide here)

[Decide: in-browser editor stack](https://github.com/YashBhalodi/kul/issues/334) still has to name a stack and the host responsibilities it forces. Facts this note leaves open on purpose:

- Provider-shaped host (Monaco `register*Provider`, or CodeMirror extensions for lint/complete/hover plus host commands for the rest) versus embedding a first-party LSP client (`@codemirror/lsp-client` published; `@vscode/monaco-lsp-client` alpha / unpublished) in front of `kul-lsp`.
- Whether the browser language-intelligence process is `kul-lsp` over a `Transport`, or an expanded `@kullang/wasm` surface. Today's WASM module cannot answer four of the eight floor items.
- Weight, worker wiring, and mobile non-support versus modular ESM.
- How much of the multi-file workspace the host writes (Monaco model registry vs CodeMirror `Workspace` + `displayFile`).

## Sources

- [Monaco Editor homepage](https://microsoft.github.io/monaco-editor/)
- [Monaco Editor README](https://github.com/microsoft/monaco-editor/blob/main/README.md)
- [Monaco LICENSE.txt](https://github.com/microsoft/monaco-editor/blob/main/LICENSE.txt)
- [Monaco ESM integration (includes Vite)](https://github.com/microsoft/monaco-editor/blob/main/docs/integrate-esm.md)
- [Monaco Vite React worker sample](https://github.com/microsoft/monaco-editor/blob/main/samples/browser-esm-vite-react/src/userWorker.ts)
- [Monaco Editor API (typedoc)](https://microsoft.github.io/monaco-editor/typedoc/index.html)
- [monaco.languages](https://microsoft.github.io/monaco-editor/typedoc/modules/editor_editor_api.languages.html)
- [registerCompletionItemProvider](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerCompletionItemProvider.html)
- [registerHoverProvider](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerHoverProvider.html)
- [registerDefinitionProvider](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerDefinitionProvider.html)
- [registerReferenceProvider](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerReferenceProvider.html)
- [registerRenameProvider](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerRenameProvider.html)
- [registerDocumentFormattingEditProvider](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.languages.registerDocumentFormattingEditProvider.html)
- [createModel](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.editor.createModel.html)
- [createWebWorker](https://microsoft.github.io/monaco-editor/typedoc/functions/editor_editor_api.editor.createWebWorker.html)
- [monaco-lsp-client README](https://github.com/microsoft/monaco-editor/blob/main/monaco-lsp-client/README.md)
- [monaco-lsp-client package.json](https://github.com/microsoft/monaco-editor/blob/main/monaco-lsp-client/package.json)
- [npm monaco-editor](https://registry.npmjs.org/monaco-editor/latest)
- [CodeMirror docs index](https://codemirror.net/docs/)
- [CodeMirror System Guide](https://codemirror.net/docs/guide/)
- [CodeMirror Reference Manual](https://codemirror.net/docs/ref/)
- [CodeMirror core extensions](https://codemirror.net/docs/extensions/)
- [CodeMirror changelog (LSPClient)](https://codemirror.net/docs/changelog/)
- [CodeMirror lint example](https://codemirror.net/examples/lint/)
- [CodeMirror autocompletion example](https://codemirror.net/examples/autocompletion/)
- [CodeMirror language package example](https://codemirror.net/examples/lang-package/)
- [CodeMirror LICENSE](https://github.com/codemirror/dev/blob/master/LICENSE)
- [npm @codemirror/lsp-client](https://www.npmjs.com/package/@codemirror/lsp-client)
- [TanStack Start — build from scratch](https://tanstack.com/start/latest/docs/framework/react/build-from-scratch)

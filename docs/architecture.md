# Architecture

The implementation map for KulLang. Read this when you need to make a change and don't yet know where to put it.

For domain vocabulary (Person, Marriage, Birth, ResolvedDocument, etc.), see [`CONTEXT.md`](../CONTEXT.md). For the test layout, see [`testing.md`](./testing.md). For ADRs that explain *why* a particular shape was chosen, see [`adr/`](./adr/).

## Vocabulary

This document uses the architecture words consistently:

- **Module** — anything with an interface and an implementation: a function, a type, a Rust module, a crate. The unit of "this thing has a public face and an internal one."
- **Interface** — everything a caller must know: types, invariants, error modes, ordering, configuration. Not just the function signature.
- **Implementation** — the code inside.
- **Seam** — where an interface lives. A place behaviour can be altered without editing in place. `ResolvedDocument` is a seam; the LSP feature modules are adapters at the editor seam.
- **Depth** — leverage at the interface. A small interface that hides a lot of behaviour is **deep**; an interface nearly as complex as its implementation is **shallow**. The validator's `Diagnostic` type is deep; the LSP `convert::to_lsp_diagnostic` adapter is appropriately shallow.
- **Deletion test** — imagine deleting a module. If complexity vanishes, it was a pass-through. If complexity reappears across N callers, it was earning its keep.

## Pipeline

A project's manifest YAML plus its [`InputFile`](../CONTEXT.md#kulfile)s flow through the toolchain like this:

```
(manifest_name, manifest_yaml, &[InputFile])
  │
  ▼  manifest::validate    produces typed Manifest + KUL-Mxx diagnostics
  │                        (anchored at FileId::MANIFEST in kul.yml)
  │
  ▼  lexer.rs              produces tokens per .kul file
  │
  ▼  parser.rs             produces statements per .kul file (KulFile)
  │
  ▼  ast::Document         multi-file container the rest of the pipeline
  │                        operates on (manifest at FileId(0); .kul files
  │                        at FileId(1..))
  │
  ▼  semantic.rs           produces ResolvedDocument (project-wide id index)
  │                        emits rule 01 diagnostics inline (duplicate ids,
  │                        scoped project-wide per ADR-0015)
  │
  ▼  validator.rs          runs rules 02–12 per file, then rule 13 once
  │                        across the project; rule 13 delegates to cycles.rs
  │                        which walks the project-wide parent graph
  │
  ▼  CheckResult { resolved, diagnostics, manifest }
```

`kul_core::check(manifest_name, manifest_yaml, &[InputFile])` is the single entry point that runs the whole pipeline. The CLI calls it once per project: each subcommand is CWD-rooted (no positional `<file>`), discovers the manifest and every sibling `.kul` via the shared `kul-loader` crate, and hands the bytes off in one call. The LSP calls it once per project (not per URI — issue #85): `did_open` discovers the project from the opened URI's directory, reads every sibling `.kul` off disk, and feeds the whole set into one `check`; subsequent `did_change` and `did_close` mutate the per-URI overlay and re-run `check` for the same project. The WASM bridge calls `check_with_manifest` with a typed manifest the JS host constructed and synthesizes a placeholder `kul.yml` body so any `.kul`-side diagnostic into the manifest still has bytes to anchor at. The pipeline operates on `Document.kul_files` with `N>=1` consumers (per [ADR-0015](./adr/0015-global-project-namespace.md)): the resolver builds one project-wide id index, the validator threads file iteration through it, and R01/R02/R13 honour the global namespace.

The shape is deliberately linear. Each pass produces a strictly richer artifact; nothing earlier in the pipeline ever consults something later. This is why:

- The lexer doesn't know about IDs.
- The parser doesn't know which IDs are declared.
- The validator never reaches into raw `KulFile.statements` — it queries through `ResolvedDocument` (per [ADR-0001](./adr/0001-resolved-document-as-query-seam.md)). All thirteen spec rules (R01 lives inline in `semantic::resolve` because it's a property of insertion order; R02–R13 live in `validator.rs`).

## Crate map

```
kul-core   ── library (no_std-friendly intent, but uses std for now)
              the entire pipeline lives here. Public surface is the
              CheckResult API plus the AST types, ResolvedDocument
              query methods, the formatter, and the export module
              (kinship-native + cytoscape projections).

kul-loader ── library
              Shared filesystem entry point: given a project-root path,
              encapsulates kul.yml discovery, *.kul enumeration (flat
              directory per ADR-0015), and IO error handling once. Two
              entry points over the same discovery rule: `load` (strict;
              returns typed ProjectLoadError, used by the CLI) and
              `discover` (lenient; swallows missing-manifest / unreadable
              files into empty results, used by the LSP). Both share one
              internal directory-walk helper so the discovery rule lives
              in exactly one place. Lives outside kul-core (which forbids
              filesystem IO) and outside kul-cli (which already depends
              on kul-lsp).

kul-cli    ── thin binary `kul`
              Four CWD-rooted subcommands: `validate` (renders
              diagnostics with miette), `format` (canonicalize per
              ADR-0004), `export` (project to JSON via
              kul_core::export), and `lsp` (delegates to kul_lsp::run).
              `validate`, `format`, and `export` take no positional
              file argument — each loads the project from CWD via
              `kul-loader`. Owns argument parsing and human/JSON
              output formatting.

kul-lsp    ── library + binary `kul-lsp`
              tower-lsp Backend implementation. Owns the project-keyed
              cache (one cached check per project root, not per URI),
              implements feature modules (hover, definition, completion,
              diagnostics, export, …), translates kul-core types to LSP
              types. Cross-file features (definition, references,
              rename, completion) are project-wide per ADR-0015;
              diagnostics broadcast to every project file on every
              update so the Problems pane reflects project-wide health.
              Never re-implements pipeline logic — only adapts. Custom
              requests (e.g. `kul/export`) are registered via
              `LspService::build().custom_method(...)` in `lib.rs`.

kul-layout ── library
              Positioning pass for the canonical UI pattern. Public
              surface is one function: `layout(&RenderShape,
              &LayoutConfig) -> PositionedShape`. Two deep modules:
              `walker` (Buchheim O(n) Reingold–Tilford–Walker port,
              with sibling-subtree collision avoidance) and `adapter`
              (canonical-pattern wrapper — thick marriage edges
              between spouses, ghost slots per current-intimacy placement, generation rows,
              orthogonal right-angle edge routing). `PositionedShape`
              is an internal Rust seam — not Serialize, not
              schema-versioned (ADR-0016). Depends on kul-render.

kul-svg    ── library
              Theme-agnostic SVG emitter. Public surface is one
              function: `render(&PositionedShape, &ThemeConfig) ->
              String`. Emits SVG with semantic CSS classes
              (`kul-card`, `kul-edge--marriage`, `kul-edge--birth`,
              etc.) and no inline colours; theming is a per-surface
              stylesheet concern (ADR-0016). SVG-only forever —
              downstream consumers convert to raster via standard
              tools. Depends on kul-layout and kul-render.

kul-wasm   ── library (cdylib + rlib), published as `@kullang/wasm`
              wasm-bindgen adapter over kul-core. Three exposed
              functions — `check`, `exportGraph`, `format` — each a
              two-or-three-line wrapper around the matching kul-core
              deep module, plus version-metadata getters. Surface
              shape is settled in ADR-0011; TypeScript types are
              derived via Tsify, committed, and diffed in CI per
              ADR-0012. Single ESM `--target bundler` build for
              modern bundlers (Vite, Webpack, Next.js, etc.).
```

The dependency graph is unidirectional: `kul-cli → kul-lsp → kul-core`, `kul-cli → kul-loader → kul-core`, `kul-cli → kul-core`, `kul-wasm → kul-core`, and `kul-lsp → kul-svg → kul-layout → kul-render → kul-core` for the visual-rendering pipeline. Nothing depends on the CLI; nothing in core depends on the LSP, the loader, the visual-rendering crates, or the WASM crate. The loader sits below both `kul-cli` and `kul-lsp`. The visual-rendering crates (`kul-render`, `kul-layout`, `kul-svg`) sit between `kul-core` and `kul-lsp` so the LSP can fulfil the `kul/render` request without dragging the canonical-pattern projection into core. New crates should preserve this unidirectional shape.

### Why a separate `kul-lsp` crate at all?

A nontrivial chunk of LSP-specific machinery (tower-lsp, async runtime, JSON-RPC framing, document cache, byte ↔ LSP-position translation) doesn't belong in core — it's editor-protocol concern, not language concern. Bundling it into `kul-cli` would force the CLI binary to pull in `tower-lsp` and `tokio` for users who only ever run `kul validate`. The split is the deletion test passing: removing `kul-lsp` would either reproduce the editor logic in the CLI, or eliminate editor support entirely.

### Why a separate `kul-wasm` crate at all?

Same shape of argument as the LSP split, in the JS direction. Browser and Node consumers cannot shell out to the `kul` binary or speak LSP over stdio; they need a JS-callable surface. Compiling `kul-core` to WebAssembly with `wasm-bindgen` is the only viable path. The bridge has its own concerns — `cdylib` crate type, `wasm-bindgen` derives, `console_error_panic_hook` registration, `serde-wasm-bindgen` round-trip, the bundler-target build pipeline — that have no place in `kul-core` or `kul-cli`. The deletion test passes: removing `kul-wasm` would either reproduce the JS adapter elsewhere, or eliminate JS-ecosystem consumers entirely. The `kul-core/tsify` feature exists exactly so the WASM crate can derive accurate `.d.ts` from the Rust source of truth without forcing tsify onto the CLI/LSP build graph (per [ADR-0012](./adr/0012-tsify-derived-types-committed-and-diffed.md)).

## LSP request flow

A textDocument request lands here:

```
client (VSCode)
  │  JSON-RPC over stdio
  ▼
server.rs::Backend::<request> handler
  │  reads ProjectEntry from state::Documents
  │  (cache key is the URI's project root, not the URI itself —
  │   one CheckResult per project, shared by every URI in it)
  ▼
&entry.check.resolved  (cached project-wide ResolvedDocument)
  │
  ▼  resolved.node_at(entry.file_id_for(uri), byte_offset)  →  Option<Node<'_>>
  │
  ▼  features/<feature>.rs builds the response by pattern-matching
  │  on the Node variant; cross-file features resolve target FileIds
  │  through resolved.entity(name).file, then map FileId → Url via
  │  ProjectEntry::url_for(...)
  │
  ▼  convert.rs translates ByteSpan + types to LSP types (Range, Url, …)
  │
  ▼  Result<Response, Error>
```

Lifecycle: `did_open` discovers the project from the opened URI (sibling `kul.yml` plus every `.kul` file in the same directory, read off disk) and inserts one cache entry for the whole project. `did_change` mutates the overlay for the active URI and re-runs `kul_core::check` for the project. `did_close` flips the URI's overlay to `None` and evicts the entry when no URIs from the project remain open. External filesystem changes reach the cache through `workspace/didChangeWatchedFiles` (issue #86): the LSP registers watchers for `**/*.kul` and `**/kul.yml` from `initialized`, then routes per-event through `Documents::process_watcher_event` — `.kul` Created/Changed/Deleted reload the project (Changed is dropped when the URI is overlaid so the editor buffer wins); `kul.yml` Changed reloads the manifest; `kul.yml` Deleted evicts the entry. Diagnostic publishes broadcast to every file in the project — open URIs and disk-only siblings alike — so the Problems pane reflects project-wide health (issue #85).

Four things make this work:

1. The project cache (`state::Documents`) holds the full project-wide `CheckResult` — including the cached `ResolvedDocument` — per project root. Opening N files from one project triggers one resolve, not N (per [ADR-0007](./adr/0007-resolved-document-owns-document.md) and [ADR-0015](./adr/0015-global-project-namespace.md)).
2. `node_at` is the shared foundation. Hover, goto-definition, and completion all start with "what's at the cursor?" — implementing it once means the three features can't disagree about it.
3. `Node` is a typed enum, not a tag. Each LSP feature pattern-matches the variants relevant to it; the compiler enforces exhaustiveness when a new variant lands.
4. The `ProjectEntry` carries a parallel `Vec<Url>` and `Vec<LineIndex>` keyed by `FileId`. Cross-file features (definition, references, rename) turn a project-wide query result into URI-keyed LSP responses by indexing those slices — there is no separate URI-keyed cache to keep in sync.

## The seams

The most load-bearing interfaces in the codebase. Don't bypass these.

| Seam                                  | File                                          | What's behind it                                                                                                |
| ------------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `kul_core::check`                    | `crates/kul-core/src/lib.rs`                 | The whole pipeline. CLI and LSP both enter here. Takes `(manifest_name, manifest_yaml: &str, &[InputFile])` per [ADR-0013](./adr/0013-project-manifest.md) and [ADR-0014](./adr/0014-file-identity-and-per-file-namespaces.md); returns a `CheckResult` whose `resolved: ResolvedDocument` field is the cached query view (per [ADR-0007](./adr/0007-resolved-document-owns-document.md)). The WASM bridge enters via `check_with_manifest` (a typed-manifest variant) instead. |
| `FileId` / `FileSpan`                 | `crates/kul-core/src/span.rs`                | Project-wide locators. `FileId` indexes into `Document.kul_files` (with `FileId::MANIFEST` = `FileId(0)`); `FileSpan` is the `(file, byte-range)` pair every diagnostic anchors on. AST nodes keep bare `ByteSpan` (per ADR-0014) — their owning `KulFile` provides file context implicitly. |
| `Manifest` + adapters' loaders       | `crates/kul-core/src/manifest.rs`, `crates/kul-loader/src/lib.rs` | The project-level manifest, loaded by each adapter. The shared `kul-loader` crate is the single seam for project-root → (manifest YAML + `.kul` files) discovery: `load` (strict, typed errors — used by the CLI) and `discover` (lenient — used by the LSP, which tolerates missing manifests so editor sessions stay usable). Both share one directory-walk helper so the discovery rule lives once. The WASM bridge receives a typed manifest as a JS argument — it does not go through the loader. `kul-core` itself never reads the filesystem — it consumes the typed value. The directory-scoped discovery rule (spec/14.3) lives once as `kul_core::manifest::sibling_path` (for per-file lookups: the LSP receives a `.kul` URI, asks for its sibling manifest); `kul-loader::{load,discover}` is the project-root counterpart. Schema and discovery rule are normative ([`spec/14-project-manifest.md`](../spec/14-project-manifest.md)). |
| `ResolvedDocument` query methods      | `crates/kul-core/src/semantic.rs`            | All kinship questions. ADR-0001 mandates queries go through this; raw AST iteration is the seam's job. Per-id lookups (`person`, `marriage`, `entity`) take a bare id (project-wide per [ADR-0015](./adr/0015-global-project-namespace.md)); per-file iteration helpers (`persons_in(file)`, etc.) keep the file parameter for per-URI LSP needs. Owns its `Arc<Document>` so the resolved view can be cached alongside other artifacts (no self-referential lifetime). |
| `ResolvedDocument::node_at`           | `crates/kul-core/src/node_at.rs`             | "What's at byte offset X?" Returns a typed `Node`. Foundation for hover, definition, completion.                |
| `ResolvedDocument::statement_at`      | `crates/kul-core/src/semantic.rs`            | "What top-level statement encloses byte offset X?" Returns `&Statement`. Coarser than `node_at` — used by completion to know whether a cursor sitting on a fresh line is "still under" the previous statement. |
| `Node::entity_reference`              | `crates/kul-core/src/node_at.rs`             | "What entity (person / marriage) is the cursor pointing at?" Returns an `EntityNode` summary (id, kind, decl span, target). Used by goto-definition, find-references, rename. |
| `Diagnostic` + `Severity` + code + `detail` | `crates/kul-core/src/diagnostic.rs`    | The error currency. Carries spans, codes (KUL-Rxx), related info, and (per ADR-0006) an optional sub-case tag. |
| `field_meta::FieldMeta`               | `crates/kul-core/src/field_meta.rs`          | Per-field taxonomy: value shape, completion description, hover Markdown. Hover, completion, and semantic-tokens consume it (ADR-0005). |
| `export::export`                      | `crates/kul-core/src/export.rs`              | Canonical JSON projection of a `CheckResult` into an `ExportEnvelope`. Strict on errors; format-dispatched (kinship-native or cytoscape). The deep module the CLI's `kul export` and the LSP's `kul/export` both call. Schema documented in [`spec/16-export-schema.md`](../spec/16-export-schema.md); shape, posture, and versioning settled in ADRs 0008–0010. |
| `kul_layout::layout`                  | `crates/kul-layout/src/lib.rs`               | `(RenderShape, LayoutConfig) -> PositionedShape`. Positioning pass for the canonical UI pattern. Wraps Walker's algorithm with the marriage-edge / ghost-slot / generation-row adapter. `PositionedShape` is an internal Rust seam — not Serialize, not schema-versioned (ADR-0016). |
| `kul_svg::render`                     | `crates/kul-svg/src/lib.rs`                  | `(PositionedShape, ThemeConfig) -> String`. Theme-agnostic SVG emitter. Uses semantic CSS classes; consuming surfaces own theming via stylesheet (ADR-0016). |
| `LineIndex`                           | `crates/kul-lsp/src/convert.rs`              | Byte ↔ LSP-position. Handles UTF-16 code units and CRLF. Holds source as `Arc<str>` so `state::Document` shares the same heap buffer.|
| `state::Documents`                    | `crates/kul-lsp/src/state.rs`                | The LSP project cache. Thread-safe; the only path to a `ProjectEntry` from inside an LSP request handler. Keyed by `ProjectRoot` (one entry per project, not per URI), so multiple open URIs in one project share a single `CheckResult`. Each `ProjectEntry` carries the project-wide `LineIndex` slice and the matching `Url` slice in `FileId(1..)` order — cross-file features resolve through these.          |
| `kul_wasm::{check, export_graph, format_source}` | `crates/kul-wasm/src/lib.rs`     | The WASM/JS surface. Three deep-module entrypoints exposed via `wasm-bindgen` with three operation-specific return shapes (per [ADR-0011](./adr/0011-wasm-surface-three-shapes-no-wrappers.md)). No convenience layer; consumers compose helpers at the call site. |

If you find yourself reaching around one of these (e.g. iterating `document.statements` from a feature module), stop and consider extending the seam instead.

## Where to add X

### A new validator rule

1. Add a function `fn rule_NN_<short_name>(resolved: &ResolvedDocument) -> Vec<Diagnostic>` in `crates/kul-core/src/validator.rs`. (R02–R13 all live here. R01 — duplicate ids — lives inside `semantic::resolve` because it's a property of insertion order.)
2. Call it from the rule jump-list at the top of the file.
3. Allocate a code `KUL-RNN`. Update [`spec/07-validation-rules.md`](../spec/07-validation-rules.md) with the rule definition.
4. Add a test `rule_NN_<short_name>` in `crates/kul-core/tests/validator.rs` covering the positive case (rule fires) and negative case (rule doesn't fire). Snapshot the diagnostic output.
5. If the rule needs a new kinship query ("does this person have any siblings?"), add it as a method on `ResolvedDocument` rather than walking the AST inline. ADR-0001.

### A new LSP feature

1. Read [ADR-0001](./adr/0001-resolved-document-as-query-seam.md) — your feature should phrase itself as a question against `ResolvedDocument`.
2. Add a new module under `crates/kul-lsp/src/features/`.
3. Wire it into `Backend` in `server.rs` and advertise the capability in `initialize`.
4. Add an integration test in `crates/kul-lsp/tests/<feature>.rs` using the existing minimal LSP client.
5. If the feature needs new "what's at the cursor?" information, extend the `Node` enum in `node_at.rs` (additively — don't rename existing variants) and the resolution logic. The feature then matches on the new variant.
6. If the feature keys on "what entity is the cursor pointing at?" (a person / marriage id, decl or reference), call `node.entity_reference()` instead of pattern-matching the four id-bearing `Node` variants by hand. The accessor returns an `EntityNode` with `kind`, `name`, `ident_span`, `is_decl`, `target`, and a `decl_span()` method.

### A new LSP custom request

Custom (non-LSP-standard) requests like `kul/export` follow a slightly different wiring than the textDocument capabilities above:

1. Define request params (with `serde::Deserialize`) and the projection function in `crates/kul-lsp/src/features/<request>.rs`. Keep the projection a pure `(Document, Params) -> Result<Response, Error>` function so it's unit-testable without LSP plumbing.
2. Add a public method on `Backend` (`pub async fn <name>(&self, params: …) -> jsonrpc::Result<…>`) in `server.rs` that reads from `state::Documents` and calls the projection.
3. Register the method in `lib.rs` via `LspService::build(...).custom_method("<namespace>/<name>", Backend::<name>).finish()`.
4. Advertise the custom capability under `experimental.<name>` in `Backend::initialize` so clients can detect support.
5. Add an integration test that drives the real binary with the same hand-rolled stdio LSP client as the standard-capability tests use; verify both the success-payload shape and the JSON-RPC error path.

### A new AST variant

This is the highest-risk change because the AST is a stable surface. Read the additivity principle (in agent memory) before starting.

1. Extend the relevant enum / struct in `crates/kul-core/src/ast.rs` — *additively*. New variant or new optional field; never reorder, rename, or remove.
2. Update the parser to produce the new shape.
3. Update `node_at.rs` so the cursor-resolver covers it (else hover/definition/completion will silently miss it).
4. Update the validator if any rule should care.
5. Update LSP feature modules whose `match` arms might now be non-exhaustive (the compiler will tell you).
6. Update the spec.

### A new field on a statement

Per ADR-0005 the field taxonomy lives in `field_meta`. Adding a field is mostly a one-table change.

1. Extend `FieldName` (in `lexer.rs`) and the relevant `*FieldKind` enum in `ast.rs` — *additively*.
2. Add a row to `META` in `crates/kul-core/src/field_meta.rs` and add the `FieldName` to the right `*_FIELDS` slice (canonical formatter order).
3. Update the parser to emit the new variant.
4. If the field is required, R03 needs a new arm; otherwise the validator picks it up automatically through field accessors.
5. Hover, completion, and semantic-tokens all read the new row at runtime — no editing needed in those features.

### A new sub-case on an existing rule

Per ADR-0006 a single rule can carry multiple sub-cases on the same primary span, distinguished by a `detail` tag. Add one when the code-action provider (or any other tooling consumer) needs to behave differently per sub-case.

1. Add a `pub const` tag in `kul_core::diagnostic::detail` (naming: `<rule>-<short>`).
2. Set it on the producing diagnostic via `.with_detail(detail::TAG)`.
3. Match on it at the consumer (e.g. the code-action registry).

### A new CLI subcommand

1. Add a variant to `Command` in `crates/kul-cli/src/main.rs`.
2. Add the implementation under `crates/kul-cli/src/commands/`.
3. Add an end-to-end test in `crates/kul-cli/tests/` using `assert_cmd`.

### A new exported field

The export module (`crates/kul-core/src/export.rs`) projects every Person, Marriage, and parenthood-link field through a single per-type builder (`exported_person`, `build_graph`, `build_parenthood_links`). When you add a new field on a Person/Marriage/Adoption (per "A new field on a statement" above), the export side picks it up in three steps:

1. Add an optional field to the matching `Exported*` struct in `export.rs` (mark it `#[serde(skip_serializing_if = "Option::is_none")]` if not always present).
2. Read the field via the existing `*Stmt::<field>()` accessor in the per-type builder and assign it.
3. Document the new field in [`spec/16-export-schema.md`](../spec/16-export-schema.md) — additively, since per [ADR-0010](./adr/0010-export-schema-versioning.md) new optional fields do NOT bump the `schema` integer.

The export snapshot suite (`crates/kul-core/tests/export.rs`) auto-grows as each example file changes shape, so the new field's representation gets snapshot-locked the moment you `cargo insta accept`.

If the new construct is structurally large enough that consumers might silently drop it (a new top-level collection, an existing field's semantics changing incompatibly), bump `SCHEMA_VERSION` in `export.rs` in the same change and document the bump in [ADR-0010](./adr/0010-export-schema-versioning.md).

### A new WASM-exposed function or type

The bridge is intentionally thin (per [ADR-0011](./adr/0011-wasm-surface-three-shapes-no-wrappers.md)) and adding to it should stay thin. Re-read that ADR before adding any new surface — the rule of three says wait until a third independent consumer asks for the same helper.

When a new entrypoint genuinely belongs:

1. Add a `pub fn` in `crates/kul-wasm/src/lib.rs` with `#[wasm_bindgen(js_name = "camelCaseName")]`. Body should be ≤3 lines: `console_error_panic_hook::set_once()`, one `kul_core::*` call, return.
2. If the return type is a new struct, derive `serde::Serialize` + `tsify::Tsify`, add `#[tsify(into_wasm_abi)]` and `#[serde(rename_all = "camelCase")]`. Add the type's rustdoc explaining the JS-side contract — that text becomes the JSDoc on the generated `.d.ts`.
3. If the new type is reused from `kul-core`, prefer extending the existing `Exported*` struct in `crates/kul-core/src/export.rs` over inventing a parallel WASM-only type. Single source of truth.
4. Add a snapshot test in `crates/kul-wasm/tests/<entrypoint>.rs` mirroring the existing `check.rs` / `format.rs` / `export_graph.rs` shape.
5. Extend `crates/kul-wasm/tests/typescript/usage.ts` to exercise the new surface from a real consumer perspective. CI runs `tsc --noEmit` on it.
6. Run `just wasm` to regenerate `crates/kul-wasm/types/kul_wasm.d.ts`. Commit the diff. CI's `wasm-build` job (per [ADR-0012](./adr/0012-tsify-derived-types-committed-and-diffed.md)) fails the merge if the committed snapshot drifts from the regenerated output.
7. Extend the Node smoke test (`crates/kul-wasm/tests/node/smoke.mjs`) if the new function would meaningfully break end-to-end without protocol-level coverage.

### A new public type or function

If it's part of the kul-core surface, add `//!`/`///` rustdoc explaining the contract. This is the *interface*, not the implementation. Internal helpers can be terse; public surface earns documentation.

If the new type crosses the WASM boundary (i.e. it's part of `ExportEnvelope` or one of its descendants, or `kul-wasm` will reuse it), the rustdoc on the type *is* the JSDoc that lands in the generated `.d.ts`. Write it for a JS/TS consumer, not just a Rust reader. Mark optional fields `#[serde(skip_serializing_if = "Option::is_none")]` so they are omitted from the JSON when absent rather than serialized as `null`; `Option<T>` automatically projects to `T | undefined` (a `?`-marked TS field) via the existing `derive(Tsify)`.

## What not to add

Things to push back on if you find yourself reaching for them:

- A `Visitor` trait over the AST. Pattern matches on a 2-variant enum are clearer; add this only if the AST grows past ~6 statement variants. ADR-0001 anti-suggestion.
- A "framework" for parser error recovery. The grammar is small; ad-hoc recovery is fine. Reconsider if the grammar doubles.
- A shared LSP-feature query helper. `completion.rs` and `hover.rs` overlap a little; that's fine. Extract only when a third feature wants the same code (rule of three).
- Re-exposing `Document.statements` to external callers. ADR-0001 closes this off.
- A trait abstraction over "things that can validate." There's one validator; abstractions enabled by future-supposed alternatives are speculative.

## Performance budget

The current target is **<100ms total** for parse + check + LSP-translate on a 1000-statement document. The budget lives as a test (not a benchmark), at [`crates/kul-lsp/tests/perf.rs`](../crates/kul-lsp/tests/perf.rs). The test asserts <500ms (5× CI slack) so it doesn't flake on slow runners; the comment in the test records the actual target.

If a change pushes the test over budget, *fix the regression* — don't loosen the assertion. If a new pass legitimately needs more headroom, raise the budget in the same change with a comment justifying why.

See [`testing.md`](./testing.md) for the rationale on perf-as-tests-rather-than-benches.

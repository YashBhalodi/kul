# Architecture

The implementation map for KulaLang. Read this when you need to make a change and don't yet know where to put it.

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

A `.kula` source string flows through the toolchain like this:

```
source: &str
  │
  ▼  lexer.rs              produces tokens (flat sequence + spans)
  │
  ▼  parser.rs             produces Document (typed AST + spans)
  │
  ▼  semantic.rs           produces ResolvedDocument (AST + id indexes)
  │                        emits rule 01 diagnostics inline (duplicate ids)
  │
  ▼  validator.rs          runs rules 02–13, accumulates diagnostics
  │                        rule 13 delegates to cycles.rs
  │
  ▼  CheckResult { resolved: ResolvedDocument, diagnostics: Vec<Diagnostic> }
```

`kula_core::check(source)` is the single entry point that runs the whole pipeline. The CLI calls it once per file; the LSP calls it once per document update.

The shape is deliberately linear. Each pass produces a strictly richer artifact; nothing earlier in the pipeline ever consults something later. This is why:

- The lexer doesn't know about IDs.
- The parser doesn't know which IDs are declared.
- The validator never reaches into raw `Document.statements` — it queries through `ResolvedDocument` (per [ADR-0001](./adr/0001-resolved-document-as-query-seam.md)). All thirteen spec rules (R01 lives inline in `semantic::resolve` because it's a property of insertion order; R02–R13 live in `validator.rs`).

## Crate map

```
kula-core   ── library (no_std-friendly intent, but uses std for now)
              the entire pipeline lives here. Public surface is the
              CheckResult API plus the AST types, ResolvedDocument
              query methods, the formatter, and the export module
              (kinship-native + cytoscape projections).

kula-cli    ── thin binary `kula`
              Four subcommands: `validate` (renders diagnostics with
              miette), `format` (canonicalize per ADR-0004), `export`
              (project to JSON via kula_core::export), and `lsp`
              (delegates to kula_lsp::run). Owns argument parsing and
              human/JSON output formatting.

kula-lsp    ── library + binary `kula-lsp`
              tower-lsp Backend implementation. Owns the document cache,
              implements feature modules (hover, definition, completion,
              diagnostics, export, …), translates kula-core types to LSP
              types. Never re-implements pipeline logic — only adapts.
              Custom requests (e.g. `kula/export`) are registered via
              `LspService::build().custom_method(...)` in `lib.rs`.

kula-wasm   ── library (cdylib + rlib), published as `@kulalang/wasm`
              wasm-bindgen adapter over kula-core. Three exposed
              functions — `check`, `exportGraph`, `format` — each a
              two-or-three-line wrapper around the matching kula-core
              deep module, plus version-metadata getters. Surface
              shape is settled in ADR-0011; TypeScript types are
              derived via Tsify, committed, and diffed in CI per
              ADR-0012. Single ESM `--target bundler` build for
              modern bundlers (Vite, Webpack, Next.js, etc.).
```

The dependency graph is unidirectional: `kula-cli → kula-lsp → kula-core`, `kula-cli → kula-core`, and `kula-wasm → kula-core`. Nothing depends on the CLI; nothing in core depends on the LSP or the WASM crate. New crates should preserve this.

### Why a separate `kula-lsp` crate at all?

A nontrivial chunk of LSP-specific machinery (tower-lsp, async runtime, JSON-RPC framing, document cache, byte ↔ LSP-position translation) doesn't belong in core — it's editor-protocol concern, not language concern. Bundling it into `kula-cli` would force the CLI binary to pull in `tower-lsp` and `tokio` for users who only ever run `kula validate`. The split is the deletion test passing: removing `kula-lsp` would either reproduce the editor logic in the CLI, or eliminate editor support entirely.

### Why a separate `kula-wasm` crate at all?

Same shape of argument as the LSP split, in the JS direction. Browser and Node consumers cannot shell out to the `kula` binary or speak LSP over stdio; they need a JS-callable surface. Compiling `kula-core` to WebAssembly with `wasm-bindgen` is the only viable path. The bridge has its own concerns — `cdylib` crate type, `wasm-bindgen` derives, `console_error_panic_hook` registration, `serde-wasm-bindgen` round-trip, the bundler-target build pipeline — that have no place in `kula-core` or `kula-cli`. The deletion test passes: removing `kula-wasm` would either reproduce the JS adapter elsewhere, or eliminate JS-ecosystem consumers entirely. The `kula-core/tsify` feature exists exactly so the WASM crate can derive accurate `.d.ts` from the Rust source of truth without forcing tsify onto the CLI/LSP build graph (per [ADR-0012](./adr/0012-tsify-derived-types-committed-and-diffed.md)).

## LSP request flow

A textDocument request lands here:

```
client (VSCode)
  │  JSON-RPC over stdio
  ▼
server.rs::Backend::<request> handler
  │  reads document from state::Documents
  ▼
&doc.check.resolved  (cached ResolvedDocument for this URI)
  │
  ▼  resolved.node_at(byte_offset)  →  Option<Node<'_>>
  │
  ▼  features/<feature>.rs builds the response by pattern-matching
  │  on the Node variant
  │
  ▼  convert.rs translates ByteSpan + types to LSP types (Range, Url, …)
  │
  ▼  Result<Response, Error>
```

Three things make this work:

1. The document cache (`state::Documents`) holds the full `CheckResult` — including the cached `ResolvedDocument` — per open URI. Re-parsing or re-resolving on every request would be wasteful; the cache is the source of truth (per [ADR-0007](./adr/0007-resolved-document-owns-document.md)).
2. `node_at` is the shared foundation. Hover, goto-definition, and completion all start with "what's at the cursor?" — implementing it once means the three features can't disagree about it.
3. `Node` is a typed enum, not a tag. Each LSP feature pattern-matches the variants relevant to it; the compiler enforces exhaustiveness when a new variant lands.

## The seams

The most load-bearing interfaces in the codebase. Don't bypass these.

| Seam                                  | File                                          | What's behind it                                                                                                |
| ------------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `kula_core::check`                    | `crates/kula-core/src/lib.rs`                 | The whole pipeline. CLI and LSP both enter here. Returns a `CheckResult` whose `resolved: ResolvedDocument` field is the cached query view (per [ADR-0007](./adr/0007-resolved-document-owns-document.md)). |
| `ResolvedDocument` query methods      | `crates/kula-core/src/semantic.rs`            | All kinship questions. ADR-0001 mandates queries go through this; raw AST iteration is the seam's job. Owns its `Arc<Document>` so the resolved view can be cached alongside other artifacts (no self-referential lifetime). |
| `ResolvedDocument::node_at`           | `crates/kula-core/src/node_at.rs`             | "What's at byte offset X?" Returns a typed `Node`. Foundation for hover, definition, completion.                |
| `ResolvedDocument::statement_at`      | `crates/kula-core/src/semantic.rs`            | "What top-level statement encloses byte offset X?" Returns `&Statement`. Coarser than `node_at` — used by completion to know whether a cursor sitting on a fresh line is "still under" the previous statement. |
| `Node::entity_reference`              | `crates/kula-core/src/node_at.rs`             | "What entity (person / marriage) is the cursor pointing at?" Returns an `EntityNode` summary (id, kind, decl span, target). Used by goto-definition, find-references, rename. |
| `Diagnostic` + `Severity` + code + `detail` | `crates/kula-core/src/diagnostic.rs`    | The error currency. Carries spans, codes (KULA-Rxx), related info, and (per ADR-0006) an optional sub-case tag. |
| `field_meta::FieldMeta`               | `crates/kula-core/src/field_meta.rs`          | Per-field taxonomy: value shape, completion description, hover Markdown. Hover, completion, and semantic-tokens consume it (ADR-0005). |
| `export::export`                      | `crates/kula-core/src/export.rs`              | Canonical JSON projection of a `CheckResult` into an `ExportEnvelope`. Strict on errors; format-dispatched (kinship-native or cytoscape). The deep module the CLI's `kula export` and the LSP's `kula/export` both call. Schema documented in [`spec/15-export-schema.md`](../spec/15-export-schema.md); shape, posture, and versioning settled in ADRs 0008–0010. |
| `LineIndex`                           | `crates/kula-lsp/src/convert.rs`              | Byte ↔ LSP-position. Handles UTF-16 code units and CRLF. Holds source as `Arc<str>` so `state::Document` shares the same heap buffer.|
| `state::Documents`                    | `crates/kula-lsp/src/state.rs`                | The LSP document cache. Thread-safe; the only path to a `Document` from inside an LSP request handler. Each cached `Document` shares one `Arc<str>` between its `source` field and the `LineIndex`.          |
| `kula_wasm::{check, export_graph, format_source}` | `crates/kula-wasm/src/lib.rs`     | The WASM/JS surface. Three deep-module entrypoints exposed via `wasm-bindgen` with three operation-specific return shapes (per [ADR-0011](./adr/0011-wasm-surface-three-shapes-no-wrappers.md)). No convenience layer; consumers compose helpers at the call site. |

If you find yourself reaching around one of these (e.g. iterating `document.statements` from a feature module), stop and consider extending the seam instead.

## Where to add X

### A new validator rule

1. Add a function `fn rule_NN_<short_name>(resolved: &ResolvedDocument) -> Vec<Diagnostic>` in `crates/kula-core/src/validator.rs`. (R02–R13 all live here. R01 — duplicate ids — lives inside `semantic::resolve` because it's a property of insertion order.)
2. Call it from the rule jump-list at the top of the file.
3. Allocate a code `KULA-RNN`. Update [`spec/07-validation-rules.md`](../spec/07-validation-rules.md) with the rule definition.
4. Add a test `rule_NN_<short_name>` in `crates/kula-core/tests/validator.rs` covering the positive case (rule fires) and negative case (rule doesn't fire). Snapshot the diagnostic output.
5. If the rule needs a new kinship query ("does this person have any siblings?"), add it as a method on `ResolvedDocument` rather than walking the AST inline. ADR-0001.

### A new LSP feature

1. Read [ADR-0001](./adr/0001-resolved-document-as-query-seam.md) — your feature should phrase itself as a question against `ResolvedDocument`.
2. Add a new module under `crates/kula-lsp/src/features/`.
3. Wire it into `Backend` in `server.rs` and advertise the capability in `initialize`.
4. Add an integration test in `crates/kula-lsp/tests/<feature>.rs` using the existing minimal LSP client.
5. If the feature needs new "what's at the cursor?" information, extend the `Node` enum in `node_at.rs` (additively — don't rename existing variants) and the resolution logic. The feature then matches on the new variant.
6. If the feature keys on "what entity is the cursor pointing at?" (a person / marriage id, decl or reference), call `node.entity_reference()` instead of pattern-matching the four id-bearing `Node` variants by hand. The accessor returns an `EntityNode` with `kind`, `name`, `ident_span`, `is_decl`, `target`, and a `decl_span()` method.

### A new LSP custom request

Custom (non-LSP-standard) requests like `kula/export` follow a slightly different wiring than the textDocument capabilities above:

1. Define request params (with `serde::Deserialize`) and the projection function in `crates/kula-lsp/src/features/<request>.rs`. Keep the projection a pure `(Document, Params) -> Result<Response, Error>` function so it's unit-testable without LSP plumbing.
2. Add a public method on `Backend` (`pub async fn <name>(&self, params: …) -> jsonrpc::Result<…>`) in `server.rs` that reads from `state::Documents` and calls the projection.
3. Register the method in `lib.rs` via `LspService::build(...).custom_method("<namespace>/<name>", Backend::<name>).finish()`.
4. Advertise the custom capability under `experimental.<name>` in `Backend::initialize` so clients can detect support.
5. Add an integration test that drives the real binary with the same hand-rolled stdio LSP client as the standard-capability tests use; verify both the success-payload shape and the JSON-RPC error path.

### A new AST variant

This is the highest-risk change because the AST is a stable surface. Read the additivity principle (in agent memory) before starting.

1. Extend the relevant enum / struct in `crates/kula-core/src/ast.rs` — *additively*. New variant or new optional field; never reorder, rename, or remove.
2. Update the parser to produce the new shape.
3. Update `node_at.rs` so the cursor-resolver covers it (else hover/definition/completion will silently miss it).
4. Update the validator if any rule should care.
5. Update LSP feature modules whose `match` arms might now be non-exhaustive (the compiler will tell you).
6. Update the spec.

### A new field on a statement

Per ADR-0005 the field taxonomy lives in `field_meta`. Adding a field is mostly a one-table change.

1. Extend `FieldName` (in `lexer.rs`) and the relevant `*FieldKind` enum in `ast.rs` — *additively*.
2. Add a row to `META` in `crates/kula-core/src/field_meta.rs` and add the `FieldName` to the right `*_FIELDS` slice (canonical formatter order).
3. Update the parser to emit the new variant.
4. If the field is required, R03 needs a new arm; otherwise the validator picks it up automatically through field accessors.
5. Hover, completion, and semantic-tokens all read the new row at runtime — no editing needed in those features.

### A new sub-case on an existing rule

Per ADR-0006 a single rule can carry multiple sub-cases on the same primary span, distinguished by a `detail` tag. Add one when the code-action provider (or any other tooling consumer) needs to behave differently per sub-case.

1. Add a `pub const` tag in `kula_core::diagnostic::detail` (naming: `<rule>-<short>`).
2. Set it on the producing diagnostic via `.with_detail(detail::TAG)`.
3. Match on it at the consumer (e.g. the code-action registry).

### A new CLI subcommand

1. Add a variant to `Command` in `crates/kula-cli/src/main.rs`.
2. Add the implementation under `crates/kula-cli/src/commands/`.
3. Add an end-to-end test in `crates/kula-cli/tests/` using `assert_cmd`.

### A new exported field

The export module (`crates/kula-core/src/export.rs`) projects every Person, Marriage, and parenthood-link field through a single per-type builder (`exported_person`, `build_graph`, `build_parenthood_links`). When you add a new field on a Person/Marriage/Adoption (per "A new field on a statement" above), the export side picks it up in three steps:

1. Add an optional field to the matching `Exported*` struct in `export.rs` (mark it `#[serde(skip_serializing_if = "Option::is_none")]` if not always present).
2. Read the field via the existing `*Stmt::<field>()` accessor in the per-type builder and assign it.
3. Document the new field in [`spec/15-export-schema.md`](../spec/15-export-schema.md) — additively, since per [ADR-0010](./adr/0010-export-schema-versioning.md) new optional fields do NOT bump the `schema` integer.

The export snapshot suite (`crates/kula-core/tests/export.rs`) auto-grows as each example file changes shape, so the new field's representation gets snapshot-locked the moment you `cargo insta accept`.

If the new construct is structurally large enough that consumers might silently drop it (a new top-level collection, an existing field's semantics changing incompatibly), bump `SCHEMA_VERSION` in `export.rs` in the same change and document the bump in [ADR-0010](./adr/0010-export-schema-versioning.md).

### A new WASM-exposed function or type

The bridge is intentionally thin (per [ADR-0011](./adr/0011-wasm-surface-three-shapes-no-wrappers.md)) and adding to it should stay thin. Re-read that ADR before adding any new surface — the rule of three says wait until a third independent consumer asks for the same helper.

When a new entrypoint genuinely belongs:

1. Add a `pub fn` in `crates/kula-wasm/src/lib.rs` with `#[wasm_bindgen(js_name = "camelCaseName")]`. Body should be ≤3 lines: `console_error_panic_hook::set_once()`, one `kula_core::*` call, return.
2. If the return type is a new struct, derive `serde::Serialize` + `tsify::Tsify`, add `#[tsify(into_wasm_abi)]` and `#[serde(rename_all = "camelCase")]`. Add the type's rustdoc explaining the JS-side contract — that text becomes the JSDoc on the generated `.d.ts`.
3. If the new type is reused from `kula-core`, prefer extending the existing `Exported*` struct in `crates/kula-core/src/export.rs` over inventing a parallel WASM-only type. Single source of truth.
4. Add a snapshot test in `crates/kula-wasm/tests/<entrypoint>.rs` mirroring the existing `check.rs` / `format.rs` / `export_graph.rs` shape.
5. Extend `crates/kula-wasm/tests/typescript/usage.ts` to exercise the new surface from a real consumer perspective. CI runs `tsc --noEmit` on it.
6. Run `just wasm` to regenerate `crates/kula-wasm/types/kula_wasm.d.ts`. Commit the diff. CI's `wasm-build` job (per [ADR-0012](./adr/0012-tsify-derived-types-committed-and-diffed.md)) fails the merge if the committed snapshot drifts from the regenerated output.
7. Extend the Node smoke test (`crates/kula-wasm/tests/node/smoke.mjs`) if the new function would meaningfully break end-to-end without protocol-level coverage.

### A new public type or function

If it's part of the kula-core surface, add `//!`/`///` rustdoc explaining the contract. This is the *interface*, not the implementation. Internal helpers can be terse; public surface earns documentation.

If the new type crosses the WASM boundary (i.e. it's part of `ExportEnvelope` or one of its descendants, or `kula-wasm` will reuse it), the rustdoc on the type *is* the JSDoc that lands in the generated `.d.ts`. Write it for a JS/TS consumer, not just a Rust reader. Mark optional fields `#[serde(skip_serializing_if = "Option::is_none")]` so they are omitted from the JSON when absent rather than serialized as `null`; `Option<T>` automatically projects to `T | undefined` (a `?`-marked TS field) via the existing `derive(Tsify)`.

## What not to add

Things to push back on if you find yourself reaching for them:

- A `Visitor` trait over the AST. Pattern matches on a 2-variant enum are clearer; add this only if the AST grows past ~6 statement variants. ADR-0001 anti-suggestion.
- A "framework" for parser error recovery. The grammar is small; ad-hoc recovery is fine. Reconsider if the grammar doubles.
- A shared LSP-feature query helper. `completion.rs` and `hover.rs` overlap a little; that's fine. Extract only when a third feature wants the same code (rule of three).
- Re-exposing `Document.statements` to external callers. ADR-0001 closes this off.
- A trait abstraction over "things that can validate." There's one validator; abstractions enabled by future-supposed alternatives are speculative.

## Performance budget

The current target is **<100ms total** for parse + check + LSP-translate on a 1000-statement document. The budget lives as a test (not a benchmark), at [`crates/kula-lsp/tests/perf.rs`](../crates/kula-lsp/tests/perf.rs). The test asserts <500ms (5× CI slack) so it doesn't flake on slow runners; the comment in the test records the actual target.

If a change pushes the test over budget, *fix the regression* — don't loosen the assertion. If a new pass legitimately needs more headroom, raise the budget in the same change with a comment justifying why.

See [`testing.md`](./testing.md) for the rationale on perf-as-tests-rather-than-benches.

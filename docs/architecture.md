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
  │                        emits rule 02 diagnostics inline (unresolved refs)
  │
  ▼  validator.rs          runs rules 03–13, accumulates diagnostics
  │                        rule 13 delegates to cycles.rs
  │
  ▼  CheckResult { resolved: ResolvedDocument, diagnostics: Vec<Diagnostic> }
```

`kula_core::check(source)` is the single entry point that runs the whole pipeline. The CLI calls it once per file; the LSP calls it once per document update.

The shape is deliberately linear. Each pass produces a strictly richer artifact; nothing earlier in the pipeline ever consults something later. This is why:

- The lexer doesn't know about IDs.
- The parser doesn't know which IDs are declared.
- The validator never reaches into raw `Document.statements` — it queries through `ResolvedDocument` (per [ADR-0001](./adr/0001-resolved-document-as-query-seam.md)).

## Crate map

```
kula-core   ── library (no_std-friendly intent, but uses std for now)
              the entire pipeline lives here. Public surface is the
              CheckResult API plus the AST types and ResolvedDocument
              query methods.

kula-cli    ── thin binary `kula`
              Two subcommands: `validate` (renders diagnostics with
              miette) and `lsp` (delegates to kula_lsp::run).
              Owns argument parsing and human/JSON output formatting.

kula-lsp    ── library + binary `kula-lsp`
              tower-lsp Backend implementation. Owns the document cache,
              implements feature modules (hover, definition, completion,
              diagnostics), translates kula-core types to LSP types.
              Never re-implements pipeline logic — only adapts.
```

The dependency graph is unidirectional: `kula-cli → kula-lsp → kula-core`, and `kula-cli → kula-core`. Nothing depends on the CLI; nothing in core depends on the LSP. New crates should preserve this.

### Why a separate `kula-lsp` crate at all?

A nontrivial chunk of LSP-specific machinery (tower-lsp, async runtime, JSON-RPC framing, document cache, byte ↔ LSP-position translation) doesn't belong in core — it's editor-protocol concern, not language concern. Bundling it into `kula-cli` would force the CLI binary to pull in `tower-lsp` and `tokio` for users who only ever run `kula validate`. The split is the deletion test passing: removing `kula-lsp` would either reproduce the editor logic in the CLI, or eliminate editor support entirely.

## LSP request flow

A textDocument request lands here:

```
client (VSCode)
  │  JSON-RPC over stdio
  ▼
server.rs::Backend::<request> handler
  │  reads document from state::Documents
  ▼
ResolvedDocument (already cached for this URI)
  │
  ▼  resolved.node_at(byte_offset)  →  Option<Node<'a>>
  │
  ▼  features/<feature>.rs builds the response by pattern-matching
  │  on the Node variant
  │
  ▼  convert.rs translates ByteSpan + types to LSP types (Range, Url, …)
  │
  ▼  Result<Response, Error>
```

Three things make this work:

1. The document cache (`state::Documents`) holds parsed-and-resolved documents — re-parsing on every request would be wasteful, and the cache is the source of truth.
2. `node_at` is the shared foundation. Hover, goto-definition, and completion all start with "what's at the cursor?" — implementing it once means the three features can't disagree about it.
3. `Node` is a typed enum, not a tag. Each LSP feature pattern-matches the variants relevant to it; the compiler enforces exhaustiveness when a new variant lands.

## The seams

The most load-bearing interfaces in the codebase. Don't bypass these.

| Seam                                  | File                                          | What's behind it                                                                                                |
| ------------------------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `kula_core::check`                    | `crates/kula-core/src/lib.rs`                 | The whole pipeline. CLI and LSP both enter here.                                                                |
| `ResolvedDocument` query methods      | `crates/kula-core/src/semantic.rs`            | All kinship questions. ADR-0001 mandates queries go through this; raw AST iteration is the seam's job.          |
| `ResolvedDocument::node_at`           | `crates/kula-core/src/node_at.rs`             | "What's at byte offset X?" Returns a typed `Node`. Foundation for hover, definition, completion.                |
| `Diagnostic` + `Severity` + code      | `crates/kula-core/src/diagnostic.rs`          | The error currency. Carries spans, codes (KULA-Rxx), related info. Both renderers (miette, LSP) consume it.     |
| `LineIndex`                           | `crates/kula-lsp/src/convert.rs`              | Byte ↔ LSP-position. Handles UTF-16 code units and CRLF. The only place that knows about LSP position semantics.|
| `state::Documents`                    | `crates/kula-lsp/src/state.rs`                | The LSP document cache. Thread-safe; the only path to a `Document` from inside an LSP request handler.          |

If you find yourself reaching around one of these (e.g. iterating `document.statements` from a feature module), stop and consider extending the seam instead.

## Where to add X

### A new validator rule

1. Add a function `fn rule_NN_<short_name>(resolved: &ResolvedDocument, diagnostics: &mut Vec<Diagnostic>)` in `crates/kula-core/src/validator.rs`.
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

### A new AST variant

This is the highest-risk change because the AST is a stable surface. Read the additivity principle (in agent memory) before starting.

1. Extend the relevant enum / struct in `crates/kula-core/src/ast.rs` — *additively*. New variant or new optional field; never reorder, rename, or remove.
2. Update the parser to produce the new shape.
3. Update `node_at.rs` so the cursor-resolver covers it (else hover/definition/completion will silently miss it).
4. Update the validator if any rule should care.
5. Update LSP feature modules whose `match` arms might now be non-exhaustive (the compiler will tell you).
6. Update the spec.

### A new CLI subcommand

1. Add a variant to `Command` in `crates/kula-cli/src/main.rs`.
2. Add the implementation under `crates/kula-cli/src/commands/`.
3. Add an end-to-end test in `crates/kula-cli/tests/` using `assert_cmd`.

### A new public type or function

If it's part of the kula-core surface, add `//!`/`///` rustdoc explaining the contract. This is the *interface*, not the implementation. Internal helpers can be terse; public surface earns documentation.

## What not to add

Things to push back on if you find yourself reaching for them:

- A `Visitor` trait over the AST. Pattern matches on a 2-variant enum are clearer; add this only if the AST grows past ~6 statement variants. ADR-0001 anti-suggestion.
- A "framework" for parser error recovery. The grammar is small; ad-hoc recovery is fine. Reconsider if the grammar doubles.
- A shared LSP-feature query helper. `completion.rs` and `hover.rs` overlap a little; that's fine. Extract only when a third feature wants the same code (rule of three).
- Re-exposing `Document.statements` to external callers. ADR-0001 closes this off.
- A trait abstraction over "things that can validate." There's one validator; abstractions enabled by future-supposed alternatives are speculative.

## Performance budget

The current target is **<100ms total** for parse + check + LSP-translate on a 1000-statement document. The budget lives as a test (not a benchmark), at `crates/kula-lsp/src/features/diagnostics.rs::one_thousand_statement_check_and_translate_under_budget`. The test asserts <500ms (5× CI slack) so it doesn't flake on slow runners; the comment in the test records the actual target.

If a change pushes the test over budget, *fix the regression* — don't loosen the assertion. If a new pass legitimately needs more headroom, raise the budget in the same change with a comment justifying why.

See [`testing.md`](./testing.md) for the rationale on perf-as-tests-rather-than-benches.

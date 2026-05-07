# ADR 0007 — `ResolvedDocument` owns its `Document` via `Arc<Document>`

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

`ResolvedDocument` is the kinship-query seam ([ADR-0001](./0001-resolved-document-as-query-seam.md)) — the public surface every validator rule and LSP feature queries through. Originally it borrowed the parsed AST: `ResolvedDocument<'a>` held `&'a Document` plus a set of `HashMap<&'a str, &'a PersonStmt>` indexes. The borrow lifetime tied the resolved view to whatever owned the `Document` — typically a local variable, or the `CheckResult` returned by `kul_core::check`.

This caused a real and visible problem in the LSP. `CheckResult` owned the `Document`; `ResolvedDocument<'a>` borrowed it; storing both in a single struct was self-referential, so `CheckResult` could not cache the resolved view. The `resolved()` method instead re-ran `semantic::resolve` on every call. That meant every LSP request handler — hover, goto-definition, completion, references, rename, code-actions, document-symbol, semantic-tokens — re-built the id index per keystroke. The comment in `lib.rs` openly acknowledged the regression: *"can't be stored alongside the owned `Document` (the borrow lifetime would be self-referential). Cost: re-runs `semantic::resolve` (one pass over the AST, hashmap insertions per statement). Cheap for editor-scale documents but not free."*

Three options were considered:

1. **Self-referential helper crate (`ouroboros` / `yoke`).** Keep the borrowed-lifetime `ResolvedDocument<'a>` and use a macro-generated wrapper to store it alongside its `Document`. Smallest API change. Cost: a new dependency for one use site, an extra layer of indirection at every call, and macro-driven types are harder for both humans and AI agents to reason about.
2. **`Arc<Document>` ownership inside `ResolvedDocument`.** Drop the `'a` lifetime entirely. `ResolvedDocument` holds `Arc<Document>` (cheap to clone), plus an id index keyed by owned `String` mapping to statement indices. Query methods rebuild the borrowed view (`&PersonStmt`, `EntityRef<'_>`) on demand, tied to the `&self` borrow. `CheckResult` stores `ResolvedDocument` directly — no recomputation.
3. **Index-based decoupling (no `Arc`).** Split the resolved data into the `Document` plus a separate `ResolvedIndexes` value. Both owned, neither self-referential. `ResolvedDocument` becomes a `(Arc<Document>, &ResolvedIndexes)` view built per request. Adds a layer; query methods need to thread two refs everywhere; effectively reinvents path 2 with an extra container.

## Decision

Option 2. `ResolvedDocument` owns an `Arc<Document>`. The id index uses owned `String` keys mapping to a private `ResolvedEntity { kind, statement_idx }` value — no lifetime parameters, no borrows into the document. Query methods (`person`, `marriage`, `entity`, `spouses_of`, `parents_of`, `node_at`) take `&self` and return references with the elided lifetime of `&self`.

`semantic::resolve` takes `Arc<Document>` and returns `(ResolvedDocument, Vec<Diagnostic>)`. `CheckResult` owns the `ResolvedDocument` directly:

```rust
pub struct CheckResult {
    pub resolved: ResolvedDocument,
    pub diagnostics: Vec<Diagnostic>,
}
```

`CheckResult::resolved()` now returns `&ResolvedDocument` (a reference into the cache) rather than rebuilding the view per call. `CheckResult::document()` is a forwarder to `resolved.document()` for callers that only need the AST.

The previously-separate `persons: HashMap<&str, &PersonStmt>` and `marriages: HashMap<&str, &MarriageStmt>` indexes collapse into the single `entities` map. Per-kind lookup (`person(id)` / `marriage(id)`) checks the stored `kind` and dereferences the statement at the recorded index.

`Arc<Document>` integrates naturally with the `Arc<str>` source already adopted in `kul-lsp::convert::LineIndex` and `kul-lsp::state::Document` ([refactor #3](../architecture.md)). The shared-immutable-data idiom is already paying for itself in the LSP cache; this extends the same shape to the AST itself.

## Consequences

- The LSP document cache (`state::Document`) holds one `CheckResult` per open URI, and every request handler reads through `doc.check.resolved` directly — no `semantic::resolve` call per keystroke. For an N-statement document this saves an O(N) hashmap rebuild on every hover/completion/definition/etc. request. The performance test in `crates/kul-lsp/tests/perf.rs` continues to assert <500ms on 1000-statement documents.
- The borrowed view types — `Node<'a>`, `EntityNode<'a>`, `EntityTarget<'a>`, `EntityRef<'a>`, `ParentLink<'a>` — keep their lifetime parameters. They're transient view types whose `'a` is now the elided lifetime of `&resolved`. Pattern matches and method calls in feature modules continue to work as before; the only difference is what the lifetime is *measured against* (a `&ResolvedDocument` rather than a `&'a Document`).
- `ResolvedDocument: Send + Sync` (because every field is). The LSP server can hold one in `tokio::sync::RwLock<HashMap<Url, Document>>` without further wrapping.
- `Document` itself does **not** need a `Send + Sync` bound beyond what it already had (all fields are owned plain data: `String`, `Vec`, primitive spans). `Arc<Document>` works without any new derives.
- `entity()` returns `Option<EntityRef<'_>>` constructed on the fly from the stored `ResolvedEntity` — one extra match arm per query. The cost is a few cycles; the alternative (storing `&'a Ident` in the map) is what we're moving away from.
- New invariant inside `ResolvedDocument`: every `ResolvedEntity::statement_idx` points at a statement of the matching `kind`. Maintained by `resolve()` and asserted via `unreachable!` in `person`/`marriage` lookups. If the invariant is ever broken, the panic surfaces immediately rather than silently returning a wrong result.

## Anti-suggestions (do not re-propose)

- **"Re-introduce `ResolvedDocument<'a>` and store it next to `Document` via `ouroboros` / `yoke`."** Adds a dependency for a problem `Arc<Document>` solves with stdlib types. Self-referential macros also obscure the borrow story; the explicit Arc is easier to reason about for both humans and AI agents.
- **"Use raw pointers (`*const PersonStmt`) inside the index."** Faster lookup, but unsafe, and fails the "AI-agent DX" goal — a future agent extending the seam would have to reason about lifetime invariants the type system isn't checking.
- **"Drop the index entirely and search `document.statements` linearly per query."** O(N) per lookup; defeats the cache that the LSP exists to amortise.
- **"Make `ResolvedDocument` itself `Arc`-wrapped at every call site."** Unnecessary — a single `&ResolvedDocument` borrow already lets every feature query through it. The Arc is internal to the implementation; consumers never see it unless they ask via `document_arc()`.
- **"Restore the `pub document: Document` field on `CheckResult`."** Was only public because the type couldn't expose a method that returned `&'a Document` cleanly. Now `document()` is a one-line accessor with stable contract; callers should use it.

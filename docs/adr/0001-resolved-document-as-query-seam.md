# ADR 0001 — `ResolvedDocument` is the kinship-query seam

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

The validator and the cycle-detector both need to ask questions of a
parsed Kula document: "who are this person's bio parents?", "who is the
spouse of this marriage?", "is this id declared?". Initially each call
site walked `Document.statements` directly and pattern-matched on
`Statement::Person` / `Statement::Marriage`, plus poked the
`HashMap<&str, &PersonStmt>` indexes that `semantic::resolve` returned.

Eleven of the thirteen validator rules ended up with the same outer
loop. The cycle-detector duplicated parent-graph reconstruction. When a
question got even slightly cross-cutting ("the spouses of this
marriage, skipping unresolved ones"), each rule re-derived it inline.
The hypothetical seam at `ResolvedDocument` was not real — adapters
went around it.

## Decision

`ResolvedDocument` is the canonical query interface for resolved
documents. All cross-reference and kinship questions are answered by
methods on this type:

- `persons()`, `marriages()` — source-order iteration.
- `person(id)`, `marriage(id)`, `entity(id)` — id lookup.
- `spouses_of(&MarriageStmt)` — yields the resolved spouses, skipping
  unresolved refs (which rule 2 has already reported).
- `parents_of(&PersonStmt)` — yields the union of bio + adoptive
  parent links, each tagged with the `&PersonStmt` of the parent and
  the source span of the link.

The underlying `HashMap` indexes are `pub(crate)`. Internal helpers
inside the `semantic` module may still iterate `document.statements`
directly when source-order traversal is the contract (e.g. rule 2,
which runs as part of `resolve` itself); external callers always go
through the methods.

`Document` itself remains accessible via `ResolvedDocument::document()`
for downstream consumers that need the raw AST (e.g. a future LSP that
maps file offsets to statements).

## Consequences

- New kinship questions land as new methods on `ResolvedDocument`, not
  as inline AST walks at the call site. This is the answer to "where
  does this question live?" for any future rule or feature.
- The Phase 3 LSP plugs into the same query surface without any crate
  reshape. Hover, completion, code actions all phrase themselves as
  questions in this vocabulary.
- The AST nodes (`PersonStmt`, `MarriageStmt`, `AdoptionSub`) carry
  field accessors (`name()`, `born()`, `start()`, etc.) so callers do
  not enumerate their `Vec<*Field>` storage either. Field-storage
  shape is a private detail of the AST module.
- The validator file shrinks: each rule is now its rule logic plus a
  query call, not a query call plus a rule logic plus a traversal
  plus a HashMap lookup.

## Anti-suggestions (do not re-propose)

- "Expose `document.statements` to consumers so they don't need to use
  `persons()` / `marriages()`" — bypasses the seam; the whole point is
  that source-order walking is the seam's job, not the caller's.
- "Add a `Visitor` trait" — would replace methods with double-dispatch
  for no clear leverage given 13 rules and a stable AST shape.
  Reconsider only if rules grow to >30 or the AST starts changing
  weekly.
- "Move kinship queries onto AST node types" — AST nodes don't know
  about resolution; they don't know which spouse-id resolves to which
  person. Kinship is a property of the resolved document, not the
  individual statement.

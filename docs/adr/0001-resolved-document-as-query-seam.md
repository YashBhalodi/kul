# ADR 0001 — `ResolvedDocument` is the kinship-query seam

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

The validator and the cycle-detector both need to ask questions of a parsed Kul document: "who are this person's bio parents?", "who is the spouse of this marriage?", "is this id declared?". Initially each call site walked `Document.statements` directly and pattern-matched on `Statement::Person` / `Statement::Marriage`, plus poked the `HashMap<&str, &PersonStmt>` indexes that `semantic::resolve` returned.

Eleven of the thirteen validator rules ended up with the same outer loop. The cycle-detector duplicated parent-graph reconstruction. When a question got even slightly cross-cutting ("the spouses of this marriage, skipping unresolved ones"), each rule re-derived it inline. The hypothetical seam at `ResolvedDocument` was not real — adapters went around it.

## Decision

`ResolvedDocument` is the canonical query interface for the resolved view of a Kul project. All cross-reference and kinship questions are answered by methods on this type. Per [ADR-0014](./0014-file-identity-and-per-file-namespaces.md) the project may now hold multiple files; per-id queries take a `FileId` so the seam works the same whether the project is one file or many:

- `persons()`, `marriages()`, `statements()` — source-order iteration across every `.kul` file in the project.
- `persons_in(file)`, `marriages_in(file)`, `statements_in(file)` — per-file iteration.
- `person(file, id)`, `marriage(file, id)`, `entity(file, id)` — id lookup, scoped to the file (per ADR-0014's per-file namespaces).
- `spouses_of(file, &MarriageStmt)` — yields the resolved spouses inside `file`, skipping unresolved refs (which rule 2 has already reported).
- `parents_of(file, &PersonStmt)` — yields the union of bio + adoptive parent links, each tagged with the `&PersonStmt` of the parent and the source span of the link.

The underlying `HashMap<FileId, HashMap<id, ResolvedEntity>>` index is private to the `semantic` module. Internal helpers may still iterate a `KulFile`'s statements directly when source-order traversal is the contract (e.g. rule 2, which runs as part of `resolve` itself); external callers always go through the methods.

`Document` (the multi-file project) remains accessible via `ResolvedDocument::document()` for downstream consumers that need access to source bytes by `FileId`, file names, or the `KulFile` list.

## Consequences

- New kinship questions land as new methods on `ResolvedDocument`, not as inline AST walks at the call site. This is the answer to "where does this question live?" for any future rule or feature.
- The LSP plugs into the same query surface without any crate reshape. Hover, completion, code actions all phrase themselves as questions in this vocabulary.
- The AST nodes (`PersonStmt`, `MarriageStmt`, `AdoptionSub`) carry field accessors (`name()`, `born()`, `start()`, etc.) so callers do not enumerate their `Vec<*Field>` storage either. Field-storage shape is a private detail of the AST module.
- The validator file shrinks: each rule is now its rule logic plus a query call, not a query call plus a rule logic plus a traversal plus a HashMap lookup.

## Anti-suggestions (do not re-propose)

- "Expose `document.statements` to consumers so they don't need to use `persons()` / `marriages()`" — bypasses the seam; the whole point is that source-order walking is the seam's job, not the caller's.
- "Add a `Visitor` trait" — would replace methods with double-dispatch for no clear leverage given 13 rules and a stable AST shape. Reconsider only if rules grow to >30 or the AST starts changing weekly.
- "Move kinship queries onto AST node types" — AST nodes don't know about resolution; they don't know which spouse-id resolves to which person. Kinship is a property of the resolved document, not the individual statement.

# ADR 0015 — Global project namespace for multi-file Kul projects

**Status:** Accepted
**Date:** 2026-05-16
**Deciders:** owner
**Supersedes (in part):** [ADR-0014](./0014-file-identity-and-per-file-namespaces.md) — Position B (per-file namespaces)

## Context

ADR-0014 deliberately left the cross-file resolution question open. It introduced `FileId`, `FileSpan`, `KulFile`, and the multi-file `Document` so the toolchain could *talk about* multiple files at the type level, but committed only to **per-file id namespaces**: R01 fired only within one file, R02 looked up references only within the declaring file, and the resolved id index was a `HashMap<FileId, HashMap<String, ResolvedEntity>>`. The rationale at the time was explicit: the structural retrofit should not decide a language feature on issue #63's behalf — "Position B keeps the namespace policy minimal and lets #63's PRD design cross-file resolution from a clean slate."

[PRD 0001 — Multi-file Kul projects with cross-file id references](../prd/0001-multi-file-kul-projects.md) (tracking issue #63) is that PRD. The product question it answers: *how do authors of large family trees split a `.kul` file across multiple files while still being able to reference the same person or marriage from any file?* Three positions were considered (full history in [the PRD's grilling session](../prd/0001-multi-file-kul-projects.md#user-stories)):

- **Position A — global namespace across the project.** Every declared id is visible from every file. Cross-file references are bare names; collisions across files fire R01.
- **Position B — per-file namespaces (status quo).** Each file is its own scope; cross-file references would need explicit syntax.
- **Position C — per-file with explicit imports.** Per-file by default, with `from "other.kul" import alice` (or similar) syntax to opt into visibility.

The PRD's load-bearing author-facing constraint is "no imports": authors splitting a dense tree across files don't want to pay syntactic ceremony at every cross-file reference. Position C buys a property authors don't want (explicit boundaries) at a real ergonomic cost (every reference is now positional or qualified). Position B is structurally impossible for the use case — splitting a tree into one-file-per-family means *every* parent-child reference crosses a file boundary; with per-file namespaces, no such reference would resolve.

That leaves Position A — global namespace — as the only position that satisfies the "no imports" constraint without inventing syntax. The grilling session reached this conclusion before this ADR was written.

## Decision

### Pure global namespace across every `.kul` file in the project

A Kul **project** is a directory containing a `kul.yml` manifest plus one or more `*.kul` files. Every person- or marriage-id declared in any of those files is visible from every file in the project. There is no `import` statement, no namespace prefix, no qualified-reference syntax. The file boundary is purely organizational; the project is one logical namespace.

A single-file project is just a project with `N=1`: nothing changes from the author's perspective; nothing changes from the toolchain's perspective beyond the implementation detail that the resolver now walks every file.

### `ResolvedDocument`'s id index is flat

The previous `HashMap<FileId, HashMap<String, ResolvedEntity>>` becomes `HashMap<String, ResolvedEntity>`, with each stored entry carrying the `FileId` of its declaring file so query methods can reconstruct the original location. The per-id query methods drop their leading `FileId` parameter:

- `resolved.person(id)` — was `resolved.person(file, id)`.
- `resolved.marriage(id)` — was `resolved.marriage(file, id)`.
- `resolved.entity(id)` — was `resolved.entity(file, id)`.

Kinship traversal methods drop the file parameter the same way:

- `resolved.spouses_of(marriage)` — was `resolved.spouses_of(file, marriage)`.
- `resolved.parents_of(person)` — was `resolved.parents_of(file, person)`.

Project-wide iteration helpers (`persons()`, `marriages()`, `statements()`) stay; per-file iteration helpers (`persons_in(file)`, `marriages_in(file)`, `statements_in(file)`) stay too — the LSP needs them for per-URI symbol listings and the source-order diagnostic walk in the validator needs them to keep R03–R12 grouped per file. `node_at(file, offset)` and `statement_at(file, offset)` keep their `FileId` parameter because byte offsets are inherently per-file: the same byte offset means different things in different files.

### R01 fires across files

When two declarations of the same id appear anywhere in the project, R01 fires. The **primary span** anchors at the second declaration in file-discovery order (ties broken by byte offset within the file); a related-span points to the first declaration. The message wording flips from "every id must be unique across all persons and marriages" (per-file phrasing) to "every id must be unique across the project."

### R02 traverses the project-wide id index

When a reference (a spouse position, a `birth` marriage-ref, or an `adoption` marriage-ref) names an id, the resolver looks up that id in the flat project-wide index. The message wording flips from "no person with id X is declared in this file" to "no person with id X is declared in the project."

The wrong-kind sub-cases (a spouse position naming a marriage id, or a `birth` ref naming a person id) carry a related-span pointing to the prior declaration — which now may live in a different file. Diagnostic rendering (per ADR-0006) filters cross-file related-spans to the single-source miette frontend; the rendering surface remains unchanged.

### R13 walks the project-wide parent graph in one pass

The parent graph spans every file in the project. `cycles::find_cycles(resolved)` (dropping its `FileId` parameter) starts from `resolved.persons()` and walks parent links via `resolved.parents_of(person)`. A cycle that spans two files is reported as one cycle, not as two per-file fragments. Each related-span on the diagnostic anchors at the file containing the corresponding parent-link — which is the child's owning file, since the parent-link span is the `birth`/`adoption` ref inside the child's declaration.

### New `KUL-M06` — empty project

A project with a `kul.yml` manifest but zero sibling `.kul` files is structurally invalid. The top-level `kul_core::check` entry point now emits `KUL-M06` (severity error) anchored at the manifest's start when `inputs.is_empty()` and `manifest_yaml` is non-empty. The "non-empty `manifest_yaml`" guard preserves the in-memory-callers path (the WASM `format` helper, ad-hoc unit tests that pass `""` for the manifest source): they are not asserting a project, just running the pipeline against in-memory inputs.

### Cross-file LSP features land in a later slice

The CLI, LSP, and WASM surfaces continue to feed `kul_core::check` one `InputFile` at a time as of this slice. The resolver's project-wide capability exists, the test corpus exercises it, but multi-file *consumers* are split out into their own follow-up issues (per the PRD's issue-breakdown plan):

- Slice 2 (CLI): arg-less `kul validate` / `format` / `export` discover the project from CWD.
- Slice 4 (WASM): `check(files: Array<{name, source}>, manifest)` lift.
- Slice 5 (LSP): project-keyed cache, broadcast diagnostics.
- Slice 6 (LSP): file watching.

This ADR records the supersession decision and lands with slice 1 (the resolver retrofit + new `examples/07-multi-file-extended-family/` showcase + multi-file test fixtures + perf budget extension).

## Consequences

**Positive.** Authors get the load-bearing property the PRD identified: a `.kul` file split that requires zero rename work and zero new syntax. A parent in `parents.kul` and their children in `kids.kul` reference each other by bare id. The deletion test passes for "no imports" — removing the global namespace would force *some* cross-file ceremony, which the PRD rejected outright.

R13 cycle detection now catches cross-file mistakes (`alice` in `branch-a.kul` adopts a parent declared in `branch-b.kul` who, three hops away in `branch-c.kul`, is adopted by `alice`). Under per-file namespaces these cross-file cycles were structurally undetectable.

The resolver retains all the existing per-file iteration helpers, so per-URI LSP features (document symbols, semantic tokens, per-file diagnostic streams) keep working unchanged. The change is concentrated in the per-id query surface and R01/R02/R13 semantics — exactly the seams the PRD identified.

**Negative.** R01 cross-file ordering pins a "file-discovery order" contract: the second declaration is whatever file the toolchain happens to walk second. For the CLI, this is the alphabetic order `kul-cli` enumerates the directory in (slice 2 will lock this in). For now, the contract is "input order to `kul_core::check`" — adapters are responsible for picking a stable order. Snapshot tests on multi-file R01 must therefore commit to one input order; reordering inputs in a test flips primary/related.

Cross-file related-spans in R02's wrong-kind sub-cases (e.g. "this spouse position names a marriage; the marriage is declared *here*, in a different file") cannot be rendered by miette's single-source frontend. ADR-0006's existing same-file related-span filter handles this — it drops cross-file related-spans from the miette label list. The diagnostic message itself still carries the cross-file context, so the user sees *what* collided; just not a visual snippet for the related site. The LSP filters diagnostics per-URI today, so this is invisible in editor squiggles; the CLI's miette path is the only renderer affected, and the loss is minor (the message is enough). A future deepening could extend `RenderableDiagnostic` to a multi-source rendering path, but neither this slice nor any near-term slice plans it.

The `ResolvedDocument` field shape changes (`HashMap<FileId, HashMap<...>>` → `HashMap<String, ResolvedEntity>`). The resolved entity stores `(kind, file, statement_idx)` instead of `(kind, statement_idx)`, growing by 4 bytes per entry. Negligible on any realistic project.

## Alternatives considered

**Position B — keep per-file namespaces.** Rejected for the structural reason above: splitting a tree across files makes nearly every reference a cross-file reference, and per-file resolution would refuse them all. The PRD's primary use case is impossible under Position B.

**Position C — explicit cross-file imports.** Rejected for the ergonomic reason: every cross-file reference would carry import-statement ceremony (or a qualified-reference prefix), which the PRD's "no imports" constraint rejects. Authors who split a file want the split to be a copy-paste operation, not a refactor.

**Implicit file ordering = lexicographic always.** Rejected at the `kul-core` layer. The order of inputs to `kul_core::check` is the adapter's responsibility — the CLI will sort alphabetically (slice 2), the LSP will hand the LSP cache's iteration order (slice 5), the WASM bridge will pass whatever the JS host built. Locking lexicographic order into `kul-core` would force adapters that have a different natural order to pre-sort. Keeping the contract at "input order is significant" lets each adapter pick what makes sense.

**Cross-file R01 anchored on the *first* declaration with related-span to the second.** Rejected because the diagnostic should point at the *new* declaration, not the *prior* one — the new declaration is what the author needs to change. This matches what the per-file R01 already did (anchored at the second occurrence within a file); the cross-file extension preserves the convention.

**Bump the Kul language version (`kul: "0.1"` → `kul: "0.2"`).** Rejected per the PRD's "no language version bump" decision. Every legal 0.1 single-file project remains valid under project-wide resolution; multi-file is a *new shape*, not a redefinition of an existing one. The additivity principle ([CONTEXT.md](../../CONTEXT.md)) says new shapes that don't break existing sources don't bump the version.

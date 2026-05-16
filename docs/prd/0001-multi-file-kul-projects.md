# PRD 0001 — Multi-file Kul projects with cross-file id references

**Status:** Draft
**Date:** 2026-05-16
**Tracks:** issue #63

## Problem Statement

A Kul author describing a dense family tree quickly outgrows a single `.kul` file. Past a certain density, the file becomes unwieldy to scroll, edit, and reason about. Authors naturally want to break the document into multiple files — one per family, one per branch, one per generation — even though the same person may appear across multiple of those files (e.g. as a spouse in one family's file and as a child in another).

Today this is structurally impossible. A Kul project is `kul.yml` plus exactly one `.kul` file the toolchain consumes; ADR-0014 deliberately introduced the multi-file in-memory shape but no consumer constructs `N>1` yet. Every author who hits the single-file ceiling has no way out — they can't even reference an id declared in a sibling `.kul` file, because per-file id namespaces (ADR-0014) keep each file's resolution scope isolated.

The product cost is real: large family trees are exactly the corpus Kul is designed for, and the single-file ceiling forecloses that use case before authors get there.

## Solution

A Kul **project** is a directory containing a `kul.yml` manifest plus one or more `*.kul` files. Every id declared in any of the project's files is visible from every other file — no `import` statement, no namespace prefix, no qualified reference syntax. The project is one logical namespace; the file boundary is purely organizational.

From an author's perspective:

- They write the same `.kul` syntax as today. The grammar does not change.
- They put as many `.kul` files in their project directory as they want. Each has a sibling `kul.yml` (just one, shared).
- They reference any declared id — person or marriage — from anywhere in the project, by bare name. `birth m_alice_bob` works whether `m_alice_bob` is in the same file or a sibling file.
- They run `kul validate`, `kul format`, or `kul export` from the project directory with no arguments. The toolchain operates on the whole project.
- They open any `.kul` file in their editor and the LSP gives them goto-definition, find-references, and rename across every file in the project.

A single-file project remains a normal project — it just has `N=1`. There is no separate "single-file mode" or "multi-file mode" anywhere in the toolchain.

## User Stories

### Author writing kinship

1. As a Kul author, I want to split my large family tree across multiple `.kul` files in the same directory, so that each file stays small enough to scroll, read, and edit.
2. As a Kul author, I want to reference a person declared in one file from a marriage statement in another file using only the bare id, so that the file boundary does not impose syntactic ceremony on cross-file references.
3. As a Kul author, I want to reference a marriage declared in one file from a `birth` or `adoption` sub-statement in another file using only the bare id, so that children of cross-file marriages remain expressible without ceremony.
4. As a Kul author, I want my existing single-file `.kul` project to keep working unchanged after multi-file support lands, so that the change is purely additive from my perspective.
5. As a Kul author splitting one file into many, I want to keep my existing ids (no rename required just because I moved the declaration), so that splitting is a pure copy-paste-and-delete operation, not a rewrite.
6. As a Kul author, I want a clear, anchored error when I accidentally declare the same id in two files, so that the collision is caught at validate time rather than producing silently surprising behavior.
7. As a Kul author, I want the duplicate-id error to point at *both* declarations (the second one as the primary, the first as a related span), so that I can immediately see which two declarations collide and pick one to rename.
8. As a Kul author, I want a clear, anchored error when I reference an id that is not declared anywhere in the project, so that typos and orphan references surface at validate time.
9. As a Kul author, I want subdirectories of my project directory to be invisible to the toolchain, so that I can keep notes, archives, or backups next to my `.kul` files without affecting validation.
10. As a Kul author, I want non-`.kul` files in my project directory (README, .gitignore, editor backups) to be silently ignored, so that the project directory can also hold supporting files.
11. As a Kul author, I want a clear error when my project directory has a `kul.yml` but no `.kul` files, so that an empty project is flagged loudly rather than silently producing an empty graph.

### Author using the CLI

12. As a Kul author, I want to run `kul validate` from my project directory with no arguments, so that I do not have to name files individually.
13. As a Kul author, I want `kul validate` to report diagnostics for every file in the project in a single run, so that I see all the project's problems at once.
14. As a Kul author, I want `kul format` to format every `.kul` file in my project with one command, so that formatting an entire project is a single keystroke.
15. As a Kul author, I want `kul export` to produce a single graph for the entire project, so that downstream consumers (renderers, visualizers) see a unified kinship graph rather than `N` per-file slices.
16. As a Kul author, I want a clear error when I run `kul validate` (or `format`, or `export`) from a directory that is not a Kul project root, so that the toolchain doesn't silently misbehave when I'm in the wrong directory.

### Author using the editor (LSP)

17. As a Kul author editing one `.kul` file in my editor, I want goto-definition on a person or marriage id to jump to its declaration even if the declaration lives in a sibling file, so that I can navigate the project without thinking about file boundaries.
18. As a Kul author, I want find-references on a person or marriage id to surface every use across the entire project, so that I can see all the places a given entity participates.
19. As a Kul author, I want rename on a person or marriage id to update every reference across the entire project, so that renaming is safe and complete.
20. As a Kul author, I want the editor's Problems pane to show errors in every file of my project — even files I haven't opened — so that I have a complete view of the project's health without manually opening each file.
21. As a Kul author, I want completion suggestions for cross-file references to include ids declared in sibling files, so that I can author cross-file references without needing to remember exact spellings.
22. As a Kul author, I want the LSP to keep the project view in sync when I create, rename, or delete `.kul` files outside the editor (via `git checkout`, the shell, or Finder), so that diagnostics never go stale relative to disk.
23. As a Kul author working in a VSCode workspace that contains multiple Kul projects (each its own directory with its own `kul.yml`), I want each project to be analyzed independently, so that diagnostics in one project never spill into another.

### Author using the WASM bridge

24. As a JS host integrating `@kullang/wasm`, I want to pass an array of `{name, source}` to `check` and `exportGraph`, so that I can analyze multi-file projects from a browser or Node application.
25. As a JS host, I want `format` to keep taking a single source string, so that formatting one file (e.g. in a live editor playground) does not require building an array wrapper.

### Author migrating an existing project

26. As an existing single-file Kul author, I want my project to remain valid byte-for-byte after the change ships, so that I have no migration work to do.
27. As an existing single-file author who wants to split into multiple files, I want a documented example in the `examples/` corpus demonstrating cross-file references, so that I have a working reference to model my own split on.

### Toolchain maintainer

28. As a toolchain maintainer, I want the in-memory pipeline to continue operating on `Document` and `ResolvedDocument` exactly as it does today (the seams already accept multi-file shapes from ADR-0014), so that the structural retrofit is concentrated in the resolver and the adapter surfaces rather than scattered across the pipeline.
29. As a toolchain maintainer, I want a deep module that turns a project-root path into `(manifest_yaml, Vec<InputFile>)` plus typed errors, so that CLI subcommands and LSP project discovery share one well-tested filesystem entry point rather than each rolling their own.
30. As a toolchain maintainer, I want the LSP's cache keyed by project root rather than URI, so that multiple open files in the same project share one cached `CheckResult` rather than re-running the resolver per URI.
31. As a toolchain maintainer, I want the existing performance budget (<100ms / 1000 statements) reinterpreted as per-project, with a multi-file fixture added to the existing perf test, so that we catch regressions in multi-file scaling at the same time we catch single-file regressions.

## Implementation Decisions

### Language and namespace

- **No grammar change.** No new keywords, no new statement types, no qualified-reference syntax. The `.kul` grammar in `spec/01-*.md` through `spec/13-*.md` is unchanged.
- **Pure global namespace.** Every person or marriage id declared in any file of the project is visible from every file in the project. There is no `import` statement and no namespace prefix syntax.
- **Per-file scoping (ADR-0014's Position B) is superseded** by project-wide scoping. A new ADR records the supersession with the reasoning from the grilling session: "no imports" is the load-bearing author-facing constraint; global namespace is the simplest implementation that satisfies it.

### Project structure

- **Project root = directory containing `kul.yml`.** All `*.kul` files in that exact directory are part of the project. Subdirectories are invisible to the toolchain. Non-`*.kul` files (README, .gitignore, editor backups) are silently ignored.
- **No recursive discovery.** Subdirectories are not walked. This is a flat-directory model.
- **No `files:` enumeration in `kul.yml`.** The manifest schema does not gain a `files:` field; project membership is implicit in the directory listing.
- **Manifest schema unchanged.** `kul.yml` still carries only `kul: "0.1"`. Any future project-level fields (export config, project name) land additively in later issues.

### Validator rules

- **R01 (duplicate id)** fires when two declarations of the same id appear anywhere in the project. The primary span anchors at the second declaration in file-discovery order (ties broken by byte offset within a file); a related-span points to the first declaration.
- **R02 (unresolved reference)** fires when a referenced id is not declared anywhere in the project. The message wording stays the form it has today; "in the project" is implicit because the project is the namespace.
- **R13 (parenthood cycles)** detects cycles in the project-wide parent graph rather than per-file. The cycle detector in `cycles.rs` reads from `ResolvedDocument.parents_of` which now spans files.
- **New code KUL-M06 (empty project).** A `kul.yml` with zero sibling `*.kul` files emits `KUL-M06` anchored at the manifest. Severity: error.

### Resolver and ResolvedDocument

- **Flat project-wide id index.** `resolved.person(id)`, `resolved.marriage(id)`, `resolved.entity(id)` drop the `FileId` parameter. Lookups consult one map for the whole project.
- **Per-file iteration helpers retained.** `resolved.persons_in(file)`, `resolved.marriages_in(file)`, `resolved.statements_in(file)` continue to exist; the LSP uses them for document-symbol listings, breadcrumbs, and per-URI diagnostic filtering.
- **Project-wide iteration helpers exist alongside.** `resolved.persons()`, `resolved.marriages()`, `resolved.statements()` walk every file in source order.
- **`node_at` and `statement_at` keep `(file, offset)`** parameters because byte offsets are inherently per-file.

### CLI

- **All subcommands are CWD-rooted and accept no positional `<file>` argument.** `kul validate`, `kul format`, `kul export` discover the project from the current working directory.
- **Project-root validation runs first.** Each subcommand checks for `kul.yml` in CWD; if absent, errors out with a clear message ("not a Kul project root: no kul.yml in current directory").
- **`kul format` writes back to every `.kul` file** in the project. Project-wide format is a CLI-level iteration over a per-file primitive; the formatter itself remains per-file.
- **`kul export` emits one envelope** carrying the union of all files' persons, marriages, and parenthood links. No file attribution in the exported graph.

### LSP

- **Cache keyed by project root.** One `CheckResult` per project, with per-URI overlay holding editor-buffer source (or `None` if a file is only on disk). `did_change` mutates the overlay and re-runs check for the project, not the URI.
- **Lazy multi-project discovery.** On first `did_open` of a `.kul` URI, the server discovers the project (find sibling `kul.yml`, enumerate `*.kul` siblings, read each from disk). Eager workspace-wide scan is out of scope.
- **File watching via `workspace/didChangeWatchedFiles`.** The LSP registers globs for `**/*.kul` and `**/kul.yml`. On a fired event, the affected project's files are reloaded and check is re-run. (File-watching is tracked as its own follow-up issue per the issue-breakdown plan.)
- **Diagnostics broadcast project-wide.** The LSP publishes diagnostics for every file in the active project — not just open URIs — so the Problems pane reflects project-wide health. Files leaving the project (deleted on disk, moved out) get an empty diagnostic publish to clear stale entries.

### WASM bridge

- **`check(files: Array<{name, source}>, manifest)`** — replaces today's `check(source, manifest)`. JS hosts enumerate files themselves.
- **`exportGraph(files: Array<{name, source}>, manifest, options)`** — same shape change.
- **`format(source: string)` unchanged.** Format is per-source by nature; no symmetry-only wrapper is added.
- **Breaking change to `@kullang/wasm`** captured in the CHANGELOG. Per the project's "no real users yet" policy, the breakage is intentional and lands in one PR.

### Examples corpus migration

- **Per-example subdirectories.** Each existing `examples/NN-<name>.kul` moves to `examples/NN-<name>/<name>.kul` with a sibling `kul.yml`. This restructure is independently honest (today's six examples are six teaching narratives, not one shared project) and unblocks #63's project-wide namespace by removing the latent cross-file id collisions.
- **New multi-file showcase.** `examples/07-multi-file-extended-family/` ships with 2–3 `.kul` files demonstrating cross-file references (a parent declared in one file, their children declared in another). The spec section on multi-file projects links to this example.

### Spec deliverables

- **`spec/14-project-manifest.md`** gains a multi-file section: project = directory with `kul.yml` + one-or-more `*.kul` files; ids globally unique within the project; flat directory rule; subdirectories ignored; non-`.kul` files ignored; new `KUL-M06` listed.
- **`spec/07-validation-rules.md`** updates R01 ("duplicate id within the project") and R02 ("id not declared in the project") wording. KUL-M06 listed alongside the other manifest codes.
- **`spec/16-export-schema.md`** gains an explicit note: one envelope per project, regardless of file count.
- **No other spec sections change.** The grammar / lexical structure sections are untouched.

### Other documentation

- **New ADR** — global project namespace. Supersedes ADR-0014's per-file commitment with explicit reasoning. Records the "no imports" author-facing constraint as the load-bearing decision.
- **`CONTEXT.md`** updates: ResolvedDocument's "per-file id index" wording flips to "project-wide id index"; per-file iteration helpers section stays; new short paragraph on project-wide namespace.
- **`docs/architecture.md`** updates: pipeline diagram caption notes that `Document.kul_files` now genuinely has `N>=1` consumers; LSP request-flow section notes the project-keyed cache; "Where to add X" recipes for validator rules and LSP features stay correct.
- **No language version bump.** `kul: "0.1"` covers the multi-file behavior under the additivity principle — every legal 0.1 single-file project remains valid; multi-file is a new shape, not a redefinition.

### Module sketch — deep modules to build or deepen

- **`semantic.rs` (existing, deepened).** The resolver's id index flips from per-file to project-wide. R01 collision detection moves into project-wide insertion logic. Interface narrows (`person`, `marriage`, `entity` drop their `FileId` param).
- **`validator.rs` (existing, lightly modified).** R02 and R13 traverse the project-wide structures the resolver now exposes. R13's `cycles.rs` reads project-wide parents.
- **`manifest.rs` (existing, lightly modified).** Gains `KUL-M06` (empty project).
- **Project loader (new).** A deep module that takes a path (CWD for CLI, sibling-of-URI for LSP) and returns `(manifest_yaml, Vec<InputFile>)` or a typed error. Encapsulates: `kul.yml` discovery, manifest read, `*.kul` enumeration, IO errors. Shared by `kul-cli` and `kul-lsp`. Lives in `kul-core` (or a new `kul-loader` crate if dependency direction requires it; the cleaner placement depends on the dependency graph at implementation time).
- **`kul-lsp` `state.rs` (existing, restructured).** Cache key becomes the project root path. Each entry holds the cached `CheckResult` plus a per-URI overlay map. Cache lifecycle (insert on first project discovery, evict when no URIs remain open and the file-watcher reports no project files remain) is the deep module here.
- **`kul-lsp` server (existing, modified).** `did_open` discovers project, `did_change` mutates overlay, `workspace/didChangeWatchedFiles` registration, diagnostic publishes broadcast to every project file.
- **`kul-wasm` `lib.rs` (existing, modified).** `check` and `exportGraph` signatures lift to `files: Array<{name, source}>`; `format` unchanged.

## Testing Decisions

Tests should target **external observable behavior** — diagnostics emitted, `ResolvedDocument` query outputs, CLI exit codes and stdout/stderr, LSP messages published, WASM bridge return shapes. Not internal data-structure layouts. The existing project conventions (snapshot tests via `insta` per ADR-0003; positive corpus in `examples/`; perf as a test rather than a bench) carry over without modification.

### Modules to test

1. **Project loader (new deep module).** Fixture-directory inputs in `crates/*/tests/fixtures/`; assertions on the returned `(manifest_yaml, Vec<InputFile>)` shape and on typed errors. Covers: happy path with `kul.yml` + N `.kul` files; missing `kul.yml`; empty project (zero `.kul` files); subdirectories ignored; non-`.kul` files ignored; non-UTF-8 names or unreadable files. Snapshot the error rendering for the failure cases.

2. **Project-wide resolver (`semantic::resolve`).** Multi-file `Document` inputs constructed in-memory or via the project loader. Assertions:
   - Cross-file reference resolves (R02 quiet) when an id declared in file A is referenced in file B.
   - Cross-file duplicate id (R01) fires with primary on the second declaration and related-span to the first.
   - Per-file iteration helpers (`persons_in(file)`) return only that file's declarations.
   - Project-wide iteration helpers (`persons()`) return every file's declarations.
   - Empty project produces `KUL-M06`.
   Snapshot diagnostic output for each scenario.

3. **Project-keyed LSP cache.** A test harness that drives the cache with sequences of `did_open` / `did_change` / `did_close` / file-watch events and asserts:
   - Two `did_open`s in the same project share one cached `CheckResult`.
   - `did_change` on one URI updates only that URI's overlay; other URIs in the project read the disk-backed source.
   - `did_close` on the last URI of a project triggers eviction.
   - File-watch events trigger appropriate cache invalidation.
   Use the existing minimal LSP client pattern from `crates/kul-lsp/tests/`.

4. **End-to-end multi-file via existing test layers.**
   - **CLI** integration test (`crates/kul-cli/tests/`): a fixture project with cross-file references; assert `kul validate`, `kul format`, `kul export` produce expected output. Snapshot.
   - **WASM** snapshot test (`crates/kul-wasm/tests/`): multi-file `check` and `exportGraph` invocations mirror the existing single-file shape. TypeScript usage (`tests/typescript/usage.ts`) exercises the new array signature.
   - **LSP** integration test (`crates/kul-lsp/tests/`): cross-file goto-definition, find-references, rename, and broadcast diagnostics.

5. **Performance budget.** Extend the existing `crates/kul-lsp/tests/perf.rs` with a multi-file fixture (e.g. 10 files × 100 statements = 1000 statements total) and assert the same <500ms wall-clock bound. The comment in the test records the actual target.

### Prior art

- Snapshot tests via `insta`: existing examples in `crates/kul-core/tests/validator.rs`, `crates/kul-core/tests/export.rs`. Each new diagnostic or query output gets a snapshot file under `crates/kul-core/tests/snapshots/`.
- LSP integration tests with a hand-rolled stdio client: `crates/kul-lsp/tests/` already has the pattern; the new multi-file tests extend it.
- CLI integration tests via `assert_cmd`: existing in `crates/kul-cli/tests/`; new tests for the arg-less subcommand shape follow the same shape.
- Fixture directories: `crates/*/tests/fixtures/` (or `examples/` for shared corpus). Multi-file test fixtures live in fixtures, not in `examples/` (which is the curated showcase).

## Out of Scope

- **Per-file CLI invocation.** `kul validate <file>.kul` and the equivalents are *not* re-introduced in this PRD. A future issue can add per-file flags if a real use case appears (e.g. CI-scoped validation, language-server-protocol-less editor integration).
- **Recursive project discovery.** Subdirectories are not walked. If a future use case demands sub-folder organization within a project, the flat-directory rule can be relaxed additively without breaking existing flat projects.
- **`files:` enumeration in `kul.yml`.** The manifest does not gain a project-membership list. Discovery stays implicit.
- **Project name, export config, or other manifest fields.** The manifest schema stays single-field for #63. Any future fields land additively in later PRDs.
- **Walk-up manifest discovery.** The CLI requires `kul.yml` to be in CWD, not in any ancestor. A future ergonomic improvement (`kul init`, walk-up, or both) is a separate issue.
- **Eager workspace-wide LSP scan.** The LSP discovers projects lazily on first `did_open`. A workspace-wide "show me all errors across all projects" command is a future issue.
- **Project-wide WASM `format`.** `@kullang/wasm`'s `format` stays single-source. JS hosts iterate if they want to format a whole project.
- **Identifier styling guidance.** The spec does not recommend a naming convention (`smith_alice` vs `alice` etc.). If best-practices guidance becomes warranted, it lands in a separate non-normative doc.
- **Migration tooling for non-existent users.** The project has no real users yet; no migration script, deprecation period, or compatibility shim is provided. Existing single-file projects continue to work; multi-file is purely additive.

## Further Notes

### Issue breakdown

This PRD is expected to fan out into multiple GitHub issues during the next triage pass. Natural seams for splitting:

1. **Core resolver retrofit.** `semantic.rs`'s flat id index, R01/R02 scope change, R13's project-wide cycle walk, new `KUL-M06`. Ships with the project loader (if placed in `kul-core`) and the existing test corpus reshaped onto multi-file fixtures.
2. **CLI multi-file subcommands.** `kul validate` / `kul format` / `kul export` lose positional args, run on CWD. Depends on (1).
3. **Examples corpus migration.** Per-example subdirectories + new `07-multi-file-extended-family/`. Can ship alongside (1) or as its own PR; either works since the corpus reshape is mechanical.
4. **WASM multi-file bridge.** `check` and `exportGraph` lift to `files` arrays; `format` unchanged. Depends on (1); independent of (2).
5. **LSP project-keyed cache + broadcast diagnostics.** `state.rs` restructure, `did_open` discovery, diagnostic broadcast. Depends on (1).
6. **LSP file watching.** `workspace/didChangeWatchedFiles` registration and handlers. Builds on (5); explicitly carved out as its own issue per the grilling session.
7. **Spec + ADR + docs updates.** Land alongside the code that motivates each piece (per the project's atomicity discipline). A small ADR-only PR may be useful at the start to record the global-namespace supersession of ADR-0014's Position B.

The triage label flow on each issue follows the existing convention; the breakdown step uses the `to-issues` skill.

### Relationship to ADR-0014

ADR-0014's "Position B — per-file namespaces" was framed at the time as the **minimum commitment** needed to land the structural refactor without prejudicing this PRD. Its alternatives section explicitly notes: "Position B keeps the namespace policy minimal and lets #63's PRD design cross-file resolution from a clean slate." This PRD picks the choice ADR-0014 deferred to it (global project namespace) and a new ADR will record the supersession in the same change as the resolver retrofit.

### Performance posture

The existing perf budget (<100ms / 1000 statements, asserted at 500ms with 5× CI slack) is reinterpreted as per-project. The pipeline is bounded by total bytes parsed, not file count; multi-file projects scale linearly with statement count. For projects in the 5000–25000-statement range, the current architecture should land in the 50–400ms range — usable. Above that, incremental analysis (re-resolving only the files that changed) is an optimization track that is **not** in scope here.

### Risks

- **Examples corpus churn.** Per-example directory migration touches every `examples/NN-*.kul` path and every test that references those paths. Snapshot test names that encode the old path (`01-single-couple.kul`) need updating. The change is mechanical but broad; reviewers should expect a large file-rename diff alongside small content changes.
- **LSP cache key transition.** The shift from URI-keyed to project-keyed caching changes the lifecycle of every cached entry. Careful test coverage on `did_open`/`did_change`/`did_close` sequences is what catches regressions here, not type-checking.
- **File-watching reliability.** OS-level file-watching has historically been a source of LSP flakiness in other ecosystems (rust-analyzer, gopls). The dedicated file-watching issue (item 6 in the breakdown) should plan for diagnostic instrumentation from day one.

# ADR 0014 — File-identity types and per-file id namespaces

**Status:** Accepted (Position B superseded by [ADR-0015](./0015-global-project-namespace.md))
**Date:** 2026-05-09
**Deciders:** owner

> The file-identity types and the manifest-diagnostics decision in this ADR remain in force. The "Position B — per-file namespaces" decision is superseded by [ADR-0015](./0015-global-project-namespace.md), which adopts the global-project-namespace position this ADR's "Alternatives considered" left to PRD 0001.

## Context

Issue #69 promised a manifest. Issue #63 names the multi-file feature. This issue (#70) is the *type-system retrofit* that lets #63 build on file-aware spans without re-touching every diagnostic, AST node, validator rule, and LSP feature module along the way. The change is structural: `kul-core` previously had one `ast::Document` (one parsed `.kul` file) and a `Diagnostic` whose `primary` was a bare `ByteSpan` into that file's source. Adapters threaded "which file does this span belong to?" implicitly: every check call took one source string, every diagnostic anchored into it, every LSP request operated on one URI.

That implicit contract breaks the moment a project has more than one input file. Even single-file v1 needs a clean home for **manifest diagnostics** that point into `kul.yml` rather than the `.kul` source — a follow-through #69 explicitly deferred to this issue, where the seam to do it cleanly was about to exist anyway.

Two separable design questions fall out:

1. **How does file identity flow through the type system?** What does a span anchored to "the manifest" vs. "this `.kul` file" look like, and where does the file-context live (on AST nodes? on diagnostics? on the resolved view? all three?).
2. **What's the resolution semantic across files?** When two `.kul` files in the same project both declare `person alice`, is that an R01 duplicate-id error, a shadowing relation, two distinct `alice`s in different namespaces, or something else?

The two questions interact: the answer to (2) shapes whether `ResolvedDocument`'s id index is a flat `HashMap<id, FileSpan>` (one global namespace), nested per-file (per-file namespaces), or scoped some other way.

## Decision

### Three new types — `FileId`, `FileSpan`, `InputFile`

`kul_core::span::FileId` is an opaque `Copy` newtype indexing into a `Document.kul_files` slice (with a stable convention: `FileId(0)` is the manifest, `FileId(1..)` are the `.kul` files in input order). Construction is mostly internal — adapters reach for `FileId::MANIFEST` or read ids back out of existing `FileSpan`s — but `FileId::from_raw(u32)` exists as a back door for tests and adapter code that builds synthetic documents.

`kul_core::span::FileSpan { file: FileId, span: ByteSpan }` is the project-wide locator a diagnostic anchors on. AST nodes keep bare `ByteSpan` because their owning `KulFile` provides file context implicitly; cross-cutting consumers (diagnostics, the resolved id index, kinship queries) carry `FileSpan`.

`kul_core::ast::InputFile { name, source }` is the public input shape `kul_core::check` accepts. The `name` is opaque (a path-string from the CLI, a URI-string from the LSP, whatever the JS host chose for WASM); `kul-core` doesn't interpret it.

### Renamed types

- `ast::Document` → `ast::KulFile` (one parsed `.kul` file).
- `ast::Document` (new meaning) — multi-file container holding `Vec<Arc<KulFile>>` plus the manifest's name/source for diagnostic anchoring.
- `kul_lsp::state::Document` → `kul_lsp::state::OpenFile` (per-URI LSP cache entry; disambiguates from `ast::Document`).

### `Diagnostic.primary: Option<FileSpan>`

Most diagnostics anchor at a real source position; `KUL-M01` (manifest-not-found) cannot, because the file the toolchain wanted isn't there. `Option<FileSpan>` reflects that reality and keeps the renderer honest: an unanchored diagnostic surfaces with code + message but no source-block snippet.

`RelatedSpan.span: FileSpan` is mandatory — a related span without a position is just a note in `message`, so there's no useful "unanchored related" case.

`RenderableDiagnostic` takes a `&Document` (via `for_diagnostic`) so it can resolve any file's source by `FileId` for miette rendering — no caller has to thread around a `&str` source argument that only matches one file.

### `check` takes `(manifest_name, manifest_yaml, &[InputFile])`

`kul_core::check(manifest_name, manifest_yaml, &[InputFile])` runs the full pipeline; `kul_core::check_with_manifest(...)` is the variant for callers that already have a typed `Manifest` (the WASM bridge, in-memory tests). The previous `check(source, &Manifest)` from #69 is fully removed — no shim. Every consumer migrates in the same PR.

### Per-id queries on `ResolvedDocument` take `FileId`

`resolved.person(file, id)`, `resolved.marriage(file, id)`, `resolved.entity(file, id)`. Iteration queries (`persons()`, `marriages()`, `statements()`) walk every `.kul` file in source order; the `_in(file)` variants restrict to one file (the LSP uses these to enumerate symbols inside the active document, the cycle-detector to confine analysis to one file at a time).

`node_at` and `statement_at` both take a `FileId` because the same byte offset means different things in different files.

### Per-file id namespaces (Position B)

R01 (duplicate id) fires only within the same file. R02 (unresolved reference) fails for any reference to an id not declared in the *same* file. The same id may appear in two different `.kul` files without conflict; cross-file resolution is explicitly out of scope for v1.

Three positions were on the table:

- **Position A — global namespace.** Two files with `person alice` collide; R01 fires across files.
- **Position B — per-file namespaces (chosen).** Each `.kul` file is its own resolution scope; R01 is per-file.
- **Position C — per-file with explicit cross-file references.** Per-file by default; an explicit syntax (`from "other.kul" import alice`) makes a name visible elsewhere.

The chosen position is the smallest commitment the structural refactor has to make. Position A would shape `ResolvedDocument`'s id index as a flat map and force #63 to either accept it or undo it. Position C is a language feature, not a structural decision, and bakes a concrete syntax into the type system before #63 has a chance to design one. Position B keeps the namespace policy minimal and lets #63's PRD design cross-file resolution from a clean slate, with no inherited assumption about whether the project shares one namespace or many.

### Manifest diagnostics flow through the same pipeline

The manifest is treated as `FileId::MANIFEST` (= `FileId(0)`) inside the multi-file `Document`, with its raw YAML bytes stored alongside its name. A new `manifest::validate(yaml, file)` pass produces a typed `Manifest` plus diagnostics with normative `KUL-M02..M05` codes anchored at the manifest's `FileId`. The CLI prepends a `KUL-M01` (unanchored) diagnostic when the manifest is missing on disk; this is the only manifest code that has no anchor, since the file the toolchain wanted isn't there.

The `KUL-Mxx` codes:

- `KUL-M01` — manifest not found at expected path. Unanchored; the would-be path is in the message.
- `KUL-M02` — manifest YAML malformed. Anchors at the line/column the YAML parser reported.
- `KUL-M03` — manifest is well-formed YAML but missing the required `kul:` field. Anchors at the manifest start.
- `KUL-M04` — manifest's `kul:` value is not a recognized Kul language version. Anchors at the value.
- `KUL-M05` — manifest carries an unknown top-level field. Severity warning. Anchors at the field key.

The adapter-level string-rendering hack from #69 (CLI stderr strings; LSP synthetic byte-0 LSP `Diagnostic`; WASM `tsify` exception path for *content* errors) is removed. Manifest diagnostics now render through `RenderableDiagnostic` like any other; the WASM bridge surfaces them in the `CheckEnvelope.diagnostics` array; the LSP filters them out of the `.kul`-URI squiggle list (per the same per-file filter that handles all out-of-file diagnostics) but they still appear in the export envelope's failure-envelope diagnostics.

## Consequences

**Positive.** A multi-file project is now expressible at the type level, even though no consumer constructs N>1 yet. #63 can design cross-file resolution without re-shaping `ResolvedDocument`'s id index, the `Diagnostic` struct, or the `RenderableDiagnostic` rendering surface — the seam already exists.

Manifest errors are first-class diagnostics with line/column anchors into `kul.yml` (when the YAML was readable). The CLI renders them with miette like any other diagnostic; the WASM bridge surfaces them in the same `CheckEnvelope.diagnostics` array; the LSP no longer needs the synthetic byte-0 `Diagnostic` displacement. One pipeline, one rendering path, one diagnostic taxonomy.

The `LSP::state::Document` → `OpenFile` rename keeps the LSP's per-URI cache distinct from `ast::Document` (which now means the multi-file project container). The disambiguation is small but pays for itself the moment a maintainer reads `state.rs` and can tell at a glance which Document is which.

**Negative.** Every `Diagnostic::error` call site got rewritten to construct a `FileSpan` (or pass an `Option<FileSpan>` for unanchored cases). Many LSP feature functions gained a leading `file: FileId` parameter. Snapshot tests that serialize a `Diagnostic` gained a per-snapshot `Some(FileSpan { file: FileId(N), span: ByteSpan { ... } })` wrapper around what used to be a bare `ByteSpan`. The diff is mechanical but large.

The WASM bridge synthesizes a placeholder `kul.yml` body from the typed `Manifest` it receives so any `.kul`-side diagnostic that needs the manifest source has bytes to anchor at. This is a small ergonomic compromise — the JS host gives the bridge a typed manifest, not raw YAML, so we round-trip back to YAML to keep the renderer happy.

## Alternatives considered

**Position A — global cross-file namespace.** Rejected as a structural commitment. It would shape `ResolvedDocument`'s id index as a flat `HashMap<id, EntityRef>` and force #63 to either accept it or pull a global namespace back out. A structural refactor shouldn't decide a language feature on #63's behalf.

**Position C — explicit cross-file imports.** Rejected as out of scope. The syntax is a language design question for #63's PRD, not a type-system question for this issue. Once the PRD picks one, it lands additively on top of per-file namespaces — a `references_to` query that walks every file gets a `from_file` parameter; the existing per-file queries don't change.

**`Diagnostic.primary: FileSpan` (no `Option`).** Rejected because `KUL-M01` has no source position to anchor at. The minimum-surprise alternative — a synthetic span pointing to byte 0..0 of an empty manifest source — leaks "this is a placeholder" implementation detail into the renderer. `Option<FileSpan>` keeps the type honest; the renderer's "unanchored" case is one match arm.

**Path-keyed `FileId` (`enum FileId { Manifest, Source(PathBuf) }`).** Rejected because it bakes a notion of file identity into `kul-core` that adapters disagree on. The CLI knows on-disk paths; the LSP knows URIs; the WASM bridge knows whatever the JS host chose. An opaque `u32` lets each adapter decide what `name: String` means for them, and keeps `kul-core` honest about "I don't know what this string means; I just hand it back to you in diagnostics."

**Keep AST node spans as `FileSpan` (not `ByteSpan`).** Rejected because every AST-node walk would gain a redundant file-id check that the parent `KulFile` already pins. AST nodes live inside one file by construction; carrying the file-id on every span is overhead with no consumer.

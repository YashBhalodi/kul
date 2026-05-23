# Changelog

All notable changes to KulLang are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres to [Semantic Versioning](https://semver.org/).

The CLI (`kul`), language server (`kul-lsp`), and VSCode extension (`YashBhalodi.kul`) ship in lockstep — one tag, one set of artifacts. Per-component notes live under each version.

## [Unreleased]

### `kul-render`

- **New crate** owning the canonical UI pattern as data — projects the kinship-native [`ExportEnvelope`](./crates/kul-core/src/export.rs) into a `RenderShape` that realizes every canonical UI pattern principle (P1–P16). Two public entry points: `compute(&CheckResult) -> RenderShape` (runs `kul_core::export::export` with positions on, then projects) and `transform(&ExportEnvelope) -> RenderShape` (pure projection over an already-exported envelope, surfaced for fixture-driven tests). Output is hierarchical card slots (`Component → MarriageBranch → PersonCard → MarriageBranch …`) plus a flat edge list; generation indices, ghost emission (P8 past-marriage, P16 past-adoption), and P6 cross-component nesting are all baked in so a surface renderer (VSCode preview, web visualizer, …) reads the shape rather than re-deriving pattern decisions. Schema-versioned independently of the export envelope as `RENDER_SCHEMA_VERSION = 1` per [ADR-0010](./docs/adr/0010-export-schema-versioning.md)'s pattern. Crate-boundary rationale in [ADR-0016](./docs/adr/0016-kul-render-crate-boundary.md); shape and versioning in [ADR-0017](./docs/adr/0017-render-shape-schema-and-versioning.md) (#121).

### `examples/`

- Four new principle-complete examples extend the corpus to exercise canonical-UI-pattern cases the existing 01–07 don't surface: `08-divorce-and-remarriage/` (P8 child-anchoring ghosts after a clean divorce + both spouses remarrying with new children), `09-multi-adoption/` (P16 most-recent-adoption-canonical with child-ghosts at past adoptive families), `10-disconnected-lineages-and-orphan/` (P12 source-order arrangement of multiple components plus a P13 declared-with-no-edges orphan rendering as a single-card component between the lineages), and `11-cousin-marriage/` (P11 within-family absorb rule producing a within-family cross-edge). Example 03 gains a header-comment audit note calling out the P8 mechanic and the consequence that Bob becomes an orphan-component card (#121).

## [0.2.0] — 2026-05-18

The "multi-file projects" release. A Kul project is now formally a directory carrying a `kul.yml` manifest and one or more sibling `.kul` files; ids resolve project-wide across every file ([ADR-0015](./docs/adr/0015-global-project-namespace.md)); the CLI, LSP, and WASM surfaces all speak the project shape end-to-end. Three pre-1.0 breaking changes land together to crystallize the model — the **CLI subcommands drop their positional `<FILE>` and run CWD-rooted**, the **WASM `check` / `exportGraph` take an array of `{name, source}`**, and the **`kul X.Y` line moves from the `.kul` grammar into `kul.yml`**. Six new manifest validation codes (`KUL-M01..M06`) surface manifest-shape errors through the same diagnostic infrastructure as the thirteen `KUL-Rxx` rules. The Kul *language* version is unchanged at `0.1`.

The release also bundles two cross-cutting architecture deepening sweeps, a substantially trimmer test-fixture surface, and ships the [`kul-authoring`](./skills/kul-authoring/SKILL.md) agent skill so LLM agents can author `.kul` source from natural-language family narratives.

### Language

- **`kul X.Y` version declaration moves from the `.kul` grammar into a sibling `kul.yml` manifest** (breaking) — a `.kul` file no longer carries its target language version; tools resolve it from `kul.yml` in the file's directory. A `.kul` file without a sibling `kul.yml` is not a valid Kul project and tools report `KUL-M01` (manifest not found). The lexer no longer reserves `kul` — it lexes as a normal identifier. New spec §14 "Project manifest" is normative; ADR-0013 covers normative-vs-conventional and the directory-scoped discovery rule (#71). This is the precursor refactor that the multi-file work in #82–#86 builds on.
- **`kul-core::manifest` module** — new `Manifest` type plus a `parse(yaml) -> Result<Manifest, ParseError>` entry point gated behind a default-on `yaml` feature (kul-wasm opts out to keep `serde_yaml` out of the wasm blob). Manifest diagnostics flow through the same pipeline as `.kul`-side ones; six manifest validation codes report shape errors: `KUL-M01` not found (unanchored), `KUL-M02` malformed YAML (anchored at parser-reported line/col), `KUL-M03` missing required `kul:` field, `KUL-M04` unrecognized language version, `KUL-M05` unknown top-level field (warning), `KUL-M06` empty project — `kul.yml` present with zero sibling `.kul` files (#70, #71, #74, #82).
- **First-listed spouse is the marriage's host** (clarification, non-breaking) — spec §4.2 replaces the previous "spouse order carries no semantic significance" wording with the host definition: the first-listed spouse is the marriage's host (the structural role downstream consumers use for ordering and layout); the second joins the host's family. To change the host, swap the two spouse identifiers — no override field. CONTEXT.md gains a `Host (of a marriage)` entry; spec §8.5 adds a worked example; the LSP surfaces the role in hover ([see below](#kul-lsp)) (#111).

### `kul-core`

- **Project-wide id resolution** — `ResolvedDocument` flips its id index from per-file `HashMap<FileId, HashMap<...>>` to a flat `HashMap<String, ResolvedEntity>`. `person(id)`, `marriage(id)`, `entity(id)`, `spouses_of(marriage)`, and `parents_of(person)` lose their `FileId` parameter; per-file iteration helpers (`persons_in(file)`, `node_at(file, offset)`, …) keep theirs because byte offsets and per-URI LSP features remain file-scoped. R01 (duplicate id) now fires across files with primary on the second declaration (file-discovery order, then byte offset) and a related-span on the first; R02 (unresolved reference) wording flips to "in the project"; R13 (parenthood cycles) walks one project-wide parent graph and detects cycles spanning multiple files. The supersession of [ADR-0014](./docs/adr/0014-per-file-id-namespaces-in-v1.md)'s position B is recorded in [ADR-0015](./docs/adr/0015-global-project-namespace.md) (#82). `references_to` follows suit in a separate refactor — drops its `file` parameter and walks every `.kul` file in the project, returning `Vec<FileSpan>`; per-URI LSP consumers filter at the call site (#90).
- **File-identity types** — new `FileId`, `FileSpan`, `InputFile`, `KulFile` plus a multi-file `ast::Document` container holding `Vec<Arc<KulFile>>` and the manifest. AST nodes keep bare `ByteSpan` (their owning `KulFile` provides the file context implicitly); diagnostics and the resolved id index carry `FileSpan`. `Diagnostic.primary` becomes `Option<FileSpan>` so `KUL-M01` (which has no source position to anchor at) has a clean home. `kul_core::check`'s signature is `(manifest_name, manifest_yaml, &[InputFile]) -> CheckResult`; the WASM bridge enters via a new `check_with_manifest` (#70).
- **`kul_core::manifest::sibling_path(input)`** — hoists the spec §14.3 directory-scoped manifest discovery rule (`<dir>/<file>.kul` → `<dir>/kul.yml`, no walk-up) into kul-core as the single source of truth. The CLI and LSP previously each inlined it; now both call the core helper. Pure path manipulation, no filesystem IO (kul-core remains IO-free per [ADR-0014](./docs/adr/0014-per-file-id-namespaces-in-v1.md)) (#91).
- **Field-node cursor accessor** — `Node::field_node()` returns a typed `FieldNode { name, name_span, value_span, is_name }` for any of the six `*FieldName` / `*FieldValue` variants. Hover, completion, and document-symbol all consume it instead of carrying parallel per-variant match arms. The new vocabulary noun is in CONTEXT.md (#104).
- **Cursor seam carries target file across the project** — `Node::PersonRef.target` / `MarriageRef.target` are `Option<(FileId, &Stmt)>`; `EntityTarget::decl_span()` returns a project-wide `FileSpan`. Goto-definition, find-references, and rename no longer re-query `ResolvedDocument::entity(name)` solely to recover the target's `FileId`. New `ResolvedDocument::person_with_file` / `marriage_with_file` populate the seam (#104).
- **Display projections lifted onto their types** — `DateLit::format_canonical()`, `DateLit::format_year()`, `PersonStmt::display_name()`. Three LSP feature modules previously each carried their own copies that had silently drifted; the formatter, hover, document-symbol, and completion now all call the same methods (#75).
- **Lexer `is_identifier` / `is_reserved_word`** — promoted to `pub` so the LSP rename feature stops re-implementing them. `is_reserved_word` derives from `classify_word` so adding a new field-name keyword extends the reserved set automatically (#75).
- **`format/` module split** — the ~1400-line `format.rs` decomposes into `format/cells.rs` (Cell types, canonical column tables, AST → cell builders), `format/emit.rs` (Emitter, line emission, separator rules), and `format/source.rs` (`SourceFormatter` and comment scanning). Behavior unchanged (#75).
- **Canonical field order consolidated** — `field_meta::fields_for` is now the single source of truth for per-statement field order; the formatter's column sequence is composed from a structural prefix plus `fields_for(StatementKind)` plus a trailing comment column. The drift `PERSON_FIELDS` had against spec §15.2 is gone — adding or reordering a field is a one-row edit (#74).
- **`Document::new` / `Document::with_manifest_source` / `KulFile::new` constructors** — absorb the 30+ test/fixture sites that hand-spelled `Document { manifest_name: "kul.yml".to_string(), manifest_source: String::new(), kul_files: vec![...] }`. The `check` / `check_with_manifest` entry points share their post-manifest body via a private `run_pipeline`; the `KUL-M06` empty-project gate is split correctly between the two entries (#101).

### `kul-cli`

- **`validate`, `format`, `export` are CWD-rooted and accept no positional `<FILE>` argument** (breaking) — each subcommand discovers the project (a `kul.yml` plus every sibling `*.kul`) from the current working directory via the new `kul-loader` crate, runs `kul_core::check` once over the whole project, and renders project-wide output. `validate` reports diagnostics for every file in one pass; `format` rewrites every `.kul` in place (or, with `--check`, lists not-formatted files); `export` emits one envelope unioning every file's persons, marriages, and parenthood links. Running any of the three from a directory without a `kul.yml` prints `not a Kul project root: no kul.yml in current directory` and exits 1. Slice 2 of [PRD 0001](./docs/prd/0001-multi-file-kul-projects.md), per [ADR-0015](./docs/adr/0015-global-project-namespace.md) (#83).
- **Removed stdin (`-`) input and `--manifest` flag from `validate`, `format`, `export`** (breaking) — these subcommands now accept only on-disk file paths and discover the manifest as a sibling `kul.yml` of the input. Stdin had no real users and the `--manifest` flag existed solely to anchor manifest discovery in the stdin case; both fall outside the project-shaped model the multi-file work in #63/#64 will operate on. Removing both pre-1.0 keeps the project / workspace work in those issues clean (#72). The `kul lsp` subcommand's stdio transport is unaffected; the WASM bridge already takes the manifest as an explicit argument.
- **Shared diagnostic rendering across `validate`, `format`, `export`** — the miette + JSONL renderer that previously lived in `validate.rs` is now `commands::diag::{render_human, render_human_matching, render_json}`. `format` consumes `render_human_matching` with a blocking-parse-error predicate, so parse errors during `kul format` now surface the full miette report (caret anchor, source snippet, code annotation) instead of the previous flat `CODE: message` line. `validate.rs` shrinks from 165 lines to 45; `format.rs` drops its local parse-error printer entirely (#104).
- **Cross-file `see also` footnotes in CLI diagnostics** — for project-wide diagnostics (R01 duplicate-id, R02 type-mismatch) whose related-info entries live in a sibling file, the CLI renderer emits a `see also: <file>:<line>:<col> — <label>` footnote beneath the miette block. miette's `SourceCode` model is single-file and can't draw spans into two source blocks, so the footnote is the supported surfacing path. Same-file related-info renders unchanged (#104).
- **`commands::project::load_and_check`** — the load → check skeleton each subcommand previously open-coded is now one helper. Each subcommand body collapses to its renderer plus a single match (#101).

### `kul-lsp`

- **Project-keyed cache with broadcast diagnostics** — one cached `CheckResult` per project root, shared by every URI that lives in the project. `did_open` discovers the project from the URI's directory (sibling `kul.yml` plus every `.kul` file, read off disk); `did_change` mutates the URI's overlay and re-runs `check` for the project; `did_close` evicts the entry when the last open URI closes. Each `publishDiagnostics` broadcasts to every project file so the Problems pane reflects project-wide health (#85). Slice 5 of [PRD 0001](./docs/prd/0001-multi-file-kul-projects.md).
- **Cross-file definition, references, rename, completion** — each feature resolves the target's `FileId` through the project-wide resolver and maps it back to the right URL via the new `ProjectEntry`'s parallel `urls` / `line_indices` slices. Goto-definition from a `birth` reference in `02-parents.kul` now jumps into `01-founders.kul`; `find references` and `rename` aggregate hits across every project file; completion proposes ids declared in any project file (#85).
- **File watching via `workspace/didChangeWatchedFiles`** — registers OS-level watchers for `**/*.kul` and `**/kul.yml` via dynamic `client/registerCapability` from `initialized`. `.kul` `Created` / `Changed` / `Deleted` only fires when the parent directory is already a cached project (discovery stays lazy); `Changed` is ignored when the URI is currently overlaid so the editor buffer remains authoritative. `kul.yml` `Created` / `Changed` reloads the manifest; `Deleted` evicts the project entirely. Every event is logged at `tracing::debug!` with the action taken (`reloaded`, `ignored-because-overlaid`, `evicted`, `unknown-project`, …); the `register_capability` call is dispatched on a background task so clients that don't support dynamic registration cannot stall the `initialized` lifecycle (#86).
- **Marriage host role surfaced in hover** — the `marriage` keyword hover appends a one-line host note; the marriage-statement panel marks the host position as `- spouses: \`a\` (Name) (host) & \`b\` (Name)`; hover on a spouse-id token inside a `marriage` statement appends a role line ("Host of marriage `m`." / "Joining spouse in marriage `m`."). The third case resolves the enclosing marriage from the cursor via `ResolvedDocument::statement_at(file, byte_offset)` rather than extending `Node::PersonRef` — keeps the cursor-seam variant unchanged (#111).
- **Cross-file related-info in published diagnostics** — `features::diagnostics::to_lsp` now resolves related-info entries whose `FileSpan.file` differs from the primary's file through the project entry to produce a `DiagnosticRelatedInformation { location: { uri: sibling_url, ... } }`, using the sibling file's `LineIndex` for byte→UTF-16 translation. Same-file related-info is unchanged (#104).
- **`OpenFile::view()` / `OpenFile::cursor(position)` adapters** — six cursor-shaped request handlers (hover, goto_definition, prepare_rename, rename, references, completion) and two file-level handlers (document_symbol, semantic_tokens) previously each repeated a 3-line OpenFile destructure. Each handler now reads its inputs as a single typed value; the position → byte-offset conversion (UTF-16 ↔ UTF-8, CRLF) lives once on `Cursor`. The View / Cursor vocabulary is in CONTEXT.md (#101). The `#[cfg(test)] pub(crate) fn test_open_file` consolidation drops ~191 lines of per-feature test scaffolding (#101).
- **Inline code-action dispatch** — the per-request `HashMap<&'static str, ProviderFn>` registry collapses to a 2-arm match on `diag.code`. Adding a new fix is one extra arm; the registry shape can return once the rule of three justifies it (#104).

### `@kullang/wasm`

- **`check` and `exportGraph` now take `files: Array<{name, source}>` instead of `source: string`** (breaking) — the JS host enumerates the project's `.kul` files itself; the bridge no longer wraps a single source into a one-element `Vec<InputFile>` internally. `format(source: string)` is unchanged: formatting is per-file by nature (the underlying `kul_core::format::format_source` is single-source), whereas `check` and `exportGraph` are project-scoped so cross-file id resolution works through the WASM ABI. The internal `WASM_INPUT_NAME = "input.kul"` constant is gone — diagnostics anchored to a `.kul` source now carry the file name the JS host provided. Adds `WasmInputFile { name, source }` (`tsify from_wasm_abi`) exposed across the JS ABI; the committed `crates/kul-wasm/types/kul_wasm.d.ts` regenerates accordingly (the ADR-0012 `wasm-build` job catches drift). Slice of [PRD 0001](./docs/prd/0001-multi-file-kul-projects.md) (#84).
- **Manifest parameter on `check_with_manifest`** — the WASM bridge accepts a `tsify`-derived `Manifest` JS object alongside the source. The bridge no longer synthesizes a placeholder `kul: "X"\n` body — it passes empty bytes (#71, #101).
- **`ExportedDiagnostic.primary` is now optional** — unanchored `KUL-M01` (manifest not found) has no source position to anchor at, so consumers should optional-chain. The TS consumer compile-test exercises the pattern (#70).

### `kul-loader`

- **New crate** — shared filesystem entry point. Given a project-root path, returns `LoadedProject { manifest_name, manifest_yaml, inputs, root: PathBuf }` or a typed `ProjectLoadError` (`ManifestNotFound`, `ManifestReadFailed`, `DirectoryReadFailed`, `InputReadFailed`). Encapsulates `kul.yml` discovery, `*.kul` enumeration (flat directory per [ADR-0015](./docs/adr/0015-global-project-namespace.md) — subdirectories and non-`.kul` files silently ignored), and IO error variants. The CLI uses it for every subcommand; the LSP will follow in a later slice. Lives outside `kul-core` (which forbids filesystem IO) and outside `kul-cli` (which already depends on `kul-lsp` via `kul lsp`), so both `kul-cli` and a future `kul-lsp` can depend on it without a cycle (#83).

### Examples

- **Per-example project directories** — each `examples/NN-name.kul` moves to its own subdirectory carrying a sibling `kul.yml` (e.g. `examples/01-single-couple/single-couple.kul` + `examples/01-single-couple/kul.yml`). The shared `examples/kul.yml` is removed. Each example continues to validate as an independent one-file project; the restructure removes the latent cross-example id collisions (4 `alice`s, 3 `m_alice_bob`s) that would block project-wide id resolution. Closes #81 (#88).
- **`07-multi-file-extended-family`** — three `.kul` files (`01-founders` / `02-parents` / `03-grandchildren`) plus a shared `kul.yml` demonstrate cross-file `birth` references resolving by bare id under project-wide resolution. The numeric file prefixes give a stable reading order (alphabetic sort = generational sort) that keeps the export envelope's `persons` collection in chronological order (#82).

### Skills

- **`kul-authoring` — first agent skill** — `skills/kul-authoring/` is an [agentskills.io](https://agentskills.io)-compliant skill that teaches LLM agents to author idiomatic `.kul` source from natural-language family narratives. Install into a downstream project's `.agents/skills/kul-authoring/` via `npx skills add YashBhalodi/kul --skill kul-authoring`. Generate-only — validation, formatting, and export remain tooling concerns handled via the CLI / VSCode extension. The skill is self-contained (every external link is an absolute `https://github.com/YashBhalodi/kul/...` URL) so it works when installed away from its source repo (#65, #107, #108). The first-listed-spouse host vocabulary propagates into the skill alongside the spec change (#111).

### Tooling

- **`.claude/` agent tooling** — project-local Claude Code configuration so any AI agent dropped into this repo follows KulLang's existing quality bar by default. `.claude/settings.json` carries a permissions allowlist for the high-frequency safe commands (`just check`, `cargo nextest`, `cargo insta review`, `gh issue …`); PostToolUse hooks on `Edit | Write | MultiEdit` of `*.rs` run `cargo fmt -- <file>` (non-blocking) and `cargo clippy -p <crate> --all-targets -- -D warnings` (exits 2 on failure); a Stop hook blocks end-of-turn while any `*.snap.new` files exist, enforcing the deliberate `cargo insta review` step. Adds two subagents — `rust-implementer` (mandates orient → recipe → implement → test → verify → docs → commit grounded in AGENTS.md, CONTEXT.md, architecture.md, testing.md, and the ADR series) and `kul-pr-reviewer` (seven explicit pre-merge gates with grep recipes). One-page agent-facing definition-of-done at `docs/agents/rust-quality-checklist.md` cross-references the existing docs rather than duplicating them (#105).
- **`tests/common::check_one`** — the in-memory single-source convenience that three of the four kul-core integration-test files (validator.rs, export.rs, format.rs) previously inlined as a five-line wrap-and-call boilerplate (#92). Matching `project_dir(name)` helper in kul-cli integration tests collapses the temp-dir-plus-kul.yml prologue six call sites previously repeated (#101).
- **One-issue-one-PR discipline documented** — AGENTS.md gains the two-layer atomicity rule: each issue → one PR (atomic from a product perspective), each commit within a PR → one logical change that compiles (atomic from a codebase perspective). Squash-merge collapses the second layer into the first on `main` (#68).

## [0.1.3] — 2026-05-07

CI/infrastructure release. The release pipeline now publishes the VSCode extension to the **VS Code Marketplace** alongside Open VSX. No user-facing language, library, CLI, LSP, or behavior changes; the only difference at the consumer level is that upstream VSCode users can now install via `code --install-extension YashBhalodi.kul` instead of sideloading a `.vsix` from the GitHub Release.

### Pipeline

- **VS Code Marketplace publishing** — `release.yml`'s `extension-publish` matrix (renamed from `openvsx-publish`) now publishes each per-platform `.vsix` to both Open VSX and the Marketplace. Pre-flight checks for `OVSX_PAT` and `VSCE_PAT` run before either publish so a missing secret fails fast without leaving the registries out of sync (#62). The Marketplace setup walkthrough — Azure DevOps publisher, PAT scope (Marketplace > Manage, All accessible organizations), and partial-failure recovery — lives in [`docs/release.md`](./docs/release.md).

## [0.1.2] — 2026-05-07

CI/infrastructure release. No user-facing language, library, CLI, LSP, or extension behavior changes — every committed surface is byte-identical to `v0.1.1` aside from the lockstep version bump. Cut to keep the release pipeline exercised end-to-end against the upgraded GitHub Actions runtime baseline.

### Pipeline

- **All workflow actions on Node 24** — `actions/checkout@v4 → @v6`, `actions/setup-node@v4 → @v6`, `actions/upload-artifact@v4 → @v7`, `actions/download-artifact@v4 → @v8`, `softprops/action-gh-release@v2 → @v3` (#57). Closes the GitHub Actions Node 20 deprecation that takes effect 2026-06-02 (force-upgrade) and 2026-09-16 (Node 20 removal). `Swatinem/rust-cache@v2` was already on Node 24; `dtolnay/rust-toolchain@stable` and `taiki-e/install-action@nextest` are composite (no Node runtime).
- **Replaced `jetli/wasm-pack-action@v0.4.0` (last released 2022, pinned to Node 16) with `taiki-e/install-action@v2`** for the `wasm-build` and `wasm-publish` jobs (#61). Same prebuilt `wasm-pack@0.13.1` install path, just from a maintained, Node-24 action the repo already trusts (`taiki-e/install-action` is also used to install `nextest`). Closes #55.
- **VSCode extension dev dependencies bumped** (#56) — `@types/node 22 → 25`, `esbuild 0.24 → 0.28` (includes the GHSA-67mh-4wv8-2f99 dev-server CORS fix from 0.25; not exploitable here since the extension doesn't use esbuild's `serve` mode), `typescript 5.9 → 6.0`. The `vscode-extension.yml` lint job runs `vsce package`, which executes `tsc --noEmit && esbuild bundle` via `vscode:prepublish`, so the bumps were typecheck-and-bundle validated before merge.

## [0.1.1] — 2026-05-07

Hotfix for marketplace install. `v0.1.0` shipped a single un-targeted `.vsix`; Cursor's marketplace install path treats untagged extensions as platform-independent and strips bundled platform binaries on install, leaving the extension with no language server. Fixed by publishing four `--target`-tagged `.vsix` files (one per platform) and chmoding the bundled binary on activation as a belt-and-suspenders against vsce's zip layer dropping the execute bit (#59).

### `kul-vscode-extension`

- **Per-platform `.vsix` publishing** — `release.yml`'s `openvsx-publish` job now runs as a 4-target matrix (`darwin-arm64`, `darwin-x64`, `linux-x64`, `win32-x64`); each entry packages a `--target`-stamped `.vsix` containing only its platform's `kul-lsp` binary. Each `.vsix` is uploaded to Open VSX and attached to the GitHub Release as `kul-<version>-<target>.vsix`.
- **`chmodSync(serverPath, 0o755)` on activation** — the bundled-binary resolver chmods the LSP binary on Unix before the executable check, so a marketplace install with a stripped-permission binary (vsce's zip layer drops the execute bit) recovers transparently.

### `kul-core`, `kul-cli`, `kul-lsp`, `@kullang/wasm`

- **Lockstep version bump** — no functional changes. Bumped to keep all surfaces aligned with the VSCode extension hotfix per the [`release.yml` `verify` gate](./.github/workflows/release.yml).

## [0.1.0] — 2026-05-07

First public release. Everything below ships together at tag `v0.1.0`.

### Renamed

- **Language `kula` → `kul`; project `kulalang` → `KulLang`.** Pre-release rename, applied atomically across the repository. The version-declaration keyword is now `kul 0.1` (was `kula 0.1`); file extension is `.kul` (was `.kula`); CLI binary is `kul` (was `kula`); language-server binary is `kul-lsp` (was `kula-lsp`); npm package is `@kullang/wasm` (was `@kulalang/wasm`); validator codes are `KUL-Rxx` (was `KULA-Rxx`); export envelope field is `kul` (was `kula`); VSCode publisher.extensionId is `YashBhalodi.kul` with display name `KulLang`. Crate paths, GitHub repository URL, and every doc/example/test-corpus reference move with the rename. Motivated by pronunciation: `kul` orthographically forces /kuːl/ for English readers; `kula` was being parsed as KOO-lah.

### Language

- **Kul 0.1 specification** — fourteen normative sections plus a standalone EBNF grammar, covering document structure, lexical structure, top-level statements (`person`, `marriage`), person sub-statements (`birth`, `adoption`), semantics, validation rules, edge cases, file conventions, reserved keywords, formal grammar, versioning policy, and formatter rules. Lives at [`spec/`](./spec/README.md).
- **Versioning policy** — additive evolution: new fields and statements land without rewriting existing declarations.

### `kul-core` (library)

- **Parser + AST** for the full Kul 0.1 surface: version declaration, `person` and `marriage` top-level statements, `birth` and `adoption` person sub-statements, all field types (string, gender, end-reason, date), date literals at three granularities (full, year-month, year-only) with optional `~` (circa) prefix.
- **Semantic resolution** — ID indexing across persons and marriages, reference resolution for marriage spouses and `birth`/`adoption` marriage refs, parent-graph queries via the `ResolvedDocument` seam ([ADR-0001](./docs/adr/0001-resolved-document-as-query-seam.md), [ADR-0007](./docs/adr/0007-resolved-document-owns-document.md)).
- **Validator** implementing all 13 spec rules:
  - `KUL-R01` duplicate id; `KUL-R02` unresolved reference; `KUL-R03` required field missing; `KUL-R04` self-marriage; `KUL-R05` end-consistency (`KUL-R05b` for invalid `end_reason`).
  - Temporal: `KUL-R06` died-before-born; `KUL-R07` marriage-end-before-start; `KUL-R08` adoption-end-before-start; `KUL-R09` marriage-before-spouse-born; `KUL-R10` spouse-died-before-marriage; `KUL-R11` bio-child-born-before-parent; `KUL-R12` adoption-before-adopter-born.
  - Cycles: `KUL-R13` parenthood cycle (iterative DFS, O(V+E)).
- **Formatter** — opinionated, idempotent canonicalization ([ADR-0004](./docs/adr/0004-formatter-canonical-rules.md)).
- **Node-at-cursor query** (`ResolvedDocument::node_at`) — the shared foundation for hover, go-to-definition, completion, find-references, and rename.
- **Field metadata table** — single source of truth for per-field value shape, hover Markdown, and completion descriptions ([ADR-0005](./docs/adr/0005-field-metadata-table.md)).
- **Diagnostic detail tags** — sub-case discrimination on a single rule for code-action providers ([ADR-0006](./docs/adr/0006-diagnostic-detail-tag.md)).
- **Export module** (`kul_core::export`) — canonical JSON projection of a `CheckResult` into an `ExportEnvelope` (kinship-native graph or failure envelope). Strict on error-severity diagnostics; opt-in `with_positions` adds source spans on every entity. Cytoscape sub-module (`kul_core::export::cytoscape`) projects the same graph into the bipartite `nodes`/`edges` shape. Schema is normative — see [`spec/16-export-schema.md`](./spec/16-export-schema.md), [ADR-0008](./docs/adr/0008-export-kinship-native-shape.md), [ADR-0009](./docs/adr/0009-export-strict-on-diagnostics.md), [ADR-0010](./docs/adr/0010-export-schema-versioning.md).

### `kul-cli`

- **`kul validate`** — multiple files in one invocation, `-` reads from stdin, `--quiet` for exit-code-only, `--format json` (jsonl), `--no-color`. Exit `0` on success, `1` on any error.
- **`kul format`** — canonicalize a file in place; `--check` mode for CI gating (non-zero if not canonical).
- **`kul export`** — project a clean document to the canonical JSON envelope. `--format json` (default) emits the kinship-native shape; `--format cytoscape` emits the `nodes`/`edges` shape; `--with-positions` adds opt-in byte spans. Strict on errors; same stdin/-/multi-file ergonomics as `validate`.
- **`kul lsp`** — speak LSP over stdio (typically driven by an editor extension).

### `kul-wasm` / `@kullang/wasm`

- **WebAssembly bindings for `kul-core`** — published to npm as [`@kullang/wasm`](https://www.npmjs.com/package/@kullang/wasm) and to each GitHub Release as `kul-wasm.tar.gz`. Single ESM `--target bundler` build for modern bundlers (Vite, Webpack 5+, Next.js, Turbopack, SvelteKit, Nuxt, Astro).
- **Three exposed operations** ([ADR-0011](./docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md)):
  - `check(source) -> { diagnostics }` — empty array means clean (no `ok` field; emptiness is the discriminator).
  - `exportGraph(source, options?) -> SuccessEnvelope | FailureEnvelope` — bit-identical to `kul export --format=json`. Strict-on-errors per [ADR-0009](./docs/adr/0009-export-strict-on-diagnostics.md).
  - `format(source) -> string` — best-effort even on partial-parse input.
- **Version metadata getters** — `KUL_CORE_VERSION()`, `KUL_LANGUAGE_VERSION()`, `EXPORT_SCHEMA_VERSION()` for consumer compatibility checks without parsing an envelope.
- **TypeScript types derived from Rust** via [`tsify`](https://docs.rs/tsify), committed at `crates/kul-wasm/types/kul_wasm.d.ts` and CI-diffed against the regenerated output ([ADR-0012](./docs/adr/0012-tsify-derived-types-committed-and-diffed.md)). A type change that crosses the WASM boundary surfaces as a reviewable PR diff, not silent runtime drift.
- **Lockstep versioning** — `@kullang/wasm`'s npm version, the workspace `Cargo.toml` version, the VSCode extension version, and the git tag all match. Enforced by the `verify` job in [`release.yml`](./.github/workflows/release.yml).

### `kul-core` cleanups (surfaced by WASM packaging)

- **Workspace `miette` dependency narrowed** — the `fancy` feature is now enabled only in `kul-cli` (where the terminal-rendering machinery is actually used). `kul-core`, `kul-lsp`, and `kul-wasm` depend on plain `miette`, shrinking the WASM and LSP artifact sizes.
- **Optional `tsify` feature on `kul-core`** — default-off; enables `Tsify` derives on the export-envelope types so `kul-wasm` can emit accurate TypeScript types. The CLI and LSP never pull `tsify` or `wasm-bindgen` into their builds.
- **Export envelope JSON shape uses camelCase** — `parenthoodLinks`, `endReason`, `marriageId`, `childId`, `byteStart`, `byteEnd`, `withPositions`. JS-ecosystem convention; applied via `#[serde(rename_all = "camelCase")]` to the export structs. The CLI's `kul export --format=json` output and the WASM `exportGraph` output share one source of truth in `kul_core::export`. The Kul source language keeps its own snake_case identifiers — only the JSON projection changed. Normative in [`spec/16-export-schema.md`](./spec/16-export-schema.md).

### `kul-lsp`

- **Live diagnostics** — full Kul 0.1 validator, results pushed via `publishDiagnostics`.
- **Hover** — keyword, identifier, field-name, and reference hover with Markdown content.
- **Go to definition** — for person and marriage references.
- **Find references** — for person and marriage IDs.
- **Rename** — workspace edits across declaration and references; rejects collisions and reserved keywords.
- **Completion** — keyword, field-name, enum-value, and ID-aware completion (token-stream-first classifier per [ADR-0002](./docs/adr/0002-token-stream-first-completion-classifier.md)); auto-quoting for string fields.
- **Document symbols** — outline with persons, marriages, and nested sub-statements.
- **Code actions** — quick-fixes for `KUL-R03` missing-required-field and `KUL-R05` end-consistency.
- **Document formatting** — wraps `kul_core::format`.
- **Semantic tokens** — declaration / reference distinction for IDs, plus keyword / field / enum / date / string highlighting.
- **`kul/export` custom request** — project the in-memory buffer (including unsaved edits) through `kul_core::export`. Capability advertised under `experimental.kulExport`.

### Editor

- **VSCode extension** — TextMate grammar, file icon, snippets, language configuration, format-on-save, and full LSP integration with the bundled `kul-lsp` binary. No additional configuration required.
- **Export commands** — **Kul: Export to JSON** and **Kul: Export to Cytoscape JSON** in the command palette (visible only on `.kul` files). Routes through the LSP's `kul/export` request, prompts for a save location, surfaces a notification if the document has errors.

### Tooling and CI

- **`just check`** — single-command gate (fmt, clippy at deny, full nextest run).
- **`just wasm`** — builds `crates/kul-wasm` via `wasm-pack`, patches the npm package name, and refreshes the committed `.d.ts` snapshot.
- **Cross-platform release pipeline** producing CLI and language-server binaries for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, and `x86_64-pc-windows-msvc`, plus the marketplace `.vsix` with all four platform binaries bundled, plus the `@kullang/wasm` npm package and `kul-wasm.tar.gz` archive. See [`docs/release.md`](./docs/release.md).
- **Per-PR WASM gates** in [`.github/workflows/rust.yml`](./.github/workflows/rust.yml) — `wasm-pack` build, gzipped bundle-size budget (≤ 1 MB), generated `.d.ts` snapshot diff, Rust-side snapshot tests, Node smoke test, and TypeScript consumer compile-test (`tsc --noEmit`).

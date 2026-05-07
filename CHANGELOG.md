# Changelog

All notable changes to KulLang are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres to [Semantic Versioning](https://semver.org/).

The CLI (`kul`), language server (`kul-lsp`), and VSCode extension (`YashBhalodi.kul`) ship in lockstep — one tag, one set of artifacts. Per-component notes live under each version.

## [0.1.1] — Unreleased

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
- **Export module** (`kul_core::export`) — canonical JSON projection of a `CheckResult` into an `ExportEnvelope` (kinship-native graph or failure envelope). Strict on error-severity diagnostics; opt-in `with_positions` adds source spans on every entity. Cytoscape sub-module (`kul_core::export::cytoscape`) projects the same graph into the bipartite `nodes`/`edges` shape. Schema is normative — see [`spec/15-export-schema.md`](./spec/15-export-schema.md), [ADR-0008](./docs/adr/0008-export-kinship-native-shape.md), [ADR-0009](./docs/adr/0009-export-strict-on-diagnostics.md), [ADR-0010](./docs/adr/0010-export-schema-versioning.md).

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
- **Export envelope JSON shape uses camelCase** — `parenthoodLinks`, `endReason`, `marriageId`, `childId`, `byteStart`, `byteEnd`, `withPositions`. JS-ecosystem convention; applied via `#[serde(rename_all = "camelCase")]` to the export structs. The CLI's `kul export --format=json` output and the WASM `exportGraph` output share one source of truth in `kul_core::export`. The Kul source language keeps its own snake_case identifiers — only the JSON projection changed. Normative in [`spec/15-export-schema.md`](./spec/15-export-schema.md).

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

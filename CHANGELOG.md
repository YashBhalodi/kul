# Changelog

All notable changes to KulaLang are documented here. The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project adheres to [Semantic Versioning](https://semver.org/).

The CLI (`kula`), language server (`kula-lsp`), and VSCode extension (`YashBhalodi.kulalang`) ship in lockstep — one tag, one set of artifacts. Per-component notes live under each version.

## [0.1.0] — Unreleased

First public release. Everything below ships together at tag `v0.1.0`.

### Language

- **Kula 0.1 specification** — fourteen normative sections plus a standalone EBNF grammar, covering document structure, lexical structure, top-level statements (`person`, `marriage`), person sub-statements (`birth`, `adoption`), semantics, validation rules, edge cases, file conventions, reserved keywords, formal grammar, versioning policy, and formatter rules. Lives at [`spec/`](./spec/README.md).
- **Versioning policy** — additive evolution: new fields and statements land without rewriting existing declarations.

### `kula-core` (library)

- **Parser + AST** for the full Kula 0.1 surface: version declaration, `person` and `marriage` top-level statements, `birth` and `adoption` person sub-statements, all field types (string, gender, end-reason, date), date literals at three granularities (full, year-month, year-only) with optional `~` (circa) prefix.
- **Semantic resolution** — ID indexing across persons and marriages, reference resolution for marriage spouses and `birth`/`adoption` marriage refs, parent-graph queries via the `ResolvedDocument` seam ([ADR-0001](./docs/adr/0001-resolved-document-as-query-seam.md), [ADR-0007](./docs/adr/0007-resolved-document-owns-document.md)).
- **Validator** implementing all 13 spec rules:
  - `KULA-R01` duplicate id; `KULA-R02` unresolved reference; `KULA-R03` required field missing; `KULA-R04` self-marriage; `KULA-R05` end-consistency (`KULA-R05b` for invalid `end_reason`).
  - Temporal: `KULA-R06` died-before-born; `KULA-R07` marriage-end-before-start; `KULA-R08` adoption-end-before-start; `KULA-R09` marriage-before-spouse-born; `KULA-R10` spouse-died-before-marriage; `KULA-R11` bio-child-born-before-parent; `KULA-R12` adoption-before-adopter-born.
  - Cycles: `KULA-R13` parenthood cycle (iterative DFS, O(V+E)).
- **Formatter** — opinionated, idempotent canonicalization ([ADR-0004](./docs/adr/0004-formatter-canonical-rules.md)).
- **Node-at-cursor query** (`ResolvedDocument::node_at`) — the shared foundation for hover, go-to-definition, completion, find-references, and rename.
- **Field metadata table** — single source of truth for per-field value shape, hover Markdown, and completion descriptions ([ADR-0005](./docs/adr/0005-field-metadata-table.md)).
- **Diagnostic detail tags** — sub-case discrimination on a single rule for code-action providers ([ADR-0006](./docs/adr/0006-diagnostic-detail-tag.md)).
- **Export module** (`kula_core::export`) — canonical JSON projection of a `CheckResult` into an `ExportEnvelope` (kinship-native graph or failure envelope). Strict on error-severity diagnostics; opt-in `with_positions` adds source spans on every entity. Cytoscape sub-module (`kula_core::export::cytoscape`) projects the same graph into the bipartite `nodes`/`edges` shape. Schema is normative — see [`spec/15-export-schema.md`](./spec/15-export-schema.md), [ADR-0008](./docs/adr/0008-export-kinship-native-shape.md), [ADR-0009](./docs/adr/0009-export-strict-on-diagnostics.md), [ADR-0010](./docs/adr/0010-export-schema-versioning.md).

### `kula-cli`

- **`kula validate`** — multiple files in one invocation, `-` reads from stdin, `--quiet` for exit-code-only, `--format json` (jsonl), `--no-color`. Exit `0` on success, `1` on any error.
- **`kula format`** — canonicalize a file in place; `--check` mode for CI gating (non-zero if not canonical).
- **`kula export`** — project a clean document to the canonical JSON envelope. `--format json` (default) emits the kinship-native shape; `--format cytoscape` emits the `nodes`/`edges` shape; `--with-positions` adds opt-in byte spans. Strict on errors; same stdin/-/multi-file ergonomics as `validate`.
- **`kula lsp`** — speak LSP over stdio (typically driven by an editor extension).

### `kula-lsp`

- **Live diagnostics** — full Kula 0.1 validator, results pushed via `publishDiagnostics`.
- **Hover** — keyword, identifier, field-name, and reference hover with Markdown content.
- **Go to definition** — for person and marriage references.
- **Find references** — for person and marriage IDs.
- **Rename** — workspace edits across declaration and references; rejects collisions and reserved keywords.
- **Completion** — keyword, field-name, enum-value, and ID-aware completion (token-stream-first classifier per [ADR-0002](./docs/adr/0002-token-stream-first-completion-classifier.md)); auto-quoting for string fields.
- **Document symbols** — outline with persons, marriages, and nested sub-statements.
- **Code actions** — quick-fixes for `KULA-R03` missing-required-field and `KULA-R05` end-consistency.
- **Document formatting** — wraps `kula_core::format`.
- **Semantic tokens** — declaration / reference distinction for IDs, plus keyword / field / enum / date / string highlighting.
- **`kula/export` custom request** — project the in-memory buffer (including unsaved edits) through `kula_core::export`. Capability advertised under `experimental.kulaExport`.

### Editor

- **VSCode extension** — TextMate grammar, file icon, snippets, language configuration, format-on-save, and full LSP integration with the bundled `kula-lsp` binary. No additional configuration required.
- **Export commands** — **Kula: Export to JSON** and **Kula: Export to Cytoscape JSON** in the command palette (visible only on `.kula` files). Routes through the LSP's `kula/export` request, prompts for a save location, surfaces a notification if the document has errors.

### Tooling and CI

- **`just check`** — single-command gate (fmt, clippy at deny, full nextest run).
- **Cross-platform release pipeline** producing CLI and language-server binaries for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, and `x86_64-pc-windows-msvc`, plus the marketplace `.vsix` with all four platform binaries bundled. See [`docs/release.md`](./docs/release.md).

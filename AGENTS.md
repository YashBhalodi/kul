# AGENTS.md

Conventions, layout, and workflow for anyone (human or AI) working in this repository. Read this on entry.

## Repository layout

```
crates/
  kula-core/   — library: lexer, parser, AST, semantic, validator, diagnostics, node-at-cursor query, formatter
  kula-cli/    — binary `kula`: `kula validate`, `kula format`, `kula lsp` subcommands
  kula-lsp/    — library + binary `kula-lsp`: LSP adapter over kula-core
docs/
  vision.md    — language scope and design intent
  architecture.md — implementation map: pipeline, seams, "where to add X" recipes
  testing.md   — test conventions: snapshots, corpus, perf budgets
  release.md   — operational handbook for cutting a release
  adr/         — Architectural Decision Records
  prd/         — Product Requirements Documents (transient — deleted after the epic ships; see [`prd/README.md`](./docs/prd/README.md))
  agents/      — agent-tooling docs (issue tracker, triage labels, domain-docs convention)
spec/          — Kula 0.1 language specification (the normative source of truth)
editor/vscode/ — VSCode extension (LSP-backed, marketplace-publishable)
examples/      — `.kula` corpus used as both docs and the positive test corpus
CONTEXT.md     — domain glossary; canonical vocabulary for the project
```

## Where to look first

| You need to…                                | Read                                   |
| ------------------------------------------- | -------------------------------------- |
| Understand the language                     | [`spec/`](./spec/README.md)            |
| Understand the codebase shape               | [`docs/architecture.md`](./docs/architecture.md) |
| Understand the domain vocabulary            | [`CONTEXT.md`](./CONTEXT.md)           |
| Understand a major design decision          | [`docs/adr/`](./docs/adr/)             |
| Understand product scope of an epic         | [`docs/prd/`](./docs/prd/)             |
| Add a test or perf budget                   | [`docs/testing.md`](./docs/testing.md) |
| Cut a release                               | [`docs/release.md`](./docs/release.md) |
| Triage / file an issue                      | [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md), [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md) |

## Rust development

### Prerequisites

- Rust toolchain (stable, edition 2024). Install via [`rustup`](https://rustup.rs/).
- [`just`](https://just.systems/) — task runner. `cargo install just --locked` or `brew install just`.
- [`cargo-nextest`](https://nexte.st/) — test runner. `cargo install cargo-nextest --locked`.

### One command for green

```sh
just check
```

Runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo nextest run --workspace`. Local-green should imply CI-green; the same commands run in `.github/workflows/rust.yml`.

Other recipes:

- `just test` — tests only.
- `just fmt` — auto-format.
- `just lint` — clippy alone.
- `just run -- validate examples/03-three-generations.kula` — passthrough to `cargo run -p kula-cli --`.

### Definition of done (Rust changes)

A change is done when:

1. `just check` is green.
2. New behavior is covered by tests in the appropriate crate. Snapshot tests (via `insta`) are the default for parser / validator / LSP-feature output — see [`docs/testing.md`](./docs/testing.md) and [ADR-0003](./docs/adr/0003-snapshot-tests-as-primary-validation.md).
3. Public items have rustdoc; clippy lints are at deny level (no `#[allow]` without a justifying comment).
4. If a non-obvious design choice is being made, it lands as an ADR in [`docs/adr/`](./docs/adr/) — not as a code comment that a future agent might "simplify" away.

## Workflow

The standard loop, end to end. Each step links to the doc that owns the detail.

### 1. Pick or file an issue

All work starts as a [GitHub issue](https://github.com/YashBhalodi/kulalang/issues). New ideas land with `needs-triage`; once specified they get `ready-for-agent` (AFK-suitable) or `ready-for-human`. Conventions: [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md), label mapping: [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md).

### 2. Branch and commit

- Work on a topic branch off `main`. No long-lived branches.
- Commits are **imperative and descriptive**, lead with a verb (e.g. `Cache ResolvedDocument, share source via Arc`). No conventional-commits prefixes (`feat:`, `fix:`). One logical change per commit; squash WIPs before opening a PR.
- Every commit must keep `just check` green.

### 3. Make the change

- Use the recipes in [`docs/architecture.md` § "Where to add X"](./docs/architecture.md) — they cover validator rules, LSP features, AST variants, fields, sub-cases, and CLI subcommands.
- Query through the seams (`ResolvedDocument`, `node_at`, `entity_reference`, `field_meta`). Don't iterate `document.statements` from a feature module — that's [ADR-0001](./docs/adr/0001-resolved-document-as-query-seam.md) territory.
- Use the vocabulary in [`CONTEXT.md`](./CONTEXT.md). If the concept isn't there, either rename or extend the glossary in the same change.
- AST and field changes are **additive only** — never reorder, rename, or remove existing variants. New fields are optional unless the spec marks them required.

### 4. Cover with tests

- Snapshot test by default for anything that emits structured output (diagnostics, AST, completion lists, hover Markdown, formatted source, semantic-token streams). [`docs/testing.md`](./docs/testing.md) has the full layout, snapshot workflow (`cargo insta review`), and the test-addition checklist.
- Validator rules: positive case + negative case + diagnostic snapshot, named `rule_NN_<short_name>`.
- Examples under `examples/*.kula` are the positive test corpus — adding one pulls it into the test suite automatically. Don't put failing fixtures in `examples/`; inline `&str` literals next to the test.
- Perf-sensitive paths get a `#[test]` in [`crates/kula-lsp/tests/perf.rs`](./crates/kula-lsp/tests/perf.rs) with a 5× ceiling and the real target documented in a comment.

### 5. Add an ADR if a non-obvious choice is being made

ADRs in [`docs/adr/`](./docs/adr/) capture *why* a load-bearing decision was made. Add one when:

- Choosing between two viable designs and the choice will influence later decisions.
- Settling a rule that downstream code or specs will depend on (e.g. formatter canonicality, ADR-0004).
- Closing off a tempting alternative that future agents might re-propose ("anti-suggestion").

Do **not** add an ADR for routine refactors, bug fixes, or restating something that's already clear in the code.

ADR file naming: `NNNN-kebab-short-title.md` with the next free `NNNN`. Frontmatter: status (`Accepted` / `Superseded by ADR-XXXX`), date (ISO), deciders. Sections: Context → Decision → Consequences → Anti-suggestions.

### 6. Open the PR

- Title: short imperative summary.
- Body: link the issue (`Fixes #N`), describe the user-visible change, call out anything reviewers should look at twice. PR descriptions are not commit messages — explain the *why*.
- CI runs the same `just check` plus the extension lint workflow. Both must be green before merge.

### 7. Cut a release (when appropriate)

The CLI, language server, and VSCode extension release in lockstep — one tag, one pipeline. Procedure: [`docs/release.md`](./docs/release.md). The `verify` job blocks tagging if `Cargo.toml`, `editor/vscode/package.json`, and the git tag drift apart.

## Domain vocabulary

This repo is **single-context**: one [`CONTEXT.md`](./CONTEXT.md) at the repo root plus [`docs/adr/`](./docs/adr/) cover the entire project. When naming things — in issue titles, hypothesis statements, tests, PR descriptions — use the terms `CONTEXT.md` defines. Don't drift into "service / handler / component" speak; the architecture vocabulary (module / interface / seam / depth) is in [`docs/architecture.md`](./docs/architecture.md).

If a concept isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (extend the glossary in the same change).

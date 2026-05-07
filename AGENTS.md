# AGENTS.md

Conventions, layout, and workflow for anyone (human or AI) working in this repository. Read this on entry.

## Repository layout

```
crates/
  kul-core/   — library: lexer, parser, AST, semantic, validator, diagnostics, node-at-cursor query, formatter, export
  kul-cli/    — binary `kul`: `kul validate`, `kul format`, `kul export`, `kul lsp` subcommands
  kul-lsp/    — library + binary `kul-lsp`: LSP adapter over kul-core (handles standard capabilities plus the `kul/export` custom request)
  kul-wasm/   — library (cdylib): WASM adapter over kul-core, published as `@kullang/wasm` (npm) and `kul-wasm.tar.gz` (GitHub Release). Surface is `check`, `exportGraph`, `format` (per ADR-0011).
docs/
  vision.md    — language scope and design intent
  architecture.md — implementation map: pipeline, seams, "where to add X" recipes
  testing.md   — test conventions: snapshots, corpus, perf budgets
  release.md   — operational handbook for cutting a release
  adr/         — Architectural Decision Records
  prd/         — Product Requirements Documents (transient — deleted after the epic ships; see [`prd/README.md`](./docs/prd/README.md))
  agents/      — agent-tooling docs (issue tracker, triage labels, domain-docs convention)
spec/          — Kul 0.1 language specification (the normative source of truth)
editor/vscode/ — VSCode extension (LSP-backed, marketplace-publishable)
examples/      — `.kul` corpus used as both docs and the positive test corpus
CONTEXT.md     — domain glossary; canonical vocabulary for the project
```

## Where to look first

| You need to…                        | Read                                                                                                                               |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Understand the language             | [`spec/`](./spec/README.md)                                                                                                        |
| Understand the codebase shape       | [`docs/architecture.md`](./docs/architecture.md)                                                                                   |
| Understand the domain vocabulary    | [`CONTEXT.md`](./CONTEXT.md)                                                                                                       |
| Understand a major design decision  | [`docs/adr/`](./docs/adr/)                                                                                                         |
| Understand product scope of an epic | [`docs/prd/`](./docs/prd/)                                                                                                         |
| Add a test or perf budget           | [`docs/testing.md`](./docs/testing.md)                                                                                             |
| Cut a release                       | [`docs/release.md`](./docs/release.md)                                                                                             |
| Triage / file an issue              | [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md), [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md) |

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
- `just run -- validate examples/03-three-generations.kul` — passthrough to `cargo run -p kul-cli --`.
- `just vscode` (or `just vscode release`) — build LSP, package the VSCode extension, and install the `.vsix`. Re-run after each code change; reload the VSCode window to pick it up. See [`editor/vscode/README.md`](./editor/vscode/README.md#install-for-development).

### Definition of done (Rust changes)

A change is done when:

1. `just check` is green.
2. New behavior is covered by tests in the appropriate crate. Snapshot tests (via `insta`) are the default for parser / validator / LSP-feature output — see [`docs/testing.md`](./docs/testing.md) and [ADR-0003](./docs/adr/0003-snapshot-tests-as-primary-validation.md).
3. Public items have rustdoc; clippy lints are at deny level (no `#[allow]` without a justifying comment).
4. If a non-obvious design choice is being made, it lands as an ADR in [`docs/adr/`](./docs/adr/) — not as a code comment that a future agent might "simplify" away.

## Domain vocabulary

This repo is **single-context**: one [`CONTEXT.md`](./CONTEXT.md) at the repo root plus [`docs/adr/`](./docs/adr/) cover the entire project. When naming things — in issue titles, hypothesis statements, tests, PR descriptions — use the terms `CONTEXT.md` defines. Don't drift into "service / handler / component" speak; the architecture vocabulary (module / interface / seam / depth) is in [`docs/architecture.md`](./docs/architecture.md).

If a concept isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (extend the glossary in the same change).

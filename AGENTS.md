# AGENTS.md

Conventions and configuration for AI agents working in this repository. Read this on entry.

## Repository layout

```
crates/
  kula-core/   — library: lexer, parser, AST, semantic, validator, diagnostics
  kula-cli/    — binary `kula`: thin CLI wrapper around kula-core
docs/          — vision, roadmap PRDs, agent docs
spec/          — Kula 0.1 language specification (the normative source of truth)
editor/        — VSCode extension (Phase 1)
examples/      — `.kula` corpus used as both docs and the positive test corpus
```

## Rust development

The Rust workspace at the repo root is the home for Phases 2–4.

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
2. New behavior is covered by tests in the appropriate crate (snapshot tests via `insta` for lexer/parser/validator output, golden corpus for validation rules).
3. Public items have rustdoc; clippy lints are at deny level (no `#[allow]` without a justifying comment).

## Agent skills

### Issue tracker

GitHub Issues at `YashBhalodi/kulalang` via the `gh` CLI. See [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md).

### Triage labels

The five canonical triage roles use their default label strings (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md).

### Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root (neither file exists yet — the `grill-with-docs` and `improve-codebase-architecture` skills will populate them lazily as the implementation begins). See [`docs/agents/domain.md`](./docs/agents/domain.md).

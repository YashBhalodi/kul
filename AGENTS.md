# AGENTS.md

Conventions, layout, and workflow for anyone (human or AI) working in this repository. Read this on entry.

## Repository layout

```
crates/
  kul-core/   — library: lexer, parser, AST, semantic, validator, diagnostics, node-at-cursor query, formatter, export
  kul-loader/ — library: project filesystem loader shared by `kul-cli` and `kul-lsp` (no `kul-core` dependency on disk IO)
  kul-render/ — library: the canonical UI pattern as data — projects `ExportEnvelope` into `RenderShape` for surface renderers to consume (ADR-0016, ADR-0017)
  kul-layout/ — library: positioning pass — turns `RenderShape` into a `PositionedShape` (cards and edges in absolute pixel coordinates; marriages render as thick marriage edges) via a Walker's-algorithm port plus a canonical-pattern adapter (ADR-0018)
  kul-svg/    — library: theme-agnostic SVG emitter over `PositionedShape` (semantic CSS classes; no inline colours) (ADR-0016)
  kul-visual/ — library: thin composition facade over the pinned pipeline crates — `render_from_check` owns the `compute → layout → render` success sequence so every SVG-producing surface routes through one entrypoint (ADR-0031)
  kul-cli/    — binary `kul`: `kul validate`, `kul format`, `kul export`, `kul lsp` subcommands
  kul-lsp/    — library + binary `kul-lsp`: LSP adapter over kul-core (handles standard capabilities plus the `kul/export` and `kul/render` custom requests)
  kul-wasm/   — library (cdylib): WASM adapter over kul-core, published as `@kullang/wasm` (npm) and `kul-wasm.tar.gz` (GitHub Release). Surface is `check`, `exportGraph`, `format` (per ADR-0011).
docs/
  vision.md    — language scope and design intent
  architecture.md — implementation map: pipeline, seams, "where to add X" recipes
  testing.md   — test conventions: snapshots, corpus, perf budgets
  adr/         — Architectural Decision Records
  prd/         — Product Requirements Documents (transient — deleted after the epic ships; see [`prd/README.md`](./docs/prd/README.md))
  agents/      — agent-tooling docs (issue tracker, triage labels, domain-docs convention)
spec/          — Kul 0.1 language specification (the normative source of truth)
editor/vscode/ — VSCode extension (LSP-backed, published to both the VS Code Marketplace and Open VSX). Thin host over `@kullang/preview` for the preview chrome.
packages/
  preview/     — `@kullang/preview` npm workspace package: webview chrome (HTML shell, bootstrap, tooltip, legend, pan/zoom controls, error popover, ghost-badge injection, selection-sync highlighting, `--kul-*` theme tokens). Consumed by the VSCode extension via a `HostAdapter` (ADR-0016 amendment 2026-06-09)
examples/      — `.kul` corpus used as both docs and the positive test corpus
skills/        — agentskills.io-compliant skills delivered separately via `npx skills add` (see `skills/kul-authoring/`)
CONTEXT.md     — domain glossary; canonical vocabulary for the project
```

## Where to look first

| You need to…                        | Read                                                                                                                               |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Understand the language             | [`spec/`](./spec/README.md)                                                                                                        |
| Understand the codebase shape       | [`docs/architecture.md`](./docs/architecture.md)                                                                                   |
| Understand the domain vocabulary    | [`CONTEXT.md`](./CONTEXT.md)                                                                                                       |
| Understand cross-cutting coding principles | [`CODING_STANDARDS.md`](./CODING_STANDARDS.md)                                                                              |
| Understand how Kul renders visually | [`docs/canonical-ui-pattern.md`](./docs/canonical-ui-pattern.md)                                                                   |
| Understand a major design decision  | [`docs/adr/`](./docs/adr/)                                                                                                         |
| Understand product scope of an epic | [`docs/prd/`](./docs/prd/)                                                                                                         |
| Add a test or perf budget           | [`docs/testing.md`](./docs/testing.md)                                                                                             |
| Cut a release                       | the `/release` command ([`.claude/commands/release.md`](./.claude/commands/release.md)) — sole source of truth for the release runbook |
| Triage / file an issue              | [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md), [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md) |
| Help an AI agent author Kul         | [`skills/kul-authoring/`](./skills/kul-authoring/SKILL.md)                                                                          |

## Rust development

### Prerequisites

- Rust toolchain (stable, edition 2024). Install via [`rustup`](https://rustup.rs/).
- [`just`](https://just.systems/) — task runner. `cargo install just --locked` or `brew install just`.
- [`cargo-nextest`](https://nexte.st/) — test runner. `cargo install cargo-nextest --locked`.
- [Node 22](https://nodejs.org/) with `npm ci` run once at the repo root — `just check`'s TypeScript gate and the VSCode extension build need it. The pinned version lives in `.nvmrc`.
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) — only for `just wasm`. `cargo install wasm-pack --locked` or the installer script.

### One command for green

```sh
just check
```

Runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, and then the TypeScript workspace tests (`npm test --workspaces --if-present`, i.e. Vitest in `packages/preview` and `editor/vscode`). Local-green should imply CI-green; the Rust gates run in `.github/workflows/rust.yml` and the same TypeScript suites run in `.github/workflows/vscode-extension.yml`.

Other recipes:

- `just test` — tests only.
- `just fmt` — auto-format.
- `just lint` — clippy alone.
- `just run -- --help` — passthrough to `cargo run -p kul-cli --`. The CLI's `validate` / `format` / `export` subcommands are CWD-rooted (they discover the project from the current directory), so to drive one against an example use the cargo binary directly: `cargo run -p kul-cli -- validate` from inside `examples/02-three-generations/`.
- `just vscode` (or `just vscode release`) — build LSP, package the VSCode extension, and install the `.vsix`. Re-run after each code change; reload the VSCode window to pick it up. See [`editor/vscode/DEVELOPING.md`](./editor/vscode/DEVELOPING.md).

### Definition of done (Rust changes)

A change is done when:

1. `just check` is green.
2. New behavior is covered by tests in the appropriate crate. Snapshot tests (via `insta`) are the default for parser / validator / LSP-feature output — see [`docs/testing.md`](./docs/testing.md) for layout/mechanics, [`CODING_STANDARDS.md`](./CODING_STANDARDS.md) for the behavior-not-implementation principle, and [ADR-0003](./docs/adr/0003-snapshot-tests-as-primary-validation.md).
3. Public items have rustdoc; clippy lints are at deny level (no `#[allow]` without a justifying comment).
4. If a non-obvious design choice is being made, it lands as an ADR in [`docs/adr/`](./docs/adr/) — not as a code comment that a future agent might "simplify" away.
5. If your change touches `spec/`, the kinship section of `CONTEXT.md`, or `examples/`, update [`skills/kul-authoring/`](./skills/kul-authoring/SKILL.md) accordingly in the same PR.

## Issues, PRs, and commits

The boundary of one PR is defined by one issue. Each issue represents one atomic unit of work *from a project / product perspective* — one refactor, one feature, one bug fix. PR diff size is not a constraint; what matters is the work-shape.

- **One issue = one PR.** Don't split an issue's work across multiple PRs to `main`. If the work won't fit in one PR, the issue is too big — split the *issue*, not the PR.
- **Squash and merge is the policy.** All PRs squash-merge into `main`; the resulting single commit represents the whole issue's worth of work.
- **Within a PR, commits stay atomic from a *codebase* perspective.** Use [Conventional Commits](https://www.conventionalcommits.org/). Each commit is one logical change that compiles; the PR groups them into one product-perspective unit, and the squash collapses the group on merge.
- **Docs, ADRs, and tests land in the same PR as the code that motivates them.** A PR that ships an ADR but not the code it documents (or vice versa) is not atomic at the product perspective — it splits one piece of work across two `main` commits.

Two layers of atomicity, then: the PR / issue is atomic from a product perspective; commits inside it are atomic from a codebase perspective. The squash-merge policy collapses the second layer into the first on `main`.

## Domain vocabulary

This repo is **single-context**: one [`CONTEXT.md`](./CONTEXT.md) at the repo root plus [`docs/adr/`](./docs/adr/) cover the entire project. When naming things — in issue titles, hypothesis statements, tests, PR descriptions — use the terms `CONTEXT.md` defines. Don't drift into "service / handler / component" speak; the architecture vocabulary (module / interface / seam / depth) is in [`docs/architecture.md`](./docs/architecture.md).

If a concept isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (extend the glossary in the same change).

# AGENTS.md

Conventions, layout, and workflow for anyone (human or AI) working in this repository. Read this on entry.

## Repository layout

```
crates/
  kul-core/             — library: lexer, parser, AST, semantic resolution, validator, diagnostics, node-at-cursor query, formatter, export, the kinship-query engine
  kul-render/           — library: the canonical UI pattern as data — projects `ExportEnvelope` into `RenderShape` (ADR-0016, ADR-0017)
  kul-layout/           — library: positioning pass — turns `RenderShape` into a `PositionedShape` (cards and edges in absolute pixel coordinates) via a Walker's-algorithm port plus a canonical-pattern adapter (ADR-0018)
  kul-svg/              — library: theme-agnostic SVG emitter over `PositionedShape` (semantic CSS classes, `data-*` attributes, no inline colours) (ADR-0016)
  kul-visual/           — library: composition facade — `render_from_check` owns the `compute → layout → render` sequence so every SVG-producing caller routes through one entrypoint (ADR-0031)
  kul-language-service/ — library: the editor features (diagnostics, hover, completion, rename, tokens, …) as pure functions over a checked project, with Kul-native results (ADR-0049)
  kul-wasm/             — library (cdylib): the browser's engine, one stateful `Workspace` over the language service and the renderer, built with `wasm-pack --target web` into `pkg/` (ADR-0011, ADR-0049)
  kul-cli/              — binary `kul`: `validate`, `format`, `export`, `query` over a project directory
apps/
  web/                  — the web app (TanStack Start, shadcn/ui, CodeMirror): landing page, docs, llms.txt and the app at /app (ADR-0047)
    src/app/            — the app's UI: shell, project session, editor window, canvas, dialogs, commands and shortcuts
    src/language/       — the language worker and its client
    src/editor/         — the CodeMirror extensions, independent of the app
    src/canvas/         — the canvas's pure modules: camera, reveal, selection, details model, legend
    src/store/          — the project store (OPFS and memory); the only code that touches browser storage
    src/highlight/      — build-time highlighting and checking of Kul for the docs and landing page
    src/docs/, content/ — the docs runtime and the MDX pages
    src/landing/        — the landing page and its build-time content
    src/offline/, src/sw/ — service-worker registration, the update prompt, and the worker itself
    e2e/                — Playwright suites and the screenshot tour
    scripts/            — build steps and checks (service worker, _headers, check-dist, link and deploy-rule checks, serve-dist)
docs/
  vision.md             — language scope and design intent
  architecture.md       — implementation map: pipeline, seams, "where to add X" recipes
  testing.md            — test conventions: snapshots, Vitest, Playwright, perf budgets
  canonical-ui-pattern.md — the canonical family-tree visual
  kinship-graph-shape.md  — the structural framing of a Kul project as a graph
  deploy.md             — how the site deploys to Netlify, its rules, and the hosting and domain setup
  adr/                  — Architectural Decision Records
  prd/                  — Product Requirements Documents (transient — deleted after their epic ships; see [`prd/README.md`](./docs/prd/README.md))
  agents/               — agent-tooling docs (issue tracker, triage labels, domain-docs convention, Rust quality checklist)
spec/                   — Kul 0.1 language specification (the normative source of truth)
examples/               — `.kul` projects used as docs, as the positive test corpus and as the app's example projects
skills/                 — agentskills.io-compliant skills delivered separately via `npx skills add` (see `skills/kul-authoring/`)
scripts/                — repository scripts: the release version check, example rendering, the deploy smoke check, the retired-names search
CONTEXT.md              — domain glossary; canonical vocabulary for the project
```

## Where to look first

| You need to…                                      | Read                                                                                                                               |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Understand the language                           | [`spec/`](./spec/README.md)                                                                                                        |
| Understand the codebase shape                     | [`docs/architecture.md`](./docs/architecture.md)                                                                                   |
| Add a language feature, end to end                | [`docs/architecture.md` → A new language feature](./docs/architecture.md#a-new-language-feature-end-to-end)                         |
| Add a page or route to the web app                | [`docs/architecture.md` → A new app route](./docs/architecture.md#a-new-route-in-the-web-app)                                       |
| Understand the domain vocabulary                  | [`CONTEXT.md`](./CONTEXT.md)                                                                                                       |
| Understand cross-cutting coding principles        | [`CODING_STANDARDS.md`](./CODING_STANDARDS.md)                                                                                     |
| Understand how Kul renders visually               | [`docs/canonical-ui-pattern.md`](./docs/canonical-ui-pattern.md)                                                                   |
| Understand a major design decision                | [`docs/adr/`](./docs/adr/) (the browser app: ADR-0047 to ADR-0052)                                                                  |
| Understand product scope of an epic               | [`docs/prd/`](./docs/prd/)                                                                                                         |
| Add a test or perf budget                         | [`docs/testing.md`](./docs/testing.md)                                                                                             |
| Deploy the site, or set up its hosting and domain | [`docs/deploy.md`](./docs/deploy.md)                                                                                               |
| Cut a release                                     | the `/release` command ([`.claude/commands/release.md`](./.claude/commands/release.md)) — sole source of truth for the release runbook |
| Triage / file an issue                            | [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md), [`docs/agents/triage-labels.md`](./docs/agents/triage-labels.md) |
| Help an AI agent author Kul                       | [`skills/kul-authoring/`](./skills/kul-authoring/SKILL.md)                                                                          |

## Development

### Prerequisites

`just check` builds and tests both halves of the repository, so it needs all of these:

- Rust toolchain (stable, edition 2024). Install via [`rustup`](https://rustup.rs/).
- [`just`](https://just.systems/) — task runner. `cargo install just --locked` or `brew install just`.
- [`cargo-nextest`](https://nexte.st/) — test runner. `cargo install cargo-nextest --locked`.
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) — builds the engine the web app imports (`just wasm`). `cargo install wasm-pack --locked` or the installer script.
- [Node](https://nodejs.org/) at the version in `.nvmrc` (22), then `npm ci` at the repository root. The web app is an npm workspace (`apps/web`), and so is the built engine (`crates/kul-wasm/pkg`).
- Playwright's browsers, once: `cd apps/web && npx playwright install chromium webkit firefox`. Vitest's browser mode runs Chromium and WebKit; the end-to-end suites run all three.
- [`actionlint`](https://github.com/rhysd/actionlint) for `just lint-workflows` (part of `just verify`).

### One command for green

```sh
just check
```

Runs, in order: `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo nextest run --workspace`; `just wasm-check` (the `--target web` build, the committed `.d.ts` diff, the 1 MB gzipped budget, the Node smoke test and the TypeScript compile-test); `just web-check` (the `serve-dist` tests, then the app's typecheck, ESLint, Prettier and colour lint, and Vitest in Node and in Chromium and WebKit); `just web-build` (the production build with prerender, the service worker, the generated `_headers`, and the checks over the output: `check-dist`, `check-links`, `check-landing`, the deploy rules); and `just release-test`. Local-green implies CI-green: `.github/workflows/rust.yml` and `web.yml` run the same recipes.

```sh
just verify
```

Runs `just check`, then `just e2e` (Playwright against the built site served by `serve-dist`, on Chromium, Firefox and WebKit), `just shots` (the screenshot tour, with axe on every state), `just lighthouse` (score floors on the landing page and a docs page), `just retired-names` (fails if a name of a surface Kul retired comes back; `scripts/retired-names.sh`) and `just lint-workflows` (actionlint).

Other recipes:

- `just web` — the app's dev server on a fresh WASM build.
- `just wasm` — build the engine into `crates/kul-wasm/pkg` and refresh the committed `.d.ts`.
- `just test` — Rust tests only. `just fmt` — auto-format. `just lint` — clippy alone.
- `just perf-release` — the per-edit perf gates under a release build.
- `just run -- validate examples/02-three-generations` — passthrough to `cargo run -p kul-cli --`. Every subcommand takes the project directory as its last argument (`[DIR]`, default `.`).
- `just install` — install this checkout's `kul` onto `PATH`.
- `just render-examples` — regenerate each example's committed `tree.svg`.

Inside `apps/web`, `npm run <script>` runs one gate on its own (`typecheck`, `lint`, `test`, `build`, `e2e`, `shots`, `lighthouse`, `dev`).

### Definition of done

A change is done when:

1. `just check` is green. A change to the web app also passes `just e2e`, and a change to what the app looks like passes `just shots` with the changed screenshots opened and checked.
2. New behaviour is covered by tests at the right level — see [`docs/testing.md`](./docs/testing.md): `insta` snapshots for parser, validator and language-feature output ([ADR-0003](./docs/adr/0003-snapshot-tests-as-primary-validation.md)); Vitest for the app's pure modules and anything that needs a real browser API; Playwright for behaviour a user sees. [`CODING_STANDARDS.md`](./CODING_STANDARDS.md) has the behaviour-not-implementation principle.
3. Public items have rustdoc; clippy lints are at deny level (no `#[allow]` without a justifying comment). ESLint runs with `--max-warnings 0`; no `eslint-disable` without a justifying comment.
4. If a non-obvious design choice is being made, it lands as an ADR in [`docs/adr/`](./docs/adr/) — not as a code comment that a future agent might "simplify" away.
5. If your change touches `spec/`, the kinship section of `CONTEXT.md`, or `examples/`, update [`skills/kul-authoring/`](./skills/kul-authoring/SKILL.md) and the docs pages under `apps/web/content/docs/` in the same PR.

## Issues, PRs, and commits

The boundary of one PR is defined by one issue. Each issue represents one atomic unit of work *from a project / product perspective* — one refactor, one feature, one bug fix. PR diff size is not a constraint; what matters is the work-shape.

- **One issue = one PR.** Don't split an issue's work across multiple PRs to `main`. If the work won't fit in one PR, the issue is too big — split the *issue*, not the PR.
- **Squash and merge is the policy.** All PRs squash-merge into `main`; the resulting single commit represents the whole issue's worth of work.
- **Within a PR, commits stay atomic from a *codebase* perspective.** Use [Conventional Commits](https://www.conventionalcommits.org/). Each commit is one logical change that compiles; the PR groups them into one product-perspective unit, and the squash collapses the group on merge.
- **Docs, ADRs, and tests land in the same PR as the code that motivates them.** A PR that ships an ADR but not the code it documents (or vice versa) is not atomic at the product perspective — it splits one piece of work across two `main` commits.

Two layers of atomicity, then: the PR / issue is atomic from a product perspective; commits inside it are atomic from a codebase perspective. The squash-merge policy collapses the second layer into the first on `main`.

Every pull request gets a Netlify preview deploy and a comment with its URL once the repository's Netlify secrets are set ([`docs/deploy.md`](./docs/deploy.md)); a push to `main` deploys production.

## Domain vocabulary

This repo is **single-context**: one [`CONTEXT.md`](./CONTEXT.md) at the repo root plus [`docs/adr/`](./docs/adr/) cover the entire project. When naming things — in issue titles, hypothesis statements, tests, PR descriptions — use the terms `CONTEXT.md` defines. Don't drift into "service / handler / component" speak; the architecture vocabulary (module / interface / seam / depth) is in [`docs/architecture.md`](./docs/architecture.md).

If a concept isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (extend the glossary in the same change).

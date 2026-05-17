# Rust Quality Checklist (for AI agents)

The one-page definition-of-done any agent working in this workspace is held to. It is a digest — the canonical source for each line below is linked. Read those before you start; come back here at the end to confirm.

## Before you write code

- [ ] Read [`AGENTS.md`](../../AGENTS.md) (repo layout, where to look first).
- [ ] Read [`CONTEXT.md`](../../CONTEXT.md) for the domain vocabulary your change names things in. **No drift to "service / handler / component / API".**
- [ ] Read the matching "where to add X" recipe in [`docs/architecture.md`](../architecture.md). If no recipe matches, the change is either misclassified or warrants an ADR (see below).
- [ ] Read any ADR in [`docs/adr/`](../adr/) that touches the area you're editing. The seam ADRs (0001, 0007, 0014, 0015) and the per-feature ones are the most cited.

## While implementing

- [ ] Vocabulary comes from `CONTEXT.md`. If you need a term that isn't there, either find the canonical one or extend `CONTEXT.md` in the same change.
- [ ] Queries go through seams. `ResolvedDocument` is the kinship-query seam ([ADR-0001](../adr/0001-resolved-document-as-query-seam.md)); features and validator rules never walk `document.statements` directly.
- [ ] Additive changes on the AST and field set. Never reorder, rename, or remove. See the **additivity principle** in [`CONTEXT.md`](../../CONTEXT.md) and [ADR-0014](../adr/0014-file-identity-and-per-file-namespaces.md).
- [ ] No speculative abstractions. The **rule of three** applies — extract a helper only when the third consumer asks for it. See [`docs/architecture.md`](../architecture.md) "What not to add."
- [ ] Comments explain *why*, not *what*. The repo prefers ADRs over load-bearing comments because comments get "simplified" away by future agents.
- [ ] No `#[allow(…)]` without a one-line justifying comment. No `#[ignore]`. No `// TODO`.

## Tests

- [ ] Snapshot tests via `insta` for structured output (parser, validator, LSP-feature). Hand-written assertions for cardinal counts, exit codes, cross-platform paths, single-scalar checks. See [`docs/testing.md`](../testing.md) and [ADR-0003](../adr/0003-snapshot-tests-as-primary-validation.md).
- [ ] Validator rule tests cover the **positive case** (rule doesn't fire) *and* the **negative case** (rule fires; diagnostic snapshot). Named `rule_NN_<short_name>` to match the function.
- [ ] Failing fixtures live inline (`let source = "kul 0.1\n…";`), not under `examples/`. The corpus is the **positive** test surface.
- [ ] LSP integration tests use the stdio client in `crates/kul-lsp/tests/`. Each runs `initialize → did_open → request → shutdown`.
- [ ] Perf-sensitive paths get a `#[test]` in `crates/kul-lsp/tests/perf.rs` with a 5× ceiling and the real target in a comment.
- [ ] WASM surface changes: `crates/kul-wasm/tests/typescript/usage.ts` exercises the new shape, and `just wasm` regenerates `kul_wasm.d.ts`. Commit the diff. CI fails on drift ([ADR-0012](../adr/0012-tsify-derived-types-committed-and-diffed.md)).
- [ ] If `*.snap.new` files appeared, `cargo insta review` them. The `Stop` hook will block end-of-turn while any `.snap.new` exists.

## Verify

- [ ] `just check` is green. (= `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo nextest run --workspace`.) Local-green should imply CI-green; the same commands run in `.github/workflows/rust.yml`.
- [ ] If `just check` fails, **fix the regression** — don't loosen the assertion, don't raise a perf budget without a justifying comment, don't `#[allow]` a lint without justification.
- [ ] The per-edit format hook and the blocking clippy hook (configured in `.claude/settings.json`) keep you green during the inner loop. If they ever fail, address the failure before continuing — don't try to outrun them.

## Docs land with code (in the same PR)

- [ ] **Non-obvious design call** → ADR in `docs/adr/`, next number in sequence. Follow the existing 15-ADR format: Status / Date / Deciders / Context / Decision / Consequences / Anti-suggestions.
- [ ] **New domain term** → entry in [`CONTEXT.md`](../../CONTEXT.md).
- [ ] **New spec rule / field / construct** → matching `spec/*.md` updated.
- [ ] **New public type or function on `kul-core`** → rustdoc on the item. If the type crosses the WASM boundary, write the doc for a JS/TS consumer (it becomes the JSDoc on the generated `.d.ts`).

A PR that ships an ADR but not the code it documents (or vice versa) is not atomic at the product perspective. Don't split one piece of work across two `main` commits — see [`AGENTS.md`](../../AGENTS.md) "Issues, PRs, and commits."

## Commit hygiene

- [ ] Conventional Commits inside the PR (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`).
- [ ] Each commit compiles on its own.
- [ ] Only files you explicitly added or modified are staged. Avoid `git add -A` / `git add .`.
- [ ] No `*.snap.new`, no `target/`, no `.env` in the commit.

## When in doubt

- Reach for an existing seam before inventing a new one.
- Reach for a snapshot test before a hand-written assertion.
- Reach for an ADR before a load-bearing comment.
- Reach for the `CONTEXT.md` glossary before inventing terminology.

These four reflexes are most of what keeps the codebase coherent across agents.

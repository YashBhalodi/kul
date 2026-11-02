# Testing

How tests are organized in KulLang and what conventions to follow when adding one.

For the architectural reasons behind these patterns, see [ADR-0003](./adr/0003-snapshot-tests-as-primary-validation.md).

## Layout

Tests in this workspace live at two layers, and both layers are load-bearing.

### Integration tests in `tests/`

| Crate     | Tests live at                                  | Style                                                                          |
| --------- | ---------------------------------------------- | ------------------------------------------------------------------------------ |
| kul-core | `crates/kul-core/tests/{lexer,parser,validator,format,export}.rs` | Integration tests; insta snapshots. `export.rs` macro-generates one snapshot per `examples/*/<stem>.kul` per option matrix (default / with-positions / cytoscape). |
| kul-core | `crates/kul-core/tests/{query,kin,filter,resolve,detail}.rs` | The query seam, one file per capability; insta snapshots of the serialized `QueryEnvelope` over the example corpus. `detail.rs` covers the [batched detail lookup](../CONTEXT.md#batched-detail-lookup) — mixed link kinds, 0..6 parent sets, several marriages, an adoption entity, and the batch's order / `null`-in-position semantics. |
| kul-cli  | `crates/kul-cli/tests/cli.rs`                 | End-to-end via `assert_cmd` + `predicates`. Covers every subcommand including `kul export` (success / failure envelopes, both formats, `--with-positions`). |
| kul-language-service | inline `#[cfg(test)]` modules in `src/*.rs` | Each feature module tests its `pub fn` directly against checked fixtures; insta snapshots of hover Markdown, completion lists, symbols, semantic tokens, rename edits, quick fixes, diagnostics and details. Cross-file cases (definition, references, rename, locate, entity-at) check a two-file project. |
| kul-language-service, kul-wasm, kul-visual | `tests/perf.rs` in each | Performance budget gates (see below). |
| kul-wasm | `crates/kul-wasm/tests/{workspace,utf16}.rs` | The `Workspace` natively: the file lifecycle (versions, rename, delete, file order, re-check after an edit), each method's wiring to the language service, the rendered SVG against `render_from_check` for every example in both themes, and — in `utf16.rs` — every offset crossing the boundary, with Gujarati, Devanagari and emoji before each feature's inputs and outputs. Feature behaviour itself is proven once, in `kul-language-service`. |
| kul-wasm | `crates/kul-wasm/tests/typescript/usage.ts` (driven by `tsc --noEmit` in `just wasm-check`) | TypeScript consumer compile-test. Exercises realistic patterns (narrowing `RenameResult` on `ok`, switching on `EntityDetail.kind`, an exhaustive `TokenKind` switch, slicing JS strings with returned spans, `@ts-expect-error` on illegal arguments) so a `.d.ts` that compiles against itself but isn't usable in real consumer code surfaces. |
| kul-wasm | `crates/kul-wasm/tests/node/smoke.mjs` (driven by `node` in `just wasm-check`) | End-to-end Node smoke test: loads the built `--target web` module with `initSync` and drives a `Workspace` through every method on the example corpus and a non-ASCII two-file project — analyze, hover, completion, rename across files, format, quick fixes, locate, entity-at, details, both token kinds and `renderSvg` in both themes — slicing JS strings with the returned spans. Catches WASM-toolchain or JS-glue regressions invisible to Rust-only tests. |

These cross public-API surfaces and exercise wire formats / process behavior. They are the highest-fidelity tests in the suite.

### Inline tests in `#[cfg(test)] mod tests { … }`

Several src/ files carry inline test modules at the bottom. They serve two roles:

1. **Unit tests of pure-logic helpers** that have no external surface: `kul-wasm`'s `Utf16Index` offset math, `date.rs` partial-date parsing, `span.rs` arithmetic. These could not reasonably live in `tests/` without widening visibility.
2. **Feature entry points**: every `kul-language-service` module tests its `pub fn` inline against checked fixtures. The features are pure functions, so these run in milliseconds and are the features' primary tests; `kul-wasm`'s `tests/` then cover the wiring, the lifecycle and the offset conversion at the boundary, not the features again.

If you're testing genuinely public behavior and don't need the inner-loop speed, `tests/` is the better home. If you're testing a private helper or want sub-millisecond feedback while iterating on a feature module, inline is fine. The deciding question is "does this need to run as part of every cargo nextest in <50ms, and does it touch private surface?" — yes to either pushes inline; no to both pushes to `tests/`.

There are no skipped or ignored tests anywhere in the workspace. If a test is broken, fix it or delete it — don't `#[ignore]` it.

## One command for green

```sh
just check
```

Runs fmt-check, clippy at deny-level, the full test suite via `cargo nextest`, the WASM build and its gates (`just wasm-check`), and the web app's gates. CI runs the same recipes — local-green implies CI-green. If you want only the test suite: `just test`. `just verify` adds the browser suites, Lighthouse and actionlint (see [`AGENTS.md`](../AGENTS.md)).

## Snapshots (insta)

Most parser, validator, and language-feature tests are **snapshot tests**: the output is rendered once, committed to `.snap` files, and subsequent runs assert byte-for-byte equality. This makes regressions visible as a diff rather than a single boolean.

### Workflow

```sh
# 1. Add or change a test. Run it.
cargo nextest run -p kul-core --test validator -E 'test(rule_07_)'

# 2. If the snapshot is new or changed, the test fails and writes a `.snap.new` file.

# 3. Review the diff.
cargo insta review                 # interactive accept/reject

# 4. (Or, after careful inspection:)
cargo insta accept                 # accept all pending

# 5. Commit `*.snap` files alongside the source change.
```

Never commit a `.snap.new` file; the gitignore catches it but always check `git status` before pushing.

### When to use a snapshot vs a hand-written assertion

- **Snapshot**: any structured output a human would otherwise compare field-by-field. Diagnostic lists, parsed AST shapes, completion lists, hover Markdown. Diff-driven review catches subtle regressions (a stray newline, a re-ordered enum variant).
- **Hand-written assertion**: cross-platform path checks, cardinal counts, panics, exact float comparisons, sentinel values. Anything where the test's question is a single boolean.

If you find yourself writing a 50-line assertion of "this field equals X, that field equals Y", switch to a snapshot.

## Examples corpus as positive tests

The `.kul` files under [`examples/`](../examples/) are the **positive test corpus**. Several integration tests glob the directory and assert every file validates cleanly. This means:

- Adding a new example automatically pulls it into the test suite.
- Changing an example may update many snapshots — review the diffs carefully (a single comma in a name field can ripple through hover, completion, and diagnostic snapshots).
- Examples must be self-contained (every reference resolves) and must validate cleanly. Anything else belongs as a per-test inline fixture, not in the corpus.

See [`examples/README.md`](../examples/README.md) for the corpus conventions.

## Negative fixtures

Failing-validation cases live next to the test that exercises them — *not* in the examples corpus. The conventional pattern is an inline `&str` literal:

```rust
mod common;
use common::check_one;

#[test]
fn rule_07_rejects_marriage_with_self() {
    let source = "\
person a name:\"A\" gender:female
marriage m_self a a start:2000-01-01
";
    let diagnostics = check_one(source).diagnostics;
    insta::assert_yaml_snapshot!(diagnostics);
}
```

Don't put failing fixtures in `examples/`. The corpus is for documentation; failing fixtures are for tests.

## Validator rule tests

Each of the thirteen spec rules has its own test function, named `rule_NN_<short_name>` to match the function in `validator.rs`. Each test covers:

1. The positive case — input that should *not* trigger the rule.
2. The negative case — input that *should* trigger it. Snapshot the diagnostic.

If a single test is doing both, that's fine; if it grows complex, split.

When you add a new rule (KUL-R14, etc.), follow the same pattern. The numbering is part of the public diagnostic contract — once shipped, codes don't get reassigned.

## Performance budgets

Perf budgets are tests, not benchmarks. Each crate keeps its gates in one `tests/perf.rs`, easy to find, running as part of `cargo nextest`; `just perf-release` runs the per-edit ones (language service, workspace, render) under a release build:

- [`crates/kul-language-service/tests/perf.rs`](../crates/kul-language-service/tests/perf.rs) — the per-edit language pipeline. `one_thousand_statement_check_and_diagnostics_under_budget` builds a 1000-person document, runs `kul_core::check` plus the diagnostics projection, and asserts a 500ms ceiling (target 100ms); `ten_files_of_one_hundred_statements_under_budget` does the same across ten files, to catch a per-file cost in the resolver.
- [`crates/kul-wasm/tests/perf.rs`](../crates/kul-wasm/tests/perf.rs) — one `Workspace::analyze()` on `examples/09-family-across-a-century` after an edit: check, diagnostics with UTF-16 offsets, and the render, measured natively as the median of nine runs. Targets 2ms release and 10ms debug; ceilings 10ms and 50ms. Observed on a developer laptop: ≈0.17ms release, ≈1.2ms debug. The in-browser measurement (keystroke to canvas swap) lives with the canvas's end-to-end suite.
- [`crates/kul-visual/tests/perf.rs`](../crates/kul-visual/tests/perf.rs) — the canonical render over a generated ~5,000-card project, with each stage (`compute`, `layout`, `render`) timed and an SVG node-count ceiling. Release ≈40ms against a 250ms ceiling; debug ≈190ms against 1500ms.
- [`crates/kul-core/tests/perf.rs`](../crates/kul-core/tests/perf.rs) — the **kinship query engine** ([ADR-0029](./adr/0029-query-engine-performance-posture.md)). `ten_thousand_person_query_operations_under_budget` builds a deterministic ~10k-person synthetic corpus and times the eight representative query operations, guarding the `< ~50 ms/query` interactive target under the engine's no-index / no-cache constraint. `batched_detail_cost_is_flat_in_the_number_of_targets` guards a *shape* rather than a ceiling: the [batched detail lookup](../CONTEXT.md#batched-detail-lookup) answering forty targets must cost no more than 2× answering one, check included ([ADR-0035](./adr/0035-detail-surface-one-selection-one-batched-lookup.md), [ADR-0037](./adr/0037-batched-detail-lookup-shape.md)) — the observed ratio is ≈1.0, and the whole detail surface rests on it.

The "tests, not benches" choice is deliberate:

- Budgets run on every PR — no separate `cargo bench` step to forget.
- A regression fails loudly and immediately.
- Each budget asserts a generous ceiling (typically 5× the real target) so CI runner variability doesn't cause flakes, but doesn't hide a 2× regression. The actual target lives in a comment so a future agent who sees `< 500ms` knows the real budget is 100ms.

When adding a perf-sensitive code path:

1. Add a `#[test]` to `tests/perf.rs` that exercises the hot path through the public surface.
2. Pick a representative workload (size, shape) that matches realistic editor use — not a microbench.
3. Pick a ceiling that absorbs runner variability (5× the target is a reasonable default) and document the real target in a comment.

If a budget gets in the way of a legitimate change, *raise it deliberately*, with a comment explaining why. Don't delete it.

## WASM artifact gates

The WASM artifact has two CI gates that aren't tests in the `cargo nextest` sense, but fail the build the same way:

- **Bundle-size budget** — `just wasm-check` (the `wasm-build` job in `.github/workflows/rust.yml`) gzips the built `.wasm`, prints its size, and fails if it is over 1 MB (it is ≈170 KB). Prevents silent regressions where a dependency bump bloats the artifact.
- **Generated TypeScript types diff** — the same job regenerates `crates/kul-wasm/types/kul_wasm.d.ts` via `wasm-pack build --target web` and `git diff --exit-code`s against the committed snapshot. A Rust-side type change that crosses the WASM boundary fails the merge with a reviewable diff instead of landing as silent runtime drift on consumers (per [ADR-0012](./adr/0012-tsify-derived-types-committed-and-diffed.md)). Regenerate locally with `just wasm` and commit the diff.

Both gates are part of `just check` and `.github/workflows/rust.yml` (per-PR), not just `release.yml` — a Rust-side change that breaks WASM is caught at PR time, not at release tag.

## What not to test

- The exact text of error messages — too brittle. Snapshot the diagnostic struct; if you must check a phrase, use a substring match.
- Floating-point edge cases for date arithmetic — dates are integer math; tests should reflect that.
- Concurrent behavior of the document cache without a deterministic harness — flaky tests teach the team to rerun, not to read.

## Adding a test: checklist

- [ ] Name follows the local convention (`rule_NN_<short_name>` for validator, snake_case description elsewhere).
- [ ] Positive *and* negative case if relevant.
- [ ] Snapshot if the output has structure; assertion if it has a single answer.
- [ ] No `#[ignore]`, no `// TODO: fix later`.
- [ ] `just check` is green.
- [ ] If a snapshot was added or changed, `cargo insta review` shows the diff makes sense and `*.snap` files are committed.

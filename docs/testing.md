# Testing

How tests are organized in KulLang and what conventions to follow when adding one.

For the architectural reasons behind these patterns, see [ADR-0003](./adr/0003-snapshot-tests-as-primary-validation.md).

## Layout

Tests in this workspace live at two layers, and both layers are load-bearing.

### Integration tests in `tests/`

| Crate     | Tests live at                                  | Style                                                                          |
| --------- | ---------------------------------------------- | ------------------------------------------------------------------------------ |
| kul-core | `crates/kul-core/tests/{lexer,parser,validator,format,export}.rs` | Integration tests; insta snapshots. `export.rs` macro-generates one snapshot per `examples/*/<stem>.kul` per option matrix (default / with-positions / cytoscape). |
| kul-cli  | `crates/kul-cli/tests/cli.rs`                 | End-to-end via `assert_cmd` + `predicates`. Covers every subcommand including `kul export` (success / failure envelopes, both formats, `--with-positions`). |
| kul-lsp  | `crates/kul-lsp/tests/{handshake,diagnostics,hover,definition,completion,cold_start,export}.rs` | Stdio LSP client driving the real server; insta snapshots. `export.rs` covers the `kul/export` custom request end-to-end (success, failure, cytoscape, document-not-open error). |
| kul-lsp  | `crates/kul-lsp/tests/perf.rs`                | Performance budget gates (no LSP-protocol round-trip)                          |
| kul-wasm | `crates/kul-wasm/tests/{check,export_graph,format}.rs` | Rust-side snapshot tests over the example corpus, asserting the WASM serde-bridge faithfully round-trips the underlying `kul-core` output. `export_graph.rs` includes a cross-surface bit-identical assertion against `kul_core::export::export`. |
| kul-wasm | `crates/kul-wasm/tests/typescript/usage.ts` (driven by `tsc --noEmit` in CI) | TypeScript consumer compile-test. Exercises realistic patterns (discriminating on `ok`, narrowing `GraphPayload`, iterating `parenthoodLinks`, `@ts-expect-error` on illegal arguments) so a `.d.ts` that compiles against itself but isn't usable in real consumer code surfaces. |
| kul-wasm | `crates/kul-wasm/tests/node/smoke.mjs` (driven by `node --experimental-wasm-modules` in CI) | End-to-end Node smoke test: imports the wasm-pack output as a downstream consumer would, calls all three functions on the example corpus and a known-broken fixture, asserts shape and basic invariants. Catches WASM-toolchain or JS-glue regressions invisible to Rust-only tests. |

These cross public-API surfaces and exercise wire formats / process behavior. They are the highest-fidelity tests in the suite.

### Inline tests in `#[cfg(test)] mod tests { … }`

Several src/ files carry inline test modules at the bottom. They serve two roles:

1. **Unit tests of pure-logic helpers** that have no external surface: `convert::LineIndex` round-trip math, `date.rs` partial-date parsing, `span.rs` arithmetic. These could not reasonably live in `tests/` without widening visibility.
2. **Fast-inner-loop coverage of feature entry points**: `features/{hover,definition,completion,diagnostics}.rs` each have inline tests that call their `pub fn` directly with hand-built `ResolvedDocument` fixtures. They run in milliseconds; the integration tests in `tests/` cover the same scenarios at higher fidelity (full stdio JSON-RPC) but ~25× slower per test. Both layers are kept on purpose — the inline tests catch logic bugs in the dev loop; the integration tests catch wire-format and lifecycle bugs.

If you're testing genuinely public behavior and don't need the inner-loop speed, `tests/` is the better home. If you're testing a private helper or want sub-millisecond feedback while iterating on a feature module, inline is fine. The deciding question is "does this need to run as part of every cargo nextest in <50ms, and does it touch private surface?" — yes to either pushes inline; no to both pushes to `tests/`.

There are no skipped or ignored tests anywhere in the workspace. If a test is broken, fix it or delete it — don't `#[ignore]` it.

## One command for green

```sh
just check
```

Runs fmt-check, clippy at deny-level, and the full test suite via `cargo nextest`. CI runs the same — local-green should imply CI-green. If you want only the test suite: `just test`.

## Snapshots (insta)

Most parser, validator, and LSP-feature tests are **snapshot tests**: the output is rendered once, committed to `.snap` files, and subsequent runs assert byte-for-byte equality. This makes regressions visible as a diff rather than a single boolean.

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

- **Snapshot**: any structured output a human would otherwise compare field-by-field. Diagnostic lists, parsed AST shapes, LSP completion lists, hover Markdown. Diff-driven review catches subtle regressions (a stray newline, a re-ordered enum variant).
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

## LSP integration tests

The LSP integration tests in `crates/kul-lsp/tests/` use a small in-test LSP client (Content-Length framing, JSON-RPC, threads + mpsc) to drive the real server. They are the highest-fidelity tests in the suite — they catch regressions in capabilities advertisement, lifecycle handlers, and message ordering.

Conventions:

- Each feature has its own test file (`hover.rs`, `definition.rs`, etc.).
- Each test does an `initialize` → `did_open` → feature-request → `shutdown` cycle.
- Use `recv_until(predicate, deadline)` instead of `recv()` when the server may emit unsolicited notifications (e.g. `publishDiagnostics`) before your expected response.
- 5-second deadlines are conventional; if your test legitimately needs longer, comment why.

## Performance budgets

Perf budgets are tests, not benchmarks. They live at [`crates/kul-lsp/tests/perf.rs`](../crates/kul-lsp/tests/perf.rs) — one file collecting every gate, easy to find, runs as part of `cargo nextest`. The canonical example is `one_thousand_statement_check_and_translate_under_budget`: builds a 1000-person document, runs the full `kul_core::check` + `to_lsp` pipeline, asserts the elapsed time is under a ceiling.

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

- **Bundle-size budget** — the `wasm-build` job in `.github/workflows/rust.yml` gzips the built `.wasm` and fails if size > 1 MB. Prevents silent regressions where a dependency bump bloats the artifact.
- **Generated TypeScript types diff** — the same job regenerates `crates/kul-wasm/types/kul_wasm.d.ts` via `wasm-pack build` and `git diff --exit-code`s against the committed snapshot. A Rust-side type change that crosses the WASM boundary fails the merge with a reviewable diff instead of landing as silent runtime drift on consumers (per [ADR-0012](./adr/0012-tsify-derived-types-committed-and-diffed.md)). Regenerate locally with `just wasm` and commit the diff.

Both gates are part of `.github/workflows/rust.yml` (per-PR), not just `release.yml` — a Rust-side change that breaks WASM is caught at PR time, not at release tag.

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

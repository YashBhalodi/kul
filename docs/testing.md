# Testing

How tests are organized in KulaLang and what conventions to follow when adding one.

For the architectural reasons behind these patterns, see [ADR-0003](./adr/0003-snapshot-tests-as-primary-validation.md).

## Layout

| Crate     | Tests live at                                  | Style                                                                          |
| --------- | ---------------------------------------------- | ------------------------------------------------------------------------------ |
| kula-core | `crates/kula-core/tests/{lexer,parser,validator}.rs` | Integration tests; insta snapshots                                             |
| kula-cli  | `crates/kula-cli/tests/`                       | End-to-end via `assert_cmd` + `predicates`                                     |
| kula-lsp  | `crates/kula-lsp/tests/{handshake,diagnostics,hover,definition,completion,cold_start}.rs` | Integration tests against a hand-rolled stdio LSP client; insta snapshots      |

Unit tests live in `#[cfg(test)] mod tests { … }` blocks at the bottom of source files. Use them for narrow logic with no external surface (e.g. `LineIndex` round-trip math, date arithmetic). Anything that crosses a public-API boundary belongs in `tests/`.

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
cargo nextest run -p kula-core --test validator -E 'test(rule_07_)'

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

The `.kula` files under [`examples/`](../examples/) are the **positive test corpus**. Several integration tests glob the directory and assert every file validates cleanly. This means:

- Adding a new example automatically pulls it into the test suite.
- Changing an example may update many snapshots — review the diffs carefully (a single comma in a name field can ripple through hover, completion, and diagnostic snapshots).
- Examples must be self-contained (every reference resolves) and must validate cleanly. Anything else belongs as a per-test inline fixture, not in the corpus.

See [`examples/README.md`](../examples/README.md) for the corpus conventions.

## Negative fixtures

Failing-validation cases live next to the test that exercises them — *not* in the examples corpus. The conventional pattern is an inline `&str` literal:

```rust
#[test]
fn rule_07_rejects_marriage_with_self() {
    let source = "kula 0.1
person a name:\"A\" gender:female
marriage m_self a a start:2000-01-01
";
    let diagnostics = kula_core::check(source).diagnostics;
    insta::assert_yaml_snapshot!(diagnostics);
}
```

Don't put failing fixtures in `examples/`. The corpus is for documentation; failing fixtures are for tests.

## Validator rule tests

Each of the thirteen spec rules has its own test function, named `rule_NN_<short_name>` to match the function in `validator.rs`. Each test covers:

1. The positive case — input that should *not* trigger the rule.
2. The negative case — input that *should* trigger it. Snapshot the diagnostic.

If a single test is doing both, that's fine; if it grows complex, split.

When you add a new rule (KULA-R14, etc.), follow the same pattern. The numbering is part of the public diagnostic contract — once shipped, codes don't get reassigned.

## LSP integration tests

The LSP integration tests in `crates/kula-lsp/tests/` use a small in-test LSP client (Content-Length framing, JSON-RPC, threads + mpsc) to drive the real server. They are the highest-fidelity tests in the suite — they catch regressions in capabilities advertisement, lifecycle handlers, and message ordering.

Conventions:

- Each feature has its own test file (`hover.rs`, `definition.rs`, etc.).
- Each test does an `initialize` → `did_open` → feature-request → `shutdown` cycle.
- Use `recv_until(predicate, deadline)` instead of `recv()` when the server may emit unsolicited notifications (e.g. `publishDiagnostics`) before your expected response.
- 5-second deadlines are conventional; if your test legitimately needs longer, comment why.

## Performance budgets

A perf budget is a test, not a benchmark. The canonical example:

```rust
#[test]
fn one_thousand_statement_check_and_translate_under_budget() {
    let source = /* 1000 person declarations */;
    let start = std::time::Instant::now();
    let core = kula_core::check(&source);
    let _ = to_lsp(&url(), &core.diagnostics, &LineIndex::new(&source));
    let elapsed = start.elapsed();
    assert!(elapsed < std::time::Duration::from_millis(500),
            "1000-statement budget exceeded: {elapsed:?}");
}
```

— at `crates/kula-lsp/src/features/diagnostics.rs`. The PRD target is 100ms; the test asserts 500ms (5× slack) to absorb CI runner variability.

The "tests, not benches" choice is deliberate: budgets must run on every PR; they have to fail loudly when violated; they shouldn't require a separate `cargo bench` step. If a future regression doubles parse time, the test fires immediately.

When adding a perf-sensitive code path:

1. Locate the perf budget closest to the hot path (or add a new one).
2. Pick a CI slack factor that absorbs runner variability without hiding 2× regressions. 5× is a reasonable default.
3. The comment in the test records the actual target (so a future agent who sees `< 500ms` knows the *real* budget is 100ms).

If a budget gets in the way of a legitimate change, *raise it deliberately*, with a comment explaining why. Don't delete it.

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

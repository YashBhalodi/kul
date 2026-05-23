# Coding Standards

Cross-cutting principles that apply across this codebase. For mechanical test conventions (where tests live, snapshot workflow, perf budgets), see [`docs/testing.md`](./docs/testing.md) — that file says *where* tests live, this one says *how to think* about what they assert.

## Testing

### Core principle

Tests verify **behavior through public interfaces**, not implementation details. The implementation can change entirely; tests shouldn't break unless behavior changed.

A test that breaks during a refactor that did not change observable behavior is a test that should be rewritten — not a refactor that should be reverted. **An implementation-coupled test is a blocker disguised as a guardian.**

"Public interface" is whichever boundary the test naturally lives at: a `pub` function at the crate edge for integration tests in `tests/`, or a `pub(crate)` function for inline `#[cfg(test)]` tests in `src/`. Both are interfaces. Neither is private state.

### Good tests

```rust
// GOOD — tests observable behavior through the public surface.
#[test]
fn rule_07_rejects_marriage_with_self() {
    let source = "\
person a name:\"A\"
marriage m a a start:2000-01-01
";
    let diagnostics = check_one(source).diagnostics;
    insta::assert_yaml_snapshot!(diagnostics);
}
```

- Test what callers care about.
- Use the public (or `pub(crate)`) interface the module exposes — not its private innards.
- Survive internal refactors.
- One logical assertion per test (a snapshot counts as one assertion).

### Bad tests

```rust
// BAD — reaches into private AST fields, bypassing the public surface.
#[test]
fn parser_sets_node_kind_internal_marker() {
    let ast = parser::parse_for_test(source);
    assert_eq!(ast.nodes[3].kind_raw, NodeKindRaw::Marriage); // private field
}

// BAD — asserts on internal call order; coupled to pipeline structure, not behavior.
#[test]
fn check_invokes_validator_before_formatter() {
    let calls = run_with_call_log(source);
    assert_eq!(calls, &["validator::run", "formatter::run"]);
}
```

Red flags:

- Reaching into private state to verify a result instead of asking through the interface.
- Asserting on call counts, call order, or which internal module was invoked.
- Widening visibility (`pub(crate)` → `pub`, or a `pub fn _for_test`) purely so a test can see internals.
- A test name that describes *how* (`parser_calls_lexer_first`) rather than *what* (`parser_rejects_unterminated_string`).
- Test breaks during a refactor that did not change observable behavior.

### Mocking

Mock at **system boundaries** only:

- External processes — e.g. the LSP client driving the server in `crates/kul-lsp/tests/`, child processes the CLI spawns.
- The file system at the edge, when the test's question is not about disk.
- Time and clock, when behavior depends on them.

**Never mock your own modules or internal collaborators.** If something is hard to test without mocking internals, redesign the interface — the hard-to-test seam is the bug.

In Rust this rules out introducing a `trait` whose only implementation is the real one, purely so a test can swap in a fake. A trait at a test seam is justified only when there is (or will be) more than one real adapter — *one adapter = hypothetical seam; two adapters = real seam* (from the architecture skill's depth vocabulary).

### Implication for refactoring

When an improvement breaks a test, distinguish:

1. **Behavior actually changed** — the test is right. Either the refactor is wrong, or the snapshot needs a deliberate `cargo insta accept` with a commit message that says why.
2. **Behavior unchanged; the test was coupled to the old implementation** — the *test* is wrong. Rewrite it to assert observable behavior through the public surface, then proceed with the refactor in the same commit.

Case 2 is part of the tech debt the refactor is removing. Don't revert the refactor to placate an implementation-coupled test.

### TDD: vertical slices

Do not write all tests first, then all implementation. That produces tests that verify *imagined* behavior and are insensitive to real changes.

One test, one piece of implementation, repeat:

```
RED → GREEN: test1 → impl1
RED → GREEN: test2 → impl2
RED → GREEN: test3 → impl3
```

Each test responds to what you learned from the previous cycle. Never refactor while RED — get to GREEN first.

---

*The testing principles here are adapted from [sandcastle's `CODING_STANDARDS.md`](https://github.com/mattpocock/sandcastle/blob/main/.sandcastle/CODING_STANDARDS.md).*

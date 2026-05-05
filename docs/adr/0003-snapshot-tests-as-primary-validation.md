# ADR 0003 — Snapshot tests are the primary validation strategy

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

The kula-core pipeline produces structured output at every stage: tokens with spans, AST nodes with spans, resolved indexes, lists of diagnostics with codes and related-info. The kula-lsp feature modules produce LSP responses: hover Markdown blocks, lists of completion items, location lists for goto-definition, lists of LSP diagnostics. In every case, the test's question is "did the output structure change?", not "is one specific scalar equal to a literal?"

Three test styles were considered:

1. **Hand-written assertions.** `assert_eq!(diagnostics[0].code, "KULA-R03");` — fine for a single-field check, brittle and verbose for whole-output comparison. A change to a Markdown header text in hover.rs would update two dozen asserts.
2. **Golden-file string comparison.** Render the output as a string, compare to a checked-in `.expected` file. Better than hand-written, but the diff workflow is manual: re-run, diff, copy-paste, commit.
3. **Snapshot tests via `insta`.** The output is rendered (YAML/JSON/text), compared to a `.snap` file. On mismatch, insta writes a `.snap.new` file; `cargo insta review` shows an interactive diff and accepts/rejects. Changes are explicit, reviewable, and committed alongside code.

Insta also handles redaction (filter out timestamps, normalize paths) and supports multiple serialization formats per test — JSON for LSP responses, YAML for diagnostics.

## Decision

Snapshot tests via `insta` are the primary validation method for parser output, validator output, and LSP-feature output. Hand-written assertions are reserved for:

- Cardinal counts ("there are exactly 3 diagnostics").
- Exit-code checks in CLI tests.
- Cross-platform path / panic / sentinel checks.
- Unit tests of pure logic where the output is a single scalar (e.g. `LineIndex` math).

Snapshots are committed to the repo. Acceptance happens through `cargo insta review` in a deliberate review step — not auto-accepted in CI. Pre-existing `.snap` files are gitignored as `.snap.new` until accepted.

The examples corpus under `examples/` is treated as the **positive snapshot corpus**: integration tests glob the directory and snapshot the output for each file. Adding an example automatically extends the test surface.

## Consequences

- Regressions surface as a diff. A subtle change to a hover Markdown line, an off-by-one in a span, a re-ordered field — all produce a diff a reviewer can read in seconds. This is the diff-driven review pattern the codebase is built around.
- Output format becomes part of the public-test contract. Re-ordering fields in a diagnostic struct is a snapshot change in dozens of files. This is a feature, not a bug — it forces the author to confront the blast radius of "trivial" refactors. (To avoid pointless churn, prefer additive changes; see ADR-0001 anti-suggestions.)
- The corpus is load-bearing in two ways at once (docs *and* tests). Writers of new examples must consider snapshot fallout; readers of the corpus get tests that cover real shapes, not contrived ones.
- New contributors must learn the insta workflow. This is a small upfront cost; the [testing guide](../testing.md) documents it.
- Snapshot tests do not catch what they don't render. If a feature's output structure omits a field, the snapshot won't notice. The mitigation is that the output shape is itself defined by a typed Rust struct (`Diagnostic`, `LSP types`, etc.); adding a field anywhere shows up as a snapshot diff.

## Anti-suggestions (do not re-propose)

- "Replace snapshots with hand-written equality assertions for clarity" — clarity is what makes the diff readable; the assertion form is verbose noise. Snapshots win on signal-to-noise for structured output.
- "Auto-accept snapshots in CI" — defeats the review step. The whole point is that a human (or AI agent) decides whether the diff was intentional. CI should fail on `*.snap.new`, not accept it.
- "Hide snapshots in a separate directory, not next to the tests" — insta's default placement keeps `.snap` files adjacent to the test source. Discoverability matters more than directory tidiness.
- "Use a different snapshot tool" — insta is the de facto Rust standard. Switching would cost migration work for no behavioral gain.

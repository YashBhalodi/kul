# PRD 02 — Parser, validator, CLI

**Phase:** 2 of 4
**Headline deliverable:** `kula validate <file>` CLI binary that parses a `.kula` file and reports the 13 spec errors with line/column anchors.
**Target outcome version:** `kula-cli` `0.1.0`, `kula-core` `0.1.0`.

## Problem Statement

Today there is no programmatic way to check whether a `.kula` document is valid. A user authoring a complex family file has no automated feedback — they must read the spec, manually verify each rule, and hope they got it right. Tools that want to consume Kula documents (visualizers, downstream apps, the future LSP) have nothing to call into; everyone would have to re-implement parsing and validation from scratch. Phase 1's TextMate grammar gives colors but cannot detect a missing `gender` field, an unresolved marriage reference, a parenthood cycle, or any of the other 13 spec errors.

## Solution

Build a Rust workspace containing a reusable language-implementation library (`kula-core`) and a thin CLI binary (`kula`) on top of it.

`kula-core` exposes:

- A **parser** that consumes `.kula` source and produces a typed AST with span (line/column) information preserved on every node. The parser recovers from errors so partial/broken sources still yield a usable AST plus a list of parse diagnostics.
- A **semantic analyzer** that resolves cross-references (marriage spouses → declared persons; `birth` and `adoption` marriage refs → declared marriages), produces a "resolved document" model, and computes derived facts (active marriage at time T, parenthood graphs).
- A **validator** that runs the 13 rules from spec section 7 against the resolved document and emits a flat list of diagnostics with span info, severity, and message.
- A **diagnostic** type that any consumer (CLI, LSP, future tools) can render in its preferred format.

`kula-cli` is a thin command-line wrapper:

- `kula validate <file>` parses, validates, and renders diagnostics with rich line/column-anchored output (powered by `miette`-style rendering). Exit code `0` on success, `1` on any error diagnostic.
- Future subcommands (`kula format`, `kula lsp`) slot in without restructuring.

The whole project follows Rust conventions a beginner can navigate: `cargo` workspace, `rustfmt` defaults, `clippy` set to deny-warnings, `cargo-nextest` for the test runner, `insta` for snapshots, `miette` for diagnostic rendering, `clap` (derive style) for arg parsing. A `justfile` at the repo root provides a single `just check` target that runs format-check + clippy + tests, so any contributor (human or agent) has one command to confirm green.

## User Stories

### As the Kula author (end user of the CLI)

1. As a Kula author, I want to run `kula validate family.kula` and have the command exit with status `0` if my file is valid so that I can use it in shell pipelines and CI.
2. As a Kula author, when validation fails I want the exit status to be `1` so that scripts and CI workflows fail loudly.
3. As a Kula author, when validation fails I want each error printed with the file path, line number, column number, and a clear message so that I can navigate directly to the problem.
4. As a Kula author, I want each error to reference the spec rule it represents (e.g. "rule 4: self-marriage") so that I can look up the rule's full definition.
5. As a Kula author, I want errors to be rendered with a snippet of the offending source line and a caret pointing at the bad token (Rust-compiler style, via `miette`) so that I can see context immediately.
6. As a Kula author, I want to be able to validate multiple files in one invocation (`kula validate a.kula b.kula c.kula`) so that I can run a batch check.
7. As a Kula author, I want `kula --version` and `kula --help` to work as expected by Unix conventions.
8. As a Kula author, I want `kula validate -` to read from stdin so that I can pipe content in.
9. As a Kula author, I want a `--quiet` flag that suppresses successful-validation output so that I get clean shell pipeline output.
10. As a Kula author, I want a `--format json` flag that emits diagnostics as machine-readable JSON so that other tools can consume them.
11. As a Kula author on macOS, Linux, or Windows, I want to download a single static binary from GitHub Releases and run it without installing Rust.

### As a downstream tool author (future LSP, future visualizer)

12. As a tool author, I want `kula-core` published as a regular Rust library crate (eventually on crates.io) so that I can `cargo add kula-core`.
13. As a tool author, I want the parser to expose a typed AST (not a JSON blob, not a string) so that I get compile-time guarantees about node shapes.
14. As a tool author, I want the parser to never panic on user input — well-formed or malformed — so that consuming it is safe.
15. As a tool author, I want the parser to produce a useful partial AST even when the input has errors so that I can build features (LSP completion, hover) over half-typed documents.
16. As a tool author, I want diagnostics with stable identifiers (e.g. `KULA-R04`) so that I can suppress, deduplicate, or categorize them.
17. As a tool author, I want every node in the AST to carry a span (file offset + length, or line/column) so that I can highlight problems precisely in any consumer.
18. As a tool author, I want the validator's 13 rules to be runnable independently or as a batch so that I can disable specific rules in specific contexts.

### As an AI agent developer

19. As an AI agent developer, I want a single command `just check` that runs `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo nextest run` so that I can confirm green with one invocation.
20. As an AI agent developer, I want the test suite to complete in under 5 seconds so that my iteration loop is fast.
21. As an AI agent developer, I want every spec rule to have at least one positive (valid) and one negative (invalid, with expected diagnostic) golden test so that I cannot accidentally regress a rule.
22. As an AI agent developer, I want golden tests stored as `.kula` source paired with `.expected.txt` (or `insta` snapshots) so that the diff on a test failure is human-readable and easy to update.
23. As an AI agent developer, I want compiler errors and test failures to anchor at line/column with copy-pasteable file paths so that I can act on them without parsing prose.
24. As an AI agent developer, I want a `CONTRIBUTING.md` (or `AGENTS.md`) that documents: how to set up the dev env, how to run the tests, where each module lives, what "done" means for a feature.
25. As an AI agent developer, I want each module of `kula-core` to have a narrow public interface (e.g. `parse(source: &str) -> (Ast, Vec<Diagnostic>)`) so that I can mock or test in isolation.
26. As an AI agent developer, I want CI on GitHub Actions to run the same `just check` so that local-green implies CI-green.
27. As an AI agent developer, I want zero flaky tests — determinism is non-negotiable.

### As the project maintainer

28. As the project maintainer, I want the repo to follow conventional Rust workspace layout (top-level `Cargo.toml` workspace, sub-crates under `crates/`) so that any Rustacean opening the repo can navigate it.
29. As the project maintainer, I want all Rust code formatted by `rustfmt` defaults so that diffs are pure-content.
30. As the project maintainer, I want `clippy` with `-D warnings` so that lint regressions block CI.
31. As the project maintainer, I want the CLI binary published as a GitHub Release artifact for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, and `x86_64-pc-windows-msvc` so that users can download a binary without installing Rust.
32. As the project maintainer, I want a bumped semver release flow: tagging `cli-v0.1.0` triggers a release workflow that builds binaries and publishes them.

### As a Rust beginner (the repo owner)

33. As a Rust beginner, I want each Rust file to be small (under ~500 lines) and single-purpose so that I can understand each piece in isolation.
34. As a Rust beginner, I want the choice of every external crate documented in a brief `crates/kula-core/CRATES.md` (or in the `Cargo.toml` comments) so that I understand why each dependency exists.
35. As a Rust beginner, I want consistent naming conventions: snake_case modules, PascalCase types, SCREAMING_SNAKE_CASE constants.

## Implementation Decisions

### Workspace layout

A Cargo workspace at the repo root. Two crates land in this phase; a third (`kula-lsp`) lands in Phase 3 in the same workspace.

```
crates/
  kula-core/      # library: parser, AST, semantic, validator, diagnostics
  kula-cli/       # binary: the `kula` command
```

The workspace `Cargo.toml` at the repo root pins the Rust edition (2024 if stable at the time, otherwise 2021) and shared lints (`clippy::all -D warnings`).

### `kula-core` modules (deep modules with narrow interfaces)

Each module below is intended to be deep — significant functionality behind a small, stable interface that the rest of the system depends on without caring about internals.

- **`lexer`** — turns a `&str` source into a stream of typed tokens with span info. Public surface: `tokenize(source: &str) -> Vec<Token>`. Handles comments, strings (with escapes), date literals, identifiers, indentation as INDENT/DEDENT tokens. Indentation handling is centralized here so the parser can treat indentation as just another token kind.
- **`parser`** — turns a token stream into an AST with parse diagnostics. Public surface: `parse(tokens: &[Token]) -> (Document, Vec<Diagnostic>)`. Hand-written recursive descent. Recovers from errors by skipping to a sync token (typically newline) and continuing, so a single broken statement doesn't kill the whole parse.
- **`ast`** — typed AST node definitions (`Document`, `Statement` enum, `PersonStmt`, `MarriageStmt`, `BirthSub`, `AdoptionSub`, `Field` enum, `DateLit`, etc.). All nodes carry spans. References are stored as raw identifiers + span here; resolution happens later in `semantic`.
- **`semantic`** — turns an AST into a `ResolvedDocument`. Builds the ID index, resolves all references, surfaces unresolved-reference diagnostics, and exposes a query API for derived facts (active marriage at time T, biological parents of a person, adoptive parents at time T, full parenthood graph). Public surface: `resolve(doc: &Document) -> (ResolvedDocument, Vec<Diagnostic>)`.
- **`validator`** — runs the 13 rules against a `ResolvedDocument`. Public surface: `validate(resolved: &ResolvedDocument) -> Vec<Diagnostic>`. Each rule is a small function `fn rule_NN(resolved: &ResolvedDocument) -> Vec<Diagnostic>`; the top-level `validate` is just the composition of all rules. Rules can be invoked individually for testing.
- **`diagnostic`** — the `Diagnostic` type used by all of the above. Carries `code` (a stable identifier like `KULA-R04`), `severity`, `message`, primary span, and optional related spans. Implements the `miette::Diagnostic` trait so consumers can pretty-print with one line of code.

The library exposes a top-level convenience API: `pub fn check(source: &str) -> CheckResult` that runs lex → parse → resolve → validate and returns everything in one call. Most consumers (the CLI in this phase, the LSP in Phase 3) will use this entry point.

### `kula-cli` modules

Thin wrapper. One module per subcommand.

- **`main`** — argument parsing via `clap` derive, dispatch.
- **`commands::validate`** — implements `kula validate <file...>`. Reads each file (or stdin for `-`), calls `kula_core::check`, renders diagnostics via `miette`, returns the appropriate exit code.

`kula format` and `kula lsp` subcommands have stub modules that print "not yet implemented" — they fill in during Phases 3 and 4.

### Choice of crates and rationale

| Crate | Purpose | Why this one |
| --- | --- | --- |
| `clap` (with `derive`) | CLI argument parsing | De facto standard, derive-macro style is idiomatic, well-documented |
| `miette` | Diagnostic rendering with line/col context | Industry standard for Rust-compiler-style error output |
| `thiserror` | Error type derivation | Standard for library error types |
| `anyhow` | Error wrapping in the binary | Standard for application-level error handling |
| `insta` | Snapshot testing | Best-in-class for golden-file testing in Rust; great agent DX |
| `cargo-nextest` (dev tool) | Test runner | Faster + clearer output than default `cargo test` |
| `just` (dev tool) | Task runner | Lighter than Make, conventional in modern Rust projects |

No async runtime, no parser-generator framework, no JSON in the core. Every dependency is well-trodden.

### AST and span design

- Spans are stored as `(byte_offset, byte_length)` pairs (more space-efficient than line/col). A `SourceMap` utility converts byte spans to line/col when needed for rendering.
- Each AST node has a single span covering its full extent. Sub-spans (e.g. for the field-name vs the field-value) are stored on the inner nodes.
- The AST is `Clone` and `Debug`-derivable but not necessarily `Serialize`-derivable in this phase (JSON output is for diagnostics, not the AST itself).

### Error recovery strategy

- Lexer is total — every byte sequence produces tokens, including an `Error` token for invalid input. No lexer panic.
- Parser uses **panic-mode recovery**: on an unexpected token, emit a diagnostic, advance to the next newline token, attempt to resume parsing the next statement.
- A document with N broken statements produces N diagnostics, not one fatal error. This is critical for the LSP in Phase 3.

### CLI output format

- Default: `miette` rendering with colors when stderr is a TTY, plain when piped.
- `--format json`: each diagnostic emitted as one JSON object on its own line (newline-delimited JSON / `jsonl`), with stable schema.
- A `--no-color` flag forces colorless output (useful in CI logs).

### Versioning and release

- Crate versions follow semver. `kula-core 0.1.0` and `kula-cli 0.1.0` ship together.
- Git tags: `core-v0.1.0` and `cli-v0.1.0`. CI on tag publishes binaries and (eventually) crates.
- `kula --version` prints both the CLI version and the linked `kula-core` version.

### CI

GitHub Actions workflow `.github/workflows/rust.yml`:

- Triggers on push and pull request affecting `crates/**`, `Cargo.toml`, `Cargo.lock`, or the workflow file itself.
- Matrix: Rust stable on Ubuntu, macOS, Windows.
- Steps: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`.
- A second workflow for releases triggers on `cli-v*` tags and uses `cargo-dist` (or hand-rolled jobs) to build cross-platform binaries and attach them to a GitHub Release.

### Repo-level developer experience

- A `justfile` at the repo root with targets:
  - `just check` — runs fmt-check + clippy + tests
  - `just test` — just the tests
  - `just fmt` — auto-formats
  - `just lint` — clippy alone
  - `just run -- validate examples/03-three-generations.kula` — passthrough to `cargo run`
- A `CONTRIBUTING.md` (or `AGENTS.md`) at the repo root documenting: how to install Rust + just + nextest; how to run tests; the module map; what "done" means for a feature (tests pass, golden snapshots updated, CHANGELOG entry, doc comments on public items).
- A `rustfmt.toml` with project conventions (probably defaults — not customizing without a reason).

## Testing Decisions

### What makes a good test (in this codebase)

A good test asserts the **external behavior** of a deep module — given an input source string, the lexer produces these tokens; given this AST, the validator produces these diagnostics — without coupling to internal data structures. We test the public interface of each deep module in isolation, then have a smaller suite of integration tests that exercise the whole pipeline.

Tests must be **fast** (full suite under 5 seconds), **deterministic** (no flakiness), and **diff-friendly** (snapshot output is human-readable so a test failure is immediately interpretable).

### Per-module test plan

- **`lexer`** — table-driven tests asserting the token stream for representative inputs. Snapshot tests via `insta` for token-stream output of each `examples/*.kula` file.
- **`parser`** — snapshot tests of the AST for each `examples/*.kula` file. Negative tests with intentionally broken inputs verifying error recovery (the parser produces N diagnostics for N broken statements and a usable partial AST).
- **`semantic`** — tests for reference resolution success and failure cases, parent derivation correctness, "active marriage at T" computation across edge cases (concurrent marriages, post-death queries, divorce + remarriage).
- **`validator`** — **golden tests for every one of the 13 rules**, organized as one test module per rule. Each rule's module has at least one positive case (valid input, no diagnostic) and one negative case (invalid input, expected diagnostic with the right code and span). New rules cannot be added without test coverage.
- **`kula-cli`** — integration tests that invoke the binary against `examples/*.kula` and assert exit codes and stderr/stdout content via `assert_cmd` + `insta` for stdout snapshots. JSON-output mode is snapshot-tested.

### Test corpus organization

```
crates/kula-core/tests/
  corpus/
    valid/
      01-single-couple.kula              # mirrored from examples/
      ...
    invalid/
      rule-01-duplicate-id.kula
      rule-02-unresolved-reference.kula
      ...
      rule-13-parenthood-cycle.kula
  snapshots/                              # insta snapshots
```

The `examples/` files at the repo root double as the positive corpus — symlinked or referenced into `valid/`. The `invalid/` directory has one curated file per spec rule, designed to trigger that specific rule and ideally only that rule.

### Prior art

- `rust-analyzer`: snapshot-driven testing with `expect-test` (similar to `insta`). Good reference for how a Rust language tool organizes parser and semantic tests.
- `taplo` (TOML toolkit): clean parser + LSP separation in the same workspace. Reference for the multi-crate layout we're proposing.
- `biome`: snapshot-driven validator tests organized per rule. Reference for the per-rule test module pattern.

### What we don't test

- We don't test private functions directly — we test through the module's public interface.
- We don't test the framework crates (`clap`, `miette`, etc.). They're trusted dependencies.
- We don't test exhaustive Unicode behavior in strings (smoke tests are enough; the spec just says "valid UTF-8").
- Property-based testing (`proptest`) could be useful for parser robustness but is deferred — golden tests cover us for the 13 specified rules, which is what conformance means here.

## Out of Scope

- The Language Server Protocol — Phase 3.
- A formatter (`kula format`) — Phase 4. The CLI subcommand stub exists but prints "not yet implemented."
- The VSCode extension — Phase 1 already shipped, Phase 3 wires it to the LSP. This phase does not touch the extension.
- Multi-file documents, imports, cross-file reference resolution — out of v1 entirely.
- Performance optimization beyond reasonable defaults. Current corpus is small; no profiling needed.
- A query language for derived kinship terms (siblings, cousins, etc.) — `semantic` exposes the graph but doesn't ship pre-built kinship-term queries beyond what the spec section 6.4 mentions.
- Publishing crates to `crates.io`. We can publish `0.1.0` but defer the marketplace presence until the API is more settled (Phase 3 or later).
- A WASM build of the parser. Could be useful for browser-side validation later but not v1.
- `kula-core` Rust API documentation hosted on docs.rs. The crate has rustdoc on public items but the docs.rs publish step is deferred to when we publish to crates.io.

## Further Notes

- **Why hand-written recursive descent over a parser generator (`pest`, `chumsky`, `lalrpop`).** Our grammar has ~20 productions including indentation handling. Indentation is awkward in most parser generators, error recovery is critical (LSP needs partial parses), and a generator's payoff (declarative grammar) is small at this scale. Hand-written gives us total control over both at the cost of slightly more lines of code. The spec's `grammar.ebnf` is the source of truth; we cross-reference it in code comments.
- **Why a single workspace instead of separate repos for the parser and CLI.** They evolve together. A single workspace makes cross-crate refactoring trivial and keeps the AI-agent feedback loop tight. If we ever publish to `crates.io`, the workspace publishes are still independent.
- **Why `miette` for diagnostic rendering.** It's specifically designed for Rust-compiler-style error output with source snippets, captions, and rich formatting. The `Diagnostic` trait is well-designed for our 13 rules.
- **Why `insta` for snapshot testing.** Editing snapshots after intentional output changes is `cargo insta review` — interactive, ergonomic, and agent-readable. Snapshot diffs are pure JSON or plain text, so an agent reading a failed test understands immediately what changed.
- **Why `just` instead of Make.** Cleaner syntax, no tab-vs-space gotchas, conventional in modern Rust projects, well-trodden.
- **Risk: indentation handling.** This is the most novel piece of the parser. We mitigate by writing the lexer's indentation logic first, with extensive unit tests, before building parser logic on top. The lexer emits explicit `INDENT` and `DEDENT` tokens (or a simpler "first token of a sub-statement line") so the parser doesn't deal with whitespace at all.
- **Risk: getting the right error message phrasing.** Diagnostic messages are user-facing and should be tested. Snapshot tests of CLI output verify the rendered messages don't drift accidentally.
- **The `.kula` extension and `kula` binary name** are reserved by the spec (see [`../../spec/10-file-conventions.md`](../../spec/10-file-conventions.md)) and used as-is.

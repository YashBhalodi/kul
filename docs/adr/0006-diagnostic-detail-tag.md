# ADR 0006 — `Diagnostic::detail` is the sub-case discriminator

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

Some validator rules cover multiple distinguishable conditions on the same primary span. The clearest example is `KULA-R03` (required fields missing), which fires for three separate sub-cases — missing `name:`, missing `gender:`, missing marriage `start:` — all anchored on the offending statement's `id.span`. The diagnostic message text differs per sub-case (e.g. it mentions `` `gender:` `` or `` `name:` ``), but the rule code and primary span are the same.

Code-action providers need to distinguish the sub-cases to offer the right quick-fixes. The R03 provider previously did this by string-searching the diagnostic's message: `if diag.message.contains("\`gender:\`")`. The contract — "validator messages are stable, code-actions parse them" — was load-bearing but invisible: a message rewrite would silently break the editor's quick-fixes, with no compiler signal and no test signal until somebody tried the lightbulb.

Three options were considered:

1. **Split the rule code.** R03 becomes R03a / R03b / R03c, each anchored on the same span but with its own code. The code-action registry keys on the new codes. Pros: typed, no per-message coupling. Cons: code splits leak into the spec (`spec/07-validation-rules.md` would either keep one rule with multiple codes — confusing — or split the rule, which doesn't reflect the conceptual grouping). The codes are also user-visible in CLI output and editor diagnostic codes; rewriting them on every taxonomy refinement is churny.
2. **Structured payload (an enum or `Box<dyn Any>`).** A `kind: DiagnosticKind` field with one variant per sub-case. Pros: exhaustively typed. Cons: couples `Diagnostic` to the rule taxonomy; every consumer that *doesn't* care about the discriminator now has a non-exhaustive match to handle; serialisation and snapshot output gains shape per variant.
3. **Opaque tag (`&'static str`).** A `detail: Option<&'static str>` field carrying a short canonical tag like `"r03-missing-name"`. Producer and consumer reference shared `pub const` strings declared next to the producing rule. Pros: minimal-cost on producers and consumers, no impact on consumers that don't care, no exhaustivity headache. Cons: typo-resistance comes from the constants, not from the type system; a brand-new tag string used inconsistently would compile.

## Decision

Option 3. `Diagnostic` gains a `detail: Option<&'static str>` field. The validator sets it via `Diagnostic::with_detail(detail::SOME_TAG)`; consumers match on the literal value via the same `pub const` declared in `kula_core::diagnostic::detail`. Tags are namespaced by rule (`r03-missing-name`, `r05-end-without-end-reason`).

The R03 code-action registry now reads `diag.detail` instead of scanning the message. Same for R05 (which has two sub-cases, "extra `end:`" vs. "extra `end_reason:`"). Other rules continue without a tag — `detail` is `None` — and pay nothing.

## Consequences

- Validator messages can be reworded freely. Code-action wiring keeps working as long as the producer/consumer share the same tag constant.
- Adding a new tag is a one-line `pub const` addition in `kula_core::diagnostic::detail`, plus its use at the producer and any consumer that wants to dispatch on it. The naming convention (`<rule>-<short>`) keeps the constant name self-documenting.
- The `Diagnostic` struct picks up one extra field. All existing snapshot tests that render `Diagnostic` via custom formatters (e.g. `render_diagnostics` in `crates/kula-core/tests/validator.rs`) are unaffected; the few that snapshot via `Debug` derive show an additive `detail: None` line.
- The mechanism applies to any rule that wants to expose internal sub-cases to tooling without splitting its rule code. This is intentional: the spec keeps a clean rule taxonomy; `detail` is a runtime discriminator for tooling.

## Anti-suggestions (do not re-propose)

- "Make `detail` a strongly-typed enum." The same coupling problem as option 2 — every diagnostic-level consumer pays for it, and a new tag becomes an enum variant change rather than a `pub const`.
- "Inline the tag string at every call site, no `pub const`s." Drops typo-resistance. The whole point of the constants is that producer and consumer agree on a single source.
- "Move the `detail::*` constants next to each rule rather than centralising them in `diagnostic.rs`." Tried mentally — call sites don't import them more cheaply, and a single file is easier to diff against the rule taxonomy in the spec.
- "Drop `code` since `detail` carries finer-grained info." `code` is the spec-stable error identifier (KULA-Rxx), surfaced to users in diagnostics. `detail` is for tooling. They have different audiences and lifetimes.

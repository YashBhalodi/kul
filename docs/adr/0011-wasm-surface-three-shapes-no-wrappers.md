# ADR 0011 — WASM surface is three operations, three return shapes, no convenience layer

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** owner

## Context

`kula-wasm` exposes `kula-core` to JS-ecosystem consumers (forthcoming graph renderer, VSCode webview live preview, standalone web playground). The bridge had to settle two questions before any code crossed the WASM boundary:

1. **What's the shape of the public surface?** A single uniform envelope around every operation (`{ ok: true, value }` vs `{ ok: false, diagnostics }` for *every* function), or three operation-specific shapes that mirror what each underlying `kula-core` function actually does?
2. **What goes in the bridge beyond the deep modules?** Convenience query helpers (`byId`, `descendants`, `siblings`, `aliveAt`), error-coercion wrappers, retry layers — or only the raw `wasm-bindgen` translation of `kula_core::check`, `kula_core::export::export`, and `kula_core::format::format_source`?

The deep modules already disagree on failure modes. `format_source` cannot fail — it always returns a string, partial-parse input included. `check` cannot meaningfully "fail" either — it produces a diagnostic list, possibly empty. `export` is the only one with a real binary outcome (success envelope or failure envelope per [ADR-0009](./0009-export-strict-on-diagnostics.md)). Forcing a uniform `{ ok, ... }` shape over all three would either erase real distinctions (giving `format` an artificial failure path) or paper over emptiness with a redundant boolean.

The convenience-helpers question has the symmetric pull. Every consumer will eventually want `byId`, `descendantsOf`, `siblingsOf`. Writing them once in the bridge looks like leverage. But the rule of three says wait — until a third consumer has hand-rolled the same helper independently, the "shared" version is shaped by guesses about what the third caller would want, not evidence.

## Decision

The WASM surface is exactly three `#[wasm_bindgen]` functions plus three version-metadata getters. Each operation returns the shape that mirrors its underlying `kula-core` semantics:

- **`check(source) -> { diagnostics }`.** Always succeeds. An empty `diagnostics` array is the discriminator — there is no `ok` field, because emptiness already carries that information unambiguously.
- **`exportGraph(source, options?) -> SuccessEnvelope | FailureEnvelope`.** A tagged union with `ok: true` / `ok: false`, bit-identical to what `kula export --format=json` (or `--format=cytoscape`) emits. Strict on errors per [ADR-0009](./0009-export-strict-on-diagnostics.md).
- **`format(source) -> string`.** Returns a string unconditionally — best-effort even on partial-parse input, mirroring `kula_core::format::format_source`'s contract. Callers that want to reject malformed input run `check` first.

Plus three version constants exposed as zero-arg functions (wasm-bindgen does not currently support `&'static str` consts at the top level): `KULA_CORE_VERSION()`, `KULA_LANGUAGE_VERSION()`, `EXPORT_SCHEMA_VERSION()`. Consumers negotiate compatibility on these without parsing an envelope.

The bridge contains *no* convenience layer. No `byId`, no `descendantsOf`, no `siblingsOf`, no `aliveAt`. Every such helper is single-digit lines from the exported graph; consumers compose them at the call site. The bridge is a wasm-bindgen translation of three deep modules, and nothing else.

The implementation discipline that backs this decision is in [`crates/kula-wasm/src/lib.rs`](../../crates/kula-wasm/src/lib.rs): each function is a `pub fn` of three lines or fewer, calling exactly one `kula_core::*` function and (for `exportGraph`) one round-trip through `serde-wasm-bindgen`.

## Consequences

- The WASM surface is small enough to memorize and audit. Three function signatures, three return shapes, three constants. The TypeScript file is dominated by types lifted from `kula-core::export` (via Tsify), not by hand-written API conventions.
- `exportGraph`'s output is byte-for-byte identical to `kula export --format=json`. A consumer that switches between server-side CLI and browser-side WASM gets the same bytes. The cross-surface bit-identical assertion in `crates/kula-wasm/tests/export_graph.rs` enforces this.
- `check`'s empty-array discriminator is a single `result.diagnostics.length === 0` check at the call site. No `ok` field to maintain, no extra branch in consumer code. The TS compile-test fixture exercises this pattern.
- `format` always returns a string. Editors that want to format-as-you-type don't have to special-case error states. Editors that want to refuse malformed input run `check` first — a pattern that's already in the LSP server side.
- The deletion test passes by design: removing `kula-wasm` would either reproduce the JS adapter elsewhere or eliminate JS-ecosystem consumers entirely. There is no "but we'd lose the helper layer" pull, because the helper layer does not exist.
- When the third consumer of a derived view (`byId`, `descendantsOf`, etc.) materializes, the helper graduates into a follow-on PRD — written from three concrete call sites, not three guesses.

## Anti-suggestions (do not re-propose)

- **"Wrap every operation in `{ ok, ... }` for consistency."** Erases real semantic differences between operations. `format` cannot fail; `check` reports diagnostics rather than success/failure; only `export` has a binary outcome. Forcing a uniform shape pushes redundant booleans onto consumers and obscures what each operation actually does.
- **"Add `byId` / `descendantsOf` / `siblingsOf` to the bridge now."** Speculative. Single-digit lines from the exported graph at the consumer's call site, where the consumer's exact need is known. Re-propose only after three independent consumers have hand-rolled the same helper — then write a query-API PRD informed by their actual call shapes.
- **"Make `exportGraph` return a partial graph on errors."** Same answer as [ADR-0009](./0009-export-strict-on-diagnostics.md): the strict envelope is the foundation; partial-render UX is a consumer concern. The WASM bridge inherits the CLI's contract verbatim — no per-surface UX policy in the bridge.
- **"Throw JS exceptions on validation errors."** Errors are part of the contract, not exceptions to it. The structured envelope (`FailureEnvelope`) is how the API surfaces them. The panic hook (`console_error_panic_hook`) only triggers on genuine `kula-core` bugs, not expected failure modes.
- **"Hand-write a fluent JS façade in `crates/kula-wasm/src/js/`."** Doubles the surface — every function gains a Rust signature *and* a JS wrapper, both of which need to stay in sync. The wasm-bindgen output IS the public surface; one source of truth.

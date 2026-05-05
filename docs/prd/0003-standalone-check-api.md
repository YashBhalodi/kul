# PRD-0003: Standalone check() public API — validate Kula source as a first-class WASM call

## Problem Statement

The export foundation (#37) embeds validation as the precondition for export — a clean validate is required, and on failure the diagnostics come back inside the export envelope. That is the right scoping for export, but it leaves a gap: a downstream consumer that wants to ask **"is this Kula source valid?"** without paying the cost of, or caring about, an export.

Concretely, a browser-based consumer (the future web visualizer, a VSCode webview live-preview, a wiki-style "preview as you type" widget) frequently has Kula source where it does NOT want the JSON graph yet — it only wants to render error squiggles or show a "fix N issues" badge. Today the only way to surface diagnostics from JavaScript is to call `exportGraph` (after the WASM PRD #38 ships) and look at the failure envelope, which is wasted projection work and forces the consumer to discard a graph it actually wanted on success.

The CLI side already has `kula validate` for this exact purpose. The browser side does not, and inventing it after the fact would push consumers to either reimplement the diagnostic shape in JS or call `exportGraph` for things `exportGraph` was not designed for.

## Solution

Add a dedicated `check(source)` function to the `kula-wasm` surface, exposing the existing `kula_core::check` pipeline as a first-class WASM call.

Returns a structured result containing the diagnostic list (matching the existing `kula validate --format json` shape), with no graph payload. Diagnostics carry severity, code, message, primary span, and related spans — the same shape consumers will already know from the export's failure envelope.

The CLI side already does this via `kula validate`; this PRD does not change CLI behavior. The work is exclusively about extending the WASM surface from one function (`exportGraph`) to two (`exportGraph` and `check`), with all the consumer benefits and design discipline that implies.

## User Stories

1. As a browser-based-consumer-app developer, I want to call `check(source)` and get a structured diagnostic list, so that I can render error squiggles or a problems list without doing a full export.
2. As a live-preview developer, I want `check` to be substantially faster than `exportGraph` on a clean document (because there is no graph to project), so that I can call it on every keystroke without measurable overhead.
3. As a JavaScript consumer, I want `check` to share its diagnostic shape with `exportGraph`'s failure envelope and with the CLI's `kula validate --format json`, so that my code that handles diagnostics is portable across surfaces.
4. As a JavaScript consumer, I want `check` to never throw — always return a structured result — so that error handling is part of the API contract rather than ambient.
5. As a JavaScript consumer, I want `check` to return the empty-diagnostics case unambiguously (`{ diagnostics: [] }`), so that I can short-circuit the "is this clean?" question with a single length check.
6. As a JavaScript consumer building a VSCode-like editor in the browser, I want diagnostic spans returned as byte offsets matching the source string, so that I can convert to my editor's position model myself.
7. As a Kula maintainer, I want `check` and `exportGraph` to share the same `kula_core::check` call internally, so that diagnostics for a given document are bit-identical regardless of which WASM function the consumer called.
8. As a future consumer-app developer, I want this surface in place before I start building, so that the editor experience (diagnostics) and the visualization experience (graph) can be wired up to two clean WASM calls rather than one overloaded one.

## Implementation Decisions

### Modules

- **A new `check` exposed function in the existing `kula-wasm` crate.** The crate already exists from the WASM packaging PRD (#38); this PRD adds one more `#[wasm_bindgen]` function alongside `exportGraph`. No new crate needed.
- **A small return-type addition** — `CheckResult` in WASM-land — wrapping `{ diagnostics: Diagnostic[] }`. Lives in the same hand-written `index.d.ts` already maintained for the WASM crate.
- **Internal sharing.** Both `exportGraph` and `check` call the same `kula_core::check` pipeline; `check` simply does not project the resolved view to a graph. This is the deletion-test-passes shape — there is no separate "validation logic"; it is the same pipeline, just consumed differently.
- **No CLI changes.** The existing `kula validate` subcommand already serves the CLI side of this surface; this PRD does not touch it.

### Contract

- **Always returns a result, never throws.** Same discipline as `exportGraph`. The result shape is `{ diagnostics: Diagnostic[] }` with no `ok` field — a valid document just has an empty array. Simpler than a tagged union because there is no "graph or diagnostics" choice to make.
- **Diagnostic shape matches existing surfaces.** Same fields, same codes (`KULA-Rxx`), same severity values, same span representation as `kula validate --format json` and the export's failure envelope. Single source of truth.
- **No options for v1.** No filters by severity, no rule include/exclude, no formatter selection. The function takes only a source string and returns the full diagnostic list. Filtering is consumer-side and trivial to do in JS.
- **Performance budget.** The `check` call should be measurably faster than `exportGraph` on a clean document because no projection runs. Asserted by a perf-as-test that runs `check` and `exportGraph` on the same 1000-statement source and verifies `check` is ≤ `exportGraph` (with appropriate slack to avoid CI flake).
- **Spans as byte offsets.** The same offsets the existing `Diagnostic` type carries; the WASM bridge does no position translation. JavaScript consumers convert to their editor's position model themselves (matching how the existing CLI emits them as byte offsets in its JSON format).

## Testing Decisions

What makes a good test here: end-to-end exercising `check` from JavaScript as a real consumer would, asserting diagnostic shape and content matches the existing CLI output bit-for-bit, asserting performance properties.

Modules getting tests:

- **The Rust-side WASM adapter for `check`:** unit tests in `crates/kula-wasm/tests/` exercising `check` against a corpus of known-good and known-broken sources. Snapshot the diagnostic output. Per ADR-0003, snapshots are the default for structured output.
- **Cross-surface consistency check:** a test that runs `kula_core::check` directly, runs the WASM `check` adapter, and asserts the diagnostics are bit-identical. Catches any silent transform applied at the WASM boundary.
- **Cross-surface consistency with `exportGraph`:** a test that calls both `check` and `exportGraph` on a broken document and asserts the diagnostics returned are identical. Catches drift between the two surfaces.
- **Performance:** a perf-as-test asserting `check(source)` is at least as fast as `exportGraph(source)` on a clean document, with the same 5× CI slack as the existing perf tests. Catches a regression where check accidentally does projection work.
- **Node smoke test in CI:** the existing Node smoke test from the WASM packaging PRD extends to also exercise `check` against a known-broken example. Catches WASM-toolchain or JS-glue regressions specific to the new function.
- **TypeScript types snapshot:** the existing TS-type snapshot test from the WASM PRD extends to cover the `CheckResult` shape.

## Out of Scope

- **Any change to the CLI.** `kula validate` already serves this purpose for shell consumers. Touching it would be feature creep that this PRD does not justify.
- **A standalone `check` function exposed to non-WASM Rust consumers.** `kula_core::check` already IS that function — the public surface is unchanged.
- **Filtering / configuration options** (by severity, by rule code, by include/exclude). Consumer-side and trivial in JS; adding them to the WASM API would be speculative.
- **Diagnostic source-position translation** (byte offsets → line/column, UTF-16 conversion, etc.). Consumer-side; doing it in WASM would lock in a position model the consumer may not want.
- **A "fix-it" / code-action API.** That is a much larger surface that overlaps the existing LSP code-action machinery; out of scope here.
- **Watch-mode / incremental check.** Consumers that want this can debounce calls to `check` themselves; building incrementality into the WASM API is premature.

## Further Notes

This PRD has a hard dependency on the WASM packaging PRD (#38) shipping first. Without the `kula-wasm` crate and its build/distribution pipeline existing, there is nothing for this PRD to extend.

It is deliberately small. The whole point of the strict-on-diagnostics design in #37 was to keep `exportGraph` doing one thing; the consequence is that `check` becomes a clean separate surface to add later, exactly when consumers need it. This PRD is that "later."

The deletion test is informative: removing `check` from the WASM surface would force every consumer that wants diagnostics-without-projection to either call `exportGraph` and discard the projection, or reimplement the validator in JS. Both are bad enough that `check` earns its spot.

There is no analogous PRD planned for a query API (`descendants`, `ancestors`, etc.) because — as settled in the foundation grilling — JavaScript consumers can derive every kinship query in single-digit lines from the exported graph, and exposing those queries would freeze definitions consumers may want to redefine. If a real consumer eventually hits a wall doing this in JS, that PRD gets written then; not before.

# ADR 0009 — Export refuses on errors; consumer owns mid-edit UX

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

The export's contract on a document with validation errors had to be settled in one of two postures:

1. **Permissive-with-envelope.** Always project a partial graph, even when errors are present. Wrap it in an envelope that carries both `graph` and `diagnostics` so the consumer can render whatever exists and overlay error squiggles on top. Useful for live-preview UX in editors — the user keeps seeing their family while typing the next person's name.
2. **Strict-on-errors.** If any error-severity diagnostic is present, refuse to project. Return a failure envelope carrying only the diagnostics. The consumer is told "your document is broken; nothing to render" and decides what to do — show an error banner, keep the previously-rendered tree, debounce until the doc is clean.

Both positions are coherent. The choice came down to where to put the UX policy. Permissive-mode embeds an opinion ("a partial render is more useful than a banner") into the foundation; every consumer inherits it. Strict-mode hands the policy to the consumer, where it can be tuned per surface — a CLI pipeline wants the loud failure, a web visualizer might want to render the last-clean projection, a future webview live-preview might cache and debounce.

## Decision

The export is **strict on error-severity diagnostics**. The contract is binary:

- If `check.has_errors()` is `false`, return a success envelope carrying the graph.
- If `check.has_errors()` is `true`, return a failure envelope carrying every diagnostic the validator produced (errors, warnings, and notes alike). Do not project.

Warnings alone do not block; the success envelope is returned with no surfaced warnings (a future schema bump may add a warnings array additively).

Consumers that want partial-render UX implement it themselves: cache the last-clean envelope, debounce export calls during rapid edits, render an error banner over the cached graph until the doc validates again. The foundation does not impose this choice.

The CLI runs `check` internally as the precondition for `export` — callers do not need to thread a separate validate step. The function signature is `export(source, &check, options) -> ExportEnvelope`; the LSP integration ([issue #43](https://github.com/YashBhalodi/kul/issues/43)) reuses the cached `CheckResult` from the document store rather than re-checking.

## Consequences

- The foundation is small and easy to memorise. One function, two outcomes, no UX-shaped knobs.
- The consumer surface is more, not less, capable: a permissive-mode foundation can be approximated by a strict-mode foundation plus a one-line consumer cache. The reverse — turning permissive output into a clean strict failure — would require the consumer to re-run the validator.
- Diagnostic shape inside the failure envelope reuses the existing `kul validate --format json` representation. A consumer that already understands `kul validate` output understands the failure envelope.
- The CLI's exit code maps directly to the envelope: `0` for success envelopes, `1` for failure envelopes. CI pipelines need no extra parsing to gate on a clean export.
- A standalone `check()` API that exposes validation as its own product surface coexists without overlap — the WASM bridge in `kul-wasm` ships exactly that (see [ADR-0011](./0011-wasm-surface-three-shapes-no-wrappers.md)), where `check(source) -> { diagnostics }` is its own entrypoint and `exportGraph(source)` reuses `kul_core::check` internally as its precondition.

## Anti-suggestions (do not re-propose)

- **"Default to permissive-mode and add a `--strict` flag."** Exports a UX opinion ("partial-render is the right default") into the foundation. The opposite default — strict — is the one that lets the consumer choose. Never re-propose adding partial-render to the foundation; build it in the consumer.
- **"Add a `?force=true` option that bypasses the error check."** Same shape as the above; same answer. The consumer that wants a partial graph computes it from a stale cached envelope or by re-running with the broken input through their own pipeline. The foundation's contract stays binary.
- **"Surface warnings in the success envelope today."** Tempting because the data is right there, but locks in a surface shape (object key, array of warning objects) before any consumer needs it. A future schema bump can add warnings additively; doing it now is speculative.
- **"Have the LSP `kul/export` request return a partial graph on errors."** Doubles the export's contract for one consumer (the editor) that already has the error squiggles via `textDocument/publishDiagnostics`. The strict envelope keeps the editor's two streams (diagnostics, exports) cleanly separated.
- **"Run a separate `validate` pass internally before `check`."** `kul_core::check` already runs the full pipeline including the validator. Calling it twice is wasted work. The export must trust `check`'s diagnostic list — that's the contract `check` exists to provide.

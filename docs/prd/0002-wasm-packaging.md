# PRD-0002: WASM packaging — kula-core in browser/Node consumers

## Problem Statement

The export foundation has shipped (`kula export` CLI subcommand + `kula_core::export` module + spec §15). Every downstream consumer that runs in JavaScript — the VSCode webview live preview, a web visualizer, a static-site generator that bakes the family tree into HTML, an Observable notebook, etc. — now needs a way to run `kula-core` in its own runtime. The two viable paths are:

1. Reimplement the lex/parse/resolve/validate/export pipeline in JS (lossy and a maintenance time-bomb — every Kula spec change forces a parallel update).
2. Compile `kula-core` to WebAssembly and expose a thin JS-callable surface.

Path 2 is the only one that preserves "the canonical pipeline runs everywhere"; without it, the VSCode webview and the future web visualizer either re-invent the wheel or have to shell out to a CLI binary that browsers cannot invoke.

This PRD is the foundation for every browser-based consumer that comes after — it is the explicit prerequisite for the downstream consumer-app epic.

## Solution

Ship a new `kula-wasm` adapter crate that compiles `kula-core` to WebAssembly via `wasm-bindgen` + `wasm-pack`, plus a published artifact (`.wasm` + JS glue + TypeScript types) that JavaScript consumers can install and use.

The initial surface is intentionally minimal — exactly **one function**: `exportGraph(source, options)` — mirroring the export-only foundation already in `kula_core::export`. Diagnostics, validation as a standalone call, and any kinship queries are deferred to their own PRDs and added to this WASM surface incrementally.

Distribution starts with **GitHub releases only** (a tarball attached alongside the existing `kula` and `kula-lsp` archives). An npm package is a desirable future addition that gets revisited when the first browser-based consumer app is being built — at that point the friction of "manually download and vendor" will be visible and worth solving. Until then, the GitHub-release tarball matches the existing distribution pattern with no new infrastructure to maintain.

The published package contains both a web-target build and a node-target build, so both browser consumers (via `<script type=module>` or a bundler) and Node-based tools (static-site generators, scripts) can use the same artifact.

## User Stories

1. As a web-app developer, I want to install a single artifact that gives me parser-grade Kula-source-to-JSON in my browser, so that I do not have to re-implement the pipeline.
2. As a Node-script author, I want the same artifact to work under Node.js, so that I can build CLI-adjacent tooling without learning Rust.
3. As a JavaScript consumer, I want a typed API (`.d.ts` shipped alongside the WASM), so that my IDE catches mistakes when I invoke the export incorrectly.
4. As a JavaScript consumer, I want the WASM `exportGraph` function to return the same envelope shape as the CLI, so that my code is portable between server-side and browser-side execution.
5. As a JavaScript consumer, I want predictable initialization semantics (an explicit `init()` call), so that I do not have to remember different patterns for browser vs Node.
6. As a JavaScript consumer, I want the WASM payload to be small enough that it does not bloat my bundle (target: well under 1 MB gzipped), so that I can ship it in a normal web app.
7. As a JavaScript consumer, I want the WASM call to fail cleanly with a structured `{ ok: false, diagnostics }` result rather than throwing exceptions, so that error handling is part of the API contract rather than ambient.
8. As a Kula maintainer, I want the WASM build to share its diagnostic and export shape with the CLI, so that consumer behavior never drifts between native and browser environments.
9. As a Kula maintainer, I want the WASM artifact to be version-locked with the CLI / LSP / extension on a single tag, so that there is one release vehicle for the whole project.
10. As a Kula maintainer, I want CI to build and smoke-test the WASM artifact on every push, so that a Rust-side change that breaks WASM is caught before release.
11. As a JavaScript consumer, I want the artifact's GitHub-release archive to contain everything needed (`.wasm`, JS glue, `.d.ts`, README), so that I can vendor it into my project without hunting for pieces.
12. As a future browser-based-consumer-app developer, I want this surface to be in place before I start building, so that my consumer app is unblocked from day one.

## Implementation Decisions

### Modules

- **A new `kula-wasm` crate** (sibling of `kula-cli` and `kula-lsp`). Library + cdylib. Single dependency: `kula-core`. Brings in `wasm-bindgen` and `serde-wasm-bindgen` as build-time deps; nothing in `kula-core` knows WASM exists. Same role and crate-graph position as `kula-lsp` plays for LSP — a thin adapter at the edge of the workspace.
- **A `wasm-bindgen` adapter** exposing a single function: `exportGraph(source: &str, options: JsValue) -> JsValue`. The function calls `kula_core::check(source)` and `kula_core::export::export(...)`, returns the same `ExportEnvelope` (success or failure) serialized via `serde-wasm-bindgen`.
- **Hand-written TypeScript types** (one `index.d.ts`) defining `ExportEnvelope`, `ExportedGraph`, `ExportedDate`, `Diagnostic`, `ExportOptions`, `ExportFormat`. Maintained alongside the Rust types; a snapshot test compares a sample export's TypeScript inference against the declared types so silent drift is caught in CI.
- **A `wasm-pack`-driven build** producing two builds in subdirectories of one npm-style package: `web/` (target browsers, requires explicit `await init()`) and `node/` (target Node.js, no init required). The `package.json` `exports` field routes consumers to the right one automatically.
- **Release-pipeline integration.** Extends `.github/workflows/release.yml` to add a `build-wasm` job that runs `wasm-pack` and uploads the resulting tarball as a release asset alongside `kula-<target>` and `kula-lsp-<target>`. The `verify` job extends to ensure the WASM crate's version field matches `Cargo.toml` and the git tag.

### Contract

- **Tagged-union return value**, never thrown exceptions. Cross-boundary exceptions are messy; success/failure is part of the API contract, not an exceptional condition. Same envelope shape as the CLI.
- **Module format: ESM only.** Every modern bundler is ESM-native. CJS is legacy support no current consumer needs; add it only if a CJS-only consumer shows up.
- **Initialization: explicit `init()`** documented in the README. Browsers cannot synchronously instantiate WASM, so `--target web` requires init; making it explicit for both targets keeps consumer code uniform across environments.
- **Bundle size budget: < 1 MB gzipped, target ~300-500 KB.** Asserted by a CI check that fails if the artifact grows past 1 MB unexpectedly.
- **No JS convenience wrapper layer.** The wasm-bindgen output IS the public surface. Helpers like `byId(graph, "alice")` or `childrenOf(graph, "alice")` are the future query-API epic creeping in via the back door — Rule of Three, ship the bare API and add helpers when a third consumer wants the same code.

### Distribution

- **GitHub releases only for now.** A `kula-wasm-<version>.tar.gz` archive attached to each release alongside the existing `kula` and `kula-lsp` archives. Documentation in the README (and on the release page) explains how to vendor: extract, point your bundler at the package directory, import.
- **No npm publishing in this PRD.** Captured as a follow-up consideration to revisit when the first browser-based consumer app starts. At that point npm becomes worth the credential / publishing-pipeline overhead because consumers will benefit from `npm install`-grade DX.
- **Lockstep versioning.** The `Cargo.toml` workspace version, `editor/vscode/package.json` version, the `kula-wasm` crate version, and the git tag all match. The `verify` job in the release pipeline enforces this — same discipline as today.

## Testing Decisions

What makes a good test here: end-to-end exercising the WASM artifact as a JavaScript consumer would, asserting the envelope shape over example documents, running fast enough to be part of CI on every push.

Modules getting tests:

- **The Rust-side `kula-wasm` adapter:** unit tests in `crates/kula-wasm/tests/` exercising `exportGraph` against the example corpus. Snapshot the returned values; per ADR-0003 this is the default for structured output. The actual export shape correctness is already covered by the foundation PRD's snapshot tests; these tests verify the WASM serde-bridge faithfully round-trips.
- **A Node-based smoke test:** a small Node script (run in CI) that imports the published artifact, calls `exportGraph` against `examples/03-three-generations.kula`, and asserts the envelope matches the same JSON the CLI produces. Catches WASM-build-toolchain or JS-glue regressions that would not show up in a Rust-only test.
- **A bundle-size check:** CI step that fails if the gzipped `.wasm` exceeds the budget. Prevents silent regressions where a dependency bump or feature addition bloats the artifact.
- **TypeScript-type snapshot:** a tiny TS file that imports the types and constructs a fixture envelope; if the types drift away from the actual envelope shape, the TS file fails to compile in CI. Cheap insurance against silent drift between hand-written types and the Rust types they describe.

No browser-side automated tests for v1 — the wasm-pack toolchain plus the node smoke test cover the meaningful surface, and adding a headless-browser harness is out of proportion for a single function. Manual smoke in a real browser is part of the release checklist.

## Out of Scope

- **`check()` exposed as a WASM call.** Captured in its own PRD. This PRD only adds `exportGraph`.
- **Any kinship query API on the WASM surface** (`descendants`, `ancestors`, `siblings`, `alive_at`). Deferred to a much later PRD that may or may not be built; only triggered if a real consumer-app need surfaces.
- **Publishing to npm.** Acknowledged as a likely future need; captured in this PRD's distribution section as a "revisit when the first browser consumer app is being built" note. Not part of the v1 ship.
- **A TypeScript wrapper / SDK** sitting on top of the wasm-bindgen output. Would invent abstractions speculatively; rule of three says wait.
- **The browser-based consumer apps themselves.** This PRD is the foundation; the apps are downstream.
- **CJS module support.** Add only if a CJS-only consumer materializes.
- **Threading / Web Workers / async-by-default.** The export pipeline is fast; synchronous calls are fine for v1. If a future consumer needs to run export off the main thread, they wrap the call in a Web Worker themselves.

## Further Notes

The deep module that `kula-wasm` adapts is `kula_core::export::export` — already in the codebase as of the export-foundation epic, so this PRD is unblocked and ready to start whenever a browser consumer needs it.

The crate-graph position is identical to `kula-lsp`'s: a thin adapter at the edge of the workspace, depending on `kula-core`, that translates a foreign protocol (LSP for the language server, JS / WASM ABI for this crate) into native Kula calls. The deletion test passes the same way: removing `kula-wasm` would either reproduce the JS adapter elsewhere, or eliminate browser/Node consumers entirely.

The choice to ship via GitHub releases first (rather than npm immediately) is explicit and documented as a follow-up consideration. Reasoning: npm adds publishing pipeline complexity (registry credentials, CI flow, lockstep gating) that is only worth taking on when there is a JavaScript consumer who will actually feel the friction of manual vendoring. Publishing to npm before that consumer exists is speculative infrastructure work; deferring keeps this PRD focused.

The WASM artifact's first real consumer will likely be the VSCode webview live-preview feature, which appears in the future consumer-app PRD. Once that consumer is being built, the question of "should we publish to npm?" becomes concrete and gets its own decision.

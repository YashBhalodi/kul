# PRD-0004: WASM packaging — kula-core in browser/Node consumers

## Problem Statement

The CLI, LSP, and VSCode extension serve KulaLang's first-class consumer: a human authoring `.kula` files in an editor. Every other consumer of `kula-core` lives somewhere else — a downstream JS application that consumes a Kula document and does something with it (renders a graph, bakes a static site, embeds in an Observable notebook, exposes a public web playground). Two such consumers are already on the roadmap: a graph-renderer panel that the VSCode extension will host as a webview live preview, and a standalone web playground (editor + graph) accessible to anyone over the internet.

Today there is no path for those JS consumers to reach `kula-core`'s capabilities. The CLI is a binary they can't shell out to from a browser; reimplementing the lex/parse/resolve/validate/export pipeline in JS is a maintenance time-bomb that drifts every time the spec moves. The only viable option is to compile `kula-core` to WebAssembly and expose a JS-callable surface.

## Solution

Ship a new `kula-wasm` adapter crate that compiles `kula-core` to WebAssembly and exposes the three deep-module entrypoints `kula-core` already has — `check`, `export`, `format` — as a JS-callable API published to npm as `@kulalang/wasm` and to GitHub Releases as `kula-wasm.tar.gz`.

The crate sits in the same workspace position as `kula-lsp`: a thin adapter at the edge of the workspace that translates a foreign protocol (the JS / WASM ABI) into native Kula calls. The deletion test passes the same way — removing `kula-wasm` would either reproduce the JS adapter elsewhere, or eliminate JS-ecosystem consumers entirely.

This PRD also bundles three small upstream cleanups in `kula-core` that the WASM packaging surfaces as forcing functions:

1. **Drop `fancy` from the workspace `miette` dep.** `kula-core` doesn't need fancy; it's a CLI-rendering concern. The current setup over-enables. `kula-cli` re-enables `miette/fancy` locally; `kula-core`, `kula-lsp`, and `kula-wasm` use plain miette.
2. **Refactor the export envelope's JSON shape from snake_case to camelCase.** Apply `#[serde(rename_all = "camelCase")]` to the export structs. JS-ecosystem convention; no users exist yet so backwards compat is not a constraint. The CLI's `kula export` output flips in lockstep — single source of truth in `kula-core::export`.
3. **Add a `tsify` feature to `kula-core`.** Optional, default-off. Lets the WASM crate emit accurate, drift-free TypeScript types derived from the Rust source of truth.

Versions stay locked: `kula-core`, `kula-cli`, `kula-lsp`, `kula-wasm`, `editor/vscode/package.json`, the npm `@kulalang/wasm` publish, and the git tag all match. Same `verify` job, one more line.

## User Stories

1. As a downstream JS app developer, I want to install a single npm package (`@kulalang/wasm`) that gives me parser-grade Kula source-to-graph in my browser or Node bundler, so that I do not have to reimplement the pipeline.
2. As a graph-renderer-component author, I want `exportGraph(source, options)` to return the same envelope shape as the CLI emits, so that my code is portable between server-side and browser-side execution.
3. As a standalone web playground developer, I want browser-side `check(source)` and `format(source)` calls in the same package, so that I can render diagnostics and offer a format command without an LSP backend.
4. As a JavaScript consumer, I want a typed API generated from the Rust source of truth, so that my IDE catches mistakes when I invoke the export incorrectly and I never have to wonder whether the types match the runtime.
5. As a JavaScript consumer, I want the WASM functions to fail cleanly with a structured envelope rather than throwing exceptions, so that error handling is part of the API contract.
6. As a JavaScript consumer, I want camelCase field names in the JSON envelope, so that destructured properties read idiomatically in TypeScript / JavaScript code.
7. As a JavaScript consumer, I want the WASM payload to fit comfortably in a normal web app bundle (target ≤ 1 MB gzipped), so that I can ship it without bloating my page.
8. As a JavaScript consumer, I want runtime panics to surface as readable JS console errors rather than opaque WASM traps, so that bugs are debuggable.
9. As a Node consumer using a modern bundler (Next.js, Vite, etc.), I want the package to work out of the box, so that I can use it in static-site generators or backend tooling without configuration.
10. As a JavaScript consumer, I want predictable initialization semantics that the bundler handles transparently, so that I do not have to remember different patterns for browser vs Node.
11. As a JavaScript consumer, I want `check` to return an empty diagnostics array unambiguously when the document is clean, so that I can short-circuit the "is this valid?" question with a single length check.
12. As a JavaScript consumer, I want `format` to always return a string (best-effort even on partial-parse input), so that an editor that wants to format-as-you-type does not have to special-case error states.
13. As a Kula maintainer, I want WASM and CLI to share a single export-shape source of truth in `kula-core::export`, so that consumer behavior never drifts between environments.
14. As a Kula maintainer, I want the WASM crate's version to be locked to the rest of the workspace, so that there is one release vehicle for the whole project.
15. As a Kula maintainer, I want CI to build and smoke-test the WASM artifact on every push (not just on release), so that a Rust-side change that breaks WASM is caught at PR time.
16. As a Kula maintainer, I want the bundle-size budget asserted in CI, so that a dependency bump or feature addition can't silently bloat the artifact.
17. As a Kula maintainer, I want the TypeScript types committed to the repo and diffed in CI, so that a Rust type change that affects consumers shows up as a reviewable diff.
18. As a Kula maintainer, I want the `kula-cli` JSON output and the WASM JSON output to be bit-identical, so that consumers who switch between them are never surprised.
19. As a Kula maintainer, I want the workspace `miette` dependency narrowed so crates only enable the features they actually use, so that downstream artifact sizes (WASM, LSP) aren't paying for the CLI's terminal-rendering machinery.
20. As a future browser-consumer-app developer, I want this surface in place before the consumer-app PRDs land, so that downstream work is unblocked from day one.

## Implementation Decisions

### Modules

- **A new `kula-wasm` crate** at `crates/kula-wasm/`, sibling of `kula-cli` and `kula-lsp`. Library + cdylib. Single dependency on `kula-core` (with the `tsify` feature enabled); build-time deps on `wasm-bindgen`, `serde-wasm-bindgen`, `tsify`, and `console_error_panic_hook`. Same workspace position as `kula-lsp` — a thin adapter at the edge translating the JS/WASM ABI into native Kula calls.
- **Three `#[wasm_bindgen]` exposed functions:**
  - `check(source)` returns `{ diagnostics }` where each diagnostic shares the shape of `exportGraph`'s failure-envelope diagnostics. Always succeeds; an empty array means a clean document. No `ok` field — emptiness is the discriminator.
  - `exportGraph(source, options)` returns the existing tagged `ExportEnvelope` (success-or-failure). Internally calls `kula_core::check` then `kula_core::export::export`. Strict-on-errors per ADR-0009.
  - `format(source)` returns a string unconditionally — best-effort even on partial-parse input, mirroring `kula_core::format::format_source`'s documented contract. Callers that want to reject malformed input run `check` first.
- **Version metadata exports.** `KULA_CORE_VERSION`, `KULA_LANGUAGE_VERSION`, `EXPORT_SCHEMA_VERSION` exposed as `#[wasm_bindgen]` constants so consumers can negotiate compatibility without parsing an export envelope.
- **Panic hook.** `console_error_panic_hook` installed on first call (idempotent). Ensures internal kula-core bugs surface as readable JS console errors rather than `RuntimeError: unreachable`.
- **JS-side naming.** `#[wasm_bindgen(js_name = "...")]` so JS sees `exportGraph` / `check` / `format` (camelCase) while Rust keeps snake_case. Field-name camelCase is handled by the `serde(rename_all)` refactor below — no per-call attribute juggling.

### kula-core changes (in scope of this PRD)

- **Drop `fancy` from workspace miette.** Workspace dep becomes `miette = "7"`. `kula-cli` re-enables `miette/fancy` in its own Cargo.toml. `kula-core` and `kula-lsp` depend on plain miette (which is what the trait impls actually need; fancy was over-broad). Strictly correct, surfaced by but not contingent on the WASM work.
- **Add a `tsify` feature to `kula-core`.** Optional dep, default-off. The export-envelope types carry `#[cfg_attr(feature = "tsify", derive(Tsify))]`. `kula-wasm` enables `kula-core/tsify`; nothing else does.
- **Refactor export JSON to camelCase.** `#[serde(rename_all = "camelCase")]` on the export-envelope structs in `kula-core/src/export.rs` and `kula-core/src/export/cytoscape.rs`. Field renames the JSON sees: `parenthood_links` → `parenthoodLinks`, `end_reason` → `endReason`, `marriage_id` → `marriageId`, `child_id` → `childId`, `byte_start` / `byte_end` → `byteStart` / `byteEnd`, `with_positions` (option) → `withPositions`. CLI output flips automatically. `spec/15-export-schema.md` updated normatively. Existing insta snapshots re-accepted in the same change. The Kula source language keeps its own snake_case identifiers (`end_reason` is a language-level field name); only the JSON projection changes.

### Contract

- **Three return shapes, one per operation's actual semantics.** No forced uniformity. `check` always returns `{ diagnostics }`; `exportGraph` returns a success-or-failure tagged union; `format` returns a string. Each shape mirrors the underlying `kula-core` operation's failure modes precisely.
- **Cross-surface JSON consistency.** WASM `exportGraph` output is bit-identical to `kula export --format=json` output. Both call the same `kula_core::export::export` function with the same options; the WASM bridge does no transformation.
- **Module format: ESM-only, single `--target bundler` build.** Modern bundlers (Vite, Webpack 5+, Next.js, Turbopack, SvelteKit, Nuxt, Astro) handle the bundler-target output natively. esbuild and Rollup users need a WASM plugin. Plain Node scripts (no bundler) need a build step.
- **No JS convenience wrapper layer.** The wasm-bindgen output IS the public surface. Helpers (`byId`, `descendants`, `siblings`) are deferred to a future query-API PRD; rule-of-three says wait until a third consumer wants the same code.

### Distribution

- **Two channels.** `@kulalang/wasm` published to npm; `kula-wasm.tar.gz` attached to each GitHub Release. Both carry the same artifact.
- **Lockstep versioning.** Workspace `Cargo.toml` version, `editor/vscode/package.json` version, `kula-wasm` Cargo.toml (auto via `version.workspace = true`), npm `package.json` (auto via wasm-pack reading from Cargo.toml), and the git tag all match. The `verify` job in `release.yml` enforces all five.
- **First publish coincides with v0.1.0.** Issue #36 (the v0.1.0 release ceremony) is extended with a one-time NPM_TOKEN secret and `@kulalang` npm scope claim, alongside the existing marketplace publisher setup. Implementation work for this PRD ships secrets-free; the human-touched moment is bundled into the release ceremony.
- **Tarball naming: `kula-wasm.tar.gz`.** Matches the existing CLI/LSP convention (no version in the filename — release page provides the version).

### CI integration

- **Every-push WASM build** in `.github/workflows/rust.yml`. Builds via `wasm-pack`, runs the bundle-size assertion (gzipped `.wasm` ≤ 1 MB), runs the Tsify-generated-types snapshot diff, runs the Node smoke test, runs `tsc --noEmit` on the TS consumer compile-test fixture.
- **Release-time publish** in `.github/workflows/release.yml`. Same build, plus `npm publish` (gated on `NPM_TOKEN`), plus tarball upload to the GitHub Release. Gated on the existing `verify` job's version-coordination check.

## Testing Decisions

What makes a good test here: end-to-end exercising the WASM artifact as a JavaScript consumer would, asserting that the bridge faithfully translates the same kula-core deep modules the CLI uses, asserting bundle and type properties that don't show up in Rust-only tests.

Modules getting tests:

- **The Rust-side WASM adapter.** Snapshot tests in `crates/kula-wasm/tests/` exercising `check`, `exportGraph`, and `format` against the example corpus (`examples/*.kula`). Per ADR-0003, snapshots are the default for structured output. The export-shape correctness itself is already covered by `kula-core::export`'s snapshot tests; these tests verify the WASM serde-bridge faithfully round-trips.
- **Cross-surface bit-identical JSON.** A test that runs `kula_core::export::export` directly and runs the WASM `exportGraph` adapter on the same source, asserts the JSON is byte-for-byte identical. Catches any silent serde transform applied at the WASM boundary.
- **Tsify-generated `.d.ts` snapshot.** The generated TypeScript types are committed to the repo. CI regenerates them via `wasm-pack build` and `git diff --exit-code`s. A change to a Rust type that crosses the boundary surfaces as a reviewable types-file diff, not as silent runtime drift.
- **TypeScript consumer compile-test.** A `tests/typescript/usage.ts` fixture imports the generated types and exercises realistic consumer patterns (discriminating on `ok`, narrowing `GraphPayload`, iterating `parenthoodLinks`, handling diagnostics). CI runs `tsc --noEmit`. Catches the case where types compile against themselves but aren't usable in real consumer code.
- **Node smoke test.** A small Node script imports the published-shape package and exercises all three functions end-to-end against `examples/03-three-generations.kula` (and at least one known-broken example). Catches WASM-toolchain or JS-glue regressions invisible to Rust-only tests.
- **Bundle-size assertion.** CI step that gzips the built `.wasm` and fails if size > 1 MB. Prevents silent regressions where a dependency bump bloats the artifact.

Prior art: the `kula-cli` and `kula-lsp` crates use insta snapshot tests for structured output (per ADR-0003); the WASM snapshot tests follow the same pattern. The existing `release.yml` build-cli / build-lsp matrix is the prior art for the new `wasm-build` and `wasm-publish` jobs.

## Out of Scope

- **JS query helpers** (`byId`, `descendants`, `ancestors`, `siblings`, `aliveAt`). Consumers can derive these in single-digit lines from the exported graph; the rule-of-three discipline says wait until a third consumer wants the same code.
- **CJS module support.** Single ESM bundler-target package only. Add only if a CJS-only consumer materializes.
- **Plain Node scripts (no bundler / no build step).** The bundler target requires a build step; consumers running ad-hoc `node script.js` need to add one.
- **Web Worker / threading wrapper.** Synchronous calls are fine for typical document sizes. Consumers wanting off-main-thread execution wrap the WASM call in a Web Worker themselves.
- **Browser-side LSP-equivalent language services beyond the three core functions.** Hover, completion, find-references, code actions, semantic tokens, etc. are LSP/editor concerns and remain in the `kula-lsp` binary served over stdio. A future "browser language services" PRD could add them if a real consumer needs them.
- **The graph-renderer component itself.** This PRD ships the WASM substrate; the renderer (the actual JS UI that turns the graph payload into pixels) is a separate downstream PRD.
- **The VSCode webview live-preview integration and the standalone web playground app.** Consumer-app PRDs, downstream of this one and the future graph-renderer PRD.
- **CDN-direct browser usage** (`<script type="module" src="...">`). Out of scope until a consumer needs it; would require a separate `--target web` build alongside the bundler one.
- **Changes to `kula validate` CLI behavior.** The CLI's `kula validate --format=json` shape flips to camelCase as a consequence of the export-shape refactor, but the CLI's exit codes, flags, and overall UX are unchanged.

## Further Notes

The deep modules this PRD adapts (`kula_core::check`, `kula_core::export::export`, `kula_core::format::format_source`) all already exist in the codebase. The WASM crate is purely additive on top of shipped functionality, plus the three small upstream cleanups in `kula-core` (miette, tsify, camelCase).

The crate-graph position is identical to `kula-lsp`'s: a thin adapter at the edge of the workspace that translates a foreign protocol into native Kula calls. The deletion test passes the same way.

The first known consumers downstream of this PRD are the graph-renderer component (its own future PRD) and two consumer apps that will mount the renderer: the VSCode webview live preview, and a standalone web playground accessible over the internet. Per the design discussion, the VSCode webview is a graph-only renderer driven by the VSCode editor proper (LSP handles editor-side language services in the editor pane); the standalone playground hosts a browser-side editor and uses all three WASM functions.

The deliberate choice to ship npm publishing in v0.1.0 (rather than starting with GitHub-release tarball only and adding npm later) reflects the framing that WASM packaging exists to make `kula-core` capabilities available to the JS ecosystem broadly — and the JS ecosystem moves through npm. Manual-vendor tarball is a fallback channel, not the primary distribution path.

The `console_error_panic_hook` cost is approximately 3 KB compressed — well within the bundle budget. The hook only triggers on genuine bugs (Rust panics); expected failures (parse errors, validation errors) flow through the structured envelope as designed.

Once v0.1.0 ships and real consumers start using `@kulalang/wasm`, follow-up questions worth flagging for a future PRD: CDN-direct usage (`<script type="module">` from unpkg/jsDelivr) and a possible plain-Node target for ad-hoc scripts. Adding a `--target web` build alongside bundler is straightforward when motivated; the wasm-pack pipeline produces both from the same crate.

This PRD supersedes and replaces the earlier brainstorm-dump PRDs `0002-wasm-packaging.md` and `0003-standalone-check-api.md`, which are deleted in the same change. The substance of those drafts informed this PRD's decisions; nothing load-bearing is lost.

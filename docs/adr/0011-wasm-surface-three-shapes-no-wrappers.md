# ADR 0011 — WASM surface is operation-specific shapes, no convenience layer

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** owner

## Context

`kul-wasm` exposes `kul-core` to JS-ecosystem consumers (forthcoming graph renderer, VSCode webview live preview, standalone web playground). The bridge had to settle two questions before any code crossed the WASM boundary:

1. **What's the shape of the public surface?** A single uniform envelope around every operation (`{ ok: true, value }` vs `{ ok: false, diagnostics }` for *every* function), or three operation-specific shapes that mirror what each underlying `kul-core` function actually does?
2. **What goes in the bridge beyond the deep modules?** Convenience query helpers (`byId`, `descendants`, `siblings`, `aliveAt`), error-coercion wrappers, retry layers — or only the raw `wasm-bindgen` translation of `kul_core::check`, `kul_core::export::export`, and `kul_core::format::format_source`?

The deep modules already disagree on failure modes. `format_source` cannot fail — it always returns a string, partial-parse input included. `check` cannot meaningfully "fail" either — it produces a diagnostic list, possibly empty. `export` is the only one with a real binary outcome (success envelope or failure envelope per [ADR-0009](./0009-export-strict-on-diagnostics.md)). Forcing a uniform `{ ok, ... }` shape over all three would either erase real distinctions (giving `format` an artificial failure path) or paper over emptiness with a redundant boolean.

The convenience-helpers question has the symmetric pull. Every consumer will eventually want `byId`, `descendantsOf`, `siblingsOf`. Writing them once in the bridge looks like leverage. But the rule of three says wait — until a third consumer has hand-rolled the same helper independently, the "shared" version is shaped by guesses about what the third caller would want, not evidence.

## Decision

The WASM surface is one stateful class, `Workspace`, plus two version getters (`KUL_CORE_VERSION()`, `KUL_LANGUAGE_VERSION()`; wasm-bindgen does not support `&'static str` consts at the top level). See the 2026-11-02 amendment below for why it holds state.

- **The workspace holds a project's files** — `setFile(name, text, version)`, `renameFile(from, to)`, `deleteFile(name)` — checks them on demand, and caches the `CheckResult` (and with it the `ResolvedDocument`, [ADR-0007](./0007-resolved-document-owns-document.md)) until the next edit. Edits replace a file's whole text; the version is the caller's tag for that text.
- **Each method is one deep-module call**, returning the shape that mirrors it. No uniform envelope is forced over the surface:
  - `analyze() -> Analysis` — `{ files, diagnostics, svg? }`: every file with the version analyzed, every diagnostic of the project, and the theme-agnostic SVG when the tree can be drawn. There is no `ok` field; the diagnostics list and the presence of `svg` already say it.
  - One method per language-service feature (`hover`, `complete`, `definition`, `references`, `prepareRename`, `rename`, `symbols`, `quickFixes`, `semanticTokens`, `lexicalTokens`, `format`, `entityAt`, `locate`, `details`), each a thin call into `kul-language-service` returning its Kul-native type, `undefined` where the service answers nothing.
  - `rename(...) -> RenameResult` is the one tagged union, `{ ok: true, edits } | { ok: false, message }`, because a refused rename has a reason the author must see.
  - `renderSvg(theme) -> string | undefined` renders the same tree as `analyze()` in the `agnostic` theme, or self-contained with a legend in the `fileExport` theme (ADR-0022), through `kul_visual::render_from_check` (ADR-0031).
- **Offsets are UTF-16 code units** at the boundary, in both directions. Rust counts UTF-8 bytes; one converter per file (`Utf16Index`) translates every offset that crosses, so neither the language service nor the JavaScript side handles the other's unit.

The bridge contains *no* convenience layer. No `byId`, no `descendantsOf`, no `siblingsOf`, no `aliveAt`, and no kinship queries: the query engine stays in `kul-core` and the CLI. The implementation discipline is in [`crates/kul-wasm/src/lib.rs`](../../crates/kul-wasm/src/lib.rs): each method converts its offsets, calls one function, and converts the result.

## Consequences

- The WASM surface is small enough to memorize and audit: one class whose methods are the language service's functions, plus two getters. The TypeScript file is dominated by types derived from the Rust result types (via Tsify, [ADR-0012](./0012-tsify-derived-types-committed-and-diffed.md)), not by hand-written API conventions.
- The SVG the workspace renders is the SVG `kul export --format=svg` writes for the same files: both go through `render_from_check`, and the workspace tests assert it for every example in both themes.
- A clean project is `analysis.diagnostics.length === 0` at the call site. No `ok` field to maintain, no extra branch in consumer code.
- A JavaScript caller never converts offsets. CodeMirror positions, `String.prototype.slice` and every span the workspace returns are all UTF-16 code units; the conversion is tested with Gujarati, Devanagari and emoji at every method.
- The deletion test passes by design: removing `kul-wasm` would either reproduce the JS adapter elsewhere or eliminate the browser app. There is no "but we'd lose the helper layer" pull, because the helper layer does not exist.
- When the third consumer of a derived view (`byId`, `descendantsOf`, etc.) materializes, the helper graduates into a follow-on PRD — written from three concrete call sites, not three guesses.

## Anti-suggestions (do not re-propose)

- **"Wrap every method in `{ ok, ... }` for consistency."** Erases real semantic differences between methods. `format` answers with edits or nothing; `analyze` reports diagnostics rather than success/failure; only `rename` has a refusal that needs a reason. Forcing a uniform shape pushes redundant booleans onto consumers and obscures what each method actually does.
- **"Add `byId` / `descendantsOf` / `siblingsOf` to the bridge now."** Speculative. Single-digit lines from the exported graph at the consumer's call site, where the consumer's exact need is known. Re-propose only after three independent consumers have hand-rolled the same helper — then write a query-API PRD informed by their actual call shapes.
- **"Render a partial tree on errors."** Same answer as [ADR-0009](./0009-export-strict-on-diagnostics.md): rendering is strict; keeping the last valid tree on screen is a consumer concern. The bridge inherits the CLI's contract verbatim — no per-surface UX policy in the bridge.
- **"Throw JS exceptions on validation errors."** Errors are part of the contract, not exceptions to it. Diagnostics and `RenameResult` are how the API surfaces them. The panic hook (`console_error_panic_hook`) only triggers on genuine `kul-core` bugs, not expected failure modes.
- **"Hand-write a fluent JS façade in `crates/kul-wasm/src/js/`."** Doubles the surface — every function gains a Rust signature *and* a JS wrapper, both of which need to stay in sync. The wasm-bindgen output IS the public surface; one source of truth.

## Amendments

### 2026-05-23 — `renderSvg` added (issue #126)

The canonical visual pipeline (`kul_render::compute` → `kul_layout::layout` → `kul_svg::render`) shipped behind the LSP's `kul/render` custom request in [PRD 0002](../prd/0002-canonical-ui-pattern-visual-renderer.md)'s v1 slice. JS-ecosystem consumers (web playground, future web app) could reach the pipeline only by shelling out to the LSP or reimplementing it. This amendment extends the WASM surface with a fourth operation, `renderSvg(files, manifest) -> RenderSuccess | RenderFailure`, whose JSON shape is bit-identical to `kul_lsp::features::render::RenderResponse` (enforced by a cross-surface snapshot test).

This is an extension of the principle, not a reversal. The original decision's spine — operation-specific shapes that mirror what each underlying call actually does, plus no convenience layer — applies to `renderSvg` verbatim. The three anti-suggestions above remain accurate: no uniform envelope, no `byId`-style helpers, no partial-graph fallback, no exception-based errors, no hand-written JS façade. The title and decision section are amended in place to drop the "three" count; the surface is now small but not literally three.

The rule-of-three on shared envelope types fires here: `RenderResponse` (LSP) and `RenderEnvelope` (wasm) are the first two consumers of "an envelope around `{ ok, svg | diagnostics }`". A third independent consumer would justify lifting them into a common crate; until then, the two adapters declare their own envelopes and a snapshot test enforces bit-identity at the JSON level.

### 2026-11-02 — a stateful `Workspace` replaces the stateless operations (issue #346)

The browser app's editor needs every language feature on every keystroke's worth of change: hover, completion, rename across files, highlighting, plus the live render. The stateless operations (`check(files)`, `exportGraph(files)`, `format(source)`, `renderSvg(files)`) re-ran the whole check on each call and took the file set each time; serving a dozen features per edit through them would multiply the dominant cost by a dozen. The query operations (`queryPerson`, `queryMarriage`, `queryDetail`, `queryKin`, `runQuery`, `queryResolve`) served a query UI the app does not have.

So the surface becomes the `Workspace` described in the Decision above. It holds the files and caches one check per edit; the language features move from the retired language server into `kul-language-service` and the workspace is their adapter, converting offsets to UTF-16 at the boundary. The stateless and query operations, `EXPORT_SCHEMA_VERSION()` (only `exportGraph` emitted that schema), and the Tsify derive on every type that no longer crosses the boundary are removed. `wasm-opt` drops the query engine from the blob: the gzipped module went from 208,251 to 169,798 bytes. The build is `--target web` only.

The spine of the original decision holds: shapes that mirror what each underlying call does, and no convenience layer. Holding state does not add a layer — the workspace's state is exactly the inputs a stateless call took, kept between calls. The anti-suggestions above are restated for the new surface.

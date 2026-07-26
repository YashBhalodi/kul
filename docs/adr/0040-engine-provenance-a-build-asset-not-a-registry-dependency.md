# ADR 0040 — Engine provenance: the webview's engine is a build asset, not a registry dependency

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[ADR-0034](./0034-query-transport-and-result-node-identity.md) decided the query *transport*: the
engine runs in the webview, `kul-lsp` gains no sixth custom method, and the module loads lazily on
first query. It did not decide the module's **provenance** — where the bytes the webview loads come
from. Its consequences section, and issue [#297](https://github.com/YashBhalodi/kul/issues/297) after
it, assumed the obvious answer: *"the preview package gains a runtime dependency on
`@kullang/wasm`"*.

Building #297 established four facts that make that answer unimplementable as written, and a fifth
that makes a better one available:

1. **`crates/kul-wasm/pkg/` is gitignored.** Only the generated `.d.ts` is committed, extracted to
   `crates/kul-wasm/types/kul_wasm.d.ts` by the `just wasm` recipe. Declaring the build output an npm
   workspace member would break `npm ci` on a fresh clone.
2. **The operation the epic needs does not exist in the published package.** `queryDetail`
   ([ADR-0037](./0037-batched-detail-lookup-shape.md), [#306](https://github.com/YashBhalodi/kul/issues/306))
   landed in this repo after `@kullang/wasm@0.4.3` was published, and epic
   [#296](https://github.com/YashBhalodi/kul/issues/296) ships to `main` with **no release**. A
   registry dependency would therefore be pinned to a version that cannot answer the detail panel.
3. **The `.wasm` was never going to be bundled anyway.** ADR-0034 already decided the module loads
   lazily and that the CSP needs a `connect-src` allowance to *fetch* it. It is a fetched asset
   regardless of where its glue JS comes from, so an npm dependency would buy only the glue.
4. **The extension already has an asset-staging step.** `editor/vscode`'s `build:preview` runs
   `scripts/copy-preview-assets.mjs` into `media/preview/` before `vsce package`.
5. **`wasm-pack` builds a `--target web` output** — an ESM module with an `init(url)` entry point
   that fetches and instantiates — alongside the existing `--target bundler` one, from the same
   crate, into a second directory.

## Decision

### The engine reaches the webview as a build asset, and `@kullang/preview` never imports it

There is no `@kullang/wasm` edge anywhere in the npm dependency graph. Instead:

- **`just wasm` builds a second output**, `crates/kul-wasm/pkg-web/`, with
  `wasm-pack build --target web`. The existing `--target bundler` build in `pkg/`, the `package.json`
  name patch, the committed `.d.ts` extraction and `release.yml`'s `wasm-publish` job are all
  **untouched**. The npm package keeps shipping exactly as it did.
- **The extension stages `pkg-web/` into `media/preview/wasm/`** in the asset-copy step it already
  runs, and hands the webview the two resulting webview-resource URIs (the glue module and the
  `.wasm`) through the HTML shell, as `data-kul-engine-module` / `data-kul-engine-wasm` on the mount
  point.
- **`@kullang/preview` receives a URL, not a module.** `createQueryEngine({moduleUri, wasmUri})`
  dynamic-`import()`s the glue on the first query and calls its `init` with the binary's URI.

This is **better than what #297 anticipated**, not merely a workaround for it. That issue noted the
`@kullang/wasm` dependency would be "the first thing in a host-agnostic chrome package that is not
host-agnostic". Under this decision nothing in the package is host-specific: the chrome is handed a
URL by whoever embeds it, which is the same shape as the stylesheet and script URIs it already takes.
A future web embedding supplies its own two URLs and needs no change here.

### Types come from the committed `.d.ts`, checked by a text conformance test

The preview must know the wire shapes without importing the module. It gets them from
`crates/kul-wasm/types/kul_wasm.d.ts` — the tsify-derived snapshot already committed under
[ADR-0012](./0012-tsify-derived-types-committed-and-diffed.md) — restated by hand in
`packages/preview/src/engine-wire.ts`, with `packages/preview/tests/engine-wire.test.ts` re-reading
that file and failing when a mirrored declaration drifts. A second assertion fails when a type is
added to the mirror without being added to the checked list, and a third fails if any preview source
file ever grows an `@kullang/wasm` import.

Only the closure `queryDetail` needs is mirrored. The `RelationshipDescriptor` family belongs to the
phrasing layer ([ADR-0033](./0033-phrasing-layer-architecture.md)), which mirrors it against the same
generated file; duplicating it here would put two mirrors on one contract.

**Generating the mirror from the `.d.ts` was rejected.** A codegen step is a build-order dependency
and a generated file to review, to buy a check a 60-line test already performs — and the test fails
*louder*, because it names the drifted declaration.

### `wasm-pack` is a prerequisite for packaging the extension, not for `npm ci`

`AGENTS.md` said wasm-pack was needed "only for `just wasm`". It is now also needed to package the
`.vsix`, and both CI workflows that package one install it:
`.github/workflows/vscode-extension.yml` before `vsce package`, and `release.yml`'s
`extension-publish` job before its. `npm ci`, `just check`, and every Rust gate are unaffected — a
fresh clone with no wasm toolchain still installs, still typechecks and still tests, because the
unit tests substitute the module loader at its one system boundary rather than loading a real module.

A build with no staged engine is a supported state, not a broken one: the extension detects the
missing asset, logs it, omits the URIs, and the shell then emits the **tighter** CSP. The preview
renders; it just answers no queries.

### What this does and does not do to ADR-0034's recorded skew risk

ADR-0034 recorded, and deliberately accepted, that *"two engines describe the same file, knowingly"*:
the LSP renders the picture, the bundled WASM answers the queries, and a skewed pair would produce
silently wrong answers about a family rather than a crash.

**This decision fixes one half of that and leaves the other exactly as it was.**

- **Fixed: the engine can no longer lag the repo.** The querying engine is built from
  `crates/kul-wasm` at packaging time, so it is always the same commit as the `kul-core` the rest of
  the artifact was cut from. The registry alternative could not promise this — it would have pinned
  a published version that is, by construction, older than the branch, and #306's operation proves
  that gap is not hypothetical.
- **Not fixed: the LSP can still be a different build.** `editor/vscode/scripts/fetch-server-binaries.mjs`
  fetches server binaries separately, and `kul.serverPath` lets a user point at any `kul-lsp` on
  disk. So the core that *renders* can still differ from the core that *queries*, and the symptom is
  unchanged: wrong answers, not a crash.

`KUL_CORE_VERSION` is still exported and a startup handshake is still one field away. ADR-0034 said
not to build it now; that instruction stands, and this decision does not change the calculus — it
only narrows which pairs can drift.

## Consequences

- **`npm ci` on a fresh clone keeps working**, and `just check` needs no wasm toolchain. The TS gate
  substitutes the module loader.
- **`just wasm` now produces two directories.** `pkg/` publishes; `pkg-web/` ships inside the
  extension. Both are gitignored.
- **Packaging the extension fails loudly without `pkg-web/`**, with a message naming `just wasm`.
  Previously `wasm-pack` was optional for every workflow; now it gates one.
- **The `.vsix` grows by ~630 KB** (the `--target web` `.wasm` plus ~29 KB of glue). It is fetched
  only on the first query, so preview-open cost is unchanged.
- **The preview's wire types have a lint, not a dependency.** Adding a field to `ExportedPerson`
  breaks `engine-wire.test.ts` with the declaration named, in the same `just check` run.
- **A second embedding costs two URLs.** Anything that can serve two files can host the chrome and
  its engine.

## Anti-suggestions (do not re-propose)

- **"Just add `@kullang/wasm` to `packages/preview`'s dependencies — that is what #297 says."** It
  cannot resolve: the local build output is gitignored, so it cannot be a workspace member, and the
  published version predates `queryDetail`. Even once a release closes that gap, the registry
  dependency re-imports the skew this decision removes — the published package is always at least
  one release behind the branch.
- **"Declare `crates/kul-wasm/pkg/` an npm workspace member and skip the registry."** Same objection
  from the other side: `npm ci` on a fresh clone would fail on a directory that does not exist until
  someone runs `wasm-pack`. Installing dependencies must not require a Rust toolchain.
- **"Use the `--target bundler` output and let esbuild inline it into `preview-webview.js`."** That
  is the base64-inlining ADR-0034 already rejected: ~580 KB becomes ~780 KB inside the shell, and it
  is paid at preview-open by everyone, including readers who never query. The `--target web` build
  exists precisely to be fetched.
- **"Generate `engine-wire.ts` from the `.d.ts` in a build step."** A build-order dependency and a
  generated file in review, to replace a test that already fails by name. `kul_wasm.d.ts` is itself
  committed-and-diffed for the same reason (ADR-0012).
- **"Mirror the whole `.d.ts` while we are in there."** The descriptor family is the phrasing
  layer's (ADR-0033). One contract, one mirror.
- **"Have the extension import the engine and proxy queries over `postMessage`."** That reinstates
  the round-trip ADR-0034 removed, and a zero-click hover lens is the affordance that cannot afford
  one.
- **"Ship the engine in the `.vsix` but resolve it from the webview by a relative path."** The
  webview's document origin is a VSCode resource host, not the extension directory; only the host can
  mint a legal resource URI. Handing the URI in is what makes the package host-agnostic rather than
  VSCode-shaped.

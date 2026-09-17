# Research: TanStack Start on Netlify (WASM, client-only vault, deploy shape)

**Ticket:** [#329](https://github.com/YashBhalodi/kul/issues/329)
**Date:** 2026-09-17
**Sources retrieved:** 2026-09-17
**Status:** facts only. Does not pick a hosting mode.

**Gist:** Netlify's official Start adapter deploys SSR, server routes, server functions, and middleware as Netlify Functions and publishes `dist/client`; a client-only IndexedDB/OPFS vault and `kul-wasm` do not force that shape — they only require browser-gated code plus `application/wasm` and CSP `'wasm-unsafe-eval'` — so SSR vs SPA-static remains the leftover for [#338](https://github.com/YashBhalodi/kul/issues/338).

This note answers, from first-party docs and this repo's own ADRs, what deploying a TanStack Start app to Netlify requires when the app is client-heavy, talks to `kul-wasm`, and persists a vault entirely in the origin (no server database). It does not choose among Start's hosting modes.

## Repo facts (given, not rediscovered)

- Published WASM is `wasm-pack --target bundler` as `@kullang/wasm`. CI and release both run `wasm-pack build crates/kul-wasm --target bundler --out-dir pkg --out-name kul_wasm`.[^1][^2]
- Preview uses `wasm-pack --target web` as a build asset, not an npm import ([ADR-0040](../adr/0040-engine-provenance-a-build-asset-not-a-registry-dependency.md)).[^3]
- Gzipped `.wasm` budget is ≤ 1 MB.[^1][^4]
- There is no `netlify.toml` and no TanStack app in this repository today.
- The vault is origin-private (IndexedDB or OPFS — a later ticket). There is no server database on this map.[^5]

## 1. Start's hosting modes

TanStack Start is a full-stack React framework (Release Candidate as of this retrieval) that ships full-document SSR, streaming, server routes, server functions, middleware, and full-stack client/server builds on Vite or Rsbuild.[^6] Code is **isomorphic by default**: it is included in both server and client bundles unless constrained. Route `loader`s run on the server during SSR **and** on the client during navigation.[^7]

The first-party modes that matter here:

| Mode | What Start actually does | First-party switch |
| --- | --- | --- |
| **Default SSR** | Routes matching the initial request run `beforeLoad` / `loader` on the server and render route components to HTML; the client hydrates.[^8] | Default. `ssr` is `true` unless set. Change the default with `defaultSsr` on `createStart`.[^8] |
| **Selective SSR** | Per-route `ssr: true` (default), `ssr: 'data-only'` (server loaders, no server HTML for the component), or `ssr: false` (no server loaders, no server component). Children may only become *more* restrictive. The HTML shell (`shellComponent`) is still SSRed even when the root route component is not.[^8] | Route `ssr` or `createStart({ defaultSsr: false })`.[^8] |
| **SPA mode** | Disables server-side execution of `beforeLoad` / `loader` and server-side rendering of route components. A build prerenders the root route only (matched routes render the pending fallback) to a static `/_shell.html` (configurable). Default rewrites send 404s to that shell. SPA mode does **not** forbid server functions or server routes; it only means the initial document is a client-bootstrapped shell.[^9][^8] | `tanstackStart({ spa: { enabled: true } })`.[^9] |
| **Static prerendering** | Generates static HTML at build time, for performance or for hosts that do not do SSR.[^10] | `tanstackStart({ prerender: { enabled: true, … } })`.[^10] |
| **Server functions** | Same-origin RPC. Handlers run on the server; the client bundle gets a `fetch` stub. They need a runtime server unless marked static.[^11] | `createServerFn()`.[^11] |
| **Static server functions** (experimental) | Executed at prerender time; result cached as a static JSON asset. Later client calls fetch that JSON instead of hitting a server.[^12] | `staticFunctionMiddleware` on `createServerFn`.[^12] |

Start's own comparison: SPA mode "completely disables" server execution of loaders and server rendering of route components; Selective SSR configures that per route, statically or dynamically.[^8] SPA benefits Start names: easier/cheaper CDN deploy, no hydration surface. SPA costs Start names: slower time to full content, weaker SEO unless the crawler executes JS.[^9]

## 2. What Netlify's adapter supports today

Netlify is an official TanStack Start hosting partner.[^13] The current official path (Start ≥ 1.132.0) is the Vite plugin `@netlify/vite-plugin-tanstack-start`, not a `target: 'netlify'` option on `tanstackStart()`.[^13][^14] The plugin's peer range is `@tanstack/react-start` or `@tanstack/solid-start` ≥ 1.132.0 and `vite` ≥ 7.0.0.[^15]

Netlify's own framework guide states, without qualification:

> TanStack Start apps are fully supported on Netlify. SSR, Server Routes, Server Functions, and middleware are all seamlessly deployed to Netlify serverless functions.[^14]

That is the adapter's documented job: configure `vite build` for Netlify, deploy those server surfaces to Functions, and emulate the production Netlify platform inside `vite dev` (so `netlify dev` is not required for local work).[^14][^15] CLI deploys that use the plugin require `netlify-cli` ≥ 17.31.[^14]

What the adapter docs **do not** say:

- They do not list SPA mode, static prerendering, or a "static-only / no Functions" preset as a plugin option.[^14][^15]
- They do not say SPA or prerender are unsupported. Start's own SPA guide uses Netlify `_redirects` as the example host for the shell rewrite, including an allow-list for `/_serverFn/*` and `/api/*` when those server surfaces are kept.[^9]
- They do not require a database, auth provider, or any application environment variable.[^13][^14]

Older Start versions used different wiring (`tanstackStart({ target: 'netlify' })` for 1.121.0–1.131.x; `app.config.ts` + `vinxi build` before 1.121.0) and a different publish directory (`dist` rather than `dist/client`).[^14] Those are historical; a new app should be on ≥ 1.132.0.

## 3. Where a `wasm-pack` artifact lives in a Start + Vite app

`wasm-pack --target` changes the JS glue, not the `.wasm` ISA.[^16]

| Target | What the official docs say | Where it lives in Vite / Start |
| --- | --- | --- |
| **`bundler`** (this repo's published `@kullang/wasm`) | JS suitable for a bundler. `wasm-bindgen` treats the Wasm module as a native ES module — a model no JS engine implements natively today — so a bundler is required. Webpack is the only bundler `wasm-bindgen` names as fully compatible.[^16][^17] | Imported from `node_modules/@kullang/wasm`. Vite's current WASM feature can import a precompiled `.wasm` as an ES module (WASM/ESM integration) or via `?init` / `?url`.[^18] This repo already documents `import { check, exportGraph, format } from '@kullang/wasm'` as working in Vite.[^19] |
| **`web`** (this repo's preview build asset) | Native ESM for the browser. The Wasm must be instantiated and loaded manually. Default export is `init`; `await init()` uses `import.meta` to find the sibling `*_bg.wasm`, or you pass a URL / `Response` / `ArrayBuffer` / `WebAssembly.Module`.[^16][^20] | A pair of static files (glue JS + `.wasm`). In preview they are staged as host URLs and `init` is called with the binary URI ([ADR-0040](../adr/0040-engine-provenance-a-build-asset-not-a-registry-dependency.md)).[^3] In a Start app they would live as Vite static assets (`public/`, or `import wasmUrl from '…wasm?url'`) and be fetched at runtime. |

Two Vite facts constrain placement:

1. **Production emit.** `.wasm` files smaller than `assetsInlineLimit` are inlined as base64; otherwise they are copied to the client asset graph and fetched on demand.[^18] This repo's gzipped budget is 1 MB,[^1] and [ADR-0034](../adr/0034-query-transport-and-result-node-identity.md) already rejected base64-inlining the module into the HTML shell (~580 KB → ~780 KB, paid at open).[^21] Treat the binary as a fetched asset, not an inline string.
2. **SSR load path.** Vite's direct `.wasm` import and `.wasm?init` rely on `node:fs` in the SSR build. Official Vite docs: those features "will only work in Node.js compatible runtimes for SSR builds."[^18] Netlify Functions are a Node-compatible runtime, but a client-only engine must not be imported on the server anyway (see §5). `?url` + `WebAssembly.instantiateStreaming(fetch(wasmUrl))` is the documented way to keep instantiation in the browser.[^18]

`import.meta.env.SSR` is Vite's built-in boolean for "running on the server."[^22] Start's `createClientOnlyFn` / `ClientOnly` / `*.client.*` import protection are the Start-native equivalent gates.[^7]

## 4. CSP, MIME, and `instantiateStreaming` traps

**MIME.** `WebAssembly.instantiateStreaming()` requires the HTTP response `Content-Type` to be `application/wasm`, or it throws.[^23][^24] The rustwasm production guide names this as the deploy-time server requirement if you want streaming compile (or if your bundler uses that function).[^24]

`wasm-bindgen`'s `--target web` `init` tries `instantiateStreaming` when given a URL/`Request`, and on failure checks `Content-Type`; if it is not `application/wasm` it warns and falls back to `arrayBuffer()` + `WebAssembly.instantiate` (slower).[^20][^25] A wrong MIME is therefore a performance/console trap, not always a hard load failure — unless CSP or a consumed body blocks the fallback.

**CSP.** If a page has a CSP with `default-src` or `script-src` and does not allow `'wasm-unsafe-eval'` (or the broader `'unsafe-eval'`), WebAssembly compilation and instantiation are blocked, including `instantiateStreaming`.[^26][^23] `'wasm-unsafe-eval'` permits Wasm compile/instantiate without enabling `eval()`.[^26] Fetching the `.wasm` also needs a `connect-src` (or `default-src`) that allows that URL. [ADR-0034](../adr/0034-query-transport-and-result-node-identity.md) already accepted `'wasm-unsafe-eval'` on `script-src` plus a `connect-src` fetch allowance for the preview webview, and rejected `'unsafe-eval'` as the broader grant.[^21]

**Netlify headers apply only to files Netlify serves from its backing store.** Custom headers are **not** applied to responses from Functions, Edge Functions, or proxied/SSR pages.[^27] A CSP or `Content-Type` rule in `netlify.toml` / `_headers` therefore covers the published client assets (the `.wasm` under `dist/client`) and does **not** cover HTML that a Start SSR Function emitted. For SSR HTML, the Function (or Start itself) must set CSP.

Netlify does not document a default `Content-Type` for `.wasm` in the pages retrieved for this note. The first-party way to *guarantee* `application/wasm` on a static asset is a custom header on the published file:[^27]

```toml
[[headers]]
  for = "/*.wasm"
  [headers.values]
    Content-Type = "application/wasm"
```

`Content-Type` is not in Netlify's list of headers the platform ignores / overwrites.[^27]

## 5. Client-only vault vs Start SSR

IndexedDB is a **client-side** store: you open it via `Window.indexedDB` or `WorkerGlobalScope.indexedDB`. It is same-origin; clearing site data deletes it.[^28]

OPFS is reached with `navigator.storage.getDirectory()`. It is origin-private, quota-limited, deleted when site data is cleared, and available only in a **secure context** (HTTPS). Synchronous access (`createSyncAccessHandle`) is worker-only.[^29]

Neither API exists on Start's server environment (Node, no `window` / `navigator.storage`).[^7] Start's own Selective SSR examples name `localStorage` and `canvas` as the reason to turn SSR off for a route.[^8] IndexedDB and OPFS are the same class of browser API.

This is a **gating** problem, not a hosting-mode veto. Start's documented tools:

| Gate | Server behaviour | Use |
| --- | --- | --- |
| `ClientOnly` | Renders `fallback` on the server; children mount after hydration.[^7][^30] | Vault-backed UI. |
| `useHydrated` | `false` during SSR and on the first client render; `true` after hydration.[^7] | Conditional render of origin data. |
| `createClientOnlyFn` | Throws if called on the server.[^7][^31] | Vault read/write helpers. |
| `*.client.*` or `import '@tanstack/react-start/client-only'` | Import protection denies the file on the server.[^7][^32] | Whole vault module. |
| Route `ssr: false` / `defaultSsr: false` / SPA mode | Loaders and components that touch the vault never run on the server.[^8][^9] | Whole-route or whole-app opt-out. |

Conflicts that **do** exist if the vault is touched isomorphically:

- **`window` / `navigator` in a `loader` or during prerender.** Loaders are isomorphic; the SPA shell is prerendered with the SSR build, and root-route loaders run during that prerender.[^7][^9]
- **Hydration mismatch.** Server HTML that assumes an empty shelf, then a client first paint that lists vault projects, is the mismatch Start's hydration guide warns about (user prefs, `localStorage`, anything client-only). The prescribed fixes are `ClientOnly`, Selective SSR, or making server and client HTML match.[^30]
- **Wasm on the server.** Vite's SSR Wasm path is Node/`fs`-shaped; importing `@kullang/wasm` or a `.wasm?init` module from isomorphic code will either fail off-Node or pull the engine into the server bundle.[^18] Keep engine init behind the same client-only gates as the vault.

A client-only vault therefore **does not forbid** default SSR, Selective SSR, SPA mode, or static prerender. It forbids rendering vault contents (or instantiating Wasm) during SSR / prerender without a fallback. HTTPS is required for OPFS; Netlify sites are HTTPS in production.[^29][^14]

## 6. Concrete Netlify shape

### Adapter and `netlify.toml`

For Start ≥ 1.132.0, both TanStack and Netlify document the same three pieces:[^13][^14]

```ts
// vite.config.ts
import { defineConfig } from 'vite'
import { tanstackStart } from '@tanstack/react-start/plugin/vite'
import netlify from '@netlify/vite-plugin-tanstack-start'
import viteReact from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [tanstackStart(), netlify(), viteReact()],
})
```

```toml
# netlify.toml
[build]
  command = "vite build"
  publish = "dist/client"

[dev]
  command = "vite dev"
  port = 3000
```

`publish = "dist/client"` is the current client-asset directory. Pre-1.132.0 used `publish = "dist"`.[^13][^14] Build settings can instead be set in the Netlify UI; the plugin auto-configures them on first `npx netlify deploy` of a new project.[^13] Generic Vite-on-Netlify detection still suggests publish `dist`; that suggestion is wrong for current Start and must be overridden.[^33]

This repository is a monorepo. If the Start app is not at the repo root, Netlify's `base` (and a `command` that also builds `kul-wasm`) will need to be set; that path is not decided here. There is no Start package and no `netlify.toml` today.

### Redirects for a client router

Two first-party recipes, depending on mode:

**SPA / static shell (Start's recipe).** Static assets that exist are served first. Then optionally allow-list server surfaces. Then rewrite remaining 404s to the shell. Start's Netlify example:[^9]

```
/_serverFn/* /_serverFn/:splat 200
/api/* /api/:splat 200
/* /_shell.html 200
```

The shell path is `/_shell.html` by default, not `/index.html`.[^9]

**Generic Netlify SPA.** Netlify's own SPA rewrite is `/* → /index.html` with status 200, typically last, and it does **not** shadow a URL that already exists as a file (so hashed JS/CSS/Wasm under `dist/client` still win).[^34][^35]

**Full-stack via the adapter.** Netlify documents that SSR / server routes / server functions / middleware deploy to Functions.[^14] The plugin's job is to wire that build. The adapter pages retrieved for this note do **not** ask you to add the SPA `/* → /index.html` rewrite on top; adding a catch-all to a static HTML file would fight Function routing if SSR is on. Start SPA docs, conversely, say the catch-all **is** required when the deploy is a client shell.[^9]

Shadowing: a `/*` rewrite does not replace a real file unless the rule is forced (`200!` / `force = true`).[^34] That is what keeps `*.wasm` / hashed assets reachable under a SPA rewrite.

### Environment variables

No Start-on-Netlify page retrieved for this note names a required env var.[^13][^14] A client-only vault with in-page `kul-wasm` does not need a database URL, an API key, or a public app URL to boot.

Rules that apply *if* something is added later:

- Vite exposes only `VITE_`-prefixed names to client source, statically replaced at build time. They must not hold secrets.[^22]
- `import.meta.env.SSR` / `DEV` / `PROD` / `MODE` / `BASE_URL` are built in.[^22]
- Netlify does not read `.env` files in its build system; values must be imported into Netlify or inlined on the build command.[^36]
- `netlify.toml` `[build.environment]` / `[context.*.environment]` is visible at **build** time only. SSR / Functions cannot read `netlify.toml` at runtime; those values must be set in the Netlify UI, CLI, or API, scoped to **Functions and Builds**.[^37]
- Start itself warns that module-scope `process.env` reads leak into the client bundle and can be `undefined` on edge SSR; secrets go in `createServerOnlyFn` / a server-function handler.[^7]

None of that is required for the vault described in this map.

## 7. What is *not* decided

The facts do not pick a hosting mode. That is [#338](https://github.com/YashBhalodi/kul/issues/338).

All of the following are first-party-supported and compatible with a client-only vault and `kul-wasm`, provided vault/Wasm stay behind the gates in §5:

1. **Default SSR + official Netlify adapter.** Documented Netlify path. Server Functions exist even if this map does not use them. Hydration and isomorphic loaders must not touch `window` / IndexedDB / OPFS / Wasm init.
2. **Selective SSR or `defaultSsr: false`, still on the adapter.** Start's per-route / default-off SSR, with the HTML shell still SSRed. Same Netlify Functions deploy shape; less server HTML for the authoring UI.
3. **SPA mode (`spa.enabled`) on Netlify.** Start-documented. Needs the `/_shell.html` rewrite (and `/_serverFn/*` / `/api/*` only if those surfaces are kept). Can be CDN-static if no server functions are called. Start names this as cheaper and simpler (no hydration), at the cost of time-to-content and SEO.[^9]
4. **Static prerender + optional static server functions.** Start-documented for hosts that do not SSR.[^10][^12] Experimental on the server-function side. The official Netlify adapter's documented mission is still the Functions deploy, not this.

Nothing in the adapter docs *forces* (1). Nothing in the vault or Wasm facts *forces* (3). The map has no SaaS backend and no server database, so a runtime server is not justified by persistence — but Start's default and Netlify's adapter are still the full-stack shape, and SPA mode can still keep server functions if wanted.[^9][^14]

[#338](https://github.com/YashBhalodi/kul/issues/338) decides which of those shapes this app actually ships, plus the adapter/redirect/CSP/Wasm-asset constraints the epic must obey. This note supplies the constraints; it does not make the choice.

Also not decided here (and not #338's whole job): monorepo `base` / which package path holds the Start app; whether the published `--target bundler` package or a `--target web` build asset is what the browser app loads; IndexedDB vs OPFS (a separate ticket).

## Sources

[^1]: [`.github/workflows/rust.yml`](../../.github/workflows/rust.yml) — `wasm-pack build … --target bundler`; gzipped `.wasm` budget `1024 * 1024`.
[^2]: [`.github/workflows/release.yml`](../../.github/workflows/release.yml) — same `--target bundler` publish; `--target web` used only to stage the webview engine.
[^3]: [ADR-0040](../adr/0040-engine-provenance-a-build-asset-not-a-registry-dependency.md) — preview engine is `wasm-pack --target web` into `pkg-web/`, fetched by URL; `@kullang/wasm` stays `--target bundler`.
[^4]: [`docs/testing.md`](../testing.md) — bundle-size budget > 1 MB fails CI.
[^5]: [Wayfinder map #328](https://github.com/YashBhalodi/kul/issues/328) — vault is origin-private; no SaaS / server-side Kul projects on this map.
[^6]: [TanStack Start Overview](https://tanstack.com/start/latest/docs/framework/react/overview) — RC; full-document SSR, streaming, server routes, server functions, full-stack builds.
[^7]: [TanStack Start — Execution Model](https://tanstack.com/start/latest/docs/framework/react/guide/execution-model) — isomorphic default; loaders on both sides; `createClientOnlyFn` / `ClientOnly` / `useHydrated` / `*.client.*`; `process.env` anti-pattern.
[^8]: [TanStack Start — Selective SSR](https://tanstack.com/start/latest/docs/framework/react/guide/selective-ssr) — default `ssr: true`; `false` / `data-only`; inheritance; SPA comparison; `shellComponent` still SSRed.
[^9]: [TanStack Start — SPA mode](https://tanstack.com/start/latest/docs/framework/react/guide/spa-mode) — `spa.enabled`; `/_shell.html`; Netlify `_redirects` including `/_serverFn/*` and `/api/*`; shell prerender uses the SSR build; CDN/cheap vs hydration.
[^10]: [TanStack Start — Static Prerendering](https://tanstack.com/start/latest/docs/framework/react/guide/static-prerendering) — `prerender.enabled` for static HTML / hosts without SSR.
[^11]: [TanStack Start — Server Functions](https://tanstack.com/start/latest/docs/framework/react/guide/server-functions) — same-origin RPC; client stub is `fetch`; needs a server at runtime.
[^12]: [TanStack Start — Static Server Functions](https://tanstack.com/start/latest/docs/framework/react/guide/static-server-functions) — experimental; build-time execution; runtime fetch of static JSON.
[^13]: [TanStack Start — Hosting (Netlify)](https://tanstack.com/start/latest/docs/framework/react/guide/hosting) — official partner; `@netlify/vite-plugin-tanstack-start`; `publish = "dist/client"`; `vite build`.
[^14]: [Netlify — TanStack Start](https://docs.netlify.com/build/frameworks/framework-setup-guides/tanstack-start/) — SSR / server routes / server functions / middleware → Functions; plugin install; `netlify.toml`; CLI ≥ 17.31; pre-1.132.0 variants; local emulation.
[^15]: [`@netlify/vite-plugin-tanstack-start` README](https://www.npmjs.com/package/@netlify/vite-plugin-tanstack-start) — build + `vite dev` emulation; peerDeps `>=1.132.0` / `vite >=7`.
[^16]: [wasm-pack — `build --target`](https://rustwasm.github.io/docs/wasm-pack/commands/build.html) — `bundler` vs `web` vs `nodejs`.
[^17]: [wasm-bindgen — Deployment](https://rustwasm.github.io/docs/wasm-bindgen/reference/deployment.html) — bundler target assumes Wasm-as-ESM; bundler required; webpack named as fully compatible; `--target web` is native ESM + manual init.
[^18]: [Vite — Features / WebAssembly](https://vitejs.dev/guide/features.html#webassembly) — ESM integration, `?init`, `?url` + `instantiateStreaming`; `assetsInlineLimit`; SSR uses `node:fs` / Node-only.
[^19]: [`README.md`](../../README.md) — `@kullang/wasm` documented as working in Vite (and Webpack 5+, Next, SvelteKit, Nuxt, Astro).
[^20]: [wasm-bindgen — Without a Bundler](https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html) — `import init, { add } from './pkg/….js'`; `await init()` or `init(url|Response|…)`.
[^21]: [ADR-0034](../adr/0034-query-transport-and-result-node-identity.md) — lazy fetch; `'wasm-unsafe-eval'` + `connect-src`; reject `'unsafe-eval'` and base64 inlining.
[^22]: [Vite — Env Variables and Modes](https://vitejs.dev/guide/env-and-mode) — `VITE_` client exposure; `import.meta.env.SSR`.
[^23]: [MDN — `WebAssembly.instantiateStreaming()`](https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/JavaScript_interface/instantiateStreaming_static) — streaming compile; requires `application/wasm`; CSP may block (see `script-src`).
[^24]: [rustwasm book — Deploying to Production](https://rustwasm.github.io/docs/book/reference/deploying-to-production.html) — server must serve `application/wasm` or `instantiateStreaming` throws.
[^25]: [wasm-bindgen PR #1996](https://github.com/rustwasm/wasm-bindgen/pull/1996) — generated `init` fallback when `Content-Type != application/wasm`.
[^26]: [MDN — CSP `script-src`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/script-src) — `'wasm-unsafe-eval'` required when a `script-src`/`default-src` CSP is present; narrower than `'unsafe-eval'`.
[^27]: [Netlify — Custom headers](https://docs.netlify.com/manage/routing/headers/) — headers apply only to backing-store files, not Function/SSR/proxy responses; `[[headers]]` syntax.
[^28]: [MDN — IndexedDB API](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API) — client-side; `Window.indexedDB`; same-origin.
[^29]: [MDN — Origin private file system](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system) — `navigator.storage.getDirectory()`; secure context; quota; cleared with site data.
[^30]: [TanStack Start — Hydration Errors](https://tanstack.com/start/latest/docs/framework/react/guide/hydration-errors) — client prefs / `Intl` / `Date` mismatches; `ClientOnly`; Selective SSR.
[^31]: [TanStack Start — Environment Functions](https://tanstack.com/start/latest/docs/framework/react/guide/environment-functions) — `createClientOnlyFn` throws on the server.
[^32]: [TanStack Start — Import Protection](https://tanstack.com/start/latest/docs/framework/react/guide/import-protection) — `*.client.*` denied on the server; `ClientOnly` / `createClientOnlyFn` suggestions.
[^33]: [Netlify — Vite](https://docs.netlify.com/build/frameworks/framework-setup-guides/vite/) — generic Vite detection suggests publish `dist`.
[^34]: [Netlify — Rewrites and proxies](https://docs.netlify.com/manage/routing/redirects/rewrites-proxies/) — SPA `/* → /index.html` 200; shadowing; force.
[^35]: [Netlify — JavaScript SPAs](https://docs.netlify.com/build/configure-builds/javascript-spas/) — history `pushState` requires a rewrite to the HTML shell.
[^36]: [Netlify — Build environment variables](https://docs.netlify.com/build/configure-builds/environment-variables/) — `.env` not read by Netlify's build system; Builds scope.
[^37]: [Netlify — Environment variables with frameworks](https://docs.netlify.com/build/frameworks/use-environment-variables-with-frameworks/) — `netlify.toml` is build-time only; SSR/Functions need UI/CLI/API vars scoped Functions + Builds; framework prefixes for client embeds.

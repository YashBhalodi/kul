# ADR 0047 — The browser app is Kul's only interactive surface

**Status:** Accepted
**Date:** 2026-11-02
**Deciders:** owner

Supersedes [ADR-0036](./0036-two-tier-theming-and-the-accessibility-non-goal.md) (its accessibility non-goal) and [ADR-0040](./0040-engine-provenance-a-build-asset-not-a-registry-dependency.md). Amends [ADR-0016](./0016-visualization-pipeline-crate-boundaries.md) (where the chrome lives) and [ADR-0024](./0024-query-seam-and-envelope.md), [ADR-0025](./0025-kinship-query-engine-contract-and-traversal.md) and [ADR-0028](./0028-relationship-resolution-and-honest-emptiness.md) (the query engine is reached through the CLI only). Superseded query-UI records: [ADR-0033](./0033-phrasing-layer-architecture.md), [ADR-0035](./0035-detail-surface-one-selection-one-batched-lookup.md), [ADR-0039](./0039-phrasing-lookup-mechanics.md), [ADR-0041](./0041-the-gu-pack-and-the-locale-toggle.md), [ADR-0042](./0042-the-selection-seam-and-occlusion-aware-centring.md), [ADR-0043](./0043-explore-kin-thirteen-query-values-and-what-a-row-costs.md), [ADR-0044](./0044-the-hover-lens-transliteration-as-data-and-the-unspent-hue.md) and [ADR-0045](./0045-the-filter-bars-three-states-derived-reasons-and-one-class-per-painter.md): the preview UI they governed was removed with the preview package. Epic [#343](https://github.com/YashBhalodi/kul/issues/343).

## Context

Kul had three interactive surfaces: an editor extension with a language server binary behind it, a preview package that drew the tree inside the extension, and the `kul` CLI. Each surface carried its own host plumbing — a message protocol between the extension and its preview, a stdio protocol between the extension and the server, a published npm package for the engine, two marketplace listings and their secrets. Every language feature had to be written once and then carried across two protocols before a reader saw it.

The audience Kul wants — people writing down their own family — does not install an editor extension. A page they open is the shortest path from "I want to write down my family" to a drawn tree, and the engine already compiles to WebAssembly, so the browser can run every feature with no server at all.

## Decision

### One web app at kullang.dev; the CLI stays for agents and CI

The web app in `apps/web/` is the only interactive surface. It is three things on one origin: a landing page (`/`), the docs ("Learn Kul", `/docs/`, plus `llms.txt` and `llms-full.txt`), and the app (`/app/`): a floating, minimizable code editor over a canvas that draws the tree live.

The editor extension, the language server crate and binary, the preview package, the engine's npm package and both marketplace listings are deleted. The `kul` CLI (`validate`, `format`, `export`, `query`) stays: AI agents and CI drive Kul from a shell, and it is the only place the query engine and the JSON export are reachable. Tags release CLI binaries on GitHub Releases and nothing else.

### The stack

- **TanStack Start on Vite 8**, prerendering every page that has content (landing, docs, the Markdown and text variants, the search index) to static files, plus one SPA shell for the app. `/app` and `/app/$projectId` are client-only routes (`ssr: false`).
- **shadcn/ui** (CLI v4, Base UI primitives, Tailwind v4) for components, themed only from `apps/web/src/styles/tokens.css`. No colour literal exists anywhere else in `src/`; a lint (`scripts/lint-colors.mjs`) enforces it.
- **Fumadocs, in part.** Fumadocs MDX compiles the docs, Fumadocs core builds the source, the static search index and the llms files, and Fumadocs UI's search dialog runs search in the browser. Fumadocs UI's *layout* is not used: its sidebar, table of contents and popover stack added about 60 KB gzipped and twenty chunks and put `/docs/` below the Lighthouse floor (0.92–0.94 against 0.95). The docs layout is the site's own, and the docs share the site header and footer.
- TanStack Router, Store, Hotkeys and Pacer; no TanStack DB, Query or Form. ESLint and Prettier (the Start template's), not Biome.

### Spike outcomes that shaped the build

- **The SPA shell's mask path is `/app`, not `/`.** The prerender dedupes jobs by path, so with the default `/` the shell and the landing page collide and the landing page is never prerendered. The shell renders from the client-only route's pending component and is written to `/_shell.html`; `_redirects` sends `/app/*` to it with a 200.
- **Non-HTML files are server routes**, listed for the prerender: `llms.txt`, `llms-full.txt`, `docs/search.json` and `docs/<slug>.md`. Start writes a non-HTML response at the page path itself.
- **`_headers` is generated after the build** (`scripts/finalize-dist.mjs`), not kept static. Start emits three inline scripts per page, different per page, so each HTML file's block carries the CSP with the SHA-256 of exactly its inline scripts. Blocks never overlap, because a header named in two matching blocks is joined.
- **The engine is a workspace package** (`kul-wasm`, built by `just wasm` into `crates/kul-wasm/pkg`) imported by the app's language worker; Vite hashes the `.wasm` as an asset. It is not published anywhere.
- **The theme's pre-paint script is an external, hashed, render-blocking asset**, not inline, so every page's CSP stays free of `'unsafe-inline'` scripts. The theme control is a three-button switch rather than a menu: a menu cost 44 KB gzipped on the landing page, and the switch's pressed state comes from CSS before hydration.
- The prerender's preview server is pinned to `127.0.0.1`: on Linux it otherwise bound `::1` while the crawler fetched `127.0.0.1`.
- React and the router ship as one `framework` chunk. Lighthouse's simulated load is dominated by the number of requests started before the LCP; the landing page went from 28 files to 6 and from 0.94 to 0.98.

### Desktop only, for the app

`/app` needs a fine pointer and at least 1024 px; smaller screens get a notice. The landing page and the docs are fully responsive.

### Accessibility is a gate

The app's components are keyboard-reachable with visible focus, and every state of the screenshot tour (`apps/web/e2e/tour.spec.ts`) runs axe; a serious or critical violation fails it. This replaces ADR-0036's accessibility non-goal, which was written for a preview inside a host that owned focus and theming.

### Hosting: Netlify, static only, deployed by GitHub Actions

- The site is the static `apps/web/dist/client` and nothing else. Netlify serves it and never builds it: the deploy job runs `netlify deploy --no-build --filter kul-web` on the artifact of exactly the build the gates tested.
- `web.yml` deploys production on a push to `main` and a preview (`--alias pr-<n>`) per pull request, then runs `scripts/smoke-deployed.sh` against the deploy's own URL (the alias for previews, the deploy permalink for production). The production host redirects to kullang.dev, so smoke-checking it would test DNS rather than the deploy. The deploy job is the only one with a write permission (`pull-requests: write`, for the preview comment). Without the two Netlify secrets it prints a notice and skips.
- The site is `kullang-app` on Netlify (the name `kullang` is taken); the domain is **kullang.dev**. `_redirects` sends `www.kullang.dev` and the Netlify host to kullang.dev with a forced 301. Only https rules exist: `.dev` and `netlify.app` are HSTS-preloaded.
- `scripts/serve-dist.mjs` reproduces Netlify's `_redirects` and `_headers` rules for local and CI tests. Where a real deploy disagreed with it (a host rule's splat keeps the trailing slash, `/x/index` redirects to `/x/`, directory redirects carry no `_headers`), serve-dist follows Netlify. `scripts/deploy-rules.mjs` checks both files on every build.

The setup steps and the rules are in [`docs/deploy.md`](../deploy.md).

### The records of the retired surfaces

A search for the retired surfaces' names must come back empty; it stays as `scripts/retired-names.sh`, run by `just verify` and CI, so the names don't creep back. ADRs are records, so the search skips the whole of any ADR whose status line says *Superseded*, and that ADR's file name where another document links to it: rewriting a superseded record would falsify history, and a link to one is how a newer record says what it replaced. Nothing else is exempt, so ADRs still in force that named a retired surface were edited: the name became a description ("the language server", "the editor preview"), the decision text was kept, and each such record says in its status line which new ADR amends it. Living docs link the record in force (ADR-0048 for a project's files, not the project-file ADR it replaced).

## Consequences

- A language feature is written once in Rust and reaches the reader through one adapter (ADR-0049) rather than two protocols.
- The CLI is the only home of the query engine and the JSON export. The app has neither a query UI nor a JSON export; both can be added later without changing the engine.
- The JS toolchain is part of `just check`: Node and `wasm-pack` are prerequisites, and the web gates (typecheck, lint, Vitest in Node and browsers, the production build and its checks) run beside the Rust ones.
- The site's correctness lives in build-time checks (`check-dist`, `check-links`, `check-landing`, `deploy-rules`) because a static host has no runtime to catch mistakes.

## Anti-suggestions (do not re-propose)

- **"Bring back an editor extension as a second surface."** Every feature would again need a protocol adapter and a host. The CLI covers non-browser use.
- **"Use Fumadocs UI's full docs layout."** Measured: it costs the Lighthouse floor. Use its search dialog and core, not its layout.
- **"Keep `_headers` static."** The inline-script hashes differ per page; a static file means `'unsafe-inline'`.
- **"Let Netlify build the site."** The deploy must be the artifact the gates tested.

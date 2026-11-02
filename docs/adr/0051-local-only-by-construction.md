# ADR 0051 — Local-only by construction: OPFS, one writer per project, no egress, offline

**Status:** Accepted
**Date:** 2026-11-02
**Deciders:** owner

Issues [#349](https://github.com/YashBhalodi/kul/issues/349), [#352](https://github.com/YashBhalodi/kul/issues/352).

## Context

A family record is personal. The app has no account, no server and no upload: everything a reader writes stays in their browser. That has to be a property of how the app is built — something a test can prove — not a promise on the landing page (which deliberately does not make it). The same construction gives offline use: if nothing needs the network, nothing should break without it.

## Decision

### The project store: OPFS, one interface, two backends

`apps/web/src/store/` is the only code that touches browser storage. An ESLint rule (`no-restricted-globals` / `no-restricted-properties` on `localStorage`, `sessionStorage`, `indexedDB` and `getDirectory`) forbids it everywhere else in `src/` except `src/prefs/` (theme, reminders).

- Each project is a folder in the origin private file system: `projects/<uuid>/project.json` (`{ schema: 1, name, createdAt, updatedAt, ui }`, where `ui` is the window state: open tabs, active file, editor window layout, camera) and `files/<name>.kul`. `project.json` is written last, so a folder without one is ignored. The project list is the folder listing.
- One interface (`createProjectStore(backend)`) over a six-method `StorageBackend`, with two real backends: OPFS (`createWritable`) and memory. `openProjectStore()` probes OPFS by writing, reading and removing a file, and falls back to memory with a "can't save files" banner. Browsers without `createWritable` (Safari before 26) take the fallback rather than a worker-based writer. One contract suite runs against both.
- Operations on a project run in call order through a per-project queue. File names follow the project rules: `.kul`, no `/` or `\`, no control characters, at most 128 code points, unique ignoring case after NFC. `readFiles` returns file order (ADR-0048).
- Window-state writes are not edits: they bump no `updatedAt`, request no persistence and announce nothing to other tabs.
- Import takes files, folders and `.zip` (flattened, `.kul` only, one choice for every name clash). Export is a project `.zip` (fixed timestamps, so the same files give the same bytes), one file, or the tree as SVG. There is no "open folder" on disk.

### One writer per project: tab ownership

A project has at most one **owning tab**: the one holding the Web Lock `kul-project-<id>` (`ifAvailable`, then `steal` on request). Every other tab showing the project is **read-only**: it follows the owner's writes through a `BroadcastChannel` of the same name and offers "Use here instead", which steals the lock.

- A tab that loses the lock **discards** its unsaved edits, because the new owner has already read the files. Edits flush on `visibilitychange: hidden`, so switching tabs saves first.
- A released lock is not re-acquired automatically; a read-only tab stays read-only until the reader asks.

### No request leaves the origin

- Every HTML page and the service worker are served with a CSP whose `connect-src` is `'self'` and whose `default-src` is `'self'`; `script-src` adds only `'wasm-unsafe-eval'` and per-page hashes. Fonts are self-hosted. There is no analytics, telemetry or CDN.
- **Proof:** `apps/web/e2e/no-egress.spec.ts` drives a whole session (the landing page, docs pages and their text files, then the app: first run, an edit, an import, both downloads and a reload) and records every request and WebSocket; any request not to the origin fails it (`data:` and `blob:` count as the page's own bytes). Firefox and WebKit do not report the service worker's own fetches to Playwright, so the CSP on `/sw.js` is the guard there; the test asserts that pages, Markdown, `llms.txt`, scripts and fonts were all recorded.
- `scripts/deploy-rules.mjs` fails the build if any HTML page or `/sw.js` lacks the CSP, or if `connect-src` is anything but `'self'`; `scripts/smoke-deployed.sh` checks the deployed headers.

### Offline: a hand-written service worker over Workbox modules

- `src/sw/sw.ts` is bundled by Vite's `build()` in a post-build script (`scripts/build-sw.mjs`) and given its precache manifest by `workbox-build`'s `injectManifest`; there is no PWA plugin. The precache is **every file the build wrote** (glob `**/*`), not a list; `check-dist` holds the manifest equal to the served file set in both directions. Any Workbox warning fails the build.
- Content-hashed `/assets/*` are served precache-first. Everything else on the origin is network-first with a 3-second fallback to the precached copy; every `/app` path falls back to `/_shell.html`. Online readers get new pages at once.
- No `clientsClaim` and no automatic `skipWaiting`: a first visit stays uncontrolled, and a new build waits. The page shows "A new version of Kul is ready" with **Reload** only when there is a controller *and* a waiting worker (workbox-window reports `waiting` for a slow first install too). Reload first writes the open project's pending edits, then activates the new worker.
- Installability (a web manifest) is out of scope; adding one later is additive (`manifest-src 'self'` is already in the CSP).

## Consequences

- Clearing site data deletes the reader's projects. The app asks for persistent storage after the first edit, shows when the browser refuses, and reminds the reader to download a backup.
- Playwright's WebKit has no OPFS in an ephemeral context, so the WebKit tests run in persistent contexts (`e2e/fixtures.ts`, the Vitest WebKit instance).
- The memory backend is a product path (no storage), not only a test double.

## Anti-suggestions (do not re-propose)

- **"Sync projects to a server" or "add analytics."** Either breaks the CSP and the no-egress test by design.
- **"Store projects in IndexedDB or localStorage."** OPFS keeps a project as plain files, which is what import and export speak.
- **"Merge concurrent edits from two tabs."** One writer is simpler and correct; the other tab is told and can take over.
- **"Activate new service workers automatically."** A reload under the reader's feet can lose an edit in flight.

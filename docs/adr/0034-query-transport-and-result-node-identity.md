# ADR 0034 — Query transport: the engine runs in the webview, and results bind to cards by person id

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

The preview needs to reach all four engine capabilities — kin-set queries, two-anchor resolution, attribute filtering, and detail lookups — and bind their answers back to the SVG the user is looking at. [ADR-0024](./0024-query-seam-and-envelope.md) pinned the `QueryEnvelope` and the two detail lookups; [ADR-0025](./0025-kinship-query-engine-contract-and-traversal.md) pinned one `Query` value with one evaluation path; [ADR-0029](./0029-query-engine-performance-posture.md) pinned on-demand evaluation with no caches and an interactive target of `< ~50 ms` per query; [ADR-0033](./0033-phrasing-layer-architecture.md) put phrasing in the webview and therefore requires full descriptors, backbone included, to reach it. None of them said how a query gets from a webview to the engine.

This ADR decides that. It resolves [#280](https://github.com/YashBhalodi/kul/issues/280) on the [query-UX wayfinder map (#275)](https://github.com/YashBhalodi/kul/issues/275).

Six facts were established before deciding, because each one moved an answer:

1. **The LSP already has five custom methods** — `kul/export`, `kul/render`, `kul/exportSvg`, `kul/locate`, `kul/entityAt`. A `kul/query` would be the sixth, and the extension drives all of them the same way.
2. **`@kullang/wasm` already exposes the whole query surface** — `queryPerson`, `queryMarriage`, `queryKin`, `runQuery`, `queryResolve`. Running queries in the webview needs no new adapter work.
3. **Every WASM query operation is stateless**: it takes `files + manifest` and calls `check_with_manifest` before querying. There is no persistent document handle, so each call re-checks the project.
4. **`check` alone on the 10,000-person synthetic corpus is ≈12.4 ms** (release, best of five, measured with a temporary probe against `crates/kul-core/tests/perf.rs`'s `generate_corpus`). Against ADR-0029's measured 1–3 ms for kin-set/filter/lookup and ≈16 ms for `resolve`, the re-check is the *dominant* per-call cost but not a disqualifying one.
5. **The rendered SVG already carries the detail payload.** Per [ADR-0021](./0021-language-properties-plumb-to-svg.md)'s exhaustive plumb-through, a card carries every `ExportedPerson` field except `span`, and the marriage/birth/adoption edges carry every `ExportedMarriage` field plus the children-with-link-kinds list.
6. **A person is not one card.** [ADR-0019](./0019-ghost-model-and-bio-anchor.md) emits a ghost per past intimacy, so one person can hold a canonical card plus several ghosts — and ghost cards carry `data-person-id`, `data-kind` and `data-ghost-reason` but **no marriage id**, so two past-adoption ghosts of the same person are indistinguishable in the DOM.

## Decision

### The engine runs in the webview; no `kul/query` LSP request is added

The preview calls `@kullang/wasm` directly. The answer to the ticket's title question is **no** — the LSP gains no sixth custom method, and query traffic never crosses the webview→extension→LSP boundary.

The decisive property is that a query becomes a local function call: the hover lens, which is a zero-click affordance firing on pointer movement, answers without a round-trip. Fact 2 means this costs no adapter work — the surface it needs already ships.

The trade accepted in exchange is fact 3: **every call re-checks the project.** At the ~10k ceiling that is ≈12.4 ms of check on top of 1–16 ms of query, which sits inside ADR-0029's no-spinner target; at the sizes real projects reach it is negligible. A stateful WASM document handle would remove the re-check entirely and was considered — it is rejected here as out of scope, not as wrong, because it adds a statefulness axis to a surface [ADR-0011](./0011-wasm-surface-three-shapes-no-wrappers.md) deliberately kept to stateless shapes. It is the obvious first move if the epic's measurements come back badly.

### The project source reaches the webview with the render

The webview receives only `svg` today. Because the WASM surface is stateless, the extension must also post the project's `files` and `manifest` — carried with each render, so the source the webview queries is always the source the picture came from.

The ~580 KB WASM module loads **lazily on first query**, not at preview open, so opening a preview stays exactly as fast as it is now for anyone who never queries.

### Two engines describe the same file, knowingly

The LSP renders the SVG; the bundled WASM answers the queries. Both are thin adapters over one `kul-core` and ship inside one extension artifact, so in normal distribution they move together.

**This risk is accepted deliberately, and the failure mode is recorded rather than mitigated**: the extension can fetch LSP server binaries separately (`editor/vscode/scripts/fetch-server-binaries.mjs`), so a skewed pair is possible, and its symptom would be silently wrong answers about a family rather than a crash. `KUL_CORE_VERSION` is already exported on the WASM surface, so a startup handshake that disables querying on mismatch is one LSP-side field away. It is not built now; if skew is ever observed, this is the fix, and it does not need a new decision.

### Detail is engine-backed per entity; the kin list hydrates eagerly

> **Superseded by [ADR-0035](./0035-detail-surface-one-selection-one-batched-lookup.md) (#281)** — on #292's measurements, the per-entity shape is replaced by the **batched detail operation** named as the first out below. Engine-backed detail is unchanged; only its granularity is. The eager kin list survives, served by the same batched call.

Person and marriage details come from `queryPerson` / `queryMarriage` — one call per entity, on demand — so the detail surface flows through ADR-0024's pinned envelope and its single-sourced export shapes. A field added to `ExportedPerson` reaches the panel without anyone touching the SVG attribute vocabulary. The kin list fetches **every row on open**, so the panel opens fully populated with no per-row loading states.

**This is the most expensive combination available here, and the cost is recorded rather than discovered.** Per-entity lookups against a stateless surface means an N-row list is N re-checks — roughly a quarter-second at the 10k ceiling for a 20-row list, native, before any WASM multiplier, and paid on every selection. Two cheap outs exist and neither needs a new decision, only a measurement:

- **A batched detail operation** on the WASM surface (one call, many ids), paying the check once. This is new engine-adapter work and a shape ADR-0024 did not pin.
- **List labels off the cards.** Fact 5 means every name in the list is already in the DOM; the details widget could stay engine-backed while the list labels itself for free. The cost is two provenance paths for person data.

The epic ([#282](https://github.com/YashBhalodi/kul/issues/282)) weighs these against its measured numbers. Reading details off the SVG entirely was considered and rejected: it would make the chrome depend on the attribute vocabulary rather than on the pinned contract, and `span` is the one export field the cards do not carry.

### Results bind by `data-person-id`, and paint every card for that person

A query result is about a **person**, not about a card, so `[data-person-id="…"]` selects the canonical card and every ghost, and they paint together. This is what [#276](https://github.com/YashBhalodi/kul/issues/276) already decided when it settled that ghosts light with results, it needs no render-pipeline change, and it keeps a result visible in the past family whose edges the ghost exists to anchor. [#277](https://github.com/YashBhalodi/kul/issues/277)'s dimming makes multi-card lighting read as emphasis rather than noise.

Edges bind the same way, from data the descriptor already carries: an `across` hop names its marriage, so a resolved path selects `[data-marriage-id="…"]`, and vertical hops select birth/adoption edges by `data-child-id`.

**Ghost cards stay individually unaddressable.** `kul-layout` already keys child-ghosts by `(person_id, marriage_id)`, so surfacing a per-card identity is cheap — but no decided interaction needs it, and adding it is a `kul-svg` change and a Rust release. It is a follow-up if a future interaction (lighting only the ghost a resolved path routes *through*) earns it, not a speculative addition.

### Latency posture: two numbers the epic must measure, not assume

Single operations sit inside ADR-0029's no-spinner target on the arithmetic above. Two numbers are genuinely unknown and must be measured before the epic commits to no progress affordance:

1. **The WASM-over-native multiplier** on `check` + query. For parse- and allocation-heavy work 1.5–2× is typical, which would put a single query at the 10k ceiling near the top of the 50 ms budget.
2. **The eager kin-list fetch** at a realistic kin-set size.

> **Measured ([#292](https://github.com/YashBhalodi/kul/issues/292), 2026-07-26)** — see [`docs/query-path-measurements.md`](../query-path-measurements.md). The multiplier is **1.0–1.14× at the ceiling**, not 1.5–2×, so every single operation stays inside the budget (worst: `queryResolve` at 32 ms). The eager kin-list fetch does **not** survive: cost is `N × 1.37 µs × persons`, so any list of 4+ rows misses the budget at 10k persons. The fix is one of the two outs recorded below, and needs no new decision.

Queries are gated on a clean project — [ADR-0009](./0009-export-strict-on-diagnostics.md) strict-on-diagnostics means a failing check yields the envelope's error arm, never a partial answer — and the preview's existing error popover is where that surfaces.

### The webview CSP must be relaxed

Running WASM under the preview's `default-src 'none'; style-src …; script-src 'nonce-…' …` requires **`'wasm-unsafe-eval'` in `script-src`**, plus a `connect-src` allowance to fetch the module. This is a real loosening of a deliberately tight posture, accepted as the cost of the transport decision. Inlining the module as base64 avoids `connect-src` but inflates ~580 KB to ~780 KB inside the HTML shell, and is worse.

## Consequences

- **`kul-lsp` is untouched by this epic's query work.** Its five custom methods stay as they are; the preview's query path does not widen the LSP contract.
- **The preview package gains a runtime dependency on `@kullang/wasm`** and a lazy module loader, and the extension gains a wire message carrying `files + manifest`. `@kullang/preview` is host-agnostic chrome today; the WASM dependency is the first thing in it that is not.
- **Phrasing gets what ADR-0033 needs for free.** Descriptors never cross a wire, so the backbone constraint that ADR decision placed on this one is satisfied by construction.
- ~~**A re-render no longer destroys query state.**~~ **Reversed by [ADR-0035](./0035-detail-surface-one-selection-one-batched-lookup.md) (#281):** a render means a document change, and an edit now deliberately clears selection, panel, kin paint and filter alike — querying and authoring are separate modes. The transport property still holds (query state does live in the webview); the epic simply chose not to preserve it across edits.
- **Two measurements are owed to [#282](https://github.com/YashBhalodi/kul/issues/282)** before it can scope loading affordances: the WASM multiplier, and the eager kin-list fetch. The map's **Not yet specified** entry on latency affordances can now be judged, since the transport is known.
- **The per-call re-check is the epic's main latency lever.** If it bites, the ordered fixes are: batch the detail lookups, take list labels off the cards, then add a stateful WASM handle. None requires revisiting this ADR.

## Anti-suggestions (do not re-propose)

- **"Add a `kul/query` LSP request so queries go through the server like everything else."** Considered and rejected: it puts two IPC hops in front of a zero-click hover affordance to reach an engine the webview can already call, and the shipped WASM surface makes the local path free of adapter work. The LSP path's real advantage — a cached checked project, so no per-call re-check — is recoverable by a stateful WASM handle if it is ever needed.
- **"Run queries in the webview *and* keep an LSP fallback."** Two evaluation paths is exactly what ADR-0025 refuses, and the fallback would be the one nobody tests.
- **"Reach the engine over the exported graph the preview could already hold."** ADR-0024 pins the engine's substrate as `ResolvedDocument`, never `ExportedGraph`. The export is the engine's output-side sibling, not its input.
- **"Read person and marriage details off the SVG's `data-*` attributes instead of calling the engine."** Tempting — ADR-0021's plumb-through is exhaustive and the data is on screen — but it makes the detail surface depend on the attribute vocabulary rather than on ADR-0024's pinned contract and single-sourced export shapes. It stays available as a *labels-only* optimisation for the kin list if measurement demands it.
- **"Give ghost cards a per-card id now, while we're in here."** The layout adapter already has `(person_id, marriage_id)`, so it is cheap — but no decided interaction distinguishes one ghost from another, and a result is about a person. Add it when an interaction earns it.
- **"Paint only the canonical card so the highlight count matches the result count."** #276 settled that ghosts light with results; a result would otherwise read as absent from the past family whose edges its ghost anchors.
- **"Loosen the CSP to `script-src 'unsafe-eval'` — it's simpler than enumerating."** `'wasm-unsafe-eval'` is the narrow grant that exists for exactly this; the broad one re-enables `eval` on a surface that renders untrusted document content.
- **"Assume the WASM build performs like the native measurements in this ADR."** The 12.4 ms is native release. ~~The multiplier is unmeasured and is one of the two numbers the epic owes itself.~~ Measured in #292: it is 1.0–1.14× at the ceiling, so the assumption turns out to be nearly true — but it was worth checking, because the same exercise is what caught the eager-hydration failure.

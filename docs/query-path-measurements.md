# Query-path measurements — the shipped WASM build

Measurement asset for issue [#292](https://github.com/YashBhalodi/kul/issues/292), part of the
[query-UX wayfinder map (#275)](https://github.com/YashBhalodi/kul/issues/275). It closes the two
numbers [ADR-0034](./adr/0034-query-transport-and-result-node-identity.md) declared **must be
measured, not assumed**: the WASM-over-native multiplier, and the cost of the eager kin-list
hydration. It is a scoping input to the epic ([#282](https://github.com/YashBhalodi/kul/issues/282)).

**Lifecycle**: transient. When the query UX ships, whatever survives here as a standing constraint
becomes a perf gate in `crates/kul-core/tests/perf.rs` (budgets are tests, not benchmarks —
[`testing.md`](./testing.md)) and this file is deleted.

## Method

The corpora are `perf.rs`'s deterministic synthetic dynasty — breadth-first, fully populated
generations, a ~12-generation spine, with the polygamy / adoption-into-relatives /
divorce-and-remarriage hazards and a second disconnected component — parameterised by target
person count instead of fixed at ~10k. One `.kul` file per size, 18 KB to 725 KB of source.

Native baselines and WASM measurements run the **same operations over the same bytes**. Every
`@kullang/wasm` query op is stateless (it takes `files + manifest` and calls `check_with_manifest`
before querying), so each native baseline is likewise a full `check` + query — not a query against
a pre-checked document. The comparison is per-call cost as the webview would pay it.

- **Native**: `cargo nextest run -p kul-core --release`, best of 7 after a warm-up call.
- **WASM**: `wasm-pack build crates/kul-wasm --target nodejs` (593,212-byte module; the shipped
  `--target bundler` build is 593,493 bytes — the difference is JS glue, not codegen), driven from
  Node v22.18.0, best of 7 after a warm-up call. Hydration is best of 3.
- **Machine**: Apple M3, macOS 14.5, rustc 1.95.0, release profile (`lto = "thin"`,
  `codegen-units = 1`).

## Finding 1 — the WASM multiplier is 1.0–1.3×, and it *shrinks* as the corpus grows

ADR-0034 assumed 1.5–2× ("typical for parse- and allocation-heavy work") and warned that this would
put a single query at the ceiling near the top of [ADR-0029](./adr/0029-query-engine-performance-posture.md)'s
50 ms interactive budget. The assumption was pessimistic.

| persons | op | native (ms) | WASM (ms) | × |
| --- | --- | ---: | ---: | ---: |
| 266 | `check` | 0.47 | 0.99 | 2.08 |
| 266 | `queryPerson` | 0.44 | 0.55 | 1.25 |
| 266 | `queryResolve` | 3.18 | 3.47 | 1.09 |
| 266 | `runQuery` (filter) | 0.33 | 0.36 | 1.09 |
| 266 | `queryKin` | 0.39 | 0.50 | 1.26 |
| 1 000 | `check` | 1.12 | 1.25 | 1.12 |
| 1 000 | `queryPerson` | 1.09 | 1.29 | 1.19 |
| 1 000 | `queryResolve` | 5.52 | 7.18 | 1.30 |
| 1 000 | `runQuery` (filter) | 1.15 | 1.50 | 1.30 |
| 1 000 | `queryKin` | 1.27 | 1.63 | 1.29 |
| 2 500 | `check` | 2.87 | 3.33 | 1.16 |
| 2 500 | `queryPerson` | 2.83 | 3.32 | 1.17 |
| 2 500 | `queryResolve` | 9.22 | 11.65 | 1.26 |
| 2 500 | `runQuery` (filter) | 3.14 | 3.98 | 1.27 |
| 2 500 | `queryKin` | 3.34 | 4.02 | 1.20 |
| 5 000 | `check` | 6.50 | 6.76 | 1.04 |
| 5 000 | `queryPerson` | 6.42 | 6.76 | 1.05 |
| 5 000 | `queryResolve` | 15.99 | 19.19 | 1.20 |
| 5 000 | `runQuery` (filter) | 6.95 | 7.67 | 1.10 |
| 5 000 | `queryKin` | 7.65 | 8.18 | 1.07 |
| **10 000** | `check` | 13.49 | **13.72** | **1.02** |
| **10 000** | `queryPerson` | 13.69 | **13.70** | **1.00** |
| **10 000** | `queryResolve` | 28.30 | **32.37** | **1.14** |
| **10 000** | `runQuery` (filter) | 14.40 | **14.67** | **1.02** |
| **10 000** | `queryKin` | 15.90 | **16.64** | **1.05** |

The multiplier is largest on the *smallest* corpus, where the fixed JS↔WASM bridge cost (serialising
the input files in, the result out) is a visible fraction of a sub-millisecond call. It is
irrelevant there in absolute terms. Where the numbers matter — at the ceiling — the WASM path costs
between nothing and 14% more than native.

## Finding 2 — every single query stays inside the 50 ms budget at the ceiling

The worst single operation at 10,000 persons is `queryResolve` at **32.4 ms**, or 65% of ADR-0029's
interactive budget. Everything else lands at 13–17 ms, under 35%.

`queryResolve` scales at ≈3.2 µs/person, so it crosses 50 ms at roughly **15,600 persons** — beyond
the ceiling ADR-0029 pins, but not by much.

This is the hover lens ([#276](https://github.com/YashBhalodi/kul/issues/276)), a zero-click
affordance firing on pointer movement, and it runs on the webview's main thread. "Inside budget" is
not the same as "free": 32 ms per call means a lens that fires on every pointer move caps the frame
rate at ~30 fps and blocks paint while it runs. **The lens needs a trailing-edge debounce at any
corpus size worth calling large — but it does not need a spinner.**

## Finding 3 — `check` is the cost; the query is rounding

`check` is 42% of a `queryResolve`, 93% of a `runQuery` filter, and effectively 100% of a
`queryPerson` (13.72 vs 13.70 ms — a detail lookup against a checked document is below the noise
floor). It is linear and predictable: **≈1.35 µs per declared person** above ~1k, holding to within
2% across the whole range.

That single constant predicts every number in this document. It also says where the headroom is: a
stateful WASM document handle — the third fix in ADR-0034's ladder — would take `queryResolve` from
32 ms to ~19 ms and `runQuery` from 15 ms to ~1 ms, because it removes the part that dominates.

## Finding 4 — eager per-entity hydration fails at the ceiling, and the failure is arithmetic

ADR-0034 chose to fetch every kin-list row on open, recorded it as "the most expensive combination
available here", and projected roughly a quarter-second for 20 rows at the ceiling, native, before
any multiplier. That projection was right, and the multiplier did not save it.

Eager hydration = N sequential `queryPerson` calls, WASM, milliseconds:

| persons | N=5 | N=10 | N=20 | N=40 |
| --- | ---: | ---: | ---: | ---: |
| 266 | 1.7 | 3.5 | 6.9 | 13.8 |
| 1 000 | 6.5 | 13.1 | 26.1 | 51.6 |
| 2 500 | 16.7 | 33.4 | 67.0 | 134.1 |
| 5 000 | 33.9 | 67.4 | 135.9 | 271.7 |
| **10 000** | **68.7** | **137.6** | **275.7** | **545.0** |

Because each row re-checks the whole project, cost is `N × 1.37 µs × persons` — the model predicts
every cell above to within 3%. So the budget line is a single product:

> **N × persons ≤ ~36,000**, or the panel takes longer than 50 ms to populate.

| kin-list rows | largest project that stays under 50 ms |
| ---: | ---: |
| 3 | ~12,000 persons |
| 5 | ~7,300 |
| 10 | ~3,650 |
| 20 | ~1,825 |
| 40 | ~910 |

At the 10,000-person ceiling, **any eager list of 4 or more rows misses the budget**, and a
plausible 20-row list misses it by 5.5×. The full cost of one selection at the ceiling —
`queryPerson` for the details widget, `queryKin` for the list, then 20 hydrations — is ≈306 ms.

## Finding 5 — the batched detail op is flat in N, and it is the whole fix

ADR-0034 names a batched detail operation (one call, many ids, paying the check once) as the first
cheap out. Measured natively as one `check` + N lookups off the same `ResolvedDocument`:

| persons | N=5 | N=10 | N=20 | N=40 |
| --- | ---: | ---: | ---: | ---: |
| 266 | 0.37 | 0.37 | 0.36 | 0.35 |
| 1 000 | 1.25 | 1.20 | 1.12 | 1.11 |
| 2 500 | 2.84 | 2.88 | 2.87 | 2.88 |
| 5 000 | 5.92 | 5.89 | 5.89 | 5.86 |
| **10 000** | **13.29** | **13.14** | **13.17** | **13.40** |

Flat — the lookups are free, the check is everything. At the ceiling this turns 545 ms into 13 ms
for a 40-row list: a **41× improvement**, and it makes list size stop mattering entirely. Applying
Finding 1's ~1.0× multiplier at that size, the WASM cost would be the same 13 ms.

## Finding 6 — phrasing needs no lookup at all; only the display name does

`queryKin` returns `Member { personId, descriptor }` — the full `RelationshipDescriptor`, backbone
included. That is exactly what [ADR-0033](./adr/0033-phrasing-layer-architecture.md)'s phrasing layer
consumes, so **every row's kinship phrase is already in the kin-set answer**. The per-row lookup
buys one thing: the person's *name*.

And the name is already in the DOM. [ADR-0021](./adr/0021-language-properties-plumb-to-svg.md)'s
exhaustive plumb-through puts `data-family` and `data-given` on every card, which is ADR-0034's
second cheap out ("list labels off the cards") — free, and it needs no engine work at all. Its cost
is the one the ADR already named: two provenance paths for person data.

## Finding 7 — module load is not a concern

`require` + synchronous compile + instantiate of the 593 KB module: **3.6 ms**. Lazy-loading on
first query, as ADR-0034 specifies, costs the first query a few milliseconds and nothing after.

## What this means for #282

1. **The multiplier is a non-issue and should stop being treated as a risk.** ADR-0034's arithmetic
   holds; its pessimism did not need to.
2. **No query interaction needs a progress affordance** — with one exception, below. Single
   operations are 13–32 ms at the ceiling, which is under the threshold where a spinner helps rather
   than flickers.
3. **The hover lens needs a trailing-edge debounce**, not a spinner. 32 ms of main-thread work per
   pointer move is a jank source, not a latency source.
4. **The eager kin-list hydration as decided does not ship at the ceiling.** This is the one place
   the measurements change scope rather than polish. ADR-0034 anticipated it and ordered the fixes;
   the numbers pick one. Recommended: **take the list labels off the cards** (Finding 6) — zero
   engine work, zero Rust release, and it collapses the whole N-row cost to nothing, because the
   descriptors that phrase each row already arrive with the kin-set answer. A batched detail
   operation (Finding 5) is the alternative if the PRD would rather not have two provenance paths
   for person data; it costs new WASM-surface work and a shape ADR-0024 did not pin.

   > **Settled by [ADR-0035](./adr/0035-detail-surface-one-selection-one-batched-lookup.md) (#281)**:
   > the **batched operation** was chosen over this recommendation. The detail panel turned out to
   > need new engine work anyway — `queryMarriage` cannot return a marriage's children — so batching
   > came almost free with it, and buying one provenance path for the whole widget was judged worth
   > the Rust release. Finding 5's flat-in-N number is what makes it work.
5. **The stateful WASM document handle stays unnecessary.** It is the largest win available
   (Finding 3) and it is still not needed: with hydration fixed, nothing else is close to the
   budget.

None of this revisits ADR-0034. It exercises the outs that ADR already recorded.

## Caveats

- **Node, not the webview.** Measured under Node v22.18.0 (V8), not VS Code's Chromium webview. The
  WASM engine family is the same and these are engine-cost numbers, but the webview additionally
  pays a module fetch over `vscode-webview:` and contends with rendering on the same main thread.
  Neither was measured.
- **Best-of-N on an idle machine**, matching the convention of the existing perf gate (best of 5).
  These are floors, not p95s.
- **One file, dense.** The corpora are a single `.kul` file with fully populated generations. A real
  project split across many files parses the same total bytes; a sparser tree of the same person
  count would check slightly faster.
- **`persons` is the parameter that matters.** Every cost here tracks declared person count, and
  `check` tracks it linearly, so the model extrapolates — but only within the shapes the synthetic
  corpus covers.

## Reproduction

The harness is deliberately not committed — it is a one-shot measurement, not a gate. To rebuild it:

1. Copy `crates/kul-core/tests/perf.rs` lines 1–293 (module docs through `generate_corpus`) into a
   scratch test file, and add a `corpus_sized(target)` variant of `generate_corpus` that scales
   `primary_budget` to 94% of `target` and the second component to 4.5%, keeping `max_gen = 12`.
2. Add a test that, per size, writes `corpus-{n}.kul` and times `check_with_manifest` plus each of
   `person`, `resolve`, `run_query` and `cousins_of` — each preceded by its own `check`, since the
   WASM ops are stateless. Run under `--release`.
3. `wasm-pack build crates/kul-wasm --target nodejs --out-dir <scratch>/pkg-node --out-name kul_wasm`.
4. Drive `check`, `queryPerson`, `queryResolve`, `runQuery` and `queryKin` from a Node script over
   the same corpus files, best of 7 after a warm-up, plus N sequential `queryPerson` calls for
   N ∈ {5, 10, 20, 40}.

The landmark ids used: `queryResolve` runs `deepLeaf` ↔ `linealAncestor` (~8 generations apart, 25
distinct relationship paths), `queryKin` runs `collateralByDegree {degree: 2, removed: 0}` from the
mid-spine person, and the filter is `born` in `[1850, 1950]` sorted ascending over `allPersons`.

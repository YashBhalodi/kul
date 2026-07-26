# PRD 0006: Kinship Query UX in the VSCode Preview

> Product epic: [#296](https://github.com/YashBhalodi/kul/issues/296). Child issues are linked from the epic. This PRD is transient — delete it in the same PR as the final piece of implementation work (see [README](./README.md)).

## Problem Statement

[PRD-0005](https://github.com/YashBhalodi/kul/pull/254) shipped the kinship query engine: kin-set queries, two-anchor relationship resolution, attribute filtering, and id → detail lookups, surfaced over WASM and `kul query`. Its governing line, pinned as [ADR-0025](../adr/0025-kinship-query-engine-contract-and-traversal.md), is that **the engine owns kinship correctness and consumers own only the UX of querying.**

Nobody owns that UX yet. The engine's answers are reachable from a terminal and from a JavaScript API, and from nowhere a reader actually reads a family. The VSCode preview — the one surface where someone looks at a Kul project rather than at Kul source — cannot answer a single kinship question. A reader looking at a rendered tree can see that two cards are connected; they cannot ask who someone's cousins are, cannot ask how two people are related, cannot ask which people in this project were born before 1950, and cannot see a person's recorded details beyond a hover tooltip that reads attributes off the SVG.

The engine also emits no words, by design and permanently ([ADR-0026](../adr/0026-relationship-descriptor-and-path-identity.md)): it returns a terminology-neutral `RelationshipDescriptor` and refuses to say "sister-in-law", because the moment the core emits a kinship word it becomes the first culture pack, shipped by accident. That refusal is correct and it leaves a hole. Somebody has to turn a descriptor into a word, and until somebody does, the engine's most valuable answer — *how are these two people related?* — cannot be shown to a human at all.

The gap this PRD closes: **the four engine capabilities have no reader-facing surface, and the descriptor has no renderer.**

## Solution

Make the rendered tree in the VSCode preview the query surface, and phrase its answers in English and Gujarati.

The design is settled. Twenty prototype rounds across [#276](https://github.com/YashBhalodi/kul/issues/276) and [#277](https://github.com/YashBhalodi/kul/issues/277), a term-inventory research pass ([#278](https://github.com/YashBhalodi/kul/issues/278)), a measurement pass ([#292](https://github.com/YashBhalodi/kul/issues/292)) and four decision tickets ([ADR-0033](../adr/0033-phrasing-layer-architecture.md) through [ADR-0036](../adr/0036-two-tier-theming-and-the-accessibility-non-goal.md)) produced a complete specification on the [query-UX wayfinder map (#275)](https://github.com/YashBhalodi/kul/issues/275). This PRD assembles that specification into one buildable document. **It decides nothing new.** Where a decision has an ADR, the ADR is the authority and this document summarises it; where a decision lives only in a prototype resolution comment, this document is the authority.

Five moves make up the product:

- **Direct manipulation, one selection.** Clicking a person card, a marriage edge or an adoption edge selects that entity. There is no query language, no query box, and no second selection state. Clicking is also how you walk the family.
- **A details panel that is also a navigator.** The selection populates a floating right-edge panel — one variant per entity kind — fed by a single batched engine call. Every person named in it is clickable: clicking moves the selection and pans the canvas to centre that person.
- **Kin sets as paint on the tree.** "Explore kin" opens one flat scrollable list of every kin-set query with its count; clicking a row lights the matching cards teal, ghosts included, and dims the rest. The tree is the result; there is no results list anywhere.
- **Relationship resolution as a hover lens.** While a person is selected, hovering any other card resolves the tie live and whispers it in a pill docked under the target — zero clicks, no mode. This is the affordance that made the whole model click, and it is why phrasing has to be fast and local.
- **Attribute filtering as a chip sentence.** *Showing Everyone where family = Rossi and born < 1950, certain.* The bar lives in stage chrome above the tree, independent of selection. **Filtering never hides a node or an edge — it only dims.** The canonical tree survives every filter intact.

Under all five sits the layer the engine refuses to own: **phrasing**, a consumer-side TypeScript module where a language pack is pure data, shipping `en` and `gu`, structured so the next language is one additive data module and zero lines of logic.

Two engineering consequences fall out of the design rather than being chosen for their own sake. The engine **runs in the webview** via the already-shipped `@kullang/wasm` surface, because the hover lens cannot afford a round-trip. And the preview's theming is **rebuilt in two tiers** before any new chrome is styled, because the shipped ~120-token flat scheme has no room for four new hues and no structure to grow into.

## User Stories

### Reader of a family they did not author (primary actor)

1. As a reader, I want to click a person on the tree and see their recorded details, so that I can learn who someone is without reading the source.
2. As a reader, I want to click a marriage line and see that marriage's own record — spouses, span, how it ended, children with their link kinds — so that a line between two cards is something I can interrogate.
3. As a reader, I want to click an adoption line and see the adoption's record, so that every kind of connection the picture draws is readable.
4. As a reader, I want a selected person's kin sets listed with counts, so that I can see at a glance that they have four siblings and eleven descendants before I look at any of them.
5. As a reader, I want clicking a kin set to light its members on the tree, so that I read the answer in the picture I am already looking at rather than in a list beside it.
6. As a reader, I want every card a person owns to light — canonical and ghosts alike — so that a result is never invisible in the past family whose edges its ghost anchors.
7. As a reader, I want to hover any other person while someone is selected and be told how they are related, with no click and no mode, so that exploring a tie costs nothing.
8. As a reader, I want the relationship whispered under the person I am hovering, so that I never have to look away from the two people to read about them.
9. As a reader, I want the connecting people traced on the tree while the lens reads, so that I can see *why* the answer is what it is.
10. As a reader, I want to be told when two people are related more than one way, so that a consanguineous family is not flattened to one tie.
11. As a reader, I want every name in the details panel to be clickable, so that the panel is a way to walk the family and not just a readout.
12. As a reader, I want the canvas to centre the person I clicked in the panel, so that a selection I cannot see is never a dead end.
13. As a reader, I want to filter the whole project by attributes — family, dates, presence of a field — so that I can ask questions about the project and not only about one person.
14. As a reader, I want filtering to dim rather than hide, so that the shape of the family I am reading never changes under me.
15. As a reader, I want to filter *within* a painted kin set, so that "which of Giuseppe's descendants were born before 1950" is one question and not two.
16. As a reader, I want relationships phrased in my language, so that a tie reads as a word I use rather than a diagram I decode.
17. As a Gujarati reader, I want *māmā* and *kākā* to be different words, so that the tool does not flatten a distinction my language makes and English does not.
18. As a bilingual reader, I want to flip the language and see the same tie both ways, so that I can compare what each language merges.
19. As a reader, I want a relationship with no lexical term rendered as a genitive chain built from terms my language *does* have — *māmā-no dīkro*, not a four-link walk — so that the fallback still sounds like my language.
20. As a reader, I want to reveal the selected entity in the editor when I choose to, so that reading can turn into editing deliberately rather than by accident.

### Honesty (cross-cutting — the stance, stated as stories)

21. As a reader, I want an empty kin set to be a plain zero, so that "no cousins" reads as an answer rather than as a failure.
22. As a reader, I want an unrelated pair to be told to me as "not related within bounds", so that I know the difference between *no tie* and *no tie as far as the engine looked*.
23. As a reader, I want people the filter could not judge to be shown as *can't say* with their own paint, so that they are never silently dropped into the non-matching pile.
24. As a reader, I want to be told *why* someone is unjudgeable — the field is not recorded, the circa date straddles the boundary — so that I can fix the record or accept the gap knowingly.
25. As a reader, I want certain mode to disclose how many people it dropped, so that a filter's answer includes what it could not answer.
26. As a reader, I want `died not recorded` never to be presented as "living", so that the surface does not assert a fact the data does not carry.
27. As a reader, I want a seniority-marked term withheld when the dates cannot order two people, so that the tool says *bhāī* rather than inventing *moṭā bhāī*.
28. As a reader, I want an adoptive mother called "mother", so that the phrasing matches how families actually speak while the fact stays available on the record.

### Author (secondary actor — querying mid-authoring)

29. As an author, I want querying to work on an incomplete project, so that I can check what I have entered so far.
30. As an author, I want editing to end query mode cleanly, so that querying and authoring never fight over what a highlight means.
31. As an author, I want editor⇄preview sync to resume on its own after I edit, so that the mode boundary costs me no gesture.

### Contributor (secondary actor — the codebase)

32. As a contributor adding a third language, I want to add one data module and zero lines of logic, so that the additivity promise ADR-0026 designed the descriptor for is structurally true.
33. As a contributor, I want a language pack reviewable as data, so that a term correction is a record change and not a code change.
34. As a theme author, I want to supply ~20 values rather than ~120, so that theming the preview is tractable and stays tractable as chrome grows.
35. As a contributor, I want a lint that fails when a token layer is violated, so that the two tiers cannot erode the way the flat scheme did.
36. As a contributor, I want the details panel fed by the pinned engine contract rather than by SVG attributes, so that there is one provenance path for person data.

## Implementation Decisions

### Transport — the engine runs in the webview

Per **[ADR-0034](../adr/0034-query-transport-and-result-node-identity.md)**:

- **No `kul/query` LSP request.** The preview calls `@kullang/wasm` directly; `kul-lsp` keeps its five custom methods and is untouched by this epic. A query is a local function call, which is what makes a zero-click hover lens viable.
- **The extension posts `files + manifest` with each render**, because the WASM surface is stateless. The source the webview queries is always the source the picture came from.
- **The ~580 KB module loads lazily on first query**, so opening a preview stays exactly as fast for anyone who never queries.
- **Every call re-checks the project.** Measured at ≈13.7 ms (WASM) on 10,000 persons — the dominant per-call cost, inside budget, and the epic's main latency lever if anything bites.
- **The CSP gains `'wasm-unsafe-eval'`** in `script-src` plus a `connect-src` allowance. A real loosening of a deliberately tight posture, accepted as the cost of the transport decision; base64-inlining was rejected as worse.
- **Two engines describe the same file, knowingly** — the LSP renders, the bundled WASM queries. Skew is possible via separately-fetched server binaries; the failure mode is recorded rather than mitigated, and `KUL_CORE_VERSION` makes a handshake one field away if skew is ever observed.
- **Queries are gated on a clean project.** [ADR-0009](../adr/0009-export-strict-on-diagnostics.md) strictness means a failing check yields the envelope's error arm; the preview's existing error popover is where that surfaces.

### The batched detail operation — the epic's Rust critical path

Per **[ADR-0035](../adr/0035-detail-surface-one-selection-one-batched-lookup.md)**:

- **One new WASM operation** returns, in one check, an entity's own fields plus its relational neighbourhood — parents, spouses, and children carrying their `biological` / `adoptive` link kinds. It also supplies the kin list's row labels.
- **This is forced, not preferred.** `queryMarriage` returns `ExportedMarriage` with no children, and no parenthood-link operation exists at all, so [ADR-0024](../adr/0024-query-seam-and-envelope.md)'s pinned surface cannot answer the marriage panel. The alternative was reading the panel off SVG `data-*` attributes, rejected twice on provenance.
- **Batched rather than fine-grained, on measurement.** Composing `queryPerson` + three `queryKin` calls costs ≈63 ms at the 10k ceiling, over [ADR-0029](../adr/0029-query-engine-performance-posture.md)'s 50 ms budget for one click. One batched call is **flat at ≈13 ms** regardless of N: a 20-row kin list goes 276 ms → 13 ms, and list size stops mattering.
- **This puts a `kul-wasm` change, a version bump and a release inside the epic.** The detail panel cannot ship before it does.

### Result ↔ node identity

Per **[ADR-0034](../adr/0034-query-transport-and-result-node-identity.md)**:

- **Results bind by `data-person-id` and paint every card for that person** — canonical and ghosts together. A result is about a person, not about a card.
- **Edges bind from data the descriptor already carries**: an `across` hop names its marriage (`data-marriage-id`), vertical hops select birth/adoption edges by `data-child-id`.
- **Ghost cards stay individually unaddressable.** `kul-layout` keys child-ghosts by `(person_id, marriage_id)` so surfacing per-card identity is cheap, but no decided interaction needs it and it would cost a `kul-svg` change and a Rust release.

### The interaction model

Per **[#276](https://github.com/YashBhalodi/kul/issues/276)**'s resolution, widened by **[ADR-0035](../adr/0035-detail-surface-one-selection-one-batched-lookup.md)**:

- **One selection, over three entity kinds.** Person cards, marriage edges and adoption edges are all selectable; clicking another entity *moves* the selection; Esc or a canvas click clears. "Single selection, always" is unchanged in number, widened from persons to entities.
- **A person selection gets the full model** — panel, Explore kin, live lens. **An edge selection gets a panel only** (the lens needs a person ego; a kin set needs a person anchor). An edge is a **waypoint, not a dead end**: its spouse and child rows walk the selection back to a person.
- **Kin sets are one flat scrollable list** with per-set counts — no category grouping. Clicking a row paints results; the list stays open as the selection walks.
- **The hover lens** resolves and whispers the tie live while a person is selected, with a light dashed glow tracing the connecting persons. It keeps reading while a kin-set paint is active.
- **The relation card is a docked-tag whisper** — a minimal pill under the target card, terms only, single language, a violet viewpoint dot ("as the selected person sees it"). Position disambiguates direction. Multi-tie stacks inline (*māmā · sasro*). Hovering a term gives the fuller gloss.
- **Paint vocabulary:** selection violet, hover-target amber, results teal with non-matches dimmed, connecting path dashed sky, matched ghosts dashed teal.
- **Honest emptiness:** an empty kin set is a zero count plus a quiet toast; an unrelated pair under the lens whispers "not related (within bounds)". Nothing invented, nothing modal.

### The details panel

Per **[ADR-0035](../adr/0035-detail-surface-one-selection-one-batched-lookup.md)**, refined by **[ADR-0036](../adr/0036-two-tier-theming-and-the-accessibility-non-goal.md)**:

- **Three variants** — person, marriage, adoption — all fed by the one batched operation.
- **The panel floats over the canvas; the canvas never reflows.** It is selection-scoped, so docking would resize the tree on every select and every clear and force `svg-pan-zoom` to re-fit each time. The accepted cost is that an opening panel can cover a card you were looking at.
- **Absent fields are omitted**, never rendered as "not recorded" — mirroring the wire, where both export shapes use `skip_serializing_if`. A missing death date is invisible rather than loud; the panel's height varies as the selection walks.
- **Each marriage occupies its own line, with span and end reason.** Load-bearing rather than cosmetic: it carries the explanation for why a person appears on more than one card. **There is no ghost note** — sourcing one would either re-derive [ADR-0019](../adr/0019-ghost-model-and-bio-anchor.md) inside the query layer or re-open a second provenance path.
- **Every person named in the widget is clickable**, moving the selection and centring that person's canonical card in the *visible* region — so a panel-driven walk never parks a card behind the panel.
- **The hover tooltip retires.** `tooltip.ts`, its tests and its mount are deleted; the widget is the only detail surface. This is a deliberate deletion of a working feature and the price of hover becoming the lens: zero-commitment "what is this person" reading goes away, and reading a person now costs a click.
- **Reveal-in-editor moves from the tree to the panel header**, posting the same `onRevealRequest({kind: "entity", id})` on an unchanged seam — now working for marriages as well as persons. Adoption links have no entity id, so the adoption panel's reveal targets the **child**.

### Attribute filtering

Per **[#277](https://github.com/YashBhalodi/kul/issues/277)**'s resolution:

- **Home is stage chrome, not the widget.** A filter is a project-scoped question; the details panel is selection-scoped. The panel grows no filter affordances.
- **Input is a chip sentence** — *Showing ⟨Everyone 10 ▾⟩ where ⟨family = Rossi ✕⟩ and ⟨born < 1950 ✕⟩ ⟨+⟩ ⟨certain ▾⟩*. Each chip opens a small field/op/value editor; `+` picks a field; the `and` keywords are literal text, so **conjunction-only is visible in the grammar** rather than documented elsewhere. Ops are field-aware: ordering comparisons only on `born`/`died`, `∈` only on text fields.
- **No OR, and the UI says so.** No OR chip, no OR affordance, no "advanced mode". OR is permanently out of engine scope (ADR-0025), so the surface must not imply it exists.
- **Certainty is a chip inside the sentence** — `certain` / `certain + can't say` — not a display toggle. The third truth value is part of the query.
- **Non-matching people dim. Always.** No hiding, no collapsing, no re-layout. **Standing rule, generalised past this feature: filtering never hides a node or an edge — it only dims.** Hiding punches holes in the canonical layout and leaves marriage stubs and birth edges running into empty space.
- **"Can't say" has its own paint** — amber dashed outline plus a `?` riding the card, distinct from both match and non-match — and hovering whispers the reason in the lens's docked-tag grammar (*"can't say — family not recorded"*, *"~1942 is approximate (±5y) — straddles born < 1945"*).
- **The tally discloses what certain mode dropped**: *"3 of 10 · 7 dimmed · 1 can't say"*. Unjudgeable people are counted and nameable, never silently absent.
- **Presence predicates never imply a fact.** `died not recorded` is not "living", and the surface says so where the predicate is offered.
- **Composition with kin sets:** the scope chip switches between `allPersons` and `kinOf(anchor, set)`, and the tally names the scope — *"3 of 4 in Giuseppe's descendants"*.

### Phrasing

Per **[ADR-0033](../adr/0033-phrasing-layer-architecture.md)**:

- **Phrasing is consumer-side TypeScript** at `packages/preview/src/phrasing/`, under a standing **no DOM imports** rule so extraction to its own package stays mechanical. A `kul-terms` Rust crate was rejected on scope, not principle.
- **The phrasing key is the descriptor's normalized dimensions plus six facets derived from the path backbone** — `acrossCount`, `acrossAtStart`, `acrossAtEnd`, `spouseGender`, `linkGender`, `endedMarriage`. The set is closed and validated: it separates the six Gujarati terms that share one normalized cell (*sāḷo* / *jeṭh* / *naṇand* / *bhābhī* / *banevī* / *vevāī*). **No engine change, no new wire field.**
- **A language pack is data** — a facet-match table where an omitted facet is a wildcard, **most-specific-wins**, declaration order meaningless, and an exact tie is a **defect caught by a test**, not a coin toss.
- **Productive morphology is data too** — affix rules with `when` / `affix` / `position` / `repeatPer` / `cap`, covering English's repeating `great-` and Gujarati's single `par-` alike.
- **An unknown facet disqualifies the entry that keys it**, falling through to the less specific entry. `seniority: unknown` yields *bhāī*, never a guessed *moṭā bhāī*. `notApplicable` is a value and matches normally.
- **The fallback is recursive prefix lexicalization over the same table** — longest prefix that hits becomes the head, the rest renders as a genitive chain. **There is no second lexicon.** Each pack adds a hop lexicon and a genitive join pattern (English `'s` left-to-right; Gujarati's `-no` / `-nī` / `-nũ` agreeing with the following noun).
- **Sub-path keys mark `sharing`, `apexSeniority` and the `side = both` refinement `unknown`**, because a hop sequence lacks the graph context to compute them — and the never-guess rule then makes that automatically safe.
- **Phrases are bare** — "mother's brother", not "your mother's brother". Ego-relative framing is chrome copy.
- **Locale is a preview toggle** in webview-local state (`getState`/`setState` behind a small store interface; in-memory by default, VSCode-backed in `adapter-vscode.ts`). `HostAdapter` is untouched. A `language:` field in `kul.yml` was rejected — `kul.yml` is normative and would bind every third-party consumer.
- **The result is `{ text, kind, hopCount }`**, so chrome can treat a crisp *kākā* and a five-hop chain differently in the same slot without string-sniffing.
- **Three terminology policies:** ended marriages phrase marked where the language has an affix and unmarked where it does not (English "former mother-in-law"; Gujarati *sāsu*, with disclosure left to chrome); `edgeNature: adoptive` renders unmarked; *jamāī* / *vahu* key `linkGender`, not `alterGender`, so a son's husband is never rendered *jamāī*.
- **Gujarati output needs `lang="gu"` markup.** `html.ts` hardcodes `<html lang="en">`; per-element `lang` is a correctness item for font selection, not a styling preference.

### Theming and layout

Per **[ADR-0036](../adr/0036-two-tier-theming-and-the-accessibility-non-goal.md)**:

- **The token layer is rebuilt in two tiers.** Tier 1 is what a theme supplies — colour roles, spacing scale, radius scale, motion durations, reserved hues; roughly twenty values that stay roughly twenty as chrome grows. Tier 2 is the per-site semantic layer, aliasing onto tier 1 rather than onto `--vscode-*`.
- **Values are re-expressed, not re-chosen.** The diagram keeps its gender hues, edge colours and ghost treatment; the query chrome keeps the visuals #276 and #277 settled. This is a theming-architecture change, not a visual redesign.
- **[ADR-0016](../adr/0016-visualization-pipeline-crate-boundaries.md)'s two load-bearing properties survive** — one `var(--kul-*)` per application site, and the physical two-file split. What changes is where values come from and how many a theme must supply (~120 → ~20).
- **Four query hues are reserved literals** — selection violet, result teal, can't-say amber, resolution-path sky — documented in place exactly as the existing `--kul-selected-outline-color: #ff2d95` is. `--vscode-charts-*` is fully spent on document meaning (three genders, three edge kinds), so any themed query paint would collide with something the diagram already says. The filter dim is a single tier-1 alpha, theme-symmetric by construction.
- **Chrome layout becomes a region model** — filter bar in flow, canvas, overlay stack, entity-anchored floats, transient notifications — replacing hand-computed `calc()` insets. Collisions become impossible by construction, which fixes a **shipped bug found on the way past**: the legend and the error popover pin to the identical inset with independent visibility booleans and overlap when both are open.
- **`mountKeyboardPan` gains a target guard** so arrows and `+`/`-`/`0` do not fire from inside the filter bar's inputs. Not an accessibility accommodation — a plain bug the moment that bar has text fields.
- **The legend does not learn query paint.** [ADR-0022](../adr/0022-svg-legend.md) is untouched; the legend explains *document* vocabulary and is shared verbatim with the baked SVG. Query paint is always the direct consequence of something the user just did.
- **`kul-svg` bakes tier 1 plus the diagram aliases** and skips the chrome aliases, the same structural-subset rule it follows today. The token rename reaches `crates/kul-svg/src/emit.rs` and the literal token assertions in `visual.rs` — **the epic's second Rust touchpoint.**

### Accessibility is a stated non-goal

Per **[ADR-0036](../adr/0036-two-tier-theming-and-the-accessibility-non-goal.md)**. The query UX assumes a working mouse and keyboard. This epic builds **no** keyboard path into a query, **no** focus machinery on the tree, **no** screen-reader affordances, and **no** reduced-motion handling.

The non-goal reaches into the markup: new query chrome carries **no** `role`, `aria-*` or `:focus-visible`. This is deliberately unlike its neighbours — `controls.ts`, `legend.ts` and `errors.ts` carry all of it — and the inconsistency is the accepted cost of a clear position, because partial accessibility advertises a path that dead-ends at the tree, where every query begins.

It is written down rather than left silent because silence reads as an oversight, and an oversight gets "fixed" by the next contributor. **Nothing here licenses breaking what works**: existing chrome keeps every attribute it has. The roving-`tabindex` keyboard twin was considered in full and rejected on roadmap cost — it needs a spatial traversal order invented over `kul-layout`, and it collides head-on with arrows already being bound window-wide to panning.

### The mode boundary

Per **[ADR-0035](../adr/0035-detail-surface-one-selection-one-batched-lookup.md)** and **[#276](https://github.com/YashBhalodi/kul/issues/276)**:

- **Editor⇄preview sync suspends while a query selection or an active filter exists** (with a hint, bottom-right). The two meanings of "highlighted" never co-paint.
- **An edit ends query mode.** A render clears the selection, its panel, any kin paint and the filter; the tree returns to plain and sync resumes on its own. Renders only ever come from document changes, so no class of repaint clears a selection surprisingly.
- The accepted cost: a one-character typo fix costs the user their selection, their painted kin set and their filter. Refetching the bundle per render was affordable (≈13 ms per 300 ms-debounced render) and was rejected in favour of the mode boundary — no query artefact outlives the source it was computed from.

### Latency posture

Per **[#292](https://github.com/YashBhalodi/kul/issues/292)** ([`docs/query-path-measurements.md`](../query-path-measurements.md)):

- **The WASM multiplier is 1.0–1.14× at the ceiling**, not the assumed 1.5–2×, and it *shrinks* as the corpus grows. Every single operation stays inside the 50 ms budget at 10,000 persons — worst case `queryResolve` at 32 ms.
- **`check` is 42–100% of every call**, at ≈1.35 µs/person, linear. It is the cost; the query is rounding.
- **No spinners.** The hover lens gets a **trailing-edge debounce** instead — the affordance that fires on pointer movement is the one that must not queue work per pixel.
- **Module load is 3.6 ms**, so the lazy first-query load needs no affordance of its own.
- **Eager per-entity hydration is dead**, retired by the batched operation rather than worked around.

### Vocabulary and records

- **New domain vocabulary** — phrasing layer, language pack, phrasing key, derived facet, hover lens, kin paint, query mode, region model, tier-1 / tier-2 tokens — is added to `CONTEXT.md` by the slices that introduce it.
- **No new ADRs are expected.** The four decisions this epic implements are already recorded (ADR-0033 … ADR-0036). If a slice makes a non-obvious choice these did not anticipate, it lands as an ADR in the same PR per `AGENTS.md`.
- **Three transient documents graduate when the epic ships**: this PRD, [`docs/kinship-term-inventory.md`](../kinship-term-inventory.md) (its findings are recorded in ADR-0033; its term data becomes the `en`/`gu` packs), and [`docs/query-path-measurements.md`](../query-path-measurements.md) (its findings are recorded in ADR-0034 and ADR-0035).

## Testing Decisions

- **Phrasing is table-driven, not case-per-term.** A pack is records, so the suites are: every bounded key resolves to a term or a well-formed chain (coverage), no two entries match one key with equal specificity (conflict-freedom), and every ⚠-marked inventory row selects its intended term (discrimination). Key-space enumeration bounds generations, `cousinDegree` and `removed` at 4 — beyond that the fallback fires anyway.
- **The never-guess rule gets explicit fixtures**: `seniority: unknown` → *bhāī* not *moṭā bhāī*; `apexSeniority: unknown` → *kākā* not *moṭā bāpā*; sub-path keys marking graph-dependent facets `unknown` and the right entries falling out as a result.
- **The `gu` pack's PR must touch no logic.** That is the additivity promise's test, and it is reviewable as a diff shape, not only as an assertion.
- **The token architecture is held by a Vitest structural lint** reading both stylesheets as text: every consumed `var(--kul-*)` is defined, no `var(--vscode-*)` outside tier 1, every tier-2 alias resolves to a tier-1 primitive, no raw hex in the application sheet except the documented reserved hues, and no token defined but unused. Direct precedent: `crates/kul-svg/tests/visual.rs` already lints the baked token layer as text. jsdom resolves neither custom-property substitution nor layout, so a computed-value test is not reachable with the current stack.
- **No browser-based visual regression.** Rejected on dependency weight and CI flakiness: it means browser automation in a repo with none, screenshot baselines across three themes, and platform-dependent rendering sitting on the epic's critical path. The structural lint covers the failure mode a rename actually has.
- **`kul-svg` snapshots are re-blessed once, in one deliberate commit**, as part of the token-rename slice. A snapshot diff anywhere else in the epic is a defect.
- **The batched detail operation is proven at the core seam with `insta`**, per [ADR-0003](../adr/0003-snapshot-tests-as-primary-validation.md) and PRD-0005's discipline — the relational neighbourhood (parents, spouses, children with link kinds) against the example corpus, including a marriage with children of mixed link kinds and a person with a 0..4+ parent set. **The WASM adapter is tested for wiring and serialization only**; kinship correctness is not re-tested at the adapter.
- **Chrome behaviour is tested at the module seam, not through a browser.** Selection state transitions, the waypoint rule (edge selection → spouse row → person selection), result-to-card binding including ghosts, filter evaluation and tally arithmetic, and the render-clears-query-mode rule are all unit-testable over the existing jsdom setup.
- **The engine's own filter semantics are not re-tested in the preview.** `kul-core`'s `filter__*.snap` suites own three-valued predicate correctness; the preview tests that its chip sentence produces the right `Query` value and renders the right tally.
- **A perf assertion guards the batched operation's flatness in N** — the property the whole detail surface rests on — alongside the existing corpus perf budgets in `docs/testing.md`.

## Out of Scope

- **`sort` in the preview.** The decided model paints results on a tree that keeps its canonical layout order, and only a list can express an order — this model has none. `sort` stays a `kul query` plumbing feature for consumers who do have a list. Putting it in the preview requires introducing a results list first, which is a scope change and not a detail.
- **A results list, anywhere.** The tree is the result; a list would compete with it as the place to read answers.
- **Accessibility**, as stated above and in ADR-0036. Changing this position supersedes that ADR; it is not a bug fix.
- **Languages beyond `en` and `gu`.** The architecture must keep the next one cheap; no further term inventories ship here.
- **CLI human-readable query output.** `kul query` keeps its JSON/plumbing surface. Phrasing on the CLI is a separate future effort and would want the `kul-terms` Rust crate ADR-0033 rejected on scope.
- **A standalone web app.** This epic targets the VSCode preview only.
- **Query history, saved queries, result export.** Ruled out at charting to keep the epic lean.
- **A stateful WASM document handle.** The named fix if the per-call re-check bites; not built speculatively.
- **Per-card ghost identity.** Cheap in `kul-layout`, but no decided interaction distinguishes one ghost from another.
- **A `kul/query` LSP request.** Considered and rejected in ADR-0034; two IPC hops in front of a zero-click affordance.
- **Redesigning the diagram's visual language** while the tokens are rewritten. Out of scope by decision — it crosses into what `kul-svg` bakes.
- **Re-opening #276 or #277.** Twenty prototype rounds are closed with resolution comments and living prototype branches. Re-litigating them inside the epic built to implement them is the definition of churn.
- **[#289](https://github.com/YashBhalodi/kul/issues/289)** — the `filter.rs` date truth-table discrepancy, caught in passing by #277. A `kul-core` defect, independent of this epic, tracked on its own ticket.

## Further Notes

- **Why the hover lens is the load-bearing interaction.** It is what decided the transport. A zero-click affordance firing on pointer movement cannot afford a webview→extension→LSP round-trip, so the engine had to come to the webview — and once it is local, phrasing has to be local too, which is most of ADR-0033. Nine prototype rounds tried shift-click, drag-thread, corner badges and pick-mode relating; the lens superseded all four, and the architecture followed the gesture rather than the other way round.
- **Why phrasing needs facets the engine could not have anticipated.** The descriptor is maximally discriminating, but only *with* its backbone. Six Gujarati terms share one normalized cell, and four more term families split on hop genders. This is exactly the escape hatch ADR-0026 designed the lossless path for — and it worked: the answer was six derived facets in the consumer, not an engine change. That is the additivity promise paying out for the first time.
- **Why measurement changed the design twice.** #292 was expected to confirm a 1.5–2× WASM multiplier and instead found 1.0–1.14×, which retired a worry. The same run found that eager kin-list hydration fails arithmetically at 4+ rows on a 10k project — a failure nobody had suspected, since the individual operations all looked fine. The batched operation exists because of a measurement, not because of a preference, and the panel it feeds is cheaper *and* has one provenance path instead of two.
- **Why theming grew from a token family into a rebuild.** #284 was chartered to ask which parts of the query chrome get `--kul-*` tokens. Inventorying that turned up `--kul-control-*` — nominally the pan/zoom buttons — already serving as the de-facto base tier for the legend and error popover, a fully-spent chart palette with no unspent hue for query paint, and two panels pinned to the identical inset with no cross-talk. The bounded option (keep ~120 tokens, add a `--kul-query-*` family) was available and was rejected because it leaves all three problems in place.
- **Two shipped behaviours are deleted, and users will notice.** The hover tooltip and click-to-source on the tree both go. Hover becomes the lens; revealing becomes a deliberate panel control. Both belong in the epic's change notes, not only in its diff.
- **The honesty stance is inherited, not invented.** Empty kin sets, `disconnected` vs `noneWithinBounds`, certain-mode disclosure, unknown-disqualifies-the-entry, and `died not recorded` ≠ living are all one principle the engine already holds — never assert what the data does not support — carried into a surface where the temptation to smooth it over is much stronger, because a UI can always show *something*.
- **Map traceability:** every decision above traces to a closed ticket on [#275](https://github.com/YashBhalodi/kul/issues/275) — #276 → interaction model; #277 → filter UX, the dim-never-hide rule, and the `sort` scope call; #278 → the Gujarati inventory; #279 → ADR-0033 (phrasing); #280 → ADR-0034 (transport, identity); #281 → ADR-0035 (detail surface, batched op, mode boundary); #284 → ADR-0036 (two-tier theming, reserved hues, a11y non-goal, region model); #292 → the measurements that forced the batched op and the debounce.

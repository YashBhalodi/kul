# ADR 0033 — The phrasing layer: consumer-side, data-only language packs over a backbone-derived facet key

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[ADR-0026](./0026-relationship-descriptor-and-path-identity.md) pinned the relationship descriptor and forbade the engine ever emitting a kinship word: "the moment the core emits 'grandmother' it becomes the first culture pack, shipped by accident, and the additive-terminology promise is dead." [ADR-0025](./0025-kinship-query-engine-contract-and-traversal.md) drew the matching line — the engine owns kinship correctness, **consumers own only the UX of querying**. Both ADRs deferred the same thing: the layer that turns a descriptor into a word.

This ADR decides that layer. It resolves [#279](https://github.com/YashBhalodi/kul/issues/279) on the [query-UX wayfinder map (#275)](https://github.com/YashBhalodi/kul/issues/275), whose scope is interactive kinship querying in the VSCode preview with phrasing in English and Gujarati, and whose standing constraint from charting is that **adding a language must be one additive mapping entry**.

The forcing input is [`docs/kinship-term-inventory.md`](../kinship-term-inventory.md) ([#278](https://github.com/YashBhalodi/kul/issues/278)) — every lexicalized Gujarati and English kinship term expressed against the descriptor's dimensions. Four of its findings are load-bearing here, and the inventory is transient (it graduates into `en`/`gu` term data when the epic ships and is then deleted), so they are lifted into this ADR:

1. **The normalized descriptor fields alone cannot select a term — by a wide margin.** Four Gujarati term families key on facts that live only in the path backbone: grandchildren by the linking child's gender (*pautra* vs *dohitra*), nephews by the linking sibling's gender (*bhatrījo* vs *bhāṇej*), the cousin lines (*pitrāī* / *masiyāī* / …), and the whole affinal collision class.
2. **The affinal collision class is the sharpest case.** *sāḷo* (wife's brother), *jeṭh* / *diyar* (husband's elder / younger brother), *bhābhī* (brother's wife), *banevī* (sister's husband) and *vevāī* (child's spouse's father) are **all** `collateral {up:1, down:1, cousinDegree:0, removed:0}` + `affinity: inLaw`. Six terms, one normalized cell. What separates them is where the `across` hop sits and the gender of the person on specific hops. English merges the lot into "brother-in-law" and never notices.
3. **The unlexicalized regions are large and common** — second cousins and beyond, removed cousins, lineal generations ≥ 4, grand-nephews, almost all two-`across` chains — and the natural fallback in both languages is the genitive chain (*māmā-no dīkro*, "your mother's brother's son"). Designing the fallback is half of designing the layer.
4. **Nothing in either lexicon exceeds the descriptor.** Every term found fits the descriptor + backbone and the [ADR-0027](./0027-affinal-traversal-ceiling-and-step-subsumption.md) two-`across` ceiling; several descriptor dimensions are strictly richer than either lexicon. **No engine change is needed for phrasing**, now or for the next language.

## Decision

### The phrasing layer is consumer-side TypeScript, in the preview package

Phrasing lives at `packages/preview/src/phrasing/` — a pure module inside `@kullang/preview`, under a standing rule of **no DOM imports**, so extracting it to its own package is mechanical if a second surface ever wants it.

This follows ADR-0025's line directly: phrasing is the UX of querying, not kinship correctness. It is also the only home with a consumer today — the map has ruled **CLI human-readable query output** and a **standalone web app** out of scope, so a Rust home would be built entirely for speculative reach while putting every term typo behind a Rust release and forcing phrased strings onto the LSP wire.

A Rust sibling crate (`kul-terms`) was the serious alternative: it would earn `insta` snapshot tests next to the descriptor derivations they key on, which is this repo's strongest testing culture. It is rejected on scope, not on principle — if `kul query` ever grows human output, extracting a language-neutral core is a re-implementation in Rust, and that cost is accepted knowingly.

### The phrasing key: normalized dimensions plus six derived facets

Phrasing is a function of a **phrasing key**, computed by the consumer from one `RelationshipDescriptor`. It is the descriptor's normalized dimensions — `classification` (with its materialized `cousinDegree` / `removed`), `edgeNature`, `affinity`, `sharing`, `side`, `seniority`, `apexSeniority`, `egoGender`, `alterGender` — plus six facets derived mechanically from the path backbone:

| Facet | Value | Derivation |
| --- | --- | --- |
| `acrossCount` | `0 \| 1 \| 2` | number of `across` hops (bounded by ADR-0027's ceiling) |
| `acrossAtStart` | boolean | the path's **first** hop is `across` — the relationship runs through ego's own spouse |
| `acrossAtEnd` | boolean | the path's **last** hop is `across` — the alter is someone's spouse |
| `spouseGender` | `male \| female \| other \| notApplicable` | gender of the person landed on by a path-initial `across` |
| `linkGender` | `male \| female \| other \| notApplicable` | gender of the person landed on by the **first `down` hop** |
| `endedMarriage` | boolean | any `across` hop on the path carries `status: ended` |

All six are pure functions of the hop sequence, which ADR-0026 guarantees is lossless and carries per-hop gender. **No engine change, no new wire field** — this is exactly the escape hatch the backbone was designed to be.

The set is closed and validated, not open-ended: every ⚠-marked row in the inventory — every term the normalized fields could not select — is selected by these six. Worked against the collision class of finding 2, where all six terms share one normalized cell:

| Term | Path | Discriminating facets |
| --- | --- | --- |
| *sāḷo* / *sāḷī* | `across·up·down` | `acrossAtStart`, `spouseGender = female` |
| *jeṭh* / *diyar* | `across·up·down` | `acrossAtStart`, `spouseGender = male`, `apexSeniority` |
| *naṇand* | `across·up·down` | `acrossAtStart`, `spouseGender = male`, `alterGender = female` |
| *bhābhī* | `up·down·across` | `acrossAtEnd`, `linkGender = male` |
| *banevī* | `up·down·across` | `acrossAtEnd`, `linkGender = female` |
| *vevāī* / *vevāṇ* | `down·across·up` | neither `acrossAtStart` nor `acrossAtEnd` |

The same six resolve *sāḍhu* vs *naṇdoī* vs *jeṭhāṇī* (`spouseGender` × `alterGender` × `apexSeniority`), spouse vs *savat* (`acrossCount`), *sāsu* vs *savkī mā* (`acrossAtStart` — and `affinity` already encodes the ancestor-position rule), *pautra* vs *dohitra*, *bhatrījo* vs *bhāṇej*, *pitrāī* vs *masiyāī*, and *jamāī* vs *vahu* (all `linkGender`).

Because `apexSeniority`, `sharing` and the `side = both` couple-apex refinement are properties of the graph *around* the path rather than of the hop sequence, they are the three the consumer cannot re-derive. It never has to for a whole-path descriptor — the engine supplies them. See "sub-path derivation" below for the one place this matters.

### A language pack is data: a facet-match table, most-specific-wins

> **Refined by [ADR-0039](./0039-phrasing-lookup-mechanics.md) (#298)** — the tie rule below is sharpened to *disagreement*: entries tied for maximum specificity that yield the **same** term are legitimate, and both packs need them (English writes *brother-in-law* twice, keying each end of the path, so the third shape in that cell stays unnamed; Gujarati's *savkā* many-to-one lands the same way). Only tied entries yielding **different** terms are a defect. ADR-0039 also pins that an entry matches by equality only — a numeric floor lives in an affix rule, never on an entry.

An entry is a record — a **partial match over the phrasing key** plus the term it yields. An omitted facet is a wildcard, so a coarse entry (`{cousinDegree: 1, removed: 0} → bhāī/bahen`) and a specific one (`… + side: maternal, linkGender: female → masiyāī bhāī`) legitimately coexist.

- **Precedence is by specificity: the entry keying the most facets wins.** Declaration order carries no meaning.
- **An exact tie is a conflict, not a coin toss.** Two entries matching the same key with the same facet count is a defect in the pack, caught by a test that walks a bounded enumeration of the key space (generations, `cousinDegree` and `removed` bounded at 4 — beyond that the fallback fires anyway). Resolving ties by declaration order would make specificity implicit, which is the trap inventory finding 6 names.
- **Adding a language is adding one module of records and zero lines of logic.** This is the additivity promise ADR-0026 designed a maximally-discriminating descriptor to keep; a per-language code hook would end it the day it were added.

Many-to-one collapses are a pack's own business: colloquial Gujarati conflates the descriptor's `affinity: step` and `sharing: half` under *savkā*, so the `gu` pack simply writes both entries to the same term. The shared layer never collapses dimensions on a pack's behalf.

### Productive morphology is data too

English's `great-great-grandmother` is unbounded and Gujarati's `par-` is productive for exactly one generation, so a flat table cannot cover the lineal spine. Packs therefore also declare **affix rules**, one vocabulary covering every case found:

```
{ when: <partial key match, with optional numeric threshold>,
  affix, position: "prefix" | "suffix", repeatPer?: <numeric dimension>, cap?: number }
```

- English: `grand-` at `generations = 2`; `great-` repeating per generation from 3.
- Gujarati: `par-` at `generations = 3`, no repeat, `cap: 3` — generation 4 and beyond falls through to the fallback, which is what speakers actually do.
- Ended marriages (below) ride the same vocabulary with a non-numeric `when`.

Seniority modifiers (*moṭā* / *nānā bhāī*) need no rule — they are ordinary entries keying `seniority`.

### Never guess: an unknown facet disqualifies the entry that keys it

**An entry is ineligible if any facet it keys is `unknown` on the key.** Lookup falls through to the unmarked, less specific entry.

So `seniority: unknown` yields *bhāī*, never a guessed *moṭā bhāī*; `apexSeniority: unknown` yields *kākā*, never *moṭā bāpā*. This is ADR-0026's honesty principle ("`unknown` and `notApplicable` are explicit, and never conflated") and [#277](https://github.com/YashBhalodi/kul/issues/277)'s certain-by-default filtering, carried into terminology: the layer never asserts a distinction the data does not support. `notApplicable` is a *value* and matches normally — it is a fact about the path shape, not a gap.

### The fallback is recursive prefix lexicalization over the backbone

> **Refined by [ADR-0039](./0039-phrasing-lookup-mechanics.md) (#298)** — "recursive" overstates it, and deliberately so in the implementation: **exactly one** prefix is lexicalized and the tail is spelled hop by hop, never re-read as a relationship of its own (an `up·down` tail is "X's mother's son", not "X's brother"). Re-lexicalizing a tail would phrase it relative to a second ego, which the descriptor's ego-relative facets cannot support. Where a flat tail names the *wrong* relationship, the fix is a pack entry.

When no entry matches the whole path, the layer walks prefixes from longest to shortest, deriving a sub-path key for each and looking it up in the **same table**. The first hit becomes the lexicalized head; the remaining hops render as a genitive chain. *māmā-no dīkro*, not *mātā-no bhāī-no dīkro*; "your grandmother's cousin", not a four-link walk.

There is therefore **no second lexicon** — the fallback consumes the pack it already has. Each pack adds only a hop lexicon (gendered nouns for `up` / `down` / `across`) and a genitive join pattern. The join is per-language morphology: English joins left-to-right with `'s`; Gujarati's possessive particle agrees with the *following* noun, so the pattern selects `-no` / `-nī` / `-nũ` by the head noun's gender.

**Sub-path derivation marks graph-dependent facets `unknown`.** `sharing`, `apexSeniority` and the `side = both` refinement need graph context a hop sequence does not carry, so a sub-path key sets them `unknown` rather than computing a plausible wrong value — and the never-guess rule then makes that automatically safe, disqualifying exactly the entries that would have been wrong. The ambiguous region for `side` is narrow (an initial ascent of exactly one `up` hop: siblings, nephews, and their affinal variants) and the inventory confirms **no term in either language keys `side` there** — §3, §6 and §7 each note it independently.

Phrases are **bare**: "mother's brother", not "your mother's brother". Ego-relative framing is chrome copy, so it changes with the surface without touching a language pack.

### Locale is a toggle in the preview chrome, in webview-local state

The reader picks the language from a control in the preview, not from a setting and not from the document. A toggle makes the compare gesture cheap — flipping one English "brother-in-law" against the six Gujarati terms it merges is genuinely useful, and it keeps [#276](https://github.com/YashBhalodi/kul/issues/276)'s single-language-at-a-time phrasing intact rather than doubling every label.

The choice persists in **webview-local state** (`getState` / `setState`), behind a small store interface inside the preview — in-memory by default, VSCode-backed in `adapter-vscode.ts`. `HostAdapter` is untouched, the preview stays host-agnostic, and the store can graduate to a contributed setting additively if per-panel turns out to be the wrong grain.

A `language:` field in `kul.yml` was rejected: `kul.yml` is normative and spec-governed, so it would bind every third-party Kul consumer for the sake of a preview UX feature. It remains the right shape only if phrasing ever leaves the preview.

### The result is `{ text, kind, hopCount }`

`kind` is `"lexical"` (from the table) or `"composed"` (from the fallback), and `hopCount` is the length of a composed tail. The chrome can then treat a crisp *kākā* and a five-hop chain differently in the same slot — [#276](https://github.com/YashBhalodi/kul/issues/276)'s docked whisper pill and [#281](https://github.com/YashBhalodi/kul/issues/281)'s kin list both have to hold either — without string-sniffing for apostrophes or sizing every label for the worst case.

### Three terminology policies

The inventory surfaced these and deliberately left them here.

- **Ended marriages phrase marked where the language has an affix, unmarked where it does not.** English renders "former mother-in-law" via an affix rule keyed on `endedMarriage`; Gujarati, which does not lexicalize this, renders *sāsu* and leaves disclosure to the chrome (the backbone carries `status` and `endReason`). The engine's report-and-tag stance hands phrasing the fact; asserting a live in-law tie that ended is the same over-claim `certain` mode refuses.
- **`edgeNature: adoptive` renders unmarked.** An adoptive mother is "mother". This matches the toolchain's adoption-is-kinship stance and ordinary usage in both languages — Gujarati's *dattak* is formal/legal register. The fact stays on the descriptor for any surface that wants to show it; a marked explicit-detail mode is deferred until something asks for it, and is additive when it does.
- **`jamāī` / `vahu` key `linkGender`, not `alterGender`.** The terms mean "daughter's husband" and "son's wife", so they key the linking child. Under a same-sex marriage, alter-gender keying would render a son's husband as *jamāī* — wrong, not merely coarse — and the same facet already selects the neighbouring *pautra* / *dohitra* and *bhatrījo* / *bhāṇej* families.

## Consequences

- **The engine is untouched, and stays untouched for the next language.** Phrasing reads a descriptor that already exists; adding a language adds a data module. ADR-0026's additivity promise is now structurally true rather than merely intended.
- **The query transport must carry full descriptors, including the backbone.** Phrasing is consumer-side and its fallback walks hops, so [#280](https://github.com/YashBhalodi/kul/issues/280) cannot ship phrased strings or a backbone-stripped descriptor. This ADR constrains that ticket.
- **Gujarati output needs `lang` markup.** `packages/preview/src/html.ts` hardcodes `<html lang="en">`; Gujarati phrases need per-element `lang="gu"` for correct font selection and screen-reader pronunciation. This is a correctness item for [#284](https://github.com/YashBhalodi/kul/issues/284), not a styling preference.
- **The language toggle is new chrome** — it needs `--kul-*` tokens and a keyboard path like the rest, also [#284](https://github.com/YashBhalodi/kul/issues/284).
- **Term data is testable as data.** A pack is records, so coverage ("every bounded key resolves to a term or a well-formed chain") and conflict-freedom are table-driven vitest suites, not hand-written cases per term.
- **[`docs/kinship-term-inventory.md`](../kinship-term-inventory.md) can now graduate.** Its findings 1, 2, 3, 5, 6 and 7 are recorded above; when the epic ships the `en` / `gu` packs, the inventory is deleted per its stated lifecycle.

## Anti-suggestions (do not re-propose)

- **"Put the term tables in `kul-core` so `kul query` gets human output for free."** ADR-0026 forbids the engine emitting a word, CLI phrasing is explicitly out of scope for this map, and it would put every term typo behind a Rust release and phrased strings on the LSP wire. A language-neutral Rust core is a deliberate future re-implementation, not a thing to retrofit now.
- **"Key terms on the normalized descriptor fields only — the descriptor is maximally discriminating."** It is, but only *with* its backbone. Six Gujarati terms share one normalized cell, and four more term families split on hop genders. The inventory disproves this empirically; the six derived facets are the answer.
- **"Let a language pack export a function for the awkward cases."** The day a hook exists, "adding a language is one additive entry" is no longer true, packs stop being reviewable as data, and specificity goes back to being implicit in code order. Awkward cases extend the affix-rule vocabulary instead.
- **"Resolve entry conflicts by declaration order."** That makes precedence invisible and order-fragile. Specificity is counted; ties on *different* terms are a defect a test catches ([ADR-0039](./0039-phrasing-lookup-mechanics.md)), and even a defective pack resolves deterministically without consulting order.
- **"Render the composed fallback as a flat hop walk from ego."** It discards a lexicalized term the language has and the reader prefers — "your mother's brother's son" where every speaker says *māmā-no dīkro*. The fallback reuses the table by design.
- **"Guess the unmarked-versus-marked term when the deciding facet is `unknown`."** Defaulting `apexSeniority` so *kākā* always beats *moṭā bāpā* invents a fact about someone's family. Unknown disqualifies; the unmarked term is the honest answer.
- **"Add a `language:` field to `kul.yml` so a family's tree reads in its own language."** Correct instinct, wrong lever — `kul.yml` is normative and binds every third-party consumer. Revisit only if phrasing leaves the preview.
- **"Return a plain string from the phrasing function."** Then the chrome cannot tell *kākā* from a five-hop chain and either sizes everything for the worst case or overflows. `kind` and `hopCount` cost three fields and are unrecoverable later without changing the signature.
- **"Collapse `step` and `half` in the shared layer, since Gujarati merges them."** That is one pack's many-to-one mapping, written as two entries pointing at *savkā*. English composes the two prefixes freely, which is exactly why ADR-0026 split the dimensions.

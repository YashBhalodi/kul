# ADR 0046 — The mode boundary's render paths, and what the retired documents leave behind

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

The kinship-query epic ([#296](https://github.com/YashBhalodi/kul/issues/296)) closes with
[#304](https://github.com/YashBhalodi/kul/issues/304), which wires the one rule that touches every
other slice and then retires the epic's transient documents.

The rule itself is not new. **An edit ends query mode** is
[ADR-0035](./0035-detail-surface-one-selection-one-batched-lookup.md)'s decision of record: a render
clears the selection, its panel, any kin paint and the filter; the tree returns to plain and editor
sync resumes on its own. Nine slices built the surfaces; this one composes them. Two of them left
the boundary explicitly deferred —
[ADR-0042](./0042-the-selection-seam-and-occlusion-aware-centring.md) named `clearSelection()`
rather than `clear()` so the narrow name could not quietly encode *query mode == a selection
exists*, and said in as many words that this slice "does not get one call for free"; and
[ADR-0045](./0045-the-filter-bars-three-states-derived-reasons-and-one-class-per-painter.md) built
the filter bar with the post-render hook re-*asking* rather than clearing, "because a slice that
only builds the filter has no standing to decide when query mode ends".

Composing them raised three things none of those ADRs settles, and each one reads as an arbitrary
choice unless it is written down.

1. **Is "renders only ever come from document changes" true?** The whole design rests on it: it is
   what makes the rule crisp rather than surprising, and ADR-0035 states it as a fact. Nobody had
   checked it against `editor/vscode/src/extension.ts`.
2. **What does deleting `docs/kinship-term-inventory.md` discard?** ADR-0044 flagged that the
   inventory is *not* exhausted — roughly a dozen terms never became `gu` pack entries — and said
   in as many words that #304 should know before deleting it. Deleting it is therefore a decision,
   not a clean-up.
3. **What do `docs/query-path-measurements.md` and `docs/prd/0006-preview-kinship-query-ux.md`
   still carry that a live ADR depends on?** The PRD lifecycle
   ([`docs/prd/README.md`](../prd/README.md)) requires the lift *before* the delete, and the same
   discipline applies to the other two transient assets.

## Decision

### Three render paths exist; one is an edit, and neither of the others can surprise a reader

The claim was checked rather than trusted, and it is **not literally true**. `refreshPreview` — the
only producer of a `render` message — has four callers in `extension.ts`:

| Trigger | A document change? | What query state is on screen |
| --- | --- | --- |
| `onDidChangeTextDocument`, debounced 300 ms | **yes** | whatever the reader built — this is the rule's case |
| `showPreview`'s first `refreshPreview`, at panel creation | no | none: the webview has only just mounted |
| `previewPanel.onDidChangeViewState`, hidden → visible | no | **none, and this is why** |
| `showPreview` re-invoked on a live panel (`reveal` + refresh) | no | whatever the reader built |

The third row is the interesting one, and it holds for a reason that is recorded in the code rather
than assumed: the panel is created **without `retainContextWhenHidden`**, so VSCode destroys the
webview's DOM and JS context when its tab moves to the background. The self-heal render exists
*because* of that — [#206](https://github.com/YashBhalodi/kul/issues/206) added it after the panel
came back blank — so by the time it fires, `mountPreview` has already run again on a fresh document
and there is no selection, no kin paint and no filter left to clear. The rule is a no-op there, not
a violation.

The fourth row is a reader **invoking a command by name**. *Kul: Show Preview* re-rendering the
preview is the gesture's whole meaning; a reader who runs it is not surprised that the picture is
rebuilt.

**So the rule stands, and its justification is narrower than ADR-0035's phrasing.** The honest
statement is not "renders only ever come from document changes" but **"every render that can reach
live query state is either an edit or an explicit command"** — which is what "no class of repaint
clears a selection surprisingly" was reaching for. Recorded here rather than quietly weakened,
because the day someone adds `retainContextWhenHidden: true` (a plausible optimisation — it is
cheaper than re-rendering) the third row *becomes* a violation, and this table is where they will
find out.

### The boundary is exactly two calls, and the lens is not a third

`QuerySurface.repaintQueryChrome()` becomes **`endQueryMode()`**, composed of `clearSelection()`
and `clearFilter()` and nothing else. Both are idempotent and neither depends on the other having
run, so the order is readability: whichever lifts the last sync-suspension reason is the one that
replays the held editor highlight.

ADR-0042 expected three parts — "`clearSelection()`, the kin paint and the filter". It is two,
because the kin paint is anchored to the selection: the surface's own selection subscriber runs
`resetKin`, which drops the answer, its paint, its dim source and its toast. The list's open state
survives, which is right and is the same thing that happens on Esc.

The hover lens is deliberately **not** a third call. It subscribes to the selection seam itself and
dismisses when the selection changes (ADR-0044), so it comes down on the same path Esc and a canvas
click already take. An explicit `lens.dismiss()` inside the boundary would be unreachable — the lens
only reads while a person is selected — and a call that cannot fire is a call whose docstring will
eventually lie about what takes the pill down.

**Esc runs the same function.** ADR-0045 gave Esc the same two clears "as a keyboard exit rather
than a render", scoped to whatever holds sync down. There is now no difference to preserve: query
mode has two exits and one behaviour, so they are one function and one suite. The `suspensions.size`
guard stays, because Esc must not be swallowed when there is no query state to end.

The hook is renamed rather than kept: `repaintQueryChrome` names the gesture that used to be
underneath it, and a hook whose name says *repaint* is an invitation to put repainting back.

### `FilterBar.repaint()` goes with the re-ask

It had exactly one production caller and the mode boundary is why that caller is gone. Keeping a
documented member reading "Called after a render swapped the SVG out. It re-*asks* rather than
re-applying the last answer" would be prose contradicted by the code in the same commit — the
failure this epic has paid for repeatedly. Its module suites drive re-evaluation through the
sentence instead, which is the only way a reader can, and one suite goes with it: *"discards a
render's re-ask that a chip edit has already superseded"* tested a race that no longer exists. The
generation guard it shared is still held by the suite beside it.

`FilterBar.reset()` absorbs the docstring: three callers — Esc, the reader clearing the last chip,
and a render — with **one** meaning between them.

### Deleting the term inventory discards sixteen register variants, and here they are

ADR-0044 named twelve. The list was checked against the file and against
`packages/preview/src/phrasing/packs/gu.ts` term by term, mechanically: every Latin form in the
inventory's term columns, matched against every string in the pack. **All twelve are correct** —
none of them is in the pack, so the `sāḍhu`-shaped false entry the reviewer caught in a similar
list does not recur here (`sāḍhu` *is* in the pack, and ADR-0044 rightly does not list it). The
sweep turned up **four more**, which is why the appendix below is the list of record rather than
that bullet.

The call: **lift them into this ADR as register variants deliberately not lexicalized, with the
term each one lost to.** A one-term-per-key phrasing layer has no slot for them and
[ADR-0041](./0041-the-gu-pack-and-the-locale-toggle.md) decided the shipped pack *becomes* the
record — but "why doesn't Kul say *bā*?" is an obvious future question, and an answer that exists
only in a deleted file is not an answer. This is knowledge worth keeping in prose even though it
can never be data.

### What survives the other two deletions

**`docs/query-path-measurements.md`.** Every number a live ADR leans on is already quoted inline
where it is used — ADR-0034's ceiling multipliers and the 32 ms `queryResolve`, ADR-0037's ≈63 ms
and ≈13 ms, ADR-0043's 16.6 ms datapoint and Finding 4's arithmetic, ADR-0044's 32 ms and its
frame-rate consequence, ADR-0045's ≈13.7 ms. What is *not* recorded anywhere else is the
**method and its caveats**, and `crates/kul-core/tests/perf.rs` cites the file twice for exactly
that — "the floors-not-p95s convention" and "Findings 3 and 5". So ADR-0034's measured-note is
widened to carry the ceiling table, the method in a sentence, the module-load figure and the
caveats, and the two comments in `perf.rs` are repointed at it. Finding 5's flatness needed no lift
at all: it is already a standing test
(`batched_detail_cost_is_flat_in_the_number_of_targets`), which is precisely what the file's own
lifecycle note said should happen to it.

**`docs/prd/0006-preview-kinship-query-ux.md`.** Read end to end against ADR-0033…ADR-0045. It
decides nothing the ADRs do not, which is what it said of itself at the top and what the slices
bore out: every implementation-decision bullet traces to an ADR, and the two places #303 corrected
(`:150`'s tally illustration, `:197`'s deferred filter clear) are both resolved — the first by
ADR-0045's disjoint-groups rule, the second by this ADR making it true. Its **user stories** are
the only content with no ADR home, and they are requirements rather than decisions: the surfaces
that answer them shipped, `CHANGELOG.md` describes them in reader-facing words, and `CONTEXT.md`
carries the vocabulary. Nothing further is lifted.

## Consequences

- **`mount.ts` still has exactly one post-render hook**, and the whole rule lives behind it. A
  future surface that needs to let go on an edit registers inside `endQueryMode`, not as a second
  call in the mount.
- **Query mode now has a definition worth glossing**, and `CONTEXT.md` gains it. The epic's
  vocabulary list named it; no slice had a boundary to define it against until this one.
- **The accepted cost is unchanged and is now paid in full**: a one-character typo fix costs the
  reader their selection, their painted kin set and their filter. Refetching the bundle per render
  was measured as affordable (≈13 ms per debounced render) and rejected on principle rather than on
  cost — querying and authoring are separate activities, and no query artefact outlives the source
  it was computed from.
- **Sync is self-completing.** Both suspension reasons lift inside the boundary, so the held
  editor-sync highlight replays with no gesture. That is the whole content of PRD story 31, and it
  falls out of the reason set rather than needing a rule of its own.
- **The Explore-kin list's open/closed state survives**, exactly as it does on Esc and on a canvas
  click. Ending query mode is not a reset of the preview: the list is a property of the reader
  (ADR-0043) and the panel it lives in is gone anyway, so the next selection opens straight back
  onto it.
- **Three transient documents are gone and nothing points at them.** The references were in ten
  ADRs, two `perf.rs` comments, a dozen preview source comments and four preview test comments;
  each is repointed at the ADR, the issue or the `CONTEXT.md` entry that now carries the claim.
- **The `gu` pack is now the sole record of Gujarati kinship terminology in this repo**, with this
  ADR's appendix as the note of what it deliberately does not hold. A native reviewer wanting a
  variant back adds a pack entry; a reviewer wanting to know why it is absent reads the appendix.

## Anti-suggestions (do not re-propose)

- **"Keep the selection across a render and only clear the filter."** The two are one mode. A
  half-cleared boundary puts the reader in a state where sync is still suspended by a selection
  they did not re-make, against a picture computed from source they have since changed.
- **"Refetch the query bundle on every render instead — it is only ~13 ms."** Measured, affordable,
  and rejected in ADR-0035 on grounds that cost cannot address: the answers would be about the old
  source. The number was never the objection.
- **"Debounce or delay the clear so a fast typist keeps their selection."** The render is already
  debounced at 300 ms; a second delay makes the boundary a race the reader can feel and cannot
  predict.
- **"Set `retainContextWhenHidden: true` so the preview does not re-render on restore."** It would
  turn the hidden→visible render into a real violation of the rule — live query state, cleared by
  something that is not an edit. If it is ever wanted, the fix is to stop re-rendering on that
  transition (the render exists only to repopulate a destroyed context), not to accept the clear.
- **"Put the un-landed variants back in the pack as alternates."** A pack maps a key to *one* term;
  alternates need a register dimension in the phrasing key, which is a phrasing-layer redesign and
  not a data change. ADR-0033's most-specific-wins lookup has no tie-break that could choose between
  *bā* and *mā*, and ADR-0039 makes an exact tie a test failure.

## Appendix — Gujarati terms the inventory listed and the `gu` pack did not take

Sixteen, verified against `packages/preview/src/phrasing/packs/gu.ts` at the time this ADR landed.
Each row names what the pack says instead. They are **register and dialect variants**, not gaps: in
every case the pack has a term for the key, and the variant is another way the same relative is
addressed.

| Not in the pack | Inventory § | The pack's term | Why it lost |
| --- | --- | --- | --- |
| *bā* (બા) | §1 | *mā* (મા) | Register variant for "mother"; *mā* is the neutral one. |
| *mummy* | §1 | *mā* (મા) | English borrowing, common in speech; a Gujarati pack that emitted it would be phrasing in two languages at once. |
| *bāpuji* (બાપુજી) | §1 | *pappā* (પપ્પા) | Respectful/older register for "father". |
| *nānā-bāpā* | §1 | *nānā* (નાના) | Compound variant of the same term; the inventory's script column gives only નાના. |
| *nānī-mā* | §1 | *nānī* (નાની) | As above. |
| *ben* (બેન) | §3 | *bahen* (બહેન) | Spelling/pronunciation variant. The pack normalises all four gendered sibling forms onto *bahen*, and says so at its head. |
| *moṭā kākā* | §4 | *moṭā bāpā* (મોટા બાપા) | Regional variant for father's elder brother; both key `apexSeniority: elder`. |
| *fai* (ફઈ) | §4 | *foī* (ફોઈ) | Pronunciation variant for father's sister. |
| *bhāṇejo* (ભાણિયો) | §6 | *bhāṇej* (ભાણેજ) | Variant for sister's son. *bhāṇī* (sister's daughter) did land. |
| *var* (વર) | §7 | *pati* (પતિ) | Register variant for "husband"; *var* is also "bridegroom", which is why the formal term wins. |
| *dhaṇī* (ધણી) | §7 | *pati* (પતિ) | Colloquial, and carries a proprietary sense the pack has no way to disclose. |
| *śokya* (શોક્ય) | §7 | *savat* (સવત) | Variant for co-wife. |
| *putravadhū* (પુત્રવધૂ) | §7 | *vahu* (વહુ) | Sanskritic formal register for son's wife. |
| *der* (દેર) | §7 | *diyar* (દિયર) | Variant for husband's younger brother. (દેરાણી / *derāṇī*, his wife, **is** in the pack — the two spellings coexist in the source.) |
| *apar-mā* (અપરમા) | §8 | *savkī mā* (સાવકી મા) | Formal register for stepmother. |
| *dattak* (દત્તક) | §8 | *(nothing — unmarked)* | Formal/legal prefix for "adopted". The pack keys `edgeNature` **nowhere**, by ADR-0033 policy: an adoptive mother is *mā*, because that is how families speak. This is the one row that is a decision rather than a register choice. |

Three things the inventory lists that are **not** in this table, because the pack does produce them:

- ***par-dādā* / *par-dādī* / *par-nānā* / *par-nānī*** (§1) — composed by the pack's one affix
  rule (`par-`, `generations: 3`, `cap: 3`) over the four grandparent entries.
- ***savkā*** as a prefix (§8) — expressed as six explicit entries rather than a rule, because
  Gujarati conflates `affinity: step` with `sharing: half` under it and a rule cannot key two
  dimensions onto one form.
- ***māmā-no dīkro* / *foī-no dīkro*** (§5) — deliberately left to the genitive fallback, because a
  chain *is* the actual usage on those two cousin lines. Writing a coarse `cousinDegree: 1` entry
  would have swallowed all four lines into *bhāī*.

One narrower loss, recorded for completeness: §7 lists *vahu* (વહુ) as a variant for "wife" as well
as for "son's wife". The **word** is in the pack; that second **sense** is not, and *patnī* is the
only term for a spouse. The pack keys *jamāī* / *vahu* on the linking child's gender rather than the
alter's (ADR-0033), which is what makes the son's-wife reading the only one it can express.

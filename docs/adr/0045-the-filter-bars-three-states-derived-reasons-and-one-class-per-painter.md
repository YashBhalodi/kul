# ADR 0045 — The filter bar's three states: derived reasons, a partitioning tally, and one class per painter

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[#277](https://github.com/YashBhalodi/kul/issues/277) decided the attribute filter's UX across
eleven prototype rounds, and [PRD-0006](../prd/0006-preview-kinship-query-ux.md) restates it: a chip
sentence in stage chrome, conjunction-only with the `and` spelled out, certainty as a chip rather
than a toggle, three paint states on the tree, a disclosing tally, and composition with a painted
kin set. [ADR-0025](./0025-kinship-query-engine-contract-and-traversal.md) pinned the engine
contract the sentence spells, [ADR-0036](./0036-two-tier-theming-and-the-accessibility-non-goal.md)
reserved the amber the third state paints in,
[ADR-0038](./0038-tier-1-register-overlay-stack-and-the-reserved-carve-out.md) gave the bar its
region, [ADR-0043](./0043-explore-kin-thirteen-query-values-and-what-a-row-costs.md) built the dim
registry that owns the receding, and [ADR-0044](./0044-the-hover-lens-transliteration-as-data-and-the-unspent-hue.md)
extracted the docked-tag grammar the reason whispers in.

Building [#303](https://github.com/YashBhalodi/kul/issues/303) against all of that turned up six
questions none of them answers, each with an answer that reads as an arbitrary style choice unless
it is written down.

1. **Where does a can't-say *reason* come from?** #277 requires the whisper — *"can't say — family
   not recorded"*, *"~1942 is approximate (±5y) — straddles born < 1945"* — and the engine has no
   operation that produces one. `runQuery` answers in **sets**.
2. **What do the tally's numbers mean?** #277's illustrative string is *"3 of 10 · 7 dimmed · 1
   can't say"*, whose three numbers do not add to ten under any reading where all three name
   disjoint groups.
3. **Whose teal is a filter match?** #277 says matches take "the existing teal result glow", and
   kin paint already owns a class that paints it. The two surfaces co-exist by design: filtering
   *within* a painted kin set is one of the ticket's own requirements.
4. **Is an unjudgeable person dimmed?** They are excluded in `certain` mode, and everything else
   excluded recedes.
5. **What happens when a card would carry two whispers?** The [hover lens](./0044-the-hover-lens-transliteration-as-data-and-the-unspent-hue.md)
   docks a relation pill under the card the pointer is on. So does this — and ADR-0044 says in as
   many words that both the hover plumbing and the shared-anchor placement are this slice's call.
6. **Where does the scope's population come from?** The tally's denominator and the dim's
   complement both need the set of persons the filter ran over, and a `Query` answers only who
   matched.

## Decision

### The reason is derived from the record, and never re-decides a truth value

`filter-reason.ts` reads the person's `ExportedPerson` — through the batched detail lookup, the one
provenance path for person data ([ADR-0035](./0035-detail-surface-one-selection-one-batched-lookup.md))
— and names what about the *record* is unjudgeable-shaped: a field the predicate needs that the
author never recorded, or an interval wider than a day on either side of a comparison (the recorded
value's, or the literal the reader typed).

The line it does not cross is the load-bearing part. **The engine has already ruled the person
unjudgeable**; this module never evaluates a predicate, and both of its tests are structural — is
the field there, and is either interval a point. Every route to `unknown` in `filter.rs` passes
through one of the two, so the candidates it finds always contain the real culprit. With exactly one
candidate the reason names the comparison it straddles; with several it names all of them and claims
none. With none — which the engine's verdict says cannot happen, and which a record shape nobody
anticipated could still produce — it degrades to a bare "can't say" rather than inventing an
explanation. Presence predicates are excluded by construction: they are decidable on any record.

**Rejected: a new WASM verb returning per-person predicate outcomes.** It is the only way to get the
engine's own attribution, and it costs a `kul-core` shape, a `kul-wasm` verb, a committed type
snapshot, and a release — for a string. The epic's Rust critical path was the batched detail
operation and it is spent.

**Rejected: re-implementing the three-valued predicates in TypeScript.** `kul-core`'s
`filter__*.snap` suites own predicate correctness and PRD-0006 says so in as many words. A second
evaluator is a second answer that can disagree with the paint beside it, and the disagreement would
surface as a card painted amber above a whisper explaining why it is teal.

### The tally names three disjoint groups, and the mode moves only which are shown

`matched + cantSay + dimmed === total`, in either certainty mode. What the mode changes is `shown` —
`certain` shows the matched, `certain + can't say` shows the matched plus the unjudgeable — and
never a count. So certain mode reads *"3 of 10 · 6 dimmed · 1 can't say"* and the same filter under
the other mode reads *"4 of 10 · 6 dimmed · 1 can't say, included"*: the disclosure is the same
number whichever way the reader is looking, which is the whole point of disclosing it.

This is a deliberate correction to #277's illustrative *"7 dimmed"*, which counted the unjudgeable
person inside the dimmed group. Keeping that wording would have required either a fourth reading of
"dimmed" (not-shown, rather than receded) or dimming the amber card — see below. The numbers now
describe exactly what is on screen.

### An unjudgeable person is not dimmed

Amber, dashed, with a `?` riding every card they own, at full opacity. The reason is the honesty
stance, not the aesthetics: an unjudgeable person receding like a non-match is precisely the silent
drop the disclosure exists to prevent (PRD-0006 story 23), and a card at 30% opacity does not read
as something to hover for an explanation.

The third state is distinguished on **two** dimensions rather than one — a different reserved hue
*and* a dashed line *and* a glyph — because a hue difference alone is exactly what a high-contrast
theme flattens.

### Each painter owns its own match class; only the dim is shared

The filter's match glow is `kul-filter-match`, aliasing the same reserved teal that kin paint's
class aliases, rather than reusing that class.

The dim registry exists because two paint sources sharing one class means two independent strippers,
whichever repaints last silently undoing the other. That failure applies to a match class exactly as
it does to the dim — and a filter running inside a painted kin set puts both on screen at once. The
dim is shared because it is genuinely **union-semantic**: a person receded by any source is receded,
full stop. "Matched the filter" and "is in this kin set" are two different claims that happen to
share a hue, so they get a class each and each surface strips only its own.

**Rejected: a second registry, for results.** The union semantics that justify one owner for the dim
do not hold for a match: there is no meaningful union of two different claims, and an outline is not
an `opacity` — it does not compound when two rules apply, so the hazard the registry solves does not
exist here.

### The can't-say whisper stands down while a person is selected

[ADR-0044](./0044-the-hover-lens-transliteration-as-data-and-the-unspent-hue.md) hands two things to
this slice by name: hover input, because `handleCanvasHover` was wired straight to the lens and a
fan-out with one consumer would have been speculative generality; and **placement when two tags share
an anchor**. This is the answer to both, and one decision covers them.

`handleCanvasHover` becomes a fan-out in `createQuerySurface`. A person selection is what arms the
relationship whisper, and its pill docks under exactly the card a can't-say reason would, so while a
person is selected the reason is **not offered** — and it is *told* to stand down (passed `null`)
rather than merely not called, so a reason already open comes off screen the moment a selection
arms the lens. One gesture gets one answer. Neither whisper knows about the other; the composition
that owns both decides.

This also matches the prototype's own framing, which introduced the reason as what happens "with no
selection".

**Rejected: stacking the two tags.** Two boxes under one card, about two different questions, with
position as the only thing saying which is which — and position is the docked-tag grammar's whole
mechanism for saying what a whisper is *about*.

**Rejected: suppressing the lens instead.** The lens is the affordance the whole transport was
arranged around, and a selection is a standing choice the reader made; a filter's ambient paint must
not take it away.

### The scope's population is supplied, not queried

For `Everyone` it is the set of `data-person-id`s the picture draws — every person owns a canonical
card, so the picture is the project. For a kin scope it is the members `createQuerySurface` is
already holding — the same ones the kin teal was painted from — handed over through a `kinScope()`
callback the bar asks on every draw.

That keeps a filter change at **two** engine calls (one per certainty mode) rather than three, which
matters: [ADR-0029](./0029-query-engine-performance-posture.md) budgets 50 ms for one click and each
call re-checks the project at ≈13.7 ms on a 10,000-person corpus. A third call for a denominator
would spend a quarter of the budget on a number already implied.

A callback rather than a shared store because the painted set changes without the filter being told,
and a store would be a second seam for a question one caller can answer. Composition itself is the
engine's: the scope chip switches the `Query`'s `source`, and `kinOf` + `where` is evaluated as one
query — the surface never intersects two answers.

The scope's *name* is the reader's, not the engine's: the row label the kin list already shows,
possessed by the same anchor name the empty-kin toast uses, so one painted set is named one way
wherever it appears.

### A render re-asks; it does not replay

`repaintQueryChrome` is the single post-render hook, and the filter re-evaluates from inside it
rather than re-applying its last answer. A render carries a new project snapshot, so replaying would
paint a verdict about source that is no longer on screen. Whether a render should instead *end* query
mode is [#304](https://github.com/YashBhalodi/kul/issues/304)'s decision; `QuerySurface.clearFilter()`
is the call it composes that from, and it is separate from `clearSelection()` because a filter and a
selection are independent — either can exist without the other.

### Two smaller calls, recorded so they are not re-litigated

- **`id` is not a filterable field.** The engine accepts it; it is a person's *address*, not
  something an author wrote about them, and every other field in the menu is.
- **A half-written chip asks nothing.** `+` creates a chip and the editor fills it in, so an empty
  value is a normal intermediate state. `family = ` is not the question "who has an empty family
  name"; it is a question nobody has finished asking, and answering it would paint a confident zero.

## Consequences

- **The whisper costs one batched lookup per unjudgeable person, once per answer.** Cached while the
  answer stands, dropped when the filter or the project changes, and guarded against the pointer
  firing per pixel over one card.
- **The reason can name more than one candidate.** On a two-condition filter where both conditions
  are unjudgeable-shaped for one person, the whisper names both. That is less crisp than the
  prototype's single clause and is the honest reading of what the surface actually knows.
- **The tally's wording differs from #277's example string.** Deliberately, and the arithmetic is
  asserted at the module seam.
- **Filter paint and kin paint can both be on screen, in the same teal.** By decision — #277 says
  matches take *the existing* result glow — and the tally is what distinguishes "3 of 4 in
  Giuseppe's descendants" from a bare kin set.
- **The kin scope is fed from the answer already on screen.** Painting a kin set gives the scope
  chip a second entry; dropping it takes the chip back to `Everyone`. A host that supplies no flow
  region gets no filter bar at all, and one that paints no kin sets gets a chip with one entry and a
  note saying why.
- **The last reserved name is spent, and the carve-out is closed.** All five names ADR-0036 reserved
  now have consumers — the selection violet (#300), the result teal and the dim alpha (#301), the
  resolution-path sky (#302), and the can't-say amber here — so `RESERVED_PENDING_CONSUMERS` is
  empty and the ledger ADR-0038 asked the lint to keep has nothing left to track. Its converse
  assertion, which forces a reserved token out of the list the day it gains a consumer, is what
  emptied it. A name added back is a claim that a *new* reservation was decided, which is an
  ADR-0036 amendment rather than a slice's business.
- **Nothing in this slice touches `crates/`.** `runQuery` has been on the WASM surface since
  PRD-0005 and its wire types were mirrored by #301; only the verb on `QueryEngine`, the
  `PreviewHandle` and the mount's existing shared transport policy are new.
- **`queryKin` and `runQuery` stay two verbs over one evaluation path.** `kul-core`'s `kin_query` is
  a documented alias of `query_envelope`, so folding them would cost nothing at runtime — and would
  route an `allPersons` source through a verb whose own docstring says "one kin-set query". Both
  docstrings now say why they are apart.

## Anti-suggestions (do not re-propose)

- **"Ask the engine which predicate went unknown."** It is a new operation, a new wire shape, a
  committed type snapshot and a release, for a string the record already explains. Revisit only if
  the derived attribution is observed to be wrong, which would mean a route to `unknown` that is not
  a missing field or a wide interval.
- **"Port `filter.rs` to TypeScript so the preview can explain itself."** Two evaluators, one of
  which is not the one that painted the tree. PRD-0006 rules this out by name.
- **"Reuse kin paint's result class so there is one teal."** There is one *hue*; two owners of one
  class is the failure the dim registry was built to prevent, and both surfaces are on screen
  together by design.
- **"Dim the can't-say cards so the tally can say '7 dimmed'."** That makes the unjudgeable read as
  a non-match, which is the exact outcome the whole disclosure exists to prevent.
- **"Add an OR chip behind an advanced mode."** OR is permanently out of engine scope (ADR-0025).
  There is no node in the filter's own model that could represent one, which is deliberate: the
  surface must not imply the engine can do something it never will.
- **"Add `sort` to the bar."** The tree keeps its canonical layout order and only a list can express
  an order. PRD-0006 puts a results list out of scope; adding sort means adding the list first.
- **"Hide non-matching people."** Filtering never hides a node or an edge. It punches holes in the
  canonical layout and leaves marriage stubs and birth edges running into empty space — the standing
  rule #277 generalised past this feature, asserted here across every filter state.

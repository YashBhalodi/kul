# ADR 0041 — What shipping a second language pack cost: the possessor's stem, transliteration, and a wall clock

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[ADR-0033](./0033-phrasing-layer-architecture.md) designed the phrasing layer around one
promise: **adding a language is one additive module of records and zero lines of
logic**. [ADR-0039](./0039-phrasing-lookup-mechanics.md) pinned the mechanics that promise
rests on. [#299](https://github.com/YashBhalodi/kul/issues/299) is the test — the `gu`
pack, the language the whole design was forced by.

**The promise held.** Seventy entries, one affix rule, a hop lexicon and a genitive
pattern; the diff under `packages/preview/src/phrasing/` is `packs/gu.ts` plus one line in
`packs/index.ts`. Nothing in `pack.ts`, `key.ts`, `lexicalize.ts` or `phrase.ts` moved, and
the pack suites passed over `gu` without an assertion changing. The seven-way affinal
collision class, the `par-` cap, seniority-as-entries, the gender-agreeing genitive and the
*savkā* many-to-one collapse all expressed themselves in the vocabulary already there.

Three things it did **not** buy fell out of building it. Each is small; each would
otherwise be re-litigated, or worse, "fixed" by a change that ends the promise.

## Decision

### The genitive join agrees with the following noun and cannot inflect the possessor

`GenitivePattern`'s record form selects `નો` / `ની` / `નું` by the **possessed** noun's
gender, which is the agreement Gujarati actually has and the reason ADR-0033 gave the field
a record form at all. What it cannot express is the *other* half of the morphology: a
masculine `-o` possessor takes an oblique `-ā` before the particle, so careful Gujarati
writes *dīkrā-no dīkro* where this pack renders *dīkro-no dīkro*.

**This is accepted, not deferred to a fix.** The possessor's stem change is a function of
the possessor's own shape, so expressing it means either a second gendered lexicon of
oblique forms (doubling every hop noun and every term that can head a chain) or a
per-language morphology hook — which is exactly ADR-0033's "let a language pack export a
function for the awkward cases" anti-suggestion, the one whose arrival ends additivity.

The damage is bounded and the pack is written to bound it further: the hop lexicon uses the
formal register (*pitā* / *mātā*, not *bāp* / *mā*), whose stems are oblique-invariant, so
the artefact appears only where a `-o` noun is a possessor mid-chain. The canonical case
the inventory records — *māmā-no dīkro* — is unaffected, because *māmā* is invariant too.

### Term-level transliteration needs a field the pack type does not have, so it was not built

[#299](https://github.com/YashBhalodi/kul/issues/299) asks for "transliteration available on
hover". What shipped is the **language name**: the toggle's label is the endonym (ગુજરાતી),
and its `title` gives the transliterated name in English chrome copy — so a reader who does
not yet read the script knows what they are switching to.

Per-*term* transliteration — hovering *કાકા* to see *kākā* — was **not** built, and the
reason is the one this slice exists to police. It needs a second string per entry, which
means a field on `PackEntry`, a field on `Phrase`, and `lexicalize` carrying it through:
three logic changes to the module whose diff was supposed to be zero. The pure-data
alternative is worse — a parallel `gu-Latn` pack duplicating seventy entries with nothing
enforcing that the two agree.

It is recorded here rather than done, because the honest sequence is to decide whether a
romanized reading surface is worth a field on the pack type **before** adding one, not
during the slice that promised not to.

### A data-only pack says "render unmarked" by writing nothing

Two of ADR-0033's three terminology policies land in this pack as absences: nothing keys
`edgeNature` (an adoptive mother is *mā*), and there is no affix rule on `endedMarriage`
(a former mother-in-law is *sāsu*; Gujarati does not lexicalize the distinction and
disclosure is the chrome's job).

Stated because an absence reads as an oversight, and the ADR-0033 policies name English's
behaviour — *former mother-in-law* — which invites the mirror rule being "restored" here.
The `gu` fixtures assert both absences directly, so the policy has a failing test rather
than only a comment.

Its converse is also a pack decision worth naming: **no coarse `cousinDegree: 1` entry**.
One would collapse all four cousin lines into *bhāī* and swallow the two the language
genuinely leaves compositional — and the fallback already answers those exactly as the
inventory records them (*māmā-no dīkro*), off the *māmā* and *foī* entries it has.

### The enumeration widens itself for free; its wall clock does not

ADR-0039's self-widening enumeration worked as designed: `gu` is the first pack to key
`seniority` and `apexSeniority`, and the coverage scan grew from ≈22k descriptors to ≈219k
the moment it was registered, with no test edited. That is the mechanism paying out — a new
language inherits coverage over its own facets without anyone thinking about it.

It also pushed one suite past Vitest's 5 s default timeout. The ceiling moves in
`vitest.config.ts` rather than on the suite, so **the pack tests stay untouched by the packs
they scan** — which is the property "the `gu` PR touches no logic" is reviewed as. No
assertion, bound or enumeration changed.

This is the single place additivity was not literally free, and the cost is a config line,
not a design concession. A third language keying a facet neither pack does will widen it
again; if that ever stops being affordable, the fix is bounding the enumeration, not
narrowing what it reaches — ADR-0039's second invariant explains why a narrowing that drops
a value reports defective packs clean.

### The locale store is a second seam, not a widened `HostAdapter`

`LocaleStore` (`read` / `write`) sits beside `HostAdapter` rather than inside it. Two real
implementations exist — in-memory and VSCode `getState` / `setState` — so it is a genuine
seam by `CODING_STANDARDS.md`'s two-adapters test, and an embedding with no durable state
constructs neither and gets the default.

One mechanical consequence: `acquireVsCodeApi` may be called **once** per webview load, and
there are now two consumers, so the adapter caches the handle. This is the kind of thing
that fails only in a real webview, so it is under test.

## Consequences

- **The additivity promise is now evidenced, not asserted.** The `gu` pack's diff is data;
  the machinery slice's design survived contact with the language it was designed for.
- **A son's husband renders *vahu*.** *jamāī* / *vahu* key `linkGender`, so the son's spouse
  takes the son's-spouse term whatever their gender — the ADR-0033 policy, followed exactly.
  It is coarse where the language has no word, and it is never *jamāī*, which is the
  outcome that policy exists to prevent.
- **The husband's brother composes when `apexSeniority` is `unknown`.** Gujarati has no
  unmarked lexeme between *jeṭh* and *diyar*, so the never-guess rule hands the key to the
  fallback and it answers "father-in-law's son". Honest, and a place where the fallback is
  doing real work rather than covering a gap.
- **Chrome growth cost the theme contract nothing.** The toggle takes eleven tier-2 aliases
  and no tier-1 token, which is what ADR-0036's two tiers promised and ADR-0038's budget
  assertion enforces.
- **`html.ts` keeps `lang="en"`, now named.** `CHROME_LANG` makes the document's language a
  decision rather than a hardcode, so the next contributor does not "fix" it by retagging
  the document `gu` and mislabelling the English chrome around the phrase.

## Anti-suggestions (do not re-propose)

- **"Add an `oblique` form to the hop lexicon so *dīkrā-no* comes out right."** That is a
  second lexicon inside the pack for one language's case morphology, and the next language
  wants a different one. The join stays a template; the register the pack writes in is what
  keeps the artefact rare.
- **"Give `PackEntry` a `roman` field so terms transliterate on hover."** Three logic
  changes to the module whose whole point is having none. Decide whether a romanized
  reading surface is worth the field first — on its own ticket, with the chrome that would
  consume it.
- **"Ship a `gu-Latn` pack for the transliteration."** Seventy duplicated entries with
  nothing enforcing agreement, offered as a third language in a toggle that promises one
  language at a time.
- **"Add a coarse cousin entry so every first cousin gets a word."** Two of the four lines
  are genitive constructions *in the language*; a coarse entry answers *bhāī* to all four
  and retires a distinction that decides marriage eligibility.
- **"Narrow the coverage enumeration back down now that it is ten times bigger."** The size
  is the mechanism working. Bound it if it ever has to be bounded; do not make it reach
  fewer values, which is how a conflict scan starts reporting defective packs clean.
- **"Put the locale on `HostAdapter` — it is host state."** It is *preview* state that some
  hosts can persist. Widening the outbound seam to carry it makes every embedding implement
  a method most of them would stub.

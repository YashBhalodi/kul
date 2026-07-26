# ADR 0037 — Phrasing lookup mechanics: equality-only entries, refusing caps, derived orders

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[ADR-0033](./0033-phrasing-layer-architecture.md) decided the phrasing layer: a
consumer-side module at `packages/preview/src/phrasing/`, a facet key derived
from the [relationship descriptor](./0026-relationship-descriptor-and-path-identity.md)
plus six backbone facets, data-only language packs with most-specific-wins
precedence, affix rules for productive morphology, a never-guess rule, and a
fallback that lexicalizes backbone prefixes.

Building the machinery and the `en` pack ([#298](https://github.com/YashBhalodi/kul/issues/298))
forced a handful of decisions ADR-0033 left at the level of "a facet-match
table". Each is small; each is exactly the kind of thing a future contributor
would otherwise re-litigate or, worse, "simplify" in a way that quietly breaks
the additivity promise the `gu` pack ([#299](https://github.com/YashBhalodi/kul/issues/299))
is the test of. They are recorded here rather than as code comments.

## Decision

### An entry matches by equality only; numeric floors live in affix rules

A `PackEntry.when` is a `Partial<PhrasingKey>`: every present facet must be
**equal**, an omitted facet is a wildcard, and specificity is `Object.keys`
length. There is no `atLeast` / range / negation on an entry.

The pressure to relax this is real. English's uncle spine (*uncle*,
*great-uncle*, *great-great-uncle*) is `up ≥ 2, down = 1`, and the obvious move
is to wildcard `up` and let an affix rule carry the greats. That is wrong here
for a reason worth writing down: `up = 1, down = 1` is the **co-parent-in-law**
cell (`down·across·up`, a child's parent-in-law), which English does not name
at all. A wildcard entry reaches it and answers "uncle" to a question the
honest answer to is "son-in-law's father". So the spine is written out as
entries and stops where usage does.

Equality-only keeps three properties that all matter more than entry count:
specificity stays a plain count, an entry stays readable as one row of a table,
and conflict detection stays a set comparison. A pack that needs a threshold has
one place to put it — an affix rule — and that place is bounded by `cap`.

### `cap` refuses lexicalization; it does not merely drop the affix

When an affix rule's facets match but the governed numeric facet exceeds `cap`,
`lexicalize` returns `null` and the compositional fallback fires for that key.
It does **not** yield the bare term with no affix.

This is what ADR-0033's Gujarati example requires — `par-` caps at generation 3
and "generation 4 and beyond falls through to the fallback" — and it makes
`cap` a statement about *the language's ceiling for this region*, not about the
affix. The payoff composes with prefix lexicalization for free: a generation-4
ancestor in a `par-`-capped pack phrases as *par-dādā*'s father, because the
refused whole-path key falls back onto the longest prefix the cap still allows.

A `notApplicable` facet value is not a number and is never capped, so a rule
capped on `generations` still applies to collateral keys.

### Affix rules apply in a derived total order, never declaration order

Applicable rules sort by *(threshold ascending, thresholdless last) → governed
facet → specificity descending → affix*, and apply innermost-first. So `grand`
(floor 2) lands inside `great-` (floor 3), giving *great-great-grandmother*, and
a thresholdless rule like `former ` ends up outermost, giving *former
mother-in-law*. Declaration order is meaningless for affix rules exactly as it
is for entries — a pack is a set of records, and reordering the file must never
change a word.

Two rules that can fire on one key with the same sort key would be
order-dependent, so that is a pack defect, caught by the same suite that catches
entry ties.

### A specificity tie is a defect only when the terms differ

ADR-0033 calls an exact tie "a defect in the pack, not a coin toss". Refined:
the defect is **two or more entries tied for maximum specificity yielding
different terms**. Tied entries yielding the *same* term are legitimate, and the
`en` pack needs them immediately: English merges the spouse's sibling
(`acrossAtStart`) with the sibling's spouse (`acrossAtEnd`) as *brother-in-law*,
and a spouse's sibling's spouse matches both entries. Writing one wildcard entry
instead would swallow the third shape in that normalized cell — the co-parent-in-law
— which English does not name. Gujarati's *savkā* many-to-one collapse
(`affinity: step` and `sharing: half` at one term) lands the same way.

Runtime never coin-tosses either: `lexicalize` picks the lexicographically first
term among the winners, so behaviour is deterministic even in a pack that is
mid-defect, and the failure mode is a wrong word rather than a word that changes
when someone reorders the file.

### A sub-path key marks endpoint `seniority` unknown too

ADR-0033 lists three facets a sub-path key marks `unknown`: `sharing`,
`apexSeniority`, and the `side = both` refinement. Endpoint `seniority` belongs
in that list for the same reason — it is a comparison of two birth dates, and a
hop sequence carries ids and genders, not dates. The phrasing layer has no way
to know whether the person at the end of a prefix is older than ego.

Marking it `unknown` is the honest value, and the never-guess rule then does the
rest: a pack keying `seniority` (Gujarati's *moṭā bhāī*) falls through to the
unmarked term on a composed head, which is what a speaker does.

`side` keeps its routing derivation on a sub-path and goes `unknown` only where
the answer could have been `both` — an initial `up` hop immediately followed by
a `down` hop, which is precisely `junction.up == 1` in
`crates/kul-core/src/query/junction.rs`. Anything else is derivable from the
hops alone and is derived.

### `hopCount` counts the hops the phrase spells out

`hopCount` is `path.length − (length of the lexicalized prefix)`: zero for a
lexical phrase, the number of genitive links for a composed one, and the whole
backbone in the degenerate case where a pack lexicalizes nothing at all. So
chrome can size a slot from `hopCount` alone without parsing the text.

### The wire types are mirrored, not imported, and pinned by a text lint

`@kullang/preview` does not depend on `@kullang/wasm`, and the phrasing module
imports nothing at all (that is what makes extraction mechanical). Its
descriptor types are therefore a verbatim copy of the committed tsify output in
`crates/kul-wasm/types/kul_wasm.d.ts` ([ADR-0012](./0012-tsify-derived-types-committed-and-diffed.md)),
and a Vitest lint reads both files as text and fails on any drift. Precedent:
`crates/kul-svg/tests/visual.rs` lints the baked token layer the same way.

### The enumeration widens itself from the packs

The pack suites walk a bounded enumeration of *realizable* descriptors built
from the path grammar. The facets a path does not determine — `sharing`,
`side`, the two seniorities, `edgeNature`, `egoGender` — are enumerated over
**the values some registered pack actually keys**, plus one default.

So the key space grows exactly when a pack starts discriminating on something,
and a new language inherits coverage and conflict-freedom over its own facets
without editing a test or paying for a combinatorial explosion it does not use.
Together with the `PACKS` registry (the single registration surface), this is
what makes "the `gu` pack's PR touches no logic" checkable as a diff shape.

## Consequences

- English's uncle / nephew spines are enumerated entries, so they stop at two
  `great-`s. Past that the fallback fires — "great-great-grandfather's son" —
  which is honest, and extending it is adding rows.
- `cap` is unused by `en` (English's `great-` is genuinely unbounded), so it is
  covered by a fixture pack shaped like Gujarati's `par-`. The `gu` slice is
  where it earns its keep in production data.
- The affix vocabulary did not need generalising beyond ADR-0033's shape: `when`
  (with an optional `atLeast`), `affix`, `position`, `repeatPer`, `cap`. No
  per-language code hook exists, and none was needed for `en`.
- A pack is checkable as data: five suites (facet validity, governed-facet
  validity, coverage, entry conflicts, affix-slot conflicts) run over every
  registered pack.

## Anti-suggestions (do not re-propose)

- **"Give entries a numeric threshold so the uncle spine is one row."** It
  reaches the co-parent-in-law cell and answers "uncle" there. Equality-only
  entries are what keep specificity a plain count and a pack a table.
- **"Let `cap` just drop the affix and keep the bare term."** Then a
  generation-4 Gujarati ancestor renders *dādā*, asserting a term the language
  does not have for that generation. Refusing hands the key to the fallback,
  which is what speakers do.
- **"Resolve affix nesting by declaration order — it reads naturally in the
  file."** Then reordering a pack changes a word. The order is derived from the
  rules' own thresholds and specificity.
- **"Treat every specificity tie as a defect."** English needs two entries at
  the same specificity yielding *brother-in-law*, and Gujarati will need the
  same for *savkā*. Only disagreement is a defect.
- **"Import the descriptor types from `@kullang/wasm`."** That puts a dependency
  on the module whose whole point is having none, and the text lint already
  catches the drift the import would have prevented.

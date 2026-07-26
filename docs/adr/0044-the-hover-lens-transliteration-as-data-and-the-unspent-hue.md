# ADR 0044 — The hover lens: transliteration as data, a settle-shaped debounce, and the hue that stays unspent

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[PRD-0006](../prd/0006-preview-kinship-query-ux.md) calls the hover lens "the affordance that made
the whole model click", and [#302](https://github.com/YashBhalodi/kul/issues/302) says why: it is the
interaction the architecture was arranged around. A zero-click affordance firing on pointer movement
cannot afford a webview→extension→LSP round-trip, so the engine came to the webview
([ADR-0034](./0034-query-transport-and-result-node-identity.md)) and phrasing came with it
([ADR-0033](./0033-phrasing-layer-architecture.md)). Five slices built the parts. This one spends
them.

Between them, #302, [ADR-0036](./0036-two-tier-theming-and-the-accessibility-non-goal.md),
[ADR-0038](./0038-tier-1-register-overlay-stack-and-the-reserved-carve-out.md),
[ADR-0039](./0039-phrasing-lookup-mechanics.md), [ADR-0041](./0041-the-gu-pack-and-the-locale-toggle.md)
and [ADR-0042](./0042-the-selection-seam-and-occlusion-aware-centring.md) settle the placement, the
paint vocabulary, the phrasing mechanics and the seam. Six questions they leave open have answers
that read as arbitrary style choices unless they are written down — and one of them, transliteration,
was explicitly deferred to this slice by name.

1. **What shape does per-term transliteration take, and what happens where a pack has none?**
   ADR-0041 recorded the cost (a field on `PackEntry`, a field on `Phrase`, threading through
   `lexicalize`) and refused to pay it in a slice whose diff had to be data. It left the decision to
   "the slice with the chrome that would consume it", which is this one.
2. **How long is the debounce, and what does it key on?** [#292](https://github.com/YashBhalodi/kul/issues/292)
   says "trailing-edge, not a spinner" and gives the measurement that forces one. It gives no number.
3. **What does the pill say when there is no tie?** #302 gives one string and simultaneously forbids
   flattening two engine states into one.
4. **What paints the hovered card?** #276's paint vocabulary, as PRD-0006 restates it, lists
   "hover-target amber". ADR-0036 reserves four hues, and the amber one is spelled
   `--kul-hue-query-uncertain`: the can't-say paint [#303](https://github.com/YashBhalodi/kul/issues/303)
   is being built to use.
5. **Which nodes is "traces the connecting persons"?** #302 names the two edge selectors and says
   nothing about the persons or about the two endpoints.
6. **What happens to a lens reading when a render swaps the picture?** ADR-0042 pins
   `repaintQueryChrome()` as the one post-render hook and says the selection *repaints*. It says
   nothing about chrome whose whole meaning is "where the pointer is right now".

## Decision

### A transliteration is a record, and a missing record withdraws the gloss

`translit` sits on `PackEntry` and on `AffixRule`; `hopsTranslit` and `genitiveTranslit` sit beside
`hops` and `genitive` on the pack. `lexicalize` returns a `LexicalForm` — text plus optional Latin
form — and `phrase` folds both readings in **one walk over one set of winning records**, so a
composed chain's gloss can never head off a different entry than its text. The reading surface is the
element's native `title`, written by `LocaleController.bind`.

Two rules make this honest rather than merely present.

**Nothing is romanized from the script.** There is no transliteration function anywhere in the layer
and there must never be one. A phrase carries a Latin reading only when *every* token it was built
from carries one; a single missing record yields no transliteration at all rather than a
half-romanized string. That is the never-guess rule ADR-0033 established for phrasing, applied to a
second reading of the same phrase: a pack that has not said how a word reads is not a pack from which
a reading can be derived.

**A pack romanizes completely or not at all**, checked by the pack suites. Per-record opt-in was the
obvious alternative and is worse in practice: a gloss that appears on *kākā* and not on *māmā* reads
as breakage, and a reader cannot tell a missing record from a term with nothing to say. `en` opts out
entirely — an English term is already its own reading, and a tooltip repeating the word underneath it
is noise on every hover. `gu` opts in for all seventy entries, its one affix rule, its hop lexicon and
its genitive, from the Latin column of [`docs/kinship-term-inventory.md`](../kinship-term-inventory.md).

Covering the **composed** forms rather than entries alone is the part worth defending, because
entries alone would have satisfied ADR-0041's literal description. A genitive chain is precisely where
Gujarati declined to supply a term, it is what the fallback produces across a large share of the key
space (ADR-0041 measured 218,370 composed phrases in the coverage enumeration alone), and it is
therefore exactly where a reader who does not read the script is most stranded. Stopping at entries
would have shipped a gloss that works on the easy half.

The **pure-data alternative** — a `gu-Latn` pack — stays rejected for ADR-0041's reason: seventy
duplicated entries with nothing enforcing that the two agree, offered as a third language in a toggle
that promises one language at a time. `translit` on the same record cannot drift from the term it
glosses, because it is written next to it.

### The debounce is trailing-edge at 120 ms and keys on the card, not the pixel

`queryResolve` measures 32 ms at the 10,000-person ceiling — inside
[ADR-0029](./0029-query-engine-performance-posture.md)'s 50 ms budget for one call, and #292 is
explicit that inside budget is not the same as free: 32 ms of main-thread work per pointer move caps
the frame rate at ~30 fps and blocks paint while it runs. That is a jank source, not a latency source,
which is why the answer is a debounce and not a spinner.

**120 ms** is chosen against the two numbers that bound it: it is comfortably above the worst measured
call, so a settle can never overlap the next one however fast the reader sweeps, and comfortably below
the ~200 ms at which an affordance stops reading as live rather than as an animation.

The **leading edge** was the alternative and is wrong here in a way that is easy to miss: a leading-edge
debounce fires on the *first* card of a sweep and then suppresses the rest, so a reader sweeping to the
card they care about gets an answer about the card they crossed first. The trailing edge answers where
the pointer stopped, which is what "settle" means.

Hover is deduplicated against **the card element**, so a pointer resting on one card re-arms nothing
and the pill appears even for a reader whose hand is not perfectly still. Deduplicating on the
*person* id was considered and rejected for a smaller cost than it looks: a person owns a canonical
card and any number of ghosts, and moving between two of them would keep an old anchor, leaving the
pill docked under a card the pointer had left — which breaks the one thing the pill's position is
carrying. The extra resolution that costs is one query, in a gesture that is rare.

The same call re-anchors a pill that is already up, so the pill stays docked while the reader drags
the canvas underneath it.

### Emptiness is two wordings, and neither is bare

`disconnected` whispers **"not related (no connection)"**; `noneWithinBounds` whispers **"not related
(within bounds)"**. A missing reason cannot occur — the engine sets one iff the list is empty — and
falls through to the bounded wording, which is the weaker claim and the same choice
`kul query rel`'s human output already makes.

#302 gives the second string and forbids a bare "not related". Read together with
[ADR-0028](./0028-relationship-resolution-and-honest-emptiness.md) — where the *distinction is the
product* — that is a requirement for two strings, not for one string plus a suppression. Rendering
"not related (within bounds)" for a genuinely disconnected pair would understate what the engine
knows; rendering a bare "not related" for a bounded miss would overstate it. Both are the same
mistake in opposite directions, and one string cannot avoid both.

Saying **nothing** for an unrelated pair was the other option and is the worst of the three: it makes
a working lens indistinguishable from a broken one, and "no cousins is an answer, not a failure" is
the epic's own stance about the neighbouring surface.

### The hovered card gets no new paint, and the amber stays reserved

The lens adds **one** paint to the tree: the dashed sky resolution path. The hovered card keeps the
`.kul-card:hover` stroke bump the diagram already ships and gains nothing else.

PRD-0006's paint-vocabulary line lists "hover-target amber", and ADR-0036 — which is the authority on
the token layer — allocates the only reserved amber to *can't say*, the three-valued filter verdict
#303 is being built around. Painting a hover target with it would make one hue mean two things on one
canvas at one time, since a filter can be active while the lens reads, and it would force #303 either
to distinguish itself by dash pattern alone or to invent a fifth hue — the exact "invent it per slice"
practice ADR-0038's carve-out exists to prevent.

There is also nothing for the paint to disambiguate. The reader's pointer is on the card; a pill is
docked under it; the card under one's own cursor is not a thing that needs a colour to be located.
The hues are spent where a reader cannot otherwise tell — which is what the sky path is for.

So `--kul-hue-query-uncertain` and `--kul-hue-query-result` stay in ADR-0038's
`RESERVED_PENDING_CONSUMERS`, and `--kul-hue-query-path` leaves it — the converse assertion that
carve-out was written to force, firing for the second time.

### The trace shows the middle, because the ends are already shown

For each relationship, the path paints every hop's edge — an `across` hop's marriage, a vertical hop's
birth or adoption edge — **plus every person the path passes through**, and **neither endpoint's card**.

The persons are what makes it an explanation rather than a highlight: #302's own words are "traces the
connecting *persons*", and a reader looking at three lit edges without the cards between them has to
reconstruct who they belong to. Every card a traced person owns lights, canonical and ghosts alike,
per [ADR-0034](./0034-query-transport-and-result-node-identity.md)'s rule that a result is about a
person.

Excluding the two endpoints' cards is not an optimisation. The ego already wears the reserved violet
and the alter is under the pointer with the pill docked to it; painting either sky would put a third
and fourth meaning on the two cards whose meaning is least in doubt, and on the ego it would sit
directly on top of the selection outline. What the trace adds is the part the reader cannot see. An
endpoint's own birth *edge* does light where a leading `up` hop was drawn as it — that edge is part
of the answer, and an edge carries no claim about which endpoint is which.

The direction bookkeeping is worth stating because getting it wrong is silent: a `down` hop lands on
the child, an `up` hop leaves *from* one, so the walk carries the person it came from rather than
reading each hop alone. Reading the hop alone paints a plausible, wrong edge.

For a multi-tie the paint is the **union** over every descriptor, for the same reason the pill shows
every term: the engine found several ways and the surface shows several ways.

### A render dismisses the lens; it does not repaint it

`repaintQueryChrome()` re-applies the selection outline and takes the lens **down**.

The asymmetry with the selection is the decision. A selection is a standing choice the reader made and
has not unmade, so repainting it onto the fresh SVG is restoring something that is still true
(ADR-0042 pins that, and leaves the question of whether an edit should *end* query mode to
[#304](https://github.com/YashBhalodi/kul/issues/304)). A lens reading is a claim about a project that
has just been replaced, computed for a pointer position that may no longer be over anything. Re-asking
would answer a question nobody posed against a picture the reader has not looked at yet; re-painting
without re-asking would assert a tie nothing checked. The next pointer move re-arms it, which costs
the reader nothing they would notice.

A selection change dismisses it for the narrower version of the same reason: the ego moved, so the tie
on screen is about the person who *was* selected.

### The pill takes the pointer, and leaving the canvas onto it is not leaving

The pill is `pointer-events: auto`, because hovering a term is how the fuller gloss is read and
`pointer-events: none` would make the gloss unreachable. That creates a loop the first implementation
of this always hits: the pill floats over the canvas but is not inside `#root`, so moving onto it
fires `#root`'s `pointerleave`, which dismisses the pill, which puts the pointer back over the canvas,
which re-arms it.

The fix is one line in each of two places rather than a state machine: the mount passes `pointerleave`'s
**`relatedTarget`** — what the pointer moved *onto* — and the lens ignores any hover whose target is
inside its own pill. Leaving the canvas onto the panel, the controls or nothing at all still dismisses.

Only the **horizontal** position is clamped into the viewport. Vertically the pill stays with its card
even when the card is near the bottom edge, because position is the whole of the direction chrome: a
pill that drifted upward to stay visible would be saying "this person" about nothing.

## Consequences

- **A pack now has two readings and one source of truth for each.** Adding a language is still one
  additive module of records; the module is a little wider. ADR-0033's "zero lines of logic" survives
  — `translit` is matched by nothing and keys nothing, it only travels.
- **`docs/kinship-term-inventory.md` has now fully graduated.** Its script column became the `gu`
  terms in #299 and its Latin column becomes their glosses here, so no column of it is still waiting
  to reach code; #304 deletes it with the PRD.
- **`--kul-hue-query-path` leaves `RESERVED_PENDING_CONSUMERS`** and `--kul-query-path-*` joins tier 2.
  Two reserved paints and the filter alpha remain: the result teal is #301's, the can't-say amber and the dim alpha are #303's.
- **`--kul-phrase-hops` is a declared token that JS writes per element.** It is how a slot sizes for a
  five-hop chain without measuring or parsing text, and it is declared in the tier-2 layer with a
  default so the structural lint has no dangling name to report. It is not a theming knob; the value
  belongs to the phrase.
- **`queryResolve` joins `PreviewHandle`** on the same transport policy as `queryDetail`, with no
  `ResolveConfig`: the default generation budget of 5 reaches through fourth cousins, a strict
  superset of every lexicalized term (ADR-0028), so the lens has nothing to tune and widening it later
  is additive.
- **`ResolveResult` imports `RelationshipDescriptor` from the phrasing layer's mirror** rather than
  restating it in `engine-wire.ts`. One mirror per wire declaration, one conformance lint, and the
  same one-provenance-path rule ADR-0024 and ADR-0034 hold person data to.
- **`QuerySurfaceOptions` gains `floatLayer`, `locale` and `resolve`, and `QuerySurface` gains
  `handleCanvasHover`.** All additive; `mount.ts` now reaches for seven members instead of six.
- **The `locale` option is a marked threading seam.** #301 threads a pack through the same
  constructor for the panel's kin rows; on rebase its version wins and this one goes, so there is one
  locale path through the chrome rather than two.
- **A third paint now shares the canvas** and the `.kul-selected` / `.kul-query-selected` invariant is
  untouched: the lens paints neither endpoint and reads only while a person is selected — which
  suspends editor sync and strips its highlight — so its class never lands on a node wearing either.

## Anti-suggestions (do not re-propose)

- **"Romanize the script at runtime instead of storing a second string."** Then the layer asserts a
  reading nobody in the language reviewed, in a module whose entire discipline is that terminology is
  reviewable data. It is the never-guess rule with a transliteration table instead of a term table.
- **"Fall back to the term itself when `translit` is missing."** A tooltip that repeats the word
  underneath it is noise, and it makes "this pack does not romanize" indistinguishable from "this
  pack romanizes and this word happens to be Latin".
- **"Put `translit` on entries only; a chain can go unglossed."** The chain is where the language had
  no word, which is where an unfamiliar script is hardest to read. That ships the gloss for the easy
  half.
- **"Let a pack romanize some entries and not others."** The reader cannot tell an omission from a
  term with nothing to say, so partial coverage reads as breakage. The choice is per pack and the
  suites hold it there.
- **"Ship a `gu-Latn` pack."** ADR-0041 rejected it and nothing here changes: seventy duplicated
  entries with nothing enforcing agreement, in a toggle that promises one language at a time.
- **"Use a leading-edge debounce so the first card answers instantly."** It answers about the card the
  reader crossed on the way, not the one they stopped on. The whole point of a settle is that it is
  where the pointer stopped.
- **"Add a spinner — 32 ms is visible."** #292 measured it and said the opposite: single operations at
  13–32 ms are under the threshold where a spinner helps rather than flickers. The problem is work per
  pointer move, and a debounce is the fix for that.
- **"Deduplicate hover on the person id — two cards of one person are one answer."** They are, and the
  pill would stay docked under the card the pointer left, which destroys the one thing its position
  carries. One extra query in a rare gesture is the cheaper side.
- **"Say 'not related' and be done."** The engine distinguishes two states on purpose and ADR-0028
  calls the distinction the product. One string discards it in one direction or the other.
- **"Show nothing when two people are unrelated."** Then a working lens looks like a broken one, and
  the reader cannot tell "no tie" from "the lens did not fire".
- **"Paint the hover target amber; the PRD's vocabulary says so."** The only reserved amber is
  `--kul-hue-query-uncertain`, which #303 needs for a verdict that can be on screen at the same time.
  There is also nothing to disambiguate — the reader's pointer is on the card.
- **"Trace the endpoints too; the path runs from ego to alter."** It does, and both ends already carry
  a paint that says which they are. A third meaning on the two least ambiguous cards buys nothing and
  collides with the selection outline on one of them.
- **"Repaint the lens after a render, like the selection."** A selection is still true after a render;
  a tie computed against the previous project is not, and the pointer may no longer be over anything.
  Re-asking answers an unasked question; re-painting asserts an unchecked one.
- **"Give the pill `pointer-events: none` so it never steals hover."** Then the gloss is unreachable,
  and "hovering a term gives the fuller gloss" is the sentence this slice exists to satisfy.
- **"Clamp the pill vertically so it is never off-screen."** Position *is* the direction chrome. A
  pill that has drifted off its card is a sentence about nobody.
- **"Add `role="tooltip"` / `aria-live` to the pill — it is announcing something."** ADR-0036 makes
  accessibility a stated non-goal for new query chrome, and partial accessibility advertises a path
  that dead-ends at the tree, where every query begins. The surviving chrome keeps everything it has.

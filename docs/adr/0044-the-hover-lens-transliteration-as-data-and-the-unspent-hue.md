# ADR 0044 — The hover lens: transliteration as data, a settle-shaped debounce, and the hue that stays unspent

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

The epic ([#296](https://github.com/YashBhalodi/kul/issues/296)) calls the hover lens "the affordance that made
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
4. **What paints the hovered card?** #276's paint vocabulary, as the epic restates it, lists
   "hover-target amber". ADR-0036 reserves four hues, and the amber one is spelled
   `--kul-hue-query-uncertain`: the can't-say paint [#303](https://github.com/YashBhalodi/kul/issues/303)
   is being built to use.
5. **Which nodes is "traces the connecting persons"?** #302 names the two edge selectors and says
   nothing about the persons or about the two endpoints.
6. **What happens to a lens reading when a render swaps the picture?** ADR-0042 pins
   `repaintQueryChrome()` as the one post-render hook and says the selection *repaints*. It says
   nothing about chrome whose whole meaning is "where the pointer is right now".
7. **Whose is the "docked-tag grammar"?** #277's resolution says the filter's can't-say reason whispers
   *"in the lens's docked-tag grammar"* — under a card, with **no selection anywhere**, on a trigger
   that is not a hover resolution. Calling something a shared grammar and building it as one widget's
   private chrome are different acts, and only the first was written down.

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
its genitive.

**Where those Latin forms came from, precisely**, because "from the inventory" would be false in both
directions. The term inventory ([#278](https://github.com/YashBhalodi/kul/issues/278), a transient
research asset since retired — see [ADR-0046](./0046-the-mode-boundarys-render-paths-and-what-the-retired-documents-leave-behind.md))
supplied the romanization wherever it had a row for the term. It does not have one for everything the `gu` pack ships: #299 added
entries the inventory does not list (*pote*, *jīvansāthī*, *vālī*, *santān*, *sahodar*, the *savko
dīkro* / *savkī dīkrī* pair), four of the nine hop nouns (*mātā*, *vālī*, *santān*, *jīvansāthī* — the
other five, *pitā*, *putra*, *putrī*, *pati* and *patnī*, are in it verbatim), and four gendered
sibling forms normalised to the pack's own *bahen* where the inventory writes the *ben* variant
(*moṭī bahen*, *nānī bahen*, *pitrāī bahen*, *masiyāī bahen*). Exactly **six** are lexemes the
document does not contain at all — *jīvansāthī*, *mātā*, *pote*, *sahodar*, *santān*, *vālī*;
everything else written by hand recombines forms it does have. So: **the inventory's Latin column
where it has one, extended by hand for what #299 added beyond it**, following the same conventions.

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
and the pill appears even for a reader whose hand is not perfectly still.

Deduplicating on the *person* id was considered and rejected, and the failure it causes is narrower
than it first looks — narrow enough to be worth stating exactly, because a vague warning invites the
"simplification" it warns against. A person owns a canonical card and any number of ghosts, and the
answer for two cards of one person is genuinely the same answer, so **once the pill is up**, keeping
it is correct and re-querying is waste. The break is in the **in-flight window**: the pending timeout
captured the card it was armed for, so a person-keyed dedupe lets the first timer survive the crossing
and dock the pill under the card the pointer has already left. That window is 120 ms wide and it is
where a moving pointer spends most of its time, and the failure destroys the one thing the pill's
position carries. Re-anchoring on a person-keyed dedupe instead of re-arming would fix the symptom and
cost a comparison per pointer move to save one query in a rare gesture; card-keyed dedupe is the same
outcome with less machinery. `tests/hover-lens.test.ts` pins the crossing directly.

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

#276's paint-vocabulary line, as the epic restates it, lists "hover-target amber". The decisive fact is simply that
**ADR-0036 reserved four hues and none of them is a hover target**: it names them "selection violet,
result teal, can't-say amber, resolution-path sky", `preview-themes.css` documents each in place, and
the amber is spelled `--kul-hue-query-uncertain` — "a three-valued predicate that answered `unknown`".
There is no fifth reserved hue and no unspent one to promote, because ADR-0036 records that
`--vscode-charts-*` is fully spent on document meaning — three genders and three edge kinds. A
hover-target paint would therefore have to take a hue that already means something else, which is the
collision the whole reservation exists to prevent.

That vocabulary line is a leftover rather than a decision, and its provenance is checkable. It
restates #276's point 7, whose amber comes from **round 7, card B ("Genitive")**, where the pill
floated between the pair and "the amber ring shows who it lands on" was that card's direction chrome. **Card A ("Docked
tag") won**, and it disambiguates by *position* plus the violet viewpoint dot — the design this slice
built. Point 7's vocabulary line was never rewritten afterwards. Taking a hue on the strength of an
un-rewritten summary of a discarded card, against the ADR that allocates hues, is the wrong way round.

Two supporting arguments, weaker than the one above and offered as such. **The amber has a consumer
waiting**: #303's can't-say verdict, which would then have to distinguish itself by dash pattern alone
or invent a fifth hue — and a filter verdict and a lens reading can be on screen together, though note
that what the epic states explicitly is that the lens keeps reading while a *kin-set* paint is active,
not a filter. And **there is nothing for the paint to disambiguate**: the reader's pointer is on the
card and a pill is docked under it, so the card under one's own cursor is not a thing that needs a
colour to be located. The hues are spent where a reader cannot otherwise tell — which is what the sky
path is for.

So `--kul-hue-query-path` leaves ADR-0038's `RESERVED_PENDING_CONSUMERS` and
`--kul-hue-query-uncertain` stays — the converse assertion that carve-out was written to force,
firing for the **third** time. The list was created with four names; the selection violet left with
#300, the result teal with #301, the sky here. The amber is the last one standing, and it is #303's.

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

`repaintQueryChrome()` re-applies the selection outline and takes the lens **down**. (**Amended by
[ADR-0046](./0046-the-mode-boundarys-render-paths-and-what-the-retired-documents-leave-behind.md) (#304):** the hook is `endQueryMode()` and a render takes
the *selection* down too. The lens's half of this is unchanged and is now the whole rule — and it
needs no call inside the boundary, because dismissing on a selection change is what this section
decided.)

The asymmetry with the selection is the decision. A selection is a standing choice the reader made and
has not unmade, so repainting it onto the fresh SVG is restoring something that is still true
(ADR-0042 pins that, and leaves the question of whether an edit should *end* query mode to
[#304](https://github.com/YashBhalodi/kul/issues/304)). A lens reading is a claim about a project that
has just been replaced, computed for a pointer position that may no longer be over anything. Re-asking
would answer a question nobody posed against a picture the reader has not looked at yet; re-painting
without re-asking would assert a tie nothing checked. The next pointer move re-arms it.

**The case that is actually felt**, named so it is read as a consequence rather than found as a bug:
the reader is typing in the editor with the pointer resting on the tree. Each edit re-renders the
preview, and each render takes the pill down — and because the pointer is not moving, nothing re-arms
it. The lens stays dark until the reader moves the mouse. That is the right trade in both directions:
a pill that survived the render would be showing a tie computed against source that no longer exists,
and a pill that silently re-queried on every render would put a resolution on the 300 ms-debounced
render path for a reader who is not looking at the tree at all. It is also the shape #304 will meet
from the other side, since its rule is that an edit *ends* query mode.

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

### The trace outranks the dim, and the exemption is read off the backbone

With a kin set painted, every person outside the answer wears the ambient dim
([ADR-0043](./0043-explore-kin-thirteen-query-values-and-what-a-row-costs.md)) — and the persons a
resolution runs through are *almost always* outside it, because "how are these two related" and "who
are Giuseppe's cousins" are different questions. Left alone, the sky trace would render at the dim's
alpha: an explanation fainter than the thing it explains.

So the lens publishes the persons it names through `QuerySurface.setDimExemption`, and they are
lifted from **every** dim source for as long as the pill is up. ADR-0043 states the rule — the lens is
what the reader is doing *now*, the dim is what they did a moment ago — and puts the mechanism on the
dim registry so #303's filter inherits it rather than re-deciding it. This slice is its first caller.

Two properties of the published set are worth pinning, because both are easy to get subtly wrong.

**It is read off the descriptors' backbones, not off the picture.** `dim.ts` obliges a source whose id
set is DOM-derived to *republish* after a render rather than merely re-apply, since a swapped-out SVG
can change what that source would dim. The exemption is a set of `path[].to` ids, so it carries no
such obligation — and the question is doubly moot here, because a render dismisses the lens, which
withdraws it.

**The ego is not in it.** Kin paint already refuses to dim an anchor, so naming the ego would imply a
need that does not exist and would quietly couple this set to a rule that belongs to the other module.
The alter *is* in it: it is the card under the pointer with the pill docked to it, and the reader
should not be pointing at something at 30%.

Publishing and withdrawing both walk every card in the picture, so both are guarded. Nothing is
published for an **empty** trace — an unrelated pair whispers a bounded form and traces nobody, so an
empty exemption would exempt no one at the cost of two walks — and nothing is withdrawn when nothing
is out. A pointer sweeping a tree costs a walk only where a reading actually names someone.

### The docked tag is the grammar; the lens is one consumer of it

The whisper splits in two. **`docked-tag.ts` owns the mechanism**: the float-layer placement, the
measurement against an anchor's screen box, the horizontal clamp, the pill chrome and the
pointer posture. **`hover-lens.ts` owns the policy**: when to resolve, what the tag says, the trace,
the selection gate, the debounce and the dismiss rules. The seam between them is a list of nodes.

```ts
openDockedTag({ layer, anchor, content, variant? }) -> DockedTag
                                    // { element, anchor, reanchor(), contains(node), close() }
```

This is not speculative generality — it has a named second consumer before it ships.
[#303](https://github.com/YashBhalodi/kul/issues/303)'s can't-say reason is specified to whisper in
this grammar, and it cannot reach it through the lens: `HoverLens` exposes `handleHover` / `dismiss` /
`dispose`, its content builder takes a `ResolveResult`, and `handleHover` **hard-gates on a person
selection existing** — while a filter verdict must whisper with no selection at all. Left private, the
lens's file would have had to grow a second trigger and a second content model for a feature that has
nothing to do with relationship resolution, or #303 would have copied the placement arithmetic. The
copy is the worse outcome by some distance: two docked-tag grammars, drifting, in a design whose whole
claim is that there is one.

**The content model is a list of nodes, deliberately.** The mechanism supplies the box and never a
word; the lens appends a viewpoint dot and phrased terms, #303 appends whatever a reason string wants.
Generalising the *content* — a `note` slot, a `reason` variant, a `severity` — would be inventing
#303's chrome from here, which is exactly the speculative half. The socket is shared; the plug is not.

The split cost the lens nothing behavioural: the selection gate, the debounce, the trace and
dismiss-on-repaint are untouched, and every test pinning them passes unchanged. The tag chrome's
tokens move `--kul-lens-*` → `--kul-docked-tag-*`, which is the rename that makes the ownership
readable, and `.kul-lens` survives as a **variant class with no rules** — a marker so the lens's own
tag stays findable once something else docks one.

## Consequences

- **A pack now has two readings and one source of truth for each.** Adding a language is still one
  additive module of records; the module is a little wider. ADR-0033's "zero lines of logic" survives
  — `translit` is matched by nothing and keys nothing, it only travels.
- **The term inventory is *not* exhausted, and #304 should know that before deleting it.** Its
  script column became the `gu` terms in #299 and its Latin column becomes their glosses here, but
  the pack deliberately did not take everything it lists: *bā*, *mummy*, *bāpuji*, *var*, *dhaṇī*,
  *śokya*, *putravadhū*, *apar-mā*, *dattak*, *fai*, *der* and *bhāṇejo* are variants and registers
  the pack chose one of, and they never became entries. Deleting the file discards them. That is a
  legitimate call — a lexicon of alternates is not something a one-term-per-cell pack can hold, and
  the epic decided the shipped pack *becomes* the record (ADR-0041) — but it is a decision, not a
  clean-up, and this ADR states it rather than letting the deletion imply it.
  **[ADR-0046](./0046-the-mode-boundarys-render-paths-and-what-the-retired-documents-leave-behind.md)
  made the call and carries the list of record**: it verified all twelve above against the pack,
  found four more (*nānā-bāpā*, *nānī-mā*, *moṭā kākā*, *ben*), and lifted the sixteen into an
  appendix with the term each one lost to.
- **`--kul-hue-query-path` leaves `RESERVED_PENDING_CONSUMERS`** and `--kul-query-path-*` joins tier 2.
  **One** reserved name remains — the can't-say amber, #303's. Everything else ADR-0036 reserved has
  been spent: the selection violet by #300, the result teal and the filter alpha by #301, the sky
  here.
- **`--kul-phrase-hops` is a declared token that JS writes per element.** It is how a slot sizes for a
  five-hop chain without measuring or parsing text, and it is declared in the tier-2 layer with a
  default so the structural lint has no dangling name to report. It is not a theming knob; the value
  belongs to the phrase.
- **It governs *composed* slots only, and that bound is load-bearing.** `hopCount` is `0` for every
  lexical phrase by construction, so it reports nothing about a lexical term's width — and lexical
  terms are not uniformly short: *kākā* is four characters and `en`'s *second cousin twice removed* is
  twenty-seven. A lexical term is therefore capped by the pill and wraps inside it. Both halves are
  under test; the overflow itself is not reachable from jsdom, which is why the assertions are on the
  two facts rather than on a rendered box.
- **`queryResolve` joins `PreviewHandle`** on the same transport policy as `queryDetail`, with no
  `ResolveConfig`: the default generation budget of 5 reaches through fourth cousins, a strict
  superset of every lexicalized term (ADR-0028), so the lens has nothing to tune and widening it later
  is additive.
- **`ResolveResult` imports `RelationshipDescriptor` from the phrasing layer's mirror** rather than
  restating it in `engine-wire.ts`. One mirror per wire declaration, one conformance lint, and the
  same one-provenance-path rule ADR-0024 and ADR-0034 hold person data to.
- **`QuerySurfaceOptions` gains `floatLayer` and `resolve`, and `QuerySurface` gains
  `handleCanvasHover`.** All additive; `mount.ts` now reaches for seven members instead of six. The
  `locale` option is **not** new — ADR-0043 put the `LocaleController` itself on the surface, and the
  lens takes the `bind` idiom from it, which is the half of that decision this slice is the consumer
  of. There is one locale path through the chrome, not two.
- **The lens is `setDimExemption`'s first caller**, and the only one. The exemption is a single slot
  rather than a map keyed by holder, because a pointer is in one place; a second concurrent holder
  would clobber the first, and keying it is the fix if one ever exists (ADR-0043).
- **#303 reuses the placement grammar and brings its own trigger.** `openDockedTag` is importable
  from the package root, works with no selection present, and needs nothing from `hover-lens.ts`;
  `floatLayer` is already on `QuerySurfaceOptions`. What it does **not** inherit is hover input:
  `handleCanvasHover` is wired straight to the lens (`handleCanvasHover: lens.handleHover`) and is
  documented as the lens's own, so a can't-say reason that whispers on hover adds its own plumbing to
  `createQuerySurface`. That is deliberately not pre-built here. A fan-out with one consumer is
  speculative generality, and guessing at the shape of #303's trigger is the same guess this ADR
  refused to make about its content — which is why the tag's content model is a bare node list.
  Placement when two tags share an anchor is #303's call for the same reason.
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
- **"Deduplicate hover on the person id — two cards of one person are one answer."** They are, and it
  is still wrong in the in-flight window: the pending timeout holds the card it was armed for, so a
  crossing during the 120 ms wait docks the pill under the card the pointer left, destroying the one
  thing its position carries. Fixing *that* needs a re-anchor on the dedupe path, which is more
  machinery than the one query it saves in a rare gesture. A test pins the crossing.
- **"Say 'not related' and be done."** The engine distinguishes two states on purpose and ADR-0028
  calls the distinction the product. One string discards it in one direction or the other.
- **"Show nothing when two people are unrelated."** Then a working lens looks like a broken one, and
  the reader cannot tell "no tie" from "the lens did not fire".
- **"Paint the hover target amber; #276's paint vocabulary says so."** ADR-0036 reserved four hues
  and none of them is a hover target; the only amber is `--kul-hue-query-uncertain`, and the charts
  family is fully spent, so there is nothing to promote. That vocabulary line is #276 point 7, whose
  amber is round 7's discarded card B. There is also nothing to disambiguate — the reader's pointer
  is on the card.
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
- **"Let the kin dim win — it is the standing state and the lens is transient."** Backwards: the
  transient thing is the one the reader is doing, and it is the one that has to be legible. A trace
  at the dim's alpha explains nothing, which is the whole reason ADR-0043 put an exemption on the
  registry.
- **"Exempt the ego too, for symmetry."** Kin paint never dims an anchor, so it would be a no-op that
  reads as a requirement — and it would couple this set to another module's rule.
- **"Have the lens republish its exemption after a render, like kin paint does."** Kin paint owes
  that because it decides who to dim by walking the *cards*; the lens reads `path[].to` off the
  answer. And a render dismisses the lens, so there is nothing left to republish.
- **"Fold the docked tag back into `hover-lens.ts` — it has one consumer."** It has two by
  specification: #277 gives #303's can't-say reason the same grammar, with no selection and its
  own trigger, and the lens's entry point refuses to serve that. The alternative to the split is a
  second copy of the placement arithmetic and two grammars that drift.
- **"Give the docked tag a `note` / `reason` slot so #303 does not have to build one."** That is
  inventing #303's chrome from here, and the shared thing would then have opinions about content it
  cannot see. The node list is the socket; the plug is the consumer's.
- **"Let the lens keep `place()` and have #303 call `HoverLens`."** `handleHover` gates on a person
  selection and takes a hover target, which a filter verdict has neither of. Widening it would put
  filter policy inside the lens.
- **"Give a lexical term `white-space: nowrap` — a crisp term should stay on one line."** A crisp one
  does anyway; the rule only bites on the long ones. And a flex item that may not wrap has a
  min-content width equal to the whole string, which is its automatic minimum size, so it refuses
  every cap and paints through the pill's border. This shipped as a blocker.
- **"Size the lexical slot from `hopCount` with a bigger base."** There is no base that works:
  `hopCount` is zero for every lexical phrase, so the slot is a constant, and the constant has to
  cover *second cousin twice removed*. At that point it is the pill's width, spelled indirectly.
- **"Add `role="tooltip"` / `aria-live` to the pill — it is announcing something."** ADR-0036 makes
  accessibility a stated non-goal for new query chrome, and partial accessibility advertises a path
  that dead-ends at the tree, where every query begins. The surviving chrome keeps everything it has.

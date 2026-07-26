# ADR 0043 — Explore kin: thirteen Query values, two projections, and what a row costs

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[#276](https://github.com/YashBhalodi/kul/issues/276)'s resolution settled the *interaction*: "Explore
kin" opens one flat scrollable vertical list of all kin queries (Parents … Step-children) with
counts; clicking a row paints results on the tree; zero-count rows toast honest absence; the list
stays open as the selection walks. [PRD-0006](../prd/0006-preview-kinship-query-ux.md) restates it and
[ADR-0034](./0034-query-transport-and-result-node-identity.md) pins the binding — a result is about a
*person*, so every card that person owns lights. [ADR-0042](./0042-the-selection-seam-and-occlusion-aware-centring.md)
built the seam this hangs off and named the one thing it deliberately left owed: a `LanguagePack`
threaded through `createQuerySurface` → `createDetailPanel` → `buildDetailPanel`.

What none of them settled is the part that turns out to decide the shape of the code:

1. **What is a row, in engine terms?** "All kin-set queries" names no set of queries. The engine has
   thirteen named sugars and a `Query` value; the WASM surface exposes only the value.
2. **Where does a count come from?** [#301](https://github.com/YashBhalodi/kul/issues/301) requires the
   engine's `count` projection rather than a client-side length. The engine answers one Query per
   call, and [#292](https://github.com/YashBhalodi/kul/issues/292) measured that *any eager list of four
   or more rows misses ADR-0029's 50 ms budget at the 10,000-person ceiling*
   ([ADR-0034](./0034-query-transport-and-result-node-identity.md)'s measured note). Those two facts pull in
   opposite directions and something has to give.
3. **What is phrased?** #301 requires row labels that re-phrase on the locale toggle, and says the
   phrasing comes from data already in hand. A kin *set* is not a relationship, so it has no
   descriptor of its own to phrase.
4. **Where does the list live, and what does the language-pack seam look like** for #302 to reuse?

## Decision

### A row is one of the engine's named kin sets, spelled as its `Query` value

`KIN_SETS` is thirteen entries — `parents`, `children`, `siblings`, `spouses`, `ancestors`,
`descendants`, `auntsUncles`, `niecesNephews`, `cousins`, `inLaws`, `stepParents`, `stepSiblings`,
`stepChildren` — each carrying the `KinPattern` that `crates/kul-core/src/query/sugar.rs` documents
as its expansion. The list order is #276's ("Parents … Step-children"), flat, ungrouped.

This is the role ADR-0025 gives a surface: *the surfaces are thin constructors of this value, never
second engines.* The sugar itself is Rust-only, so a consumer that wants "siblings" builds
`kinOf(anchor, collateral up{1,1} down{1,1})`. No membership rule lives in the preview.

**The copy is enforced, not trusted**, and that matters more here than for a normal lint: this ADR
rejects the sweep alternative below *because* a hand-copied matcher drifts silently, and thirteen
hand-copied patterns are the same hazard one layer down. Two lints, catching two different drifts:

- `crates/kul-core/tests/kin_catalogue.rs` builds each row's `Query` value, proves it is
  **behaviourally** the sugar it claims to be (both run over the corpus, member lists compared, every
  person as anchor), and JSON-snapshots the resulting `KinPattern` per sugar. An argument drifting
  inside `sugar.rs` fails the first test; a pattern drifting inside `Query::kin_*` re-blesses the
  snapshot and fails the second lint.
- `packages/preview/tests/kin-sets.test.ts` reads that snapshot and compares it field for field with
  `KIN_SETS`, and separately regexes `sugar.rs` for `pub fn *_of` so a sugar added to the engine and
  to neither is caught. The two sources are exhaustive over different things, which is why both stay.

The engine growing or reshaping a kin set is then a failing test, not a list quietly one question
short or one question wrong.

Three sugars take parameters, and the list fixes them at the value a reader means:
`cousins_of(x, degree, removed)` spans a lattice and the row asks `degree 1, removed 0`;
`ancestors_of(x, depth?)` and `descendants_of(x, depth?)` take the unbounded default. Thirteen rows
that each answer a question a reader asks beats forty that spell a lattice, and #276's list — "Parents
… Step-children" — is explicitly the former.

### Counts come from the `count` projection — thirteen calls — and the sweep alternative is refused

Opening the list issues one `count`-projected `Query` per row. Every number on screen is the engine's
own count of a pinned Query value. Painting a row issues the **same value** with `projection:
"members"`; the two differ in one field, which is why they share a constructor and a call site.

The alternative was one call: `kinOf(anchor, any{maxUp, maxDown}, affinalHops {0,2})` with
`projection: "members"`, bucketed here by descriptor into thirteen sets. It is genuinely one check
instead of thirteen. It is rejected, on three grounds and in this order:

- **It moves kin-set membership into the consumer.** Bucketing a descriptor against a `KinPattern` is
  re-implementing `pattern.rs`'s matcher — including `collateralByDegree` matching both orientations,
  and the fact that an `affinity` filter is not only a predicate but the switch that opens the
  affinal-hop budget. ADR-0025's governing line is that the engine owns kinship correctness and
  consumers own only the UX of querying. A second matcher in TypeScript is the second engine that
  line exists to prevent, and its drift would be silent — wrong counts, not a crash.
- **Every count would become a client-side length**, which #301 forbids in as many words.
- **It cannot answer two of the rows honestly.** `ancestors_of` / `descendants_of` are unbounded;
  `any{maxUp, maxDown}` is not. A swept "Descendants: 11" would silently mean "11 within four
  generations", which is the kind of quiet approximation the epic's honesty stance exists to refuse.

**The cost this leaves standing is real, is not hidden, and is not small.** Thirteen `queryKin` calls
at the 10,000-person ceiling is **≈215 ms** — #292 measures one at 16.6 ms over WASM — well over
ADR-0029's 50 ms budget for one click, and the same arithmetic Finding 4 used to kill eager per-row
hydration.

That number is an **extrapolation from one datapoint, and it leans toward being a floor**. #292's
16.6 ms was measured on a single `collateralByDegree{2,0}` query, and the catalogue's patterns are
mostly heavier: unbounded `ancestors` and `descendants`, and two affinal `any` patterns that spend
marriage hops. Pulling the other way, that datapoint was `members`-shaped while the sweep projects
`count`, which materializes nothing and is plausibly cheaper per call. Neither axis has been
measured, so the honest statement is that ≈215 ms is the order of magnitude and the pattern axis is
the larger of the two unknowns.

The bounding is narrow and worth stating exactly, because it is easy to overstate:

- **The sweep is paid per anchor whenever the list is open.** Opening it costs one sweep; so does
  every *subsequent selection* while it stays open — and staying open as the selection walks is the
  configuration this feature is designed to live in (#276's resolution). It is not "only on an
  explicit gesture": the explicit gesture buys into a per-selection cost.
- **It is memoized per anchor**, so re-opening the list, closing and reopening it, or walking back to
  a person still selected re-asks nothing.
- **The list renders before the counts land** — a row with no answer yet shows a placeholder, never a
  `0` — so no frame blocks, and a reader can paint a row while the sweep is still in flight.

The actual fix is a batched kin-query operation in the engine: N Query values in, N results out, one
check — exactly the shape [ADR-0037](./0037-batched-detail-lookup-shape.md) already proved works for
details, and additive to the pinned surface. It is not built here because this slice is chrome and
that is a `kul-core` + `kul-wasm` change with a release attached. **If the Explore-kin list is ever
measured as slow, that operation is the named answer** — not a sweep, and not a client-side matcher.

### The sweep and the paint guard their own answers

Rows are deliberately clickable before their counts land, which makes "a row was painted while the
sweep was in flight" a designed path rather than an edge case — and at ≈215 ms, a common one. The two
questions are therefore guarded independently:

- the **sweep** is guarded on the **anchor** it asked about, because the anchor moving is the only
  thing that can make its thirteen answers wrong, and it claims its memo *before* its first `await` so
  a second call cannot start a duplicate;
- the **paint** is guarded on a **counter of its own**, because a second row click must discard the
  first row's answer even though the anchor has not moved.

One shared counter is the obvious shape and is wrong: painting a row would invalidate the in-flight
sweep, the memo would never re-arm, and every row would read `·` for as long as that person stayed
selected. `explore-kin.test.ts` holds the sweep open, paints a row inside the window and asserts both
answers survive.

**The memo is a claim on work in progress, not a record that the work was done**, so it is released
the moment the sweep does not fully answer — an envelope's error arm, a `null`, or a `runKinQuery`
that rejects. The error arm is not an exotic path: ADR-0009 makes it what *any* project with a
validation error yields, which is the ordinary state of a document mid-edit. A claim held across it
would leave the list permanently dead for that anchor, outliving the fix and the re-render that
follows it, with no escape but selecting someone else — which is precisely the failure the two-guard
split above exists to prevent, arriving by a different route. Whatever *did* answer is still shown:
those numbers are the engine's, and a row the sweep could not answer shows the same placeholder it
shows before any answer lands.

### Only the painted row is glossed, and the gloss is what re-phrases

A row carries two pieces of text. Its **name** — "Aunts & uncles" — is chrome and stays English, per
#276 point 6 and ADR-0022's line for the legend. Its **gloss** is the distinct terms of the members
the engine returned, phrased through the active pack: *Siblings — brother · sister — 4*. `queryKin`
hands back a full `RelationshipDescriptor` per member, which is exactly what `phrase` consumes, so a
gloss costs no lookup of its own (#292 Finding 6).

**Only the painted row glosses**, because only the painted row has members in hand. An unpainted row
has nothing to phrase, and the two ways to give it something were both refused:

- **Synthesise a representative descriptor per set** and phrase that. It means fabricating a path
  backbone the engine never emitted, and it cannot be done honestly at all: `alterGender` has no
  `unknown` value, so every set spanning genders would have to pick one, and "Ancestors", "Descendants"
  and "In-laws" have no single descriptor shape to pick from in the first place. Inventing engine data
  in order to phrase it is the provenance mistake ADR-0034 and ADR-0035 both refused, arriving by a
  new route.
- **Fetch members for every row at open** so every row can gloss. That is thirteen `members` calls
  instead of thirteen `count` calls, materialising member lists nobody reads, and it makes every count
  a client-side length as a bonus.

The consequence to state plainly: **the locale toggle changes the list only once a row is painted.**
That is correct rather than unfortunate — before a row is painted the list contains no kinship word
for a language to disagree about.

### One locale source of truth, two consumption idioms chosen by lifetime

`QuerySurfaceOptions` takes the **`LocaleController` itself**, once. Every phrased word in the query
chrome comes from it, and the surface hands each consumer the idiom that fits that consumer's
lifetime:

- **Wholesale redraw → read `pack()` at draw time.** `createDetailPanel` takes `pack(): LanguagePack`
  and `buildDetailPanel(detail, view)` takes a resolved `DetailPanelView` — `{pack, kin}` — and stays
  pure. A getter, not a captured value: a snapshot taken at construction would need its own
  subscription to stay current, and ADR-0041's toggle already drives `QuerySurface.refresh()`. With a
  getter, `refresh()` redraws in whatever language the toggle now reports and **no second subscription
  exists to fall out of step**.
- **A persistent element that must re-phrase in place → `locale.bind`.**
  [#302](https://github.com/YashBhalodi/kul/issues/302)'s lens pill is an element that is *already up*
  when the reader flips language, and `bind` owns its `textContent`, its `lang`, its phrase metadata
  and its title for as long as it lives — five writes per phrase site that a draw-time caller would
  have to replicate. It also cannot ride `refresh()`: [ADR-0044](./0044-the-hover-lens-transliteration-as-data-and-the-unspent-hue.md)
  dismisses the lens on repaint rather than repainting it, so a `refresh()` widened to "redraw every
  phrased surface" would name a behaviour one of its surfaces deliberately does not have.

Handing the panel a `pack()` getter *only*, and leaving the controller at the mount, was the shape
this ADR first took. It is rejected because it makes the seam express less than the rule: the lens
could not reach `bind` through it and would have to add a second locale option beside the first —
two paths to one source of truth, which is the outcome the rule exists to prevent.

What stays rejected is a **surface reaching for the controller to rebind ad hoc where a wholesale
redraw would do**. The panel rebuilds its element on every draw, so binding its rows would register
bindings against elements that no longer exist and leak one per redraw. The idiom follows the
lifetime, and the lifetime is a property of the widget, not a preference.

The kin list rides in `DetailPanelModel.kin` rather than in a widget of its own for the same reason
`buildDetailPanel` is pure at all: ADR-0035's provenance criterion is a lint over this file plus an
assertion at a value seam, and a kin list rendered by some other module would need its own copy of
both.

### The list lives in the person panel; the list outlives the anchor and the answer does not

The list is a section of the details panel's **person** variant. The marriage and adoption variants
carry `kin: null` — a kin set needs a person anchor and an edge is a waypoint (ADR-0035), so the
absence of the list *is* the waypoint rule, drawn.

Two lifetimes, deliberately different:

- **The open/closed state belongs to the reader** and outlives every anchor. That is what "the list
  stays open as the selection walks" means: walking to another person keeps the list open and re-asks
  its counts about the new anchor.
- **The painted answer belongs to one anchor** and is dropped the moment the selection moves. An
  answer about Giuseppe is not an answer about Marco, and leaving the teal up while the violet moved
  would say it was.

**Closing the list drops the paint, and re-clicking the painted row drops it too.** The list is the
only thing on screen that says *which question* the teal answers; a closed list over three lit cards
and a dimmed one is an answer with its question hidden, which is the same failure as a count with no
provenance. These two gestures are also the reader's only way out of a paint that does not cost them
the selection — Esc and a canvas click clear everything, and a reader who wants the tree plain again
while keeping the person they are reading had nothing else. Neither gesture asks the engine anything,
and closing keeps the counts, so reopening is free.

The paint repaints from inside `repaintQueryChrome()` — ADR-0042 widened that hook for exactly this —
and **not** from a second call in `mount.ts`.

**Kin paint registers no sync-suspension reason of its own.** It cannot exist without a person
selection, and the selection already registers `"selection"`; a second reason keyed to the same
condition would be a set entry that can never be the last one to lift. #303's filter is a different
matter — it can be active with nothing selected — and ADR-0042 already says it registers its own.

### Result teal lights every card, dim touches cards only

Matched persons take `kul-query-result` on **every** node `selectBoundNodes` binds — canonical card
and every ghost — which is ADR-0034 held rather than restated. A matched ghost keeps the glow and
takes it dashed: it is still a past record, and a solid ring would make it read as canonical.

Non-matching **cards** dim, on the shared `--kul-dim-alpha`. **Edges never dim.** #277's standing rule
is that paint never hides, only recedes, and dimming the edges would recede the picture the answer is
drawn on — the marriage bars and birth edges are how a reader sees *why* a set is the shape it is.
#276's prototype dimmed cards alone and this does the same.

The anchor is never dimmed and never lit: it wears the selection violet, and a card that is at once
the question and the faintest thing on screen is not a readable answer. The engine excludes an anchor
from its own answers, so filtering it out of the lit set is belt-and-braces — but the rule is the
paint's to state and to test, not the engine's to be trusted for.

### The dim has one owner and unions its sources; a live read outranks it

`--kul-dim-alpha` is applied by **one** module, `dim.ts`, and paint sources publish *sets of person
ids* to it rather than touching the class. A person is dimmed iff **any** active source dims them;
`apply` recomputes the whole class from the union; a source withdraws by publishing `null`.

Building this with one consumer is the point. Two hazards were already live in the two-source future
[#303](https://github.com/YashBhalodi/kul/issues/303) makes concrete:

- **Silent un-filtering.** `repaintQueryChrome()` runs the kin repaint on every render. A kin paint
  that stripped `.kul-query-dim` outright would clear the filter's dim on every keystroke-driven
  render, and nothing would say so.
- **Multiplied alpha.** A filter that avoided that by taking a *second* class would put two `opacity`
  values on nested nodes: 0.3 × 0.3 = 0.09, a card three times fainter than either source asked for.

One thing outranks the union: an **exemption**, the set of persons a *live pointer-driven read* is
tracing. #302's lens traces the persons that justify a relationship, and with a kin set painted those
persons are almost always outside the answer — so the sky trace would render at the dim's alpha. **The
lens wins**: it is what the reader is doing now, and the dim is what they did a moment ago. The
exemption lives on the registry rather than in kin paint, so #303's filter inherits the precedence
rule instead of re-deciding it, and it is reached through `QuerySurface.setDimExemption`.

`--kul-hue-query-result` and `--kul-dim-alpha` therefore leave ADR-0038's `RESERVED_PENDING_CONSUMERS`
carve-out. Two reserved paints remain: the can't-say amber (#303's filter) and the resolution-path sky
(#302's lens).

### An empty set is a row reading `0` plus one quiet toast

The toast is a notify-region member beside the sync hint, **not** an error-popover row: an absence the
engine reported is an answer, and borrowing the diagnostics' chrome would make "no cousins" look like
a fault. It is not modal, it replaces rather than stacks — two contradictory absences on screen read
as a log — and it takes itself down after six seconds.

It names the anchor from the `EntityDetail` the panel is **already holding**, so the sentence costs no
second call and introduces no second provenance path for a person's name. That is the half of
ADR-0037's "N person targets, one check" that this surface actually needs: the anchor's name, free.

A row whose count has not arrived shows `·`, never `0`. A zero is an answer and the surface must not
show one it has not been given.

## Consequences

- **Opening the kin list adds no batched detail lookup at all**, whatever the row count — asserted by
  counting `lookup` calls at the seam. Row names are chrome and the anchor's name is the answer the
  panel already holds.
- **The sweep costs one `count` query per row — thirteen — per anchor, whenever the list is open.**
  ≥215 ms at the 10k ceiling, over ADR-0029's budget; memoized per anchor, non-blocking, and the named
  fix is a batched kin-query operation in the engine rather than a consumer-side matcher.
- **Painting a row costs one `members` query**, regardless of how large the answer is.
- **`kul-core` gains a test file, not code.** `tests/kin_catalogue.rs` and its JSON snapshot are the
  contract the preview's catalogue is linted against; the snapshot's shape is therefore a cross-crate
  interface, and reshaping it breaks a TypeScript test.
- **`buildDetailPanel` now takes a second parameter.** Every caller passes a `DetailPanelView`; a
  panel with no kin list passes `{pack, kin: null}`.
- **`createQuerySurface` takes the `LocaleController`.** #302 reaches `bind` through the same option
  rather than adding a second one, and `refresh()` keeps its narrow meaning: *redraw the open panel*.
- **`EngineModule` / `QueryEngine` / `PreviewHandle` gain `queryKin`.** Test doubles for the engine
  must answer it.
- **`.kul-query-dim` has exactly one owner.** #303 adds a source to the registry and inherits both the
  union semantics and the live-read exemption; it does not add a class, and it must not strip one.
- **`QuerySurface` grows `setDimExemption`**, which `mount.ts` does not call — it is for slices that
  compose inside the surface.
- **The `Query` closure is mirrored into `engine-wire.ts`** under ADR-0012's discipline, including the
  filter types `Query` references. #303 inherits them already mirrored and drift-checked.
- **Two reserved paints remain** in ADR-0038's carve-out: `--kul-hue-query-uncertain` and
  `--kul-hue-query-path`.

## Anti-suggestions (do not re-propose)

- **"Do it in one call: sweep `any{4,4}` and bucket the members by descriptor."** It is a second
  kinship matcher in the consumer (ADR-0025's exact line), it makes every count a client-side length
  (#301's exact line), and it cannot answer unbounded `ancestors` / `descendants` without silently
  capping them. The one-call answer is a batched op in the *engine*.
- **"Fetch `members` for every row at open so every row can gloss."** Thirteen `members` calls instead
  of thirteen `count` calls, member lists nobody reads, and counts that are lengths again. The gloss
  is worth exactly one row's worth of data, which is the row the reader painted.
- **"Synthesise a descriptor per kin set so an unpainted row can be phrased."** It fabricates a path
  backbone the engine never emitted, and it cannot be done at all for the sets that span genders or
  span shapes. Phrasing invented data is the provenance mistake by a new route.
- **"Group the rows — thirteen is a lot for a flat list."** #276 prototyped exactly that taxonomy
  (Immediate / Lineage / Extended / In-laws / Step) and its resolution replaced it with "one flat
  scrollable list … **no category grouping**". The list scrolls; the taxonomy was the thing readers
  had to learn first.
- **"Show the matched people in a list beside the tree — the paint is hard to scan."** The tree *is*
  the result (#276, PRD-0006). A list would compete for the place where answers are read, and it is
  also why `sort` has no surface in the preview at all.
- **"Dim the edges too — 'non-matches dim' should mean everything."** The edges are how a reader sees
  why a set has the shape it has. Paint recedes the non-answer; it does not recede the picture.
- **"Make the empty-set notice an error-popover row — the popover already exists."** An absence the
  engine reported is an answer. Putting it in the diagnostics surface says the query failed, which is
  precisely the reading PRD-0006's honesty stance is written to prevent.
- **"Show `0` until the count arrives; it is almost always right."** A zero is an answer. Showing one
  the engine has not given is the surface asserting a fact it does not have, in the one list whose
  whole point is that absence stays absence.
- **"Register a sync-suspension reason for the kin paint."** It cannot exist without a selection, and
  the selection already registers one. A reason that can never be the last to lift is a set entry with
  no behaviour.
- **"Bind the panel's phrased slots with `locale.bind` — it is the same controller."** The panel
  rebuilds its element on every draw, so a binding would outlive the element it was made against and
  leak one per redraw. `bind` is for chrome that persists and re-phrases in place; the panel is
  chrome that redraws wholesale. The idiom follows the lifetime.
- **"Keep `pack()` on the surface and let #302 add its own locale option."** Two options onto one
  controller is two paths to one source of truth, and it puts the lens's `bind` outside the seam every
  other phrased surface goes through.
- **"Broaden `refresh()` to redraw every phrased surface, so the lens rides it too."** #302 dismisses
  the lens on repaint. A name that promised to redraw it would be false about the one surface it was
  widened for.
- **"Let each paint source own its own dim class."** Two `opacity` values on nested cards multiply to
  0.09. One class, one owner, union of sources.
- **"Have the kin repaint strip `.kul-query-dim` — it is stateless paint."** It runs on every render,
  so it would silently clear #303's filter dim whenever the document changed. A source withdraws
  itself; only `reset` strips.
- **"Let the ambient dim win over the lens trace — the kin set is the active question."** The kin set
  is the question the reader asked a moment ago; the lens is the one they are asking right now, with
  the pointer. A trace rendered at 30% through the persons that justify it is a trace that fails at
  its one job.
- **"Let the list keep its painted set when the selection walks — the reader can re-read it."** The
  teal would then be an answer about somebody the violet is no longer on. The *list* stays open; the
  *answer* does not.
- **"Keep the paint when the list closes — the reader may want the tree lit and the panel small."**
  The list is the only thing that names the question the teal answers. Closing it leaves lit cards and
  a dimmed tree with nothing on screen explaining either, which is the same defect as an unsourced
  count.
- **"Guard the count sweep and the row paint with one generation counter."** Painting a row would
  invalidate the sweep still in flight, and the memo would never re-arm — every row `·` until the
  reader selects someone else. Two questions, two guards; the sweep's is the anchor, because the
  anchor is the only thing that can make its answers wrong.
- **"Keep the memo once claimed — a re-issued sweep is thirteen more calls."** It is, and a stranded
  claim is a permanently dead list for an anchor whose project happened to be mid-edit when the reader
  opened it. The claim marks work in flight; it is released whenever the sweep does not fully answer.

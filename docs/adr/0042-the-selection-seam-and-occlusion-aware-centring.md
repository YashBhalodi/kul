# ADR 0042 — The selection seam: a target is an address, a store is the seam, and centring measures the panel

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[ADR-0035](./0035-detail-surface-one-selection-one-batched-lookup.md) decided *what* the detail
surface is — one selection over three entity kinds, three panel variants, one batched lookup, no
tooltip, reveal on the panel header — and [ADR-0036](./0036-two-tier-theming-and-the-accessibility-non-goal.md)
refined it: the panel floats, centring targets the *visible* region, and accessibility is a stated
non-goal. [ADR-0037](./0037-batched-detail-lookup-shape.md) shaped the operation and
[ADR-0038](./0038-tier-1-register-overlay-stack-and-the-reserved-carve-out.md) gave the chrome its
regions. [#300](https://github.com/YashBhalodi/kul/issues/300) built all of it.

None of them decided the **shape of the thing three later slices attach to**. The Explore-kin list
([#301](https://github.com/YashBhalodi/kul/issues/301)), the hover lens
([#302](https://github.com/YashBhalodi/kul/issues/302)) and the mode boundary
([#304](https://github.com/YashBhalodi/kul/issues/304)) all need to read the selection, react to it
changing, and in two cases move it. Building that seam raised three questions the existing ADRs
leave open, each with an answer that reads as an arbitrary style choice unless it is written down.

1. **How is a selection spelled?** ADR-0035 speaks of selecting persons, marriage edges and adoption
   edges. ADR-0037 independently defines `DetailTarget` for addressing exactly those three things.
   Whether they are one vocabulary or two was never said.
2. **What do later slices attach to?** "The selection" is a value; three consumers need to *observe*
   it, and one of them (#304) needs to end it from outside the chrome that owns it.
3. **What does "centre in the visible region" compute?** ADR-0036 states the requirement and the
   reason. It does not say which side the panel is assumed to be on, or what happens when it is on
   neither.

## Decision

### A selection *is* a detail target

`EntitySelection` is `DetailTarget` — the same tagged union, not a parallel one: `{kind:"person",
id}`, `{kind:"marriage", id}`, `{kind:"adoption", childId, marriageId}`. What the reader clicked
goes to `queryDetail` without translation.

A separate selection type was the alternative, and it buys nothing: it would have the same three
variants with the same fields, plus a mapping function whose only failure mode is drifting from the
wire. ADR-0037 already refused a synthetic adoption id on the grounds that a language whose ids are
author-written should not grow a second identity scheme; adding a second *selection* scheme over the
same three addresses is that refusal again, one layer up. The adoption pair in particular is
load-bearing: it is the pair the rendered edge already carries (ADR-0021), the pair ADR-0037 takes,
and the one thing `RevealTarget` cannot address — which is why the adoption panel reveals its child.

The consequence to be honest about: the preview's selection vocabulary is now pinned to a wire
type. Widening `DetailTarget` (ADR-0037 leaves `birth` as an additive variant) widens what is
*expressible* as a selection, and the chrome would have to decide what a birth selection means. That
is a real coupling, accepted because the alternative is two spellings of one idea.

### The seam is a store with a subscription, and the person anchor lives on it

`createSelectionStore()` returns `{current, select, clear, subscribe}`. Both mutators are
**idempotent** — re-selecting the selected entity and clearing an empty selection notify nobody — so
every notification a listener sees is a real transition and it can refetch unconditionally. That is
what makes the panel's "cancel the in-flight answer, ask again" loop correct without the listener
tracking previous state.

A plain callback passed at construction was the alternative. It works for one consumer and forces a
fan-out helper the moment there are four, which there will be: the panel, the kin list, the lens and
the sync-suspension hint. The store is ~40 lines; the fan-out would be the same lines with a worse
name.

`selectionAnchorPerson(selection)` sits on the seam rather than in each consumer. ADR-0035's rule
that the lens needs a person ego and a kin set needs a person anchor is one rule, and an edge
answering `null` is the machine-readable form of "an edge is a waypoint". Two slices re-deriving
`selection.kind === "person" ? selection.id : null` is two places for the waypoint rule to be
half-implemented.

**The waypoint rule itself is not a store concept.** It is the panel calling `select` with a person
id when a spouse or child row is clicked. There is no "selection stack", no "previous selection",
and no second selected entity — #276 rejected the pinned second-selection state across nine
prototype rounds, and a waypoint that remembered where it came from would be that state wearing a
different name.

### `mount.ts` sees one surface, and #304 composes the mode boundary from named parts

`createQuerySurface` composes the store, the paint, the panel, the panel-driven walk and the
editor-sync suspension. `mount.ts` reaches for six members and no more —
`handleCanvasClick` (a click inside the rendered SVG), `repaintQueryChrome` (a render swapped the
SVG), `syncHighlight` (an inbound editor highlight, which becomes the handle's `highlightEntity`),
`refresh` (the locale toggle changed language), `occupiedBox` (the ghost badge's jump steers around
the panel) and `dispose` — and knows nothing else about selection.

**`repaintQueryChrome()` is named for all of it, not for the selection.** It repaints only the
selection outline today, and `mount.ts`'s single post-render call is the *only* hook query chrome
gets: `render()` replaces the SVG that every piece of query paint was drawn on. #301's kin results
and #303's filter dim land on the same replaced picture, so a name scoped to the selection would
force each of them either to contradict the docstring or to bolt a second call into the mount.
Widening the contract while there is exactly one implementation of it costs a line; doing it later is
a three-way coordination between slices that have already shipped.

The surface deliberately offers **`clearSelection()`, not `clear()`**. It bottoms out in
`selection.clear()`, which is a documented no-op when nothing is selected, so a general-sounding name
would quietly encode *query mode == a selection exists* — a claim that is true today and false the
moment [#303](https://github.com/YashBhalodi/kul/issues/303)'s filter can be active on its own, at
which point "clear query mode" would notify nobody and leave the filter up.
[#304](https://github.com/YashBhalodi/kul/issues/304) therefore does **not** get one call for free; it
composes the boundary from `clearSelection()`, the kin paint and the filter. Naming the part honestly
is what stops that composition from being discovered as a bug.

The same reasoning splits sync suspension off the selection. Suspension is a **set of reasons**, not a
predicate over the selection: the selection registers `"selection"` through
`setSyncSuspended(reason, suspended)`, #303's filter registers its own, the hint follows the set, and
sync resumes when the last reason lifts. #276 point 9 names both reasons in one breath; welding the
mechanism to one of them would have made the second a rewrite instead of a call.

**This slice does not call `clearSelection()` from `render`.** ADR-0035's "an edit ends query mode" is
#304's decision to implement, and until it lands a render repaints the surviving selection onto the
fresh SVG rather than leaving a selection with no paint. Anticipating the rule here would have made
#304 a no-op on its headline behaviour and split one decision across two slices.

### Suspending sync has to strip, not merely stop painting

An inbound `highlightEntity` is **held, not dropped**, while sync is suspended, and replayed when the
last reason lifts. Dropping it would mean Esc resumes sync to a blank tree until the reader moves the
cursor again.

The symmetric half is easier to miss and matters more: **entering suspension repaints the sync
highlight away.** In the normal VSCode path the editor cursor is on some entity, so a magenta sync
outline is usually *already on screen* when the reader clicks a card. A guard that only refuses
future highlights leaves that one painted, and the two meanings of "highlighted" co-paint — the exact
outcome #276 point 9 exists to prevent. `setSyncSuspended` therefore calls `applySyncHighlight(null)`
on the false→true edge while leaving the held ref intact, so Esc still restores it.

### The float layer carries two placements, and both are declared

The details panel hugs a stage edge; [#302](https://github.com/YashBhalodi/kul/issues/302)'s lens pill
docks under a *hovered card*. Those are two placements, and ADR-0038 named only the second ("floats
positioned by JS against an entity's screen box").

The layer therefore carves a **dock slot**. `.kul-region-float` stays a bare fixed layer, and
`.kul-region-float-dock` — declared in the stage markup alongside the regions — fills it as a flex
column that owns the edge, the inset and the gap. Edge-docked chrome joins the dock; entity-anchored
chrome is appended to the layer itself and positions absolutely, unaffected by the dock's flow. #302
picks one rather than discovering that the region it was promised now has a `justify-content`.

Making the whole layer a flex container was the shipped-in-review shape and is rejected: it gives
every member one placement, so #302's pill would have had to opt out of its own region — which is the
"invent an inset" practice ADR-0036 retired, arriving from the other direction. A separate sixth
region was the other option, rejected because the two placements share a stacking layer and a
pointer-events posture; splitting them into peers would duplicate both.

### The panel keeps the answer it drew, so it can be redrawn

`QuerySurface.refresh()` re-renders the open panel from the cached `EntityDetail` — no engine call, no
selection change. Without it there is **no lever at all**: the detail was discarded after drawing, and
`select` on the already-selected entity is correctly a no-op, so nothing could redraw an open panel.

That is not hypothetical. #301 puts phrased kin rows in this panel, and
[ADR-0041](./0041-the-gu-pack-and-the-locale-toggle.md)'s locale toggle re-reads everything on screen
when the reader flips language. The toggle's subscription therefore calls `refresh()` **now**, even
though it currently redraws identical words: the panel's present vocabulary is chrome — section
titles, `birth` / `adoption`, `from` / `until` — and #276 point 6 keeps chrome labels English while
the locale setting governs kinship phrasing.

What that buys is narrow and worth stating narrowly: **the locale → redraw wire is done**, so #301
does not have to discover that an open panel had no way to be redrawn. It does *not* mean phrased
rows are free. `buildDetailPanel` takes a detail and nothing else, so putting a phrased term in the
panel still owes a `LanguagePack` threaded through `createQuerySurface` → `createDetailPanel` →
`buildDetailPanel`. That threading is #301's, and it is real work.

One consequence to know rather than discover: `refresh()` rebuilds the panel element, so the panel's
scroll position resets. Harmless at today's height; #301's kin list is what makes it felt.

### Centring measures the panel and takes the wider free strip

`visibleCentre(viewport, occluder)` clamps the occluding box to the viewport, measures the free strip
on each side, and centres in the wider one. The panel's box is **measured**, never assumed: nothing
in the geometry knows the panel hugs the right edge today.

Hard-coding "the panel is on the right, so centre in `[0, panelLeft]`" was the cheaper option and was
rejected. ADR-0038 put placement in the region, precisely so a float can move without a widget rule
changing; a centring rule that encodes the current edge would silently un-do that the first time the
region's `justify-content` flips, and the failure mode — cards parked behind the panel — is exactly
the one ADR-0036 introduced occlusion-aware centring to prevent. The acceptance criterion for #300
says to verify both sides, which is only meaningful if both sides are representable.

Two narrower calls fall out. **Only the horizontal axis moves**: the panel is a full-height side
float, so the vertical centre is never the crowded one, and shifting it would fight the reader's
expectation that a centred card is vertically centred. And a panel that leaves **no** free strip
falls back to the raw centre rather than refusing to pan — an unreachable case with today's fixed
panel width, and "don't move" would be a worse answer than "move imperfectly".

The panel's width is a fixed token for the same reason: the visible region must not move under the
reader as a panel's content changes height between selections.

### The panel is a model plus a renderer, so provenance is testable

`buildDetailPanel(detail) -> DetailPanelModel` is pure, and the DOM layer consumes only the model.
ADR-0035's acceptance criterion — panel content comes only from the batched operation — is then two
assertions rather than an argument: a lint over `detail-panel.ts`'s own source text (no `data-`, no
`getAttribute`, no `querySelector`, no `closest`), and a behavioural test that renders a picture
whose card labels deliberately disagree with the engine's names.

Row identity travels in a **closure**, not on a `data-*` attribute of the panel's own. A widget that
stamps ids onto its DOM and reads them back has an attribute vocabulary again — a different one, but
the same shape of mistake, and it would defeat the source lint's plain reading.

## Consequences

- **Three later slices attach to one small surface.** `SelectionStore` (`current` / `select` /
  `clear` / `subscribe`) plus `selectionAnchorPerson` is the whole contract; #301, #302 and #303 need
  no new seam. #304 composes its mode boundary from `QuerySurface.clearSelection()` plus the surfaces
  its siblings add — deliberately not one call, because "query mode" outgrows "a selection exists".
- **#303 registers its own sync-suspension reason** rather than widening a condition, and the hint
  follows the reason set.
- **#302 has a placement waiting for it.** The float layer's dock takes edge-docked chrome; the layer
  itself takes entity-anchored chrome. Neither invents an inset.
- **ADR-0041's locale toggle already drives `QuerySurface.refresh()`**, and #301's kin rows inherit
  the wiring; the panel retains the answer it drew, so a redraw costs no engine call.
- **The chrome's selection vocabulary is pinned to `DetailTarget`.** A wire change that widens the
  union widens what a selection can be, and the chrome must answer for it.
- **A render repaints rather than clears, until #304.** That is a deliberately temporary state,
  named here so it is not read as a missing case.
- **`repaintQueryChrome()` is the one post-render hook for query paint.** #301 and #303 repaint from
  inside it rather than adding a second call to `mount.ts`.
- **`panToElement` gained an occluder parameter** and the ghost badge's jump-to-canonical passes it
  too, so both navigation paths steer around the panel rather than only the new one.
- **The reserved query violet gains its first consumer**, so `--kul-hue-query-selection` leaves
  ADR-0038's `RESERVED_PENDING_CONSUMERS` carve-out — which is the converse assertion that carve-out
  was written to force. Three reserved paints and the filter alpha remain.

## Anti-suggestions (do not re-propose)

- **"Give the chrome its own `Selection` type and map it to `DetailTarget`."** Same three variants,
  same fields, plus a mapping whose only behaviour is to drift. ADR-0037 refused a second address
  vocabulary for adoptions; this is the same refusal.
- **"Let the store remember the previous selection so an edge can go 'back'."** That is #276's pinned
  second-selection state under another name, rejected across nine prototype rounds. The waypoint rule
  — a panel row selects a person — solves the same problem with one selection.
- **"Clear the selection on render now; #304 is only about the filter."** #304's headline rule *is*
  the mode boundary. Implementing half of it here leaves one decision split across two slices and two
  places to look when a selection disappears.
- **"Rename `clearSelection()` back to `clear()` — #304 can just call it."** It reads as "end query
  mode" and is a no-op when nothing is selected, so once a filter can be active alone the call would
  silently do nothing. The narrow name is what makes #304 write the composition it actually needs.
- **"Suspend sync with `if (selection.current)` — there is one reason today."** #276 point 9 names
  two, and the second (#303's filter) is not a selection. The reason set costs fifteen lines and
  turns a rewrite into a call.
- **"Drop the held sync ref instead of replaying it — the cursor will move again."** It will not,
  necessarily. Esc would then resume sync to a blank tree, which reads as sync being broken rather
  than resumed.
- **"Suspension only has to stop painting new highlights."** It does not: the cursor is usually
  already on an entity, so a sync outline is usually already up when the reader clicks. Not stripping
  it is the co-paint bug, in the most common path there is.
- **"Make the whole float layer a flex container — the panel is its only member."** Until #302's lens
  pill, which anchors to a hovered card and would have to opt out of its own region's flow. The dock
  slot keeps both placements declared.
- **"Let the panel drop the detail after drawing; a redraw can just re-query."** `select` on the
  selected entity is a no-op by design, so there is no redraw to hang a re-query off — and paying a
  check to re-render words that did not change is the composition ADR-0035 measured and rejected.
- **"Narrow `repaintQueryChrome()` back to the selection — that is all it repaints."** Today, yes.
  It is the single hook a render gives query paint, and the two slices that add paint land on the
  same swapped-out SVG. A name that has to be widened later is widened by three parties instead of
  one.
- **"Assume the panel is on the right; it is."** Today. ADR-0038 made placement a property of the
  region so it can stop being true, and the failure mode is silent — a card centred behind the panel
  looks like a pan that did nothing.
- **"Let the panel stamp `data-person-id` on its rows; it is the panel's own attribute."** It is an
  attribute vocabulary the widget then reads back, which is the shape of the provenance mistake
  ADR-0034 and ADR-0035 both refused, and it would blunt the source lint that keeps them honest.
- **"Add `aria-label` to the panel's person rows — they are buttons."** ADR-0036 anti-suggests this
  directly. The rows are `<button>` elements because that is the honest element for a mouse-driven
  command; they carry no `role`, no `aria-*` and no `:focus-visible`, and the surviving chrome keeps
  everything it has.

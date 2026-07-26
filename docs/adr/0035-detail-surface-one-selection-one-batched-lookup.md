# ADR 0035 — The detail surface: one selection over three entity kinds, fed by one batched lookup

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[#276](https://github.com/YashBhalodi/kul/issues/276) settled that selection populates a fixed
right-edge details widget, and explicitly narrowed this ticket: *"the query-context detail surface
is decided (the widget); #281 narrows to the hover tooltip's relationship to it."*
[ADR-0034](./0034-query-transport-and-result-node-identity.md) put the engine in the webview and
made detail lookups per-entity; [#292](https://github.com/YashBhalodi/kul/issues/292) then measured
that path. This ADR decides what the widget contains, where its data comes from, and what happens
to the two shipped behaviours it collides with. It resolves
[#281](https://github.com/YashBhalodi/kul/issues/281) on the
[query-UX wayfinder map (#275)](https://github.com/YashBhalodi/kul/issues/275).

Five facts were established before deciding, because each one moved an answer:

1. **A hover tooltip already ships.** [`tooltip.ts`](../../packages/preview/src/tooltip.ts) mounts on
   `.kul-card, .kul-edge` with a 350 ms hover-intent delay, reads `data-*` straight off the SVG
   ([ADR-0021](./0021-language-properties-plumb-to-svg.md)), auto-surfaces new fields through a
   structural denylist, and already covers three entity kinds — Person, Marriage and **Adoption**.
   #276 gave hover to the lens.
2. **Click already ships too, and means something else.** [`mount.ts:82`](../../packages/preview/src/mount.ts)
   binds click on person cards *and* marriage edges to `adapter.onRevealRequest({kind: "entity", id})`.
   #276 gave click to selection. The seam keys on **entity id, not `span`**, so nothing in the
   export shapes is needed to drive it.
3. **The pinned contract cannot answer the marriage panel.** `queryMarriage` returns
   `ExportedMarriage | null` — `{id, spouses[2], start, end, endReason, span}` — with **no
   children**, and no query operation exists for parenthood links at all. ADR-0034's fact 5 noted
   the SVG carries the children-with-link-kinds list; it did not notice that
   [ADR-0024](./0024-query-seam-and-envelope.md)'s surface does not.
4. **Ghosts are a layout concept, not a query one.** [ADR-0019](./0019-ghost-model-and-bio-anchor.md)
   emits one ghost per past intimacy inside `kul-layout`. The engine has no notion that a person was
   drawn three times.
5. **Renders mean edits.** The preview re-renders only from `onDidChangeTextDocument`
   ([`extension.ts:519`](../../editor/vscode/src/extension.ts)), debounced at 300 ms. "A render
   arrived" and "the document changed" are the same event.

## Decision

### One selection, three entity kinds

Click selects. Person cards, marriage edges and adoption edges are all selectable, and the widget
renders a variant per kind. #276's "single selection, always" is unchanged in number — it is widened
from persons to entities.

A **person** selection gets the full model: detail panel, Explore kin, and the live hover lens. An
**edge** selection gets a panel only — the lens needs a person ego and a kin set needs a person
anchor, so neither has meaning there. This is not a dead end: the marriage panel's spouse rows and
the adoption panel's child row are clickable and move the selection to a person. An edge is a
waypoint.

Selecting edges is what keeps direct manipulation honest — you see a line, you click it — and it is
the only way `queryMarriage`, a pinned ADR-0024 operation, gets a first-class surface. Every
marriage is also reachable through either spouse, so the alternative (person-only selection with
marriage detail expanded inline) was available and was rejected: it costs the obvious gesture to buy
a coherence the waypoint rule already provides.

### The hover tooltip retires

`tooltip.ts`, its tests, and its mount are deleted. The widget is the only detail surface.

The narrower option — suppressing the tooltip on person cards only while a selection exists, since
the lens has no meaning on edges and none at all without an ego — was considered and rejected. It
preserves more, but it makes hover mean two different things depending on a state the user has to
hold in their head, and it leaves two detail surfaces to design, theme ([#284](https://github.com/YashBhalodi/kul/issues/284))
and keep in step. One surface per question is worth the shipped behaviour it costs.

**This is a deliberate deletion of a working feature**, and it is the price of hover becoming the
lens. Zero-commitment "what is this person" reading goes away; reading a person now costs a click.

### Everything comes from the engine, through one new batched operation

The widget is fed by a single new WASM operation. Given an entity id it returns, **in one check**,
that entity's own fields plus its relational neighbourhood — parents, spouses, and children carrying
their `biological` / `adoptive` link kinds. The same operation supplies the kin list's row labels.

Two alternatives were rejected. **Reading the panel off the SVG** is free and ADR-0021's
plumb-through is exhaustive, but it makes the chrome depend on the attribute vocabulary rather than
on ADR-0024's single-sourced export shapes — ADR-0034 already refused this, and fact 3 does not
change the principle, only the amount of work required to honour it. A **hybrid** — contract for own
fields, SVG for the relational lines — was rejected for the same reason: it buys cheapness with two
provenance paths for the same panel.

The operation is **batched rather than fine-grained** because #292 measured both. Composing
`queryPerson` with three `queryKin` calls costs ≈63 ms at the 10k ceiling — over
[ADR-0029](./0029-query-engine-performance-posture.md)'s 50 ms budget for a single click, because
every stateless call re-pays the check. One check with any number of lookups behind it is **flat at
≈13 ms**, N-independent. The batched shape is ADR-0034's own first-named out, and adopting it here
retires #292's eager-hydration finding outright rather than working around it: a 20-row kin list
goes from 276 ms to ≈13 ms, and list size stops mattering.

This adds Rust work and a release to the epic's critical path, and pins a surface shape ADR-0024 did
not. That is accepted knowingly, in exchange for one provenance path across the whole widget.

> **The operation's shape is decided by [ADR-0037](./0037-batched-detail-lookup-shape.md) (#306)** —
> a list of `DetailTarget`s in (person id, marriage id, or an adoption's `(childId, marriageId)`
> pair), a list of `EntityDetail` unions out, in the order asked, `null` per target that names no
> entity. The CLI is deliberately left without a matching verb.

### What the panel shows

**Absent fields are omitted**, never rendered as "not recorded". This mirrors the wire exactly —
`ExportedPerson` and `ExportedMarriage` both use `skip_serializing_if` — so the panel shows what the
engine sends and nothing else. The cost, accepted: a missing death date is invisible rather than
loud, and the panel's height varies as the selection walks.

**Each marriage occupies its own line, with its span and end reason**, rather than a comma-joined
spouse list. This is load-bearing rather than cosmetic: it is what carries the explanation for why a
person appears on more than one card.

**There is no ghost note.** The prototype carried one ("also appears as a past-record ghost"); it is
dropped. Sourcing it from the engine would re-derive ADR-0019's rules inside the query layer, where
they would have to stay in step with `kul-layout` forever; sourcing it from the rendered cards would
re-open the second provenance path this ADR just closed. The ended marriage on the panel is the
underlying truth, and being drawn twice is a rendering consequence of it.

> **Refined by [ADR-0036](./0036-two-tier-theming-and-the-accessibility-non-goal.md) (#284)** — the panel floats over the canvas rather than docking, so the centring below targets the **visible** region rather than the raw viewport. A panel-driven walk therefore never parks a card behind the panel.

**Every person named anywhere in the widget is clickable.** Clicking moves the selection *and* pans
the canvas to centre that person's canonical card. The pan is not a flourish: the panel is precisely
how off-screen ancestors get discovered, and a selection the user cannot see is a dead end. This is
a genuinely new navigation path — #276's kin list *paints* results on the tree, it does not move the
selection.

### Reveal-in-editor moves from the tree to the panel

Click on the tree no longer reveals. Revealing becomes an explicit control on the panel header,
posting the same `onRevealRequest({kind: "entity", id})` the tree posts today — the seam is unchanged
on both sides, and it now works for marriages as well as persons.

The reasoning is #276's own: it suspends editor sync while a query selection exists so the two
meanings of "highlighted" never fight. Revealing on every click would drag the editor's scroll
position around from the other direction while the user walks the family. Making it deliberate keeps
both.

**Adoption links have no entity id**, so `RevealTarget` cannot address one. Since `adoption` is a
sub-statement of the person, the adoption panel's reveal targets the **child**.

### An edit ends query mode

A render clears the selection, its panel, any kin paint, **and** [#277](https://github.com/YashBhalodi/kul/issues/277)'s
attribute filter. The tree returns to its plain state and editor sync resumes.

Fact 5 makes this rule crisp: renders only ever come from document changes, so there is no class of
repaint that would clear a selection surprisingly. It also makes #276 point 9 self-completing —
sync suspends while a selection exists, an edit clears the selection, so sync resumes without the
user pressing anything.

Refetching the bundle on every render was the alternative, and it is affordable (≈13 ms per 300 ms-debounced
render). It was rejected in favour of the mode boundary: querying and authoring are separate
activities, and no query artefact outlives the source it was computed from. The cost, accepted: a
one-character typo fix costs the user their selection, their painted kin set, and their filter.

## Consequences

- **Two shipped behaviours are deleted**: the hover tooltip, and click-to-source on the tree. Both
  removals are in this epic's scope, and both need to be called out in the epic's change notes —
  users will notice.
- **The epic gains a Rust dependency on its critical path.** The detail panel cannot ship before the
  new WASM operation does, which means a `kul-wasm` change, a version bump and a release inside the
  epic rather than pure preview work.
- **ADR-0034 is superseded in two places.** Its "detail is engine-backed per entity; the kin list
  hydrates eagerly" becomes the batched operation — its own first named out, chosen on #292's
  measurements. Its consequence *"a re-render no longer destroys query state"* is reversed: an edit
  destroys all of it, deliberately. The transport decision itself is untouched.
- **#292's recommendation is superseded, to a better end.** It recommended taking kin-list labels off
  the cards; the batched operation serves them from the contract instead, so the widget keeps **one**
  provenance path rather than the two ADR-0034 priced.
- **The widget is now a navigation surface, not just a readout.** Panel-driven selection plus canvas
  centring is new behaviour over `svg-pan-zoom`. ~~It needs a keyboard equivalent — which belongs to
  [#284](https://github.com/YashBhalodi/kul/issues/284), along with the focus order of a panel whose
  every name is now a control.~~ **Answered by [ADR-0036](./0036-two-tier-theming-and-the-accessibility-non-goal.md) (#284):**
  there is no keyboard equivalent and no focus order — accessibility is a stated non-goal, and the
  panel's name controls are mouse-driven.
- **Three panel variants must be designed and themed**, not one. #284's `--kul-*` token coverage
  applies to all three.

## Anti-suggestions (do not re-propose)

- **"Keep the tooltip for quick reads and let the widget handle the deep dive."** Considered and
  rejected. It sounds cheap but it means hover changes meaning based on whether a selection exists,
  two detail surfaces to keep in step, and two things to theme. The tooltip's job is now the panel's.
- **"Read the panel off the `data-*` attributes — it's free and it's already on screen."** True, and
  rejected twice now: ADR-0034 refused it on principle, and this ADR refused the hybrid version of it
  even after discovering the contract cannot answer everything. The fix is to widen the contract, not
  to route around it. It remains the fallback if the Rust work proves unaffordable.
- **"Compose the panel from `queryPerson` + `queryKin` × 3 instead of adding a new operation."**
  Measured at ≈63 ms at the ceiling (#292) — over ADR-0029's budget for one click, because each
  stateless call re-pays a 13 ms check. The batched shape costs one operation instead of two and
  fixes the kin list in the same stroke.
- **"Render absent fields as 'not recorded' so absence stays honest."** The prototype did this and it
  was rejected: the wire already omits absent fields, and the panel should show what the engine sends.
  Honest emptiness (#276 point 8) is about *query results*, where the alternative would be inventing
  a relationship; a blank field is not the same claim.
- **"Note on the panel that this person also appears as a ghost."** Rejected on provenance: the
  engine cannot know it without re-deriving ADR-0019 inside the query layer, and the DOM cannot
  supply it without re-opening a second provenance path. The per-marriage line with its span and end
  reason is the honest form of the same information.
- **"Let a marriage selection keep the previous person selected so the lens stays live."** That is
  the pinned second-selection state #276 rejected across nine prototype rounds. The waypoint rule —
  spouse rows move the selection to a person — solves the same problem with one selection.
- **"Preserve the selection across edits by restoring it by id."** Rejected: it re-introduces the
  per-render refetch, and it blurs a mode boundary that is doing real work — while a selection
  exists, editor sync is suspended, and an edit is the user saying they are done querying.
- **"Reveal on click as well as select — it costs nothing."** It costs the editor's scroll position on
  every step of a walk, which is the jumpiness #276 suspended sync to prevent, arriving from the
  other direction.

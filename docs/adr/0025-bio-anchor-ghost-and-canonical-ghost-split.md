# ADR 0025 — Bio-anchor ghost (`GhostReason::PastBirth`) and the P8/P16 canonical-placement-vs-ghost-emission split

**Status:** Accepted
**Date:** 2026-05-24
**Deciders:** owner
**Related:** [ADR-0017](./0017-render-shape-schema-and-versioning.md), [ADR-0021](./0021-render-shape-family-tree-rooted-at-person-card.md)
**Closes:** [#160](https://github.com/YashBhalodi/kul/issues/160)

## Context

A person with both a `birth` link and one or more `adoption`s has
their canonical card at the most-recent adoption (P16 chain step 2).
Today the bio-birth `Edge` flows from the bio marriage bar straight
to that canonical card. Visually this reads as *"Sam's bio identity
is anchored at his current intimacy,"* which is the opposite of what
canonical placement is supposed to communicate — adoption demotes the
bio family from current to past intimacy, so the bio-birth edge
should anchor locally to a bio-side ghost the same way P16 already
anchors past adoptions with a child-ghost.

`examples/09-multi-adoption` is the worked case. Sam is biologically
born to Ravi + Priya, was first adopted by Anita + Bharat (past), and
is currently adopted by Chen + Dara (canonical). P16 already emits a
past-adoption child-ghost under Anita + Bharat. The symmetric
child-ghost under Ravi + Priya is what this ADR introduces.

The existing P8 / P16 text tangled two concerns: *where a canonical
card sits* (P8 + the most-recent-adoption rule lived together) and
*when a ghost surfaces to anchor a past intimacy's edges* (P8's
past-marriage ghost lived inside the placement principle; P16
handled past-adoption ghosts separately). Adding a third ghost flavor
would have made the tangle worse — bio-anchor ghosts share emission
semantics with past-adoption and past-marriage ghosts but motivation
that's closer to "the canonical card is somewhere else, so the bio
family's edge needs a local anchor."

## Decision

Five coordinated decisions, landed in one PR.

### D1. `GhostReason::PastBirth` variant

Add a third `GhostReason` alongside `PastMarriage` and `PastAdoption`:

```rust
pub enum GhostReason {
    PastMarriage,
    PastAdoption,
    PastBirth,
}
```

`RENDER_SCHEMA_VERSION` stays at `2`. Additive enum variants are
backward-compatible at the wire boundary — schema-1 consumers that
relied on exhaustive `GhostReason` pattern matches would already
panic on `PastAdoption`; schema-2 consumers gain one more variant to
handle. The integer bump that ADR-0021 already paid covers this
addition; no further bump is warranted (per ADR-0010, the schema
integer is a discriminator at the consumer boundary, not a release
identifier).

### D2. P8/P16 split along canonical-placement vs. ghost-emission

P8 becomes *"Canonical card sits at current intimacy"* — a tight
statement of the priority chain (first-declared un-ended marriage >
most-recent adoption > bio family > orphan) plus the host /
joining-spouse placement details that follow from it. The
past-marriage-ghost text moves out of P8 into P16.

P16 becomes *"Past intimacies emit ghosts to anchor their edges"* —
one emission rule with three concrete applications:

- Past-marriage spouse-ghost (existing behavior).
- Past-adoption child-ghost (existing behavior).
- Past-bio child-ghost (new).

Each ghost slots into the past family's children/spouse row at
**source-order position** — the same declaration-order key the
children row uses for canonical siblings. This rule applies uniformly
to all three flavors; it retroactively pins past-adoption ghost
positioning (previously under-specified — every shipped past-adoption
ghost was an only-child so the rule never surfaced).

The principle numbers P9–P15 are unchanged. Both principles keep
their pre-existing numbers; only the text under each changes.

### D3. Derived-from-canonical trigger for the bio-anchor ghost

The bio-anchor ghost fires whenever a person has a `birth` link AND
their canonical card is not at the bio family — i.e. P8's canonical
chain selected an intimacy other than bio. This is a *semantic*
trigger (derived from canonical placement), not a *data-shape*
trigger ("any person with `birth` + at least one `adoption`").

In `crates/kul-render/src/build.rs`, `build_children` reads as one
loop over persons in declaration order, dispatching to one of three
mutually-exclusive branches: canonical child, past-adoption ghost,
or past-bio ghost. The trigger collapses into

```rust
facts.bio_marriage.as_deref() == Some(marriage_id)
    && !matches!(
        index.canonical_location(facts),
        CanonicalLocation::ChildOf(ref id) if id == marriage_id,
    )
```

`canonical_location` already encodes P8's chain (chain step 1 picks
the primary marriage; chain steps 2/3 fall through to
`canonical_family()`, which resolves canonical adoption ahead of
bio). The trigger therefore stays in step with any future P8 chain
amendment without needing to be re-derived.

### D4. Source-order positioning applied uniformly

`build_children` interleaves canonical children and P16 child-ghosts
in person-declaration order. The single iteration is what produces
the corpus output for `examples/09`: ghost-Sam (declared first) sits
left of canonical-Bro in `m_ravi_priya`'s children row.

For past-adoption ghosts the merged iteration is observationally
equivalent to the old two-pass code because no shipped example has a
past-adoption ghost sharing a children row with a canonical sibling.
The rule is documented here so the implementation matches the
principle even where the corpus doesn't yet stress it.

### D5. Layout-side child-ghost endpoint generalisation

`crates/kul-layout/src/adapter.rs` already keyed past-adoption ghosts
by `(person_id, marriage_id)` so the dashed edge terminates locally
rather than crossing the canvas. The same map now tracks
`PastBirth` ghosts too — one variant added to the existing
`matches!` arm, the field renamed from `past_adoption_ghost_marriage`
to `child_ghost_marriage` to match its broader scope. No new routing
path; the bio-birth edge slots into the same orthogonal bus-and-drop
geometry the past-adoption edge already uses.

## Consequences

- **Multiple corpus snapshots regenerate.** `examples/09` gains
  ghost-Sam alongside the new canonical-Bro under `m_ravi_priya`.
  `examples/11` (cousin marriage), `examples/13` (inter-family),
  and `examples/14` (grand-nested inter-family) gain past-bio ghosts
  for every joining spouse with a declared bio family. The
  `transform__p6_joining_spouse_birth_family_nests_at_connection_point`
  unit snap regenerates for the same reason. The diff is intentional.
- **P6 inter-family cross-tree edges become local.** Bob's bio-birth
  edge in `examples/13` used to traverse the gen-0/gen-1 gutter from
  his canonical card (at `m_alice_bob`'s joining slot) back to
  `m_bob_parents`'s bar. It now terminates on the local ghost-Bob in
  `m_bob_parents`'s children row — a short vertical drop. P6's
  *nesting* (Bob's birth family as a sibling-root sub-tree at the
  joining-spouse connection point) is unchanged; only the bio edge's
  endpoint moves. The P6 worked example text was updated in step.
- **Cousin-marriage joining cousin gains a bio-anchor ghost.**
  `examples/11`'s Nikhil — host of `m_bharat_janaki` becomes the bio
  family demoted by his joining of `m_maya_nikhil` — gets a
  ghost-Nikhil in `m_bharat_janaki`'s children row. The within-family
  cross-edge from canonical-Nikhil to the bar becomes a local short
  edge to ghost-Nikhil; P11's "absorb rule applies uniformly" is
  unchanged at the principle level.
- **Edge routing classification.** Edges whose endpoints both resolve
  on a local ghost now route as `EdgeRouting::InTree` rather than
  `EdgeRouting::CrossTree` (the structural-edges set is populated
  during the children-row walk, which now includes child-ghosts).
  This flips the CSS class `kul-edge--cross-tree` →
  `kul-edge--in-tree` for the affected edges. Geometry is identical
  per ADR-0018; only the discriminator changes.
- **Three crates change in lockstep.** `kul-render` emits the new
  ghost flavor; `kul-layout` extends its child-ghost endpoint
  registration; `kul-svg` adds one match arm. No downstream consumer
  (CLI, LSP, WASM) needed a code change beyond regenerating
  snapshots.

## Anti-suggestions (do not re-propose)

- **"Use a data-shape trigger: any person with `birth` AND ≥ 1
  `adoption`."** Tight to the motivating example, but it bakes
  today's P8 chain into the trigger. A future chain amendment (e.g.
  ADR-0021 already amended chain step 1 from "most-recent un-ended"
  to "first-declared un-ended") would silently desynchronise the
  trigger from the canonical placement rule. The derived-from-
  canonical trigger reads "wherever P8 doesn't pin the canonical card
  at the bio family" and stays in step automatically.
- **"Generalise the variants: `Past { intimacy: Marriage | Adoption |
  Birth }`."** Symmetric on paper, but the three flavors don't share
  a `Past`-shaped data payload — past-marriage carries `end:` /
  `end_reason:`, past-adoption carries the adoption `start:`,
  past-bio carries nothing distinctive. Three flat variants stay
  pattern-matchable in one line; the nested form would require an
  extra match arm at every consumer for zero structural benefit.
- **"Keep P8 and P16 entangled — one combined principle '*canonical
  card + past intimacy ghost*.'"** Two distinct concerns
  (where-to-place vs. when-to-ghost) deserve two principles. The
  split makes the bio-anchor ghost the third application of one
  emission rule rather than a new vocabulary flavor; without the
  split, P16 would have grown a fourth orthogonal clause every time
  a new past-intimacy kind landed.
- **"Fix this in `kul-layout` by re-routing the bio-birth edge to a
  bio-family-side endpoint without changing `RenderShape`."** The
  pattern is "ghost emission is structural data," not "ghost emission
  is layout policy." A layout-side fix would put the ghost's
  visual_row, source-order position, and edge endpoint outside the
  data the render shape exposes — surface renderers other than
  `kul-svg` (e.g. an HTML / Canvas renderer) would have to
  re-implement the same routing decision. Keeping the ghost in
  `RenderShape` makes every consumer see the same picture.
- **"Drop `GhostReason` entirely — every ghost is just a ghost."**
  The discriminator is what lets surface renderers diverge on visual
  vocabulary if they want to (e.g. tooltip text "past marriage" vs.
  "previous adoptive family" vs. "biological family"). The CSS class
  is uniform today (`kul-card--ghost`) but the data discriminator
  keeps that a renderer choice, not a structural loss.

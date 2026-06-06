# ADR 0019 — The ghost model: one emission rule, three flavors, and the bio-anchor

**Status:** Accepted
**Date:** 2026-06-07
**Deciders:** owner

## Context

The canonical pattern places a person's canonical card at their *current* intimacy ([ADR-0017](./0017-render-shape-schema-and-versioning.md)'s selection rule, the **current-intimacy placement** principle). A person carries other structural facts too — a past marriage, a demoted adoption, a biological family the chain did not select. Each of those facts owns edges: a divorced couple's children still descend from their marriage; a biological birth still happened. Those edges need somewhere to attach.

Attaching them to the canonical card directly would drag a long edge across the canvas — from a past marriage bar three components away to the person's current card — and would conflate "this person is biologically Ravi and Priya's child" with "this person's current intimacy is over here." The pattern needs a local anchor in the past family: a visual duplicate of the person, slotted where their canonical card *would* sit if that family were current, carrying only the edges of that one past fact.

That anchor is a **ghost**. Ghost emission is a concern distinct from canonical placement — *where the canonical card sits* versus *when a past fact surfaces a ghost to anchor its edges* — and the two are kept as two principles. This ADR specifies the emission rule (the **past intimacies emit ghosts** principle) and one consequence of it that is easy to get wrong: the biological-family anchor.

## Decision

### One emission rule

Every intimacy a person carries that **current-intimacy placement** does *not* select is a **past intimacy**, and each past intimacy emits one ghost: a mute (the **ghosts are mute** principle), visually distinct duplicate of the person, slotted into the past family at the position that intimacy would occupy if it were canonical. The ghost's only job is to anchor the edges that would otherwise traverse the canvas. Three flavors apply the one rule:

- **Past-marriage spouse-ghost.** A marriage carries `end:` and the moved-out spouse's canonical card lives elsewhere (or the bar floats and its host has moved on). The ghost sits in the bar's slot the moved-out spouse occupied, so the bar's children edges still attach. A childless past marriage leaves no visual trace.
- **Past-adoption child-ghost.** A person has more than one adoption; the chain selects the most recent and demotes the rest. Each demoted adoption's bar gets a child-ghost connected by a dashed edge.
- **Past-bio child-ghost (the bio-anchor).** A person has a `birth` link but the chain selected a different intimacy (a marriage, or an adoption — which demotes the bio family from current to past). The bio marriage gets a child-ghost connected by a solid edge.

`GhostReason` carries `PastMarriage`, `PastAdoption`, `PastBirth` ([ADR-0017](./0017-render-shape-schema-and-versioning.md)). The reasons share emission semantics but stay three flat variants because each anchors a different edge kind and a surface renderer may want to distinguish them in chrome (a tooltip reading "previous adoptive family" versus "biological family"); the card class is uniform (`kul-card` with `data-kind="ghost"`) while the reason rides a `data-ghost-reason` attribute ([ADR-0021](./0021-language-properties-plumb-to-svg.md)), so the discriminator keeps divergence a renderer choice, not a structural loss.

### The bio-anchor fires on a derived-from-canonical trigger

The past-bio child-ghost fires whenever a person has a `birth` link **and** their canonical card is not at the bio family — i.e. the chain selected an intimacy other than bio. This is a *semantic* trigger, read off canonical placement, not a *data-shape* trigger keyed on field presence. In `kul-render`'s `build.rs`, `build_children` is one loop over persons in declaration order, dispatching each to one of three mutually exclusive branches — canonical child, past-adoption ghost, or past-bio ghost — and the bio-anchor branch collapses to:

```rust
facts.bio_marriage.as_deref() == Some(marriage_id)
    && !matches!(
        index.canonical_location(facts),
        CanonicalLocation::ChildOf(ref id) if id == marriage_id,
    )
```

`canonical_location` already encodes the chain, so the trigger reads "wherever the chain does not pin the canonical card at the bio family" and stays in step with any future change to the chain automatically.

### Source-order positioning, applied uniformly

Each ghost slots into its past family's children or spouse row at the **source-order position** the person would occupy if their canonical card were there — the same declaration-order key the children row uses for canonical siblings. This holds for all three flavors. In `build_children` a single declaration-order pass interleaves canonical children and child-ghosts, so a ghost declared before a canonical sibling sits to its left.

### The child-ghost endpoint is local

`kul-layout`'s adapter keys child-ghosts by `(person_id, marriage_id)` in one `child_ghost_marriage` map covering both past-adoption and past-bio ghosts, so the ghost's edge terminates on the local ghost rather than crossing the canvas. Resolving through that map is exactly the edge's `data-is-past="true"` predicate; the edge routes through the same one orthogonal geometry as any other (the former `InTree` / `CrossTree` discriminator was removed — see [ADR-0018](./0018-canonical-layout-algorithm.md)).

Ghost emission is **structural data**, not layout policy: the ghost, its source-order position, and its edge endpoint all live in `RenderShape`, so every surface renderer — not just `kul-svg` — sees the same picture without re-deriving the routing.

## Consequences

- **A past fact's edge is a short local drop, not a canvas-spanning line.** A joining spouse's biological birth edge terminates on a ghost in their birth family's children row — the same orthogonal bus-and-drop a within-tree birth edge uses — rather than reaching back from their canonical card at the host bar.
- **The cousin marriage composes cleanly.** A cousin who joins a marriage and is thereby demoted from their bio family gains a past-bio ghost in that bio family's children row; the within-family connector becomes a short local edge to the ghost, and the host's lineage tree positions exactly as it would without the cross-family link.
- **One rule scales to a future link kind.** The discriminator is semantic (derived from canonical placement), so a future spec extension introducing a new family-unit link would slot in as a fourth flavor without restructuring the rule.
- **Three crates render the same fact in lockstep.** `kul-render` emits the ghost, `kul-layout` registers its local endpoint, `kul-svg` adds one match arm; no downstream consumer needs a code change beyond regenerating snapshots.

## Anti-suggestions (do not re-propose)

- **"Use a data-shape trigger for the bio-anchor: any person with `birth` AND ≥ 1 `adoption`."** It bakes today's chain into the trigger; a later change to the chain would silently desynchronise the trigger from canonical placement. The derived-from-canonical trigger stays in step automatically.
- **"Generalise the three flavors into `Past { intimacy: Marriage | Adoption | Birth }`."** Symmetric on paper, but the flavors do not share a payload — past-marriage carries `end:` / `end_reason:`, past-adoption carries the adoption `start:`, past-bio carries nothing distinctive — and the nested form needs an extra match arm at every consumer for zero structural benefit.
- **"Fold canonical placement and ghost emission into one principle."** Two distinct concerns (where to place versus when to ghost) deserve two principles; folding them would grow one principle a new orthogonal clause every time a past-intimacy kind landed, and would hide the fact that the bio-anchor is the *third application of one emission rule* rather than a new vocabulary flavor.
- **"Anchor the bio-birth edge at a bio-family-side endpoint in `kul-layout` without emitting a ghost in `RenderShape`."** That puts the ghost's position and endpoint outside the data the render shape exposes, so a non-`kul-svg` surface renderer would have to re-implement the routing decision. Ghost emission is structural data; keeping it in `RenderShape` makes every consumer see the same picture.
- **"Drop `GhostReason` entirely — every ghost is just a ghost."** The discriminator is what lets a surface renderer diverge on ghost chrome if it wants to. The uniform CSS class today is a renderer choice, not a reason to discard the data.

# ADR 0021 — `FamilyTree` rooted at a `PersonCard` (not a `MarriageBranch`)

**Status:** Accepted
**Date:** 2026-05-24
**Deciders:** owner
**Supersedes (in part):** [ADR-0017](./0017-render-shape-schema-and-versioning.md) — the `FamilyTree.root: MarriageBranch` decision

## Context

[ADR-0017](./0017-render-shape-schema-and-versioning.md) pins
`ComponentKind::FamilyTree { root: Box<MarriageBranch> }`: a component
is a marriage and its descendants, and the marriage's two spouses are
two equal-weight `CardSlot`s on the bar itself
(`MarriageBar.host_slot` and `MarriageBar.joining_slot`). That shape
treats the bar as the structural primitive a family tree hangs off,
with the host slot subordinate to the bar.

[P8](../canonical-ui-pattern.md#p8-canonical-card-at-current-intimacy-child-anchoring-ghost-for-past-marriages)
already says something stronger: *"the marriage bar's canonical
location is the host's birth-family slot at the position the host
occupied within their birth family at the time the marriage was
declared."* The host is the conceptual root of the bar's placement,
not a peer of the joining spouse. ADR-0017 mirrored the two slots on
the bar for symmetry, which works as long as a person has at most one
current-intimacy bar.

Issue [#142](https://github.com/YashBhalodi/kul/issues/142) made the
asymmetry load-bearing. A person with two concurrent un-ended
marriages (pure-host polygamy: `examples/04-polygamous-family/`) has
N bars but [P2](../canonical-ui-pattern.md#p2-one-canonical-card-per-person)
says **one** canonical card. The today's "most-recent un-ended
marriage" rule for `primary_marriage` (in `crates/kul-render/src/build.rs`)
let one bar host the canonical card and ghosted the polygamous
person at the other — silently violating P2 when the user's intent is
"multiple concurrent intimacies, all current." The fix needs a
structural primitive that admits N bars under one canonical card; the
current `FamilyTree.root: MarriageBranch` shape can't express that
without inventing a parallel `PolygamyHub` variant.

The companion structural follow-up
([#144](https://github.com/YashBhalodi/kul/issues/144)) tracks the
mixed-role case (a person host in some marriages, joining in others)
where the relocated bar's structural treatment still needs design
work; that issue is deferred.

## Decision

`ComponentKind::FamilyTree` is rooted at a `PersonCard`, not a
`MarriageBranch`:

```text
ComponentKind::FamilyTree { root: Box<PersonCard> }
```

`PersonCard` already carries the right shape — a `slot: CardSlot`
plus `hosted_marriages: Vec<MarriageBranch>` — and the existing
`MarriageBranch` recursive children stay unchanged. The root
`PersonCard` is the outermost canonical host of the component (or a
ghost-rooted `PersonCard` for the past-ended floating-bar fallback;
see below).

`MarriageBar` drops its `host_slot` field. The host face of every
bar is the parent `PersonCard.slot` in the tree — implicit via
position rather than duplicated on the bar. `host_id` stays (still
useful for consumers cross-referencing by id).

The root `PersonCard.slot.kind` is normally `Canonical`, but the
past-ended floating-bar case (`examples/08-divorce-and-remarriage/`'s
`m_alice_bob` component, where both spouses have moved on so the bar
exists only to anchor Carol's birth edge) roots the component at a
**ghost** `PersonCard` whose `slot.kind = Ghost(pastMarriage)` and
whose `hosted_marriages` carries the past-ended bar. Every `FamilyTree`
is thus rooted at a `PersonCard`, canonical or ghost. The data
permits the same person to appear as both a canonical `PersonCard` in
one component and a ghost `PersonCard` rooting another — mirroring
today's `CardSlot`-level canonical/ghost duplication.

`RENDER_SCHEMA_VERSION` bumps from `1` to `2`. The change is
structural (root variant changes; one field disappears from
`MarriageBar`), so consumers built against schema 1 would silently
mis-represent the data; per ADR-0010 / ADR-0017's discipline that is
exactly what the integer bump is for.

The selection rule for which un-ended marriage anchors the canonical
card (used in `Index::new`'s `primary_marriage` computation) flips
from "most-recent un-ended (by `start:` date)" to "first-declared
un-ended participation (by source order across hosted ∪ joined)" —
the P8 amendment recorded in `canonical-ui-pattern.md`. The field
name `primary_marriage` is kept (semantic shift, name stable; rule of
three says don't rename for a single use-site).

## Consequences

- **All corpus snapshots regenerate.** Every example renders through
  the new root shape; the structural difference is visible in every
  `corpus__example_*.snap` even when the kinship semantics are
  unchanged.
- **The bug-codifying snap retires.**
  `transform__p8_polygamy_picks_most_recent_unended_as_primary.snap`
  is replaced by a new test
  `p8_pure_host_polygamy_shares_canonical_anchor` asserting one
  component with both un-ended bars on the polygamous host's single
  `PersonCard`.
- **Downstream snapshots regenerate.** `crates/kul-layout`,
  `crates/kul-svg`, the `kul-wasm` `renderSvg` test, and the kul-lsp
  `kul/render` integration test all consume the render shape; the
  refactor regenerates their snapshots even though their visual
  output for the non-polygamy examples is structurally equivalent.
- **Tsify-derived types regenerate.** `RenderShape` is not exposed
  through the WASM bridge today (the bridge returns SVG strings, not
  the shape), so `crates/kul-wasm/types/kul_wasm.d.ts` does **not**
  change from this refactor. Future surfaces that do expose
  `RenderShape` will pick up the new variant shape automatically.
- **`kul-layout`'s adapter simplifies.** `build_branch_root` becomes
  `build_person_root` and reuses the existing `NodeKind::PersonHost`
  path — root and child PersonCards now flow through one code path.
  The `NodeKind::RootMarriage` variant disappears.
- **Mixed-role polygamy stays a known-incomplete case.** The new
  selection rule applies cleanly; the structural treatment of bars
  that need to relocate (a person host in some and joining in others)
  is tracked separately in
  [#144](https://github.com/YashBhalodi/kul/issues/144). Tests for
  mixed-role are deliberately not added here — they would lock the
  wrong behavior.
- **Two un-related canonical-card decisions kept independent.** The
  selection rule (which un-ended bar anchors the canonical card) and
  the root shape (PersonCard vs. MarriageBranch) are separate
  decisions. ADR-0017's selection rule is what `canonical-ui-pattern.md`
  documents and what ADR-0021 amends; the root shape is what this
  ADR specifies.

## Anti-suggestions (do not re-propose)

- **"Add a `PolygamyHub` parallel variant alongside `FamilyTree`."**
  Two structural primitives for "a person with hosted marriages"
  (one for N=1, one for N>1) doubles the consumer's pattern-match
  surface for no benefit — the existing `PersonCard` already carries
  `hosted_marriages: Vec<MarriageBranch>` and a `Vec` of length 1 is
  not a special case.
- **"Keep `FamilyTree.root: MarriageBranch` for past-ended floating
  bars only; switch to PersonCard for current bars."** Two root
  shapes for the same `ComponentKind` push the discriminator into
  consumer code instead of the data. The ghost-rooted PersonCard
  (Q12 A.1) gives one uniform shape — `FamilyTree` is always rooted
  at a `PersonCard`, canonical or ghost — and the ghost variant
  re-uses the `SlotKind::Ghost { reason: PastMarriage }` vocabulary
  already shipped.
- **"Synthesize a new `GhostBarRoot` primitive for past-ended
  floating bars (Q12 A.3)."** Same shape of objection as A.2: a new
  variant adds a third top-level shape consumers must handle, when
  the existing `PersonCard` + `SlotKind::Ghost` combination already
  expresses "a ghost rooting a bar."
- **"Use the schema integer to communicate the polygamy fix
  semantically (`schema: 1.1`)."** ADR-0010 settled this — the
  schema integer is a discriminator at the consumer boundary, not a
  release identifier. The bump from `1` to `2` is the right
  discipline regardless of the semantic motivation behind it.
- **"Add a Visitor trait over the new root shape."** ADR-0001's
  anti-suggestion applies here too — `Component` is a 2-variant enum
  and `ComponentKind::FamilyTree { root: PersonCard }` is one variant
  of two. Pattern matches stay clearer than a visitor at this scale.

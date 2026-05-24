# ADR 0026 — Polygamy hub must also be the declared host of every concurrent marriage

**Status:** Accepted
**Date:** 2026-05-25
**Deciders:** owner
**Related:** [ADR-0021](./0021-render-shape-family-tree-rooted-at-person-card.md), [ADR-0025](./0025-bio-anchor-ghost-and-canonical-ghost-split.md)
**Closes:** [#164](https://github.com/YashBhalodi/kul/issues/164)

## Context

The `host` role is a **per-marriage** concept: the spec
([§4.2](../../spec/04-top-level-statements.md#42-marriage-statement))
names the first-listed spouse as host, and the canonical UI pattern
(P3) uses host-ness to decide layout, ordering, and visual anchoring
inside *that one* marriage. The host concept knows nothing about
other marriages the host or joining spouse might also be in.

Polygamy introduces a **per-person** concept the spec did not have a
name for: a person with two or more un-ended marriages. The
canonical UI pattern (P8 amendment for #142, ADR-0021) renders that
person as a single canonical card hosting every concurrent bar
side-by-side; the [fan rendering primitive](../canonical-ui-pattern.md)
(future sibling issue) takes this further by treating that person as
the visual anchor for every concurrent intimacy.

The two concepts coincide in **pure-host polygamy**: one person hosts
every concurrent marriage, and the fan primitive's "visual anchor"
matches the spec's "first-listed spouse" with no ambiguity. They
**diverge** in two other shapes:

1. **Mixed-role concurrent polygamy** — alice hosts m_alice_bob and
   is the joining spouse in m_devraj_alice. Both marriages are
   un-ended. Alice is the per-person hub (two un-ended marriages) but
   only the per-marriage host of one of them.
2. **Pure-join concurrent polygamy** — alice is the joining spouse
   in two un-ended marriages (m_bob_alice, m_devraj_alice). Alice is
   the per-person hub of both, the per-marriage host of neither.

The original #144 attempt to handle mixed-role at the renderer level
(scrapped as #163) introduced a `relocated_joining_bars` field on the
render shape so the renderer could repair the divergence by
re-anchoring the joining-side bar to alice's card. The repair
sneaked the hub concept into the rendering layer through a back door
without ever giving it a name in the language. The fan primitive's
semantics became ambiguous in exactly the cases it was supposed to
unify, and authors had no syntactic signal that the document was
structurally ambiguous from a layout perspective.

## Decision

Align hub and host **by language invariant**, not by renderer repair.
The new validator rule R14 — *polygamy hub must host all un-ended
marriages* — rejects mixed-role and pure-join concurrent polygamy at
check time. Authors fix violations by swapping the two spouse
identifiers in the offending marriage; there is no override field
and no renderer-level escape hatch.

The fan rendering primitive (sibling issue) ships against the post-R14
language and assumes clean input: every polygamy hub is also the
host of every concurrent marriage it participates in. The primitive's
semantics are unambiguous because the language guarantees one
canonical layout per hub.

Mutual hub-conflict (both spouses of one un-ended marriage each have
≥2 un-ended marriages) is rejected unsatisfiably: no swap can
satisfy both spouses, so R14 fires on the marriage where the joining
spouse is a hub and the message points the author at the only
authentic fix — ending one of the conflicting marriages so the
document represents at most one current polygamy structure.

The diagnostic is one per offending marriage, anchored at the
marriage id, grouped in source order. The message reproduces the
`Currently:` and `Fix:` lines so the author sees the exact swap
without re-reading the spec.

## Consequences

- **Mixed-role and pure-join concurrent polygamy are rejected at
  validation time.** Authors swap spouse identifiers to fix. Past
  marriages (with `end:`) are unaffected: R14 only counts un-ended
  marriages, so sequential mixed-role (a hosted past marriage plus a
  current joining-spouse marriage) stays valid and renders as a
  ghost-anchored past intimacy per P8/P16.
- **The renderer's polygamy primitive ships without
  `relocated_joining_bars` or any similar repair machinery.** The
  fan primitive (sibling issue) consumes clean input. `kul-render`
  loses the conditional anchor-swap branch that #163 would have
  required; the canonical card's children-set and edges resolve
  exactly per host-ness.
- **R14's primary span is the offending marriage's id token.** The
  message's `Currently:` and `Fix:` lines spell out the swap, so
  authors do not need to consult the spec to understand the fix.
  Code-action providers can offer a one-shot swap edit by reading
  the marriage and swapping `spouse_a` / `spouse_b`; no `detail`
  tag is needed because R14 has a single sub-case.
- **R14 walks marriages in project-wide source order.** Mirrors R02
  and the other per-file rules in diagnostic ordering. Cross-file
  marriages are part of the same hub count (per ADR-0015): a hub
  declared in one file with marriages declared in another is still
  caught.
- **Mutually-polygamous marriages report exactly one R14.** The
  joining spouse's hub status condemns the marriage; swapping
  shifts the diagnostic to the other spouse without satisfying
  R14, so the author must end one of the conflicting marriages.
  R14 deliberately stays terse on this branch — the renderer-side
  fan primitive is the canonical reading of polygamy, and a
  document that cannot map onto it does not have a deterministic
  layout (P7).
- **R14 does not fire on marriages whose spouse positions are
  unresolved or wrong-kind.** R02 already condemns those; cascading
  R14 onto an R02-broken marriage would be a misleading second
  error rather than a distinct violation. The hub count itself
  also excludes such marriages, so an R02-broken marriage neither
  triggers R14 nor inflates another person's hub count.

## Alternatives considered and rejected

1. **Renderer-level relocation (the scrapped #163).** Repair the
   divergence by re-anchoring the joining-side bar at the hub's
   card. Sneaks the hub concept into the rendering layer without
   naming it; leaves the language permissive in a way that makes
   the fan primitive's semantics ambiguous in exactly the cases
   the primitive was meant to unify. Rejected.
2. **Leave admissible and do not render.** Treat mixed-role and
   pure-join concurrent polygamy as syntactically valid but
   layout-undefined. Incompatible with P7 — every document has a
   deterministic layout, and a class of valid documents with no
   defined layout breaks the contract the rendering pipeline
   relies on. Rejected.
3. **Introduce a `host: true / false` override field on `marriage`.**
   Lets the author opt into a different host per marriage without
   reordering spouses. Adds a new field (and a new ADR-0014-shaped
   maintenance burden) for a problem that swapping two
   identifiers already solves. Authors do not currently feel a
   gap here — R14's diagnostic already spells out the swap, and
   the swap is one line. Rejected; revisit if a future epic
   surfaces authoring patterns where swap-to-fix becomes
   awkward at scale.

## Anti-suggestions (do not re-propose)

- **"Relax R14 to admit mixed-role for cultural or authorial
  reasons."** The fan primitive's semantics depend on R14 holding;
  relaxing the rule re-opens the entire structural ambiguity #163
  tried to repair. If a culture-specific authoring pattern
  surfaces that R14 forbids, the fix is to teach the spec to
  *accept* that pattern in a controlled way (e.g. a new top-level
  statement, with its own ADR), not to weaken R14 globally.
- **"Resurrect `relocated_joining_bars` for the mutually-polygamous
  corner case."** The corner case is rejected on purpose; mutual
  hubs is exactly the configuration that has no deterministic
  fan layout, and a renderer-level repair would re-introduce the
  ambiguity ADR-0026 is closing.
- **"Add a code-action that auto-swaps spouses."** The diagnostic
  already spells out the swap. A code action would carry minimal
  additional value over the message text and would couple the
  validator's diagnostic to a quick-fix payload. Revisit only if
  the swap-edit workflow proves friction-heavy in practice.

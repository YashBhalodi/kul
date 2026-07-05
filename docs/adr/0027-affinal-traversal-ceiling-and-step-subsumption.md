# ADR 0027 — Affinal traversal: the fixed two-hop ceiling, the affinity scalar, and step subsumption

**Status:** Accepted
**Date:** 2026-07-05
**Deciders:** owner

## Context

The kinship query engine ([ADR-0025](./0025-kinship-query-engine-contract-and-traversal.md)) and the relationship descriptor ([ADR-0026](./0026-relationship-descriptor-and-path-identity.md)) pinned the contract and the blood-only traversal (lineal, then collateral). The `across` (marriage) hop, the `affinity` dimension, and the `MarriageStatus` type were **defined but not produced** — reserved so the wire contract never reshapes. This ADR pins the decisions that activate them: the affinal traversal slice (issue [#258](https://github.com/YashBhalodi/kul/issues/258), PRD 0005).

Three of those decisions are non-obvious enough to outlive the code that a future agent might otherwise "simplify" away.

## Decision

### The affinal ceiling is fixed at two hops — semantics, not a knob

A path may cross **at most two** marriage hops, at any position (start, middle, end), consecutive allowed. This bound is **fixed semantics and is never exposed as configuration.**

The forcing argument: no human culture lexicalizes three affinal hops. "Spouse's sibling's spouse's sibling" names nothing in any kinship system; there is no term for it to key on, so producing it would be noise no terminology layer could ever consume. The ceiling is therefore a property of the *model*, not a caller's budget.

This is the semantics side of the **semantics-vs-budget line**. The generation cap on a lineal query (`ancestors_of(x, depth)`) *is* a caller budget — the caller legitimately wants "grandparents but not great-grandparents", and unbounded is a meaningful request. The affinal ceiling is the opposite: there is no legitimate caller who wants three affinal hops, so no knob is offered. An `affinalHops` **filter** narrows *within* the ceiling (a caller can ask for exactly one marriage hop), but it can never raise the ceiling — the engine clamps to two.

Consecutive affinal hops are nonetheless required *within* the ceiling: the co-spouse (*sautan* / co-wife) shape is ego → spouse → spouse's other spouse, two consecutive `across` hops, and the polygamy corpus needs it.

### `affinity` is a three-way scalar decided by hop position

`affinity` (`blood | step | inLaw`) is derived mechanically from the `across` hops and their positions:

- no `across` hop ⇒ `blood`;
- **every** `across` hop in *ancestor position* ⇒ `step`;
- **any** `across` hop not in ancestor position ⇒ `inLaw`.

`inLaw` wins a mixed path; the [path backbone](../../CONTEXT.md) keeps the full per-hop truth for anything finer.

**Ancestor position** is the load-bearing definition: an `across` hop is in ancestor position iff it is preceded by at least one hop and *every* preceding hop is `up`. A path-initial `across` is therefore **not** ancestor position. This is exactly the line between a step-parent (`up`, `across` → the spouse of an ancestor → `step`) and a parent-in-law (`across`, `up` → an ancestor of a spouse → `inLaw`); between a father's second wife (`step`) and a spouse's mother (`inLaw`). The derived step definitions (step-parent, step-ancestor gen N, step-sibling, step-child) all fall out of this one rule plus the vertical hop counts.

A zero-vertical-displacement path with `across` hops is a `self`-classification, `inLaw` member: the spouse (one `across`) and the co-spouse (two consecutive `across`).

### Step subsumption: a step shape is suppressed by the real edge it stands in for

A step path is a *derived stand-in* for parenthood, not an independent fact. It is **suppressed, not emitted alongside**, when the underlying fact is real:

- a step-parent shape (lineal ancestor 1, `step`) to a person who is also an actual (bio or adoptive) parent of ego → suppressed;
- a step-child shape (lineal descendant 1, `step`) to an actual child → suppressed;
- a step-sibling shape (collateral 1/1, `step`) to someone who shares ≥1 actual parent with ego (a full/half sibling) → suppressed.

An **explicit adoption edge always beats the step reading** — adoptive parents live in the same parent set as bio parents, so an adopted-by-step-parent relationship emits as the adoptive parent, never doubled as a step-parent.

This is deliberately *not* a contradiction of [path identity](./0026-relationship-descriptor-and-path-identity.md) (one member per distinct path, no collapsing). Double cousins are two independent **true** paths and stay two descriptors. A step path shadowed by a real parent edge is **one fact derived two ways** — the derived-stand-in is dropped, the ground-truth edge kept. Only `step` affinity is subject to subsumption; `blood` and `inLaw` paths are never suppressed (path identity governs them fully — a person reachable as both a half-sibling and, via two marriages, an in-law, is two members).

## Consequences

- The `AffinalWalk` composes 1–3 blood segments (each `up* down*`) joined by ≤2 `across` hops, gated behind an affinal budget so blood-only queries keep their dedicated lineal/collateral walkers and their snapshots unchanged.
- `sharing` / `apexSeniority` are computed at the **first** `up`→`down` junction on the path even when `across` hops bracket it — this is what delivers *jeth/devar* and *jethani/devrani* (husband's elder/younger brother and his wife), whose junction compares the brother to the husband.
- `side` is `notApplicable` for a path-initial `across` (a spouse's kin has no maternal/paternal side *of ego*); a father's wife (`up`, `across`) is `paternal`.
- Ended marriages are traversed identically to ongoing ones and tagged `ended` with the verbatim end reason. Filtering on marriage status is a downstream UX/terminology decision the core never makes.
- The additive pattern surface (`PatternClassification::Any`, the `affinity` / `affinalHops` filters, the five affinal sugars) rides the committed-tsify discipline ([ADR-0012](./0012-tsify-derived-types-committed-and-diffed.md)) — new fields, never a reshape.

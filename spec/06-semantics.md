# 6. Semantics

## 6.1 Reference resolution

A Kula document is parsed in two phases. In the first phase, all top-level `person` and `marriage` statements are collected and indexed by ID. In the second phase, all references — marriage spouses, `birth.<marriage-id>`, and `adoption.<marriage-id>` — are resolved against the indexed declarations. Statements may appear in any order; forward references are permitted.

A reference to an undeclared identifier is an error (see [Section 7 — Validation rules](./07-validation-rules.md), rule 2).

## 6.2 Active marriage at time T

A marriage M is **active at time T** if and only if:

1. `M.start ≤ T`, and
2. `M.end` is absent OR `M.end > T`, and
3. For each spouse S of M: `S.died` is absent OR `S.died > T`.

Condition 3 means a marriage becomes inactive when either spouse dies, even though the marriage record itself is not modified. This is how Kula reconciles "death does not end the marriage" (the marriage's stored fields are unchanged) with the practical query of who is currently married to whom (which uses this derivation).

## 6.3 Parenthood derivation

The **biological parents** of a person P are:

- If P has a `birth` sub-statement referencing marriage M, then the two spouses of M.
- Otherwise, P's biological parents are undocumented.

The **adoptive parents** of a person P, at time T, are:

- The set of all couples `{ spouses of M : M is referenced by an `adoption` sub-statement on P with `start ≤ T` and (`end` absent OR `end > T`) }`.

The set of P's parents at time T is the union of biological parents (if any) and currently-active adoptive parents. P may have more than two parents at a single time T if adopted by an additional couple while biological or earlier adoptive parents remain documented.

## 6.4 Derived kinship terms

Standard kinship terms (sibling, half-sibling, cousin, grandparent, in-law, etc.) are NOT first-class concepts in Kula. They are derivable from the person and marriage graph:

- **Siblings**: persons sharing both biological parents.
- **Half-siblings**: persons sharing exactly one biological parent.
- **Step-siblings**: persons whose parents are spouses in a common marriage but who do not share a biological parent.
- **Grandparents**: parents of parents.
- **Cousins, in-laws, etc.**: derivable similarly.

A v1 conforming implementation is not required to compute these terms; they are noted here for context and as a guide for downstream tools.

---

← [Section 5 — Person sub-statements](./05-person-sub-statements.md) | [Index](./README.md) | Next → [Section 7 — Validation rules](./07-validation-rules.md)

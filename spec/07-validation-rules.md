# 7. Validation rules

A conforming validator MUST report all of the following as errors. A document with any of these errors is **invalid**.

## Structural

1. **Duplicate ID within the project** — no two top-level statements (across `person` and `marriage`) may share an ID anywhere in the project. A Kul project (see [Section 14](./14-project-manifest.md)) is one logical namespace: an ID declared in one `.kul` file collides with the same ID declared in any sibling `.kul` file. The diagnostic anchors at the second declaration in file-discovery order (ties broken by byte offset within a file); a related-span points to the first declaration.
2. **Unresolved reference** — every `birth` marriage reference, `adoption` marriage reference, and marriage spouse reference must resolve to a declared ID in the project. Cross-file references resolve cleanly: an ID declared in any `.kul` file of the project is visible from every file.
3. **Required field missing** — a `person` MUST have `name` and `gender`. A `marriage` has no required named fields — the positional `id` and two spouse references are enforced by the grammar; `start:` is optional because dates are sometimes lost to record.
4. **Self-marriage** — a marriage's two spouse references MUST be distinct identifiers.
5. **End consistency** — a marriage's `end` field and `end_reason` field MUST both be present or both absent.

## Temporal impossibilities

For each comparison below, when a date has partial granularity it is treated as a range (e.g., `1925` denotes `1925-01-01..1925-12-31`), and the rule fires only if the comparison is violated for every date pair within the range. The `~` (circa) marker adds a tolerance of ±5 years to the date when the comparison is performed. A rule with a missing operand is silent (e.g. R09 / R10 do not fire when `marriage.start` is absent).

6. **Died before born** — `person.died < person.born`.
7. **Marriage end before start** — `marriage.end < marriage.start`.
8. **Adoption end before start** — `adoption.end < adoption.start`.
9. **Marriage before spouse born** — `marriage.start < S.born` for either spouse S.
10. **Spouse already dead at marriage start** — `marriage.start > S.died` for either spouse S.
11. **Bio child born before parent** — `child.born < P.born` for either biological parent P.
12. **Adoption before adopter born** — `adoption.start < P.born` for either adoptive parent P.

## Cycles

13. **Parenthood cycle** — combining all `birth` and `adoption` parent links across every `.kul` file in the project into a directed graph from child to parent, no person may appear as their own ancestor. (A cycle in this graph is an error.) Cycles that span multiple files are detected and reported as a single cycle.

## Polygamy

14. **Polygamy hub must host all un-ended marriages** — for each person `p`, let `un_ended_count` be the number of marriages where `p` is a spouse and the marriage has no `end:` field. If `un_ended_count ≥ 2`, `p` MUST be the [host](./04-top-level-statements.md#42-marriage-statement) (first-listed spouse) in every one of those un-ended marriages. The diagnostic anchors at the offending marriage's `<id>` token; one diagnostic per offending marriage. See [ADR-0020](../docs/adr/0020-polygamy-hub-and-fan.md).

## Fields

15. **Duplicate field** — a `person`, `marriage`, or `adoption` statement MUST NOT set the same field more than once. Fields are unordered and each may appear at most once per statement; because accessors take the first occurrence, a repeated field silently discards every later value, so the second and subsequent occurrences are errors. The diagnostic anchors at the duplicate occurrence's field name; a related-span points to the first occurrence. (A field whose value failed to parse is reported by the parser and does not additionally count toward this rule.)

## Manifest

The following codes are reported by the project-manifest validator pass against `kul.yml` (see [Section 14](./14-project-manifest.md)). They flow through the same diagnostic infrastructure as the `KUL-Rxx` rules above.

- **KUL-M01** — manifest not found at expected path. Unanchored.
- **KUL-M02** — manifest YAML malformed. Anchors at the line/column the YAML parser reported.
- **KUL-M03** — manifest is well-formed YAML but missing the required `kul:` field. Anchors at the manifest start.
- **KUL-M04** — manifest's `kul:` value is not a recognized Kul language version. Anchors at the value.
- **KUL-M05** — manifest carries an unknown top-level field. Severity warning; anchors at the field key.
- **KUL-M06** — project has a `kul.yml` but zero sibling `.kul` files. Anchors at the manifest start.

## Things explicitly NOT validated in v1

The following are NOT errors, even though they may indicate questionable data. Tools MAY surface them as informational hints, but they are not validation rules:

- A bio child's `born` date falling outside the parents' marriage interval (real-world cases include premarital conception with marriage during pregnancy, and post-divorce births of children conceived during marriage).
- A marriage with a recorded `end` date later than one of the spouses' `died` date.
- A person without optional fields (`born`, `family`, `given`).
- Cultural prohibitions on marriage (incest, same-gotra, sapinda restrictions).
- Reproductive age plausibility ("parent was 9 when child was born").

These exclusions reflect Kul's design priority of recording reality over enforcing legal or cultural norms.

---

← [Section 6 — Semantics](./06-semantics.md) | [Index](./README.md) | Next → [Section 8 — Worked examples](./08-worked-examples.md)

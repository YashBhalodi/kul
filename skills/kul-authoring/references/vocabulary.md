# Kinship vocabulary

Maps natural-language kinship terms to Kul. Most everyday terms are **derived** from the four primitives — only spouses, biological birth, and adoption are declared.

## The four declared primitives

| Concept    | Statement                                                                 |
| ---------- | ------------------------------------------------------------------------- |
| Person     | `person <id> name:"…" gender:… [born:…] [died:…] [family:…] [given:…]`    |
| Marriage   | `marriage <id> <spouse_a> <spouse_b> start:… [end:… end_reason:divorce]`  |
| Birth      | `birth <marriage-id>` (sub-statement of `person`; biological link)        |
| Adoption   | `adoption <marriage-id> start:… [end:…]` (sub-statement of `person`)      |

## NL → Kul mapping

| Natural-language term                            | Declared / derived | How it surfaces in Kul                                                                              |
| ------------------------------------------------ | ------------------ | --------------------------------------------------------------------------------------------------- |
| Spouse / husband / wife                          | Declared           | `marriage` with the two persons; the first-listed is the marriage's host (structural anchor).        |
| Ex-spouse / divorced                             | Declared           | `marriage … end:<date> end_reason:divorce`.                                                         |
| Widow / widower                                  | Derived            | Surviving spouse of a marriage whose other spouse has a `died:` date and no `end:` recorded.        |
| Child / son / daughter                           | Derived            | A `person` with a `birth` (bio) or `adoption` (adoptive) sub-statement pointing at parents' marriage. |
| Adopted child                                    | Declared (`adoption`) | `adoption <marriage-id> start:…` sub-statement.                                                  |
| Parent / father / mother                         | Derived            | Spouse of a marriage referenced by some person's `birth` / `adoption` link.                         |
| Step-father / step-mother                        | Derived            | Spouse of a parent's *other* marriage.                                                              |
| Sibling / brother / sister                       | Derived            | Two persons sharing a `birth` marriage-id.                                                          |
| Half-sibling                                     | Derived            | Shares exactly one biological parent.                                                               |
| Step-sibling                                     | Derived            | Parents are spouses in a common marriage but no shared bio parent.                                  |
| Grandparent / grandchild                         | Derived            | Parent-of-parent / child-of-child (walk `birth`/`adoption` two steps).                              |
| Uncle / aunt / cousin / niece / nephew           | Derived            | Walk siblings + their marriages.                                                                    |
| In-laws (mother-in-law, brother-in-law, …)       | Derived            | Spouse's relatives.                                                                                 |
| Founder / root ancestor                          | Derived (implicit) | `person` with no `birth` sub-statement.                                                             |
| Foundling (unknown bio parents)                  | Derived (implicit) | `person` with `adoption` sub-statement(s) but no `birth`.                                           |
| Polygamous spouse / co-spouse                    | Declared           | Multiple `marriage` statements for the same person (concurrent marriages are permitted). A person with ≥2 un-ended marriages is the **polygamy hub** and must be listed as host (first spouse) in every concurrent un-ended marriage (`KUL-R14`); the renderer surfaces them as a fan ([ADR-0027](https://github.com/YashBhalodi/kul/blob/main/docs/adr/0027-fan-primitive-for-polygamy-hubs.md)). |
| Same-pair remarriage                             | Declared           | Two `marriage`s with the same spouses, distinct ids (`m_x_y_1`, `m_x_y_2`).                         |
| Friend / godparent / fiancé / cohabiting partner | Not modeled        | Outside v1 scope. Record as a `#` comment if load-bearing.                                          |

When prose names a derived relation ("Alice's uncle Ravi"), resolve it back to declared primitives: who are Alice's parents, who shares those parents' marriage, where does Ravi fit. The [translation playbook](./translation-playbook.md) walks through this pattern.

## Notes

- `gender:` is required on every `person`. The enum is exactly `male | female | other`.
- Dates take the granularity of the literal: `1985-03-15`, `1985-03`, `1985`, or `~1985` (circa, ±5y). Absence of a date field is valid (except `start:` on a `marriage`); there is no `unknown` literal.
- Identifiers match `[A-Za-z_][A-Za-z0-9_-]*` and must avoid the 17 reserved keywords — see [spec §11 — reserved keywords](https://github.com/YashBhalodi/kul/blob/main/spec/11-reserved-keywords.md).

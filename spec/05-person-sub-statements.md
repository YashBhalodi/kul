# 5. Person sub-statements

Sub-statements appear as indented lines immediately following a `person` statement. They contribute additional facts to that person's record. Sub-statement order within a person is free. Two sub-statement kinds are defined: `birth` and `adoption`.

## 5.1 birth sub-statement

```
  birth <marriage-id>
```

Declares that this person was biologically born to the union represented by `<marriage-id>`. The two persons named as spouses in that marriage are this person's biological parents.

A person MUST have at most one `birth` sub-statement. Absence is permitted: a person without a `birth` sub-statement has undocumented biological parents and is implicitly a documentation root. No keyword is required to mark a documentation root; the additivity principle ensures that adding a `birth` sub-statement later does not require any change to the existing person line.

The date of biological birth is the person's `born` field; no separate date is needed on the `birth` sub-statement.

Example:

```
person carol name:"Carol Sharma" born:1975-09-03 gender:female
  birth m_alice_bob
```

## 5.2 adoption sub-statement

```
  adoption <marriage-id> start:<date> [end:<date>]
```

Declares that this person was adopted by the couple represented by `<marriage-id>`, becoming effective on `start:<date>`. If the adoption was later terminated, `end:<date>` records when.

A person MAY have zero or more `adoption` sub-statements; multiple adoptions over a lifetime are permitted. Adoptions are permanent unless `end` is given. There is no `end_reason` field on adoption sub-statements in v1.

Example:

```
person ravi name:"Ravi Sharma" born:~1980 gender:male
  adoption m_alice_bob start:1985-06-01
```

---

← [Section 4 — Top-level statements](./04-top-level-statements.md) | [Index](./README.md) | Next → [Section 6 — Semantics](./06-semantics.md)

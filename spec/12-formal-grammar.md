# 12. Formal grammar

The complete normative grammar for Kul 0.1 is provided as a standalone EBNF file:

> [`grammar.ebnf`](./grammar.ebnf)

Identifier productions are role-named (`person-id`, `marriage-id`, `person-ref`, `marriage-ref`) — they all share the same lexical form (`identifier`), but the grammar names what each position means. Field productions are enumerated per statement context, so the grammar shows which fields are allowed on which statement kind and what value type each field expects.

## What the grammar enforces

- The shape of each top-level statement (`person`, `marriage`) and sub-statement (`birth`, `adoption`).
- Which fields are syntactically permitted on each statement kind.
- The vocabulary of enumerated values (`gender-value`, `end-reason-value`).
- The lexical structure of identifiers, strings, dates, comments, and indentation.

## What the grammar does NOT enforce (left to semantics)

- Uniqueness of IDs across the whole document — see [Section 7 — Validation rules](./07-validation-rules.md), rule 1.
- Resolution of references against declared IDs — rule 2.
- Field non-duplication within a single statement (e.g., `gender:male gender:female` is rejected by the validator, not the grammar).
- Conditional requiredness (e.g., `end_reason` is required iff `end` is present) — rule 5.
- All temporal-impossibility rules and the parenthood-cycle rule — rules 6–13.
- The constraint that an identifier MUST NOT match a reserved keyword (lexically possible but semantically forbidden — see [Section 11 — Reserved keywords](./11-reserved-keywords.md)).

## Whitespace conventions in the grammar

Whitespace separates tokens within a line and is otherwise ignored, except that:

- Inside any field production (e.g., `name-field`, `start-field`), no whitespace is allowed between the field-name keyword, `:`, and the value.
- Inside a `date`, no whitespace is allowed between any of `~`, `year`, `-`, `month`, `-`, `day`.
- Leading whitespace on a line that is not a top-level statement constitutes `INDENT` and binds the line as a sub-statement of the most recent `person-stmt`.

---

← [Section 11 — Reserved keywords](./11-reserved-keywords.md) | [Index](./README.md) | Next → [Section 13 — Versioning policy](./13-versioning-policy.md)

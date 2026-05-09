# 2. Document structure

A Kul document is a sequence of **statements**, each starting on its own line. Top-level lines are person or marriage statements. Statement order is free; the parser performs reference resolution across the whole document.

The Kul language version a document targets is metadata *about* the source, not part of the source — it lives in the project manifest `kul.yml` alongside the `.kul` file. See [Section 14 — Project manifest](./14-project-manifest.md).

The valid top-level statement keywords are:

| Keyword    | Meaning                |
| ---------- | ---------------------- |
| `person`   | Person declaration     |
| `marriage` | Marriage declaration   |

Sub-statements (`birth`, `adoption`) appear as indented continuation lines following a `person` statement; they are not valid as top-level statements. See [Section 5 — Person sub-statements](./05-person-sub-statements.md).

A document MAY contain zero statements; this represents the empty family and is valid.

## Whitespace conventions

- **Within a line:** any amount of horizontal whitespace (spaces and tabs) separates tokens. Trailing whitespace is ignored.
- **Between lines:** blank lines are permitted anywhere and are ignored.
- **Indentation:** a line beginning with horizontal whitespace is a **sub-statement** of the most recent `person` statement. Lines that begin in column 1 are top-level statements. The exact amount of indentation is not significant; one or more whitespace characters suffices.

## Encoding

Documents are UTF-8 (no BOM). Line endings are LF or CRLF; a parser MUST accept either. Identifiers and keywords use ASCII; string literals (e.g., display names) may contain any valid UTF-8.

---

← [Section 1 — Introduction](./01-introduction.md) | [Index](./README.md) | Next → [Section 3 — Lexical structure](./03-lexical-structure.md)

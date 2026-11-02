# 2. Document structure

A Kul document is a sequence of **statements**, each starting on its own line. Top-level lines are person or marriage statements. Statement order is free; the parser performs reference resolution across the whole project (see [Projects](#projects)).

The valid top-level statement keywords are:

| Keyword    | Meaning                |
| ---------- | ---------------------- |
| `person`   | Person declaration     |
| `marriage` | Marriage declaration   |

Sub-statements (`birth`, `adoption`) appear as indented continuation lines following a `person` statement; they are not valid as top-level statements. See [Section 5 — Person sub-statements](./05-person-sub-statements.md).

A document MAY contain zero statements; this represents the empty family and is valid.

## Projects

A Kul **project** is a flat set of uniquely named `.kul` files. Each file is one document; the project is the family they describe together.

- **Membership.** A tool that reads a project from a directory MUST take every file directly inside that directory whose name ends in `.kul`, and nothing else. Subdirectories are not walked, and files with any other name (`README.md`, `family.kul.bak`, editor backups) are ignored.
- **One namespace.** Every id (`person` or `marriage`) declared in any of the project's files is visible from every file by bare name. There is no `import` statement, no namespace prefix and no qualified-reference syntax; the file boundary is purely organizational. Ids are unique across the whole project: two files declaring the same id is a duplicate-id error ([Section 7](./07-validation-rules.md), KUL-R01).
- **File order.** The files of a project are ordered by the byte-wise lexicographic order of their names, comparing the UTF-8 bytes (so `B.kul` precedes `a.kul`, and `10.kul` precedes `9.kul`). A project's **declaration order** is its files in file order, then source position within each file. Wherever order is observable — which of two duplicate declarations is the second ([Section 7](./07-validation-rules.md)), the order of an export's collections ([Section 15](./15-export-schema.md)) — tools MUST use declaration order, whatever order the files were listed or created in.
- **Empty projects.** A project with no files, or whose files contain no statements, describes the empty family and is valid.

A project carries no language-version declaration. Its documents are written against a version of this specification; a tool reports the version it implements (see [Section 13](./13-versioning-policy.md)).

## Whitespace conventions

- **Within a line:** any amount of horizontal whitespace (spaces and tabs) separates tokens. Trailing whitespace is ignored.
- **Between lines:** blank lines are permitted anywhere and are ignored.
- **Indentation:** a line beginning with horizontal whitespace is a **sub-statement** of the most recent `person` statement. Lines that begin in column 1 are top-level statements. The exact amount of indentation is not significant; one or more whitespace characters suffices.

## Encoding

Documents are UTF-8 (no BOM). Line endings are LF or CRLF; a parser MUST accept either. Identifiers and keywords use ASCII; string literals (e.g., display names) may contain any valid UTF-8.

---

← [Section 1 — Introduction](./01-introduction.md) | [Index](./README.md) | Next → [Section 3 — Lexical structure](./03-lexical-structure.md)

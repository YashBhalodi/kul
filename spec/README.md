# Kula Language Specification

**Version 0.1**

> Kula — a kinship description language.

This is the normative specification of the Kula language, version 0.1. A conforming parser, validator, or other tool for Kula must implement the rules contained in these documents. The specification is intentionally rigorous enough that an independent implementation can be written from it alone.

For background on what Kula is and why it exists, see [`../docs/vision.md`](../docs/vision.md). For the project root, see [`../README.md`](../README.md).

## How to read this spec

The spec is split across fourteen sections plus a standalone formal grammar. Read sequentially for first-time orientation; jump by section once familiar.

| #   | Section                                                         | What it covers                                                              |
| --- | --------------------------------------------------------------- | --------------------------------------------------------------------------- |
| 1   | [Introduction](./01-introduction.md)                            | The two primitives (Person, Marriage); scope summary                        |
| 2   | [Document structure](./02-document-structure.md)                | Statements, lines, encoding, whitespace, indentation                        |
| 3   | [Lexical structure](./03-lexical-structure.md)                  | Comments, identifiers, string/bare/date literals, field syntax              |
| 4   | [Top-level statements](./04-top-level-statements.md)            | Version declaration, `person`, `marriage`                                   |
| 5   | [Person sub-statements](./05-person-sub-statements.md)          | `birth` and `adoption` (indented continuations of a `person`)               |
| 6   | [Semantics](./06-semantics.md)                                  | Reference resolution, active marriage at time T, parenthood derivation      |
| 7   | [Validation rules](./07-validation-rules.md)                    | The 13 hard errors a conforming validator must report                       |
| 8   | [Worked examples](./08-worked-examples.md)                      | Four progressive `.kula` documents (links to `../examples/`)                |
| 9   | [Edge cases](./09-edge-cases.md)                                | Founder persons, adoption-only persons, same-pair remarriage, circa, etc.   |
| 10  | [File conventions](./10-file-conventions.md)                    | Extension, encoding, line endings, CLI binary                               |
| 11  | [Reserved keywords](./11-reserved-keywords.md)                  | The 18 reserved identifiers                                                 |
| 12  | [Formal grammar (introduction)](./12-formal-grammar.md)         | EBNF intro and constraints not enforced by grammar; full grammar is below   |
| —   | [`grammar.ebnf`](./grammar.ebnf)                                | Standalone normative EBNF                                                   |
| 13  | [Versioning policy](./13-versioning-policy.md)                  | How future versions extend without breaking                                 |
| 14  | [Formatter rules](./14-formatter-rules.md)                      | Canonical form a conforming `kula format` must produce                      |
| 15  | [Export schema](./15-export-schema.md)                          | Canonical JSON envelope a conforming `kula export` must produce             |

## Conventions used in this spec

- **MUST**, **MUST NOT**, **SHOULD**, **MAY** carry their RFC 2119 senses.
- "Tools" refers collectively to parsers, validators, formatters, and any other Kula-aware software. "Conforming" qualifies a tool that implements every rule in this spec.
- Cross-references between sections use the form *Section N — Title* and link to the section file.
- Code examples are presented in fenced code blocks; they show valid Kula syntax unless a counter-example is being illustrated.

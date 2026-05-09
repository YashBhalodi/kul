# 15. Formatter rules

This section is normative. It specifies the canonical form a conforming Kul formatter MUST produce. The rules are settled in [ADR 0004](../docs/adr/0004-formatter-canonical-rules.md), which carries the rationale; this section is the contract.

A formatter for Kul is a function `format(s) → s'` over a Kul source string. For every input that parses without lex or parse errors, the formatter MUST return a string that satisfies every rule in this section. Inputs that fail to parse SHOULD be rejected rather than partially formatted.

## 15.1 Opinionated, no configuration

A conforming formatter MUST accept exactly one input — the source — and produce exactly one output. It MUST NOT consult a configuration file, environment variable, or command-line flag that alters its output. The reference CLI (`kul format`) carries flags only for *operational* concerns (`--check`, file selection); none of them change the canonical form.

## 15.2 Field order

Within a single statement or sub-statement, fields MUST appear in this order:

| Statement / sub-statement                      | Field order                                                |
| ---------------------------------------------- | ---------------------------------------------------------- |
| `person <id>`                                  | `name`, `gender`, `family`, `given`, `born`, `died`        |
| `marriage <id> <spouse_a> <spouse_b>`          | `start`, `end`, `end_reason`                               |
| `adoption <marriage-ref>`                      | `start`, `end`                                             |
| `birth <marriage-ref>`                         | (no fields)                                                |

The order is positional → required → optional, in the spec-table sequence. Positional arguments — the `<id>`, the spouse identifiers, the `<marriage-ref>` — keep their grammar-mandated order; the formatter MUST NOT reorder them.

## 15.3 Spacing

- **Within a field**: exactly one `:` between name and value. No whitespace before or after `:`. (The lexer rejects `name : "Alice"` as a parse error, so this is reinforcement, not a separate rule.) Examples: `name:"Alice"`, `gender:female`.
- **Between fields on a single line**: exactly two spaces. Example: `name:"Alice"  gender:female  born:1950`.
- **Between a statement keyword and the next token**: exactly one space. Example: `person alice`, `marriage m alice bob`.
- **Between positional arguments** (id, spouse ids, marriage refs): exactly one space. Example: `marriage m alice bob`.

The two-space inter-field rule is the only unusual one. It buys visual separation between fields without per-block alignment math, and it survives copy-paste into terminals or chat logs that single-space-collapse runs.

## 15.4 Indentation

Sub-statements (`birth`, `adoption`) MUST be indented with exactly two ASCII spaces. Tabs are forbidden — the lexer already treats them as parse errors.

## 15.5 Per-region sparse column alignment

The formatter MUST align columns within a *region*. A region is a maximal run of lines bounded by blank lines (or document start/end). The blank line is the only region boundary — whole-line comments, indent changes, and shape changes do NOT bound regions.

A line's cells are, in order: the statement keyword; then any positional arguments in grammar order (id, spouses, marriage-ref); then any fields the line carries in the canonical order from §15.2; then optionally an inline comment as the trailing cell. Two cells have the same *kind* iff they are the same keyword, the same positional role, the same field name, or both are inline comments.

### Alignment groups

Within a region, lines join the same *alignment group* iff they share all three of:

1. the same indent,
2. the same statement keyword (`person`, `marriage`, `birth`, `adoption`),
3. the same parent scope — top-level lines have no parent; sub-statements (`birth`, `adoption`) scope per the `person` they belong to, so two sub-statements under different persons never share a group even when both persons sit in the same region.

Lines with different keywords do NOT share a group. A `person` line and a `marriage` line in the same region are independently aligned, even though both sit at indent 0. Sub-statements at indent 2 are in their own per-parent groups, separate from the indent-0 groups around them.

### Canonical column ordering

Each statement kind has a fixed column sequence. Within an alignment group, every line's cells map to columns of this sequence:

| Statement / sub-statement                      | Column sequence                                                                                    |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `person <id>`                                  | keyword, id, `name?`, `gender?`, `family?`, `given?`, `born?`, `died?`, `comment?`                 |
| `marriage <id> <spouse_a> <spouse_b>`          | keyword, id, spouse_a, spouse_b, `start?`, `end?`, `end_reason?`, `comment?`                       |
| `birth <marriage-ref>`                         | keyword, marriage-ref, `comment?`                                                                  |
| `adoption <marriage-ref>`                      | keyword, marriage-ref, `start?`, `end?`, `comment?`                                                |

Columns marked `?` are *optional*: they are present in the group iff at least one line in the group carries that cell. Required structural cells (keyword, positional id, spouse references, marriage references) are always present.

### Column widths

The width of each present column equals the maximum content width across the lines in the group that carry that cell. Lines that do NOT carry the cell do not influence the column width.

### Rendering a line

Walk the group's column sequence left to right, emitting each line per the following rules:

- For the line's *first* cell (always the keyword), emit the cell content padded with trailing spaces to the column width.
- For each subsequent column, emit the canonical inter-cell separator from §15.3 (single space after a keyword or between positionals/references, two spaces before any field or inline comment), then either:
  - the cell content (padded to column width if it is *not* the line's last actual cell, unpadded otherwise), if the line carries this column, or
  - whitespace of exactly the column's width, if the line does not carry this column AND the line has at least one further actual cell to its right.
- After emitting the line's last actual cell, stop. The line MUST NOT be padded with trailing whitespace through any subsequent column slots.

Concretely, lines whose last actual cell sits left of the group's rightmost column end shorter than their peers; this is intentional. Leading-edge alignment is what column-scanning depends on, and trailing whitespace would break idempotence on editors that strip it.

### Worked example

```
person alice  name:"Alice Sharma"  gender:female              born:1950-04-12
person bob    name:"Bob Sharma"    gender:male    family:"X"  born:1948-11-30  died:2020-03-15
```

The group contains both lines (same indent, same `person` keyword). Columns present in the group: keyword, id, `name`, `gender`, `family`, `born`, `died`. Alice carries no `family` and no `died`; the formatter emits a whitespace placeholder of `family`-column-width before alice's `born:`, then stops alice's line at her unpadded `born:1950-04-12`. Bob's line carries every column; his `died:` is the last cell, unpadded.

### Idempotence

Anti-suggestion 5 of [ADR 0004](../docs/adr/0004-formatter-canonical-rules.md) — per-document alignment — remains rejected. Per-region sparse alignment is bounded by blank lines; an author who wants two stretches of same-keyword lines to NOT share columns MUST split them with a blank line. The blank line is load-bearing for layout.

## 15.6 Blank-line handling

- A blank line between top-level statements MUST be preserved.
- A run of more than one consecutive blank line MUST collapse to a single blank line.
- Blank lines inside a `person` block (between the header and its sub-statements, or between sub-statements) MUST be removed.
- The output MUST NOT begin with a blank line.
- The output MUST end with exactly one trailing newline if it is non-empty.

## 15.7 Comments are opaque

Everything from `#` to end-of-line is preserved byte-for-byte. The formatter MUST NOT read, combine, split, reflow, or move comment text — it operates only on the part of a line *before* any `#`.

Two normalization rules cover whitespace adjacent to comments:

- An end-of-line comment is separated from the preceding tokens by exactly two spaces, matching the inter-field rhythm: `name:"A"  gender:female  # note`.
- A whole-line comment stays on its own line. Outside any `person` block it sits at column 0; under a `person` block it MAY be indented, in which case the formatter MUST emit it at the block's two-space indent.

## 15.8 Idempotence

For every Kul source string `s` that parses without lex or parse errors:

```
format(format(s)) == format(s)   // byte-equal
```

A formatter that breaks idempotence is non-conforming. Any future rule change that would break idempotence is wrong on its face.

## 15.9 Round-trip

For every Kul source string `s` that parses without lex or parse errors:

```
parse(format(s)) ≡ parse(s)   // AST-equal modulo span positions
```

The formatter is a pure presentation pass: it MUST NOT add, remove, or transform semantic content. The only thing it may rearrange is field order within a statement (per §15.2), which by construction does not change the parsed AST. Comments are preserved verbatim (§15.7); the AST itself does not model comments, so they do not appear in this equivalence.

---

← [Section 14 — Project manifest](./14-project-manifest.md) | [Section 16 — Export schema](./16-export-schema.md) | [Index](./README.md)

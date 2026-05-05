# 14. Formatter rules

This section is normative. It specifies the canonical form a conforming Kula formatter MUST produce. The rules are settled in [ADR 0004](../docs/adr/0004-formatter-canonical-rules.md), which carries the rationale; this section is the contract.

A formatter for Kula is a function `format(s) → s'` over a Kula source string. For every input that parses without lex or parse errors, the formatter MUST return a string that satisfies every rule in this section. Inputs that fail to parse SHOULD be rejected rather than partially formatted.

## 14.1 Opinionated, no configuration

A conforming formatter MUST accept exactly one input — the source — and produce exactly one output. It MUST NOT consult a configuration file, environment variable, or command-line flag that alters its output. The reference CLI (`kula format`) carries flags only for *operational* concerns (`--check`, file selection, stdin); none of them change the canonical form.

## 14.2 Field order

Within a single statement or sub-statement, fields MUST appear in this order:

| Statement / sub-statement                      | Field order                                                |
| ---------------------------------------------- | ---------------------------------------------------------- |
| `person <id>`                                  | `name`, `gender`, `family`, `given`, `born`, `died`        |
| `marriage <id> <spouse_a> <spouse_b>`          | `start`, `end`, `end_reason`                               |
| `adoption <marriage-ref>`                      | `start`, `end`                                             |
| `birth <marriage-ref>`                         | (no fields)                                                |

The order is positional → required → optional, in the spec-table sequence. Positional arguments — the `<id>`, the spouse identifiers, the `<marriage-ref>` — keep their grammar-mandated order; the formatter MUST NOT reorder them.

## 14.3 Spacing

- **Within a field**: exactly one `:` between name and value. No whitespace before or after `:`. (The lexer rejects `name : "Alice"` as a parse error, so this is reinforcement, not a separate rule.) Examples: `name:"Alice"`, `gender:female`.
- **Between fields on a single line**: exactly two spaces. Example: `name:"Alice"  gender:female  born:1950`.
- **Between a statement keyword and the next token**: exactly one space. Example: `person alice`, `marriage m alice bob`.
- **Between positional arguments** (id, spouse ids, marriage refs): exactly one space. Example: `marriage m alice bob`.

The two-space inter-field rule is the only unusual one. It buys visual separation between fields without per-block alignment math, and it survives copy-paste into terminals or chat logs that single-space-collapse runs.

## 14.4 Indentation

Sub-statements (`birth`, `adoption`) MUST be indented with exactly two ASCII spaces. Tabs are forbidden — the lexer already treats them as parse errors.

## 14.5 Field alignment

The formatter MUST NOT align field-name columns *across* statements. Each `person` block stands alone; alignment that ripples across unrelated statements when one new field appears is the well-known anti-pattern that `gofmt` and `taplo` both rejected.

A formatter MAY perform *local* column alignment within a single `person` block when that block contains multiple sub-statements of identical shape (for instance, two `adoption` lines): identically-named field columns may be padded with extra single spaces so that values line up. Such alignment MUST NOT extend beyond the block.

## 14.6 Blank-line handling

- A blank line between top-level statements MUST be preserved.
- A run of more than one consecutive blank line MUST collapse to a single blank line.
- Blank lines inside a `person` block (between the header and its sub-statements, or between sub-statements) MUST be removed.
- The output MUST NOT begin with a blank line.
- The output MUST end with exactly one trailing newline if it is non-empty.

## 14.7 Comments are opaque

Everything from `#` to end-of-line is preserved byte-for-byte. The formatter MUST NOT read, combine, split, reflow, or move comment text — it operates only on the part of a line *before* any `#`.

Two normalization rules cover whitespace adjacent to comments:

- An end-of-line comment is separated from the preceding tokens by exactly two spaces, matching the inter-field rhythm: `name:"A"  gender:female  # note`.
- A whole-line comment stays on its own line. Outside any `person` block it sits at column 0; under a `person` block it MAY be indented, in which case the formatter MUST emit it at the block's two-space indent.

## 14.8 Idempotence

For every Kula source string `s` that parses without lex or parse errors:

```
format(format(s)) == format(s)   // byte-equal
```

A formatter that breaks idempotence is non-conforming. Any future rule change that would break idempotence is wrong on its face.

## 14.9 Round-trip

For every Kula source string `s` that parses without lex or parse errors:

```
parse(format(s)) ≡ parse(s)   // AST-equal modulo span positions
```

The formatter is a pure presentation pass: it MUST NOT add, remove, or transform semantic content. The only thing it may rearrange is field order within a statement (per §14.2), which by construction does not change the parsed AST. Comments are preserved verbatim (§14.7); the AST itself does not model comments, so they do not appear in this equivalence.

---

← [Section 13 — Versioning policy](./13-versioning-policy.md) | [Index](./README.md)

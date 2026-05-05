## ADR 0004 — Canonical formatter rules for `kula format`

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

Phase 4 ships a formatter (`kula format`, `textDocument/formatting`) that canonicalizes Kula source. Two questions decide its long-term shape: (1) what does "canonical" mean — i.e. for any two Kula documents that mean the same thing, which one does the formatter produce? (2) Should the rules be configurable?

The formatter is a deep module in the Ousterhout sense: small interface (`format(&Document) -> String`), significant logic behind it, and the choices it bakes in are visible in every `.kula` file users will ever look at. Once shipped, changing them rewrites the entire ecosystem's history. This ADR settles the rules before code lands so the implementation slot in #27 is mechanical and the spec's normative section (`spec/14-formatter-rules.md`) mirrors a stable decision.

The implementation precedents we draw on:

- **`taplo`** (TOML) — opinionated, idempotent, no field-order options.
- **`gofmt`** — single canonical layout, no flags. Cited as the reason Go style debates evaporated.
- **`prettier`** — opinionated for the same "stop the bikeshedding" reason; configuration is intentionally minimal.
- **`rustfmt`** — has options, and their existence has been a recurring drag on the project. We deliberately learn from the negative example here.

## Decision

### 1. The formatter is opinionated. There are no configuration knobs.

One canonical layout, full stop. The formatter accepts a parsed `Document` and produces a `String`; it does not consult a config file, environment, or CLI flag. Future requests for "but my team prefers X" land as `wontfix` against this ADR.

### 2. Field order: positional → required → optional, in spec-table order.

| Statement / sub-statement | Order |
| --- | --- |
| `person <id>` | `name`, `gender`, `family`, `given`, `born`, `died` |
| `marriage <id> <spouse_a> <spouse_b>` | `start`, `end`, `end_reason` |
| `adoption <marriage-ref>` | `start`, `end` |
| `birth <marriage-ref>` | (no fields) |

Positional arguments (`<id>`, spouses, marriage-refs) keep their grammar-mandated order — the formatter never reorders those.

### 3. Field separators

- **Within a field:** a single `:` between name and value. No space on either side. `name:"Alice"`, `gender:female`. (Matches lexer requirement — a space around `:` is a parse error.)
- **Between fields on a single-line statement:** **two spaces.** `name:"Alice"  gender:female  born:1950`. This is unusual; it lets fields read as visually distinct columns without per-block alignment math, and survives copy-paste into terminals or chat logs that single-space-collapse runs.
- **After the keyword:** a single space. `person alice …`, `marriage m alice bob …`.

### 4. Indentation

Sub-statements (`birth`, `adoption`) are indented with **exactly two spaces**. Tabs are forbidden — the lexer already treats them as parse errors.

### 5. Field alignment

The formatter does **not** align field-name columns across statements. Each `person` block stands alone; alignment changes that ripple across unrelated statements when one new field appears are a known anti-pattern (`gofmt` and `taplo` both rejected this for the same reason).

Within a single `person` block whose sub-statements share field shapes (multiple `adoption` lines, for instance), the formatter aligns columns of identically-named fields by padding with single spaces. This is a *local* alignment that doesn't interact with the surrounding document.

### 6. Blank-line handling

- A blank line between top-level statements is preserved.
- Runs of more than one consecutive blank line collapse to a single blank line. This bounds vertical whitespace without forcing the user to give it up entirely (people use blank lines as section separators — see the `# ---- Generation 2 ----` headers in `examples/03-three-generations.kula`).
- Blank lines inside a `person` block (between the header and its sub-statements, or between sub-statements) are removed.

### 7. Comments are opaque

Everything from `#` to end-of-line is preserved byte-for-byte. The formatter never reads, combines, splits, reflows, or moves comment text — it only operates on the part of a line *before* any `#`. This collapses several edge cases into one rule and removes any need for the formatter to reason about whether a comment "applies to" a particular field.

Two normalization rules cover whitespace adjacent to comments:

- An end-of-line comment is separated from the preceding tokens by exactly two spaces, matching the inter-field rhythm (`name:"A"  gender:female  # note`).
- A whole-line comment stays on its own line at column 0 (or at its original indentation under a `person` block, which is the only context where indented comments are meaningful).

Together with rule 8, these are enough to make comment handling round-trip stably.

### 8. Idempotence is a hard guarantee.

For any source string `s`:

```
format(parse(format(parse(s)))) == format(parse(s))   // byte-equal
```

This is non-negotiable and tested with a property test on the entire `examples/` corpus and the regression-test corpus. If any future rule change breaks idempotence, the rule change is wrong, not the test.

### 9. Round-trip preserves the AST.

For any source string `s` whose parse succeeds:

```
parse(format(parse(s))) ≡ parse(s)   // AST-equal modulo span positions
```

The formatter is a pure presentation pass. It never adds, removes, or transforms semantic content. Comments are preserved verbatim (rule 7) but the AST itself doesn't model comments, so they don't appear in this equivalence.

### 10. Spec section `spec/14-formatter-rules.md` is normative.

The implementation in #27 ships alongside a new normative spec section restating these rules in the spec's voice. The spec is the contract; this ADR is the rationale.

## Consequences

### What this rules out

- A `--style=gofmt|prettier|tabular` CLI flag.
- A `.kularc` / `.kula-format.toml` config file.
- Per-team conventions delivered as plugins.
- Per-block tabular alignment that ripples across unrelated statements.
- Reflowing or rewriting comments.

If any of these come up again, point at this ADR and decline.

### What it enables

- The formatter is callable as a library (`kula_core::format::format(&Document)`) by code-generation tools without any environment to thread through.
- `kula format --check` is a clean CI gate.
- `examples/*.kula` becomes the formatter's most visible reference; it's checked into canonical form by the `idempotent` property test in #27.
- `gofmt`-style "stop the bikeshedding" outcome.

### Open questions deferred to implementation (#27)

- Surface for `kula format --check`: exit code, stderr message format. These are CLI ergonomics and don't belong in this ADR.

## Anti-suggestions (do not re-propose)

- **"Make field order configurable."** The whole point is one layout. The cost of even *one* knob is the entire ecosystem reading "did this team turn it on?" forever.
- **"Make spacing configurable."** Same.
- **"Reformat comments."** Comments are user content. The formatter touches whitespace and structure, not authorial voice.
- **"Default to single-space field separators to match other DSLs."** Tested it on the four corpus examples — fields visually merge into the surrounding identifiers ("name:\"Alice\" gender:female" reads as one block). The two-space separator was empirically the smallest change that restored scanability.
- **"Move alignment from per-block to per-document."** Brittle — adding one field to one statement reflows the whole file. The whole reason `gofmt` doesn't do this is the diff blast radius.

## ADR 0004 — Canonical formatter rules for `kul format`

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

The formatter (`kul format`, `textDocument/formatting`) canonicalizes Kul source. Two questions decide its long-term shape: (1) what does "canonical" mean — i.e. for any two Kul documents that mean the same thing, which one does the formatter produce? (2) Should the rules be configurable?

The formatter is a deep module in the Ousterhout sense: small interface (`format(&Document) -> String`), significant logic behind it, and the choices it bakes in are visible in every `.kul` file users will ever look at. Once shipped, changing them rewrites the entire ecosystem's history. This ADR settles the rules so the implementation is mechanical and the spec's normative section (`spec/15-formatter-rules.md`) mirrors a stable decision.

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
- Runs of more than one consecutive blank line collapse to a single blank line. This bounds vertical whitespace without forcing the user to give it up entirely (people use blank lines as section separators — see the `# ---- Generation 2 ----` headers in `examples/03-three-generations.kul`).
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

### 10. Spec section `spec/15-formatter-rules.md` is normative.

The implementation in #27 ships alongside a new normative spec section restating these rules in the spec's voice. The spec is the contract; this ADR is the rationale.

## Consequences

### What this rules out

- A `--style=gofmt|prettier|tabular` CLI flag.
- A `.kulrc` / `.kul-format.toml` config file.
- Per-team conventions delivered as plugins.
- Per-block tabular alignment that ripples across unrelated statements.
- Reflowing or rewriting comments.

If any of these come up again, point at this ADR and decline.

### What it enables

- The formatter is callable as a library (`kul_core::format::format(&Document)`) by code-generation tools without any environment to thread through.
- `kul format --check` is a clean CI gate.
- `examples/*.kul` becomes the formatter's most visible reference; it's checked into canonical form by the `idempotent` property test in #27.
- `gofmt`-style "stop the bikeshedding" outcome.

### Open questions deferred to implementation (#27)

- Surface for `kul format --check`: exit code, stderr message format. These are CLI ergonomics and don't belong in this ADR.

## Anti-suggestions (do not re-propose)

- **"Make field order configurable."** The whole point is one layout. The cost of even *one* knob is the entire ecosystem reading "did this team turn it on?" forever.
- **"Make spacing configurable."** Same.
- **"Reformat comments."** Comments are user content. The formatter touches whitespace and structure, not authorial voice.
- **"Default to single-space field separators to match other DSLs."** Tested it on the four corpus examples — fields visually merge into the surrounding identifiers ("name:\"Alice\" gender:female" reads as one block). The two-space separator was empirically the smallest change that restored scanability.
- **"Move alignment from per-block to per-document."** Brittle — adding one field to one statement reflows the whole file. The whole reason `gofmt` doesn't do this is the diff blast radius.

## Amendment 2026-05-05: per-block column alignment (#35)

Reading the corpus after #27 landed showed the original §14.5 ("no column alignment, ever") was costing too much scannability. Two structurally-identical `person` lines stacked back-to-back in the founders block of `examples/03-three-generations.kul` were noticeably harder to read than their pre-canonical predecessors that had hand-aligned columns.

The amendment adds a third position between "no alignment" and the still-rejected "whole-document alignment": **per-block** alignment, where a *block* is a run of consecutive same-indent same-shape lines bounded by blank lines, whole-line comments, indent changes, and shape changes.

Strict shape matching is intentional. The two alternatives — "align columns that all rows share" and "pad missing fields with whitespace" — both leak edge cases into the spec and create the surprising-diff problem this ADR set out to avoid. With strict shape matching, every block is automatically rectangular, the rule fits in one paragraph, and a field added to one statement excludes it from the surrounding block rather than re-flowing it.

Idempotence and round-trip (rules 8 and 9) hold unchanged. Anti-suggestion 5 ("per-document alignment") remains in force — what the amendment adds is a smaller scope of alignment with hard, user-controlled boundaries, not the rejected version. See `spec/15-formatter-rules.md` §14.5 for the normative restatement and #35 for the ticket history.

## Amendment 2026-05-06: per-region column alignment

The 2026-05-05 amendment defined a *block* as a run of *consecutive* same-indent same-shape lines, bounded by four boundary types: blank line, whole-line comment, indent change, shape change. After it shipped, `examples/05-married-siblings.kul` exposed the cost of that granularity. The file's natural structure stacks two same-shape `person` lines around a `birth` sub-statement:

```
person arjun  name:"Arjun Sharma"  gender:male  born:1950-04-12
  birth m_ramesh_sita
person priya  name:"Priya Sharma"  gender:female  born:1952-08-19
```

The eye wants the second `person` to align with the first, but the previous rule refused: the `birth` between them broke adjacency, the `person` whose header was followed by sub-statements ended its top-level block at the header line, and the next `person` opened a fresh (one-row) block. The result was visually jittery in exactly the place readers most want column alignment — a column scan across same-shape rows.

This amendment relaxes the boundary rules: the **blank line is the only region boundary**. Within a region, same-shape same-indent top-level lines form one alignment group regardless of intervening different-shape lines or whole-line comments. Sub-statements still scope per parent `person` — a sub-statement under one person never joins the alignment group of a sub-statement under a different person, even within the same region. The previous "person whose header is followed by sub-statements ends its top-level block at the header" rule is retired; it was scaffolding for the consecutive-lines model that is no longer needed.

The blast radius widens slightly. Under the previous rule, editing one row of a block could reflow only its consecutive same-shape neighbors. Under this amendment, editing one row can reflow column widths for any same-shape peer in the same region — including peers separated by sub-statements or comments. The widening is bounded by blank lines, not unbounded. Authors who want two same-shape stretches *not* to share columns now have one explicit tool: a blank line.

Anti-suggestion 5 ("per-document alignment") remains in force. Per-region alignment is a strict subset of per-document alignment: it lifts the consecutive-lines requirement but keeps the blank-line boundary. The diff-blast-radius argument that motivated rejecting per-document alignment is preserved, just on a wider scope than per-block.

Idempotence and round-trip (rules 8 and 9) hold by construction: the alignment-group key is `(region, indent, shape, parent-scope-for-sub-statements)`; re-formatting the output uses the same regions, same shapes, same parent scopes → same group memberships → same column widths → byte-identical output. The formatter still only inserts whitespace before separators, so the parsed AST is unchanged.

Corpus impact is contained: only `examples/05-married-siblings.kul` visibly changes — gaining shared columns across each son's `birth` line. Examples 01–04 are byte-identical, because their region layouts already produced one-shape-per-region groupings under the previous rule. See `spec/15-formatter-rules.md` §14.5 for the normative restatement.

## Amendment 2026-05-07: sparse-by-field-name column alignment

The 2026-05-06 amendment kept *strict shape matching* as the alignment-group condition: same indent, same `Vec<CellKind>`. Reading `examples/03-three-generations.kul` after a few rounds of edits exposed why this keeps being the wrong call.

The Generation 2 region:

```
person alice  name:"Alice Sharma"  gender:female  born:1950-04-12
  birth m_ramesh_sita
person bob  name:"Bob Sharma"  gender:male  born:1948-11-30  died:2020-03-15
```

The eye expects `name:`, `gender:`, `born:` to align between alice and bob — those columns are *shared* between the two persons. Strict shape matching refused: bob has `died:` and alice doesn't, so the shapes differ, the lines fall into separate one-line groups, and bob's `name:` lands two columns left of alice's. The Generation 1 (both have `died:`) and Generation 3 (neither has `died:`) regions in the same file align cleanly because their shapes happen to match. The user is left with a layout that flips between aligned and not-aligned based on whether somebody died — exactly the kind of accidental visual cue the formatter exists to suppress.

Strict shape matching was the conservative answer to a different anxiety: that "align on what we share, ignore what we don't" leaks edge cases. After a year of corpus, the actual pattern is the opposite of brittle. **Kul is an additive language** — authors evolve a person from `[name, gender, born]` to `[name, gender, born, died]` to `[name, gender, family, born, died]` over time. Each step is a one-cell addition in canonical position. Strict shape matching turns every such addition into a small alignment regression for that person's neighbors. The cost compounds across the corpus.

This amendment replaces the shape-equality rule with **sparse-by-field-name alignment**, scoped per statement kind:

- The alignment-group key becomes `(region, indent, keyword, parent-scope-for-sub-statements)`. Two lines join the same group iff they have the same indent, same statement keyword, and (for sub-statements) the same parent person. *Shape no longer participates.*
- Each statement kind has a fixed canonical column ordering, derived from §14.2: `person → keyword, id, name?, gender?, family?, given?, born?, died?, comment?`; `marriage → keyword, id, ref_a, ref_b, start?, end?, end_reason?, comment?`; `birth → keyword, ref, comment?`; `adoption → keyword, ref, start?, end?, comment?`.
- A column is *present* in a group iff at least one line in the group carries that cell. Required structural cells (keyword, positional, references) are always present. The column's width is the max content width across lines in the group that have it; lines without the cell don't influence the width.
- A line emits its actual cells padded to their column widths and emits whitespace placeholders of column width for any missing column that sits *before* the line's last actual cell. The line's last actual cell is unpadded, and the line stops there — no trailing whitespace through subsequent column slots. (Trailing whitespace would corrupt idempotence on editors that strip it.)
- Sub-statement scoping is unchanged: same-keyword sub-statements under one person form a group; sub-statements under different parents never share columns even within the same region.
- Cross-statement-kind alignment is *not* introduced. A `person` line and a `marriage` line in the same region remain in separate groups because their canonical column orderings differ structurally (marriage carries references; person doesn't). The corpus convention of separating `person` blocks from `marriage` blocks with blank lines is preserved.

**Trade-offs taken on knowingly.**

- *Wider blast radius.* Adding a `family:` field to one statement creates a new column that pushes every same-keyword peer in the region rightward. This is the consequence ADR-0004's original §5 set out to avoid. The judgment now is that the alternative — same-keyword neighbors silently breaking column alignment whenever shapes diverge — is the worse failure mode for readers, and that authors who want two stretches of same-keyword lines *not* to share columns already have the explicit tool: a blank line. Anti-suggestion 5 (per-document alignment) remains in force; we're widening the within-region rule, not crossing region boundaries.
- *Whitespace placeholders.* The 2026-05-05 amendment specifically named "pad missing fields with whitespace" as an alternative that "leaks edge cases into the spec." With three concrete corpus examples in hand and the canonical column ordering nailed down by §14.2, the edge cases are now bounded and enumerable: the placeholder is exactly `column_width` spaces, columns are in canonical order, and the "stop at last actual cell" rule cuts the trailing-whitespace pathology. The earlier rejection was correct given what we knew then; with the field-order spec stable, the rule fits in one paragraph.
- *Different line lengths within a group.* Lines whose last actual cell is to the left of the rightmost column end shorter than their peers. This is intentional: leading-edge alignment is what column-scanning depends on, and equal line lengths would require trailing whitespace.

Idempotence and round-trip (rules 8 and 9) hold by construction: re-formatting uses the same regions, same group keys (indent + keyword + parent), same canonical column ordering, same per-column max widths → byte-identical output. The formatter still only inserts whitespace before separators and as missing-cell placeholders, neither of which is parse-significant outside string literals.

Corpus impact: `examples/03-three-generations.kul` Generation 2 gains alignment between alice and bob. Examples 01, 02, 04, 05 are unchanged because their regions already had uniform shapes. The change is verified end-to-end by re-running `kul format` over the corpus and asserting `format(format(s)) == format(s)` byte-equal.

See `spec/15-formatter-rules.md` §14.5 for the normative restatement. This amendment supersedes the strict-shape clause of the 2026-05-05 and 2026-05-06 amendments; the rest of those amendments (per-block → per-region scope, blank-line as the only boundary, sub-statement per-parent scoping) carries forward unchanged.

---
name: kul-authoring
description: Author idiomatic Kul (`.kul`) source from natural-language family narratives, and edit existing `.kul` files in line with the spec, the kinship vocabulary in `CONTEXT.md`, and the worked examples. Generate-only — produces source aligned with the project's primitives (`person`, `marriage`, `birth`, `adoption`) and the additivity principle. Does NOT cover tooling concerns (running `kul validate`, configuring VSCode, building the toolchain).
when_to_use: |
  Use this skill whenever a task asks you to author, generate, extend, or edit `.kul` source — especially when converting prose, oral histories, genealogical notes, or interview transcripts into Kul. Triggers include:
    - "Turn this family narrative into Kul"
    - "Add my grandfather to the .kul file"
    - "Split this big family across multiple .kul files"
    - "Translate this paragraph about my in-laws into .kul"
    - "Edit this person to record a divorce / adoption / circa date"

  Do NOT use for tooling concerns: running `kul validate` / `kul format` / `kul export`, configuring the VSCode extension, debugging the Rust toolchain, writing validator rules, modifying the formatter, or anything inside `crates/`. Those are tooling concerns — the human handles them via the CLI and the editor extension. The skill emits source; it does not run the toolchain on the source.
---

# Authoring Kul

You are helping a human capture a family in Kul, a small declarative language for kinship — persons, marriages, biological birth, and adoption with first-class chronology. The input is usually prose: a paragraph, an oral history, a genealogical note, a list of names with relationships. The output is one or more `.kul` files plus a `kul.yml` manifest.

This skill is **generate-only**. You write idiomatic Kul source. You do not run the validator, the formatter, the exporter, or any other tool against what you produce — the human owns that loop via `kul validate` / `kul format` / the VSCode extension.

## Kul in one minute

Kul has **two declared primitives** and **two sub-statements**:

- `person <id> name:"…" gender:… [born:… died:… family:… given:…]` — a declared individual.
- `marriage <id> <spouse_a> <spouse_b> start:… [end:… end_reason:…]` — a declared union between two persons referenced by id.
- `birth <marriage-id>` — indented under a `person`, declares that this person is the biological child of the spouses of the named marriage.
- `adoption <marriage-id> start:… [end:…]` — indented under a `person`, declares this person was adopted into the named marriage.

Everything else — siblings, cousins, half-siblings, grandparents, in-laws, stepfathers, uncles, children — is **derived**. There is no `child` declaration, no `sibling` keyword, no `parent` field. You record persons, the marriages they entered, and how each person entered a family. The rest is computed downstream by the toolchain.

The whole language is small enough to learn in one sitting. See [`references/syntax.md`](./references/syntax.md) for the full surface; see [`spec/`](../../spec/README.md) for the normative source.

### One worked snippet

```
person ramesh  name:"Ramesh Sharma"  gender:male    born:1925-03-10  died:2005-08-22
person sita    name:"Sita Sharma"    gender:female  born:1928-07-15
person alice   name:"Alice Sharma"   gender:female  born:1950-04-12
  birth m_ramesh_sita

marriage m_ramesh_sita ramesh sita  start:1948-06-10
```

Three persons, one marriage. Alice's biological parents (Ramesh and Sita) are **derived** — they are not stored on the person line; they fall out of the `birth m_ramesh_sita` link plus the spouses of `m_ramesh_sita`.

## The additivity principle (your single most important constraint)

**Adding new information must never require rewriting existing declarations.**

This shapes everything you author:

- **Missing data → omit the field.** If the prose doesn't say when someone was born, do not invent a date and do not write `born:unknown` — there is no such literal. Omit `born`. Adding it later is a one-field append, not a rewrite.
- **No coarser-than-known dates.** If the prose says "born sometime in the 1980s," write `born:~1980`. If it says "born in March 1985," write `born:1985-03`. If it says "born March 15, 1985," write `born:1985-03-15`. Date granularity in the source matches the granularity in the prose.
- **Children are not declared on parents.** When you learn of a new child, you add a `person` for them with a `birth` sub-statement pointing at their parents' marriage id. You do NOT touch the parents' lines.
- **Spousal death does not end a marriage.** A surviving spouse + a dead spouse + an unrecorded `end` is the canonical shape. Do not invent an `end_reason:death` — the v1 vocabulary for `end_reason` is `divorce` only. Spousal death's effect on "who is currently married" is derived, not stored.
- **Adoption is permanent unless terminated.** No `end_reason` exists on `adoption`; the only way to record a terminated adoption is `end:<date>` on the sub-statement.

If you find yourself wanting to *rewrite* an existing line to capture new information, stop. That's almost always a sign you should be appending a new declaration or a new sub-statement instead.

## Kinship vocabulary at a glance

Everyday English kinship terms map onto Kul's primitives as either **declared** (you write a statement for them) or **derived** (the graph computes them). The full mapping table is in [`references/vocabulary.md`](./references/vocabulary.md); the load-bearing entries:

| English term         | In Kul                                                          |
| -------------------- | --------------------------------------------------------------- |
| husband / wife / spouse | a `marriage` between two `person`s                           |
| child / son / daughter  | **derived** — a `person` with a `birth` sub-statement         |
| parent / father / mother | **derived** from `birth` (bio) or `adoption` (adoptive)      |
| sibling / brother / sister | **derived** — same `birth` marriage-id                     |
| half-sibling         | **derived** — shares exactly one biological parent              |
| step-sibling         | **derived** — parents are spouses in a common marriage but no shared bio parent |
| step-father / step-mother | **derived** — a spouse of a parent's later marriage         |
| uncle / aunt / cousin / niece / nephew | **derived** — walk siblings + their marriages       |
| grandparent / grandchild | **derived** — parent-of-parent                              |
| in-laws (mother-in-law, etc.) | **derived** — spouse's parents                             |
| adopted child        | **declared** — `person` with an `adoption` sub-statement        |
| ex-spouse            | **declared** — `marriage` with `end:` and `end_reason:divorce`  |
| founder / root       | a `person` with no `birth` sub-statement (implicit; no keyword) |

When prose mentions a derived relation ("Alice's uncle Ravi"), your job is to **resolve it back to declared primitives**: who are Alice's parents, who are *their* siblings, where does Ravi fit? The translation rules in [`references/translation-playbook.md`](./references/translation-playbook.md) cover this end-to-end.

## When you sit down to author

A workable order of operations, matched to the additivity principle:

1. **Inventory the persons.** One `person` declaration per individual the prose names. Skip honorifics ("Mrs.") and titles — the `name:` field is the full display name.
2. **Inventory the marriages.** One `marriage` declaration per union, with a stable id (`m_<spouse_a>_<spouse_b>` is the conventional shape; pick something else only if you need to disambiguate same-pair remarriage — see [`references/syntax.md`](./references/syntax.md) §same-pair remarriage).
3. **Wire the births.** For every child whose biological parents are documented in the file, add a `birth <marriage-id>` sub-statement.
4. **Wire the adoptions.** Add `adoption` sub-statements for adopted children, with a `start:` date.
5. **Decide on file partitioning.** If the family is small (<~30 persons), one `.kul` file is fine. If it's larger, split per [`references/multi-file.md`](./references/multi-file.md) heuristics — usually by generation or by branch.
6. **Add the manifest.** Every Kul project needs a sibling `kul.yml` with `kul: "0.1"`.

You are NOT expected to format the output to canonical column-alignment — the human runs `kul format` for that. Aim for readable Kul; the formatter will canonicalize spacing.

## References (load on demand)

The four reference files cover one surface each. Load them when the task touches their surface:

- [`references/vocabulary.md`](./references/vocabulary.md) — full NL-term → declared-or-derived mapping table; condensed kinship section of `CONTEXT.md`. Load when prose uses kinship terms you need to map to Kul primitives, or when you're unsure whether a relation is declared or derived.
- [`references/syntax.md`](./references/syntax.md) — digest of `spec/03..06` and `spec/12-formal-grammar.md`, with one micro-example per construct. Load when you need exact field grammar (required vs optional, enumerated values, date forms, identifier rules, reserved keywords) or sub-statement shapes.
- [`references/multi-file.md`](./references/multi-file.md) — digest of `spec/10` and `spec/14`, with partitioning heuristics for large families. Load when the prose is large enough that one file feels cramped, or when the task is explicitly multi-file.
- [`references/translation-playbook.md`](./references/translation-playbook.md) — ambiguity-handling rules (missing dates, unstated gender, implicit marriages, derived relations in prose, conflicting accounts) plus five paired NL↔.kul examples covering the most common shapes. Load whenever you're starting an NL→Kul translation; this is the most load-bearing reference for the canonical use case.

## What this skill does NOT cover

- **Running tools.** `kul validate`, `kul format`, `kul export` and their flags are tooling concerns owned by the human. If your output contains a validation error, the human will surface it via the CLI or the VSCode extension and bring it back to you.
- **Validator rules.** The 13 `KUL-Rxx` rules and the manifest `KUL-Mxx` rules exist; you should aim to produce source that passes them, but the skill does not enumerate them. See [`spec/07-validation-rules.md`](../../spec/07-validation-rules.md) if a rule's contents become load-bearing for an authoring decision.
- **Export shapes.** What `kul export` emits, the JSON envelope, the Cytoscape format — none of that is your concern. You write `.kul` source.
- **The VSCode extension and the CLI binary.** The skill is delivered *separately* from those tools via `npx skills add YashBhalodi/kul --skill kul-authoring`. Questions about installing the extension or building the CLI are tooling concerns.

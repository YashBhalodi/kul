---
name: kul-authoring
description: Author idiomatic Kul (`.kul`) source from natural-language family narratives, and edit existing `.kul` files. Generate-only — covers `person`, `marriage`, `birth`, `adoption` syntax and the kinship vocabulary; does NOT cover running `kul validate`/`format`/`export` or the toolchain build.
when_to_use: |
  Trigger on tasks that ask you to author, generate, extend, or edit `.kul` source — especially converting prose or oral histories into Kul. Examples:
    - "Turn this family narrative into Kul"
    - "Add my grandfather to the .kul file"
    - "Split this big family across multiple .kul files"
    - "Edit this person to record a divorce / adoption / circa date"
---

# Authoring Kul

Kul is a small declarative language for kinship — persons, marriages, biological birth, and adoption, with first-class chronology. The canonical artifact is a `.kul` file plus a sibling `kul.yml` manifest. The normative source is [`spec/`](../../spec/README.md); the worked corpus is [`examples/`](../../examples/).

## The four primitives

```
person <id> name:"…" gender:male|female|other  [family:…] [given:…] [born:…] [died:…]
marriage <id> <spouse_a> <spouse_b>  start:…  [end:… end_reason:divorce]
  birth <marriage-id>          # indented under a person; bio child of the marriage's spouses
  adoption <marriage-id> start:…  [end:…]  # indented under a person; adopted into the marriage
```

That's the entire surface. **Everything else — children, parents, siblings, half-siblings, cousins, uncles, in-laws, step-relations — is derived** from the person + marriage + birth/adoption graph. There is no `child` keyword, no `parent` field, no `sibling` declaration. See [`references/vocabulary.md`](./references/vocabulary.md) for the full NL→Kul mapping table.

A minimal worked snippet:

```
person ramesh  name:"Ramesh Sharma"  gender:male    born:1925-03-10  died:2005-08-22
person sita    name:"Sita Sharma"    gender:female  born:1928-07-15
person alice   name:"Alice Sharma"   gender:female  born:1950-04-12
  birth m_ramesh_sita

marriage m_ramesh_sita ramesh sita  start:1948-06-10
```

Alice's parents are derived from the `birth m_ramesh_sita` link plus the spouses of that marriage.

## The one mental model: additivity

The language is designed so that **adding new information never requires rewriting existing declarations**. The shape of the grammar reflects this:

- All `person` fields except `name` and `gender` are optional, and absence is valid — there is no `unknown` literal. Dates have four granularities (`1985-03-15`, `1985-03`, `1985`, `~1985`), so the literal can match exactly what the source says.
- Children are reached through their own `birth` / `adoption` sub-statements pointing at the parents' marriage. There is no `child` field on a parent, no `parent` field on a child.
- Spousal death does not auto-end a marriage; only an explicit `end:` + `end_reason:divorce` does. The v1 `end_reason` vocabulary is `divorce` only.

## References (load on demand)

- [`references/vocabulary.md`](./references/vocabulary.md) — NL kinship term → declared-or-derived mapping table.
- [`references/syntax.md`](./references/syntax.md) — every construct's exact shape with one micro-example each, condensed from `spec/03..06` and `spec/12`.
- [`references/multi-file.md`](./references/multi-file.md) — splitting a large family across files in one project (`spec/10`, `spec/14`).
- [`references/translation-playbook.md`](./references/translation-playbook.md) — five paired NL↔.kul examples. Load this when starting an NL→Kul translation.

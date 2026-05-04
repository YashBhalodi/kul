# KulaLang

> Kula — a kinship description language.

KulaLang is a domain-specific language project for describing human kinship — the structure of families and how they evolve over time. A Kula document is hand-authored plain text that captures persons, marriages, and parenthood (biological and adoptive) as a structured artifact you can read, edit, and reason about.

## Status

**v0.1: Language design complete; reference parser not yet started.**

| Deliverable                          | State                                                  |
| ------------------------------------ | ------------------------------------------------------ |
| Vision and scope                     | Drafted — see [`docs/vision.md`](./docs/vision.md)     |
| Language specification               | Drafted — see [`spec/`](./spec/README.md)              |
| Reference parser                     | Not started                                            |
| Validator CLI (`kula validate`)      | Not started                                            |
| VSCode extension                     | Stretch goal, not started                              |

## A taste

```
kula 0.1

person ramesh name:"Ramesh Sharma" born:1925-03-10 died:2005-08-22 gender:male
person sita   name:"Sita Sharma"   born:1928-07-15 died:2010-11-04 gender:female

marriage m_ramesh_sita ramesh sita start:1948-06-10

person alice name:"Alice Sharma" born:1950-04-12 gender:female
  birth m_ramesh_sita
```

The full feature surface — polygamy, remarriage, retroactive adoption, partial dates, circa dates — is exercised in the [`examples/`](./examples/) directory and documented section-by-section in [`spec/`](./spec/README.md).

## Repository layout

```
.
├── README.md                # this file
├── LICENSE                  # MIT
├── docs/
│   └── vision.md            # why this project exists, scope, shape
├── spec/                    # normative Kula 0.1 specification
│   ├── README.md            # spec index / table of contents
│   ├── 01-introduction.md
│   ├── 02-document-structure.md
│   ├── 03-lexical-structure.md
│   ├── 04-top-level-statements.md
│   ├── 05-person-sub-statements.md
│   ├── 06-semantics.md
│   ├── 07-validation-rules.md
│   ├── 08-worked-examples.md
│   ├── 09-edge-cases.md
│   ├── 10-file-conventions.md
│   ├── 11-reserved-keywords.md
│   ├── 12-formal-grammar.md
│   ├── 13-versioning-policy.md
│   └── grammar.ebnf         # standalone normative EBNF
└── examples/                # worked example .kula documents
    ├── 01-single-couple.kula
    ├── 02-nuclear-family.kula
    ├── 03-three-generations.kula
    └── 04-polygamous-family.kula
```

Future components — the reference parser, the validator CLI, the editor extension — will land as sibling top-level directories (`parser/`, `cli/`, `editor/`).

## Names and conventions

- **KulaLang** — the project (spec, parser, tooling, brand).
- **Kula** — the language itself.
- `.kula` — file extension.
- `kula` — CLI binary name (e.g., `kula validate family.kula`).

## Roadmap

The v1 deliverables defined by [`docs/vision.md`](./docs/vision.md) are:

1. **Language specification** — drafted in [`spec/`](./spec/README.md).
2. **Reference parser** — implementation language and approach TBD.
3. **Validator surface** — CLI at minimum (`kula validate <file>`); editor integration as a stretch goal.

## License

The language specification and (forthcoming) reference parser and tooling are released under the [MIT License](./LICENSE).

## Openness and contributions

KulaLang is a personal project with public artifacts, not (yet) a community-driven standard. There is no commitment to a contributor community, governance process, or maintenance SLA in v1. As the project matures, openness may evolve.

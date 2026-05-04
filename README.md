# KulaLang

> Kula — a kinship description language.

KulaLang is a domain-specific language project for describing human kinship — the structure of families and how they evolve over time. A Kula document is hand-authored plain text that captures persons, marriages, and parenthood (biological and adoptive) as a structured artifact you can read, edit, and reason about.

## Status

**v0.1: Language design complete; reference parser not yet started.**

| Deliverable | State |
| --- | --- |
| Vision and scope | Drafted — see [VISION.md](./VISION.md) |
| Language specification | Drafted — see [SPEC.md](./SPEC.md) |
| Reference parser | Not started |
| Validator CLI (`kula validate`) | Not started |
| VSCode extension | Stretch goal, not started |

## A taste

```
kula 0.1

person ramesh name:"Ramesh Sharma" born:1925-03-10 died:2005-08-22 gender:male
person sita   name:"Sita Sharma"   born:1928-07-15 died:2010-11-04 gender:female

marriage m_ramesh_sita ramesh sita start:1948-06-10

person alice name:"Alice Sharma" born:1950-04-12 gender:female
  birth m_ramesh_sita
```

The full feature surface — polygamy, remarriage, retroactive adoption, partial dates, circa dates — is exercised in the [`examples/`](./examples/) directory and documented section-by-section in [SPEC.md](./SPEC.md).

## Project layout

| Path | Contents |
| --- | --- |
| [VISION.md](./VISION.md) | Why the project exists, scope, and shape |
| [SPEC.md](./SPEC.md) | Normative language specification (Kula 0.1) |
| [examples/](./examples/) | Worked example `.kula` documents |
| [LICENSE](./LICENSE) | MIT license |

## Names and conventions

- **KulaLang** — the project (spec, parser, tooling, brand).
- **Kula** — the language itself.
- `.kula` — file extension.
- `kula` — CLI binary name (e.g., `kula validate family.kula`).

## Roadmap

The v1 deliverables defined by [VISION.md](./VISION.md) are:

1. **Language specification** — drafted in [SPEC.md](./SPEC.md).
2. **Reference parser** — implementation language and approach TBD.
3. **Validator surface** — CLI at minimum (`kula validate <file>`); editor integration as a stretch goal.

## License

The language specification and (forthcoming) reference parser and tooling are released under the [MIT License](./LICENSE).

## Openness and contributions

KulaLang is a personal project with public artifacts, not (yet) a community-driven standard. There is no commitment to a contributor community, governance process, or maintenance SLA in v1. As the project matures, openness may evolve.

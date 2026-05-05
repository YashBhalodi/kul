# KulaLang

> Kula — a kinship description language.

KulaLang is a domain-specific language project for describing human kinship — the structure of families and how they evolve over time. A Kula document is hand-authored plain text that captures persons, marriages, and parenthood (biological and adoptive) as a structured artifact you can read, edit, and reason about.

## Status

**v0.1: Language design, reference parser, validator CLI, language server, and LSP-backed VSCode extension all shipped. First public release pending tag push** — see [`docs/release.md`](./docs/release.md).

| Deliverable                          | State                                                          |
| ------------------------------------ | -------------------------------------------------------------- |
| Vision and scope                     | Drafted — see [`docs/vision.md`](./docs/vision.md)             |
| Language specification               | Drafted — see [`spec/`](./spec/README.md)                      |
| Reference parser (`kula-core`)       | Shipped — see [`crates/kula-core`](./crates/kula-core)         |
| Validator CLI (`kula validate`)      | Shipped — see [`crates/kula-cli`](./crates/kula-cli)           |
| Language server (`kula-lsp`)         | Shipped — see [`crates/kula-lsp`](./crates/kula-lsp)           |
| VSCode extension                     | Shipped (LSP-backed) — see [`editor/vscode`](./editor/vscode)  |
| First public release (`v0.1.0`)      | Pending — see [`docs/release.md`](./docs/release.md)           |

## Install

Pre-built binaries for Linux, macOS, and Windows are attached to each release on the [GitHub Releases page](https://github.com/YashBhalodi/kulalang/releases). Download the archive for your platform and extract the `kula` binary onto your `$PATH`.

To build from source:

```sh
git clone https://github.com/YashBhalodi/kulalang.git
cd kulalang
cargo install --path crates/kula-cli
```

Then:

```sh
kula validate examples/03-three-generations.kula
```

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
├── CHANGELOG.md             # release notes per version
├── Cargo.toml               # Rust workspace root
├── justfile                 # `just check` runs fmt + clippy + tests
├── crates/
│   ├── kula-core/           # parser, AST, semantic, validator, diagnostics
│   ├── kula-cli/            # `kula` binary
│   └── kula-lsp/            # `kula-lsp` language server binary
├── docs/                    # vision, roadmap PRDs, ADRs, release process
├── editor/vscode/           # VSCode extension (LSP-backed)
├── spec/                    # normative Kula 0.1 specification
│   └── grammar.ebnf         # standalone normative EBNF
└── examples/                # worked example .kula documents
```

## Names and conventions

- **KulaLang** — the project (spec, parser, tooling, brand).
- **Kula** — the language itself.
- `.kula` — file extension.
- `kula` — CLI binary name (e.g., `kula validate family.kula`).

## Roadmap

The v1 deliverables defined by [`docs/vision.md`](./docs/vision.md) — language spec, reference parser, validator CLI, and editor integration — are all shipped. The phase-by-phase delivery PRDs live under [`docs/roadmap/`](./docs/roadmap/README.md):

1. **Phase 1** — VSCode extension with TextMate highlighting and snippets ✓
2. **Phase 2** — Reference parser, validator, CLI ✓
3. **Phase 3** — Basic LSP (diagnostics, hover, goto-definition, completion) ✓
4. **Phase 4** — Polished LSP (cross-cutting refinement) — future work

Beyond v1: the language is intentionally minimal at v0.1; future grammar additions land additively (see [`CONTEXT.md`](./CONTEXT.md) on the additivity principle and the [language version policy](./spec/13-versioning-policy.md)).

## Releases

KulaLang ships the CLI, language server, and VSCode extension in lockstep — one tag, one pipeline, one set of artifacts. Maintainers: see [`docs/release.md`](./docs/release.md) for the full procedure. Users: pre-built binaries are attached to each [GitHub Release](https://github.com/YashBhalodi/kulalang/releases) and the VSCode extension is published to the marketplace.

## License

The language specification and reference parser, language server, and tooling are released under the [MIT License](./LICENSE).

## Openness and contributions

KulaLang is a personal project with public artifacts, not (yet) a community-driven standard. There is no commitment to a contributor community, governance process, or maintenance SLA in v1. As the project matures, openness may evolve.

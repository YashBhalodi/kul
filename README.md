# KulaLang

> Kula — a kinship description language.

Kula is a small, hand-authored language for describing human kinship — persons, marriages, biological birth, and adoption — as plain text you can read, edit, version-control, and reason about. KulaLang is the project: the language specification plus the official toolchain (`kula` CLI, `kula-lsp` language server, VSCode extension).

```
kula 0.1

person ramesh name:"Ramesh Sharma" born:1925-03-10 died:2005-08-22 gender:male
person sita   name:"Sita Sharma"   born:1928-07-15 died:2010-11-04 gender:female

marriage m_ramesh_sita ramesh sita start:1948-06-10

person alice name:"Alice Sharma" born:1950-04-12 gender:female
  birth m_ramesh_sita
```

Kula is for individuals modelling their family in a structured way; it is **not** a GEDCOM replacement and **not** a general-purpose graph language. See [`docs/vision.md`](./docs/vision.md) for scope and intent.

## Install

### Pre-built binaries

Each [GitHub Release](https://github.com/YashBhalodi/kulalang/releases) attaches `kula` and `kula-lsp` archives for Linux (x86_64), macOS (Intel + Apple Silicon), and Windows (x86_64). Download the archive for your platform, extract, and put the `kula` binary on your `$PATH`.

### From source

```sh
git clone https://github.com/YashBhalodi/kulalang.git
cd kulalang
cargo install --path crates/kula-cli
```

Requires the Rust stable toolchain (edition 2024). `kula --version` to confirm.

### Editor extension

The [KulaLang VSCode extension](https://marketplace.visualstudio.com/items?itemName=YashBhalodi.kulalang) bundles the language server — install it and `.kula` files get diagnostics, hover, go-to-definition, find-references, rename, completion, formatting, outline view, and the **Kula: Export to JSON** / **Kula: Export to Cytoscape JSON** commands automatically. No additional configuration.

For other editors, point your LSP client at the `kula-lsp` binary.

## Use

### Validate a document

```sh
kula validate family.kula
```

`kula validate` parses the file and reports the 13 spec-defined errors with line/column anchors. Exit `0` on success, `1` on any error.

```sh
kula validate examples/*.kula              # validate many files at once
cat family.kula | kula validate -          # read from stdin
kula validate --format json family.kula    # one JSON object per diagnostic (jsonl)
kula validate --quiet family.kula          # exit code only, no output on success
```

### Format a document

```sh
kula format family.kula           # canonicalize in place
kula format --check family.kula   # CI gate: non-zero exit if not canonical
```

The formatter is opinionated and idempotent: one canonical layout, no configuration. See [ADR-0004](./docs/adr/0004-formatter-canonical-rules.md) and [`spec/14-formatter-rules.md`](./spec/14-formatter-rules.md).

### Export a document to JSON

```sh
kula export family.kula                           # canonical kinship-native JSON
kula export --format cytoscape family.kula        # nodes + edges for graph viz
kula export --with-positions family.kula          # add byte spans for click-to-source
cat family.kula | kula export -                   # read from stdin
kula export *.kula                                # one envelope per line
```

Projects a clean Kula document into a stable JSON envelope downstream tools can consume — visualizers, scripts, generators. The default shape is **kinship-native** (`persons`, `marriages`, `parenthood_links`, with id-only cross-references); `--format cytoscape` projects the same data into the Cytoscape `nodes`/`edges` shape loadable by Cytoscape.js, Sigma.js, vis-network, etc. Strict on errors: a document with any error-severity diagnostic returns a failure envelope (and a non-zero exit code) rather than a partial graph. The schema is normative — see [`spec/15-export-schema.md`](./spec/15-export-schema.md).

### Run the language server

```sh
kula lsp
```

Speaks LSP over stdio. Most users go through the VSCode extension instead.

The language server also handles a custom `kula/export` request, which is what the VSCode **Kula: Export to JSON** and **Kula: Export to Cytoscape JSON** commands call — they project the in-memory buffer (including unsaved edits) and prompt for a save location.

## Learn the language

- [`spec/`](./spec/README.md) — the normative Kula 0.1 specification (14 sections + EBNF grammar). Rigorous enough to implement an independent parser from.
- [`examples/`](./examples/) — four worked `.kula` documents, smallest first, exercising the full feature surface (polygamy, retroactive adoption, partial dates, circa dates).
- [`docs/vision.md`](./docs/vision.md) — what Kula is for and explicitly is not.

## Repository layout

```
.
├── README.md                # this file
├── LICENSE                  # MIT
├── CHANGELOG.md             # release notes per version
├── AGENTS.md                # contributor / AI-agent entry point
├── CONTEXT.md               # canonical domain vocabulary
├── Cargo.toml               # Rust workspace root
├── justfile                 # `just check` runs fmt + clippy + tests
├── crates/
│   ├── kula-core/           # parser, AST, semantic, validator, formatter
│   ├── kula-cli/            # `kula` binary
│   └── kula-lsp/            # `kula-lsp` language server binary
├── docs/                    # architecture, ADRs, testing, release process
├── editor/vscode/           # VSCode extension (LSP-backed)
├── spec/                    # normative Kula 0.1 specification
└── examples/                # worked example .kula documents
```

## Names and conventions

- **KulaLang** — the project (spec, parser, tooling, brand).
- **Kula** — the language itself.
- `.kula` — file extension.
- `kula` — CLI binary name.

## Versioning

The language follows the [versioning policy](./spec/13-versioning-policy.md): new fields and statements land **additively** — adding new information to a Kula document never requires rewriting existing declarations. The CLI, language server, and VSCode extension release in lockstep, one tag per release; see the [Releases page](https://github.com/YashBhalodi/kulalang/releases).

## Contributing

KulaLang is a personal project with public artifacts, not a community-driven standard — there is no commitment to a contributor community, governance process, or maintenance SLA. That said, the codebase is set up so an outside contributor (or an AI agent) can read the docs and start making changes:

- [`AGENTS.md`](./AGENTS.md) — entry point: repo layout, dev commands (`just check`), definition of done.
- [`docs/architecture.md`](./docs/architecture.md) — the implementation map and "where to add X" recipes.
- [`docs/testing.md`](./docs/testing.md) — test conventions (snapshots, corpus, perf budgets).
- [`docs/release.md`](./docs/release.md) — how to cut a release.
- [`docs/adr/`](./docs/adr/) — architectural decision records.

## License

MIT — see [`LICENSE`](./LICENSE). The specification, reference parser, language server, and tooling all release under the same terms.

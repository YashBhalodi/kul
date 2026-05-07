# KulLang

> Kul — a kinship description language.

Kul is a small, hand-authored language for describing human kinship — persons, marriages, biological birth, and adoption — as plain text you can read, edit, version-control, and reason about. KulLang is the project: the language specification plus the official toolchain (`kul` CLI, `kul-lsp` language server, VSCode extension, and `@kul/wasm` for browser/Node consumers).

```
kul 0.1

person ramesh name:"Ramesh Sharma" born:1925-03-10 died:2005-08-22 gender:male
person sita   name:"Sita Sharma"   born:1928-07-15 died:2010-11-04 gender:female

marriage m_ramesh_sita ramesh sita start:1948-06-10

person alice name:"Alice Sharma" born:1950-04-12 gender:female
  birth m_ramesh_sita
```

Kul is for individuals modelling their family in a structured way; it is **not** a GEDCOM replacement and **not** a general-purpose graph language. See [`docs/vision.md`](./docs/vision.md) for scope and intent.

## Install

### Pre-built binaries

Each [GitHub Release](https://github.com/YashBhalodi/kul/releases) attaches `kul` and `kul-lsp` archives for Linux (x86_64), macOS (Intel + Apple Silicon), and Windows (x86_64). Download the archive for your platform, extract, and put the `kul` binary on your `$PATH`.

### From source

```sh
git clone https://github.com/YashBhalodi/kul.git
cd kul
cargo install --path crates/kul-cli
```

Requires the Rust stable toolchain (edition 2024). `kul --version` to confirm.

### Editor extension

The [KulLang VSCode extension](https://open-vsx.org/extension/YashBhalodi/kul) is published on [Open VSX](https://open-vsx.org/) and bundles the language server — install it and `.kul` files get diagnostics, hover, go-to-definition, find-references, rename, completion, formatting, outline view, and the **Kul: Export to JSON** / **Kul: Export to Cytoscape JSON** commands automatically. No additional configuration.

On editors that consume Open VSX (VSCodium, Cursor, Windsurf, Eclipse Theia / Che, Gitpod, Amazon Kiro), `<editor> --install-extension YashBhalodi.kul` resolves and installs the extension directly. On upstream Microsoft VSCode (which talks to the Microsoft Marketplace, where KulLang is intentionally not published), download `kul-<version>.vsix` from the matching [GitHub Release](https://github.com/YashBhalodi/kul/releases) and install it with `code --install-extension /path/to/kul-<version>.vsix`.

For other editors, point your LSP client at the `kul-lsp` binary.

## Use

### Validate a document

```sh
kul validate family.kul
```

`kul validate` parses the file and reports the 13 spec-defined errors with line/column anchors. Exit `0` on success, `1` on any error.

```sh
kul validate examples/*.kul              # validate many files at once
cat family.kul | kul validate -          # read from stdin
kul validate --format json family.kul    # one JSON object per diagnostic (jsonl)
kul validate --quiet family.kul          # exit code only, no output on success
```

### Format a document

```sh
kul format family.kul           # canonicalize in place
kul format --check family.kul   # CI gate: non-zero exit if not canonical
```

The formatter is opinionated and idempotent: one canonical layout, no configuration. See [ADR-0004](./docs/adr/0004-formatter-canonical-rules.md) and [`spec/14-formatter-rules.md`](./spec/14-formatter-rules.md).

### Export a document to JSON

```sh
kul export family.kul                           # canonical kinship-native JSON
kul export --format cytoscape family.kul        # nodes + edges for graph viz
kul export --with-positions family.kul          # add byte spans for click-to-source
cat family.kul | kul export -                   # read from stdin
kul export *.kul                                # one envelope per line
```

Projects a clean Kul document into a stable JSON envelope downstream tools can consume — visualizers, scripts, generators. The default shape is **kinship-native** (`persons`, `marriages`, `parenthood_links`, with id-only cross-references); `--format cytoscape` projects the same data into the Cytoscape `nodes`/`edges` shape loadable by Cytoscape.js, Sigma.js, vis-network, etc. Strict on errors: a document with any error-severity diagnostic returns a failure envelope (and a non-zero exit code) rather than a partial graph. The schema is normative — see [`spec/15-export-schema.md`](./spec/15-export-schema.md).

### Run the language server

```sh
kul lsp
```

Speaks LSP over stdio. Most users go through the VSCode extension instead.

The language server also handles a custom `kul/export` request, which is what the VSCode **Kul: Export to JSON** and **Kul: Export to Cytoscape JSON** commands call — they project the in-memory buffer (including unsaved edits) and prompt for a save location.

### Use from JavaScript / TypeScript (browser or Node)

```sh
npm install @kul/wasm
```

```ts
import { check, exportGraph, format } from '@kul/wasm';

const source = 'kul 0.1\nperson alice name:"A" gender:female\n';

check(source);                         // { diagnostics: [] }  ← empty = clean
exportGraph(source);                   // { ok: true, schema: 1, kul: "0.1", graph: { … } }
exportGraph(source, { format: 'cytoscape' });  // { ok: true, …, graph: { nodes, edges } }
format(source);                        // canonicalized source string
```

The package ships a single `--target bundler` ESM build — works out of the box in Vite, Webpack 5+, Next.js, Turbopack, SvelteKit, Nuxt, and Astro. TypeScript types are derived from the Rust source of truth ([ADR-0012](./docs/adr/0012-tsify-derived-types-committed-and-diffed.md)) and ship with the package. The exported envelope is bit-identical to `kul export --format=json` — same bytes, server-side or browser-side.

## Learn the language

- [`spec/`](./spec/README.md) — the normative Kul 0.1 specification (14 sections + EBNF grammar). Rigorous enough to implement an independent parser from.
- [`examples/`](./examples/) — four worked `.kul` documents, smallest first, exercising the full feature surface (polygamy, retroactive adoption, partial dates, circa dates).
- [`docs/vision.md`](./docs/vision.md) — what Kul is for and explicitly is not.

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
│   ├── kul-core/           # parser, AST, semantic, validator, formatter, export
│   ├── kul-cli/            # `kul` binary
│   ├── kul-lsp/            # `kul-lsp` language server binary
│   └── kul-wasm/           # `@kul/wasm` — browser/Node WASM bindings
├── docs/                    # architecture, ADRs, testing, release process
├── editor/vscode/           # VSCode extension (LSP-backed)
├── spec/                    # normative Kul 0.1 specification
└── examples/                # worked example .kul documents
```

## Names and conventions

- **KulLang** — the project (spec, parser, tooling, brand).
- **Kul** — the language itself.
- `.kul` — file extension.
- `kul` — CLI binary name.

## Versioning

The language follows the [versioning policy](./spec/13-versioning-policy.md): new fields and statements land **additively** — adding new information to a Kul document never requires rewriting existing declarations. The CLI, language server, and VSCode extension release in lockstep, one tag per release; see the [Releases page](https://github.com/YashBhalodi/kul/releases).

## Contributing

KulLang is a personal project with public artifacts, not a community-driven standard — there is no commitment to a contributor community, governance process, or maintenance SLA. That said, the codebase is set up so an outside contributor (or an AI agent) can read the docs and start making changes:

- [`AGENTS.md`](./AGENTS.md) — entry point: repo layout, dev commands (`just check`), definition of done.
- [`docs/architecture.md`](./docs/architecture.md) — the implementation map and "where to add X" recipes.
- [`docs/testing.md`](./docs/testing.md) — test conventions (snapshots, corpus, perf budgets).
- [`docs/release.md`](./docs/release.md) — how to cut a release.
- [`docs/adr/`](./docs/adr/) — architectural decision records.

## License

MIT — see [`LICENSE`](./LICENSE). The specification, reference parser, language server, and tooling all release under the same terms.

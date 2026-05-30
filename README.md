# KulLang

> Kul — a kinship description language.

A small, formally-specified DSL for modeling human families — persons, marriages, biological birth, and adoption — with first-class chronology and a full hand-built toolchain: parser, validator, formatter, language server, a canonical family-tree visual renderer, WASM bindings, and a VSCode extension.

[![Rust CI](https://github.com/YashBhalodi/kul/actions/workflows/rust.yml/badge.svg)](https://github.com/YashBhalodi/kul/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

[Spec](./spec/README.md) · [Examples](./examples/) · [Architecture](./docs/architecture.md) · [ADRs](./docs/adr/) · [Vision](./docs/vision.md)

## A family in plain text

```
# ---- Generation 1 ----
person ramesh  name:"Ramesh Sharma"  gender:male    born:1925-03-10  died:2005-08-22
person sita    name:"Sita Sharma"    gender:female  born:1928-07-15  died:2010-11-04

marriage m_ramesh_sita  ramesh sita  start:1948-06-10

# ---- Generation 2 ----
person alice  name:"Alice Sharma"  gender:female  born:1950-04-12
  birth m_ramesh_sita
person bob    name:"Bob Sharma"    gender:male    born:1948-11-30  died:2020-03-15

marriage m_alice_bob  alice bob  start:1972-05-12  end:1990-08-01  end_reason:divorce

# ---- Generation 3 ----
person carol  name:"Carol Sharma"  gender:female  born:1975-09-03
  birth m_alice_bob
person ravi   name:"Ravi Sharma"   gender:male    born:~1980
  adoption m_alice_bob  start:1985-06-01
```

Three generations, a divorce, and a retroactive adoption — in eleven declarations a human can read top-to-bottom. The whole language is small enough to learn in one sitting; rigorous enough that an independent parser can be implemented from [the spec](./spec/README.md) alone.

## Why a new language

Existing tools either don't capture the dynamics of a real family — polygamy, retroactive adoption, marriages that end and *reasons* they end — or capture them in formats nobody hand-authors twice. Kul is what happens if you ask:

> *What would a kinship language look like if hand-authoring were the design priority, chronology were first-class, and the file itself were the source of truth?*

Visualizations, editors, and other surfaces are downstream — built on top of the language. The language is the canonical artifact.

Kul is **not** a GEDCOM replacement, **not** for genealogy research, and **not** a general-purpose graph language. See [`docs/vision.md`](./docs/vision.md) for the full scope statement, and [Compared to GEDCOM](#compared-to-gedcom) below for the contrast.

## Editor experience

![KulLang in VSCode — syntax highlighting on a three-generation example](./editor/vscode/images/screenshot.png)

Open a `.kul` file in VSCode (or VSCodium / Cursor / Windsurf / Theia / Gitpod / Kiro — the extension is published to both the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=YashBhalodi.kul) and [Open VSX](https://open-vsx.org/extension/YashBhalodi/kul)) and you get live diagnostics, hover, go-to-definition, find-references, rename, completion, formatting, document outline, and one-click export to JSON or Cytoscape graph format — all backed by the same `kul-core` engine the CLI runs.

Run **Kul: Show Preview** and a panel opens beside the editor rendering the family as a canonical tree — generations as rows, spouses joined by a marriage edge, children below — colour-coded by element kind. Drag to pan and scroll to zoom (or arrow-keys to pan and `+`/`-`/`0` to zoom); click a card or marriage bar to jump to its source declaration; the editor cursor highlights the matching element back; hover surfaces an entity's details inline. The view updates live as you type, holds its viewport across edits, and tracks your editor theme.

For other editors, point any LSP client at the `kul-lsp` binary.

## Highlights

- **Normative specification** — fourteen sections plus a standalone [EBNF grammar](./spec/grammar.ebnf), rigorous enough to implement an independent parser from. → [`spec/`](./spec/README.md)
- **Fourteen validation rules** with line/column anchors: duplicate ids, unresolved references, self-marriage, end-consistency, eight temporal contradictions (born-after-died, marriage-before-spouse-born, child-born-before-parent, adoption-before-adopter-born, parenthood cycles, …), and polygamy-hub consistency. → [Validation rules](./spec/07-validation-rules.md)
- **A canonical family-tree visual.** Every valid project renders to one deterministic, parameter-free SVG — the classical descendency tree extended honestly for adoption, divorce and remarriage, polygamy, and marriages that join unrelated families. Live in the VSCode preview, over an LSP request, or via WASM in the browser; the same engine draws all three. → [Canonical UI pattern](./docs/canonical-ui-pattern.md)
- **Hand-authored, machine-checked.** The formatter is opinionated, idempotent, zero-config; format-on-save canonicalizes to a single layout. → [ADR-0004](./docs/adr/0004-formatter-canonical-rules.md)
- **End-to-end Rust toolchain** in one workspace: `kul-core` (lexer, parser, semantic resolution, validator, formatter, export, node-at-cursor query), the `kul-render` → `kul-layout` → `kul-svg` visual pipeline, `kul-cli`, `kul-lsp`, `kul-wasm`. One implementation; many packagings.
- **Two stable JSON projections** — kinship-native (`persons`, `marriages`, `parenthoodLinks`) for downstream tooling, plus Cytoscape `nodes`/`edges` loadable directly into Cytoscape.js, Sigma.js, vis-network, and friends. Strict-on-errors envelope. → [Schema](./spec/16-export-schema.md)
- **Browser-ready WASM** — `import { check, exportGraph, format } from '@kullang/wasm'` works out of the box in Vite, Webpack 5+, Next.js, SvelteKit, Nuxt, Astro. TypeScript types derived from Rust source and CI-diffed against drift. → [ADR-0011](./docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md), [ADR-0012](./docs/adr/0012-tsify-derived-types-committed-and-diffed.md)
- **[Architectural Decision Records](./docs/adr/)** for every non-obvious design call — what was chosen, what was rejected, when it might be revisited. The codebase has a paper trail.

## What Kul is and isn't

| In scope                                              | Out of scope                                       |
| ----------------------------------------------------- | -------------------------------------------------- |
| Persons, marriages, biological birth, adoption        | Friendships, professional ties, social CRM         |
| Polygamy and remarriage (concurrent or serial)        | Dating, engagement, cohabitation                   |
| Adoption at any point in a child's life               | Sperm donors, surrogates, gamete donation          |
| Multiple parenthood links per child (bio + adoptive)  | Single-parent records (every child has two)        |
| First-class chronology with end reasons               | Legal status of relationships in any jurisdiction  |
| Partial dates (`1980-03`, `1980`) and circa (`~1980`) | Genealogy research (use GEDCOM-based tooling)      |

The exclusions aren't statements about validity — they're statements about what *this language* expresses in its first version. See [`docs/vision.md`](./docs/vision.md).

## Try it

### CLI

```sh
git clone https://github.com/YashBhalodi/kul.git
cd kul
cargo install --path crates/kul-cli
kul --version
```

Pre-built `kul` and `kul-lsp` binaries for Linux (x86_64), macOS (Intel + Apple Silicon), and Windows (x86_64) ship attached to each [GitHub Release](https://github.com/YashBhalodi/kul/releases).

A Kul project is a directory: a `kul.yml` manifest plus one or more sibling `.kul` files. The CLI subcommands operate on the project rooted at the current working directory — `cd` into the project, then run:

```sh
cd my-family                                  # directory holding kul.yml + N .kul files
kul validate                                  # 13 spec-defined errors with line/col anchors, across every file
kul format                                    # canonicalize every .kul file in place
kul format --check                            # CI gate (non-zero if anything is not canonical)
kul export                                    # one kinship-native JSON envelope for the whole project
kul export --format cytoscape                 # graph viz JSON
kul export --format svg                       # self-contained canonical SVG (drop into any browser)
kul lsp                                       # speak LSP over stdio
```

`--format json` emits one diagnostic per line.

### VSCode (and forks)

The extension is published to both the [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=YashBhalodi.kul) and [Open VSX](https://open-vsx.org/extension/YashBhalodi/kul); each ships with the matching platform's `kul-lsp` binary bundled, so no extra setup is required. Upstream VSCode resolves against the Marketplace; Open VSX-aware editors (VSCodium, Cursor, Windsurf, Theia/Che, Gitpod, Kiro) resolve against Open VSX. Either way, the same one-liner works:

```sh
<editor> --install-extension YashBhalodi.kul
```

Or search for **KulLang** in the editor's Extensions panel.

Once installed, run **Kul: Show Preview** from the command palette (on any `.kul` file) to open the live canonical-visual panel beside your editor.

Prefer to sideload? Each [GitHub Release](https://github.com/YashBhalodi/kul/releases/latest) attaches per-platform `kul-<version>-<target>.vsix` files (`darwin-arm64`, `darwin-x64`, `linux-x64`, `win32-x64`); pick the one matching your OS and install with `code --install-extension /path/to/kul-<version>-<target>.vsix`.

### Browser / Node

```sh
npm install @kullang/wasm
```

```ts
import { check, exportGraph, format, renderSvg } from '@kullang/wasm';

const manifest = { kul: '0.1' };
const files = [{ name: 'family.kul', source: 'person alice name:"Alice" gender:female\n' }];

check(files, manifest);                                    // { diagnostics: [] } ← empty = clean
exportGraph(files, manifest);                              // { ok: true, schema: 1, kul: "0.1", graph: { … } }
exportGraph(files, manifest, { format: 'cytoscape' });     // { ok: true, …, graph: { nodes, edges } }
format(files[0].source);                                   // canonicalized source (per-file)
renderSvg(files, manifest);                                // { ok: true, svg: "<svg …>…</svg>" } ← the canonical visual
```

A single `--target bundler` ESM build — works in Vite, Webpack 5+, Next.js, Turbopack, SvelteKit, Nuxt, Astro out of the box. The exported envelope is byte-identical to `kul export --format=json`, and `renderSvg` produces the same canonical SVG as the VSCode preview — same shapes on the server and in the browser.

## Learn the language

- [`spec/`](./spec/README.md) — the normative specification (14 sections + EBNF). Read sequentially the first time, jump by section once familiar.
- [`examples/`](./examples/) — worked `.kul` documents, smallest first, exercising the full feature surface: nuclear families, divorce, retroactive adoption, polygamy, and partial dates.
- [`docs/architecture.md`](./docs/architecture.md) — the implementation map: pipeline stages, seams, and "where to add X" recipes.

## AI authoring

Have an LLM agent fluent in Kul. The [`kul-authoring`](./skills/kul-authoring/SKILL.md) skill teaches any [agentskills.io](https://agentskills.io)-compliant agent (Claude Code, Cursor, Copilot, Codex CLI, Gemini CLI, and others) to translate natural-language family narratives into idiomatic `.kul` source. Install it into your `.kul`-authoring project with:

```sh
npx skills add YashBhalodi/kul --skill kul-authoring
```

The skill is generate-only — validation, formatting, and export remain tooling concerns handled via the CLI / VSCode extension.

## Compared to GEDCOM

<details>
<summary>How Kul differs from the de facto kinship interchange format.</summary>

GEDCOM has been the de facto kinship interchange format for forty years. It solves a related but different problem:

- GEDCOM is designed for **ancestry research** — tracing who descended from whom for genealogy. Kul is designed for **modeling living kinship dynamics** as they evolve.
- GEDCOM **isn't pleasant to hand-author**. Kul treats hand-authoring as a primary use case.
- GEDCOM treats relationship state changes as **time-stamped events** layered onto a family-unit record. Kul treats **chronology as first-class** — every relationship has temporal extent and a reason for ending.
- GEDCOM's family-unit model is rooted in monogamous, formalized marriage. Kul's primitives accommodate dynamics common in non-Western conservative kinship — polygamy, retroactive adoption, multi-generational continuity — without retrofits.

Kul is intentionally not GEDCOM-compatible. They're independent languages aimed at different audiences. If you're doing genealogy research, use GEDCOM-based tooling.

</details>

## Project status

KulLang is a **personal project with public artifacts**, currently pre-1.0. The language specification is frozen at version 0.1; the toolchain ships in lockstep — one tag, one set of artifacts. Future versions extend additively (per the [versioning policy](./spec/13-versioning-policy.md)) — adding new fields or statements never requires rewriting existing declarations.

There's no commitment to a contributor community, governance process, or maintenance SLA. The codebase is documented well enough that an outside reader (human or AI) can find their way in — start from [`AGENTS.md`](./AGENTS.md).

## License

MIT — see [`LICENSE`](./LICENSE). The specification, reference parser, language server, and WASM bindings all release under the same terms.

## Names

- **KulLang** — the project (spec, parser, tooling, brand).
- **Kul** — the language itself, /kuːl/, from Hindi *कुल* (family, clan, lineage).
- `.kul` — file extension. `kul` — CLI binary name.

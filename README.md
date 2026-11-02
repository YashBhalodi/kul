# KulLang

> Kul — a kinship description language.

**Kul is a small language for writing down a family as plain text** — people, marriages, births and adoptions, with real dates, partial dates and "about 1925" — and **a browser app at [kullang.dev](https://kullang.dev)** that checks what you write as you type and draws the family tree beside it. The `.kul` file is the source of truth; the tree is drawn from it.

[![Rust CI](https://github.com/YashBhalodi/kul/actions/workflows/rust.yml/badge.svg)](https://github.com/YashBhalodi/kul/actions/workflows/rust.yml)
[![Web CI](https://github.com/YashBhalodi/kul/actions/workflows/web.yml/badge.svg)](https://github.com/YashBhalodi/kul/actions/workflows/web.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

[Open Kul](https://kullang.dev/app/) · [Learn Kul](https://kullang.dev/docs/) · [Spec](./spec/README.md) · [Examples](./examples/) · [Architecture](./docs/architecture.md) · [ADRs](./docs/adr/)

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

Three generations, a divorce and an adoption, in eleven declarations a person can read top to bottom. Children are never listed on their parents: a child names the marriage they were born or adopted into, so adding someone never means editing anyone else.

## The app

Open [kullang.dev/app](https://kullang.dev/app/) on a desktop browser. A first visit opens an example family you can edit straight away.

- **An editor that knows Kul**: highlighting, errors as you type with quick fixes, completion, hover, go to definition, find references, rename across files, and formatting. It floats over the tree, docks to either side or shrinks to a pill.
- **A tree that follows the text**: every edit redraws the family. Click a person or a marriage to see its details and jump to its line; move the cursor in the editor and the tree marks what it is on. Pan and zoom stay where you left them.
- **Projects in your browser**: as many projects as you like, each a set of `.kul` files. Import files, folders or a `.zip`; download a project as a `.zip`, one file, or the tree as an SVG.
- **Nothing leaves your machine.** There is no account and no server: projects are stored by your browser, and every request the site makes goes to its own origin. After one visit it works offline.

[Learn Kul](https://kullang.dev/docs/) is the guide and reference, with every example checked by the engine and a **Try it** button that opens it in the app. For language models there is [`llms.txt`](https://kullang.dev/llms.txt) and [`llms-full.txt`](https://kullang.dev/llms-full.txt).

## Why a new language

Existing tools either don't capture the dynamics of a real family — polygamy, adoption at any age, marriages that end and the *reasons* they end — or capture them in formats nobody writes by hand twice. Kul is what happens if you ask:

> *What would a kinship language look like if hand-authoring were the design priority, chronology were first-class, and the file itself were the source of truth?*

The tree, the editor and every other surface are built on the language. The language is the canonical artifact, and [the spec](./spec/README.md) is precise enough to write an independent parser from.

Kul is **not** a GEDCOM replacement, **not** for genealogy research, and **not** a general-purpose graph language. See [`docs/vision.md`](./docs/vision.md) and [Compared to GEDCOM](#compared-to-gedcom).

## What Kul is and isn't

| In scope                                              | Out of scope                                       |
| ----------------------------------------------------- | -------------------------------------------------- |
| Persons, marriages, biological birth, adoption        | Friendships, professional ties, social CRM         |
| Polygamy and remarriage (concurrent or serial)        | Dating, engagement, cohabitation                   |
| Adoption at any point in a child's life               | Sperm donors, surrogates, gamete donation          |
| Multiple parenthood links per child (bio + adoptive)  | Single-parent records (every child has two)        |
| First-class chronology with end reasons               | Legal status of relationships in any jurisdiction  |
| Partial dates (`1980-03`, `1980`) and circa (`~1980`) | Genealogy research (use GEDCOM-based tooling)      |

The exclusions are statements about what *this language* expresses in its first version, not about validity. See [`docs/vision.md`](./docs/vision.md).

## The `kul` command line

The CLI is for scripts, CI and AI agents: it validates, formats, exports and queries a project from a shell.

```sh
cargo install --git https://github.com/YashBhalodi/kul kul-cli
kul --version
```

Pre-built `kul` binaries for Linux (x86_64), macOS (Intel and Apple Silicon) and Windows (x86_64) are attached to each [GitHub Release](https://github.com/YashBhalodi/kul/releases).

A project is every `.kul` file directly inside one directory. Each command takes that directory as its last argument, `[DIR]`, defaulting to the current directory:

```sh
kul validate my-family                         # every error and warning, with line and column
kul validate --format json my-family           # one JSON diagnostic per line
kul format my-family                           # rewrite every file in canonical form
kul format --check my-family                   # CI gate: non-zero if anything is not canonical
kul export my-family                           # the kinship-native JSON envelope
kul export --format cytoscape my-family        # nodes and edges for graph libraries
kul export --format svg my-family > tree.svg   # the tree as a self-contained SVG
kul query kin ravi parents my-family           # kinship questions; see `kul query --help`
kul query rel carol ramesh my-family           # every way two people are related
```

Export and query are strict: a project with errors exits 1 with its diagnostics instead of a partial answer. The JSON schema is [spec §15](./spec/15-export-schema.md).

## AI authoring

The [`kul-authoring`](./skills/kul-authoring/SKILL.md) skill teaches any [agentskills.io](https://agentskills.io)-compliant agent (Claude Code, Cursor, Copilot, Codex CLI, Gemini CLI and others) to turn a family narrative into idiomatic `.kul` source:

```sh
npx skills add YashBhalodi/kul --skill kul-authoring
```

The skill writes Kul; `kul validate` and `kul format` check and tidy what it wrote.

## Learn the language

- [Learn Kul](https://kullang.dev/docs/) — the guide, the app's manual and the reference (fields, diagnostics, cheat sheet).
- [`spec/`](./spec/README.md) — the normative specification and its [EBNF grammar](./spec/grammar.ebnf).
- [`examples/`](./examples/) — worked projects, smallest first: nuclear families, divorce, adoption, polygamy, partial dates, a project split across files.

## Contributing

KulLang is one repository: the Rust engine (`crates/`), the web app (`apps/web/`), the spec, the docs and the examples. Start from [`AGENTS.md`](./AGENTS.md) for the layout and conventions and [`CONTRIBUTING.md`](./CONTRIBUTING.md) for issues and pull requests. `just check` runs every gate; `just web` serves the app locally.

## Compared to GEDCOM

<details>
<summary>How Kul differs from the de facto kinship interchange format.</summary>

GEDCOM has been the de facto kinship interchange format for forty years. It solves a related but different problem:

- GEDCOM is designed for **ancestry research** — tracing who descended from whom. Kul is designed for **modeling living kinship dynamics** as they evolve.
- GEDCOM **isn't pleasant to hand-author**. Kul treats hand-authoring as the primary use case.
- GEDCOM treats relationship state changes as **time-stamped events** layered onto a family-unit record. Kul treats **chronology as first-class**: every relationship has a temporal extent and a reason for ending.
- GEDCOM's family-unit model is rooted in monogamous, formalized marriage. Kul's primitives accommodate polygamy, adoption at any age and multi-generational continuity without retrofits.

Kul is intentionally not GEDCOM-compatible. If you're doing genealogy research, use GEDCOM-based tooling.

</details>

## Project status

KulLang is a **personal project with public artifacts**, pre-1.0. The language specification is at version 0.1. The web app deploys from `main`; tags release the CLI. Future language versions extend additively (per the [versioning policy](./spec/13-versioning-policy.md)): adding new fields or statements never requires rewriting existing declarations.

There's no commitment to a contributor community, governance process or maintenance SLA. The codebase is documented well enough that an outside reader (human or AI) can find their way in.

## License

MIT — see [`LICENSE`](./LICENSE).

## Names

- **KulLang** — the project (spec, engine, app, brand).
- **Kul** — the language itself, /kuːl/, from Hindi *कुल* (family, clan, lineage).
- `.kul` — file extension. `kul` — CLI binary name.

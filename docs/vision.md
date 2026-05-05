# KulaLang

> Kula — a kinship description language.

## Names and conventions

This project follows the common pattern (cf. Kotlin/KotlinLang, Go/golang) of distinguishing the umbrella project from the language itself:

- **KulaLang** — the project: the spec, the parser, the tooling, the repository, the brand.
- **Kula** — the language itself: what you actually author.
- `.kula` — file extension for Kula source files.
- `kula` — CLI binary name (e.g., `kula validate family.kula`).

Throughout the rest of this document, "Kula" refers to the language, and "KulaLang" refers to the project as a whole.

## What is Kula

Kula is a small, specialized domain-specific language for describing human kinship — the structure of families and how they evolve over time. It is designed to be both **human-readable** (you can author and review a Kula document by hand) and **programmatically actable** (parsers, validators, and visualizers can operate on it).

The name _kula_ (कुल) is Sanskrit/Hindi for _family, clan, lineage_. The cultural grounding of the language is conservative Indian kinship; the surface syntax is in English so that anyone literate in English can read and write it.

## Why this exists

I think in structured terms but struggle to remember the relationships and names in my own family and social circle. Existing tools either do not capture the dynamics I care about, or capture them in a form that is awkward to read, write, and reason about by hand.

I want a single, formally-specified language in which I can describe a family — including its evolution over time — such that the description is _itself_ the artifact. Visualizations, editors, and other tools are downstream surfaces built on top of the language, but the language is the canonical source of truth.

## Why a new language (and not GEDCOM)

GEDCOM is the de facto kinship interchange format and has existed for 40 years. It solves a related but different problem:

- GEDCOM is designed for **ancestry research** (tracing who descended from whom for genealogy). Kula is designed for **modeling living kinship dynamics** as they evolve.
- GEDCOM is **not pleasant to hand-author**. Kula treats hand-authoring as a primary use case.
- GEDCOM treats relationship state changes as **time-stamped events** layered on top of a family-unit record. Kula treats **chronology as first-class** — every relationship has temporal extent and reasons for ending.
- GEDCOM's family-unit model is rooted in monogamous, formalized marriage. Kula's structural primitives accommodate the dynamics common in non-western conservative kinship — polygamy, retroactive adoption, multi-generational continuity — without retrofits.

Kula is intentionally _not_ GEDCOM-compatible. It is an independent language with its own formal definition.

## Audience

Kula is for people like me: individuals — primarily but not exclusively from Indian or similar conservative-kinship cultural backgrounds — who want to model their family in a structured, formal way. The cultural grounding informs the _scope decisions_ (what's expressible) but stays in the backdrop; the language surface is in English and accessible to any English-literate user.

Secondary audience: language designers and DSL enthusiasts who appreciate a small, well-specified language as an artifact in its own right.

Kula is not aimed at:

- Genealogy researchers (use GEDCOM-based tooling).
- Social CRM use cases (modeling friendships, professional networks, etc.).
- General-purpose graph modeling.

## Scope

### In scope

- **Persons** as identifiable entities.
- **Marriages** between two persons, with start and end (divorce, death).
- **Polygamy** — concurrent marriages for one person are expressible.
- **Remarriage** — multiple marriages over a lifetime.
- **Biological parenthood** of children, attached to the marriage in which they were born.
- **Adoption** as a parenthood relationship distinct from biological parenthood, occurring at any point in a child's lifetime.
- **Multiple parenthood links per child** — a child can be biologically born to one couple and later adopted by another; both relationships coexist.
- **Chronology** as a first-class concept: every relationship has a beginning, may have an ending, and the ending may have a reason.

### Out of scope (for now)

These are deliberately excluded so the language stays small and coherent. They may be revisited after the kinship core is solid:

- Non-marriage romantic partnerships (dating, engagement, cohabitation without marriage).
- Sperm donors, surrogates, gamete donation.
- Single parenthood (every child in scope has two parents at the time of conception/adoption).
- Polyamorous co-parenting (more than two parents in the same parenthood unit).
- Friendships, professional relationships, mentorships, neighbors, or any other non-kinship social tie.
- Legal compliance with any jurisdiction's marriage/adoption law. Kula records _real-world relationships_, not their legal status.

The exclusions are not statements about the validity of these relationships in general — they are statements about what _this language_ attempts to express in its first version.

## Conceptual primitives

The language is built on a small set of primitive concepts. The conceptual shape is:

- **Person** — an identifiable individual with a lifespan.
- **Marriage** — a temporal binary union between two persons. Has a beginning. May end, with a reason (divorce, death). Multiple marriages for one person are allowed, both serially and concurrently.
- **Parenthood link** — a relationship from a marriage to a person (the child), qualified by _kind_ (biological or adoptive). A child may have multiple parenthood links over their lifetime, each with its own beginning.

Everything else expressible in Kula — siblinghood, cousinhood, grandparenthood, step-relationships, half-siblings, in-laws, descendants, ancestors — is _derived_ from compositions of these primitives. The language defines structure; richer relationship terms are queries or views on top of that structure.

## Project deliverables

KulaLang's v1 envelope is four coherent artifacts:

1. **The language specification.** A self-contained formal document defining the grammar, semantics, and validation rules of Kula, with worked examples and edge cases. Specified rigorously enough that someone could implement an independent parser from it alone. Lives at [`../spec/`](../spec/README.md).
2. **A reference parser and library** (`kula-core`) that implements the spec end-to-end: lexer, parser, semantic resolution, validator (13 spec rules), formatter, node-at-cursor query for editor tooling.
3. **A CLI** (`kula`) wrapping the library: `kula validate`, `kula format`, `kula lsp`.
4. **An LSP-backed VSCode extension** (`kula-lsp` + the marketplace extension) delivering live diagnostics, hover, go-to-definition, find-references, rename, completion (keyword + ID-aware), document symbols, code actions, formatting, and semantic tokens.

### Explicitly downstream / separate

- **Web visualization app.** A separate project that, if built, would let users interactively define and visualize a Kula document. Not part of v1, and may or may not be built at all. Other application surfaces (mobile, desktop, etc.) are similarly downstream and optional.

## Distribution and openness

- The **language specification** is published as a public artifact under a permissive license. Anyone may read, implement, or fork it.
- The **reference parser** and **CLI** are similarly published.
- This is a **personal project with public artifacts**, not a community-driven standard. There is no commitment to a contributor community, governance process, or maintenance SLA in v1.
- As the project matures, openness may evolve toward a fuller open-source posture (community contributions, public roadmap, etc.). That is not a v1 commitment.
- Any application surface (e.g., the webapp) may be open-source, commercial, or remain unbuilt. That decision is not part of the core project's vision.

## Non-goals

To keep the project honest and focused, KulaLang explicitly does _not_ aim to:

- Be a general-purpose graph or relationship language.
- Be culturally universal. The scope reflects Indian conservative kinship; broadening is a future decision.
- Compete with or replace GEDCOM in the genealogy ecosystem.
- Track legal status of relationships across jurisdictions.
- Model the full social graph of a person.
- Optimize for machine-generated documents over hand-authored ones (it should serve both, but hand-authoring is the design priority).

## Working conventions

- **Project name:** KulaLang
- **Language name:** Kula
- **File extension:** `.kula`
- **CLI binary:** `kula` (e.g., `kula validate family.kula`)
- **Tagline:** _Kula — a kinship description language._

## What this document is not

This is a **vision document** — it captures the why, the what at a conceptual level, the scope, and the project shape. It deliberately does not specify the concrete syntax, temporal-modeling semantics, grammar, validation rules, or parser strategy. Those decisions are settled in [`../spec/`](../spec/README.md) (the normative specification) and [`adr/`](./adr/) (architectural decision records).

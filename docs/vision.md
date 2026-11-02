# Vision

> Kul — a kinship description language.

This is the long-form *why*. For the short pitch, see the [project README](../README.md). For the normative semantics, see [`spec/`](../spec/README.md).

## The problem

I think in structured terms but struggle to remember the relationships and names in my own family and social circle. Existing tools either don't capture the dynamics that matter — polygamy, retroactive adoption, marriages that end and the *reasons* they end — or capture them in formats nobody hand-authors twice.

What I wanted was a single, formally-specified language in which a family can be described — including its evolution over time — such that the description is *itself* the artifact. The app's editor and tree are built on it; the language is the canonical source of truth.

## The name

*Kul* (कुल) is Hindi for *family, clan, lineage*. Pronounced /kuːl/, rhymes with "cool". The cultural grounding of the language is conservative Indian kinship; the surface syntax is in English so anyone literate in English can read and write it.

## Audience

Kul is for individuals — primarily but not exclusively from Indian or similar conservative-kinship cultural backgrounds — who want to model their family in a structured, formal way. The cultural grounding informs the *scope decisions* (what's expressible) but stays in the backdrop; the language surface is in English and accessible to any English-literate user.

A secondary audience: language designers and DSL enthusiasts who appreciate a small, well-specified language as an artifact in its own right.

Kul is **not** for:

- **Genealogy researchers.** Use GEDCOM-based tooling — it's mature, interoperable, and aimed at exactly that problem.
- **Social-CRM use cases.** Friendships, professional networks, mentorships — Kul deliberately doesn't reach for these.
- **General-purpose graph modeling.** Kul's primitives are kinship-shaped; if you need arbitrary edges, use a graph database.

## Why a new language (and not GEDCOM)

GEDCOM has been the de facto kinship interchange format for forty years. It's a well-engineered solution to a related but different problem:

- GEDCOM is designed for **ancestry research** — tracing who descended from whom for genealogy. Kul is designed for **modeling living kinship dynamics** as they evolve.
- GEDCOM is **not pleasant to hand-author**. Kul treats hand-authoring as a primary use case — every design decision is graded against "does this help or hurt someone writing this by hand."
- GEDCOM treats relationship state changes as **time-stamped events** layered on top of a family-unit record. Kul treats **chronology as first-class** — every relationship has temporal extent and a reason for ending. Divorce, death, end date, end reason are part of a relationship's structure, not metadata about it.
- GEDCOM's family-unit model is rooted in monogamous, formalized marriage. Kul's primitives accommodate dynamics common in non-Western conservative kinship — polygamy, retroactive adoption, multi-generational continuity — without retrofits.

Kul is intentionally not GEDCOM-compatible. The two are independent languages aimed at different audiences.

## The shape of the language

Kul is built on a small set of primitives:

- **Person** — an identifiable individual with a lifespan.
- **Marriage** — a temporal binary union between two persons. Has a beginning. May end, with a reason (divorce, death). Multiple marriages for one person are allowed, both serially and concurrently.
- **Parenthood link** — a relationship from a marriage to a person (the child), qualified by *kind* (biological or adoptive). A child may have multiple parenthood links over their lifetime, each with its own beginning.

Everything else expressible in Kul — siblinghood, cousinhood, grandparenthood, step-relationships, half-siblings, in-laws, descendants, ancestors — is *derived* from compositions of these primitives. The language defines structure; richer relationship terms are queries or views on top.

## Scope

### In scope (Kul 0.1)

- Persons as identifiable entities.
- Marriages between two persons, with start and (optional) end (divorce or death).
- Polygamy — concurrent marriages for one person are expressible.
- Remarriage — multiple marriages over a lifetime.
- Biological parenthood, attached to the marriage in which a child was born.
- Adoption as a parenthood relationship distinct from biological parenthood, occurring at any point in a child's lifetime.
- Multiple parenthood links per child — biologically born to one couple and later adopted by another, both relationships coexisting.
- Chronology as a first-class concept: every relationship has a beginning, may have an ending, the ending may have a reason. Partial dates and circa (`~`) markers for uncertain dates.

### Out of scope (deliberately, for now)

These are not statements about the validity of the relationships in general — they're statements about what *this language* attempts to express in its first version. Each may be revisited once the kinship core is solid.

- Non-marriage romantic partnerships (dating, engagement, cohabitation without marriage).
- Sperm donors, surrogates, gamete donation.
- Single parenthood (every child in scope has two parents at the time of conception or adoption).
- Polyamorous co-parenting (more than two parents in the same parenthood unit).
- Friendships, professional relationships, mentorships, neighbors, or any other non-kinship social tie.
- Legal compliance with any jurisdiction's marriage or adoption law. Kul records *real-world relationships*, not their legal status.

## Deliverables

KulLang is four coherent artifacts:

1. **The language specification** — normative sections plus a standalone EBNF grammar. Self-contained; rigorous enough that an independent parser can be written from it alone. → [`spec/`](../spec/README.md)
2. **The engine** — [`kul-core`](../crates/kul-core/) implements the spec end to end (lexer, parser, semantic resolution, the fifteen validation rules, formatter, JSON export, the kinship-query engine); the visual pipeline draws the [canonical tree](./canonical-ui-pattern.md); [`kul-language-service`](../crates/kul-language-service/) answers what an editor needs (diagnostics, hover, completion, definition, references, rename, symbols, quick fixes, formatting, highlighting); [`kul-wasm`](../crates/kul-wasm/) runs all of it in a browser.
3. **The web app** — [kullang.dev](https://kullang.dev), built from [`apps/web`](../apps/web/): an editor and a live family tree side by side, the "Learn Kul" guide and reference, and `llms.txt` for language models. It runs entirely in the reader's browser: projects are stored there, nothing is uploaded, and it works offline.
4. **The CLI** — [`kul`](../crates/kul-cli/): `validate`, `format`, `export` and `query` over a project directory, for scripts, CI and AI agents.

## Posture

Kul is a **personal project with public artifacts**, not a community-driven standard:

- The language specification, the engine, the web app and the CLI are all published under MIT.
- Anyone may read, implement, fork, or build on top of any of them.
- There's no commitment to a contributor community, governance process, or maintenance SLA.
- As the project matures, openness may evolve toward a fuller open-source posture (community contributions, public roadmap, etc.).

## Non-goals

To keep the project honest and focused, Kul explicitly does *not* aim to:

- Be a general-purpose graph or relationship language.
- Be culturally universal. The scope reflects Indian conservative kinship; broadening is a future decision.
- Compete with or replace GEDCOM in the genealogy ecosystem.
- Track legal status of relationships across jurisdictions.
- Model the full social graph of a person.
- Optimize for machine-generated documents over hand-authored ones. Both should work; hand-authoring is the design priority.

# CONTEXT

Canonical vocabulary for KulaLang. When discussing this project — in issue titles, code reviews, hypothesis statements, ADRs, test names, PR descriptions — use these terms exactly as defined here. Don't drift into synonyms (no "service / handler / component / API"); when a concept is missing, extend this file in the same change.

The architecture vocabulary (**module**, **interface**, **seam**, **depth**, **adapter**, etc.) is documented in [`docs/architecture.md`](./docs/architecture.md) and used throughout the codebase. This file focuses on the project's domain and implementation nouns.

## What this project is

**KulaLang** is the project: a language design (Kula) plus the reference toolchain that consumes it. **Kula** is the language itself — a DSL for describing human kinship as plain text. A **Kula document** is a `.kula` file; it contains a sequence of declarations a human writes by hand and a machine can validate, query, and render.

The project's design discipline is the **additivity principle**: adding new information to a Kula document must never require rewriting existing declarations. This shapes the AST (optional fields, stable IDs), the validator (rules tolerate omissions where the spec allows them), and the [version policy](./spec/13-versioning-policy.md) (new fields land additively).

## Kinship vocabulary (the language)

These are the user-facing nouns. They appear in `.kula` source, in spec section names, in diagnostic messages, in hover popovers, and in test names. They are also the names of the AST node types in `crates/kula-core/src/ast.rs`.

### Person

A declared individual: `person <id> name:"…" born:… died:… gender:…`. The **id** is the stable handle (lowercase + digits + underscore); the rest are **fields**. A person may carry a single sub-statement — either a **birth** or an **adoption** — declaring how they entered a family.

### Marriage

A declared union: `marriage <id> <spouse_a> <spouse_b> start:… end:… end_reason:…`. The two spouse positions reference declared persons by id. Marriages are identified, not anonymous — children link to a marriage by id.

A person may participate in multiple marriages (sequential or concurrent — concurrent marriages are valid; see `examples/04-polygamous-family.kula`). The spec does not restrict marriages to particular gender combinations.

### Birth

A sub-statement under a person: `birth <marriage_id>`. It declares that this person is the biological child of the spouses of the named marriage. The person's **biological parents** are derived; they are not stored on the person directly.

### Adoption

A sub-statement under a person: `adoption <marriage_id> start:…`. Declares this person as adopted into the named marriage. A person may have both a `birth` (their biological origin) and one or more `adoption`s; all surface in the **parent set**.

### Field

Any `key:value` pair on a Person, Marriage, Birth, or Adoption. Fields are optional unless the spec marks them required (see [`spec/04-validation-rules.md`](./spec/07-validation-rules.md)). They are unordered. Repeating a field in the same declaration is an error (KULA-R05).

### Date literal

`YYYY`, `YYYY-MM`, or `YYYY-MM-DD`, optionally prefixed `~` to indicate **circa**. A **partial date** is one of the truncated forms (e.g. `1980` is the year-only form). A **circa date** is `~YYYY[-…]`. The two are independent: `~1980-03` is partial *and* circa.

### Spouse

A resolved Person on either side of a Marriage. The function `ResolvedDocument::spouses_of(&MarriageStmt)` yields them; if a spouse-id was unresolved, it's silently skipped (rule 02 has already reported it).

### Parent

A resolved Person derived from either a `birth` link (biological) or an `adoption` link (adoptive). The function `ResolvedDocument::parents_of(&PersonStmt)` yields the union; each is tagged with the link type.

### Child

The inverse of parent. There is no `child` declaration in the language — children are **derived**, not declared on parents. (This is what makes the additivity principle hold: adding a new child to a family does not require editing the parents' declarations.)

### Validator rule

One of the thirteen spec-defined checks (KULA-R01 through KULA-R13). See [`spec/04-validation-rules.md`](./spec/07-validation-rules.md). In code, each rule lives as a function in `crates/kula-core/src/validator.rs` named `rule_NN_<short_name>`; tests follow the same pattern.

### Diagnostic

An error or warning emitted by the validator. Carries a **code** (`KULA-Rxx`), a **severity**, a **message**, a **primary span**, and optional **related** spans. Rendered to the user via `miette` (CLI) or translated to LSP diagnostics (editor).

## Implementation vocabulary

These names appear in code, ADRs, and architecture discussion.

### Document

The AST root: a version declaration plus a list of **statements**. Carries the source `&str` only by reference (`Document<'a>`). Type lives at `crates/kula-core/src/ast.rs`.

### Statement

The two top-level AST nodes: `Statement::Person(PersonStmt)` and `Statement::Marriage(MarriageStmt)`. Sub-statements (birth, adoption) are nested inside a `PersonStmt`, not top-level.

### Span / ByteSpan

A `(start, end)` byte range into the source string. Every AST node carries one. Used for diagnostics, hover ranges, goto-definition targets, completion contexts. Type lives at `crates/kula-core/src/span.rs`.

### Lexer / Parser

Two passes in `crates/kula-core/src/`. The lexer produces a flat token stream (`TokenKind` + span); the parser builds the AST. Both are hand-written and small (~350 + ~750 lines). Recovery is ad-hoc per production: hit an error, sync to newline, continue.

### Resolver

The function `kula_core::semantic::resolve(&Document) -> ResolvedDocument`. Builds the id-to-statement indexes and reports unresolved references (rule 02) inline. Lives at `crates/kula-core/src/semantic.rs`.

### ResolvedDocument

The **kinship-query seam** (per [ADR-0001](./docs/adr/0001-resolved-document-as-query-seam.md)). All cross-reference questions ("who are this person's parents?", "is this id declared?", "who are the spouses of this marriage?") are answered by methods on this type. Validator rules and LSP features query through it; raw AST traversal is reserved for the seam's implementation, not its callers.

### Validator

The pass that runs all thirteen rules over a `ResolvedDocument`, accumulating diagnostics. Lives at `crates/kula-core/src/validator.rs`. Each rule is a function; the validator's job is to call them and collect output.

### Cycle detector

A standalone algorithm at `crates/kula-core/src/cycles.rs`, called by rule 13 (parenthood cycles). Pure function over the parent graph; separated from the rule because the algorithm is independently testable and the rule is a thin shell around it.

### Node-at-cursor / `node_at`

The query `ResolvedDocument::node_at(byte_offset) -> Option<Node<'a>>`. Lives at `crates/kula-core/src/node_at.rs`. Returns a typed enum identifying *what's at the cursor* — keyword, identifier declaration, identifier reference (with resolved target), field name, field value. The shared foundation for hover, goto-definition, and completion. See [`docs/architecture.md`](./docs/architecture.md) for the data-flow diagram.

### Server

The `tower-lsp` Backend implementation in `crates/kula-lsp/src/server.rs`. Owns the document cache, dispatches LSP requests to feature modules, advertises capabilities.

### Document cache

Thread-safe map from `Url` to `Document`-with-resolved-state in `crates/kula-lsp/src/state.rs`. Updated on `did_open` / `did_change` / `did_close`.

### Feature module

One per LSP feature — `crates/kula-lsp/src/features/{hover,definition,completion,diagnostics}.rs`. Each turns a typed request into a typed response by reading the document cache and querying through `ResolvedDocument` + `node_at`. None should walk the AST directly.

### Completion classifier

The token-stream-first context detector in `features/completion.rs`. Identifies which of seven contexts the cursor is in (TopLevelStart, IndentedUnderPerson, PersonFieldList, MarriageFieldList, AdoptionFieldList, AfterGenderColon, AfterEndReasonColon). Token-stream-first because partial / mid-typed input doesn't always parse cleanly. See [ADR-0002](./docs/adr/0002-token-stream-first-completion-classifier.md).

### LineIndex

Byte-offset ↔ LSP-position converter in `crates/kula-lsp/src/convert.rs`. Handles UTF-16 code-unit positions (LSP spec) ↔ UTF-8 byte offsets (kula-core), with CRLF round-trip safety.

## When this glossary is incomplete

If you're naming a concept that isn't here:

- **Common case** — you're inventing language the project doesn't use. Find the canonical term and use it.
- **Real gap** — the concept genuinely doesn't have a name yet. Add it here in the same change, with a one-paragraph definition. If the concept is load-bearing enough that future agents will need to understand *why* it exists, also write an ADR.

Architecture vocabulary (module / interface / seam / depth / adapter / leverage / locality / deletion test) is intentionally not duplicated here — see [`docs/architecture.md`](./docs/architecture.md).

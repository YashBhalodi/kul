# CONTEXT

Canonical vocabulary for KulLang. When discussing this project — in issue titles, code reviews, hypothesis statements, ADRs, test names, PR descriptions — use these terms exactly as defined here. Don't drift into synonyms (no "service / handler / component / API"); when a concept is missing, extend this file in the same change.

The architecture vocabulary (**module**, **interface**, **seam**, **depth**, **adapter**, etc.) is documented in [`docs/architecture.md`](./docs/architecture.md) and used throughout the codebase. This file focuses on the project's domain and implementation nouns.

## What this project is

**KulLang** is the project: a language design (Kul) plus the reference toolchain that consumes it. **Kul** is the language itself — a DSL for describing human kinship as plain text. A **Kul document** is a `.kul` file; it contains a sequence of declarations a human writes by hand and a machine can validate, query, and render.

The project's design discipline is the **additivity principle**: adding new information to a Kul document must never require rewriting existing declarations. This shapes the AST (optional fields, stable IDs), the validator (rules tolerate omissions where the spec allows them), and the [version policy](./spec/13-versioning-policy.md) (new fields land additively).

A Kul project is structurally a graph — Persons and Marriages as the two node kinds, three primitive edge kinds (`spouse-of`, `born-into`, `adopted-into`), every other kinship concept derived from those primitives. For the structural framing (what's standard, what deviates, what it means for graph-shaped features), see [`docs/kinship-graph-shape.md`](./docs/kinship-graph-shape.md). Read it before designing or grilling any query, layout, export, federation, or analytics feature.

## Kinship vocabulary (the language)

These are the user-facing nouns. They appear in `.kul` source, in spec section names, in diagnostic messages, in hover popovers, and in test names. They are also the names of the AST node types in `crates/kul-core/src/ast.rs`.

### Person

A declared individual: `person <id> name:"…" born:… died:… gender:…`. The **id** is the stable handle (lowercase + digits + underscore); the rest are **fields**. A person may carry a single sub-statement — either a **birth** or an **adoption** — declaring how they entered a family.

### Marriage

A declared union: `marriage <id> <spouse_a> <spouse_b> start:… end:… end_reason:…`. The two spouse positions reference declared persons by id. Marriages are identified, not anonymous — children link to a marriage by id.

A person may participate in multiple marriages (sequential or concurrent — concurrent marriages are valid; see `examples/06-polygamous-household/polygamous-household.kul`). The spec does not restrict marriages to particular gender combinations.

### Birth

A sub-statement under a person: `birth <marriage_id>`. It declares that this person is the biological child of the spouses of the named marriage. The person's **biological parents** are derived; they are not stored on the person directly.

### Adoption

A sub-statement under a person: `adoption <marriage_id> [start:…] [end:…]`. Declares this person as adopted into the named marriage. A person may have both a `birth` (their biological origin) and one or more `adoption`s; all surface in the **parent set**.

### Field

Any `key:value` pair on a Person, Marriage, Birth, or Adoption. Fields are optional unless the spec marks them required (see [`spec/07-validation-rules.md`](./spec/07-validation-rules.md)). Each field's type fixes how its value is written: text quoted, choices and dates bare ([spec §3.4](./spec/03-lexical-structure.md#34-value-spellings)). They are unordered. Repeating a field in the same declaration is an error (KUL-R15).

### Date literal

`YYYY`, `YYYY-MM`, or `YYYY-MM-DD`, optionally prefixed `~` to indicate **circa**. A **partial date** is one of the truncated forms (e.g. `1980` is the year-only form). A **circa date** is `~YYYY[-…]`. The two are independent: `~1980-03` is partial *and* circa.

### Spouse

A resolved Person on either side of a Marriage. The function `ResolvedDocument::spouses_of(&MarriageStmt)` yields them; if a spouse-id was unresolved, it's silently skipped (rule 02 has already reported it).

### Host (of a marriage)

The first-listed spouse in a Marriage's declaration (`marriage <id> <host> <joining-spouse> …`). A structural role downstream consumers (renderers, exports, queries) use for ordering and layout — the spec names the role, and the [canonical UI pattern](./docs/canonical-ui-pattern.md) places the host's canonical card in the host's lineage tree with the joining spouse adjacent. The second spouse "joins the host's family." Authors who want to change the host swap the two spouse identifiers; there is no override field. Defined normatively in [`spec/04-top-level-statements.md`](./spec/04-top-level-statements.md#42-marriage-statement) §4.2.

### Polygamy hub

A person with ≥2 un-ended marriages. The canonical UI pattern's [fan rendering primitive](./docs/canonical-ui-pattern.md) treats the hub as the visual anchor for all of their concurrent intimacies — the hub card sits alone at the top of the fan, each co-spouse splays to a wing on the row below (mirrored across that marriage's children-centre), and each marriage's children gather at the midpoint of that marriage's thick marriage edge, two rows below the hub. Rule R14 ensures the polygamy hub is also the declared [host](#host-of-a-marriage) of every concurrent marriage they participate in, so "hub" and "host" coincide by language invariant rather than by renderer repair. See [ADR-0020](./docs/adr/0020-polygamy-hub-and-fan.md) — the R14 hub-equals-host invariant and the fan rendering primitive.

### Parent

A resolved Person derived from either a `birth` link (biological) or an `adoption` link (adoptive). The function `ResolvedDocument::parents_of(&PersonStmt)` yields the union; each is tagged with the link type.

### Child

The inverse of parent. There is no `child` declaration in the language — children are **derived**, not declared on parents. (This is what makes the additivity principle hold: adding a new child to a family does not require editing the parents' declarations.) `ResolvedDocument::children_of(&PersonStmt)` yields every child of a person (the inverse of `parents_of`, one link per spouse of each resolved birth/adoption marriage); `ResolvedDocument::children_of_marriage(&MarriageStmt)` yields every child born or adopted into a marriage (one link per `birth`/`adoption` ref, not expanded per spouse). Both return [`ChildLink`](#childlink) views reconstructed from resolver-owned indexes built during `semantic::resolve`.

### ChildLink

A directed edge `parent/marriage → child` returned by `ResolvedDocument::children_of` / `children_of_marriage`. Mirrors `ParentLink` (the view `parents_of` returns): child person, link span/file, bio/adoptive kind, plus accessors for the underlying `birth` / `adoption` sub-statement. Transient borrowed view — the owned indexes store child id plus link metadata and reconstruct `&PersonStmt` via the entity index (ADR-0007).

### Validator rule

One of the fifteen spec-defined checks (KUL-R01 through KUL-R15). See [`spec/07-validation-rules.md`](./spec/07-validation-rules.md). In code, each rule lives as a function in `crates/kul-core/src/validator.rs` named `rule_NN_<short_name>`; tests follow the same pattern.

### Diagnostic

An error or warning emitted by the parser, the resolver, or the validator. Carries a **code** (`KUL-Lxx`/`KUL-Pxx` for lex/parse, `KUL-Rxx` for validator rules), a **severity**, a **message**, an optional **primary** [`FileSpan`](#filespan), and optional **related** spans (each anchored to a `FileSpan`, possibly in another file under project-wide resolution). The primary is optional because a few diagnostics describe the project or the request as a whole rather than one declaration (an unknown query anchor, a render-shape invariant such as `KUL-V01`). Rendered to the user via `miette` (CLI), or by the [language service](#language-service) as a diagnostic anchored at a named file (editor), which the editor filters to the open file.

### ExportEnvelope

The top-level value `kul export` (and the public `kul_core::export::export` function) emits. Either a **success envelope** carrying a `schema` number, the `kul` language version the toolchain implements (`LANGUAGE_VERSION`), and the [`ExportedGraph`](#exportedgraph), or a **failure envelope** carrying the diagnostic list. The export is strict on errors per [ADR-0009](./docs/adr/0009-export-strict-on-diagnostics.md).

### Project (project-wide namespace)

A flat set of uniquely named `.kul` files. Read from disk, it is every `.kul` file directly inside one directory: subdirectories are not walked and other files are ignored. Every id declared in any of the project's files is visible from every other file by bare name — there is no `import` statement, no namespace prefix, and no qualified-reference syntax. The file boundary is purely organizational; the project is one logical namespace. A project with no files, or only empty ones, is the empty family and is valid. Defined normatively in [spec §2, *Projects*](./spec/02-document-structure.md#projects); the namespace decision is recorded in [ADR-0015](./docs/adr/0015-global-project-namespace.md), and what a project is in [ADR-0048](./docs/adr/0048-a-project-is-a-flat-set-of-kul-files.md).

### File order

The order of a [project](#project-project-wide-namespace)'s files: the byte-wise lexicographic order of their names, comparing UTF-8 bytes (`B.kul` before `a.kul`, `10.kul` before `9.kul`). A project's **declaration order** — which duplicate is "the second" for R01, the order of an export's collections — is its files in file order, then source position. `kul_core::check` owns the rule: it orders its inputs itself, so every caller (the CLI, the browser [`Workspace`](#workspace)) gets the same [`FileId`](#fileid)s whatever order it listed the files in. Defined in [spec §2, *Projects*](./spec/02-document-structure.md#projects).

### ExportedGraph

The kinship-native graph projection inside a success [`ExportEnvelope`](#exportenvelope). Three flat collections — `persons`, `marriages`, `parenthood_links` — that mirror the language primitives one-to-one, with cross-references by id. Defined normatively in [`spec/15-export-schema.md`](./spec/15-export-schema.md); shape choice motivated in [ADR-0008](./docs/adr/0008-export-kinship-native-shape.md).

### Schema number

The `schema:` integer on a success [`ExportEnvelope`](#exportenvelope). Discriminator for the structural shape of the envelope; bumped only when consumers might silently mis-represent data by ignoring a new construct. Independent of the language version (the `kul:` field). Policy in [ADR-0010](./docs/adr/0010-export-schema-versioning.md).

### Query seam

The `query` module in `crates/kul-core/src/query.rs` (introduced by [ADR-0024](./docs/adr/0024-query-seam-and-envelope.md)) — the single public seam where all kinship-query capabilities live. It is a deep module **layered over [`ResolvedDocument`](#resolveddocument)**, not over the [`ExportedGraph`](#exportedgraph): the export is the escape hatch for consumers who don't use the engine, never the engine's own input. It carries the [detail lookups](#detail-lookup), the [entity detail lookup](#entity-detail-lookup), [kin-set queries](#kin-set-query), [attribute filters](#attribute-filter) and [relationship resolution](#relationship-resolution). The CLI's `kul query` is its user-facing surface; the [language service](#language-service)'s `details` uses the entity detail lookup.

### Detail lookup

An id → detail lookup on the [query seam](#query-seam): `query::person(id)` returns `Option<ExportedPerson>` and `query::marriage(id)` returns `Option<ExportedMarriage>` — the **same serialized shapes the export produces** (single-sourced through `projection::build_one_person` / `projection::build_one_marriage`, so a lookup and a whole-graph export can never drift). Lookup semantics are **absence-is-the-answer**: an unknown id, or an id of the wrong kind (a marriage id asked for as a person, or vice versa), yields `None`. There is no error type here — "no such entity" is a complete, honest answer. (Typed unknown-id errors arrive in later slices, where an id is an *input anchor* to a relationship question rather than the subject of the question.) The [entity detail lookup](#entity-detail-lookup) is the many-entities-at-once member of the same family.

### Entity detail lookup

One person or marriage, by id, with its relational neighbourhood: `query::detail(resolved, id) -> Option<EntityDetail>` answers the entity's own fields *plus* its parents, marriages (each with the spouse on the other side) and children, carrying their `biological` / `adoptive` link kinds. It exists because the plain [detail lookups](#detail-lookup) cannot answer a [details panel](#details-panel): `ExportedMarriage` carries no children, and there is no parenthood-link lookup ([ADR-0037](./docs/adr/0037-batched-detail-lookup-shape.md)). The language service's `details` is its caller. Defined in `crates/kul-core/src/query/detail.rs`.

- An **entity detail** (`EntityDetail`) is the answer: a union tagged on `kind` with a `person` and a `marriage` variant, so a panel's variants map 1:1 onto it. Every entity inside is an export shape (`ExportedPerson` / `ExportedMarriage` / `ExportedParenthoodLink`); there is no second, leaner person shape.
- A **linked person** (`LinkedPerson`) is a neighbour plus the parenthood link that reaches them (an adoption link carries its own dates); a **marriage tie** (`MarriageTie`) is a marriage plus the person on the other side. One row per *link*, never per person — [path identity](#path-identity), so a parent reached by both birth and adoption is two rows.
- **Absence is the answer**: an id naming no entity is `None`. The language service answers nothing for a project that fails its checks.

### Query envelope

The adapter-facing result of a query operation: `QueryEnvelope<T>` in `crates/kul-core/src/query.rs`. An untagged union discriminated by an `ok` boolean — mirroring the [`ExportEnvelope`](#exportenvelope) / render-envelope pattern rather than introducing a new tag — whose ok arm carries the query `result` and whose error arm carries the `diagnostics` of a project that failed its checks. Gated on passing checks (strict-on-errors per [ADR-0009](./docs/adr/0009-export-strict-on-diagnostics.md)): a failing project yields the error arm, never a partial answer. Single-sourced in `kul-core`; the CLI's `kul query --format json` prints it. Generic over `T` so every query reuses it.

### Query value

The declarative, serializable request the query engine evaluates: a `source` (`allPersons` or `kinOf(anchor, pattern)`), an optional [`where` conjunction](#attribute-filter) of [predicates](#three-valued-predicate), an optional `sort`, a [certainty `mode`](#certainty-mode), and a `projection` (`members` or `count`). Defined at `crates/kul-core/src/query/pattern.rs`. It is **the single contract with one evaluation path** ([ADR-0025](./docs/adr/0025-kinship-query-engine-contract-and-traversal.md)): every surface — Rust [named sugar](#kin-set-query) and builders (`Query::all_persons().filtered(…).sorted(…).counting()`), CLI `kul query kin` / `kul query persons` — constructs a Query value and hands it to the one `query::run_query` entry point. There is no second engine and no query DSL. Enum variants stay additive: the result is `members` (a `kinOf` set with descriptors), `personIds` (an `allPersons` set, ids only — hydrate via a [detail lookup](#detail-lookup)), or `count` (the filtered set's size), fixed by (source, projection).

### Attribute filter

The **permanently-modest third query capability**: filter persons by their *own* field [predicates](#three-valued-predicate), sort deterministically, and count — composable onto an `allPersons` source or the output of a [kin-set query](#kin-set-query) in one [Query value](#query-value). Deliberately small and *pinned* ([ADR-0025](./docs/adr/0025-kinship-query-engine-contract-and-traversal.md), PRD 0005): exact/case-sensitive string matching (`=`/`≠`/set membership — **no substring, no regex, permanently**; `family` is a first-class field), [three-valued](#three-valued-predicate) interval-date comparison (`<`/`≤`/`>`/`≥` and `=`/`≠` as containment), and presence (`present`/`absent`). Composition is **conjunction-only** — the `where` predicates AND together; **there is no OR** (that is two queries and a set union in consumer code). No cross-field or cross-entity predicates (no age arithmetic, no "alive in 1985", no "spouse's family"), no group-by, no statistics, no date arithmetic — those live on the [exported graph](#exportedgraph), in consumer code. Sort is fully deterministic (dates by interval bounds, strings by codepoint, **missing values last regardless of direction**, ties broken by id ascending) so snapshots stay stable. `count` is the final filtered set's size. Implemented in `crates/kul-core/src/query/filter.rs`; surfaced as CLI `kul query persons` (plus the same `--where` / `--sort` / `--count` / `--include-uncertain` flags on `kul query kin`).

### Three-valued predicate

An [attribute-filter](#attribute-filter) predicate evaluates to **true**, **false**, or **unknown** — the same honesty the descriptor's [seniority](#seniority) fields carry: the engine never asserts what the data cannot support. **String/id/gender** predicates (`eq`/`neq`/`in`, exact/case-sensitive/codepoint) are two-valued when the field is recorded and **unknown** when it is missing. **Date** predicates (`born`/`died`) are three-valued against the recorded value's closed interval (partial `1950` = the whole year, circa `~1950` = ±5 years), reusing the toolchain's **single** date machinery (the [`DateLit`] interval bounds `before_strict` is built on — no second comparison): an ordering (`lt`/`lte`/`gt`/`gte`) is **true** iff it holds under *every* interpretation of both intervals, **false** iff under *none*, else **unknown**; `eq` is **true** iff the recorded interval is wholly contained in the literal's period (`born eq 1950` reads "certainly born within 1950"), **false** iff disjoint, else **unknown**; `neq` is the mirror; a missing date is **unknown**. The `where` conjunction is **three-valued AND**: any `false` ⇒ false; else any `unknown` ⇒ unknown; else true. Defined in `crates/kul-core/src/query/filter.rs`.

[`DateLit`]: #date-literal

### Certainty mode

Which [three-valued](#three-valued-predicate) rows an [attribute filter](#attribute-filter) keeps. `certain` (the default) keeps only rows the predicate conjunction evaluates **true** for — the engine never asserts what the data doesn't support. `includeUncertain` (opt-in, CLI `--include-uncertain`) also keeps **unknown** rows — for gap-finding researchers hunting the fuzzy/partial records a certain-only filter silently drops (e.g. "born after 1950 with a circa or partial date"). A `false` row is never kept in either mode. The `FilterMode` enum in `crates/kul-core/src/query/filter.rs`.

### Kin-set query

A query from one anchor person + a [descriptor pattern](#descriptor-pattern) to the *set* of related persons, each carrying its [relationship descriptor](#relationship-descriptor). The relation vocabulary is the descriptor's own classification. Named **sugar** in `crates/kul-core/src/query/sugar.rs` — lineal (`parents_of`, `children_of`, `ancestors_of(depth?)`, `descendants_of(depth?)`), collateral (`siblings_of`, `aunts_uncles_of`, `nieces_nephews_of`, `cousins_of(degree, removed)`), and affinal (`spouses_of`, `in_laws_of`, `step_parents_of`, `step_siblings_of`, `step_children_of`) — is documented convenience, each *defined as* its [Query value](#query-value) expansion (e.g. `parents_of(x) ≡ kinOf(x, lineal ancestor, generations {1,1})`; `siblings_of(x) ≡ kinOf(x, collateral, up {1,1}, down {1,1})`; `spouses_of(x) ≡ kinOf(x, any {0,0}, affinalHops {1,1})`). Parameterized queries ("second cousins once removed") are expressible by construction — no dedicated API. Raw up/down/across step composition stays internal — exposing it would recreate the "compute the derivation yourself" trap (self-exclusion, cycle guarding, and subsumption are engine-owned). Results are set-shaped: no wrapper, and an empty set ("no kin matched") is a complete answer.

### Descriptor pattern

The declarative selector inside a kin-set query's `kinOf` source: a classification with numeric parameters and ranges — `lineal { role, generations: IntRange }`, `collateral { up: IntRange, down: IntRange }`, `collateralByDegree { degree: IntRange, removed: IntRange }` (the third matches **both orientations** by construction, so `degree 0, removed 1` selects aunts/uncles *and* nieces/nephews), or `any { maxUp, maxDown }` (the unclassified match within a vertical bound, used by the affinity-scoped sugars) — plus optional filters (`edgeNature`, `sharing`, `side`, `affinity`, `affinalHops`). An `affinity` or `affinalHops` filter is also the switch that lets the engine spend marriage hops (up to the [affinal ceiling](#affinal-ceiling)); with neither, a query stays blood-only. Anything [relationship resolution](#relationship-descriptor) can *name*, a pattern can *ask for* — one shared vocabulary. `KinPattern` in `crates/kul-core/src/query/pattern.rs`.

### Relationship descriptor

The terminology-neutral, **maximally discriminating** record of how one person (the alter) relates to another (the ego): endpoint ids and genders, `classification`, `edgeNature`, `affinity`, `sharing`, `side`, two seniorities, and the lossless [path backbone](#path-backbone). `RelationshipDescriptor` in `crates/kul-core/src/query/descriptor.rs`. It carries *every* distinction any future culture pack could key on, because that layer (descriptor → "sister-in-law" | "bhabhi" | …) is pure data over the descriptor and can only discriminate on what the descriptor contains ([ADR-0026](./docs/adr/0026-relationship-descriptor-and-path-identity.md)). The engine never emits a kinship word — it emits the descriptor, and the app renders the term. Serialization is pinned: camelCase, internally-tagged unions, and `unknown` / `notApplicable` as **explicit enum values, never null or absent**.

### Path identity

**Descriptor identity is path identity.** A kin-set result contains one descriptor per *distinct relationship path*, with no engine-side collapsing of same-classification descriptors and no "primary relationship" ranking. A person reachable two ways (e.g. as both a bio and an adoptive ancestor) yields two members with distinct [path backbones](#path-backbone) ([ADR-0026](./docs/adr/0026-relationship-descriptor-and-path-identity.md)). Consumers who want "just first cousins" collapse on the normalized fields themselves. Member order is deterministic (snapshots depend on it): by alter id, then hop count, then serialized backbone.

### Path backbone

The ordered hop sequence from ego to alter carried on every [relationship descriptor](#relationship-descriptor) — **lossless ground truth**, so a distinction nobody anticipated is still recoverable without an engine change. Each `PathHop` (in `crates/kul-core/src/query/descriptor.rs`) is `up` / `down` (carrying the person id landed on, that person's gender, and the `bio` / `adoptive` edge tag) or `across` (a marriage hop: marriage id + status + optional end reason). A backbone is 1–3 blood segments (each an `up* down*` run, possibly empty) joined by at most two [affinal hops](#affinal-hop): `up+` / `down+` (lineal), `up+ down+` through a single apex (collateral), or any of those bracketed by `across` hops (spouse, step, in-law). Hops carry **ids, never entity payloads** — consumers hydrate via the [detail lookups](#detail-lookup).

### Edge nature

A descriptor dimension: `blood | adoptive`, strictly about the parent-child edges on the path — `adoptive` iff *any* hop is an adoption edge, else `blood`. Distinct from `affinity` (`blood | step | inLaw`, about marriage hops); the two were split because a single flat consanguinity enum could not express half-adoptive or full-adoptive siblings. Per-hop truth stays lossless in the [path backbone](#path-backbone).

### Side

A descriptor dimension: `maternal | paternal | other | both | notApplicable`, **derived from the path's routing, never guessed**. `notApplicable` when the path never ascends from ego (descendants, direct parents); `both` when the initial ascent reaches its apex in a *single* hop that lands on a [couple apex](#couple-apex) (full siblings and every relation routed onward through them — you route through the couple, with no individual shared parent to take a side from); otherwise the gender of the first parent-person on the initial ascent — female → maternal, male → paternal, `other` → other (the grammar permits `gender:other`, so side is *derived*, never assumed binary). For an uncle/cousin (initial ascent ≥ 2 hops) that first person is ego's own parent, so the apex's couple-ness never overrides ego's side — this is what keeps *mama* (maternal uncle) vs *chacha* (paternal uncle) expressible. Side is about routing; endpoint gender is its own field.

### Seniority

Two descriptor dimensions, both riding the toolchain's single strict-interval date comparison (`before_strict` — one notion of "date A is before date B", shared with the validator's temporal rules): `seniority` (endpoint — the alter's birth order vs the ego) and [`apexSeniority`](#apex-seniority) (the [sibling junction](#apex-junction), `notApplicable` on a lineal/self path). Each is `elder | younger | unknown | notApplicable`. `elder` / `younger` only when *every* interpretation of one birth date strictly precedes the other; missing dates, overlapping partial/circa intervals, and same-day twins are `unknown`; `notApplicable` is reserved for self (endpoint) or a path with no sibling junction (apex). The engine never invents a seniority.

### Apex junction

The **apex** of a collateral blood segment (`up* down*`) is the person where ascent turns to descent. The **junction** is the triple *(apex, egoChild, alterChild)*: *egoChild* is the person the last `up` hop ascended from (ego itself, for siblings); *alterChild* is the person the first `down` hop descended to (the alter itself, for siblings). egoChild and alterChild are the two [branch siblings](#branch-siblings). Computed in `crates/kul-core/src/query/junction.rs`; it is where [sharing](#sharing), [apex seniority](#apex-seniority), the [couple apex](#couple-apex), and `side: both` are all derived. A lineal/self path has no junction (nothing is `notApplicable`-worthy there).

### Branch siblings

The two children *(egoChild, alterChild)* at an [apex junction](#apex-junction) — the pair whose parent-set comparison yields [sharing](#sharing) and whose birth order yields [apex seniority](#apex-seniority). The no-backtracking rule ([ADR-0025](./docs/adr/0025-kinship-query-engine-contract-and-traversal.md)) guarantees they are distinct.

### Sharing

A descriptor dimension: `full | half | notApplicable`, an **apex-junction comparison** of the [branch siblings](#branch-siblings)' parent sets **per edge kind** (bio set vs bio set; adoptive set vs adoptive set). `full` iff the bio sets are equal and non-empty, OR the adoptive sets are equal and non-empty (adoptive-full) — parent-*set* equality, never a shared marriage, so full siblings stay `full` across a same-couple divorce-and-remarry, and a bio child and an adoptee of the same couple do *not* read as full. `half` iff they share at least one parent of any kind but no same-kind set equality holds (polygamy and remarriage collapse identically here, distinguishable only via the [path backbone](#path-backbone)'s marriage references). `notApplicable` for lineal/self (no sibling junction). Derived in `crates/kul-core/src/query/junction.rs`.

### Apex seniority

The `apexSeniority` half of [seniority](#seniority): the birth order of the *alter-branch* sibling versus the *ego-branch* sibling at the [apex junction](#apex-junction), under the same `before_strict` rule as the endpoint field. It is why *chacha* (father's *younger* brother) vs *tau* (father's *elder* brother) is expressible — a distinction that compares the uncle to ego's father, which the ego-relative endpoint seniority cannot capture. For siblings the branch siblings *are* ego and alter, so `apexSeniority` coincides with `seniority` by construction. `notApplicable` on any path with no sibling junction.

### Couple apex

An [apex junction](#apex-junction) whose two [branch siblings](#branch-siblings) share the *same two parents* — identical bio-parent sets of size 2, or both adopted by the same couple. Two consequences: (1) paths that differ only in *which* shared co-parent they route through are **one relationship fact** — the engine canonicalizes the backbone through the shared parent whose id sorts first by codepoint (deterministic, snapshot-stable) and emits one descriptor; (2) `side = both` when the couple apex is the apex of the path's initial *single-hop* ascent. This is *not* engine-side collapsing of distinct relationships — [double cousins](#double-cousins) have *different* junctions and stay two descriptors.

### Double cousins

Two people related the same way via two distinct paths through *different* grandparent couples (two brothers marrying two sisters → the children are first cousins twice over). Because the two paths run through different [apex junctions](#apex-junction), they are two members of a kin-set result, differing in [side](#side) (paternal vs maternal) and [path backbone](#path-backbone). [Path identity](#path-identity) is non-negotiable here — collapsing them would hide a true tie; only same-junction co-parent routes (a [couple apex](#couple-apex)) collapse.

### Affinity

A descriptor dimension: `blood | step | inLaw`, about the path's marriage ([affinal](#affinal-hop)) hops — distinct from [edge nature](#edge-nature) (about parent-child edges). `blood` when the path has no `across` hop; `step` when *every* `across` hop is in [ancestor position](#ancestor-position); `inLaw` when *any* `across` hop is not (inLaw wins a mixed path; the [backbone](#path-backbone) keeps the per-hop truth). This is what separates a step-parent (`up`, `across`) from a parent-in-law (`across`, `up`), and a co-wife (`across`, `across`) from a full sibling.

### Affinal hop

A marriage (`across`) [path hop](#path-backbone): the move between the two spouses of a marriage, in either direction, carrying the marriage id, its status (`ongoing | ended`), and — when ended — the recorded reason. Ended marriages are traversed **exactly like ongoing ones**; the core reports and tags, never filters (whether divorce dissolves affinity is a downstream terminology decision). The engine crosses at most two affinal hops per path (the [affinal ceiling](#affinal-ceiling)).

### Ancestor position

The mechanical test that makes an [affinal hop](#affinal-hop) read as `step` rather than `inLaw`: an `across` hop is *in ancestor position* iff it is preceded by at least one hop and **every** preceding hop is `up`. A path-initial `across` is therefore never in ancestor position — so a spouse and a spouse's kin are `inLaw`, while a parent's spouse (`up`, `across`) is `step`. Drives the [affinity](#affinity) scalar.

### Step subsumption

A step path is a *derived stand-in* for parenthood, **suppressed — not emitted alongside** — when the underlying fact is real. A `step`-shaped path to a person who is also an actual parent of ego (a step-parent shape to a real bio/adoptive parent), an actual child (step-child shape), or someone who shares ≥1 actual parent with ego (a step-sibling shape to a full/half sibling) is dropped; only the blood/adoptive path is emitted. An explicit adoption edge always beats the step reading. This does *not* contradict [path identity](#path-identity): [double cousins](#double-cousins) are two independent *true* paths, whereas a shadowed step path is one fact derived two ways. Engine-owned (`crates/kul-core/src/query/engine.rs`).

### Co-spouse

Two people married to the same third person (the *sautan* / co-wife shape a polygamy corpus needs): ego → spouse → spouse's *other* spouse, two **consecutive** `across` hops with zero vertical displacement. Classified `self` with [affinity](#affinity) `inLaw`. The corpus reason consecutive affinal hops are allowed at all.

### Affinal ceiling

The engine crosses **at most two** [affinal hops](#affinal-hop) per path — fixed semantics, never a configuration knob. No culture lexicalizes three affinal hops (spouse's sibling's spouse's sibling names nothing), so the ceiling is part of the model, not a caller budget. This is the semantics side of the [semantics-vs-budget line](#semantics-vs-budget-line) (the [generation budget](#generation-budget), by contrast, is a caller budget). Pinned in [ADR-0027](./docs/adr/0027-affinal-traversal-ceiling-and-step-subsumption.md).

### Relationship resolution

The **two-anchor** query on the [query seam](#query-seam): given persons `x` and `y`, `query::resolve(x, y, config)` returns *all* the ways they are related — a `ResolveResult` (in `crates/kul-core/src/query/resolve.rs`) carrying one [relationship descriptor](#relationship-descriptor) per distinct path (same [path identity](#path-identity), same [descriptor](#relationship-descriptor) derivation, same [traversal engine](#query-seam) as the [kin-set queries](#kin-set-query) — **resolution never forks the kin-set logic**) plus, **only when the list is empty**, an [emptiness reason](#emptiness-reason). It is a *separate call*, not a [Query value](#query-value) pipeline stage — a different question ("how are these two related?" vs "who are this person's …?"). Enumeration is all simple paths under the full grammar where every blood segment stays within the [generation budget](#generation-budget); pure-lineal (direct ancestor/descendant) ties are additionally detected **unbounded**, regardless of the budget, so a `noneWithinBounds` never hides a recorded direct-line tie. `x == y` yields a single `self` descriptor (empty path). Results are sorted by (path hop count) → (serialized backbone). An unknown or wrong-kind id (either anchor) is a typed `UnknownPerson` error, never an empty result. Surfaced as CLI `kul query rel <x> <y> [--max-generations N]` ([ADR-0028](./docs/adr/0028-relationship-resolution-and-honest-emptiness.md), PRD 0005).

### Emptiness reason

The honest "why" a [relationship resolution](#relationship-resolution) found nothing — present on `ResolveResult.emptyReason` **iff** the relationship list is empty: `disconnected` (`x` and `y` lie in different connected components of the full relation graph — undirected reachability over every parent-child edge of both kinds plus every spouse edge — so raising the budget can never help) or `noneWithinBounds` (same component, but nothing is derivable under the semantics and the current [generation budget](#generation-budget) — a bigger budget might reveal a tie). Collapsing the two into a bare empty list would let an app render "not related" when the truth is "not related as far as we looked"; the distinction is the product ([ADR-0028](./docs/adr/0028-relationship-resolution-and-honest-emptiness.md)).

### Generation budget

The one knob of [relationship resolution](#relationship-resolution): `maxApexGenerations` (default 5) bounds **each blood segment's** ascent and descent — a nearest-common-ancestor bound, not a total-path-length bound. It is a caller *budget*: 5 reaches through fourth cousins (a strict superset of every lexicalized kinship term — cultures run out by third cousins) while cutting off the remote-ancestor haystack; a caller who wants to look nearer or farther legitimately says so. Contrast the fixed [affinal ceiling](#affinal-ceiling), which is *semantics*, not a budget — see the [semantics-vs-budget line](#semantics-vs-budget-line).

### Semantics-vs-budget line

The distinction that governs what is a caller knob and what is fixed engine law. The 2-hop [affinal ceiling](#affinal-ceiling) is **semantics**: it defines *what counts as a relationship* (no culture names a three-affinal-hop tie), so it is never configurable — two apps must never disagree about *what relationships exist*. The [generation budget](#generation-budget) is a **search budget**: two apps may legitimately look differently far, so it is a caller knob. The line keeps the contract honest — a bigger budget can reveal more of the *same* relationship space, but can never change its definition ([ADR-0027](./docs/adr/0027-affinal-traversal-ceiling-and-step-subsumption.md), [ADR-0028](./docs/adr/0028-relationship-resolution-and-honest-emptiness.md)).

### Render shape

The top-level value `kul_render::compute` and `kul_render::transform` emit — the canonical UI pattern's data form for a checked Kul project. Where [`ExportEnvelope`](#exportenvelope) is shaped to mirror what the source *says* (kinship-native), the render shape is shaped to mirror what the canonical UI pattern *draws*: components, marriage branches, card slots, ghosts. Either a success payload (`components`, `edges`, plus the `schema` / `kul` discriminators) or a failure payload (the same diagnostic list the input envelope carried). Defined at `crates/kul-render/src/shape.rs`; the principles it realizes live in [`docs/canonical-ui-pattern.md`](./docs/canonical-ui-pattern.md); crate-boundary rationale in [ADR-0016](./docs/adr/0016-visualization-pipeline-crate-boundaries.md); schema-versioning policy in [ADR-0017](./docs/adr/0017-render-shape-schema-and-versioning.md). `kul-layout` and `kul-svg` turn it into the SVG the [canvas](#canvas) and `kul export --format=svg` show.

### Card slot

One person's position in a render shape — `personId`, `kind` (`canonical` or `ghost { reason }`), generation row, and the export-envelope display fields. Card slots come in two visual flavors per the [`docs/canonical-ui-pattern.md`](./docs/canonical-ui-pattern.md) uniform-card section: canonical (solid border, full opacity) and ghost (see [`Ghost slot`](#ghost-slot)). Hierarchical placement (parent `Component`, `MarriageBranch`, or `PersonCard`) anchors the slot in the canonical-pattern tree; the explicit `generation` field is the structural data-level row computed in `kul-render`, and the surface layout row in `kul-layout` (per ADR-0018) is a one-line function of that generation.

### Ghost slot

A card slot with `kind: ghost`. Mute per [ghosts are mute](./docs/canonical-ui-pattern.md#ghosts-are-mute): connects only to the marriage/adoption bar it anchors. Two reasons surface here today: `pastMarriage` (a spouse of an ended marriage whose children need an anchor at the historical bar, or a host who has moved on to a newer current intimacy) and `pastAdoption` (a child-ghost at a past adoptive family, mirrored from the most-recent adoption that owns the canonical card). New reasons land additively per [ADR-0017](./docs/adr/0017-render-shape-schema-and-versioning.md) without bumping the render-shape schema.

## Implementation vocabulary

These names appear in code, ADRs, and architecture discussion.

### KulFile

One parsed `.kul` source file: name, raw source bytes, and a list of **statements**. Type lives at `crates/kul-core/src/ast.rs` (file identity per [ADR-0048](./docs/adr/0048-a-project-is-a-flat-set-of-kul-files.md)). AST nodes inside a `KulFile` carry bare [`ByteSpan`](#span--bytespan)s — their owning `KulFile` provides file context implicitly.

### Document

The multi-file project container: zero or more [`KulFile`](#kulfile)s in [file order](#file-order), each addressable by a [`FileId`](#fileid). Type lives at `crates/kul-core/src/ast.rs`. Each `KulFile` is held behind an `Arc` so callers can keep cheap shared handles ([ADR-0048](./docs/adr/0048-a-project-is-a-flat-set-of-kul-files.md)).

### Statement

The two top-level AST nodes: `Statement::Person(PersonStmt)` and `Statement::Marriage(MarriageStmt)`. Sub-statements (birth, adoption) are nested inside a `PersonStmt`, not top-level.

### FileId

An opaque `Copy` newtype indexing into a [`Document`](#document)'s files (`crates/kul-core/src/span.rs`). `FileId(i)` is the `i`-th `.kul` file in [file order](#file-order). Adapters and tests read ids out of an existing `FileSpan` or the document's `kul_files()`; `FileId::from_raw(u32)` is available for fixture construction.

### Span / ByteSpan

A `(start, end)` byte range into a single source string. Every AST node carries one. Used for hover ranges, goto-definition targets, completion contexts. Type lives at `crates/kul-core/src/span.rs`.

### FileSpan

A `(file: FileId, span: ByteSpan)` pair: the project-wide locator a [`Diagnostic`](#diagnostic) anchors on, and the shape of `EntityRef.span` and `EntityNode.ident_span`. Lives at `crates/kul-core/src/span.rs`. Decouples a span from the implicit "this file" context that AST nodes can rely on ([ADR-0048](./docs/adr/0048-a-project-is-a-flat-set-of-kul-files.md)).

### Lexer / Parser

Two passes in `crates/kul-core/src/`. The lexer produces a flat token stream (`TokenKind` + span); the parser builds the AST. Both are hand-written and small (~350 + ~750 lines). Recovery is ad-hoc per production: hit an error, sync to newline, continue.

### Resolver

The function `kul_core::semantic::resolve(Arc<Document>) -> (ResolvedDocument, Vec<Diagnostic>)`. Walks every [`KulFile`](#kulfile) in the [`Document`](#document), builds the project-wide id-to-statement index, reports duplicate ids (R01) inline as the index is populated, then builds the inverse parenthood indexes (`children_of` / `children_of_marriage`). Lives at `crates/kul-core/src/semantic.rs`.

### ResolvedDocument

The **kinship-query seam** (per [ADR-0001](./docs/adr/0001-resolved-document-as-query-seam.md)). All cross-reference questions ("who are this person's parents?", "who are this person's children?", "is this id declared?", "who are the spouses of this marriage?") are answered by methods on this type. Validator rules and language features query through it; raw AST traversal is reserved for the seam's implementation, not its callers.

Owns its [`Document`](#document) via `Arc<Document>` (per [ADR-0007](./docs/adr/0007-resolved-document-owns-document.md)) so the resolved view can be cached alongside other artifacts. The id index is **project-wide** (per [ADR-0015](./docs/adr/0015-global-project-namespace.md)): `resolved.person(id)`, `resolved.marriage(id)`, `resolved.entity(id)` take only the bare id and return the unique declaration regardless of which file owns it. Iteration queries (`persons()`, `marriages()`, `statements()`) walk every `.kul` file; `_in(file)` variants restrict to one file (the language service uses them for one file's symbols and tokens). One-hop parenthood queries: `parents_of(person)`, `children_of(person)`, `children_of_marriage(marriage)` — the children indexes are owned on the document (statement-idx style, no borrowed lifetimes) so callers do not rebuild inverse adjacency. `references_to(id, kind)` is project-wide too and returns `FileSpan`s; references and rename name each span's file in their results. `node_at(file, offset)` and `statement_at(file, offset)` keep their file parameter because byte offsets are inherently per-file. R01 fires across files; cross-file references resolve cleanly.

### Validator

The pass that runs spec rules R02–R13 over a `ResolvedDocument`, accumulating diagnostics. Lives at `crates/kul-core/src/validator.rs`. Each rule is a function; the validator's job is to call them and collect output. (R01 — duplicate ids — is the one rule that lives inside `semantic::resolve`, because the duplicate check is a property of insertion order as the entity table is built.) Rules R02–R12 iterate one `.kul` file at a time for deterministic source-order diagnostic grouping; R13 walks the project-wide parent graph in one pass so cross-file cycles are detected as single cycles.

### Cycle detector

A standalone algorithm at `crates/kul-core/src/cycles.rs`, called by rule 13 (parenthood cycles). Pure function over the project-wide parent graph; separated from the rule because the algorithm is independently testable and the rule is a thin shell around it. The graph spans every file in the project (per ADR-0015): cycles that cross file boundaries are detected just like within-file cycles.

### Node-at-cursor / `node_at`

The query `ResolvedDocument::node_at(byte_offset) -> Option<Node<'a>>`. Lives at `crates/kul-core/src/node_at.rs`. Returns a typed enum identifying *what's at the cursor* — keyword, identifier declaration, identifier reference (with resolved target), field name, field value. The shared foundation for hover, goto-definition, and completion. See [`docs/architecture.md`](./docs/architecture.md) for the data-flow diagram.

### Entity-reference accessor

The method `Node::entity_reference(&self) -> Option<EntityNode<'a>>` (in `crates/kul-core/src/node_at.rs`) collapses the four id-bearing `Node` variants (`PersonDeclId`, `MarriageDeclId`, `PersonRef`, `MarriageRef`) into a uniform summary: `kind`, `name`, `ident_span`, `is_decl`, and the resolved `target`. Language features that key on "what entity is the user pointing at?" (definition, references, rename, entity-at) phrase themselves as a query for this summary instead of re-pattern-matching the four variants by hand.

The `target` is an `EntityTarget` carrying both the resolved AST statement and the `FileId` that owns it (the `Node::PersonRef`/`MarriageRef` reference variants carry the same `(FileId, &Stmt)` pair). Under project-wide resolution (ADR-0015) that file may be a sibling of the cursor's file; `EntityNode::decl_span()` returns the correct project-wide `FileSpan` directly so feature modules do not re-query `ResolvedDocument::entity(name)` just to recover the target's file.

### Field-node accessor

The method `Node::field_node(&self) -> Option<FieldNode>` (in `crates/kul-core/src/node_at.rs`) is the field-shape sibling of `entity_reference`. It collapses the six field-related `Node` variants (`PersonFieldName`, `PersonFieldValue`, `MarriageFieldName`, `MarriageFieldValue`, `AdoptionFieldName`, `AdoptionFieldValue`) into a uniform `FieldNode { name, name_span, value_span, is_name }` summary, so a feature module that keys on "what field is the cursor on?" (hover, plus any future code-action or completion logic that's field-shape rather than statement-shape) writes one dispatch on `is_name` instead of six match arms. Returns `None` for keywords, ids, and whitespace.

### Language service

The crate `kul-language-service`: every editor feature as a pure function over a checked project (`CheckResult`), a file name and a byte offset — diagnostics, hover, completion, definition, references, prepare-rename and rename, symbols, quick fixes, semantic and lexical tokens, format, entity-at, locate and details. Results are Kul-native: a `Location` is a file name plus a byte `Span`, a rename is one `FileEdit` per affected file, and completion, symbol, token and diagnostic kinds are enums. No IO, no editor protocol, no async runtime; the [`Workspace`](#workspace) is its adapter in the browser.

### Feature module

One per language feature — `crates/kul-language-service/src/{hover,definition,completion,rename,…}.rs`. Each answers through `ResolvedDocument` + `node_at`; none walks the AST directly. Features keyed on the entity under the cursor share one helper, `entity_under`, so definition, references, rename and entity-at agree on what the cursor points at.

### Completion classifier

The token-stream-first context detector in `crates/kul-language-service/src/completion/classify.rs`. Identifies which context the cursor is in (TopLevelStart, IndentedUnderPerson, PersonFieldList, MarriageFieldList, AdoptionFieldList, AfterGenderColon, AfterEndReasonColon, AfterStringFieldColon, MarriageRefPosition, SpousePosition). Token-stream-first because partial / mid-typed input doesn't always parse cleanly. See [ADR-0002](./docs/adr/0002-token-stream-first-completion-classifier.md).

### Lexical tokens / semantic tokens

The two highlighting passes, both from the Rust lexer and syntax tree — there is no hand-written grammar. **Lexical tokens** classify one file's text from the lexer alone (keywords, field names, ids, values, comments), cheap enough for every keystroke and complete even on lines that do not parse. **Semantic tokens** classify from the checked syntax tree once analysis has run, and refine the lexical ones: a consumer paints the lexical tokens and overrides them with semantic ones where both cover the same text. Both use one `TokenKind` enum.

### Workspace

The browser's engine: the stateful `Workspace` class in `kul-wasm` ([ADR-0011](./docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md)). It holds one project's files — each with a caller-supplied **version** tagging its text — checks them on demand, caches the `CheckResult` until the next edit, and serves every [language service](#language-service) feature plus `analyze()` (the per-file versions, the project's diagnostics and the theme-agnostic SVG) and `renderSvg(theme)`. Every offset crossing its boundary is a UTF-16 code unit.

### Utf16Index

The UTF-8 byte offset ↔ UTF-16 code-unit offset converter for one file's text, in `crates/kul-wasm/src/utf16.rs`. The [`Workspace`](#workspace) keeps one per file and converts every offset that crosses the boundary through it — the only place the two units meet. Records only non-ASCII characters, so ASCII text costs nothing.

### Project store

Where the web app keeps a browser's [projects](#project-project-wide-namespace): `apps/web/src/store/`, the only code that touches browser storage (an ESLint rule enforces it; UI preferences live beside it in `src/prefs/`). Each project is a folder in the origin private file system holding `project.json` (name, timestamps and the **window state** — open tabs, active file, editor window, camera) and a `files/` folder of `.kul` files; the project list is the folder listing. The store enforces the file-name rules (`.kul`, no `/` or `\`, at most 128 characters, unique ignoring case) and returns files in [file order](#file-order). One interface, two backends: the origin private file system, and memory for a window that cannot store files.

### Tab ownership

One writer per project: the **owning tab** is the browser tab holding the Web Lock `kul-project-<id>`, and only it writes the project. Any other tab showing the project is **read-only**: it shows the owner's writes as they land (a `BroadcastChannel` of the same name) and offers "Use here instead", which steals the lock and turns the previous owner read-only, discarding its unsaved edits ([ADR-0051](./docs/adr/0051-local-only-by-construction.md)). Implemented in `apps/web/src/store/ownership.ts`.

### Example project

A project made from one of the repository's `examples/`, which the web app bundles at build time. A first visit opens one (`02-three-generations`), named after its family ("The Okeke family"); the rest are offered as "New from example".

### Try it hand-off

How a docs example becomes a project: **Try it** under a clean ```` ```kul ```` block leaves the example's files in `sessionStorage` (`apps/web/src/store/try-it.ts`, key `kul-try-it`, read once) and opens `/app/`, which makes them a new project named "Try it: <page title>". The files never travel in the address. A lone block opens as `family.kul`; a multi-file example opens as its files.

### Offline copy

The whole site as the service worker (`/sw.js`) precached it: every file one build wrote. Content-hashed assets come from it first; pages and text files come from the network first and from it when the network is gone. A new build installs a new copy beside the current one and waits; the page offers **Reload**, which writes the open project's pending edits and then moves to it.

### Language worker

The module Web Worker (`apps/web/src/language/worker.ts`) that holds the open project's [`Workspace`](#workspace) behind Comlink. The main thread's **language client** sends every edit as the file's whole text with a new **version**, and keeps the latest text of each file; every answer names the versions it was computed from, and one an edit has overtaken is dropped. Edits answer with lexical tokens at once; analysis waits for edits to pause and is pushed to the main thread. A crashed worker is replaced and given the file set again.

### Editor window

The app's code editor: CodeMirror over the canvas, with the file list, tabs and the **problems strip** (the whole project's errors and warnings). It floats, docks to the left or right side of the canvas, or minimizes to a **pill**; its layout is part of the project's window state. Highlighting comes only from the engine's lexical and semantic tokens — there is no grammar in the app.

### Canvas

The ground the family tree is drawn on in the app, under the editor window. Each analysis that can draw the project swaps the engine's theme-agnostic SVG into it, and the app's token sheet paints the SVG's classes in the light or dark theme. A [camera](#camera) (pan and zoom, 10%–800%) shows the tree: it fits the first tree a project draws and then stays put across edits, and it is saved with the project. While the project has errors the canvas keeps the last tree it could draw, dimmed. Ghost cards gain a return badge, and a collapsible legend lists the categories the tree draws. The [entity selection](#entity-selection) and the [linked entity](#linked-entity) are its marks.

### Camera

The canvas's view of the tree: a translation and a scale (`Camera {x, y, zoom}`, 10%–800%), applied as a CSS transform (`apps/web/src/canvas/camera.ts`). It fits the first tree a project draws — never enlarging past 100% — and then stays where the reader put it across every edit and re-render; it is saved in the project's window state and restored on open. Drag, the wheel (with Ctrl/⌘ or a pinch to zoom), the zoom controls and the canvas keys move it ([ADR-0052](./docs/adr/0052-the-canvas-contract.md)).

### Linked entity

The entity under the editor's cursor, marked on the [canvas](#canvas) more softly than the [entity selection](#entity-selection). It follows the cursor once it rests (about 150 ms) on a declaration or a resolved reference, in any file, and the canvas brings it into view if it is off screen or hidden ([occlusion-aware centring](#occlusion-aware-centring)). Following the cursor never moves the keyboard out of the editor.

### Entity selection

The one entity the reader has chosen on the [canvas](#canvas) — **the selection**: a person (their card) or a marriage (its line), keyed by its id. Clicking a card or a marriage line selects it — so do Enter and Space on one in keyboard focus, the palette's "Show details for the entity at the cursor", and a related entity named in the [details panel](#details-panel); Esc or a click on the background clears it. Selecting puts the editor's cursor on the entity's declaration without taking the keyboard. **A selection survives edits**: after every render it is applied again by id, and it ends only when the new tree no longer draws the entity (it was deleted). Implemented in `apps/web/src/canvas/selection.ts`, beside the [linked entity](#linked-entity).

### Details panel

What the canvas says about the [entity selection](#entity-selection): a side panel along the canvas's right edge that leaves the rest of the canvas usable and never takes the keyboard. It is fed **only** by the language service's `details` answer — never by reading the drawing — through a pure model (`apps/web/src/canvas/details.ts`): a person's names, gender and dates (partial and circa dates as written), parents, adoptions with their dates, marriages with their dates and end reasons, and children; a marriage's spouses, dates, end reason and children. Absent fields are omitted. Every related person or marriage in it selects that entity, and **Go to source** opens the editor at the declaration. `details` answers nothing while the project has errors, so the panel keeps what it last showed.

### Occlusion-aware centring

Bringing part of the tree into view without parking it behind the chrome that floats over the [canvas](#canvas) — the editor window and the [details panel](#details-panel) (`apps/web/src/canvas/reveal.ts`). The visible region is the widest full-height strip no occluder covers, kept 24 px clear of each, measured from the occluders' boxes so nothing assumes which side they are on; a card touching an occluder counts as hidden. **Revealing** (a selection or a [linked entity](#linked-entity) that is off screen or hidden) moves the camera the least distance that brings the entity inside that region, so the tree around it stays where the reader left it; **jumping** from a [ghost](#ghost-slot)'s return badge centres the person's own card in it.

## When this glossary is incomplete

If you're naming a concept that isn't here:

- **Common case** — you're inventing language the project doesn't use. Find the canonical term and use it.
- **Real gap** — the concept genuinely doesn't have a name yet. Add it here in the same change, with a one-paragraph definition. If the concept is load-bearing enough that future agents will need to understand *why* it exists, also write an ADR.

Architecture vocabulary (module / interface / seam / depth / adapter / leverage / locality / deletion test) is intentionally not duplicated here — see [`docs/architecture.md`](./docs/architecture.md).

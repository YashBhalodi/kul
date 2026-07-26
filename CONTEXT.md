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

A sub-statement under a person: `adoption <marriage_id> start:…`. Declares this person as adopted into the named marriage. A person may have both a `birth` (their biological origin) and one or more `adoption`s; all surface in the **parent set**.

### Field

Any `key:value` pair on a Person, Marriage, Birth, or Adoption. Fields are optional unless the spec marks them required (see [`spec/04-validation-rules.md`](./spec/07-validation-rules.md)). They are unordered. Repeating a field in the same declaration is an error (KUL-R15).

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

The inverse of parent. There is no `child` declaration in the language — children are **derived**, not declared on parents. (This is what makes the additivity principle hold: adding a new child to a family does not require editing the parents' declarations.)

### Validator rule

One of the fifteen spec-defined checks (KUL-R01 through KUL-R15). See [`spec/04-validation-rules.md`](./spec/07-validation-rules.md). In code, each rule lives as a function in `crates/kul-core/src/validator.rs` named `rule_NN_<short_name>`; tests follow the same pattern.

### Diagnostic

An error or warning emitted by the manifest validator pass, the parser, the resolver, or the validator. Carries a **code** (`KUL-Mxx` for manifest, `KUL-Lxx`/`KUL-Pxx` for lex/parse, `KUL-Rxx` for validator rules), a **severity**, a **message**, an optional **primary** [`FileSpan`](#filespan), and optional **related** spans (each anchored to a `FileSpan`, possibly in a sibling file under project-wide resolution). The optional primary covers `KUL-M01` (manifest-not-found) — the only diagnostic with no source position to anchor at. `KUL-M06` (project has `kul.yml` but zero `.kul` files) anchors at the manifest. Rendered to the user via `miette` (CLI) or translated to LSP diagnostics (editor); the latter filters to the active URI's `FileId`.

### ExportEnvelope

The top-level value `kul export` (and the public `kul_core::export::export` function) emits. Either a **success envelope** carrying a `schema` number, the source's `kul` language version (sourced from the [`Manifest`](#manifest)), and the [`ExportedGraph`](#exportedgraph), or a **failure envelope** carrying the diagnostic list. The export is strict on errors per [ADR-0009](./docs/adr/0009-export-strict-on-diagnostics.md).

### Project manifest

The `kul.yml` file alongside one or more `.kul` files. Carries the Kul language version the source targets and (in the future) any project-level configuration. Required: a `.kul` file without a sibling `kul.yml` is not a valid Kul project. Discovery is directory-scoped — no walk-up. Defined normatively in [`spec/14-project-manifest.md`](./spec/14-project-manifest.md); decision recorded in [ADR-0013](./docs/adr/0013-project-manifest.md).

### Project (project-wide namespace)

A directory containing one `kul.yml` plus one or more `.kul` files. Every id declared in any of the project's `.kul` files is visible from every other file by bare name — there is no `import` statement, no namespace prefix, and no qualified-reference syntax. The file boundary is purely organizational; the project is one logical namespace. Subdirectories are not walked; non-`.kul` files are silently ignored. Defined normatively in [`spec/14-project-manifest.md`](./spec/14-project-manifest.md); decision recorded in [ADR-0015](./docs/adr/0015-global-project-namespace.md), which supersedes ADR-0014's Position B.

### Manifest

The typed Rust representation of the project manifest. Lives at `crates/kul-core/src/manifest.rs` as `pub struct Manifest { pub kul_version: String }`. Adapters (`kul-cli`, `kul-lsp`, `kul-wasm`) load the on-disk YAML and hand the **raw bytes** to `kul_core::check` (or hand a typed `Manifest` to `kul_core::check_with_manifest` from the WASM bridge); `kul-core` itself never reads the filesystem. The `manifest::validate(yaml, file)` pass produces a typed `Manifest` plus diagnostics with normative `KUL-M01..M05` codes anchored at the manifest's [`FileId`](#fileid) (per [ADR-0014](./docs/adr/0014-file-identity-and-per-file-namespaces.md)).

### CheckEnvelope

The top-level value `@kullang/wasm`'s `check(source)` function returns. A single-field object — `{ diagnostics: ExportedDiagnostic[] }` — carrying every diagnostic the validator produced (errors, warnings, and notes alike). An empty array means a clean document; consumers discriminate on emptiness, with no `ok` field. The diagnostic shape reuses [`ExportEnvelope`](#exportenvelope)'s failure-arm `ExportedDiagnostic` so CLI export and WASM check agree on one source of truth. Defined at `crates/kul-wasm/src/lib.rs`; surface decision recorded in [ADR-0011](./docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md).

### ExportedGraph

The kinship-native graph projection inside a success [`ExportEnvelope`](#exportenvelope). Three flat collections — `persons`, `marriages`, `parenthood_links` — that mirror the language primitives one-to-one, with cross-references by id. Defined normatively in [`spec/16-export-schema.md`](./spec/16-export-schema.md); shape choice motivated in [ADR-0008](./docs/adr/0008-export-kinship-native-shape.md).

### Schema number

The `schema:` integer on a success [`ExportEnvelope`](#exportenvelope). Discriminator for the structural shape of the envelope; bumped only when consumers might silently mis-represent data by ignoring a new construct. Independent of the language version (the `kul:` field). Policy in [ADR-0010](./docs/adr/0010-export-schema-versioning.md).

### Query seam

The `query` module in `crates/kul-core/src/query.rs` (introduced by [ADR-0024](./docs/adr/0024-query-seam-and-envelope.md)) — the single public seam where all kinship-query capabilities live. It is a deep module **layered over [`ResolvedDocument`](#resolveddocument)**, not over the [`ExportedGraph`](#exportedgraph): the export is the escape hatch for consumers who don't use the engine, never the engine's own input. This first slice carries the two [detail lookups](#detail-lookup); later slices add kin-set queries, relationship resolution, and attribute filtering onto the same seam (PRD 0005, epic [#253](https://github.com/YashBhalodi/kul/issues/253)).

### Detail lookup

An id → detail lookup on the [query seam](#query-seam): `query::person(id)` returns `Option<ExportedPerson>` and `query::marriage(id)` returns `Option<ExportedMarriage>` — the **same serialized shapes the export produces** (single-sourced through the export's `build_one_person` / `build_one_marriage` builders, so a lookup and a whole-graph export can never drift). Lookup semantics are **absence-is-the-answer**: an unknown id, or an id of the wrong kind (a marriage id asked for as a person, or vice versa), yields `None`. There is no error type here — "no such entity" is a complete, honest answer. (Typed unknown-id errors arrive in later slices, where an id is an *input anchor* to a relationship question rather than the subject of the question.) The [batched detail lookup](#batched-detail-lookup) is the many-entities-at-once member of the same family.

### Batched detail lookup

**One check, any number of entities**: `query::details(resolved, &[DetailTarget])` (envelope form `query::detail_lookup`, WASM `queryDetail`) answers each target with the entity's own fields *plus* its relational neighbourhood — parents, spouses and children carrying their `biological` / `adoptive` link kinds. It exists because the single-entity [detail lookups](#detail-lookup) cannot answer a detail panel (`ExportedMarriage` carries no children, and there is no parenthood-link lookup) and because composing them re-pays the project check on every stateless call; batching makes the cost **flat in the number of targets** ([ADR-0035](./docs/adr/0035-detail-surface-one-selection-one-batched-lookup.md), [ADR-0037](./docs/adr/0037-batched-detail-lookup-shape.md)). Defined in `crates/kul-core/src/query/detail.rs`.

- A **detail target** (`DetailTarget`) is what one entry addresses: a person id, a marriage id, or — since an adoption link has no id of its own — an adoption's `(childId, marriageId)` pair, the same pair the rendered adoption edge carries.
- An **entity detail** (`EntityDetail`) is the answer for one target: a union tagged on `kind` with a `person`, `marriage` and `adoption` variant, so a consumer's panel variant maps 1:1 onto it. Every entity inside is an export shape (`ExportedPerson` / `ExportedMarriage` / `ExportedParenthoodLink`) — there is no second, leaner person shape, which is what lets this one operation also supply a [kin-set](#kin-set-query) list's row labels.
- A **linked person** (`LinkedPerson`) is a neighbour plus the parenthood link that reaches them; a **marriage tie** (`MarriageTie`) is a marriage plus the person on the other side. One row per *link*, never per person — [path identity](#path-identity), so a parent reached by both birth and adoption is two rows.
- Semantics are **absence-is-the-answer per target**: a target naming no entity is `null` in its own position while the rest of the batch answers. A project that fails its checks is the [query envelope](#query-envelope)'s error arm, whole.

### Query envelope

The adapter-facing result of a query operation: `QueryEnvelope<T>` in `crates/kul-core/src/query.rs`. An untagged union discriminated by an `ok` boolean — mirroring the [`ExportEnvelope`](#exportenvelope) / render-envelope pattern rather than introducing a new tag — whose ok arm carries the query `result` and whose error arm carries the `diagnostics` of a project that failed its checks. Gated on passing checks (strict-on-errors per [ADR-0009](./docs/adr/0009-export-strict-on-diagnostics.md)): a failing project yields the error arm, never a partial answer. Single-sourced in `kul-core` so the CLI `kul query --format json` output stays byte-identical to what the WASM `queryPerson` / `queryMarriage` surface returns. It is the fourth WASM shape (extends [ADR-0011](./docs/adr/0011-wasm-surface-three-shapes-no-wrappers.md)); its TypeScript types ship under the committed-tsify discipline of [ADR-0012](./docs/adr/0012-tsify-derived-types-committed-and-diffed.md). Generic over `T` so later slices reuse it.

### Query value

The declarative, serializable request the query engine evaluates: a `source` (`allPersons` or `kinOf(anchor, pattern)`), an optional [`where` conjunction](#attribute-filter) of [predicates](#three-valued-predicate), an optional `sort`, a [certainty `mode`](#certainty-mode), and a `projection` (`members` or `count`). Defined at `crates/kul-core/src/query/pattern.rs`. It is **the single contract with one evaluation path** ([ADR-0025](./docs/adr/0025-kinship-query-engine-contract-and-traversal.md)): every surface — Rust [named sugar](#kin-set-query) and builders (`Query::all_persons().filtered(…).sorted(…).counting()`), WASM `queryKin` / `runQuery`, CLI `kul query kin` / `kul query persons` — constructs a Query value and hands it to the one `query::run_query` entry point. There is no second engine and no query DSL. Enum variants stay additive: the result is `members` (a `kinOf` set with descriptors), `personIds` (an `allPersons` set, ids only — hydrate via a [detail lookup](#detail-lookup)), or `count` (the filtered set's size), fixed by (source, projection).

### Attribute filter

The **permanently-modest third query capability**: filter persons by their *own* field [predicates](#three-valued-predicate), sort deterministically, and count — composable onto an `allPersons` source or the output of a [kin-set query](#kin-set-query) in one [Query value](#query-value). Deliberately small and *pinned* ([ADR-0025](./docs/adr/0025-kinship-query-engine-contract-and-traversal.md), PRD 0005): exact/case-sensitive string matching (`=`/`≠`/set membership — **no substring, no regex, permanently**; `family` is a first-class field), [three-valued](#three-valued-predicate) interval-date comparison (`<`/`≤`/`>`/`≥` and `=`/`≠` as containment), and presence (`present`/`absent`). Composition is **conjunction-only** — the `where` predicates AND together; **there is no OR** (that is two queries and a set union in consumer code). No cross-field or cross-entity predicates (no age arithmetic, no "alive in 1985", no "spouse's family"), no group-by, no statistics, no date arithmetic — those live on the [exported graph](#exportedgraph), in consumer code. Sort is fully deterministic (dates by interval bounds, strings by codepoint, **missing values last regardless of direction**, ties broken by id ascending) so snapshots stay stable. `count` is the final filtered set's size. Implemented in `crates/kul-core/src/query/filter.rs`; surfaced as WASM `runQuery` and CLI `kul query persons` (plus the same `--where` / `--sort` / `--count` / `--include-uncertain` flags on `kul query kin`).

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

The **two-anchor** query on the [query seam](#query-seam): given persons `x` and `y`, `query::resolve(x, y, config)` returns *all* the ways they are related — a `ResolveResult` (in `crates/kul-core/src/query/resolve.rs`) carrying one [relationship descriptor](#relationship-descriptor) per distinct path (same [path identity](#path-identity), same [descriptor](#relationship-descriptor) derivation, same [traversal engine](#query-seam) as the [kin-set queries](#kin-set-query) — **resolution never forks the kin-set logic**) plus, **only when the list is empty**, an [emptiness reason](#emptiness-reason). It is a *separate call*, not a [Query value](#query-value) pipeline stage — a different question ("how are these two related?" vs "who are this person's …?"). Enumeration is all simple paths under the full grammar where every blood segment stays within the [generation budget](#generation-budget); pure-lineal (direct ancestor/descendant) ties are additionally detected **unbounded**, regardless of the budget, so a `noneWithinBounds` never hides a recorded direct-line tie. `x == y` yields a single `self` descriptor (empty path). Results are sorted by (path hop count) → (serialized backbone). An unknown or wrong-kind id (either anchor) is a typed `UnknownPerson` error, never an empty result. Surfaced as WASM `queryResolve` and CLI `kul query rel <x> <y> [--max-generations N]` ([ADR-0028](./docs/adr/0028-relationship-resolution-and-honest-emptiness.md), PRD 0005).

### Emptiness reason

The honest "why" a [relationship resolution](#relationship-resolution) found nothing — present on `ResolveResult.emptyReason` **iff** the relationship list is empty: `disconnected` (`x` and `y` lie in different connected components of the full relation graph — undirected reachability over every parent-child edge of both kinds plus every spouse edge — so raising the budget can never help) or `noneWithinBounds` (same component, but nothing is derivable under the semantics and the current [generation budget](#generation-budget) — a bigger budget might reveal a tie). Collapsing the two into a bare empty list would let an app render "not related" when the truth is "not related as far as we looked"; the distinction is the product ([ADR-0028](./docs/adr/0028-relationship-resolution-and-honest-emptiness.md)).

### Generation budget

The one knob of [relationship resolution](#relationship-resolution): `maxApexGenerations` (default 5) bounds **each blood segment's** ascent and descent — a nearest-common-ancestor bound, not a total-path-length bound. It is a caller *budget*: 5 reaches through fourth cousins (a strict superset of every lexicalized kinship term — cultures run out by third cousins) while cutting off the remote-ancestor haystack; a caller who wants to look nearer or farther legitimately says so. Contrast the fixed [affinal ceiling](#affinal-ceiling), which is *semantics*, not a budget — see the [semantics-vs-budget line](#semantics-vs-budget-line).

### Semantics-vs-budget line

The distinction that governs what is a caller knob and what is fixed engine law. The 2-hop [affinal ceiling](#affinal-ceiling) is **semantics**: it defines *what counts as a relationship* (no culture names a three-affinal-hop tie), so it is never configurable — two apps must never disagree about *what relationships exist*. The [generation budget](#generation-budget) is a **search budget**: two apps may legitimately look differently far, so it is a caller knob. The line keeps the contract honest — a bigger budget can reveal more of the *same* relationship space, but can never change its definition ([ADR-0027](./docs/adr/0027-affinal-traversal-ceiling-and-step-subsumption.md), [ADR-0028](./docs/adr/0028-relationship-resolution-and-honest-emptiness.md)).

### Phrasing layer

The consumer-side layer that turns a [relationship descriptor](#relationship-descriptor) into a **word** — the layer the engine refuses to own, because "the moment the core emits 'grandmother' it becomes the first culture pack, shipped by accident" ([ADR-0026](./docs/adr/0026-relationship-descriptor-and-path-identity.md)). It lives at `packages/preview/src/phrasing/` as a pure module under a standing **no DOM imports** rule, so extracting it to its own package stays mechanical ([ADR-0033](./docs/adr/0033-phrasing-layer-architecture.md)). Its whole surface is `phrase(descriptor, pack) → { text, kind, hopCount, translit? }`: `kind` is `lexical` (a term the language has) or `composed` (one it had to build), `hopCount` is how many hops the phrase spells out — never a bare string, so chrome can tell a crisp *kākā* from a five-hop chain, and size the chain, without string-sniffing. `hopCount` is `0` for **every** lexical phrase by construction, so it measures a chain and never a term's own width — a term can be *kākā* or *second cousin twice removed*, and chrome that sizes a lexical slot from it will overflow. `translit` is the [transliteration](#transliteration) where the pack supplies one. Phrases are **bare** ("mother's brother", not "your mother's brother"); ego-relative framing is chrome copy.

### Phrasing key

The input to phrasing: one flat record of facets a [language pack](#language-pack) matches on — the descriptor's normalized dimensions (`classification` with its materialized `cousinDegree` / `removed`, `edgeNature`, `affinity`, `sharing`, `side`, both seniorities, `egoGender`, `alterGender`) plus six [derived facets](#derived-facet). Two facets carry a value the wire form does not have: `sharing` and `side` gain `unknown`, which only a **sub-path key** (the key of a backbone prefix) produces. `unknown` never matches — an entry keying a facet that is `unknown` on the key is **disqualified**, so lookup falls through to the unmarked term (`seniority: unknown` yields *bhāī*, never a guessed *moṭā bhāī*). `notApplicable` is a *value* — a fact about the path's shape — and matches normally. Defined at `packages/preview/src/phrasing/key.ts`.

### Derived facet

One of the six facets the phrasing key computes from the [path backbone](#path-backbone) rather than reading off the descriptor: `acrossCount`, `acrossAtStart`, `acrossAtEnd`, `spouseGender` (the person a path-initial [affinal hop](#affinal-hop) lands on), `linkGender` (the person the **first `down` hop** lands on), and `endedMarriage`. All six are pure functions of the hop sequence — **no engine change, no new wire field**. The set is closed and validated: it separates the six Gujarati terms that share one normalized cell (*sāḷo* / *jeṭh* / *naṇand* / *bhābhī* / *banevī* / *vevāī*), and English needs two of them as well (*stepson* vs *son-in-law*). This is the escape hatch the lossless backbone was designed to be, paying out for the first time.

### Language pack

A language's terminology as **data**: lexical entries (each a partial match over the [phrasing key](#phrasing-key) plus the term it yields), [affix rules](#affix-rule), a hop lexicon (gendered nouns for `up` / `down` / `across`), a genitive pattern, and — for a pack not written in Latin script — the [transliteration](#transliteration) of every one of those records. An omitted facet is a wildcard, and **precedence is by specificity** — the entry keying the most facets wins; declaration order carries no meaning, and two entries tying for maximum specificity with *different* terms is a pack defect a test catches ([ADR-0039](./docs/adr/0039-phrasing-lookup-mechanics.md)). Adding a language is one additive module of records plus one line in the `PACKS` registry — **zero lines of logic**, which is the additivity promise ADR-0026 designed a maximally-discriminating descriptor to keep. Packs live at `packages/preview/src/phrasing/packs/`; two ship — `en` and `gu`, the second of which is the additivity promise's test ([ADR-0041](./docs/adr/0041-the-gu-pack-and-the-locale-toggle.md)).

### Affix rule

The productive-morphology half of a [language pack](#language-pack): `{ when, affix, position, repeatPer?, cap? }`. A flat table cannot carry English's unbounded `great-great-…` spine or Gujarati's single productive `par-`, so packs declare rules that decorate a matched term — `grand` from `generations = 2`, `great-` repeating from 3, `former ` on a non-numeric `when: { endedMarriage: true }`. The affix is concatenated literally (hyphenation is the pack's business), rules apply innermost-first in an order derived from their own thresholds, and `cap` **refuses** lexicalization past the language's ceiling so the [prefix lexicalization](#prefix-lexicalization) fallback fires instead of coining a word.

### Prefix lexicalization

The phrasing layer's fallback for the large unlexicalized regions (second cousins on, removed cousins, distant affinal ties): walk the [path backbone](#path-backbone)'s prefixes longest to shortest, derive a sub-path [phrasing key](#phrasing-key) for each, look it up in the **same table**, and render the remaining hops as a genitive chain off the first hit. So the composed form reuses a term the language has — *māmā-no dīkro*, "first cousin's wife" — rather than walking every hop from ego, and there is **no second lexicon**. A sub-path key marks `sharing`, `apexSeniority`, `seniority` and the `side = both` refinement `unknown`, because a hop sequence carries neither the graph around it nor birth dates; the never-guess rule then makes that automatically safe. **Exactly one prefix is lexicalized** — the tail is spelled hop by hop and never re-read as a relationship of its own, so an `up·down` tail reads "X's mother's son", not "X's brother" ([ADR-0039](./docs/adr/0039-phrasing-lookup-mechanics.md)).

### Locale toggle

The control in the preview chrome that picks which [language pack](#language-pack) everything phrases against ([ADR-0033](./docs/adr/0033-phrasing-layer-architecture.md), built in [ADR-0041](./docs/adr/0041-the-gu-pack-and-the-locale-toggle.md)). A toggle rather than a setting or a document field, because the compare gesture is the point: flipping one English *brother-in-law* against the six Gujarati terms it merges is genuinely useful. Phrasing stays **one language at a time** — never bilingual stacking — and the button reads in the language it offers, with the transliterated language name on hover; the chrome copy around it stays English, since chrome i18n is a separate question. It joins the flow region of the [region model](#region-model) by being appended, and per [ADR-0036](./docs/adr/0036-two-tier-theming-and-the-accessibility-non-goal.md) it carries no `role`, `aria-*` or `:focus-visible`. Lives at `packages/preview/src/locale.ts`; an element bound to a relationship re-phrases on every flip and carries its own per-element `lang`.

### Locale store

The seam the [locale toggle](#locale-toggle)'s choice persists through: `read()` / `write(code)`, nothing else. **Webview-local** — the choice belongs to this preview panel, not to `kul.yml` (normative, and it would bind every third-party Kul consumer for a preview UX feature) and not yet to a user setting (which it can graduate to additively). In-memory by default so a plain embedding needs nothing; VSCode-backed via `getState` / `setState` in `adapter-vscode.ts`, where it survives the panel being hidden and restored. `HostAdapter` is untouched — this is a second, independent seam, which is what keeps the preview host-agnostic. Defined at `packages/preview/src/locale-store.ts`.

### Transliteration

A [language pack](#language-pack)'s terms in Latin script, carried as a **record beside each term** — `translit` on every lexical entry and [affix rule](#affix-rule), plus Latin twins of the hop lexicon and the genitive pattern ([ADR-0044](./docs/adr/0044-the-hover-lens-transliteration-as-data-and-the-unspent-hue.md)). It is what a reader who does not read the script yet sees on hover: *કાકા* glosses *kākā*, and *મામાનો પુત્ર* glosses *māmā-no putra*, so a composed genitive chain reads as well as a crisp term does. **Nothing is romanized from the script** — there is no transliteration function in the phrasing layer and there must never be one; a phrase carries a Latin reading only when every token it was built from has one, and otherwise carries none at all, which is the never-guess rule applied to a second reading. A pack romanizes **completely or not at all** (a suite holds it there), and `en` romanizes nothing, because a Latin term is already its own reading.

### Render shape

The top-level value `kul_render::compute` and `kul_render::transform` emit — the canonical UI pattern's data form for a checked Kul project. Where [`ExportEnvelope`](#exportenvelope) is shaped to mirror what the source *says* (kinship-native), the render shape is shaped to mirror what the canonical UI pattern *draws*: components, marriage branches, card slots, ghosts. Either a success payload (`components`, `edges`, plus the `schema` / `kul` discriminators) or a failure payload (the same diagnostic list the input envelope carried). Defined at `crates/kul-render/src/shape.rs`; the principles it realizes live in [`docs/canonical-ui-pattern.md`](./docs/canonical-ui-pattern.md); crate-boundary rationale in [ADR-0016](./docs/adr/0016-visualization-pipeline-crate-boundaries.md); schema-versioning policy in [ADR-0017](./docs/adr/0017-render-shape-schema-and-versioning.md). Surface renderers (VSCode preview panel, web visualizer, …) consume it.

### Card slot

One person's position in a render shape — `personId`, `kind` (`canonical` or `ghost { reason }`), generation row, and the export-envelope display fields. Card slots come in two visual flavors per the [`docs/canonical-ui-pattern.md`](./docs/canonical-ui-pattern.md) uniform-card section: canonical (solid border, full opacity) and ghost (see [`Ghost slot`](#ghost-slot)). Hierarchical placement (parent `Component`, `MarriageBranch`, or `PersonCard`) anchors the slot in the canonical-pattern tree; the explicit `generation` field is the structural data-level row computed in `kul-render`, and the surface layout row in `kul-layout` (per ADR-0018) is a one-line function of that generation.

### Ghost slot

A card slot with `kind: ghost`. Mute per [ghosts are mute](./docs/canonical-ui-pattern.md#ghosts-are-mute): connects only to the marriage/adoption bar it anchors. Two reasons surface here today: `pastMarriage` (a spouse of an ended marriage whose children need an anchor at the historical bar, or a host who has moved on to a newer current intimacy) and `pastAdoption` (a child-ghost at a past adoptive family, mirrored from the most-recent adoption that owns the canonical card). New reasons land additively per [ADR-0017](./docs/adr/0017-render-shape-schema-and-versioning.md) without bumping the render-shape schema.

## Implementation vocabulary

These names appear in code, ADRs, and architecture discussion.

### KulFile

One parsed `.kul` source file: name, raw source bytes, and a list of **statements**. Type lives at `crates/kul-core/src/ast.rs` (introduced by [ADR-0014](./docs/adr/0014-file-identity-and-per-file-namespaces.md)). AST nodes inside a `KulFile` carry bare [`ByteSpan`](#span--bytespan)s — their owning `KulFile` provides file context implicitly.

### Document

The multi-file project container: the [`Manifest`](#manifest) plus zero or more [`KulFile`](#kulfile)s, each addressable by a [`FileId`](#fileid). Type lives at `crates/kul-core/src/ast.rs`. Each `KulFile` is held behind an `Arc` so callers can keep cheap shared handles. At v1, the toolchain only ever constructs N=1 `kul_files`; the multi-file shape exists so subsequent issues (cross-`.kul`-file resolution, document merging) can build on file-aware spans without further breaking changes ([ADR-0014](./docs/adr/0014-file-identity-and-per-file-namespaces.md)).

### Statement

The two top-level AST nodes: `Statement::Person(PersonStmt)` and `Statement::Marriage(MarriageStmt)`. Sub-statements (birth, adoption) are nested inside a `PersonStmt`, not top-level.

### FileId

An opaque `Copy` newtype indexing into a [`Document`](#document)'s files (`crates/kul-core/src/span.rs`). `FileId::MANIFEST` (= `FileId(0)`) is the project manifest by convention; subsequent ids are the `.kul` files in input order. Adapters and tests reach for `FileId::MANIFEST` or read ids out of an existing `FileSpan`; `FileId::from_raw(u32)` is available for fixture construction.

### Span / ByteSpan

A `(start, end)` byte range into a single source string. Every AST node carries one. Used for hover ranges, goto-definition targets, completion contexts. Type lives at `crates/kul-core/src/span.rs`.

### FileSpan

A `(file: FileId, span: ByteSpan)` pair: the project-wide locator a [`Diagnostic`](#diagnostic) anchors on, and the shape of `EntityRef.span` and `EntityNode.ident_span`. Lives at `crates/kul-core/src/span.rs`. Decouples a span from the implicit "this file" context that AST nodes can rely on; introduced by [ADR-0014](./docs/adr/0014-file-identity-and-per-file-namespaces.md).

### Lexer / Parser

Two passes in `crates/kul-core/src/`. The lexer produces a flat token stream (`TokenKind` + span); the parser builds the AST. Both are hand-written and small (~350 + ~750 lines). Recovery is ad-hoc per production: hit an error, sync to newline, continue.

### Resolver

The function `kul_core::semantic::resolve(Arc<Document>) -> (ResolvedDocument, Vec<Diagnostic>)`. Walks every [`KulFile`](#kulfile) in the [`Document`](#document), builds the project-wide id-to-statement index, and reports duplicate ids (R01) inline as the index is populated. Lives at `crates/kul-core/src/semantic.rs`.

### ResolvedDocument

The **kinship-query seam** (per [ADR-0001](./docs/adr/0001-resolved-document-as-query-seam.md)). All cross-reference questions ("who are this person's parents?", "is this id declared?", "who are the spouses of this marriage?") are answered by methods on this type. Validator rules and LSP features query through it; raw AST traversal is reserved for the seam's implementation, not its callers.

Owns its [`Document`](#document) via `Arc<Document>` (per [ADR-0007](./docs/adr/0007-resolved-document-owns-document.md)) so the resolved view can be cached alongside other artifacts. The id index is **project-wide** (per [ADR-0015](./docs/adr/0015-global-project-namespace.md)): `resolved.person(id)`, `resolved.marriage(id)`, `resolved.entity(id)` take only the bare id and return the unique declaration regardless of which file owns it. Iteration queries (`persons()`, `marriages()`, `statements()`) walk every `.kul` file; `_in(file)` variants restrict to one file (the LSP uses them for per-URI symbol listings). `references_to(id, kind)` is project-wide too and returns `FileSpan`s; per-URI LSP consumers (find-references, rename) filter to the active file at the call site. `node_at(file, offset)` and `statement_at(file, offset)` keep their file parameter because byte offsets are inherently per-file. R01 fires across files; cross-file references resolve cleanly.

### Validator

The pass that runs spec rules R02–R13 over a `ResolvedDocument`, accumulating diagnostics. Lives at `crates/kul-core/src/validator.rs`. Each rule is a function; the validator's job is to call them and collect output. (R01 — duplicate ids — is the one rule that lives inside `semantic::resolve`, because the duplicate check is a property of insertion order as the entity table is built.) Rules R02–R12 iterate one `.kul` file at a time for deterministic source-order diagnostic grouping; R13 walks the project-wide parent graph in one pass so cross-file cycles are detected as single cycles.

### Cycle detector

A standalone algorithm at `crates/kul-core/src/cycles.rs`, called by rule 13 (parenthood cycles). Pure function over the project-wide parent graph; separated from the rule because the algorithm is independently testable and the rule is a thin shell around it. The graph spans every file in the project (per ADR-0015): cycles that cross file boundaries are detected just like within-file cycles.

### Node-at-cursor / `node_at`

The query `ResolvedDocument::node_at(byte_offset) -> Option<Node<'a>>`. Lives at `crates/kul-core/src/node_at.rs`. Returns a typed enum identifying *what's at the cursor* — keyword, identifier declaration, identifier reference (with resolved target), field name, field value. The shared foundation for hover, goto-definition, and completion. See [`docs/architecture.md`](./docs/architecture.md) for the data-flow diagram.

### Entity-reference accessor

The method `Node::entity_reference(&self) -> Option<EntityNode<'a>>` (in `crates/kul-core/src/node_at.rs`) collapses the four id-bearing `Node` variants (`PersonDeclId`, `MarriageDeclId`, `PersonRef`, `MarriageRef`) into a uniform summary: `kind`, `name`, `ident_span`, `is_decl`, and the resolved `target`. LSP features that key on "what entity is the user pointing at?" (goto-definition, find-references, rename) phrase themselves as a query for this summary instead of re-pattern-matching the four variants by hand.

The `target` is an `EntityTarget` carrying both the resolved AST statement and the `FileId` that owns it (the `Node::PersonRef`/`MarriageRef` reference variants carry the same `(FileId, &Stmt)` pair). Under project-wide resolution (ADR-0015) that file may be a sibling of the active URI's file; `EntityNode::decl_span()` returns the correct project-wide `FileSpan` directly so feature modules do not re-query `ResolvedDocument::entity(name)` just to recover the target's file.

### Field-node accessor

The method `Node::field_node(&self) -> Option<FieldNode>` (in `crates/kul-core/src/node_at.rs`) is the field-shape sibling of `entity_reference`. It collapses the six field-related `Node` variants (`PersonFieldName`, `PersonFieldValue`, `MarriageFieldName`, `MarriageFieldValue`, `AdoptionFieldName`, `AdoptionFieldValue`) into a uniform `FieldNode { name, name_span, value_span, is_name }` summary, so a feature module that keys on "what field is the cursor on?" (hover, plus any future code-action or completion logic that's field-shape rather than statement-shape) writes one dispatch on `is_name` instead of six match arms. Returns `None` for keywords, ids, and whitespace.

### Server

The `tower-lsp` Backend implementation in `crates/kul-lsp/src/server.rs`. Owns the project cache, dispatches LSP requests to feature modules, advertises capabilities. `did_open` discovers the project from the opened URI (sibling `kul.yml` plus every `.kul` file in the URI's directory) and inserts one cache entry for the whole project; `did_change` mutates the URI's overlay and re-runs `kul_core::check` for the project; `did_close` flips the URI's overlay to `None` and evicts the entry when no URIs remain open. Diagnostic publishes broadcast to every project file (open or disk-only) so the Problems pane reflects project-wide health.

### Document cache

Project-keyed map from `ProjectRoot` (the URI's parent directory) to a [`ProjectEntry`](#projectentry) in `crates/kul-lsp/src/state.rs`. Every URI that belongs to one project shares the same cached `CheckResult` and `ResolvedDocument` — opening a second `.kul` file from the same directory does not trigger a second resolve. Updated on `did_open` / `did_change` / `did_close`; evicted when the last open URI of a project closes.

### ProjectEntry

The cached value in the project cache (`crates/kul-lsp/src/state.rs`). Bundles the project's [`CheckResult`](#resolveddocument), the per-file metadata (URL + [`LineIndex`](#lineindex)) in `FileId(1..)` order (so features can map `FileId` ↔ `Url` and translate spans to LSP ranges), and the per-URI overlay map (editor-buffer source for open URIs, `None` for files only on disk). Cross-file features (goto-definition, find-references, rename, completion) read through this single entry; the per-file URL is what turns a project-wide query result into a `Vec<Location>` keyed by the right URIs.

### View / Cursor

The per-request handles `ProjectEntry::view_for_uri(uri)` and `ProjectEntry::cursor_for_uri(uri, position)` return (`crates/kul-lsp/src/state.rs`). A `View` bundles the URI's `FileId`, the cached [`ResolvedDocument`](#resolveddocument), and the [`LineIndex`](#lineindex) for file-level LSP requests (document-symbol, semantic-tokens). A `Cursor` adds the byte offset for cursor-shaped requests (hover, definition, completion, references, rename, prepare-rename). Replaces the three-line `offset / file / resolved` setup every cursor-shaped request handler would otherwise repeat inline; the UTF-16 ↔ UTF-8 conversion lives in one place. Returns `Option<Cursor<'_>>` so a stale client request past EOF — or a URI that isn't part of this project — resolves to `None` rather than panicking. `Cursor::entity()` is the shared accessor the cursor-shaped id-navigation features (goto-definition, find-references, rename) call to get the person/marriage [`EntityNode`](#entity-reference-accessor) under the cursor, and `ProjectEntry::location_for(FileSpan)` maps a project-wide span to an LSP `Location`.

### Feature module

One per LSP feature — `crates/kul-lsp/src/features/{hover,definition,completion,diagnostics}.rs`. Each turns a typed request into a typed response by reading the document cache and querying through `ResolvedDocument` + `node_at`. None should walk the AST directly.

### Completion classifier

The token-stream-first context detector in `features/completion.rs`. Identifies which of seven contexts the cursor is in (TopLevelStart, IndentedUnderPerson, PersonFieldList, MarriageFieldList, AdoptionFieldList, AfterGenderColon, AfterEndReasonColon). Token-stream-first because partial / mid-typed input doesn't always parse cleanly. See [ADR-0002](./docs/adr/0002-token-stream-first-completion-classifier.md).

### LineIndex

Byte-offset ↔ LSP-position converter in `crates/kul-lsp/src/convert.rs`. Handles UTF-16 code-unit positions (LSP spec) ↔ UTF-8 byte offsets (kul-core), with CRLF round-trip safety.

### Tier-1 token / Tier-2 token

The two layers of the preview's `--kul-*` custom properties ([ADR-0036](./docs/adr/0036-two-tier-theming-and-the-accessibility-non-goal.md), sized in [ADR-0038](./docs/adr/0038-tier-1-register-overlay-stack-and-the-reserved-carve-out.md)). A **tier-1 token** is part of the theme contract: a colour role, a typography choice, a diagram hue, a step on the spacing / radius / motion scales, or one of the reserved query paints. Tier 1 is the only layer that may bridge to a `--vscode-*` palette variable, and it is a closed register — chrome growth does not enlarge it. A **tier-2 token** is the per-site semantic layer (`--kul-legend-border-color`, `--kul-error-row-hover-bg`): exactly one per application site, aliasing a tier-1 primitive rather than a `--vscode-*` variable. Both live in `packages/preview/src/preview-themes.css` — tier 1 inside a per-theme `body[data-theme="…"]` block, tier 2 on `body` — while the application rules that consume tier 2 live in `preview.css`. A theme re-maps tier 1 and may override any single tier-2 alias. `crates/kul-svg` bakes tier 1 plus the diagram half of tier 2 into self-contained SVG export; `packages/preview/tests/tokens.test.ts` holds the split as a structural lint.

### Region model

How the preview stage assigns space to chrome ([ADR-0036](./docs/adr/0036-two-tier-theming-and-the-accessibility-non-goal.md), mechanism in [ADR-0038](./docs/adr/0038-tier-1-register-overlay-stack-and-the-reserved-carve-out.md)). The stage declares five **regions** — `flow` (persistent chrome in the document flow), `canvas` (the diagram surface), `overlay` (a bottom-left stack: pan/zoom controls, error popover, legend), `float` (floats over the canvas) and `notify` (transient notifications) — and every piece of chrome belongs to exactly one. The region owns placement and stacking; the chrome inside it declares neither. Two widgets therefore cannot be pinned to one inset by two independent rules, which is what makes overlap unrepresentable rather than merely fixed. Joining a region is appending an element. The float region carries **two placements** ([ADR-0042](./docs/adr/0042-the-selection-seam-and-occlusion-aware-centring.md)): its `float-dock` slot owns the edge and inset for chrome that hugs a stage edge (the [details panel](#details-panel)), while chrome anchored to an entity's screen box is appended to the layer itself and positions against that box.

### Query transport

How a query gets from the preview to the engine ([ADR-0034](./docs/adr/0034-query-transport-and-result-node-identity.md)). The answer is **a local function call, not a request**: the engine runs *inside* the webview, `kul-lsp` gains no `kul/query` method, and no query crosses the webview→extension→LSP boundary. That is what makes a zero-click hover lens viable. Its two costs are recorded rather than hidden — the webview CSP gains `'wasm-unsafe-eval'` in `script-src` plus a `connect-src` allowance (never the broad `'unsafe-eval'`), and every call re-checks the project because the WASM surface is stateless. Implemented in `packages/preview/src/engine.ts`.

### Engine source

The pair of URIs — the `wasm-pack --target web` glue module and its `.wasm` — that tells the webview where the host put the [query transport](#query-transport)'s engine ([ADR-0040](./docs/adr/0040-engine-provenance-a-build-asset-not-a-registry-dependency.md)). The engine is a **build asset, not an npm dependency**: `just wasm` produces `crates/kul-wasm/pkg-web/`, the extension stages it into `media/preview/wasm/` at package time, and the HTML shell stamps the two resulting URIs onto the mount point. `@kullang/preview` therefore receives a URL rather than importing a host-specific module, and its wire types are mirrored by hand from the committed `crates/kul-wasm/types/kul_wasm.d.ts` under a text conformance test. The module is fetched on the **first query** and never at preview-open.

### Project snapshot

The `{files, manifest}` value the host posts with every render so the webview has something to query. Because every WASM query operation is stateless — it takes `files + manifest` and re-checks — the source the webview queries has to *travel with the picture it came from* ([ADR-0034](./docs/adr/0034-query-transport-and-result-node-identity.md)). The VSCode host collects it per render from the project directory (flat, `*.kul` only, lexicographic — the same discovery rule as [`kul-loader`](#kulfile)), preferring an open editor buffer over the file on disk so the queried text is the rendered text. Collected in `editor/vscode/src/project-snapshot.ts`.

### Result binding

Which nodes in the rendered SVG an engine answer paints ([ADR-0034](./docs/adr/0034-query-transport-and-result-node-identity.md)). A result is about a **person, not a card**: `data-person-id` selects the canonical card *and every ghost*, and they light together, so an answer is never invisible in the past family whose edges its [ghost slot](#ghost-slot) anchors. Edges bind from what the [relationship descriptor](#relationship-descriptor) already carries — an `across` hop names its marriage (`data-link-kind="marriage"` plus `data-marriage-id`), a vertical hop names the child it lands on (birth *and* adoption edges with that `data-child-id`, because [path identity](#path-identity) does not collapse two ties). **Ghost cards stay individually unaddressable**; no decided interaction distinguishes one ghost from another. Implemented in `packages/preview/src/result-binding.ts`.

### Entity selection

The one entity the reader has clicked in the preview ([ADR-0035](./docs/adr/0035-detail-surface-one-selection-one-batched-lookup.md), shaped by [ADR-0042](./docs/adr/0042-the-selection-seam-and-occlusion-aware-centring.md)). #276's "single selection, always" is unchanged in *number* and widened from persons to **entities**: a person card, a marriage edge or an adoption edge. Clicking another entity moves it; Esc or a canvas click clears it. A selection is spelled as a [detail target](#batched-detail-lookup) — a person id, a marriage id, or an adoption's `(childId, marriageId)` pair — so what was clicked reaches the engine without translation and the chrome carries one address vocabulary rather than two. It paints in the reserved query violet, and it registers a [sync suspension](#sync-suspension) so the two meanings of "highlighted" never co-paint. Implemented in `packages/preview/src/selection.ts`.

### Sync suspension

Editor⇄preview highlight sync held down while the reader is querying (#276 point 9). It is a **set of reasons**, not a test of the selection: an [entity selection](#entity-selection) registers one, an active [attribute filter](#attribute-filter) registers another, and sync resumes when the last one lifts — with a hint in the [notify region](#region-model) for as long as any holds. **Esc lifts whatever is holding it**, selection and filter alike, because the hint names Esc and a reason with no key behind it is a promise the chrome does not keep. Entering suspension **strips** any sync highlight already painted, not merely refuses the next one, because the editor cursor is usually on some entity when the reader clicks; the last inbound highlight is *held* rather than dropped, and replayed on resume so Esc does not restore sync to a blank tree.

### Selection seam

The interface every piece of query chrome attaches to: a store exposing `current`, `select`, `clear` and `subscribe`, plus the [person anchor](#person-anchor) accessor ([ADR-0042](./docs/adr/0042-the-selection-seam-and-occlusion-aware-centring.md)). Both mutators are idempotent — re-selecting the selected entity notifies nobody — so every notification is a real transition and a listener may refetch unconditionally. `createQuerySurface`'s own subscription drives the details panel and the [Explore-kin list](#explore-kin-list); the [hover lens](#hover-lens) subscribes to the seam directly, and none of them reaches past it to the DOM. The sync-suspension hint is *not* a subscriber — it follows the [sync suspension](#sync-suspension) reason set, of which the selection is one reason. `createQuerySurface` composes the seam with the paint, the panel, the kin list, the lens and the suspension, and exposes **`clearSelection()`** — deliberately narrow, because clearing one surface is not the same as ending query mode. The slice that ends query mode on an edit composes it from that call plus the surfaces its siblings own.

### Person anchor

The person an [entity selection](#entity-selection) is anchored on, or nothing when an edge is selected. The hover lens needs a person ego and a [kin-set query](#kin-set-query) needs a person anchor, so a person selection gets the full model while an edge selection gets a panel only. An edge is a **waypoint, not a dead end**: the marriage panel's spouse rows and the adoption panel's child row move the selection to a person. There is no second selection and no selection history — the waypoint is the panel calling `select`, not the store remembering where it came from.

### Details panel

The surface an [entity selection](#entity-selection) populates: three variants — person, marriage, adoption — each fed **only** by the [batched detail lookup](#batched-detail-lookup) ([ADR-0035](./docs/adr/0035-detail-surface-one-selection-one-batched-lookup.md)). Absent fields are **omitted**, never rendered as "not recorded", mirroring the wire where both export shapes use `skip_serializing_if`; each marriage occupies **its own line with its span and end reason**, which is what carries the explanation for why a person appears on more than one card; and there is **no ghost note**, because sourcing one would re-derive [ADR-0019](./docs/adr/0019-ghost-model-and-bio-anchor.md) inside the query layer or open a second provenance path. Every person named in it is clickable. It floats over the canvas in the [float region](#region-model) and never reflows it, and its header carries the reveal-in-editor control that click-to-source on the tree gave up. Implemented in `packages/preview/src/detail-panel.ts`.

### Hover lens

Relationship resolution as a **zero-click affordance**: while a person is selected, hovering any other card resolves the tie live and whispers it in a pill docked under the *target* ([#276](https://github.com/YashBhalodi/kul/issues/276), built per [ADR-0044](./docs/adr/0044-the-hover-lens-transliteration-as-data-and-the-unspent-hue.md)). No click, no mode, no second selection state — it is the interaction the epic's architecture was arranged around, and the reason the engine runs in the webview and phrasing is local. **Position disambiguates direction**, so the pill docks to the target rather than floating free — it is a [docked tag](#docked-tag), and carries only terms plus a violet dot meaning "as the selected person sees it". Multi-tie **stacks inline** (*māmā · sasro*): the engine returns every way two people are related and the surface shows them all, never a primary one. A [resolution path](#resolution-path) traces why. Emptiness stays two answers — *not related (no connection)* for `disconnected`, *not related (within bounds)* for `noneWithinBounds` — because the [emptiness reason](#emptiness-reason) is the product. Firing on pointer movement, it uses a **trailing-edge debounce, never a spinner**: one query per settle, not one per pixel. While a pill is up the persons it names are published to the [dim registry](#dim-registry) as an exemption, so a reading always outranks the paint the reader left behind. Lives at `packages/preview/src/hover-lens.ts`; per [ADR-0036](./docs/adr/0036-two-tier-theming-and-the-accessibility-non-goal.md) it carries no `role`, `aria-*` or `:focus-visible`.

### Docked tag

The whisper grammar the query chrome shares: a small tag positioned against one entity's **screen box**
on the float layer's entity-anchored placement ([ADR-0042](./docs/adr/0042-the-selection-seam-and-occlusion-aware-centring.md), split out by [ADR-0044](./docs/adr/0044-the-hover-lens-transliteration-as-data-and-the-unspent-hue.md)). It is **mechanism only** — the box, the measurement, the horizontal viewport clamp, the pointer posture — and decides nothing about *when* a whisper appears or *what* it says; its content is a list of nodes the consumer builds. Position is load-bearing: docking **under** the anchor is what says which entity the whisper is about, so only the horizontal axis is clamped and a tag never drifts off its card to stay visible. The [hover lens](#hover-lens)'s relation pill is its first consumer and the [can't-say reason](#cant-say-reason) its second — which is why it is a grammar and not one widget's chrome. `openDockedTag` in `packages/preview/src/docked-tag.ts`.

### Resolution path

The trace the [hover lens](#hover-lens) paints across the tree while it reads: every edge the relationship's [path backbone](#path-backbone) ran through — an [affinal hop](#affinal-hop)'s marriage, a vertical hop's birth or adoption edge — **plus every person the path passes through**, in the reserved dashed sky. It answers *why* the term is the term. Neither endpoint's **card** is painted: the ego already wears the selection violet and the alter is the card under the pointer. An endpoint's own birth edge can still light, where a leading `up` hop was drawn as it. Direction is load-bearing in the derivation — a `down` hop lands on the child, an `up` hop leaves from one — so the walk carries the person it came from rather than reading each hop alone. For a multi-tie the paint is the union over every relationship returned.

### Occlusion-aware centring

Panning a card to the centre of the **visible** region rather than of the raw viewport ([ADR-0036](./docs/adr/0036-two-tier-theming-and-the-accessibility-non-goal.md), computed per [ADR-0042](./docs/adr/0042-the-selection-seam-and-occlusion-aware-centring.md)). The [details panel](#details-panel) overlays the canvas, so a panel-driven walk would otherwise park a card behind the panel — and a selection the reader cannot see is a dead end. The panel's box is measured and the wider free strip wins, so nothing assumes which edge it hugs; only the horizontal axis moves, because the panel is a full-height side float.

### Explore-kin list

The kin-set surface in the preview: **one flat scrollable list of every kin set with its count**, no category grouping, opened from the [details panel](#details-panel)'s person variant (#276's resolution, built in [ADR-0043](./docs/adr/0043-explore-kin-thirteen-query-values-and-what-a-row-costs.md)). Its rows are the thirteen kin sets the engine's named sugar defines, each spelled as that sugar's [Query value](#query-value) — the preview constructs the value and owns **no** membership rule. The copy is enforced from both ends: `crates/kul-core/tests/kin_catalogue.rs` proves each row's Query is behaviourally its sugar and snapshots the `KinPattern` the engine builds, and the preview lints its catalogue against that snapshot *and* against the `pub fn *_of` list in `sugar.rs`. A row's *name* is English chrome; its count is the engine's `count` projection, never a client-side length, and a count that has not arrived shows a placeholder rather than a `0`. Clicking a row re-asks the same value projected to `members`, which becomes [kin paint](#kin-paint) on the tree and glosses that row with its members' phrased terms — the descriptors ride along in the answer, so the gloss costs no lookup and re-phrases with the [locale toggle](#locale-toggle). **Only the painted row is glossed**: an unasked set has no term to show. The open/closed state belongs to the reader and survives the selection walking; the painted answer belongs to one anchor and does not, and closing the list or re-clicking the painted row lets go of it without giving up the selection. The count sweep is guarded on the **anchor** and the row paint on its own counter, so painting a row inside the sweep's latency — a designed path, since rows are clickable before counts land — loses neither answer; and the sweep's per-anchor memo is a claim on work in flight, released whenever the sweep does not fully answer, so a project that was failing its checks when the list opened does not leave the list dead after the fix. An edge selection gets no list at all — a kin set needs a [person anchor](#person-anchor). Implemented in `packages/preview/src/kin-sets.ts` and `kin-list.ts`.

### Kin paint

An answered [kin-set query](#kin-set-query), drawn on the tree — **the tree is the result, and there is no results list anywhere** ([ADR-0043](./docs/adr/0043-explore-kin-thirteen-query-values-and-what-a-row-costs.md)). Matched persons take the reserved query teal on **every card they own** via the [result binding](#result-binding), canonical and ghost together, with a matched ghost keeping the glow dashed because it is still a past record. Non-matching **cards** dim through the [dim registry](#dim-registry); **edges never dim**, because they are how a reader sees why a set has the shape it has. The anchor is neither lit nor dimmed — it wears the selection violet, and the paint enforces that itself rather than trusting the engine's self-exclusion. The paint is stateless and re-applies from the one post-render hook, and it registers no [sync suspension](#sync-suspension) of its own: it cannot exist without a selection, which already registers one. An empty answer paints nothing and says so in one quiet, non-modal toast in the [notify region](#region-model) — an absence the engine reported is an answer, not an error. Implemented in `packages/preview/src/kin-paint.ts`.

### Dim registry

The single owner of the preview's dim class ([ADR-0043](./docs/adr/0043-explore-kin-thirteen-query-values-and-what-a-row-costs.md)). More than one paint recedes the non-answer — [kin paint](#kin-paint) and the [filter paint](#filter-paint) — and they publish **sets of person ids** rather than touching the class: a person is dimmed iff **any** active source dims them, the shared `--kul-dim-alpha` is applied once per card and never nests, and a source withdraws by publishing nothing. Two failure modes it makes unrepresentable: a paint stripping the class on repaint and silently un-dimming another source, and two classes whose opacities multiply to a third value neither asked for. One thing outranks the union — an **exemption**, the persons a source lifts out of *every* source's dim. Exemptions are **keyed and unioned** exactly as dims are, because two holders exist: the [hover lens](#hover-lens)'s live trace (the lens is what the reader is doing now while the dim is what they did a moment ago, so the lens wins) and the [filter paint](#filter-paint)'s standing can't-say set (an unjudgeable person must never recede, whatever else is painted). [ADR-0043](./docs/adr/0043-explore-kin-thirteen-query-values-and-what-a-row-costs.md) held one slot and named keying as the fix if a second holder appeared; [ADR-0045](./docs/adr/0045-the-filter-bars-three-states-derived-reasons-and-one-class-per-painter.md) is that moment. Implemented in `packages/preview/src/dim.ts`.

### Filter bar

The [attribute filter](#attribute-filter)'s surface in the preview: a **chip sentence** in the [flow region](#region-model) — *Showing ⟨Everyone 10 ▾⟩ where ⟨family = Rossi ✕⟩ and ⟨born < 1950 ✕⟩ ⟨+⟩ ⟨certain ▾⟩* — plus the [tally](#filter-tally) it answers with ([#277](https://github.com/YashBhalodi/kul/issues/277), built per [ADR-0045](./docs/adr/0045-the-filter-bars-three-states-derived-reasons-and-one-class-per-painter.md)). Home is **stage chrome, not the [details panel](#details-panel)**: a filter is project-scoped and the panel is selection-scoped, so the bar is live with nothing selected and the panel grows no filter affordances. The `where` and `and` keywords are **literal text**, so conjunction-only is visible in the grammar rather than documented elsewhere; there is no OR chip, no OR affordance and no advanced mode, because OR is permanently out of engine scope. Ops are field-aware (ordering only on `born` / `died`, `∈` only on free text), `id` is not offered at all — it is a person's address, not something an author wrote about them — and [certainty](#certainty-mode) is a chip *inside* the sentence rather than a display toggle, because the third truth value is part of the query. Each chip opens a field / op / value editor built from real `<select>` and `<input>` elements, which is what makes `mountKeyboardPan`'s target guard bind. Every change asks the engine **both** certainty questions, because the difference between the two answers is the unjudgeable set. An active filter registers its own [sync suspension](#sync-suspension) reason, independent of any [entity selection](#entity-selection), and a chip nobody has finished writing asks nothing at all. The sentence and the [Query value](#query-value) it spells live in `packages/preview/src/filter.ts`; the chrome in `filter-bar.ts`.

### Filter paint

The three states a [filter bar](#filter-bar) draws on the tree, and the standing rule they obey: **filtering never hides a node or an edge — it only dims** (#277, generalised past this feature — hiding punches holes in the canonical layout and leaves marriage stubs and birth edges running into empty space). A **match** takes the reserved query teal on every card the person owns, ghosts included ([result binding](#result-binding)); a **can't say** takes the reserved amber — the last hue ADR-0036 reserved — dashed, with a `?` riding each of their cards, and is deliberately *not* dimmed **by anyone**: the set is published to the [dim registry](#dim-registry) as a standing exemption rather than merely left out of this paint's own dim, because a kin set painted underneath would otherwise recede the one state that must never recede; a **non-match** takes the shared dim alpha and no colour of its own. The match glow is its own class rather than [kin paint](#kin-paint)'s even though both alias the same teal: the two are on screen together whenever a filter runs inside a painted kin set, and two owners of one class is the failure the [dim registry](#dim-registry) exists to prevent — the dim is shared because it is genuinely union-semantic and a match is not. The paint is stateless and re-applies from the one post-render hook, where the filter **re-asks** rather than replaying, because its dimmed set is derived from the cards the picture holds. Implemented in `packages/preview/src/filter-paint.ts`.

### Filter scope

The population a [filter bar](#filter-bar) runs over, and the [Query](#query-value) `source` that names it. Two exist: **Everyone** (`allPersons`, whose population is every person the picture draws) and a **kin scope** — the [kin set](#kin-paint) currently painted, `kinOf(anchor, pattern)` — which is what makes "which of Giuseppe's descendants were born before 1950" one question rather than two. Switching the scope chip switches the `Query`'s source, so `kinOf` + `where` is evaluated as **one query** and the surface never intersects two answers. The population is *supplied* — the picture's card ids, or the members the painted answer is already holding — rather than queried, which keeps a filter change at two engine calls and stops the [tally](#filter-tally)'s denominator disagreeing with what is painted ([ADR-0045](./docs/adr/0045-the-filter-bars-three-states-derived-reasons-and-one-class-per-painter.md)).

### Filter tally

The [filter bar](#filter-bar)'s answer in words: *"3 of 10 · 6 dimmed · 1 can't say"*, or *"3 of 4 in Giuseppe's siblings · 1 dimmed"* under a kin [scope](#filter-scope). Its three groups **partition** the scope — matched + can't say + dimmed = total — in either [certainty mode](#certainty-mode); the mode moves only which of them count as *shown*, never a count, so the disclosure certain mode owes the reader is the same number whichever way they are looking. Unjudgeable people are counted and nameable, **never silently absent** (PRD-0006 story 25). One `partitionScope` computes the split the tally counts and the paint draws, so a number and a card cannot disagree about who is amber.

### Can't-say reason

The whisper under an unjudgeable card, in the [docked tag](#docked-tag) grammar: *"can't say — family not recorded"*, *"~1942 is approximate (±5y) — straddles born < 1945"*. The engine answers in sets and never says *why*, so the reason is **derived from the record** — read through the [batched detail lookup](#batched-detail-lookup), the one provenance path for person data — and it **never re-decides a truth value**: the engine has already ruled the person unjudgeable, and this only names what about the record is unjudgeable-shaped (a field the author never wrote, or an interval wider than a day on either side of the comparison). Presence predicates are excluded by construction, being decidable on any record — which is also why the surface says, where they are offered, that `died not recorded` is a fact about the record and not about the person. It is offered **only while no person is selected**: a person selection arms the [hover lens](#hover-lens), whose pill docks under the same card, and one gesture gets one answer. Defined in `packages/preview/src/filter-reason.ts`; the decision and its rejected alternatives are [ADR-0045](./docs/adr/0045-the-filter-bars-three-states-derived-reasons-and-one-class-per-painter.md).

## When this glossary is incomplete

If you're naming a concept that isn't here:

- **Common case** — you're inventing language the project doesn't use. Find the canonical term and use it.
- **Real gap** — the concept genuinely doesn't have a name yet. Add it here in the same change, with a one-paragraph definition. If the concept is load-bearing enough that future agents will need to understand *why* it exists, also write an ADR.

Architecture vocabulary (module / interface / seam / depth / adapter / leverage / locality / deletion test) is intentionally not duplicated here — see [`docs/architecture.md`](./docs/architecture.md).

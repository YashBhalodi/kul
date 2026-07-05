# PRD 0005: Kinship Query Engine

> Product epic: [#253](https://github.com/YashBhalodi/kul/issues/253). Child issues are linked from the epic. This PRD is transient — delete it in the same PR as the final piece of implementation work (see [README](./README.md)).


## Problem Statement

A person who consumes a Kul project someone else authored wants to ask kinship questions of it and get correct answers. Two shapes of question dominate:

1. **"Who is this person's … ?"** — mother, father, son, daughter, siblings, grandparents, great-grandparents, cousins, and so on to arbitrary depth.
2. **"What is the relationship between these two people?"** — are they father-and-son, siblings, cousins, sister-in-law, and how distant (first cousin, second cousin once removed, …).

Plus a modest amount of **"which people match this attribute?"** — e.g. everyone born after 1950 with no recorded death date, a person's children ordered by birth year, descendants who married into a particular family.

Today none of this is first-class. `ResolvedDocument` computes only the primitive one-hop derivations (`parents_of`, `spouses_of`); there is no `children_of`, no `siblings`, no `ancestors`, no `descendants`, no cousin or in-law computation, and no way to name the relationship between two people. Every consumer that needs those either re-invents the graph traversal itself or gives up. Re-invented traversal is where correctness and staleness bugs breed — and kinship traversal over Kul's data model is genuinely easy to get wrong (marriage is a node not an edge, so every English "hop" is two graph hops; the parent set has cardinality 0..4+ once adoption and polygamy enter; the full kinship graph is not acyclic).

The governing constraint: **the correctness of kinship querying must live in the core offering, computed once, not re-implemented by every downstream consumer.** Consumers should own only the *UX* of querying, never the querying itself.

## Solution

Ship a **kinship query engine** as a core capability of the toolchain: a library that answers kinship and basic-attribute questions about a checked Kul project, exposed so that downstream applications compose reader- and author-facing experiences on top of it without re-deriving any kinship logic.

The engine is a **library capability, not an end-user query language.** There is no query DSL, no grammar, no parser. The customer is the **application developer** building reader/researcher experiences; end readers and authors are served *through* those apps (the way real genealogy products deliver — nobody's grandmother writes a query). This keeps the surface additive: an API can always grow a DSL later; shipping a DSL first and discovering the API beneath it is wrong is expensive and hard to reverse.

The engine provides three composable capabilities over one anchor concept — a checked project's `ResolvedDocument`:

- **Kin-set queries** — from one anchor person + a relation, return the *set* of related persons. The relation vocabulary is the descriptor's own classification (see below): a declarative **descriptor pattern** with named conveniences (`children_of`, `siblings_of`, `cousins_of(degree, removed)`, …) as thin sugar. Input `(anchor, relation pattern)`, output a set of `(person id, descriptor)` pairs.
- **Relationship resolution** — from two persons, return the *set* of all the ways they are related, each as a **terminology-neutral relationship descriptor** (a structured record: classification, edge nature, affinity, sharing, side, two seniorities, endpoint genders, and a lossless path backbone). The engine never emits a human word ("sister-in-law"); it emits the descriptor, and the *app* renders the word.
- **Attribute filtering** — filter/sort/count persons and marriages by their fields, composable onto the output of a kin-set query in a single request.

Supporting these, the core also exposes **id → detail lookups** (`person(id)`, `marriage(id)`) returning the same serialized shapes the export uses, so a query consumer hydrates results on demand and never needs the full export just to display a person.

All query capabilities return **sets** — a uniform contract. The composition `traverse → filter → resolve` is the product.

A deliberately-thin future layer, **out of scope here**, sits above the descriptor: a community-growable, additive map from `RelationshipDescriptor → culture-specific term` ("sister-in-law" | "bhabhi" | …). Because the descriptor is *maximally discriminating*, that layer can grow as pure data without ever changing the engine.

## User Stories

### Application developer (primary actor — the customer)

1. As an app developer, I want a single core call that returns a person's parents (biological and adoptive, each tagged), so that I never re-implement parent-set derivation and can't get it subtly wrong.
2. As an app developer, I want a person's children returned as a set, so that I can render a family view without walking parenthood links myself.
3. As an app developer, I want a person's siblings, with each tagged full or half, so that I can present them correctly without deciding "what is a half-sibling" on my own.
4. As an app developer, I want a person's spouses (across all their marriages, past and current), so that I can render their unions.
5. As an app developer, I want ancestors to a given generational depth (grandparents, great-grandparents, …), so that I can build an ancestry view of any height.
6. As an app developer, I want descendants of a person, so that I can build a lineage/"all descendants" view.
7. As an app developer, I want cousins of a person, with cousin degree and "removed" count available, so that I can label them precisely.
8. As an app developer, I want aunts/uncles and nieces/nephews (collateral relations), so that I can render an extended-family view.
9. As an app developer, I want in-law relations (relatives by marriage), so that I can show a person's affinal family.
10. As an app developer, I want every kin-set query to return a *set* with each member carrying how it was reached (its full relationship descriptor), so that I can filter to "blood only" or "including adoptive" in my UX without another core call.
11. As an app developer, I want to ask "what is the relationship between X and Y?" and receive *all* the ways they are related, so that I can decide in my UX which to surface — the core never hides a true relationship or picks a "primary" one for me.
12. As an app developer, I want each relationship returned as a structured, terminology-neutral descriptor, so that I can render whatever word or locale my product needs.
13. As an app developer building a Hindi-facing product, I want the descriptor to distinguish maternal vs paternal side and elder vs younger seniority, so that I can render *mama* vs *chacha* and *bhabhi* correctly — distinctions English discards.
14. As an app developer, I want the descriptor to carry a lossless canonical relationship path, so that if I need a distinction the normalized fields don't capture, I can still derive it without asking for an engine change.
15. As an app developer, I want to filter a set of persons by field predicates (born after 1950, no recorded death date, family = "Sharma"), so that I can build "gap-finding" and "cohort" views.
16. As an app developer, I want to sort a result set by a field (e.g. children by birth year), so that I can present ordered lists.
17. As an app developer, I want to count a result set, so that I can show "N children", "N descendants".
18. As an app developer, I want to apply a filter *to the output of a traversal* ("descendants of X who married into family Y"), so that I can express compound reader questions as one pipeline.
19. As an app developer consuming the WASM package, I want the engine surfaced there with JSON/TypeScript-typed results, so that my web app calls it directly with no bridging code.
20. As an app developer, I want the descriptor and result types shipped as committed TypeScript types, so that my build breaks loudly if the contract changes rather than failing silently at runtime.
21. As an app developer, I want the engine to run against the same checked project my renderer already uses, so that query answers and rendered trees can never disagree.
22. As an app developer, I want a core `person(id)` lookup returning that person's immediate details on demand, so that I can hydrate query results without exporting the entire project.
23. As an app developer, I want a core `marriage(id)` lookup, so that I can resolve the marriage references that path backbones carry.
24. As an app developer, I want double cousins (related the same way via two distinct paths) returned as two descriptors differing in side and path, so that consanguinity-aware products never lose a real tie.
25. As an app developer, I want an empty relationship resolution to tell me whether the two people are disconnected or merely unrelated within the search bounds, so that my UX never renders "not related" when the truth is "not related as far as we looked".
26. As an app developer, I want a typed error (not an empty set) when I pass an unknown or wrong-kind id, so that caller bugs surface instead of masquerading as "no relatives".
27. As an app developer building a Hindi-facing product, I want the descriptor to carry the apex-junction seniority (the uncle's birth order relative to ego's father), so that I can render *chacha* vs *tau* — a distinction ego-relative seniority cannot express.

### End reader / researcher (served through apps — secondary actor)

28. As a reader, I want to click a person and see "mother, father, siblings, children, spouse", so that I can navigate a family I didn't author.
29. As a reader, I want to pick two people and be told how they are related ("second cousins once removed"), so that I understand a connection I couldn't work out by eye.
30. As a genealogy researcher, I want to see all of a person's ancestors to any depth, so that I can trace a lineage.
31. As a researcher, I want to find everyone born after a year with no recorded death date, so that I can spot records needing follow-up.
32. As a researcher exploring a consanguineous family, I want to see *both* relationships when two people are related two ways (first cousins *and* in-laws), so that the tool doesn't lie by omission.
33. As a researcher, I want an opt-in `includeUncertain` filter mode, so that gap-finding queries surface the fuzzy records a certain-only filter would silently drop.
34. As a reader of a culturally-specific family history, I want the app to show me the kinship term my culture uses, so that the relationship reads naturally — served by the future terminology layer over the descriptor.

### Author (through apps — secondary actor)

35. As an author, I want an app built on the engine to answer "who do I still need to fill in?" (e.g. persons with no recorded parents), so that I catch gaps the validator can't see.
36. As an author, I want relations to be answered as best-effort over whatever I've entered so far, so that querying works mid-authoring on an incomplete project.

### Correctness, partial data, and honesty

37. As a consumer, I want the engine to compute over whatever data is present and never fabricate a relationship it can't justify, so that a partially-populated project yields honest answers, not guesses.
38. As a consumer, I want "elder vs younger" reported as *unknown* when birth dates are missing or too coarse to order (two people both `born:1980`, or twins recorded on the same day), so that the engine never invents a seniority.
39. As a consumer, I want maternal/paternal side always *derived* from the recorded path — with an explicit `other` value when the linking parent's gender is `other` — so that side-dependent terms are never needlessly ambiguous and never guessed.
40. As a consumer, I want adoptive and step relations tagged distinctly from blood relations, never silently merged, so that "who raised this person" and "who they descend from" stay separable.
41. As a consumer, I want full vs half siblings distinguished by actual parent-set sharing, so that polygamous and remarried families read correctly.
42. As a consumer, I want actual full siblings whose parents divorced and remarried each other reported as full, so that structural quirks of the marriage record never demote a blood relationship.
43. As a consumer querying a family with adoption-into-relatives, I want traversal to terminate correctly despite cycles in the full kinship graph, so that "all ancestors" never loops forever.
44. As a consumer, I want relationship resolution bounded to the nearest common ancestor(s) plus a configurable degree cap, so that two distant people don't return a haystack of remote-ancestor relationships.
45. As a consumer, I want relations through ended marriages still returned, with the marriage hop tagged with its status and end reason, so that recorded history is reported and my app decides whether divorce dissolves affinity.

### Surfaces, versioning, scale

46. As a scripting user, I want a `kul query` CLI subcommand, so that I can ask kinship questions from the terminal and in automation.
47. As a scripting user, I want `kul query` human output to stay terminology-neutral (structured facts, never kinship words), so that no accidental culture pack ships in the CLI.
48. As a maintainer, I want the CLI query path to double as a dogfood/snapshot harness, so that the engine's answers are pinned against the example corpus.
49. As a consumer, I want the engine to operate only on projects whose declared language version the toolchain recognizes (already enforced upstream by manifest validation), so that I never get answers computed under a misunderstood language version.
50. As a consumer, I want query performance to be interactive on realistic family histories (up to ~10k persons), so that a UI can call it on every click without a spinner.
51. As a consumer who needs analytics the engine deliberately doesn't do (distributions, group-by, statistics, temporal predicates), I want to fall back to the exported JSON, so that I'm never blocked — I just build that layer myself.

## Implementation Decisions

### Customer & shape
- **The customer is the application developer; the deliverable is a library capability, not an end-user query language.** No DSL, no grammar, no parser. (Reversible-upward: an API can grow a DSL later.)
- **Kinship computation lives in the core offering and is single-sourced.** Consumers own the *UX* of querying only. This rules out "ship SQL/GraphQL over the three exported tables" as the core strategy, because handing consumers raw tables + a generic query engine re-creates the "compute sibling yourself" trap. The core exposes **computed relations as first-class**, not raw tables.

### Modules & seams
- **One new logic seam: a `query` module in `kul-core`, layered over the existing `ResolvedDocument` seam (ADR-0001).** `ResolvedDocument` remains the seam for *primitive* one-hop derivations (`parents_of`, `spouses_of`); the query engine is a **deep module built on top of it**, consuming those primitives plus `persons()` / `marriages()` rather than re-walking the AST. Its public API is the single seam where all capabilities and the result/descriptor types live.
- **The engine's substrate is `ResolvedDocument`, never the `ExportedGraph`.** The export stays the deliberate *escape hatch* for consumers who don't use the engine (analytics, foreign tooling); it is never the engine's own input.
- **Two thin adapter seams: WASM and CLI.** WASM adds a **fourth shape** alongside `check` / `exportGraph` / `format` (extends the three-shape surface of ADR-0011). CLI adds `kul query`. Both are thin adapters over the core seam; neither re-implements kinship logic. Native consumers (mobile, backends) use the Rust crate directly — no dedicated C-ABI/`uniffi` binding.
- **Deferred:** LSP-backed editor query (author-at-edit-time; deprioritized with the author audience). **Ruled out:** a stateful query server.

### One vocabulary for both engines (query surface)
- **The kin-set query surface is a declarative descriptor pattern, not a fixed enum and not raw traversal steps.** A query is "return every person whose relationship to the anchor matches this pattern": a classification with numeric parameters and ranges (lineal ancestor/descendant with generation bounds; collateral with `up`/`down` or `degree`/`removed`), plus optional filters on edge nature, affinity, sharing, and side.
- **Named conveniences ship as thin, documented sugar** (`parents_of`, `children_of`, `siblings_of`, `ancestors_of(depth)`, `descendants_of(depth)`, `cousins_of(degree, removed)`, `in_laws_of`, …), each defined *as* its selector expansion. Parameterized collateral queries ("second cousins once removed") are therefore expressible by construction — no dedicated API.
- **Raw up/down/across step composition stays internal.** Exposing it would re-create the "compute sibling yourself" trap: self-exclusion, cycle guarding, and subsumption rules remain engine-owned and unreachable by consumers.
- Engine A and Engine B thus share one vocabulary: anything relationship resolution can name, a kin-set query can ask for — closure by construction.

### Traversal semantics
- **Traversal walks the full relation graph, tagging every edge, and is cycle-guarded unconditionally.** The biological-parenthood projection is a guaranteed DAG (rule R13), but following adoption or spouse edges forfeits that guarantee (adoption-into-relatives and cousin-marriages create cycles). Consumers who want blood-only filter on descriptor fields; the engine never assumes acyclicity.
- **In-law paths: blood segments joined by marriage hops, at most 2 affinal hops per path, at any position, including consecutive.** One hop at the start (spouse's kin), end (kin's spouse), or middle (child's spouse's parent — *samdhi*) are all valid; two hops admit co-in-laws (*jethani*, *sadhu*) and consecutive hops admit co-spouses (*sautan* — required by the polygamy corpus). No culture lexicalizes three affinal hops; the ceiling is **fixed semantics, not a configurable knob** (see the semantics-vs-budget line below).
- **Ended marriages traverse like any other**, with the marriage hop tagged with the marriage's status (`ongoing`/`ended`, with end reason). The data is historical; whether divorce dissolves affinity is a UX/terminology decision, so the core reports and tags rather than filters.
- **Step relations are position-of-marriage-hop, defined mechanically:** step-parent = spouse of ego's parent via a marriage ego has no birth/adoption link to; step-ancestor (gen N) = spouse of a lineal ancestor via a marriage not on ego's descent line; step-sibling = child of a step-parent sharing no parent with ego; step-child/descendant are the inverses. An explicit adoption edge always wins over a step reading (adoptive, not step).
- **Step subsumption rule:** a step path is a *derived stand-in* for parenthood, not an independent fact — when an actual parent edge or shared parent exists, the step reading is suppressed, not emitted alongside. (This does not contradict path-multiplicity below: double cousins are two independent *true* paths; a step path shadowed by a blood path is one fact derived two ways.)
- **Best-effort over partial data, honest underdetermination over guessing.** Optimized for the common case of a fully-populated project. When data is missing, report a dimension as `unknown` or a result as absent — never fabricate.

### The relationship descriptor (contract shape)
The descriptor is **terminology-neutral and maximally discriminating** — it must carry *every* distinction any future culture's terminology could key on, because the future terminology layer is a pure lookup map and can only discriminate on what the descriptor contains.

- **Descriptor identity is path identity.** The result set contains one descriptor per distinct relationship path, with **no engine-side collapsing** of same-classification descriptors. Double cousins yield two descriptors (differing in side and path backbone); collapsing would hide a true relationship and force the core to pick a winner — both forbidden. Consumers who want "just first cousins" collapse on the normalized fields themselves.
- **Classification** — `self` | lineal `{ ancestor | descendant, generations }` | collateral `{ up, down }`, with `cousin_degree = min(up,down) − 1` and `removed = |up − down|` **materialized in the serialized form** (the formulas are exactly the off-by-one traps apps would fumble).
- **Edge nature** — `blood | adoptive`: strictly about parent-child edges on the path (any adoption hop ⇒ adoptive). Per-hop truth stays lossless in the path backbone.
- **Affinity** — `blood | step | in-law`: strictly about marriage hops — none ⇒ blood; marriage hop in ancestor position ⇒ step; any other marriage hop ⇒ in-law (in-law wins the scalar when both appear; the backbone keeps the full truth). *These two fields replace the earlier flat `full|half|adoptive|step` consanguinity enum, which conflated independent dimensions and could not express half-adoptive or full-adoptive siblings.*
- **Sharing** — `full | half | n/a`: an **apex-junction comparison, not a single-path property**. At the collateral apex, compare the two branch-persons' parent sets *per edge kind*: `full` = identical bio-parent sets (or adopted by the same couple — adoptive-full); `half` = exactly one shared parent; `n/a` = lineal/self/pure-affinal. Parent-set equality (rather than shared-marriage) keeps full siblings *full* even when their parents divorced and remarried each other, while staying honest in mixed bio/adoptive corners.
- **Half via polygamy vs remarriage collapse identically** — both are one shared parent across distinct marriages; the descriptor does not distinguish the mechanism (recoverable via the backbone's marriage references).
- **Side** — `maternal | paternal | other | both | n/a`, derived per path, never guessed: `n/a` when the path never ascends from ego (self, spouse, descendants, direct parents, pure-affinal starts); `both` when the first ascent reaches its apex at a marriage without passing through an individual parent (full siblings and relations routed through them); otherwise the gender of the first parent-person on the ascent — with `other` when that parent's gender is `other` (the grammar permits it, so "always resolved" is amended to "always derived"). Side is strictly about *routing*; endpoint gender is its own field.
- **Seniority — two fields, both riding the strict interval comparison:**
  - `seniority` (endpoint): alter's birth order vs ego — `elder | younger | unknown | n/a` (`n/a` only for self). Needed by e.g. Chinese cousin terms.
  - `apex_seniority` (junction): birth order of the alter-branch sibling vs the ego-branch sibling at the collateral apex — `n/a` for lineal/self and paths with no sibling junction. Needed by *chacha/tau*, *jeth/devar*, *jethani/devrani*; a single ego-relative field cannot express these.
  - Decidability reuses the toolchain's one date-comparison rule (`before_strict`): `elder`/`younger` only when every interpretation of one date precedes every interpretation of the other; overlapping or missing intervals (including twins recorded on the same day) ⇒ `unknown`. Identical semantics to the validator's temporal rules.
- **Endpoint genders** — ego and alter gender at the descriptor top level; linking relatives' genders ride the path backbone.
- **Path backbone** — the ordered hop sequence from ego to alter: direction (`up`/`down`/`across`), edge tag (`bio`/`adoptive` for vertical hops; marriage id + status + end reason for `across`), the person id landed on, and that person's gender. **Lossless ground truth**, so a future distinction nobody anticipated is still recoverable without an engine change.

The committed TypeScript shape (from this design pass; it encodes the decisions above more precisely than prose):

```ts
type Gender = "male" | "female" | "other";

type RelationshipDescriptor = {
  egoId: string;
  alterId: string;
  egoGender: Gender;
  alterGender: Gender;
  classification:
    | { kind: "self" }
    | { kind: "lineal"; role: "ancestor" | "descendant"; generations: number }
    | { kind: "collateral"; up: number; down: number; cousinDegree: number; removed: number };
  edgeNature: "blood" | "adoptive";
  affinity: "blood" | "step" | "inLaw";
  sharing: "full" | "half" | "notApplicable";
  side: "maternal" | "paternal" | "other" | "both" | "notApplicable";
  seniority: "elder" | "younger" | "unknown" | "notApplicable";
  apexSeniority: "elder" | "younger" | "unknown" | "notApplicable";
  path: PathHop[];
};

type PathHop =
  | { step: "up" | "down"; to: string; gender: Gender; edge: "bio" | "adoptive" }
  | { step: "across"; to: string; gender: Gender; marriage: string;
      status: "ongoing" | "ended"; endReason?: string };
```

### Result shapes & lookups
- **Rust core:** each kin-set member is a borrowed person reference + owned descriptor (matching the existing `parents_of` idiom). Native consumers get full field access immediately.
- **WASM boundary:** each member is **person id + descriptor — no person payload.** The person's canonical serialization is the export shape; duplicating it in query results would create a second shape to keep in sync forever.
- **Id → detail lookups:** the core (and WASM) expose `person(id)` and `marriage(id)` returning the same serialized shapes the export uses, delivered singly. Query consumers hydrate on demand and never need the full export; the full export remains available for bulk/analytics use.
- **CLI** hydrates for humans (id, display name, relationship facts) — presentation owned by the adapter, not a third contract shape.
- **Serialization conventions (per the tsify discipline of ADR-0012):** camelCase; classification as an **internally-tagged union** (so TypeScript consumers get a discriminated union to `switch` on); **`unknown` and `notApplicable` are explicit enum values, never `null` or absent** (the distinction is load-bearing and JS collapses absent/null too easily); derived numbers materialized; path hops carry ids, not payloads.

### Errors & underdetermination
- **Relationship resolution returns a result object, not a bare set:** the descriptor list, plus — only when the list is empty — an explicit reason: `disconnected` (different connected components; raising the cap can never help) or `noneWithinBounds` (same component, nothing derivable under the rules and caps). Collapsing both to an empty list would invite apps to render "not related" when the truth is "not related as far as we looked".
- **Kin-set queries return a plain (possibly empty) set with no wrapper** — "no cousins" is a complete, honest answer.
- **Field-level:** `unknown` = data insufficient to decide; `notApplicable` = dimension doesn't apply to this path shape. Never conflated.
- **Anchor identity is person id, nothing else** (rule R01 guarantees uniqueness). Name lookup/search is app UX. An unknown id — or an id naming a marriage where a person is required — is a **typed caller error, never an empty result**: Rust queries return a typed `UnknownPerson`-class error; the `person(id)`/`marriage(id)` *lookups* return an Option/absent (for a lookup, absence is the answer); WASM wraps results in a `QueryEnvelope` with an error variant, mirroring the existing check/render envelopes (no thrown exceptions); CLI prints a diagnostic naming the bad id and exits nonzero.

### Numbers & configuration
- **Relationship resolution takes an optional `maxApexGenerations`, default 5** (bounds ascent to the common ancestor on both sides — through fourth cousins). Every lexicalized kinship term in any culture pack runs out by third cousins, so the default is a strict superset of anything the terminology layer could render, with headroom, while cutting off the remote-ancestor haystack.
- **Pure lineal detection runs unbounded.** The haystack is a collateral phenomenon; a parent-chain check is cheap, and `noneWithinBounds` must never hide a recorded direct-line tie.
- **The semantics-vs-budget line:** the 2-affinal-hop ceiling is *semantics* (defines what counts as a relationship — fixed engine rule); the generation cap is a *search budget* (caller knob). Two apps may look differently far, but may never disagree about what relationships exist.

### Attribute filtering (permanently modest)
- **Operators:** `=` / `≠` (exact, case-sensitive — fuzzy/locale matching is app UX), `present` / `absent`, set-membership, and `<` / `>` / `≤` / `≥` on dates. **No substring, no regex** (`family` is a first-class person field, so cohort queries need none of it). No group-by, no distributions, no statistics, no date arithmetic — ever. Those live on the exported JSON, in consumer code.
- **Date predicates are three-valued under the toolchain's one comparison rule:** evaluated against the value's closed interval (partial = whole period, circa = ±5 years): true iff it holds under *every* interpretation, false iff under *none*, otherwise unknown. Same "every interpretation" semantics as the validator's temporal rules and the seniority fields. This stays *field comparison* — predicates never combine fields (no age, no "alive in 1985").
- **Certainty mode:** default `certain` (unknown ⇒ excluded — never asserts what the data doesn't); opt-in `includeUncertain` (unknown ⇒ included — for gap-finding researchers hunting fuzzy records).
- **Composition is conjunction-only.** OR = run two queries and union the sets.
- **Sort is fully deterministic:** dates by (lower bound, upper bound), strings by codepoint, missing values last, ties broken by id — so snapshots stay stable. **Count** is the set's size.

### The Query value & composition surface
- **The contract artifact is a declarative, serializable Query value** evaluated by one core entry point: a source — `allPersons` or `kinOf(anchor, pattern)` — plus optional filter predicates, sort, certainty mode, and a projection (`members | count`). The WASM and CLI adapters force this representation into existence anyway; making it *the* contract gives one evaluation path for all three surfaces and keeps adapters thin wiring.
- **Relationship resolution stays a separate two-anchor call** (`resolve(x, y, config)`) — a different question, not a pipeline stage.
- **Rust-native ergonomics (builder vs free functions vs both) are deferred to the implementing change**, to be resolved with the `design-an-interface` skill, under one pinned constraint: **any sugar must desugar to the Query value; no second evaluation path.**

### `kul query` CLI surface
- One sub-subcommand per core capability, mapping 1:1 onto the Query value (no CLI-only semantics): `kul query kin <anchor-id> <relation> [params]` (named-sugar relations: `siblings`, `ancestors --depth 3`, `cousins --degree 2 --removed 1`, …), `kul query rel <id-x> <id-y> [--max-generations N]`, `kul query persons` (pure filter queries), `kul query person <id>` / `kul query marriage <id>` (detail lookups).
- Shared flags map 1:1 to the contract: repeatable `--where <field><op><value>` (conjunction), `--sort <field>`, `--count`, `--include-uncertain`. This is arg parsing over the Query value, not a query DSL.
- `--format human|json` following the existing CLI pattern; `json` emits the same envelope the WASM surface returns, so the snapshot harness pins the *contract* serialization.
- **Human output stays terminology-neutral:** names/ids plus the descriptor's structured facts — never kinship words. Rendering "sister-in-law" would make the CLI the first culture pack, shipped by accident.
- Same load-and-check gate as `kul export` (queries run only against a checked project, per ADR-0009 strictness). Exit 0 for empty results (an empty set is an answer); nonzero for bad ids, unparseable args, or a project that fails checks.

### Contract, versioning, and language coupling
- **The query engine's relation semantics and descriptor/result shapes are a toolchain/library contract, NOT normative language spec.** `spec/` remains the descriptive elaboration of the *language a human authors*; querying is a *tool built on top of Kul projects*, not part of the language. Semantics and shapes are documented in `docs/` + rustdoc + the committed TypeScript types, and versioned with the toolchain release (already distinct from the language version).
- **Pure lockstep with the language version; the engine does no version-gating of its own.** It consumes an already-checked project, so language-version compatibility is already enforced upstream by the manifest validator (strict-on-errors per ADR-0009 means an unrecognized-version project never yields a graph to query). When the language evolves, the query engine is updated in the same coordinated workspace release as the parser, validator, and export — mirroring the existing manifest-schema-in-lockstep decision (spec §14).
- **New domain vocabulary** (relationship descriptor, kin-set query, relationship resolution, descriptor pattern, path backbone, apex junction, …) is added to `CONTEXT.md` in the implementing change; the load-bearing decisions here (engine-is-core-not-consumer, descriptor-maximally-discriminating, path-identity, set-shaped output, nearest-common-ancestor bounding with unbounded lineal, full-graph-cycle-guarded traversal, semantics-vs-budget line, Query-value-as-contract) land as **ADRs**.

### Performance
- Runs **on-demand over `ResolvedDocument`**, reusing the resolver's in-memory id index. **No dedicated query indices or result caching.** Target: interactive (< ~50 ms/query) on projects up to **~10k persons**. Given the no-mega-import boundary (below), precomputed indices are unlikely ever to be needed.

## Testing Decisions

- **A good test asserts external behavior — the engine's *answers* — not its traversal internals.** Tests state a project and the expected set/descriptor result; they say nothing about how the graph was walked.
- **All kinship correctness is proven once, at the core `query` seam, via snapshot tests (`insta`, ADR-0003).** The primary corpus is `examples/`, which already covers the topologies the engine must survive: nuclear, three-generation, divorce-and-remarriage, adoption (including `gender:other` and a person carrying both a `birth` and multiple `adoption`s), cousins, polygamy, disconnected lineages, multi-file projects, and cross-century lineage.
- **Targeted fixtures cover the structural hazards this design pinned:**
  - unknown seniority: two `born:1980` siblings, twins recorded on the same day, `~` circa overlaps → `unknown`, and the decidable variants (`1980` vs `1981`, `~1980` vs `1986`);
  - apex seniority: an uncle younger and elder than ego's father (*chacha*/*tau* fixture);
  - adoption-into-relatives: cycle guard terminates; two descriptors (adoptive parent + blood aunt);
  - cousin-marriage: resolution returns *both* cousin and in-law descriptors;
  - double cousins: two `{2,2}` descriptors differing in side and path;
  - full vs half via parent-set sharing: distinct marriages, polygamous co-wives' children, same-couple divorce-and-remarry (→ full), half-adoptive and full-adoptive siblings;
  - step subsumption: step reading suppressed by a shared parent; adoption edge beating the step reading;
  - in-law shapes: affinal hop at start/middle/end, two hops (co-in-law), consecutive hops (co-spouse), ended-marriage tagging;
  - a person with an empty parent set and a person with a 0..4+ parent set via birth + multiple adoptions;
  - side derivation: `both` for full-sibling-rooted paths, `other` via a `gender:other` linking parent;
  - resolution honesty: `disconnected` vs `noneWithinBounds` (disconnected-lineages corpus vs a beyond-cap pair), unbounded lineal detection past the cap;
  - bad input: unknown id, marriage-id-where-person-expected → typed errors at each surface;
  - three-valued filtering: each operator against exact/partial/circa dates in both certainty modes; deterministic sort order.
- **Prior art:** the validator's per-rule snapshot tests and the export/render snapshot suites are the model — same `insta` mechanics, same "behavior not implementation" discipline (CODING_STANDARDS.md, docs/testing.md).
- **The WASM and CLI adapters are tested for wiring and serialization only** — that the fourth WASM shape (including the query envelope and the detail lookups) round-trips to the committed TypeScript types (ADR-0012 tsify discipline) and that `kul query` maps args onto the Query value and emits the core result. Kinship correctness is *not* re-tested at the adapters. The `--format json` CLI path doubles as the contract-serialization snapshot harness over the example corpus.
- **A perf budget** guards the ~10k-person interactive target, alongside the existing corpus perf budgets (docs/testing.md).

## Out of Scope

- **An end-user query language / DSL / grammar / parser.** The deliverable is a library API. (A DSL may be built *on top of* the API later if real usage proves the ergonomic gap.)
- **The culture-specific terminology layer** (descriptor → "sister-in-law" | "bhabhi" | …). It is the intended *next* layer — an additive, community-growable data map over the descriptor — but ships no entries here. The only obligation is that the descriptor is discriminating enough to make that layer pure data. Corollary: no kinship words anywhere in this deliverable, including `kul query` human output.
- **Name lookup or name search.** Queries key on person id only; names are non-unique display data, and name search (fuzzy, locale-aware) is app UX over the person list or export.
- **Advanced analytics — permanently.** Distributions, group-by, histograms, statistical aggregates, generation-depth stats. Consumers build these on the exported JSON.
- **Time-scoped / date-arithmetic queries** — "alive in 1985", "age at marriage", "married in the 1980s". The deferred time-axis. The engine reads dates only for seniority ordering, three-valued field predicates, and attribute sort — never as temporal arithmetic combining fields.
- **Disjunction (OR) in attribute filters.** Conjunction-only; OR is two queries and a set union in consumer code.
- **A configurable affinal-hop ceiling.** The 2-hop ceiling is semantics, fixed by the engine; only the generation *budget* is a knob.
- **LSP-backed editor query** (author-at-edit-time surface) and **a stateful query server.**
- **A dedicated native binding (C-ABI / `uniffi`) for mobile or backend consumers.** They use the Rust crate directly.
- **Large-corpus import (e.g. GEDCOM mega-trees of 100k+ persons) — never a target,** by paradigm choice; the ~10k budget is durable, not a stopgap.
- **Cross-project federation / external-genealogy identifier interop.**

## Further Notes

- **Why "app developer, not end user" is the load-bearing first decision:** an end-user-facing query *language* is a second toolchain the size of the one already built (parser, semantics, error messages for non-programmers, editor support, a co-evolving spec). Targeting the app developer collapses that cost and attacks the *proven* pain (the empty derivation seam) instead of a *speculative* one (readers wanting to type Cypher).
- **Why the descriptor must be richer than English needs:** the additive-terminology-map promise ("grow culture packs without touching the engine") only holds if the descriptor is a superset of every target culture's distinctions. English discards side (uncle) and seniority (brother); Hindi needs both (*mama*/*chacha*, *bhabhi*) — and *chacha* vs *tau* specifically forced the second, apex-junction seniority field, because those terms compare the uncle to ego's *father*, not to ego. Under-modeling the descriptor now would force an engine change later and break the additivity promise — so the design pays for the full discrimination even though it ships no culture pack.
- **Why relationship resolution returns a set:** the full kinship graph is cyclic in practice (consanguineous marriage, adoption-into-relatives are common, not edge cases), so two people are often related several ways at once. Returning one "primary" tie would embed a cultural/UX ranking in the core and hide true relationships — both violations of the "core owns correctness, consumer owns UX" line. Path identity extends the same honesty to *same-classification* multiplicity (double cousins).
- **One date-comparison rule, three uses:** the validator's temporal rules, both seniority fields, and filter predicates all reuse the same strict every-interpretation interval comparison. The toolchain has exactly one notion of "date A is before date B".
- **The clean two-tier product story:** the engine owns *kinship correctness*; the export owns *everything else*. Anything the engine deliberately refuses (analytics, temporal queries) has a defined home on the exported JSON, so no consumer is ever blocked. The id → detail lookups keep the boundary comfortable: query consumers hydrate persons on demand without importing the whole export.
- **Grilling traceability:** the 17 open topics recorded on the epic issue (A1–G17) were resolved in a dedicated grilling session; every resolution is folded into the sections above (A1 → path identity; A2 → edge-nature × sharing split; A3 → side derivation rule with `other`; B4 → 2-affinal-hop ceiling + ended-marriage tagging; B5 → step definitions + affinity three-way + subsumption; B6 → mechanism collapse + parent-set-equality full; B7 → dual seniority + `before_strict` decidability; C8/C9 → descriptor-pattern vocabulary with named sugar; D10 → ids + descriptors over WASM, refs in Rust, id → detail lookups; D11 → serialization conventions + committed TS shape; E12 → resolution result object with emptiness reason; E13 → id-only identity + typed errors + envelope; F14 → cap default 5, unbounded lineal, semantics-vs-budget; F15 → three-valued predicates + certainty mode + conjunction-only; G16 → Query value as contract, Rust sugar deferred to `design-an-interface`; G17 → CLI verbs + terminology-neutral human output).

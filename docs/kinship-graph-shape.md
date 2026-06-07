# The shape of kul's data model, as a graph

A structural framing for kul's data model. Read this before designing, grilling, or scoping any feature that treats a Kul project as a graph — queries, layout algorithms, export projections, federation with external genealogy data, statistics, validation rules that walk relationships. The framing here is what lets contributors and PMs reach for standard graph-theory vocabulary, recognise where kul is conventional, and recognise where kul deliberately deviates.

This document is descriptive: it names a structure that already exists across the codebase. It does not decide anything; load-bearing decisions live in [`adr/`](./adr/). Domain nouns (Person, Marriage, Birth, ResolvedDocument, …) live in [`../CONTEXT.md`](../CONTEXT.md). Read both alongside this.

## The one-sentence shape

A Kul project is **a project-wide directed property multigraph with two node kinds (Person, Marriage) and three edge kinds (`spouse-of`, `born-into`, `adopted-into`), whose biological-parenthood projection is a guaranteed DAG (rule R13) but whose full kinship graph is not, and which carries date-fielded entities but no time-axis on queries.**

Every word in that sentence is load-bearing and reappears below.

## Graph-theory vocabulary that applies

Kul is a graph in the textbook sense. The standard vocabulary maps cleanly onto kul:

- **Node** (vertex) — a Person or a Marriage. Two distinct *kinds* of node, not one.
- **Edge** — a `spouse-of`, `born-into`, or `adopted-into` relationship. Three distinct *kinds* of edge.
- **Properties** — the `key:value` fields on Persons, Marriages, Births, and Adoptions. This makes kul a **property graph** (the Neo4j / openCypher model), not a plain graph and not RDF triples.
- **Directionality** — varies by edge kind (see below).
- **Multigraph** — in principle a pair of nodes can be connected by edges of more than one kind (a person could be a spouse in marriage M and a parent of a child whose birth references M). In practice this stays rare; the model permits it.
- **DAG** (directed acyclic graph) — the biological parenthood projection is one; the full kinship graph is not.
- **Bipartite** — edges only ever cross between the two node kinds (Person ↔ Marriage), never within. There is no direct `spouse-to-spouse` edge anywhere in the data; spousehood is always mediated by a Marriage node.

## The four structural axes

Every graph has a position on each of these axes. Kul's positions are deliberate and worth knowing:

### 1. Directionality

- `spouse-of` is **undirected** in fact (A is spouse of B iff B is spouse of A) but carries a **structural role**: the first-listed spouse is the [Host](../CONTEXT.md#host-of-a-marriage), the second is the joiner. Layout, render, and some queries consume this asymmetry.
- `born-into` is **directed**: the edge goes from Person (child) → Marriage (the parent-marriage).
- `adopted-into` is **directed**: same shape as `born-into` but carries a `start` date and may repeat (a person may be adopted into more than one marriage).

A consumer walking "upward" through ancestry uses `born-into` and `adopted-into` in their declared direction; walking "downward" through descent reverses them.

### 2. Properties

Kul is squarely a property graph. Both node kinds carry fields; edges carry fields too (a marriage edge's start/end dates, an adoption edge's start date). This makes kul a natural fit for Cypher-style and Datalog-style query paradigms; an RDF / SPARQL view is possible but requires shredding properties into triples.

### 3. Edge cardinality

Standard graph edges connect exactly two nodes. *Hyperedges* connect N. A birth in real life is fundamentally three-party (two parents + one child). Kul models this by making the **Marriage a first-class node** rather than introducing a hyperedge: a child has a `born-into` edge to the marriage, and the marriage has its own `spouse-of` edges to each parent. This is the industry-standard property-graph workaround for hyperedges and is the single most consequential design choice in kul's data model — it is what makes the additivity principle work (a new child does not require editing the parents' declarations).

### 4. Shape constraints

- The **biological parenthood projection** (Person → biological-parent Marriage → biological-parent Person, walking only `born-into` edges and the `spouse-of` edges of the reached marriage) **is guaranteed acyclic by validator rule R13**. The cycle detector at `crates/kul-core/src/cycles.rs` enforces it on every check. This unlocks the fast DAG algorithms — topological sort, memoized ancestor sets, O(n) traversals.
- The **full kinship graph** — adding `adopted-into` edges, adding `spouse-of` edges, and deriving sibling / in-law / cousin relations — **is not guaranteed acyclic**. Adoption can introduce loops the biological graph forbids (a person adopted into their aunt-uncle's marriage); spousehood produces undirected cycles trivially (two cousins marrying).

## Three primitives, everything else derived

The full vocabulary of human kinship — mother, father, sibling, ancestor, cousin, in-law, step-parent, half-sibling — reduces to **rules over the three primitive edges**. Nothing in the source is stored as "mother": the language only declares `spouse-of` (via `marriage` statements), `born-into` (via the `birth` sub-statement), and `adopted-into` (via the `adoption` sub-statement). Everything else is a *derived* relation, computed from the primitives.

This is a defining property of kinship as a domain, not a kul-specific accident: classical Datalog textbooks use kinship rules as the canonical example for exactly the same reason. Kul commits to the discipline at the language level by refusing to allow any non-primitive relation to be declared directly (the closest non-primitive is the [host](../CONTEXT.md#host-of-a-marriage) role, which is a structural attribute of an existing `spouse-of` edge, not a fourth edge kind).

The corollary: any feature that needs "parent" or "sibling" or "ancestor" must either *compute* it from the primitives or query through a layer that does. The current computation layer is [`ResolvedDocument`](../CONTEXT.md#resolveddocument); see [ADR-0001](./adr/0001-resolved-document-as-query-seam.md). Methods like `parents_of(&PersonStmt)` are the present-day rule-evaluator.

## Where kul resembles a standard graph

Useful for reaching for off-the-shelf algorithms and tooling:

- **Property graph model.** Anything that fits Neo4j / openCypher / Memgraph in principle fits kul's shape. The cytoscape export (`kul export --format cytoscape`) takes advantage of this.
- **BFS / DFS / Dijkstra / shortest-path / topological-sort all apply.** A petgraph projection of kul is a perfectly conventional graph from these algorithms' perspective.
- **Adjacency-list storage is natural.** Persons and Marriages each have small bounded fan-out (a Marriage has 2 spouses + N children; a Person has 0–1 birth + N adoptions + N marriages). Total edges grow linearly in declarations.
- **Cycle detection is a stock algorithm.** R13's implementation in `cycles.rs` is conventional DFS-based cycle detection on the directed biological-parenthood graph.
- **Connected-component analysis applies.** A Kul project can describe disconnected lineages (see `examples/07-disconnected-lineages`); these are the graph's connected components.

## Where kul deviates from a standard graph

Useful to know up front so design discussions don't smuggle in textbook assumptions that break:

- **Two node kinds, not one.** Generic graph libraries assume uniform nodes. Kul code typically carries the kind in a sum type (`enum { Person, Marriage }`) and pattern-matches on every step. Algorithms that assume uniform nodes (e.g. naive PageRank on "people") need a projection step first.
- **Marriage is a node, not an edge.** Almost every "convert this to a tree-of-people" instinct trips on this. Going from a person to their spouse is *two hops* (Person → Marriage → Person), not one. Going from a person to a parent is *two hops* (Person → Marriage → Person via the linked `born-into` or `adopted-into`). Path lengths in the kul graph are double what an English-language reading suggests.
- **The "parent" edge is derived, not stored.** A person has zero, one, or many parent-marriages (via `born-into` + N `adopted-into`); each parent-marriage has two spouses. The parent *set* is a union with cardinality 0..4+. Code that assumes "exactly two parents" is wrong on adoption, polygamy, and missing-bio cases.
- **Partial DAG-ness.** Biological parenthood is DAG; full kinship isn't. Choosing a projection is part of every graph algorithm choice. A naive "BFS over all relationships" needs an explicit visited-set; a "BFS over biological ancestors" doesn't strictly require one but should keep one anyway.
- **Project-wide namespace, not file-scoped.** Per [ADR-0015](./adr/0015-global-project-namespace.md), every id in every `.kul` file in a project is visible to every other file. The graph spans the whole project; file boundaries are organisational, not semantic. Code that walks "the graph of one file" is almost always asking the wrong question.
- **Timeless modeling for queries (today).** Dates exist on Person (`born`, `died`), Marriage (`start`, `end`), and Adoption (`start`). But current consumers (validator, layout, render, export) ask timeless questions ("ever was a spouse," "ever was a parent"). Date-arithmetic queries — "alive in 1985," "married in 1980" — are a separate body of work, deliberately deferred. Any graph-shaped feature that needs them is opening a new axis.
- **Structural roles on edges.** The host vs joiner distinction on `spouse-of` is a per-edge attribute that generic graph libraries don't know about. Renderers and some queries depend on it.

## Implications for graph-shaped features

The framing above sharpens the question "how do I add feature X" whenever X is graph-shaped:

- **A new validator rule that walks relationships** — phrase the walk in `ResolvedDocument` vocabulary; if the question is "is this a cycle in some projection?", the cycle-detector at `cycles.rs` is the prior art. New seam methods first; new traversal code second.
- **A new layout algorithm or render projection** — explicit about which projection it works on. The current canonical UI pattern's layout (`kul-layout`) operates on a *host-lineage tree* projection — a subset of the graph rooted at each component's host. It is not the whole graph; do not assume otherwise.
- **A new export format** — pick the graph view it exposes. The kinship-native shape ([ADR-0008](./adr/0008-export-kinship-native-shape.md)) preserves the three primitives faithfully; the cytoscape projection collapses some structure for consumption by standard graph tools. A new format is a new chosen view.
- **A query layer** — see the lesson notes; the derived-relation discipline, projection choice, set-vs-path output, and time-scoping all surface as design questions the moment the graph framing is named.
- **Cross-project federation / external-genealogy interop** — needs explicit identifier semantics (kul ids are project-scoped per [ADR-0015](./adr/0015-global-project-namespace.md)). Wikidata, GEDCOM, FamilySearch all bring their own id schemes; a federation layer is fundamentally a graph-merging concern, with the projection question front and centre.
- **Statistics / analytics over a kul project** — name the projection up front. "How many ancestors does person X have?" is a question on the biological-parenthood DAG. "How many people is X connected to?" is a question on the full kinship graph. The numbers differ by 2–10× on real data.

## Reading guide

Adjacent material, in the order most useful for picking up the data model:

- [`../CONTEXT.md`](../CONTEXT.md) — domain nouns (Person, Marriage, Birth, Adoption, Host, ResolvedDocument, ExportedGraph). The vocabulary the present document leans on.
- [`./architecture.md`](./architecture.md) — the implementation pipeline that produces the queryable graph (parse → resolve → validate). The graph lives in the artefacts that pipeline emits.
- [ADR-0001](./adr/0001-resolved-document-as-query-seam.md) — the present-day rule-evaluator: every kinship question goes through `ResolvedDocument`.
- [ADR-0008](./adr/0008-export-kinship-native-shape.md) — the canonical "graph view" that consumers outside the toolchain receive.
- [ADR-0013](./adr/0013-project-manifest.md), [ADR-0014](./adr/0014-file-identity-and-per-file-namespaces.md), [ADR-0015](./adr/0015-global-project-namespace.md) — the project-wide namespace and file-identity model. The graph's *scope* lives here.
- [ADR-0016](./adr/0016-visualization-pipeline-crate-boundaries.md) — the visualization pipeline crates and their graph projections.
- [ADR-0020](./adr/0020-polygamy-hub-and-fan.md) — the polygamy-hub structural pattern. A graph topology the rendering pipeline treats specially.
- `examples/` — every example file demonstrates a distinct graph topology (nuclear family, three generations, divorce-and-remarriage, adoption, cousins, polygamy, disconnected lineages, multi-file project, cross-century lineage). Browse them when grilling a feature against real graph shapes.

## When this document is wrong

Update it. If a future change introduces a fourth edge kind, a third node kind, a stored "parent" edge that bypasses the derivation rule, a federation seam that crosses project boundaries, or a time-axis on queries, the one-sentence shape at the top will no longer be true. Edit it. The descriptive shape and the load-bearing decisions should not drift apart.

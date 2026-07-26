# ADR 0037 — The batched detail lookup: a target list in, an entity-kind union out

**Status:** Accepted
**Date:** 2026-07-26
**Deciders:** owner

## Context

[ADR-0035](./0035-detail-surface-one-selection-one-batched-lookup.md) (issue
[#281](https://github.com/YashBhalodi/kul/issues/281)) decided *that* the detail surface is fed by
one batched engine call, and why: the pinned [ADR-0024](./0024-query-seam-and-envelope.md) surface
cannot answer a marriage panel (`ExportedMarriage` has no children, and no parenthood-link operation
exists at all), and composing `queryPerson` with three `queryKin` calls costs ≈63 ms at the 10k
ceiling because every stateless call re-pays the ≈13 ms check
([#292](https://github.com/YashBhalodi/kul/issues/292); the figures are in
[ADR-0034](./0034-query-transport-and-result-node-identity.md)'s measured note). It did **not** decide the
operation's shape. This ADR does, as the implementation slice
([#306](https://github.com/YashBhalodi/kul/issues/306)).

Four questions had to be answered before code:

1. **What is the unit of a request?** ADR-0035 speaks of "an entity id"; the measurement that
   justifies the design (Finding 5) is "one call, many ids". Those are different operations.
2. **How is an adoption addressed?** ADR-0035 requires the operation to serve three entity kinds,
   and notes that an adoption link has no entity id — it is a sub-statement of the child.
3. **What shape is the answer?** One uniform neighbourhood record, or a union per entity kind?
4. **Does the CLI get a matching verb?** #306 scopes it: only if it maps 1:1 with no CLI-only
   semantics.

## Decision

### The unit of a request is a *list of targets*, and that is what makes it batched

`query::details(resolved, &[DetailTarget]) -> Vec<Option<EntityDetail>>`, wrapped for the adapters
as `query::detail_lookup(check, &[DetailTarget]) -> QueryEnvelope<DetailLookupResult>` and surfaced
as WASM `queryDetail(files, manifest, targets)`. Answers come back **in the order asked**, one per
target.

A single-target operation would have satisfied ADR-0035's detail panel and nothing else. The list is
what lets the *same* operation supply the kin list's row labels — N person targets, one check, read
`person.name` off each — which is the half of ADR-0035 that retires #292's eager-hydration finding
rather than working around it. Two operations would have meant two provenance paths for person data,
which is precisely what ADR-0035 and ADR-0034 both refused.

The engine builds **one per-invocation adjacency per batch** — a children index that is the inverse
of the resolved parent links, plus a spouse index — shared by every target. That is ADR-0025's
existing traversal posture (own adjacency per invocation, no cross-query cache, ADR-0029) applied to
a batch rather than to a single query, and it is what makes the marginal cost of a target its own
neighbourhood rather than another pass over the project.

### An adoption is addressed by `(childId, marriageId)`

`DetailTarget` is a tagged union: `{kind: "person", id}`, `{kind: "marriage", id}`,
`{kind: "adoption", childId, marriageId}`. The adoption pair is **the same pair the rendered
adoption edge already carries** (`data-child-id` + `data-marriage-id`, ADR-0021), and the same key
ADR-0034 uses to bind a vertical hop to an edge — so a consumer that can click an edge can address
it without inventing a key.

Two alternatives were rejected. **Synthesising an adoption id** (`"{child}@{marriage}"` or an
index) would put a second identity scheme into a language whose ids are author-written, and
`RevealTarget` still could not use it — ADR-0035 already routes adoption reveal to the child.
**Addressing by the child alone** cannot name *which* adoption when a person has several, and the
corpus has exactly that case (Dalisay, in `examples/04-adoption-and-belonging`).

The union is deliberately narrow: `birth` is not a target, because ADR-0035 makes person cards,
marriage edges and adoption edges selectable and nothing else. A birth target is a new variant if a
surface ever needs one — additive, never a reshape.

### The answer is a union per entity kind, carrying only export shapes

`EntityDetail` is tagged on `kind` with three variants, so a consumer's three panel variants map 1:1
onto it:

- `person` — the person, `parents` (one row per parent per link — a link naming a marriage yields a
  row per spouse), `marriages` (one row per marriage, with the other spouse), `children` (one row per
  link across those marriages);
- `marriage` — the marriage, its `spouses`, its `children` (one row per link);
- `adoption` — the adoption link, the `child`, and the adopting `parents`.

A **uniform** `{subject, parents, spouses, children}` record was the alternative. It was rejected:
it forces a marriage to carry a permanently empty `parents` list and an adoption to call its one
child "children", and it makes every consumer learn which lists are meaningful for which kind. The
union says what is true.

Every entity in the answer is an **export shape** — `ExportedPerson`, `ExportedMarriage`,
`ExportedParenthoodLink` — built through the export's `build_one_*` builders, which this slice
extends with `build_one_birth_link` / `build_one_adoption_link` so the whole-graph export loop and
the lookup can never drift. ADR-0024 already refused a second, leaner person shape for query
results; that refusal is what makes this operation able to serve a display name at all. The two row
types the union needs — `LinkedPerson` (a person plus the parenthood link that reaches them) and
`MarriageTie` (a marriage plus the other spouse) — are *pairings* of export shapes, not new entity
shapes.

**One row per link, not per person.** A person reached both by birth and by a later adoption appears
twice, with a different `link` each time. This is [ADR-0026](./0026-relationship-descriptor-and-path-identity.md)'s
path-identity discipline: the engine does not collapse two distinct ties, and a consumer that wants
"just the parents" collapses on `person.id` itself.

**Absence is the answer, per target.** A target naming no entity yields `null` *in its own position*;
the rest of the batch still answers. This is ADR-0024's subject-vs-anchor line held exactly: the
target is the subject of the question, so "no such entity" is a complete answer, and no typed error
is introduced. A project that fails its checks is the envelope's error arm, whole (ADR-0009) — never
a partial batch.

**No ghost model.** Nothing in the answer knows a person is drawn more than once. ADR-0035 pinned
this: re-deriving [ADR-0019](./0019-ghost-model-and-bio-anchor.md) inside the query layer would
oblige the engine to stay in step with `kul-layout` forever. The per-marriage row with its span and
end reason is the underlying truth.

### The CLI is untouched

`kul query` gains no verb. Two things would have to be invented to give it one, and #306 scopes both
out:

- **An adoption target has no CLI spelling.** Any single-argument form (`child@marriage`,
  `--child X --marriage Y`) is a CLI-only address for a pair the contract carries as two named
  fields.
- **A batch has no CLI spelling either.** The whole point of the operation is N targets of mixed
  kinds in one call; flattening that into argv means a per-argument kind tag, i.e. a small argument
  grammar the core does not have.

The operation exists to serve one batched click in a webview. `kul query person|marriage <id>` still
answers the same underlying data one entity at a time, and `kul export` carries the parenthood links
in full, so nothing is unreachable from a terminal. If a CLI need appears later it can be added
without reshaping the contract.

## Consequences

- The batched lookup's cost is flat in the number of targets, and that flatness is now a **test**
  (`crates/kul-core/tests/perf.rs::batched_detail_cost_is_flat_in_the_number_of_targets`), not a
  measurement in a transient document. Observed: 40 targets cost ≈1.0× one target, check included.
- The parenthood link's serialized shape now has one producer path, like the person and marriage
  shapes before it. Adding a field to `ExportedParenthoodLink` updates the export *and* the detail
  lookup in one edit.
- `queryDetail` is the sixth function on the WASM query surface and the first that takes a *list* of
  request values. Its TypeScript types ship under ADR-0012's committed-tsify discipline; the
  `DetailLookupResult` payload alias is declared in the same `typescript_custom_section` as the
  other payload aliases, because tsify erases transparent Rust type aliases.
- The CLI and the WASM surface are no longer feature-symmetric. That is a first for the query seam
  and is accepted deliberately, on the grounds above.

## Anti-suggestions (do not re-propose)

- **"Make it single-target and let the consumer loop."** That is the composition ADR-0035 measured
  and rejected: each call re-pays the check, so a 20-row kin list costs 276 ms at the ceiling
  instead of 13 ms. The list *is* the operation.
- **"Return a uniform `{parents, spouses, children}` record for all three kinds."** It forces
  meaningless empty lists (a marriage has no parents) and misnames an adoption's single child as
  "children". The tagged union costs a `switch` and tells the truth.
- **"Give adoptions a synthetic id so all three targets are just an id."** A second identity scheme
  in a language whose ids are author-written, for a link `RevealTarget` still cannot address
  (ADR-0035 routes adoption reveal to the child). The `(childId, marriageId)` pair is already on the
  rendered edge.
- **"Define a lean `{id, name}` row type for the kin list instead of sending whole persons."** That
  is the second person shape ADR-0024 pinned against. Sending the export shape is what keeps the
  widget on one provenance path, and the extra bytes are irrelevant against the check that dominates
  the call.
- **"Collapse a person reached by both birth and adoption into one row."** Descriptor identity is
  path identity (ADR-0026). Collapsing throws away the adoption's dates and the fact that there are
  two ties; a consumer that wants one row can group on `person.id`.
- **"Add a `ghost` flag / note so the panel can explain the duplicate cards."** Rejected by ADR-0035
  and unchanged here: the query layer would have to re-derive ADR-0019 and track `kul-layout`
  forever.
- **"Take the parent rows from `ResolvedDocument::parents_of` instead of walking the links."**
  `ParentLink` deliberately carries only the link's kind, not the adoption's `start:` / `end:`, so
  the dates would need a second pass anyway — and the batch needs a whole-project children index
  regardless, which is the adjacency ADR-0025 already sanctions building per invocation.
- **"Add `kul query detail` for symmetry."** It cannot be spelled without a CLI-only pair syntax and
  a CLI-only batch grammar. Symmetry is not worth a second address vocabulary.

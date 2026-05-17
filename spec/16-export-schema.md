## 16. Export schema

This section is normative. It specifies the canonical JSON envelope a conforming Kul exporter MUST produce. The reference exporter is `kul export`, surfaced both as a CLI subcommand and as the public function `kul_core::export::export`.

The export is a one-way projection from a Kul document to a flat JSON shape suitable for downstream consumers (visualizers, web apps, scripts, generators). The `.kul` source remains the canonical artifact; the export is derived.

**One envelope per project.** A Kul project (per [Section 14](./14-project-manifest.md)) is one logical namespace regardless of how many `.kul` files it spans. A conforming exporter MUST emit exactly **one** envelope per project, with the `graph`'s `persons`, `marriages`, and `parenthoodLinks` collections carrying the union of every file's contents. The export does not attribute entities to their source files; downstream consumers that care about file-of-origin track it themselves.

The decisions behind this schema — its kinship-native shape, its strict-on-diagnostics posture, and its independent versioning — are recorded in [ADR-0008](../docs/adr/0008-export-kinship-native-shape.md), [ADR-0009](../docs/adr/0009-export-strict-on-diagnostics.md), and [ADR-0010](../docs/adr/0010-export-schema-versioning.md).

### 15.1 Envelope shape

A conforming exporter MUST emit one of two top-level objects.

**Success envelope.** Emitted when validation produced no error-severity diagnostics.

```json
{
  "ok": true,
  "schema": 1,
  "kul": "0.1",
  "graph": { ... }
}
```

- `ok` MUST be the boolean `true`.
- `schema` MUST be a positive integer identifying the export schema version (currently `1`).
- `kul` MUST be the language version that produced the export — the version declared by the document's `kul <version>` line, or the implementation's default if no declaration is present.
- `graph` MUST be the [graph object](#152-graph-object).

**Failure envelope.** Emitted when validation produced one or more error-severity diagnostics. Warnings alone MUST NOT trigger the failure envelope.

```json
{
  "ok": false,
  "diagnostics": [ ... ]
}
```

- `ok` MUST be the boolean `false`.
- `diagnostics` MUST be an array of [diagnostic objects](#155-diagnostic-objects), one per diagnostic the validator produced (errors, warnings, and notes alike). The array preserves the diagnostic order produced by the validator.

### 15.2 Graph object

The graph is the kinship-native projection: three flat collections, one per language primitive.

```json
{
  "persons":          [ ... ],
  "marriages":        [ ... ],
  "parenthoodLinks":  [ ... ]
}
```

- All three collections MUST be present, even when empty.
- Cross-references MUST be by id only. Embedded objects (e.g. inlining the spouses inside a marriage object) MUST NOT appear.
- Derived projections (e.g. `person.children`, `person.siblings`, `marriage.duration`) MUST NOT appear. Consumers compose these views from the flat collections.

### 15.3 Person object

```json
{
  "id":     "alice",
  "name":   "Alice Sharma",
  "family": "Sharma",
  "given":  "Alice",
  "gender": "female",
  "born":   { ... },
  "died":   { ... }
}
```

- `id` MUST be the declared id of the person.
- `name` MUST be the value of the `name:` field (always present — rule R03).
- `gender` MUST be one of `"male"`, `"female"`, `"other"` (always present — rule R03).
- `family`, `given`, `born`, `died` MUST be present iff the corresponding field appeared on the source declaration. Absent fields MUST be omitted from the JSON object (not emitted as `null`).
- `born` and `died` MUST be [date objects](#156-date-object).
- `span` MUST be present iff the exporter was invoked with positions enabled (see §15.9). When present it MUST be a two-element `[byteStart, byteEnd]` array covering the source-level statement.

### 15.4 Marriage object

```json
{
  "id":        "m_alice_bob",
  "spouses":   ["alice", "bob"],
  "start":     { ... },
  "end":       { ... },
  "endReason": "divorce"
}
```

- `id` MUST be the declared id of the marriage.
- `spouses` MUST be a two-element array of person ids in declaration order. The first entry is the marriage's **host** (see [§4.2](./04-top-level-statements.md#42-marriage-statement)).
- `start` MUST be a [date object](#156-date-object) (always present — rule R03).
- `end` MUST be present iff the source declared an `end:` field. Per rule R05, `end` and `endReason` are paired.
- `endReason` MUST be the value as written in source (currently the only valid value is `"divorce"`).
- `span` MUST be present iff the exporter was invoked with positions enabled (see §15.9). When present it MUST be a two-element `[byteStart, byteEnd]` array covering the source-level statement.

### 15.5 Parenthood-link object

Each `birth` or `adoption` sub-statement projects to one parenthood-link entry.

```json
{
  "marriageId": "m_alice_bob",
  "childId":    "ravi",
  "kind":       "adoptive",
  "start":      { ... },
  "end":        { ... }
}
```

- `marriageId` MUST be the marriage id referenced by the sub-statement.
- `childId` MUST be the id of the person whose declaration carried the sub-statement.
- `kind` MUST be `"biological"` for `birth` links or `"adoptive"` for `adoption` links. Future kinds (e.g. surrogacy) land additively without bumping the schema (see §15.7).
- `start` MUST be present iff the source `adoption` carried a `start:` field. Always absent on biological links.
- `end` MUST be present iff the source `adoption` carried an `end:` field. Always absent on biological links.
- `span` MUST be present iff the exporter was invoked with positions enabled (see §15.9). When present it MUST be a two-element `[byteStart, byteEnd]` array covering the source-level `birth` or `adoption` sub-statement.

### 15.6 Date object

```json
{
  "value":     "1980-03",
  "precision": "month",
  "circa":     true
}
```

- `value` MUST be the date as a string in `YYYY`, `YYYY-MM`, or `YYYY-MM-DD` form. The leading `~` MUST NOT appear in `value` (the `circa` flag carries that information).
- `precision` MUST be one of `"year"`, `"month"`, `"day"` and MUST agree with the granularity of `value`.
- `circa` MUST be the boolean `true` iff the source date carried a leading `~`, otherwise `false`.

### 15.7 Diagnostic objects

Diagnostic objects in the failure envelope's `diagnostics` array MUST match the schema produced by `kul validate --format json`:

```json
{
  "code":     "KUL-R03",
  "severity": "error",
  "message":  "person `alice` needs a `name:` field — add `name:\"…\"` to the declaration",
  "primary":  { "byteStart": 7, "byteEnd": 12, "line": 1, "column": 8 },
  "related":  [
    { "label": "first declared here", "byteStart": ..., "byteEnd": ..., "line": ..., "column": ... }
  ]
}
```

- `severity` MUST be one of `"error"`, `"warning"`, `"note"`.
- `byteStart` and `byteEnd` are byte offsets into the source; `line` and `column` are 1-indexed.

### 15.8 Forward compatibility

Consumers MUST tolerate forward-compatible additions WITHOUT updating their parser:

- New optional fields on existing objects MUST be ignored if unrecognised.
- New enum values (e.g. a future `gender` or `end_reason` value) MUST be passed through verbatim and not assumed to be limited to the values listed here.
- New `parenthood_links.kind` values MUST be tolerated.

A consumer that requires an exact-known shape MAY check the `schema` field and refuse to render documents shaped by an unknown schema number.

A new `schema` number MUST be allocated only when consumers might silently mis-represent data by ignoring a new construct (e.g. a brand-new top-level collection appears, or an existing field's semantics change incompatibly). Adding optional fields, enum values, or new `parenthood_links.kind` values MUST NOT bump the schema.

### 15.9 Source positions (opt-in)

When the exporter is invoked with positions enabled (the reference CLI flag is `--with-positions`), every Person, Marriage, and parenthood-link object in the graph MUST carry a `span` field. The field MUST be a two-element array `[byteStart, byteEnd]` of half-open byte offsets into the source string, identifying the source range the entity was projected from. The range MUST cover the full statement (or sub-statement) including any indented continuations the parser attached to it.

When positions are disabled (the default), the `span` field MUST be omitted from every object — not emitted as `null`. The default keeps the envelope compact for consumers (CLI pipelines, generators) that do not need source positions.

Source positions MUST NOT appear on date objects, on the envelope itself, or on diagnostic objects (the diagnostic shape already carries its own `byteStart` / `byteEnd` per §15.7).

### 15.10 Cytoscape format (opt-in)

A conforming exporter MAY also emit the graph in the **Cytoscape JSON shape** when explicitly requested (the reference CLI flag is `--format cytoscape`). This shape is loadable into Cytoscape Desktop, Cytoscape.js, Sigma.js, vis-network, and other tools that consume the standard `{ nodes, edges }` graph form.

The envelope structure (`ok`, `schema`, `kul`, `graph`) is unchanged. Only the `graph` field's payload differs:

```json
{
  "ok": true,
  "schema": 1,
  "kul": "0.1",
  "graph": {
    "nodes": [
      { "data": { "id": "p:<person-id>", "type": "person", "name": "...", "gender": "...", ... } },
      { "data": { "id": "m:<marriage-id>", "type": "marriage", "start": {...}, "end": {...}, "endReason": "..." } }
    ],
    "edges": [
      { "data": { "source": "m:<marriage-id>", "target": "p:<person-id>", "type": "spouse" } },
      { "data": { "source": "m:<marriage-id>", "target": "p:<person-id>", "type": "biological_child" } },
      { "data": { "source": "m:<marriage-id>", "target": "p:<person-id>", "type": "adoptive_child", "start": {...} } }
    ]
  }
}
```

Modeling rules:

- Marriages MUST be promoted to first-class nodes (`type: "marriage"`), so they can carry their `start`, `end`, and `endReason` as node `data` fields.
- Person ids in the cytoscape graph MUST be prefixed `p:` and marriage ids MUST be prefixed `m:` to avoid collisions in the single flat node namespace.
- Every edge MUST run from a marriage node to a person node (the graph is bipartite). Person-to-person edges MUST NOT appear.
- Spouse edges MUST carry `type: "spouse"`, with `source` the marriage and `target` the spouse person.
- Parenthood edges MUST carry `type: "biological_child"` for `birth` links and `type: "adoptive_child"` for `adoption` links. Adoptive edges MUST carry the adoption's `start` (and `end`, if present) as edge `data` fields. Biological edges MUST NOT carry `start` / `end`.

The cytoscape format is a **derived projection** of the canonical kinship-native shape (§15.2). It contains the same data, in a different arrangement; it cannot represent anything the kinship-native shape does not.

The failure envelope shape (§15.1) is unchanged in cytoscape mode — strict-on-errors applies regardless of format.

### 15.11 Worked example

For the source in [`examples/03-three-generations/three-generations.kul`](../examples/03-three-generations/three-generations.kul), a conforming exporter produces (line breaks added for readability):

```json
{
  "ok": true,
  "schema": 1,
  "kul": "0.1",
  "graph": {
    "persons": [
      { "id": "ramesh", "name": "Ramesh Sharma", "gender": "male",
        "born": { "value": "1925-03-10", "precision": "day", "circa": false },
        "died": { "value": "2005-08-22", "precision": "day", "circa": false } },
      { "id": "sita", "name": "Sita Sharma", "gender": "female",
        "born": { "value": "1928-07-15", "precision": "day", "circa": false },
        "died": { "value": "2010-11-04", "precision": "day", "circa": false } },
      { "id": "alice", "name": "Alice Sharma", "gender": "female",
        "born": { "value": "1950-04-12", "precision": "day", "circa": false } },
      { "id": "bob", "name": "Bob Sharma", "gender": "male",
        "born": { "value": "1948-11-30", "precision": "day", "circa": false },
        "died": { "value": "2020-03-15", "precision": "day", "circa": false } },
      { "id": "carol", "name": "Carol Sharma", "gender": "female",
        "born": { "value": "1975-09-03", "precision": "day", "circa": false } },
      { "id": "ravi", "name": "Ravi Sharma", "gender": "male",
        "born": { "value": "1980", "precision": "year", "circa": true } }
    ],
    "marriages": [
      { "id": "m_ramesh_sita", "spouses": ["ramesh", "sita"],
        "start": { "value": "1948-06-10", "precision": "day", "circa": false } },
      { "id": "m_alice_bob", "spouses": ["alice", "bob"],
        "start":     { "value": "1972-05-12", "precision": "day", "circa": false },
        "end":       { "value": "1990-08-01", "precision": "day", "circa": false },
        "endReason": "divorce" }
    ],
    "parenthoodLinks": [
      { "marriageId": "m_ramesh_sita", "childId": "alice", "kind": "biological" },
      { "marriageId": "m_alice_bob",   "childId": "carol", "kind": "biological" },
      { "marriageId": "m_alice_bob",   "childId": "ravi",  "kind": "adoptive",
        "start": { "value": "1985-06-01", "precision": "day", "circa": false } }
    ]
  }
}
```

---

← [Section 15 — Formatter rules](./15-formatter-rules.md) | [Index](./README.md)

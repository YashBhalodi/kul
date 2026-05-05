## 15. Export schema

This section is normative. It specifies the canonical JSON envelope a conforming Kula exporter MUST produce. The reference exporter is `kula export`, surfaced both as a CLI subcommand and as the public function `kula_core::export::export`.

The export is a one-way projection from a Kula document to a flat JSON shape suitable for downstream consumers (visualizers, web apps, scripts, generators). The `.kula` source remains the canonical artifact; the export is derived.

The decisions behind this schema — its kinship-native shape, its strict-on-diagnostics posture, and its independent versioning — are recorded in [ADR-0008](../docs/adr/0008-export-kinship-native-shape.md), [ADR-0009](../docs/adr/0009-export-strict-on-diagnostics.md), and [ADR-0010](../docs/adr/0010-export-schema-versioning.md).

### 15.1 Envelope shape

A conforming exporter MUST emit one of two top-level objects.

**Success envelope.** Emitted when validation produced no error-severity diagnostics.

```json
{
  "ok": true,
  "schema": 1,
  "kula": "0.1",
  "graph": { ... }
}
```

- `ok` MUST be the boolean `true`.
- `schema` MUST be a positive integer identifying the export schema version (currently `1`).
- `kula` MUST be the language version that produced the export — the version declared by the document's `kula <version>` line, or the implementation's default if no declaration is present.
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
  "persons":           [ ... ],
  "marriages":         [ ... ],
  "parenthood_links":  [ ... ]
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

### 15.4 Marriage object

```json
{
  "id":         "m_alice_bob",
  "spouses":    ["alice", "bob"],
  "start":      { ... },
  "end":        { ... },
  "end_reason": "divorce"
}
```

- `id` MUST be the declared id of the marriage.
- `spouses` MUST be a two-element array of person ids in declaration order.
- `start` MUST be a [date object](#156-date-object) (always present — rule R03).
- `end` MUST be present iff the source declared an `end:` field. Per rule R05, `end` and `end_reason` are paired.
- `end_reason` MUST be the value as written in source (currently the only valid value is `"divorce"`).

### 15.5 Parenthood-link object

Each `birth` or `adoption` sub-statement projects to one parenthood-link entry.

```json
{
  "marriage_id": "m_alice_bob",
  "child_id":    "ravi",
  "kind":        "adoptive",
  "start":       { ... },
  "end":         { ... }
}
```

- `marriage_id` MUST be the marriage id referenced by the sub-statement.
- `child_id` MUST be the id of the person whose declaration carried the sub-statement.
- `kind` MUST be `"biological"` for `birth` links or `"adoptive"` for `adoption` links. Future kinds (e.g. surrogacy) land additively without bumping the schema (see §15.7).
- `start` MUST be present iff the source `adoption` carried a `start:` field. Always absent on biological links.
- `end` MUST be present iff the source `adoption` carried an `end:` field. Always absent on biological links.

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

Diagnostic objects in the failure envelope's `diagnostics` array MUST match the schema produced by `kula validate --format json`:

```json
{
  "code":     "KULA-R03",
  "severity": "error",
  "message":  "person `alice` needs a `name:` field — add `name:\"…\"` to the declaration",
  "primary":  { "byte_start": 7, "byte_end": 12, "line": 1, "column": 8 },
  "related":  [
    { "label": "first declared here", "byte_start": ..., "byte_end": ..., "line": ..., "column": ... }
  ]
}
```

- `severity` MUST be one of `"error"`, `"warning"`, `"note"`.
- `byte_start` and `byte_end` are byte offsets into the source; `line` and `column` are 1-indexed.

### 15.8 Forward compatibility

Consumers MUST tolerate forward-compatible additions WITHOUT updating their parser:

- New optional fields on existing objects MUST be ignored if unrecognised.
- New enum values (e.g. a future `gender` or `end_reason` value) MUST be passed through verbatim and not assumed to be limited to the values listed here.
- New `parenthood_links.kind` values MUST be tolerated.

A consumer that requires an exact-known shape MAY check the `schema` field and refuse to render documents shaped by an unknown schema number.

A new `schema` number MUST be allocated only when consumers might silently mis-represent data by ignoring a new construct (e.g. a brand-new top-level collection appears, or an existing field's semantics change incompatibly). Adding optional fields, enum values, or new `parenthood_links.kind` values MUST NOT bump the schema.

### 15.9 Worked example

For the source in [`examples/03-three-generations.kula`](../examples/03-three-generations.kula), a conforming exporter produces (line breaks added for readability):

```json
{
  "ok": true,
  "schema": 1,
  "kula": "0.1",
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
        "start": { "value": "1972-05-12", "precision": "day", "circa": false },
        "end":   { "value": "1990-08-01", "precision": "day", "circa": false },
        "end_reason": "divorce" }
    ],
    "parenthood_links": [
      { "marriage_id": "m_ramesh_sita", "child_id": "alice", "kind": "biological" },
      { "marriage_id": "m_alice_bob",   "child_id": "carol", "kind": "biological" },
      { "marriage_id": "m_alice_bob",   "child_id": "ravi",  "kind": "adoptive",
        "start": { "value": "1985-06-01", "precision": "day", "circa": false } }
    ]
  }
}
```

---

← [Section 14 — Formatter rules](./14-formatter-rules.md) | [Index](./README.md)

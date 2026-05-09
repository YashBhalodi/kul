# 14. Project manifest

A Kul **project** is a sibling pair: one `kul.yml` manifest plus one or more `.kul` files in the same directory. The manifest carries metadata *about* the source — most notably the language version the sibling `.kul` files conform to — that previously rode inside the grammar. Lifting it out keeps the DSL focused on kinship.

The manifest is **normative**: every Kul-language consumer (this toolchain today, third-party tools tomorrow) MUST honor the discovery rule and schema in this section. It is also **required**: a `.kul` file without a sibling `kul.yml` is not a valid Kul project, and tools MUST report this as an error.

## 14.1 Filename and location

| Property                | Value         |
| ----------------------- | ------------- |
| Filename                | `kul.yml`     |
| Encoding                | UTF-8 (no BOM) |
| Location relative to source | Same directory as the `.kul` file(s) it governs |

The manifest does NOT walk up to ancestor directories: the manifest for a `.kul` file at `<path>/<file>.kul` is `<path>/kul.yml` and only `<path>/kul.yml`. (The multi-file refactor that follows this issue may revisit; today the rule is purely directory-scoped.)

## 14.2 Schema

The `kul.yml` document is a YAML mapping with one required field today:

```yaml
kul: "0.1"
```

| Field | Type   | Required | Meaning                                                                                |
| ----- | ------ | -------- | -------------------------------------------------------------------------------------- |
| `kul` | string | yes      | The Kul language version the sibling `.kul` files conform to, in `MAJOR.MINOR` form. |

YAML `#` comments are permitted and discarded during parsing.

**Manifest schema versioning.** The manifest schema evolves in lockstep with the Kul language version — there is no separate `manifest_version:` field. New optional fields land additively (per the additivity principle, [Section 13](./13-versioning-policy.md)); new required fields gate on a major language version bump.

**Unknown fields.** Fields the parser does not recognize are silently ignored. This preserves forward compatibility: a future `kul.yml` carrying a field this implementation has not yet learned about MUST NOT cause a hard failure here.

## 14.3 Discovery rules

Given an input `.kul` file at path `<dir>/<file>.kul`:

1. The manifest path is `<dir>/kul.yml`.
2. If `<dir>/kul.yml` does not exist, the input has no manifest and tools MUST report the situation as an error.
3. If the manifest is malformed (fails YAML parsing or is missing the required `kul:` field), tools MUST report the parse error.

**Programmatic input.** When the source is supplied programmatically (e.g. the `@kullang/wasm` bridge), discovery has no path to anchor to. Programmatic embeddings MUST take the manifest as an argument alongside the source.

## 14.4 What tools MUST do on missing or malformed manifest

A conforming tool MUST report the manifest failure to its caller before any kinship validation. Manifest diagnostics carry normative `KUL-Mxx` codes (defined in [Section 7](./07-validation-rules.md)) and flow through the same diagnostic infrastructure as `.kul`-side rules:

- `KUL-M01` — manifest not found at expected path. Unanchored; the would-be path is in the message.
- `KUL-M02` — manifest YAML malformed. Anchors at the line/column the YAML parser reported.
- `KUL-M03` — manifest is well-formed YAML but missing the required `kul:` field. Anchors at the manifest start.
- `KUL-M04` — manifest's `kul:` value is not a recognized Kul language version. Anchors at the value.
- `KUL-M05` — manifest carries an unknown top-level field. Severity warning; anchors at the field key.

Each adapter chooses an appropriate surface for these diagnostics:

- `kul-cli` renders them through the standard `RenderableDiagnostic` path with line/column anchors into `kul.yml`.
- `kul-lsp` filters them out of the `.kul`-URI squiggle list (the manifest is a different file from the `.kul` file the editor has open) but they remain available through the `kul/export` failure-envelope path.
- `@kullang/wasm` surfaces them in the `CheckEnvelope.diagnostics` array; structurally-malformed JS manifest objects continue to raise a `tsify` exception on the bridge boundary because that's a JS type error, not a content error.

---

← [Section 13 — Versioning policy](./13-versioning-policy.md) | [Index](./README.md) | Next → [Section 15 — Formatter rules](./15-formatter-rules.md)

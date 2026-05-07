# ADR 0010 — Export schema is versioned independently of the language

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

The export envelope carries two version markers: a `schema` integer and a `kul` string. They look similar but mean different things, and conflating them would force the wrong upgrade behaviour on consumers.

The language version (`kul 0.1`) governs what surface syntax and semantics the source uses. It bumps when a new field, statement kind, or sub-statement kind lands. Per [`spec/13-versioning-policy.md`](../../spec/13-versioning-policy.md), MINOR bumps are additive: a `0.1` document remains valid in `0.2`.

The schema version governs what *shape* the exported JSON envelope carries. Whether a downstream consumer can safely read an envelope is a question about the JSON shape, not the source language — a consumer that does not look at a brand-new optional field does not need to know the language gained that field. Conversely, a structural change to the envelope (e.g. a new top-level collection appears, or an existing field changes meaning) needs the consumer to act *even if the language version did not change*.

A single shared version would force a schema bump on every additive language change — most of which the consumer can ignore — or, worse, would let language changes silently change envelope shape.

## Decision

The export envelope carries two independent version fields:

- `kul`: the language version that produced the export, as written in the document's `kul <version>` declaration (defaulting to the current language version if absent). Surfaced so a consumer can warn the user when the source predates a feature it relies on. Otherwise informational.
- `schema`: a positive integer identifying the export envelope's structural shape. The current value is `1`. Changes are governed by this ADR.

A new `schema` integer is allocated *only when consumers might silently mis-represent data by ignoring a new construct*. Practical examples:

- A new top-level collection appears (e.g. `households`, `migrations`). Consumers that never read it will silently drop part of the document. Schema bump.
- An existing field's semantics change incompatibly (e.g. `marriages.spouses` becomes a heterogeneous list of `{kind, id}` objects instead of a flat string array). Consumers' parsers will break. Schema bump.
- The envelope's discriminator changes (e.g. `ok: true` is replaced with `status: "success"`). Schema bump.

Conversely, the following changes are *forward-compatible additions* and MUST NOT bump the schema:

- A new optional field on an existing object (e.g. `person.nickname`). Consumers ignore unknown keys.
- A new value in an existing enum (e.g. a new `gender` value or a new `end_reason` value). Consumers handle unknown values as opaque strings.
- A new value in `parenthood_links.kind` (e.g. surrogacy). Same as above.

The single-row `field_meta` discipline ([ADR-0005](./0005-field-metadata-table.md)) is what makes this hold operationally: adding a new field is one row in the meta table plus a new variant on the AST enum; the exporter projects it automatically through the AST accessor, with no per-export-field code to update. The temptation to break compatibility for convenience never arises because the convenient path *is* the additive path.

The implementation pins `SCHEMA_VERSION` as a `pub const u32` in `crates/kul-core/src/export.rs`. Consumers index into it via the `schema` field on the envelope.

## Consequences

- A consumer that targets schema `1` keeps working across MINOR language bumps. It only has to update if the schema bumps — which the project commits to doing rarely and deliberately.
- A consumer that wants to refuse documents shaped by an unknown schema can check `schema` at the boundary and fail fast. The current value is small and stable enough that "if schema != 1, refuse" is a one-line guard.
- The `kul` field becomes purely informational. Consumers can use it to surface "this document is from language version 0.3 — your renderer is from 0.1, here are the features you might be missing", but the envelope structure itself is the schema's responsibility.
- The pre-flight check ("does this consumer support this envelope?") is one integer compare. No string parsing, no MAJOR.MINOR semantic-version logic, no compatibility matrix.
- A future MAJOR language bump may or may not coincide with a schema bump — the two are independent. A 1.0 language with the same envelope shape stays at schema 1.

## Anti-suggestions (do not re-propose)

- **"Use semantic versioning for the schema (`schema: \"1.0.0\"`)."** Hides the simple "do I support this number?" check behind a parser. The schema version is a discriminator, not a release identifier — keep it an integer.
- **"Bump the schema whenever the language bumps."** Forces every consumer to act on every language bump even when the envelope shape did not change. Defeats the entire reason for keeping them separate.
- **"Drop the `kul` field, since the schema number captures everything."** Loses the information a consumer needs to warn the user about source-side language drift. Two fields, two responsibilities.
- **"Embed the schema version in the URL of a published JSON-Schema document."** Premature. We do not publish a JSON-Schema document for the envelope yet, and may never need to — the spec section ([§15](../../spec/15-export-schema.md)) is the contract. If a JSON-Schema document is later wanted, it can carry the same integer in its `$id`.
- **"Allow MINOR schema versions for additive shape changes (e.g. `1.1`)."** Same reason as the first item — the integer's whole job is to be a check at the boundary, not a release identifier. Additive changes are explicitly defined as no-bump in this ADR.

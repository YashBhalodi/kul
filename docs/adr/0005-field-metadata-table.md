# ADR 0005 — `field_meta` is the single source of truth for per-field facts

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

Each field on a Kula statement carries several facts a tool needs: its name, its value shape (string / date / enum), a one-line description for completion items, and a long-form Markdown blurb for hover popovers. Before this ADR these facts were independently restated in three feature modules:

- `crates/kula-lsp/src/features/hover.rs` knew the hover Markdown for every `PersonFieldKind`, `MarriageFieldKind`, and `AdoptionFieldKind` variant.
- `crates/kula-lsp/src/features/completion.rs` knew the short-form description and which fields were string-shaped (so it could wrap the value in quotes).
- `crates/kula-lsp/src/features/semantic_tokens.rs` knew which token type to emit for each field's value (string / number / enum-member).

Adding a field meant editing the AST enum, the parser, the validator, and three feature modules — and the compiler caught only the first two. A typo in any one of the feature modules silently desynchronised the editor experience.

## Decision

`crates/kula-core/src/field_meta.rs` is the canonical per-field metadata table. One `FieldMeta` row per `FieldName`, containing:

- `value_kind: ValueKind` — `String`, `Date`, or `Enum`. The shape of the value as written in source.
- `short_doc: &'static str` — one-line description used in completion-item details.
- `hover_md: &'static str` — long-form Markdown for hover popovers, including the bold field-name header and any examples.

A separate `fields_for(StatementKind) -> &'static [FieldName]` lookup lists the fields valid on each statement shape (`Person`, `Marriage`, `Adoption`) in the canonical formatter order.

The path from a parsed value back to its `FieldName` lives on the AST kind enums as `field_name(&self) -> FieldName` (and `value_span(&self) -> ByteSpan`). `PersonFieldKind`, `MarriageFieldKind`, and `AdoptionFieldKind` each implement these.

Adding a new field is now:

1. Extend `FieldName` (lexer) and the relevant AST kind enum (additively, per the additivity principle).
2. Add a row to `field_meta::META` and to the relevant `*_FIELDS` slice.
3. Update the parser to emit the new variant.

Hover, completion, and semantic-token features pick up the new field automatically.

## Consequences

- The field taxonomy stops being something a feature module can disagree about. Hover, completion, and semantic tokens all read from one row each.
- A test in `field_meta` asserts that every `FieldName` has a `META` row, so a new variant without a row fails compilation-adjacent (the test panics).
- The hover content for fields that appear on multiple statement shapes (currently `start:` and `end:`, which appear on both `marriage` and `adoption`) is shared. The prose acknowledges both contexts in one paragraph rather than producing per-statement variants. If a future field has materially different semantics between statement shapes the table key can grow to `(StatementKind, FieldName)` — but that complication is unwarranted while every field is either statement-specific or near-identical across shapes.
- Field metadata is in `kula-core` rather than `kula-lsp`. The justification: it describes the language, not the editor protocol. A future CLI command (`kula explain born`) or doc-generator can consume the same table without duplicating it.

## Anti-suggestions (do not re-propose)

- "Move the table into `kula-lsp` since only the LSP currently consumes it." Premature scoping. The CLI and any future tool that reflects on fields belong on the same surface.
- "Generate the table from the spec." The spec is normative prose; the table is a programmatic surface. They share content but their formats differ enough that one isn't trivially derived from the other. Cross-checking via tests is fine; cross-generation would be a maintenance burden.
- "Replace per-statement-shape lookups (`fields_for`) with a single `FieldName::valid_on(StatementKind)` method." Same information, more scattered call sites. The slice form lets completion and the formatter iterate in canonical order with no sort step.
- "Replace `field_name()` accessors on the AST enums with a `Display` impl or trait." The accessor is a typed method that returns a typed enum; a trait/`Display` form would lose type safety for no readability gain.

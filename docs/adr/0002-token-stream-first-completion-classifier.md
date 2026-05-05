# ADR 0002 — Completion classifier walks tokens first, AST second

**Status:** Accepted
**Date:** 2026-05-05
**Deciders:** owner

## Context

The LSP completion feature has to answer "what completions are valid at this cursor position?" at every keystroke. The challenge is that the cursor is almost always on partially-typed input — `person alice gen` (asking for `gender:`), `marriage m_a alice |` (asking for the second spouse-id), `birth m_|` (asking for an existing marriage id). Most of these states do not parse cleanly: the token stream may end mid-keyword, or end with whitespace after a colon, or contain a partial identifier the parser would error on.

Two designs were considered:

1. **AST-first.** Run the parser, locate the cursor in the resulting AST, ask the AST what variant is expected next. Falls back to token inspection only when the parser errors out.
2. **Token-stream-first.** Walk the lexer's token stream up to the cursor line, classify the cursor context from the tokens directly (with cursor-adjacency rules to handle whitespace and partial tokens), and use the AST as a secondary signal (e.g. "what is the enclosing person/marriage?").

The completion-classifier implementation hit several cases where the AST-first design failed:

- A partial identifier (`gen`) tokenizes as `Ident("gen")`, but no parser production accepts a bare ident in person-field position; the parser errors and recovery skips the line. The AST then doesn't know that the cursor was *intended* to be a field name.
- Cursor adjacency matters: `gender: ` (cursor after the space) is in **value** context, but `gender:|female` (cursor before the value, no space) is also value context, and `gender :` (cursor after a stray space) is *not* a field — it's a parse error. The AST cannot distinguish these post-recovery; the token stream can.
- The seven distinguishable contexts (TopLevelStart, IndentedUnderPerson, PersonFieldList, MarriageFieldList, AdoptionFieldList, AfterGenderColon, AfterEndReasonColon) align cleanly with token sequences but only loosely with AST shape.

## Decision

The completion classifier in `crates/kula-lsp/src/features/completion.rs` walks the token stream first. The AST (specifically `ResolvedDocument` and `node_at`) is consulted only as a secondary signal — for "what is the enclosing person?" or "what marriages are declared so far?" type questions, where the AST is authoritative.

The classifier expresses its rules as cursor-adjacency on tokens: for example, "the cursor is in a field-value context only if the previous token is a `:` *or* a value-shaped token whose span ends exactly at the cursor". This is what handles whitespace correctly: `field: ` (whitespace after colon) stays in value context because the colon was the last *content* token; `field :` (whitespace before colon) does not yet have a value position.

The seven context labels are an enum (`Context::*`); each has its own completion-list builder.

## Consequences

- Completion behavior is independent of parser recovery quality. Adding new error-recovery rules to the parser does not silently change completion behavior, because the classifier doesn't depend on a successful parse.
- The classifier is testable as a pure function over `&str → Context` plus a `LineInfo` derived from the token stream. The LSP integration tests cover end-to-end protocol; the unit tests can target the classifier directly.
- The price: the classifier and the parser have small overlap in "what's a field-value-shaped token?" semantics. They must agree, and a future spec change (e.g. allowing a new shape of value literal) requires updating both. This is acceptable — the parser is the source of truth for what's *valid*; the classifier is the source of truth for what *should complete*.

## Anti-suggestions (do not re-propose)

- "Just use the AST and skip token-stream reasoning" — the AST cannot represent partially-typed input cleanly, and parser recovery is lossy. The token stream survives recovery.
- "Make completion run after a successful parse only" — would mean *no completions* during the most common typing state (mid-identifier). Defeats the feature.
- "Generate completions from the spec EBNF" — the spec describes valid programs, not in-progress edits. Cursor-adjacency and partial-token semantics are out of scope for the grammar.
- "Move classification into kula-core so the CLI could use it too" — the CLI doesn't need completion. The classifier is editor-protocol concern; it stays in kula-lsp.

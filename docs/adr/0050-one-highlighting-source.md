# ADR 0050 — One highlighting source: the Rust lexer and semantic tokens

**Status:** Accepted
**Date:** 2026-11-02
**Deciders:** owner

Issues [#346](https://github.com/YashBhalodi/kul/issues/346), [#350](https://github.com/YashBhalodi/kul/issues/350), [#354](https://github.com/YashBhalodi/kul/issues/354), [#353](https://github.com/YashBhalodi/kul/issues/353).

## Context

Kul source is shown in three places: the app's editor, the docs' code blocks and the landing page's sample. A hand-written grammar per place (a TextMate grammar for the docs' highlighter, a Lezer grammar for CodeMirror) would be a second and third definition of the language's lexical structure, each free to disagree with the real lexer — and Kul's lexer has corners a grammar gets wrong (unterminated strings, field-dependent value kinds, the id after `person` being a declaration).

## Decision

### Two passes, both from the engine

- **Lexical tokens** (`kul_language_service::lexical_tokens(source)`, `Workspace.lexicalTokens(file)`) classify one file from the lexer alone: keywords, field names, declarations, references, enum values, dates, strings and comments. Comments come from the gaps between tokens. The first id after `person` or `marriage` is a declaration; a value after `field:` takes the kind the field table (ADR-0005) gives it; an unterminated string is cut at the line end. Lexical tokens need no check, so they are cheap enough for every keystroke and complete on lines that do not parse.
- **Semantic tokens** classify from the checked syntax tree, once analysis has run. Both use one `TokenKind` enum.

A consumer paints the lexical tokens and overlays semantic tokens where both cover the same span. A test holds every semantic token on four examples to an identical lexical twin, so the overlay refines and never contradicts.

### One set of classes and colours

`TOKEN_CLASS` and `overlaySemantic` live in `apps/web/src/highlight/tokens.ts`. Every place renders a token as a `.tok-*` class coloured from the `--syntax-*` tokens in `apps/web/src/styles/tokens.css` (each at least 5.9:1 against the card in both themes).

- **Editor:** the worker answers each edit with lexical tokens at once; decorations are mapped through the edits made since the version they were computed for (`EditTrail`), so a stale answer is applied rather than dropped. Semantic tokens overlay only when their version equals the lexical one.
- **Docs and landing page, at build time:** `apps/web/src/highlight/` runs `kul-wasm` in Node (`analyzeKul`, `highlightKul`), and `rehype-kul` turns each ```` ```kul ```` block into highlighted lines. Inside a Shiki block the `.tok-*` rules also set `--shiki-light` / `--shiki-dark`.

### The same pass checks the examples

Because the build already analyses every block, it also checks it: a plain ```` ```kul ```` block must check clean, `expect-error=KUL-R03` (or a comma list for errors that always cascade) must report exactly those codes, `project="x" title="…"` blocks are checked together as one project, and `tree` attaches the engine's SVG. A failing block fails the build. The landing page's sample and its feature examples go through the same check (`landing-content.ts`), and the error messages shown under docs examples are the engine's, generated at build time.

## Consequences

- No grammar file exists in the repository: `rg "tmLanguage|StreamLanguage|@lezer/lr|LRParser" apps crates` finds nothing. A change to the lexer reaches the editor, the docs and the landing page with no grammar to update.
- Docs examples cannot drift from the language: a block that stops checking fails the build.
- `@codemirror/language` is used only for indentation; its parser stack is never imported.

## Anti-suggestions (do not re-propose)

- **"Add a TextMate or Lezer grammar for speed."** Lexical tokens are already per-keystroke and cover unparsable lines.
- **"Highlight docs blocks on the client."** The build already has the tokens; shipping the engine to read a docs page costs every reader.
- **"Paint semantic tokens alone."** They exist only after analysis, and lines that do not parse have none.

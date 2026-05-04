# PRD 04 — Polished LSP

**Phase:** 4 of 4
**Headline deliverable:** A set of incremental LSP features that take Kula from "feels like a real language" to "feels like a polished language": ID-aware autocomplete, find references, rename, document symbols, code actions, formatter, semantic tokens.
**Target outcome version:** Extension `0.2.0` and beyond, `kula-lsp` `0.2.x`, `kula-cli` gains `kula format`.

## Problem Statement

After Phase 3 the user has live diagnostics, hover, jump-to-definition, and basic completion in VSCode. But several features that distinguish a hobby DSL from a polished language are still missing:

- Completion suggests *keywords* but not *the actual person and marriage IDs you've declared* — so writing `birth m_` doesn't help you discover that `m_alice_bob` exists.
- "Find all references to this person" requires manual grep.
- Renaming a person ID requires manually editing every reference, with no safety net for missed ones.
- The document outline view doesn't show the file's structure (persons, marriages).
- An error like "missing required `gender`" can't be one-click-fixed.
- The file's whitespace and field order are subject to author whim — there's no canonical layout.
- Highlighting still uses TextMate (Phase 1), so person IDs and marriage IDs render the same color.

These features are individually optional but collectively define what users expect from a "first-class" language tool in 2026.

## Solution

Phase 4 is a *bag of incremental features*, each shippable as a point release. None of them are foundational the way the parser or the LSP itself were — they extend the existing layers. The overall plan: add a feature, ship a point release, repeat.

The features:

1. **ID-aware completion.** After `birth `, suggest declared marriage IDs with their display info. After `marriage <id> `, suggest declared person IDs.
2. **Find references.** Right-click → Find References on a person or marriage ID returns all locations that reference it.
3. **Rename.** F2 on a person or marriage ID prompts for a new name; the LSP returns a workspace edit covering the declaration and all references.
4. **Document symbols.** The VSCode outline view shows persons and marriages as a hierarchical tree.
5. **Code actions.** For specific diagnostics, the LSP offers quick-fixes (e.g. "add required field `gender`").
6. **Formatter.** A new `format` module in `kula-core`, surfaced as `kula format <file>` in the CLI and `textDocument/formatting` in the LSP. Canonicalizes whitespace, field order, and alignment.
7. **Semantic tokens.** Richer highlighting via the LSP's `semanticTokens/full` capability — different visual treatment for declared person IDs vs marriage IDs vs references vs keywords.

Each feature is independently scoped and ships when ready.

## User Stories

### ID-aware completion

1. As a Kula author, after typing `birth ` (with the trailing space), I want autocomplete to show me every declared marriage with its ID and a short description (e.g. "alice + bob, 1972–1990") so that I can pick the right one without scrolling.
2. As a Kula author, after typing `marriage <id> ` I want autocomplete to show me declared person IDs with their display names so that I can quickly assemble a marriage.
3. As a Kula author, after typing `marriage <id> alice ` I want the second-spouse suggestion to *exclude* `alice` (you can't marry yourself) so that I don't pick an invalid combination.
4. As a Kula author, after typing `adoption ` (in a person sub-statement) I want autocomplete to show me marriage IDs.
5. As a Kula author, completion suggestions for IDs should include a "details" string showing relevant context (display name, birth year for persons; spouse names and date span for marriages) so that I can disambiguate similar IDs at a glance.
6. As a Kula author, ID completion should not include the *current person's own ID* in the spouse list of a `marriage` they're being assembled into, where it's contextually invalid.

### Find references

7. As a Kula author, I want to right-click on a person ID and choose "Find All References" to see every place that ID is referenced (marriages where they're a spouse, `birth`/`adoption` sub-statements that point at marriages they're in).
8. As a Kula author, I want the same on a marriage ID — find all `birth` and `adoption` references to that marriage.
9. As a Kula author, the references view should distinguish *declarations* from *references* so that I can tell which is which.

### Rename

10. As a Kula author, I want to press F2 on a person ID, type a new name, and have all declarations and references in the document update atomically so that I never have to do this manually.
11. As a Kula author, the same on marriage IDs.
12. As a Kula author, I want rename to refuse to rename to an ID that's already taken (would cause a duplicate-ID error) so that I don't accidentally break my document.
13. As a Kula author, I want rename to refuse to rename to a reserved keyword (e.g. `person`, `marriage`) so that I don't accidentally produce an unparseable document.

### Document symbols

14. As a Kula author, I want VSCode's Outline view (and the file-symbol search via Cmd+Shift+O) to show a hierarchical list of all persons and marriages in the file so that I can navigate by structure.
15. As a Kula author, I want each symbol to show its display name (not just the ID) so that the outline is human-readable.
16. As a Kula author, I want sub-statements (`birth`, `adoption`) to nest under their parent person in the outline so that the outline mirrors the document's logical structure.

### Code actions

17. As a Kula author, when a `person` is missing the required `gender` field, I want a quick-fix that adds `gender:female` (or `gender:male`, or `gender:other`) at the end of the line.
18. As a Kula author, when a `person` is missing the required `name` field, I want a quick-fix that adds `name:""` with the cursor positioned inside the quotes.
19. As a Kula author, when a marriage has `end:` but no `end_reason:`, I want a quick-fix that adds `end_reason:divorce` (the only v1 value).
20. As a Kula author, when a marriage has `end_reason:` but no `end:`, I want a quick-fix that removes the `end_reason:` field (since `end:` would need a date the user hasn't supplied).
21. As a Kula author, code actions should appear on the lightbulb icon in the gutter and in the right-click menu so that I can discover them naturally.

### Formatter

22. As a Kula author, I want `kula format file.kula` to canonicalize my file in place so that I get consistent formatting without thinking about it.
23. As a Kula author, I want format-on-save in VSCode to apply the formatter automatically so that I never see un-formatted code.
24. As a Kula author, I want the formatter to be **opinionated** — no configuration knobs (or very few). One canonical format is the goal.
25. As a Kula author, I want the formatter to align field-name columns within a person block when reasonable so that fields read as a table.
26. As a Kula author, I want the formatter to put fields in canonical order: positional first, then required fields, then optional fields, ordered by spec section.
27. As a Kula author, I want the formatter to preserve my comments and blank lines so that my organizing structure (e.g. `# Generation 1` headers) survives formatting.
28. As a Kula author, I want the formatter to be **idempotent** — formatting an already-formatted file is a no-op so that the result is stable.
29. As a Kula author, I want `kula format --check file.kula` to exit non-zero if the file is not formatted so that I can use it as a CI gate.
30. As a Kula tooling author, I want the formatter exposed as a library function in `kula-core` so that I can format programmatically (e.g. from a code-generation tool).

### Semantic tokens

31. As a Kula author, I want declared person IDs colored differently from declared marriage IDs so that I can scan the file's structure quickly.
32. As a Kula author, I want references to a person colored differently from a person's declaration site so that "this is where this person is *defined*" is visually distinct from "this is a *use*."
33. As a Kula author, I want similar treatment for marriages.
34. As a Kula author, semantic tokens should respect my color theme (no hard-coded colors) so that they look right in light themes, dark themes, and high-contrast themes.

### As an AI agent developer

35. As an AI agent developer, I want each new LSP capability to land as its own commit / PR with its own tests so that bisecting bugs is easy.
36. As an AI agent developer, I want the formatter's round-trip property (parse → format → parse → assert AST equivalence) to be a property test so that any formatter regression is caught immediately.
37. As an AI agent developer, I want a small benchmark suite (`cargo bench` against a synthetic 5000-statement document) so that I can detect performance regressions from new features.

### As the project maintainer

38. As the project maintainer, I want each Phase 4 feature to have a brief migration note in the extension's CHANGELOG so that users know what's new.
39. As the project maintainer, I want the `kula-lsp` capabilities reported in `initializeResult.capabilities` to grow truthfully as features ship — never claim a capability the server doesn't support.

## Implementation Decisions

### Module additions

- **`kula-core::format`** (NEW deep module). Public surface: `format(doc: &Document) -> String`. Pure function: takes a parsed document, returns canonical formatted source. The CLI's `kula format` subcommand and the LSP's `textDocument/formatting` handler both call this directly.
- **`kula-core::semantic`** gains a `references_to(id: SymbolId) -> Vec<Location>` query.
- **`kula-core::semantic`** gains a `symbols(doc) -> Vec<Symbol>` query (used for both document symbols and ID-completion details).
- **`kula-lsp::features::references`** — new module. Implements `textDocument/references`.
- **`kula-lsp::features::rename`** — new module. Implements `textDocument/prepareRename` and `textDocument/rename`.
- **`kula-lsp::features::document_symbol`** — new module.
- **`kula-lsp::features::code_action`** — new module. Diagnostics-driven: each diagnostic code can register a `CodeActionProvider` that, given the diagnostic and document, returns one or more `CodeAction`s.
- **`kula-lsp::features::formatting`** — new module. Wraps `kula_core::format::format`.
- **`kula-lsp::features::semantic_tokens`** — new module. Walks the AST, emits one semantic token per identifier with a stable `SemanticTokenType`.
- **`kula-lsp::features::completion`** is extended to handle ID-reference contexts (after `birth `, after `marriage <id> ` and `marriage <id> <person> `, after `adoption `).

### Formatter design

- **Opinionated, no config.** One canonical layout. We document the rules in `spec/14-formatter-rules.md` (added in this phase) so the formatter is itself a normative artifact.
- **Field order:** positional args first, then required fields in spec-table order, then optional fields in spec-table order. For person: `name`, `gender`, `family`, `given`, `born`, `died`. For marriage: `start`, `end`, `end_reason`. For adoption sub-statement: `start`, `end`.
- **Field alignment:** within a single person block (one statement line + indented sub-statements), align the colons of fields that fit on the same conceptual column. Don't align across statements.
- **Single space** around `:` is forbidden — fields are `name:value` (matches lexer requirement).
- **Two spaces** between fields on a one-line `person` or `marriage` statement.
- **Indentation:** sub-statements use exactly two spaces.
- **Blank lines:** preserve user's blank lines but collapse runs of more than one blank line to a single blank line.
- **Comments:** preserve verbatim, including position (end-of-line vs own-line).
- **Idempotence is non-negotiable.** A formatted file that's formatted again must produce identical output. Tested with a property test on the entire `examples/` corpus and the test corpus.

### Semantic tokens taxonomy

| Token | LSP `SemanticTokenType` | Usage |
| --- | --- | --- |
| Top-level keywords (`person`, `marriage`, `kula`) | `keyword` | TextMate also matches; semantic tokens override |
| Sub-statement keywords (`birth`, `adoption`) | `keyword` | |
| Field names | `property` | |
| Enum values (`male`, `female`, `other`, `divorce`) | `enumMember` | |
| Person ID at declaration | `class` (or `namespace`) | TBD; pick the one that themes color most distinctly from `function` |
| Marriage ID at declaration | `function` | Different from person decls |
| Person ID at reference | `variable` | |
| Marriage ID at reference | `parameter` | |
| Date literals | `number` | |
| String literals | `string` | |

The exact mapping is reviewed against three popular themes (One Dark, GitHub Light, Solarized) before shipping to make sure the visual distinction is real.

### Code action registry

Each diagnostic code that has fixes is paired with one or more `CodeActionProvider` functions. The mapping lives in `kula-lsp::features::code_action::registry` as a static table. Adding a new fix is one entry in the table plus the provider function.

Initial set:

| Diagnostic | Code action |
| --- | --- |
| Missing `gender` on person | "Add `gender:male`", "Add `gender:female`", "Add `gender:other`" |
| Missing `name` on person | "Add `name:\"\"`" (cursor positioned inside quotes) |
| End/end_reason mismatch (end without end_reason) | "Add `end_reason:divorce`" |
| End/end_reason mismatch (end_reason without end) | "Remove `end_reason:`" |

We deliberately don't try to fix complex errors (cycles, temporal impossibilities) — these need human judgment.

### Rename safety checks

`textDocument/prepareRename` returns the editable range only if the cursor is on an ID that can be renamed. The actual `rename` request:

1. Validates the new ID against the identifier production (`[A-Za-z_][A-Za-z0-9_-]*`).
2. Checks the new ID isn't a reserved keyword.
3. Checks the new ID doesn't collide with an existing ID in the document.
4. If any check fails, returns an error response with a clear message.
5. Otherwise, returns a workspace edit covering the declaration and every reference.

### Versioning

Each Phase 4 feature ships as its own minor release of the extension and (when applicable) the LSP:

| Release | Adds |
| --- | --- |
| Extension `0.2.0` / `kula-lsp 0.2.0` | Document symbols, ID-aware completion |
| Extension `0.3.0` / `kula-lsp 0.3.0` | Find references, rename |
| Extension `0.4.0` / `kula-lsp 0.4.0` / `kula-cli 0.2.0` | Formatter (CLI + LSP) |
| Extension `0.5.0` / `kula-lsp 0.5.0` | Code actions |
| Extension `0.6.0` / `kula-lsp 0.6.0` | Semantic tokens |

Order of these is loose — whichever feature is ready ships first. The grouping above is an estimate.

## Testing Decisions

### What makes a good test (in this phase)

Same standards as Phase 2 and 3: test external behavior of each module, not internals. Each new feature has its own test module. Snapshot tests are the default for anything that produces structured output (workspace edits, code actions, completion lists, semantic tokens, formatted output).

### Per-feature test plan

- **Formatter** — golden tests: pairs of `<input>.kula` and `<output>.kula` files in `crates/kula-core/tests/format/`. Round-trip property test: for every file in the corpus, `format(parse(format(parse(source)))) == format(parse(source))` (idempotence) and the AST is unchanged. Spec compliance: every example in the spec is checked to be in canonical form (so format-on-save would be a no-op).
- **ID-aware completion** — extended completion test corpus from Phase 3 with new context cases. Each case is a `.kula` snippet with a cursor marker (`<CURSOR>`); the test asserts the snapshot of the completion list at that position.
- **Find references** — for each feature corpus document, place a cursor on each declared ID and assert the snapshot of returned locations.
- **Rename** — for each rename case, snapshot the workspace edit. Negative tests: invalid identifiers, reserved keywords, collisions all return errors with expected messages.
- **Document symbols** — snapshot the symbol tree for each `examples/*.kula`.
- **Code actions** — for each diagnostic-with-fix, snapshot the code actions returned. End-to-end test: apply the fix's edit, re-validate, assert the diagnostic is gone.
- **Semantic tokens** — snapshot the semantic token stream for each `examples/*.kula`.

### Performance tests

A small bench suite in `crates/kula-core/benches/` (using `criterion` or `divan`):

- Parse + validate: 100, 1000, 5000 person-statement document.
- Format: same scales.
- Reference resolution: same scales.

These don't gate CI but are run periodically. We set rough targets (e.g. parse + validate of 1000 statements should complete in under 50ms on a 2024-era laptop) and watch for regressions.

### Prior art

- `rust-analyzer`: refactor and code-action infrastructure is the gold standard but huge. We borrow patterns at a much smaller scale.
- `taplo`: formatter design (idempotent, opinionated). Reference for the formatter module structure.
- `biome`: code-action registry pattern.

## Out of Scope

- Cross-file refactoring (multi-file rename, multi-file find-references). v1 has no multi-file projects.
- AI-powered features (suggesting names based on cultural context, completion that learns from your style, etc.). Not what this language is for.
- Live family-tree visualization. Out of v1.
- Editing diagnostics for non-spec issues (style suggestions, "consider adding `family` to enable derived queries"). Out of scope unless the spec adds them as warnings later.
- Custom formatter configuration. The formatter is intentionally opinionated.
- Workspace symbol search (`workspace/symbol`). v1 is single-file; this would be vacuous.
- Inlay hints (showing computed values inline like derived parents). Could be a future addition but not in v1.
- Call hierarchy or type hierarchy (LSP capabilities that don't apply to a kinship language).

## Further Notes

- **Order is loose.** This PRD lists seven features; their internal order is not fixed. Whichever one is ready first ships first. The order in the "Versioning" table is an educated guess, not a commitment.
- **The formatter is the deepest of these features** in the Ousterhout sense — significant logic behind a tiny API (`format(&Document) -> String`). It's the one feature where I'd recommend doing a small design write-up (an ADR-style note in `docs/adr/`) before implementation, because canonical-format choices have long-term consequences.
- **Code actions are the shallowest.** Each fix is a few lines in a registry. They can land one-by-one as we identify diagnostics worth fixing.
- **Semantic tokens are the hardest to test rigorously.** Snapshot tests cover correctness of the token *stream*, but the *visual outcome* depends on the user's theme. We mitigate by reviewing the visual result against a few popular themes before each release.
- **Risk: feature creep.** Phase 4 has clear bounds (the seven features above). Anything else proposed during implementation gets deferred to a hypothetical Phase 5 or to a separate roadmap doc.
- **What happens after Phase 4?** v1 is "done" when all four phases ship coherently. Beyond v1: a query language for derived kinship terms (`alice.MZD` Murdock-style chains), a v0.2 spec (multi-file imports), web visualization. None of these are committed.

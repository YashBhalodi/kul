# ADR 0023 — Unterminated-string recovery stops at the next top-level keyword

**Status:** Accepted
**Date:** 2026-07-01
**Deciders:** owner

## Context

Embedded newlines are legal inside string literals (spec §3.3): "String contents may contain any valid UTF-8 character including newlines." The lexer's `lex_string` therefore scans to the next closing quote or EOF, treating newlines as ordinary body.

That rule collides with the most common string typo. On

```
person alice name:"Alice
person bob name:"Bob" gender:male
```

the missing closing quote on Alice's name turns every subsequent byte into string body: the scan runs to EOF, emits a single `Error` token spanning the rest of the file, and `person bob` is never lexed as a statement. The user sees one misleading error far from the real typo, and the whole tail of a document silently disappears from validation, hover, export, and rendering.

A byte-level cutoff (e.g. "strings can't cross a newline") would fix the cascade but break spec-legal multi-line strings — the spec explicitly permits them, so the lexer cannot forbid them. The recovery has to be a heuristic: cheap, local, and biased toward the far-more-common case (a forgotten quote) without outlawing the rare-but-legal one (an intentional multi-line name).

## Decision

**While scanning for a closing quote, if a line begins at column 0 with a top-level statement keyword (`person` / `marriage`), treat the string as unterminated, ending before that line.**

Concretely, in `lex_string`'s scan loop, on each embedded line terminator (`\n` or `\r\n`) the lexer peeks at the following line. If that line starts — with no leading whitespace — with a word that `classify_word` maps to `PersonKw` or `MarriageKw`, the lexer stops: it emits the existing `unterminated string literal: missing closing "` error spanning the string so far, leaves `self.pos` at the terminator, and returns. The terminator then lexes as a normal `Newline` and the keyword line lexes as its own statement. The parser already surfaces the `Error` token as a `KUL-L01` diagnostic and recovers to the newline, so the next statement parses and is diagnosed independently.

The keyed-on set is exactly the two keywords the parser's top-level loop dispatches on (`Statement::Person`, `Statement::Marriage`). Sub-statement keywords (`birth`, `adoption`) are *not* in the set: they are legal only when indented under a person, so a `birth`/`adoption` word at column 0 is not a statement boundary and must not trigger recovery. The set is single-sourced through `classify_word` rather than duplicated as a literal list, so it can't drift from what the lexer actually tokenizes those words to.

## Consequences

- **The error lands near the typo.** The unterminated-string diagnostic anchors on the string that was never closed, not on a token pages later.
- **Later statements survive.** A forgotten quote costs the author one broken field, not the rest of the file. Validation, export, and editor features see every statement after the typo.
- **Multi-line strings still work — with one sharp edge.** A spec-legal multi-line string whose continuation lines do *not* start at column 0 with `person`/`marriage` lexes unchanged as a single `String` token. The sharp edge: a multi-line string that genuinely contains a line beginning `person …` or `marriage …` at column 0 will be cut short there. This is deliberate — that shape is indistinguishable from a forgotten quote by any local rule, and the forgotten quote is overwhelmingly more likely. Authors who need such a string can indent the continuation line (any leading whitespace defeats the column-0 check) or escape around it; the far more common forgotten-quote case is what the heuristic optimizes for.
- **Recovery stays local and allocation-free.** The check is a bounded scan of the next line's leading word on each embedded newline — no lookahead structure, no parser involvement.

## Anti-suggestions (do not re-propose)

- **"Forbid newlines in string literals outright."** The spec permits them (§3.3). The lexer cannot narrow the language to simplify recovery.
- **"Add `birth` / `adoption` to the trigger set."** Those are sub-statements, legal only when indented; a bare `birth` at column 0 is not a top-level boundary. Widening the set would cut legal multi-line strings more aggressively for no recovery gain.
- **"Make the cutoff configurable / add an escape to opt out."** The heuristic is a recovery affordance, not a language feature. A knob would be a second source of truth for what ends a string; indentation already provides the opt-out.
- **"Build a general parser-error-recovery framework to drive this."** Recovery in this codebase is ad-hoc per production by design (see the lexer/parser module docs). This is one local heuristic in one scan loop, not a framework seam.

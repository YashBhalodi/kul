// Highlighting from the engine's tokens: no grammar lives in the app. The
// lexer's tokens arrive on every edit; the semantic tokens refine them
// after each analysis. A token answer is for one version of the text, so
// edits made since are mapped over it until the next answer arrives.

import { ChangeSet } from "@codemirror/state"
import type { ChangeDesc } from "@codemirror/state"

import type { Token, TokenKind } from "@/language/protocol"

/** The CSS class of each token kind; the token sheet colours them. */
export const TOKEN_CLASS: Readonly<Record<TokenKind, string>> = {
  keyword: "tok-keyword",
  fieldName: "tok-field",
  declaration: "tok-declaration",
  reference: "tok-reference",
  enumValue: "tok-enum",
  date: "tok-date",
  string: "tok-string",
  comment: "tok-comment",
}

/** A class to paint over `[from, to)`. */
export interface TokenRange {
  from: number
  to: number
  className: string
}

/**
 * Lexical tokens with the semantic ones painted over them: a semantic
 * token replaces the lexical token with the same span, and is added where
 * the lexer has none. Sorted by position.
 */
export function overlaySemantic(
  lexical: ReadonlyArray<Token>,
  semantic: ReadonlyArray<Token>
): Array<Token> {
  const key = (token: Token) => `${token.span.start}:${token.span.end}`
  const refined = new Map(semantic.map((token) => [key(token), token]))
  const merged = lexical.map((token) => {
    const better = refined.get(key(token))
    if (better) refined.delete(key(token))
    return better ?? token
  })
  merged.push(...refined.values())
  return merged.sort(
    (a, b) => a.span.start - b.span.start || a.span.end - b.span.end
  )
}

/**
 * Token ranges for a text, mapped through `changes` made to it since.
 * A token that an edit deleted is dropped; one an edit typed into grows.
 */
export function tokenRanges(
  tokens: ReadonlyArray<Token>,
  length: number,
  changes?: ChangeDesc
): Array<TokenRange> {
  const ranges: Array<TokenRange> = []
  for (const token of tokens) {
    let from = Math.min(token.span.start, length)
    let to = Math.min(token.span.end, length)
    if (changes) {
      from = changes.mapPos(from, 1)
      to = changes.mapPos(to, -1)
    }
    if (to > from) {
      ranges.push({ from, to, className: TOKEN_CLASS[token.kind] })
    }
  }
  return ranges.sort((a, b) => a.from - b.from || a.to - b.to)
}

/**
 * The edits made to one file's text since each version was sent, so an
 * answer for an older version can be mapped onto the text as it is now.
 */
export class EditTrail {
  #since: Array<{ version: number; changes: ChangeDesc }> = []

  /**
   * Record that the text, now `length` long after `changes`, was sent as
   * `version`.
   */
  sent(version: number, length: number, changes?: ChangeDesc): void {
    if (changes) {
      for (const entry of this.#since) {
        entry.changes = entry.changes.composeDesc(changes)
      }
    }
    this.#since.push({ version, changes: ChangeSet.empty(length).desc })
  }

  /**
   * The edits made since `version` was sent, or undefined when that version
   * is unknown (too old, or never sent from here).
   */
  since(version: number): ChangeDesc | undefined {
    return this.#since.find((entry) => entry.version === version)?.changes
  }

  /** Forget versions older than `version`. */
  forgetBefore(version: number): void {
    this.#since = this.#since.filter((entry) => entry.version >= version)
  }

  /** The latest version sent. */
  get latest(): number | undefined {
    return this.#since.at(-1)?.version
  }
}

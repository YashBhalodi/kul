// Kul highlighting comes from the engine's own tokens, never from a
// grammar: the lexer's tokens cover every character class, and the
// semantic tokens (from the checked syntax tree) refine them. The editor
// paints these live; the docs and the landing page paint them at build time.

import type { Token, TokenKind } from "kul-wasm"

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

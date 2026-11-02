import { describe, expect, it } from "vitest"

import type { Token } from "kul-wasm"
import { TOKEN_CLASS, overlaySemantic } from "./tokens"

const token = (start: number, end: number, kind: Token["kind"]): Token => ({
  span: { start, end },
  kind,
})

describe("TOKEN_CLASS", () => {
  it("gives every token kind its own class", () => {
    const classes = Object.values(TOKEN_CLASS)
    expect(new Set(classes).size).toBe(classes.length)
    expect(classes).toHaveLength(8)
  })
})

describe("overlaySemantic", () => {
  it("replaces a lexical token by the semantic token with the same span", () => {
    const lexical = [token(0, 6, "keyword"), token(7, 12, "reference")]
    const semantic = [token(7, 12, "declaration")]
    expect(overlaySemantic(lexical, semantic)).toEqual([
      token(0, 6, "keyword"),
      token(7, 12, "declaration"),
    ])
  })

  it("adds semantic tokens the lexer has no twin for, in order", () => {
    const lexical = [token(10, 12, "string")]
    const semantic = [token(0, 4, "reference")]
    expect(overlaySemantic(lexical, semantic)).toEqual([
      token(0, 4, "reference"),
      token(10, 12, "string"),
    ])
  })
})

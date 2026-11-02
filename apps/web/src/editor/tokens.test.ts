import { ChangeSet } from "@codemirror/state"
import { describe, expect, it } from "vitest"

import type { Token } from "@/language/protocol"
import { EditTrail, tokenRanges } from "./tokens"

const token = (start: number, end: number, kind: Token["kind"]): Token => ({
  span: { start, end },
  kind,
})

describe("tokenRanges", () => {
  // person ann name:"Ann"
  const tokens = [
    token(0, 6, "keyword"),
    token(7, 10, "declaration"),
    token(11, 16, "fieldName"),
    token(16, 21, "string"),
  ]
  const text = 'person ann name:"Ann"'

  it("maps token kinds to classes", () => {
    expect(tokenRanges(tokens, text.length)).toEqual([
      { from: 0, to: 6, className: "tok-keyword" },
      { from: 7, to: 10, className: "tok-declaration" },
      { from: 11, to: 16, className: "tok-field" },
      { from: 16, to: 21, className: "tok-string" },
    ])
  })

  it("maps tokens through edits made since their version", () => {
    // Insert "x" at 0 and grow the id: "xperson annie name:..."
    const changes = ChangeSet.of(
      [
        { from: 0, insert: "x" },
        { from: 9, insert: "ie" },
      ],
      text.length
    )
    expect(tokenRanges(tokens, text.length, changes.desc)).toEqual([
      { from: 1, to: 7, className: "tok-keyword" },
      { from: 8, to: 13, className: "tok-declaration" },
      { from: 14, to: 19, className: "tok-field" },
      { from: 19, to: 24, className: "tok-string" },
    ])
  })

  it("drops tokens an edit deleted and clamps to the text", () => {
    const changes = ChangeSet.of({ from: 7, to: 10 }, text.length)
    const ranges = tokenRanges(tokens, text.length, changes.desc)
    expect(ranges.map((range) => range.className)).toEqual([
      "tok-keyword",
      "tok-field",
      "tok-string",
    ])
    expect(tokenRanges([token(0, 50, "comment")], 5)).toEqual([
      { from: 0, to: 5, className: "tok-comment" },
    ])
  })
})

describe("EditTrail", () => {
  it("composes the edits made since each version was sent", () => {
    const trail = new EditTrail()
    trail.sent(1, 3) // "abc"
    const first = ChangeSet.of({ from: 3, insert: "d" }, 3)
    trail.sent(2, 4, first.desc) // "abcd"
    const second = ChangeSet.of({ from: 0, insert: "z" }, 4)
    trail.sent(3, 5, second.desc) // "zabcd"

    // A token on "c" of version 1 sits at 3..4 now.
    expect(trail.since(1)?.mapPos(2, 1)).toBe(3)
    expect(trail.since(2)?.mapPos(3, 1)).toBe(4)
    expect(trail.since(3)?.empty).toBe(true)
    expect(trail.latest).toBe(3)
  })

  it("forgets old versions", () => {
    const trail = new EditTrail()
    trail.sent(1, 0)
    trail.sent(2, 0, ChangeSet.empty(0).desc)
    trail.forgetBefore(2)
    expect(trail.since(1)).toBeUndefined()
    expect(trail.since(2)).toBeDefined()
    expect(trail.since(99)).toBeUndefined()
  })
})

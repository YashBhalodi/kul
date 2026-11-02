import { describe, expect, it } from "vitest"

import { holdsStatement, kulIndentation } from "./indent"

describe("kulIndentation", () => {
  it("indents under a person, for its sub-statements", () => {
    expect(kulIndentation("", 'person a name:"A" gender:male')).toBe(2)
    expect(kulIndentation("birth m", 'person a name:"A"')).toBe(2)
    expect(kulIndentation("", "  birth m_a_b")).toBe(2)
    expect(kulIndentation("", "  adoption m start:2000")).toBe(2)
  })

  it("keeps top-level statements at the margin", () => {
    expect(kulIndentation("marriage m a b", 'person a name:"A"')).toBe(0)
    expect(kulIndentation("  person b", "  birth m")).toBe(0)
    expect(kulIndentation("", "marriage m a b")).toBe(0)
    expect(kulIndentation("", undefined)).toBe(0)
  })
})

describe("holdsStatement", () => {
  it("skips blank and comment lines", () => {
    expect(holdsStatement("   ")).toBe(false)
    expect(holdsStatement("  # a note")).toBe(false)
    expect(holdsStatement("person a")).toBe(true)
  })
})

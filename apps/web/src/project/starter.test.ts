import { describe, expect, it } from "vitest"

import { workspaceWith } from "@/highlight/node-engine"
import { STARTER_SNIPPET, starterInsertion } from "./starter"

function analyze(source: string) {
  const workspace = workspaceWith({ "family.kul": source })
  try {
    const analysis = workspace.analyze()
    return {
      diagnostics: analysis.diagnostics,
      cards: (analysis.svg?.match(/class="kul-card"/g) ?? []).length,
    }
  } finally {
    workspace.free()
  }
}

describe("the starter snippet", () => {
  it("is a valid project that draws one person", () => {
    expect(analyze(STARTER_SNIPPET)).toEqual({ diagnostics: [], cards: 1 })
  })

  it("draws a family once its comments are removed", () => {
    const uncommented = STARTER_SNIPPET.split("\n")
      .map((line) => line.replace(/^# ?(?=person|marriage| {2}birth)/, ""))
      .join("\n")
    expect(analyze(uncommented)).toEqual({ diagnostics: [], cards: 3 })
  })
})

describe("starterInsertion", () => {
  it("is the snippet alone in an empty file", () => {
    expect(starterInsertion("")).toBe(STARTER_SNIPPET)
    expect(starterInsertion("\n\n")).toBe(STARTER_SNIPPET)
  })

  it("starts after a blank line in a file with text", () => {
    expect(starterInsertion("# notes")).toBe(`\n\n${STARTER_SNIPPET}`)
    expect(starterInsertion("# notes\n")).toBe(`\n${STARTER_SNIPPET}`)
    expect(starterInsertion("# notes\n\n")).toBe(STARTER_SNIPPET)
  })
})

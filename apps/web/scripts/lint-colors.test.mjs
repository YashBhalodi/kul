import { describe, expect, it } from "vitest"

import { TOKEN_SHEET, colourProblems } from "./lint-colors.mjs"

const texts = (path, source) =>
  colourProblems(path, source).map((problem) => problem.text)

describe("the colour lint", () => {
  it("lets the token sheet hold colours", () => {
    expect(texts(TOKEN_SHEET, ":root { --card: oklch(1 0 0); }")).toEqual([])
  })

  it("finds colour literals in CSS", () => {
    const css = [
      ".a { fill: #fff; }",
      ".b { stroke: rgb(0 0 0); }",
      ".c { color: oklch(0.5 0.1 200); }",
      ".d { background: white; }",
      ".e { border-color: hsl(10 20% 30%); }",
    ].join("\n")
    expect(colourProblems("src/styles.css", css)).toEqual([
      { line: 1, text: "#fff" },
      { line: 2, text: "rgb(" },
      { line: 3, text: "oklch(" },
      { line: 4, text: ": white" },
      { line: 5, text: "hsl(" },
    ])
  })

  it("allows tokens, keywords and colour words that aren't colours", () => {
    const css = [
      "/* a white card, #fff in light */",
      ".a { fill: var(--card); stroke: transparent; color: currentColor; }",
      ".b { white-space: nowrap; }",
      '.c[data-gender="male"] rect { stroke: var(--tree-hue-male); }',
    ].join("\n")
    expect(texts("src/styles.css", css)).toEqual([])
  })

  it("finds Tailwind palette classes and literals in components", () => {
    const tsx = [
      '<div className="bg-white text-red-600" />',
      '<div className="border-[#abc]" />',
      'const style = { color: "#123456" }',
      'const shade = "rgb(1 2 3)"',
    ].join("\n")
    expect(texts("src/app/x.tsx", tsx)).toEqual([
      "bg-white",
      "text-red-600",
      "border-[#",
      "#abc",
      "#123456",
      "rgb(",
    ])
  })

  it("allows token classes and comments in components", () => {
    const tsx = [
      "// a white card: #fff",
      '<div className="bg-card text-muted-foreground bg-primary/20" />',
      '<a href="#main" />',
    ].join("\n")
    expect(texts("src/app/x.tsx", tsx)).toEqual([])
  })
})

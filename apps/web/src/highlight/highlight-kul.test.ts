import { describe, expect, it } from "vitest"

import {
  analyzeKul,
  expectedErrors,
  highlightKul,
  snippetProblem,
  splitLines,
} from "./highlight-kul"
import type { SnippetDiagnostic } from "./highlight-kul"

describe("highlightKul", () => {
  it("colours every token from the engine, keeping the text exact", () => {
    const text = 'person asha  name:"Asha"  gender:female  born:~1961  # note'
    const [line] = highlightKul(text)
    expect(line?.map((run) => run.text).join("")).toBe(text)
    expect(line?.filter((run) => run.kind)).toEqual([
      { text: "person", kind: "keyword" },
      { text: "asha", kind: "declaration" },
      { text: "name", kind: "fieldName" },
      { text: '"Asha"', kind: "string" },
      { text: "gender", kind: "fieldName" },
      { text: "female", kind: "enumValue" },
      { text: "born", kind: "fieldName" },
      { text: "~1961", kind: "date" },
      { text: "# note", kind: "comment" },
    ])
  })

  it("splits lines, keeps blank ones and drops a trailing newline", () => {
    const lines = highlightKul(
      'person a  name:"A"  gender:male\n\nmarriage m a b\n'
    )
    expect(lines).toHaveLength(3)
    expect(lines[1]).toEqual([])
    expect(lines[2]?.[0]).toEqual({ text: "marriage", kind: "keyword" })
  })

  it("marks references from the checked project, not just the lexer", () => {
    const { lines } = analyzeKul([
      {
        name: "family.kul",
        text: 'person a  name:"A"  gender:male\nperson b  name:"B"  gender:female\nmarriage m a b\n',
      },
    ])
    const marriage = lines.get("family.kul")?.[2] ?? []
    expect(marriage.filter((run) => run.kind).map((run) => run.kind)).toEqual([
      "keyword",
      "declaration",
      "reference",
      "reference",
    ])
  })
})

describe("splitLines", () => {
  it("handles CRLF line ends without leaking \\r into a run", () => {
    const lines = splitLines("ab\r\ncd", [
      { span: { start: 4, end: 6 }, kind: "keyword" },
    ])
    expect(lines).toEqual([[{ text: "ab" }], [{ text: "cd", kind: "keyword" }]])
  })
})

describe("analyzeKul", () => {
  it("checks several files as one project, by line", () => {
    const { diagnostics, svg } = analyzeKul([
      { name: "a.kul", text: 'person a  name:"A"  gender:male\n' },
      { name: "b.kul", text: '\nperson b  name:"B"\n  birth m_missing\n' },
    ])
    const found = diagnostics
      .map(({ code, file, line }) => [code, file, line])
      .sort()
    expect(found).toEqual([
      ["KUL-R02", "b.kul", 3],
      ["KUL-R03", "b.kul", 2],
    ])
    expect(svg).toBeUndefined()
  })

  it("draws the tree of a clean project", () => {
    const { diagnostics, svg } = analyzeKul([
      { name: "a.kul", text: 'person a  name:"A"  gender:male\n' },
    ])
    expect(diagnostics).toEqual([])
    expect(svg).toContain('class="kul-card"')
  })
})

describe("expectedErrors", () => {
  it("reads one code or a comma-separated list from the fence meta", () => {
    expect(expectedErrors(undefined)).toBeUndefined()
    expect(expectedErrors('title="x.kul"')).toBeUndefined()
    expect(expectedErrors("expect-error=KUL-R03")).toEqual(["KUL-R03"])
    expect(expectedErrors("tree expect-error=KUL-L01,KUL-P07")).toEqual([
      "KUL-L01",
      "KUL-P07",
    ])
    expect(expectedErrors("expect-error=KUL-R05b")).toEqual(["KUL-R05b"])
  })

  it("refuses a value that is not a code", () => {
    expect(() => expectedErrors("expect-error=R03")).toThrow(/R03/)
  })
})

describe("snippetProblem", () => {
  const diagnostic = (code: string): SnippetDiagnostic => ({
    code,
    message: `${code} happened`,
    file: "family.kul",
    line: 1,
  })

  it("passes a clean block and a block with exactly its expected codes", () => {
    expect(snippetProblem([], undefined)).toBeUndefined()
    expect(
      snippetProblem(
        [diagnostic("KUL-R03"), diagnostic("KUL-R03")],
        ["KUL-R03"]
      )
    ).toBeUndefined()
    expect(
      snippetProblem(
        [diagnostic("KUL-L01"), diagnostic("KUL-P07")],
        ["KUL-L01", "KUL-P07"]
      )
    ).toBeUndefined()
  })

  it("fails an unexpected error, a missing one and an extra one", () => {
    expect(snippetProblem([diagnostic("KUL-R03")], undefined)).toMatch(
      /does not check clean:\n {2}KUL-R03 \(family\.kul, line 1\): KUL-R03 happened/
    )
    expect(snippetProblem([], ["KUL-R03"])).toMatch(
      /expects KUL-R03 but checks clean/
    )
    expect(
      snippetProblem(
        [diagnostic("KUL-R03"), diagnostic("KUL-R02")],
        ["KUL-R03"]
      )
    ).toMatch(/expects exactly KUL-R03/)
  })
})

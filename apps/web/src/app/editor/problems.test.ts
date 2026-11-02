import { describe, expect, it } from "vitest"

import type { Diagnostic } from "@/language/protocol"
import {
  countProblems,
  describeProblems,
  lineAndColumn,
  listProblems,
  problemsByFile,
} from "./problems"

const diagnostic = (
  severity: Diagnostic["severity"],
  file?: string,
  start = 0
): Diagnostic => ({
  code: `KUL-${severity}`,
  severity,
  message: severity,
  location: file ? { file, span: { start, end: start + 1 } } : undefined,
  related: [],
})

describe("countProblems and describeProblems", () => {
  it("counts errors and warnings, not notes", () => {
    const counts = countProblems([
      diagnostic("error", "a.kul"),
      diagnostic("warning", "a.kul"),
      diagnostic("error"),
      diagnostic("note", "b.kul"),
    ])
    expect(counts).toEqual({ errors: 2, warnings: 1 })
    expect(describeProblems(counts)).toBe("2 errors, 1 warning")
    expect(describeProblems({ errors: 1, warnings: 0 })).toBe(
      "1 error, 0 warnings"
    )
    expect(describeProblems({ errors: 0, warnings: 0 })).toBe("No problems")
  })
})

describe("lineAndColumn", () => {
  it("counts from 1, in UTF-16 units", () => {
    const text = "ab\ncd\n😀x"
    expect(lineAndColumn(text, 0)).toEqual({ line: 1, column: 1 })
    expect(lineAndColumn(text, 4)).toEqual({ line: 2, column: 2 })
    expect(lineAndColumn(text, 8)).toEqual({ line: 3, column: 3 })
    expect(lineAndColumn(text, 99)).toEqual({ line: 3, column: 4 })
  })
})

describe("listProblems", () => {
  it("orders project-wide problems first, then by file order and position", () => {
    const files = [
      { name: "a.kul", text: "x\ny\nz" },
      { name: "b.kul", text: "" },
    ]
    const problems = listProblems(
      [
        diagnostic("warning", "b.kul", 0),
        diagnostic("error", "a.kul", 4),
        diagnostic("error"),
        diagnostic("error", "a.kul", 0),
      ],
      files
    )
    expect(
      problems.map((problem) => [problem.location?.file, problem.line])
    ).toEqual([
      [undefined, undefined],
      ["a.kul", 1],
      ["a.kul", 3],
      ["b.kul", 1],
    ])
  })
})

describe("problemsByFile", () => {
  it("counts per file", () => {
    const counts = problemsByFile([
      diagnostic("error", "a.kul"),
      diagnostic("warning", "a.kul"),
      diagnostic("warning", "b.kul"),
      diagnostic("error"),
    ])
    expect(counts.get("a.kul")).toEqual({ errors: 1, warnings: 1 })
    expect(counts.get("b.kul")).toEqual({ errors: 0, warnings: 1 })
    expect(counts.size).toBe(2)
  })
})

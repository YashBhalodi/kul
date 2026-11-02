// The project's problems as the problems strip lists them: counted by
// severity, ordered by file and position, each with its line and column.

import type { Diagnostic, Location } from "@/language/protocol"

export interface ProblemCounts {
  errors: number
  warnings: number
}

export function countProblems(
  diagnostics: ReadonlyArray<Diagnostic>
): ProblemCounts {
  let errors = 0
  let warnings = 0
  for (const diagnostic of diagnostics) {
    if (diagnostic.severity === "error") errors++
    else if (diagnostic.severity === "warning") warnings++
  }
  return { errors, warnings }
}

const plural = (count: number, word: string) =>
  `${count} ${word}${count === 1 ? "" : "s"}`

/** "2 errors, 1 warning"; "No problems" when there are none. */
export function describeProblems({ errors, warnings }: ProblemCounts): string {
  if (errors === 0 && warnings === 0) return "No problems"
  return `${plural(errors, "error")}, ${plural(warnings, "warning")}`
}

export interface Problem {
  diagnostic: Diagnostic
  location: Location | undefined
  /** 1-based line and column of the start, when it has a location. */
  line: number | undefined
  column: number | undefined
}

/** The 1-based line and column of a UTF-16 offset into `text`. */
export function lineAndColumn(
  text: string,
  offset: number
): { line: number; column: number } {
  const before = text.slice(0, Math.min(offset, text.length))
  const lastBreak = before.lastIndexOf("\n")
  let line = 1
  for (let index = 0; index < before.length; index++) {
    if (before.charCodeAt(index) === 10) line++
  }
  return { line, column: before.length - lastBreak }
}

const SEVERITY_RANK = { error: 0, warning: 1, note: 2 } as const

/**
 * Every problem in display order: project-wide ones first, then by file
 * (in file order), position and severity.
 */
export function listProblems(
  diagnostics: ReadonlyArray<Diagnostic>,
  files: ReadonlyArray<{ name: string; text: string }>
): Array<Problem> {
  const order = new Map(files.map((file, index) => [file.name, index]))
  const texts = new Map(files.map((file) => [file.name, file.text]))
  return diagnostics
    .map((diagnostic): Problem => {
      const location = diagnostic.location
      if (!location) {
        return { diagnostic, location, line: undefined, column: undefined }
      }
      const { line, column } = lineAndColumn(
        texts.get(location.file) ?? "",
        location.span.start
      )
      return { diagnostic, location, line, column }
    })
    .sort((a, b) => {
      const fileA = a.location ? (order.get(a.location.file) ?? -1) : -2
      const fileB = b.location ? (order.get(b.location.file) ?? -1) : -2
      return (
        fileA - fileB ||
        (a.location?.span.start ?? 0) - (b.location?.span.start ?? 0) ||
        SEVERITY_RANK[a.diagnostic.severity] -
          SEVERITY_RANK[b.diagnostic.severity]
      )
    })
}

/** Error and warning counts by file, for the file list's dots. */
export function problemsByFile(
  diagnostics: ReadonlyArray<Diagnostic>
): ReadonlyMap<string, ProblemCounts> {
  const byFile = new Map<string, Array<Diagnostic>>()
  for (const diagnostic of diagnostics) {
    const file = diagnostic.location?.file
    if (!file) continue
    const list = byFile.get(file) ?? []
    list.push(diagnostic)
    byFile.set(file, list)
  }
  return new Map([...byFile].map(([file, list]) => [file, countProblems(list)]))
}

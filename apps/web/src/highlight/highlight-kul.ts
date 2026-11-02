// Kul code as highlighted lines, with what the engine thinks of it. The
// docs build runs every ```kul block through here: the lines become the
// rendered code, and the diagnostics decide whether the block may ship.

import type { TokenKind } from "kul-wasm"

import { workspaceWith } from "./node-engine.ts"
import { overlaySemantic } from "./tokens.ts"

/** A stretch of one line: plain text, or one token. */
export interface CodeRun {
  text: string
  kind?: TokenKind
}

export type CodeLine = Array<CodeRun>

/** A file of Kul source. */
export interface KulFile {
  name: string
  text: string
}

/** One problem the engine found, located by file and line (1-based). */
export interface SnippetDiagnostic {
  code: string
  message: string
  file: string | undefined
  line: number | undefined
}

export interface AnalyzedProject {
  /** Each file's highlighted lines, by file name. */
  lines: Map<string, Array<CodeLine>>
  diagnostics: Array<SnippetDiagnostic>
  /** The family tree (theme-agnostic SVG), when the project draws. */
  svg: string | undefined
}

function lineStartsOf(text: string): Array<number> {
  const starts = [0]
  for (
    let at = text.indexOf("\n");
    at !== -1;
    at = text.indexOf("\n", at + 1)
  ) {
    starts.push(at + 1)
  }
  return starts
}

function lineIndex(starts: ReadonlyArray<number>, offset: number): number {
  let line = 0
  while ((starts[line + 1] ?? Infinity) <= offset) line += 1
  return line
}

/**
 * Split `text` into lines of plain runs and token runs. Tokens never span
 * a line break; a trailing newline adds no line.
 */
export function splitLines(
  text: string,
  tokens: ReadonlyArray<{
    span: { start: number; end: number }
    kind: TokenKind
  }>
): Array<CodeLine> {
  const pending = [...tokens].sort((a, b) => a.span.start - b.span.start)
  const lines: Array<CodeLine> = []
  let start = 0
  for (const raw of text.split("\n")) {
    const end = start + raw.length - (raw.endsWith("\r") ? 1 : 0)
    const line: CodeLine = []
    let cursor = start
    let token = pending[0]
    while (token !== undefined && token.span.start < end) {
      pending.shift()
      if (token.span.start >= cursor) {
        if (token.span.start > cursor) {
          line.push({ text: text.slice(cursor, token.span.start) })
        }
        cursor = Math.min(token.span.end, end)
        line.push({
          text: text.slice(token.span.start, cursor),
          kind: token.kind,
        })
      }
      token = pending[0]
    }
    if (cursor < end) line.push({ text: text.slice(cursor, end) })
    lines.push(line)
    start += raw.length + 1
  }
  if (text.endsWith("\n")) lines.pop()
  return lines
}

/**
 * Highlight each file from the engine's lexical and semantic tokens, and
 * check the files together as one project.
 */
export function analyzeKul(files: ReadonlyArray<KulFile>): AnalyzedProject {
  const workspace = workspaceWith(
    Object.fromEntries(files.map((file) => [file.name, file.text]))
  )
  try {
    const lines = new Map<string, Array<CodeLine>>()
    const starts = new Map<string, Array<number>>()
    for (const file of files) {
      const tokens = overlaySemantic(
        workspace.lexicalTokens(file.name),
        workspace.semanticTokens(file.name)
      )
      lines.set(file.name, splitLines(file.text, tokens))
      starts.set(file.name, lineStartsOf(file.text))
    }
    const analysis = workspace.analyze()
    const diagnostics = analysis.diagnostics.map((diagnostic) => {
      const location = diagnostic.location
      const fileStarts = location && starts.get(location.file)
      return {
        code: diagnostic.code,
        message: diagnostic.message,
        file: location?.file,
        line:
          location && fileStarts
            ? lineIndex(fileStarts, location.span.start) + 1
            : undefined,
      }
    })
    return { lines, diagnostics, svg: analysis.svg }
  } finally {
    workspace.free()
  }
}

/** One snippet's highlighted lines: the landing page's sample, say. */
export function highlightKul(text: string): Array<CodeLine> {
  const name = "snippet.kul"
  return analyzeKul([{ name, text }]).lines.get(name) ?? []
}

/**
 * The diagnostic codes a block declares with `expect-error=KUL-R03`, or
 * `expect-error=KUL-L01,KUL-P07` for a mistake that always brings a second
 * one with it. Undefined when the block expects none.
 */
export function expectedErrors(
  meta: string | undefined
): Array<string> | undefined {
  const list = /(?:^|\s)expect-error=(\S+)/.exec(meta ?? "")?.[1]
  if (list === undefined) return undefined
  const codes = list.split(",")
  for (const code of codes) {
    if (!/^KUL-[A-Z]\d+[a-z]?$/.test(code)) {
      throw new Error(
        `expect-error names "${code}", which is not a diagnostic code`
      )
    }
  }
  return codes
}

/**
 * Why a block may not ship, or undefined when it may. A plain block must
 * check clean; a block that expects errors must produce exactly those
 * codes, each at least once, and no other.
 */
export function snippetProblem(
  diagnostics: ReadonlyArray<SnippetDiagnostic>,
  expected: ReadonlyArray<string> | undefined
): string | undefined {
  const describe = () =>
    diagnostics
      .map((diagnostic) => {
        const where = [
          diagnostic.file,
          diagnostic.line === undefined
            ? undefined
            : `line ${String(diagnostic.line)}`,
        ]
          .filter(Boolean)
          .join(", ")
        return `  ${diagnostic.code}${where ? ` (${where})` : ""}: ${diagnostic.message}`
      })
      .join("\n")
  if (expected === undefined) {
    if (diagnostics.length === 0) return undefined
    return `it does not check clean:\n${describe()}`
  }
  const codes = new Set(diagnostics.map((diagnostic) => diagnostic.code))
  const wanted = new Set(expected)
  const same =
    codes.size === wanted.size && [...wanted].every((code) => codes.has(code))
  if (same) return undefined
  if (codes.size === 0) {
    return `it expects ${expected.join(", ")} but checks clean`
  }
  return `it expects exactly ${expected.join(", ")}, and the engine reports:\n${describe()}`
}

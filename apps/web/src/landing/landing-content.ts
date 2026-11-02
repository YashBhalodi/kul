// Builds the landing page's content from the engine, in Node, when the site
// is built: the sample file and its tree from examples/, and every example
// of "What the language handles", checked as it would be in the app. A
// sample or example that no longer says what the page claims fails the
// build, so the page can't drift from the language.

import { readFileSync, readdirSync } from "node:fs"
import { join } from "node:path"
import { fileURLToPath } from "node:url"

import { analyzeKul, snippetProblem } from "../highlight/highlight-kul.ts"
import type { CodeLine, KulFile } from "../highlight/highlight-kul.ts"
import { FEATURES } from "./features.ts"
import type { FeatureSource } from "./features.ts"
import type {
  LandingContent,
  LandingFeature,
  LandingSample,
  SampleNote,
} from "./landing-data.ts"

const EXAMPLES = fileURLToPath(new URL("../../../../examples", import.meta.url))

/** The example "How it works" shows. */
export const SAMPLE_EXAMPLE = "02-three-generations"

/** An example project's files, in file order. */
export function exampleFiles(
  example: string,
  root: string = EXAMPLES
): Array<KulFile> {
  const dir = join(root, example)
  return readdirSync(dir)
    .filter((name) => name.endsWith(".kul"))
    .sort()
    .map((name) => ({ name, text: readFileSync(join(dir, name), "utf8") }))
}

const lineText = (line: CodeLine) => line.map((run) => run.text).join("")

/** True for a line that holds nothing but a comment, or nothing at all. */
function commentOrBlank(line: CodeLine): boolean {
  return line.every((run) =>
    run.kind === undefined ? run.text.trim() === "" : run.kind === "comment"
  )
}

/**
 * The text without its comment lines: blank lines kept only between code,
 * one at a time.
 */
export function withoutComments(lines: ReadonlyArray<CodeLine>): string {
  const kept: Array<string> = []
  for (const line of lines) {
    if (line.length > 0 && commentOrBlank(line)) continue
    const text = lineText(line)
    if (text.trim() === "" && (kept.length === 0 || kept.at(-1) === "")) {
      continue
    }
    kept.push(text.trim() === "" ? "" : text)
  }
  while (kept.at(-1) === "") kept.pop()
  return `${kept.join("\n")}\n`
}

/** The most frequent `family:` value in the text. */
function familyNameOf(text: string): string {
  const counts = new Map<string, number>()
  for (const [, name] of text.matchAll(/\bfamily:"([^"]+)"/g)) {
    if (name) counts.set(name, (counts.get(name) ?? 0) + 1)
  }
  const [best] = [...counts].sort((a, b) => b[1] - a[1])
  if (!best) throw new Error("the sample declares no family: name")
  return best[0]
}

interface Found {
  line: number
  run: number
  text: string
}

/** The first run, line by line, that `test` accepts. */
function findRun(
  lines: ReadonlyArray<CodeLine>,
  test: (run: CodeLine[number]) => boolean
): Found | undefined {
  for (const [line, runs] of lines.entries()) {
    const run = runs.findIndex(test)
    const found = runs[run]
    if (found) return { line, run, text: found.text }
  }
  return undefined
}

/** The runs of one kind on a line, after `from`. */
function runsOf(line: CodeLine, kind: string, from = 0): Array<string> {
  return line
    .slice(from)
    .filter((run) => run.kind === kind)
    .map((run) => run.text)
}

/**
 * "How it works": the example's file with its comments left out, the
 * tokens the callouts point at, and the tree. Fails when the example stops
 * showing what the page says it shows.
 */
export function buildSample(root: string = EXAMPLES): LandingSample {
  const files = exampleFiles(SAMPLE_EXAMPLE, root)
  const [file] = files
  if (files.length !== 1 || !file) {
    throw new Error(`${SAMPLE_EXAMPLE} should be a one-file example`)
  }
  const original = analyzeKul(files)
  const text = withoutComments(original.lines.get(file.name) ?? [])
  const shown = analyzeKul([{ name: file.name, text }])
  const lines = shown.lines.get(file.name) ?? []
  const problem = snippetProblem(shown.diagnostics, undefined)
  if (problem) throw new Error(`the landing sample: ${problem}`)
  if (!shown.svg || shown.svg !== original.svg) {
    throw new Error(
      "the landing sample without its comments draws a different tree"
    )
  }

  const keyword = (word: string) => (run: CodeLine[number]) =>
    run.kind === "keyword" && run.text === word
  const person = findRun(lines, keyword("person"))
  const marriage = findRun(lines, keyword("marriage"))
  const birth = findRun(lines, keyword("birth"))
  const partialDate = findRun(
    lines,
    (run) => run.kind === "date" && /^\d{4}-\d{2}$/.test(run.text)
  )
  const circaDate = findRun(
    lines,
    (run) => run.kind === "date" && run.text.startsWith("~")
  )
  if (!person || !marriage || !birth || !partialDate || !circaDate) {
    throw new Error(
      `${SAMPLE_EXAMPLE} no longer has a person, a marriage, a birth, a month-only date and a circa date for the landing page to point at`
    )
  }
  const personId = runsOf(lines[person.line] ?? [], "declaration")[0]
  const spouses = runsOf(lines[marriage.line] ?? [], "reference")
  const birthMarriage = runsOf(lines[birth.line] ?? [], "reference")[0]
  const [first, second] = spouses
  if (!personId || !first || !second || !birthMarriage) {
    throw new Error("the landing sample's callouts lost their ids")
  }

  const marks: Array<[SampleNote["kind"], Found]> = [
    ["person", person],
    ["partialDate", partialDate],
    ["marriage", marriage],
    ["birth", birth],
  ]
  const notes = marks
    .sort(([, a], [, b]) => a.line - b.line || a.run - b.run)
    .map(([kind, at], index) => ({
      kind,
      number: index + 1,
      line: at.line,
      run: at.run,
    }))

  return {
    example: SAMPLE_EXAMPLE,
    file: file.name,
    familyName: familyNameOf(text),
    lines,
    notes,
    facts: {
      personId,
      spouses: [first, second],
      birthMarriage,
      partialDate: partialDate.text,
      circaDate: circaDate.text,
    },
    svg: shown.svg,
  }
}

const joinParts = (...parts: Array<string | undefined>) =>
  `${parts.filter((part) => part !== undefined).join("\n")}\n`

/**
 * One point of "What the language handles": its example checked with the
 * lines it hides, and its shown lines highlighted. Fails when the example
 * doesn't check as the point says (clean, or exactly the expected codes on
 * the shown lines).
 */
export function buildFeature(source: FeatureSource): LandingFeature {
  const withShown = source.files.filter((file) => file.shown !== undefined)
  const [shownFile] = withShown
  if (withShown.length !== 1 || !shownFile?.shown) {
    throw new Error(`feature "${source.id}" must show lines from one file`)
  }
  const files = source.files.map((file) => ({
    name: file.name,
    text: joinParts(file.before, file.shown, file.after),
  }))
  const project = analyzeKul(files)
  const problem = snippetProblem(project.diagnostics, source.expectErrors)
  if (problem) throw new Error(`feature "${source.id}": ${problem}`)

  const first =
    shownFile.before === undefined
      ? 0
      : joinParts(shownFile.before).split("\n").length - 1
  const count = shownFile.shown.split("\n").length
  const all = project.lines.get(shownFile.name) ?? []
  const lines = all.slice(first, first + count)

  const diagnostics = project.diagnostics.map((diagnostic) => {
    const line =
      diagnostic.file === shownFile.name && diagnostic.line !== undefined
        ? diagnostic.line - first
        : undefined
    if (line === undefined || line < 1 || line > count) {
      throw new Error(
        `feature "${source.id}": ${diagnostic.code} is reported outside the shown lines`
      )
    }
    return { code: diagnostic.code, message: diagnostic.message, line }
  })

  return {
    id: source.id,
    title: source.title,
    body: source.body,
    ...(source.showFileName ? { fileName: shownFile.name } : {}),
    lines,
    diagnostics,
  }
}

/** Everything the landing page shows that comes from the engine. */
export function buildLandingContent(): LandingContent {
  return {
    sample: buildSample(),
    features: FEATURES.map(buildFeature),
  }
}

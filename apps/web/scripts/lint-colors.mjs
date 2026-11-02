#!/usr/bin/env node
// The colour lint: every colour the app shows comes from the token sheet
// (src/styles/tokens.css), so light and dark both work and a theme is one
// block of values. Anywhere else in src/, a colour literal is an error: a
// hex value, a colour function (rgb(), hsl(), oklch(), ...), a named colour
// in CSS, or a Tailwind palette class (bg-white, text-red-600, bg-[#fff]).
//
//   node scripts/lint-colors.mjs        # lint src/, exit 1 on a problem

import { readFileSync, readdirSync } from "node:fs"
import { join, relative } from "node:path"
import { pathToFileURL } from "node:url"

const ROOT = join(import.meta.dirname, "..")
export const TOKEN_SHEET = "src/styles/tokens.css"

const HEX = /#[0-9a-f]{3,8}\b/gi
const COLOUR_FUNCTION = /\b(?:rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(/gi
const NAMED_COLOURS = [
  "black",
  "white",
  "red",
  "green",
  "blue",
  "yellow",
  "orange",
  "purple",
  "pink",
  "gray",
  "grey",
  "silver",
  "maroon",
  "navy",
  "teal",
  "olive",
  "lime",
  "aqua",
  "fuchsia",
]
/** A named colour as (part of) a CSS declaration's value. */
const CSS_NAMED = new RegExp(
  `:[^;{}]*\\b(?:${NAMED_COLOURS.join("|")})\\b(?![-(])`,
  "gi"
)
const TAILWIND_PALETTE = [
  "slate",
  "gray",
  "zinc",
  "neutral",
  "stone",
  "red",
  "orange",
  "amber",
  "yellow",
  "lime",
  "green",
  "emerald",
  "teal",
  "cyan",
  "sky",
  "blue",
  "indigo",
  "violet",
  "purple",
  "fuchsia",
  "pink",
  "rose",
]
const UTILITY =
  "(?:bg|text|border(?:-[xytrbl])?|fill|stroke|ring|ring-offset|outline|from|via|to|shadow|decoration|accent|caret|divide|placeholder)"
/** A Tailwind class naming a palette colour, black, white or a literal. */
const TAILWIND_COLOUR = new RegExp(
  `\\b${UTILITY}-(?:(?:${TAILWIND_PALETTE.join("|")})-\\d{2,3}|black|white)\\b|\\b${UTILITY}-\\[#`,
  "g"
)

/** Strip comments, keeping line breaks so line numbers stay right. */
function withoutComments(text, css) {
  const blank = (match) => match.replace(/[^\n]/g, " ")
  const block = text.replace(/\/\*[\s\S]*?\*\//g, blank)
  return css ? block : block.replace(/(^|[^:"'`])\/\/[^\n]*/g, blank)
}

function lineOf(text, index) {
  return text.slice(0, index).split("\n").length
}

/**
 * The colour literals in one file of src/, as `{ line, text }`. The token
 * sheet itself is where colours belong, so it has none.
 */
export function colourProblems(path, source) {
  if (path === TOKEN_SHEET) return []
  const css = path.endsWith(".css")
  const text = withoutComments(source, css)
  const patterns = css
    ? [HEX, COLOUR_FUNCTION, CSS_NAMED]
    : [HEX, COLOUR_FUNCTION, TAILWIND_COLOUR]
  const problems = []
  for (const pattern of patterns) {
    for (const match of text.matchAll(pattern)) {
      if (
        !css &&
        pattern === HEX &&
        !/^["'`\s(:[]/.test(text[match.index - 1] ?? "")
      ) {
        continue
      }
      problems.push({ index: match.index, text: match[0].trim() })
    }
  }
  return problems
    .sort((a, b) => a.index - b.index)
    .map((problem) => ({
      line: lineOf(text, problem.index),
      text: problem.text,
    }))
}

function* sourceFiles(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name)
    if (entry.isDirectory()) yield* sourceFiles(path)
    else if (/\.(css|tsx?|jsx?)$/.test(entry.name)) yield path
  }
}

function main() {
  let count = 0
  for (const file of sourceFiles(join(ROOT, "src"))) {
    const path = relative(ROOT, file).split("\\").join("/")
    for (const problem of colourProblems(path, readFileSync(file, "utf8"))) {
      count++
      console.error(
        `${path}:${problem.line}: colour "${problem.text}" outside the token sheet (${TOKEN_SHEET})`
      )
    }
  }
  if (count > 0) {
    console.error(`${count} hard-coded colour${count === 1 ? "" : "s"}`)
    process.exit(1)
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  main()
}

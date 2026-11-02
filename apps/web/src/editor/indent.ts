// Kul's indentation: a person's `birth` and `adoption` sub-statements sit
// two spaces under it; every top-level statement starts at the margin.

/** The indent unit, as the formatter writes it. */
export const KUL_INDENT = "  "

const TOP_LEVEL = /^\s*(person|marriage)\b/

/**
 * The column a line should start at, given its own text and the nearest
 * line above it that holds a statement (comments and blank lines skipped).
 */
export function kulIndentation(
  line: string,
  statementAbove: string | undefined
): number {
  if (TOP_LEVEL.test(line)) return 0
  if (statementAbove === undefined) return 0
  if (/^person\b/.test(statementAbove)) return KUL_INDENT.length
  if (/^\s+(birth|adoption)\b/.test(statementAbove)) return KUL_INDENT.length
  return 0
}

/** Whether `line` holds a statement (not blank, not only a comment). */
export function holdsStatement(line: string): boolean {
  const trimmed = line.trim()
  return trimmed !== "" && !trimmed.startsWith("#")
}

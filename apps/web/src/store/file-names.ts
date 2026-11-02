// The rules for a project's file names. A project is a flat set of .kul
// files, unique by name ignoring case, read in the byte-wise order of their
// UTF-8 names (the order the CLI and the engine use).

export const KUL_EXTENSION = ".kul"

/** The longest file name, in characters, extension included. */
export const MAX_FILE_NAME_LENGTH = 128

export type FileNameProblem =
  "empty" | "extension" | "separator" | "control" | "too-long" | "taken"

/**
 * What is wrong with `name` as the name of a file in a project that already
 * holds `existing`, or `undefined` when it is fine. When renaming, pass the
 * file's current name as `renaming`: the file may keep its name or change
 * only its case.
 */
export function fileNameProblem(
  name: string,
  existing: Iterable<string>,
  renaming?: string
): FileNameProblem | undefined {
  if (!name.endsWith(KUL_EXTENSION)) return "extension"
  if (name.length === KUL_EXTENSION.length) return "empty"
  if (/[/\\]/.test(name)) return "separator"
  if (/[\p{Cc}]/u.test(name)) return "control"
  if (characterCount(name) > MAX_FILE_NAME_LENGTH) return "too-long"
  const self = renaming === undefined ? undefined : foldFileName(renaming)
  const folded = foldFileName(name)
  for (const other of existing) {
    const otherFolded = foldFileName(other)
    if (otherFolded === self) continue
    if (otherFolded === folded) return "taken"
  }
  return undefined
}

/** The sentence the UI shows for a problem. */
export function describeFileNameProblem(problem: FileNameProblem): string {
  switch (problem) {
    case "empty":
      return "Enter a name."
    case "extension":
      return "File names end in .kul."
    case "separator":
      return "Names can’t contain / or \\."
    case "control":
      return "Names can’t contain control characters."
    case "too-long":
      return `Names can be at most ${MAX_FILE_NAME_LENGTH} characters long.`
    case "taken":
      return "Another file already has this name."
  }
}

/**
 * The file name for what someone typed: trimmed, with `.kul` appended
 * unless it already ends that way (in any case).
 */
export function withKulExtension(input: string): string {
  const trimmed = input.trim()
  if (trimmed.toLowerCase().endsWith(KUL_EXTENSION)) {
    return trimmed.slice(0, -KUL_EXTENSION.length) + KUL_EXTENSION
  }
  return trimmed + KUL_EXTENSION
}

/** `name` without its `.kul` extension. */
export function fileStem(name: string): string {
  return name.endsWith(KUL_EXTENSION)
    ? name.slice(0, -KUL_EXTENSION.length)
    : name
}

/** Whether two names would collide in one project. */
export function sameFileName(a: string, b: string): boolean {
  return foldFileName(a) === foldFileName(b)
}

/**
 * How many characters (Unicode code points) `text` has; an emoji counts
 * once, though JavaScript's `length` counts it twice.
 */
export function characterCount(text: string): number {
  return Array.from(text).length
}

function foldFileName(name: string): string {
  return name.normalize("NFC").toLowerCase()
}

/**
 * Compare names by their UTF-8 bytes. UTF-8 sorts like Unicode code points,
 * which JavaScript's default comparison (UTF-16 code units) does not: a
 * character beyond the Basic Multilingual Plane sorts after U+FFFF here.
 */
export function compareFileNames(a: string, b: string): number {
  const left = a[Symbol.iterator]()
  const right = b[Symbol.iterator]()
  for (;;) {
    const l = left.next()
    const r = right.next()
    if (l.done || r.done) return (l.done ? 0 : 1) - (r.done ? 0 : 1)
    const difference =
      (l.value.codePointAt(0) ?? 0) - (r.value.codePointAt(0) ?? 0)
    if (difference !== 0) return difference
  }
}

/** `items` in file order. */
export function inFileOrder<T extends { name: string }>(
  items: Iterable<T>
): Array<T> {
  return [...items].sort((a, b) => compareFileNames(a.name, b.name))
}

/**
 * `name`, or the first of `name-2.kul`, `name-3.kul`, … that `taken` does
 * not hold (ignoring case). The stem is shortened if the suffix would make
 * the name too long.
 */
export function uniqueFileName(name: string, taken: Iterable<string>): string {
  const folded = new Set([...taken].map(foldFileName))
  if (!folded.has(foldFileName(name))) return name
  const stem = Array.from(fileStem(name))
  for (let n = 2; ; n++) {
    const suffix = `-${n}${KUL_EXTENSION}`
    const room = MAX_FILE_NAME_LENGTH - suffix.length
    const candidate = stem.slice(0, room).join("") + suffix
    if (!folded.has(foldFileName(candidate))) return candidate
  }
}

// Importing files into a project. Whatever arrives (single files, folders,
// .zip archives) is flattened to the .kul files in it, by name; everything
// else is ignored and counted. Names that clash with the project's files,
// or with each other, are resolved the one way the person chooses.

import {
  fileNameProblem,
  sameFileName,
  uniqueFileName,
} from "@/store/file-names"
import type { ProjectFile } from "@/store/project-store"

import { unzipEntries } from "./zip"

/** A file picked or dropped, with its path inside what was picked. */
export interface ImportEntry {
  path: string
  bytes: Uint8Array
}

export interface ImportBatch {
  /** The .kul files, in the order they arrived. */
  files: Array<ProjectFile>
  /** How many other files were left out. */
  ignored: number
}

function baseName(path: string): string {
  return path.split(/[/\\]/).at(-1) ?? path
}

function isMacMetadata(path: string): boolean {
  return path.split(/[/\\]/).some((part) => part === "__MACOSX")
}

function decode(bytes: Uint8Array): string | undefined {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes)
  } catch {
    return undefined
  }
}

/**
 * The .kul files in `entries`, flattened to their names. A .zip is opened
 * and read the same way (one level: a .zip inside it is ignored). A file
 * that is not UTF-8 text, or whose name a project can't hold, is ignored.
 */
export function flattenImport(
  entries: ReadonlyArray<ImportEntry>
): ImportBatch {
  const batch: ImportBatch = { files: [], ignored: 0 }

  function take(entry: ImportEntry, insideZip: boolean) {
    const name = baseName(entry.path)
    if (isMacMetadata(entry.path)) {
      batch.ignored++
    } else if (!insideZip && name.toLowerCase().endsWith(".zip")) {
      let inner: Array<ImportEntry>
      try {
        inner = unzipEntries(entry.bytes)
      } catch {
        batch.ignored++
        return
      }
      for (const innerEntry of inner) take(innerEntry, true)
    } else {
      const text = fileNameProblem(name, []) ? undefined : decode(entry.bytes)
      if (text === undefined) batch.ignored++
      else batch.files.push({ name, text })
    }
  }

  for (const entry of entries) take(entry, false)
  return batch
}

export type CollisionChoice = "keep-both" | "replace" | "skip"

/**
 * The incoming names that clash, ignoring case, with a file the project
 * has or with a file earlier in the batch.
 */
export function findCollisions(
  existing: ReadonlyArray<string>,
  incoming: ReadonlyArray<ProjectFile>
): Array<string> {
  const names = [...existing]
  const clashes: Array<string> = []
  for (const file of incoming) {
    if (names.some((name) => sameFileName(name, file.name))) {
      clashes.push(file.name)
    } else {
      names.push(file.name)
    }
  }
  return clashes
}

export interface ImportPlan {
  /** The files to write, under the names they get. */
  writes: Array<ProjectFile>
  /** Files new to the project. */
  added: number
  /** Project files whose text is replaced. */
  replaced: number
  /** Incoming files left out. */
  skipped: number
}

/**
 * What importing `incoming` into a project holding `existing` writes, with
 * every clash resolved by `choice`: keep both (the newcomer is numbered,
 * `name-2.kul`), replace (the newcomer's text wins, under the name already
 * there), or skip (the newcomer is left out).
 */
export function planImport(
  existing: ReadonlyArray<string>,
  incoming: ReadonlyArray<ProjectFile>,
  choice: CollisionChoice
): ImportPlan {
  const names = [...existing]
  const writes = new Map<string, ProjectFile>()
  const plan: ImportPlan = { writes: [], added: 0, replaced: 0, skipped: 0 }

  for (const file of incoming) {
    const clash = names.find((name) => sameFileName(name, file.name))
    if (clash === undefined) {
      names.push(file.name)
      writes.set(file.name, file)
      plan.added++
    } else if (choice === "keep-both") {
      const name = uniqueFileName(file.name, names)
      names.push(name)
      writes.set(name, { name, text: file.text })
      plan.added++
    } else if (choice === "replace") {
      // A newcomer replacing an earlier newcomer leaves that one out.
      if (writes.has(clash)) plan.skipped++
      else plan.replaced++
      writes.set(clash, { name: clash, text: file.text })
    } else {
      plan.skipped++
    }
  }
  plan.writes = [...writes.values()]
  return plan
}

function count(n: number, noun: string): string {
  return `${n} ${noun}${n === 1 ? "" : "s"}`
}

/** The toast after an import: "Imported 6 files, ignored 2". */
export function importSummary(result: {
  imported: number
  skipped: number
  ignored: number
}): string {
  const rest = [
    result.skipped > 0 ? `skipped ${result.skipped}` : undefined,
    result.ignored > 0 ? `ignored ${result.ignored}` : undefined,
  ].filter((part) => part !== undefined)
  if (result.imported === 0) {
    return ["No .kul files imported", ...rest].join(", ")
  }
  return [`Imported ${count(result.imported, "file")}`, ...rest].join(", ")
}

/**
 * The name for a project made from an import: the folder's or the
 * archive's name when one was picked on its own.
 */
export function importedProjectName(
  pickedNames: ReadonlyArray<string>
): string {
  const only = pickedNames.length === 1 ? pickedNames[0] : undefined
  const name = only?.replace(/\.(zip|kul)$/i, "").trim()
  return name ? name : "Imported project"
}

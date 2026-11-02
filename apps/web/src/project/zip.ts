// A project as a .zip: its files at the archive's root, so unzipping it into
// a folder gives a project the CLI reads as it is (`kul validate`).

import { strToU8, unzipSync, zipSync } from "fflate"
import type { Zippable } from "fflate"

import type { ProjectFile } from "@/store/project-store"

import type { ImportEntry } from "./import"

// A fixed time stamp keeps the archive's bytes a function of the files.
const MTIME = new Date("2000-01-01T00:00:00Z")

export function zipProject(files: ReadonlyArray<ProjectFile>): Uint8Array {
  const entries: Zippable = {}
  for (const file of files) entries[file.name] = strToU8(file.text)
  return zipSync(entries, { level: 6, mtime: MTIME })
}

/** Every file in a .zip, with its path inside the archive. */
export function unzipEntries(bytes: Uint8Array): Array<ImportEntry> {
  return Object.entries(unzipSync(bytes))
    .filter(([path]) => !path.endsWith("/"))
    .map(([path, data]) => ({ path, bytes: data }))
}

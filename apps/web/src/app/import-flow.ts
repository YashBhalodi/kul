// Importing into the app: read what was picked or dropped, flatten it to
// .kul files, ask about clashing names, write, and report what happened.

import { createStore } from "@tanstack/react-store"
import { toast } from "sonner"

import {
  findCollisions,
  flattenImport,
  importSummary,
  importedProjectName,
  planImport,
} from "@/project/import"
import type {
  CollisionChoice,
  ImportBatch,
  ImportEntry,
} from "@/project/import"
import type { ProjectFile } from "@/store/project-store"

import { createImportedProject } from "./projects"
import { sessionStore, writeFiles } from "./project-session"

/** Where an import goes: into the open project, or into a new one. */
export type ImportTarget = "current" | "new"

/** What was picked or dropped, read into memory. */
export interface PickedFiles {
  entries: Array<ImportEntry>
  /** The names of the top-level items picked (files and folders). */
  rootNames: Array<string>
}

export interface PendingImport {
  target: ImportTarget
  /** The project a new-project import creates. */
  projectName: string
  batch: ImportBatch
  /** The incoming names that clash. */
  collisions: Array<string>
  /** Opens a project the import created. */
  openProject: (id: string) => void
}

/** An import waiting for the person to choose what to do with clashes. */
export const importStore = createStore<{ pending: PendingImport | undefined }>({
  pending: undefined,
})

/** Only .kul files and archives are worth reading; the rest is counted. */
function worthReading(name: string): boolean {
  const lower = name.toLowerCase()
  return lower.endsWith(".kul") || lower.endsWith(".zip")
}

async function entryOf(file: File, path: string): Promise<ImportEntry> {
  const bytes = worthReading(file.name)
    ? new Uint8Array(await file.arrayBuffer())
    : new Uint8Array()
  return { path, bytes }
}

/**
 * Files from a file input. A folder input gives each file's path inside
 * the folder (`webkitRelativePath`), whose first part is the folder.
 */
export async function readFileList(
  files: Iterable<File>
): Promise<PickedFiles> {
  const entries: Array<ImportEntry> = []
  const roots = new Set<string>()
  for (const file of files) {
    const path = file.webkitRelativePath || file.name
    roots.add(path.split("/")[0] ?? path)
    entries.push(await entryOf(file, path))
  }
  return { entries, rootNames: [...roots] }
}

function fileOf(entry: FileSystemFileEntry): Promise<File> {
  return new Promise((resolve, reject) => entry.file(resolve, reject))
}

function childrenOf(
  entry: FileSystemDirectoryEntry
): Promise<Array<FileSystemEntry>> {
  const reader = entry.createReader()
  const all: Array<FileSystemEntry> = []
  // readEntries returns the children in batches, then an empty one.
  return new Promise((resolve, reject) => {
    const next = () =>
      reader.readEntries((batch) => {
        if (batch.length === 0) resolve(all)
        else {
          all.push(...batch)
          next()
        }
      }, reject)
    next()
  })
}

async function walk(
  entry: FileSystemEntry,
  path: string,
  into: Array<ImportEntry>
): Promise<void> {
  if (entry.isFile) {
    into.push(await entryOf(await fileOf(entry as FileSystemFileEntry), path))
  } else if (entry.isDirectory) {
    for (const child of await childrenOf(entry as FileSystemDirectoryEntry)) {
      await walk(child, `${path}/${child.name}`, into)
    }
  }
}

/**
 * Files and folders dropped on the page. Call it from the drop handler
 * itself: the dropped items are only readable during the event.
 */
export function readDataTransfer(transfer: DataTransfer): Promise<PickedFiles> {
  const roots = [...transfer.items]
    .filter((item) => item.kind === "file")
    .map((item) => item.webkitGetAsEntry())
    .filter((entry) => entry !== null)
  if (roots.length === 0) return readFileList(transfer.files)
  return (async () => {
    const entries: Array<ImportEntry> = []
    for (const root of roots) await walk(root, root.name, entries)
    return { entries, rootNames: roots.map((root) => root.name) }
  })()
}

/** Whether a drag carries files (rather than text or a link). */
export function carriesFiles(transfer: DataTransfer | null): boolean {
  return transfer?.types.includes("Files") ?? false
}

/** Start importing what was picked; clashes wait for a choice. */
export function startImport(
  picked: PickedFiles,
  target: ImportTarget,
  openProject: (id: string) => void
): void {
  const batch = flattenImport(picked.entries)
  if (batch.files.length === 0) {
    toast(importSummary({ imported: 0, skipped: 0, ignored: batch.ignored }))
    return
  }
  const existing =
    target === "current"
      ? sessionStore.state.files.map((file) => file.name)
      : []
  const pending: PendingImport = {
    target,
    projectName: importedProjectName(picked.rootNames),
    batch,
    collisions: findCollisions(existing, batch.files),
    openProject,
  }
  if (pending.collisions.length > 0) {
    importStore.setState(() => ({ pending }))
  } else {
    void finish(pending, "keep-both")
  }
}

/** Finish the waiting import with the person's choice for the clashes. */
export function resolveImport(choice: CollisionChoice): void {
  const pending = importStore.state.pending
  importStore.setState(() => ({ pending: undefined }))
  if (pending) void finish(pending, choice)
}

export function cancelImport(): void {
  importStore.setState(() => ({ pending: undefined }))
}

async function finish(
  pending: PendingImport,
  choice: CollisionChoice
): Promise<void> {
  const existing =
    pending.target === "current"
      ? sessionStore.state.files.map((file) => file.name)
      : []
  const plan = planImport(existing, pending.batch.files, choice)
  const summary = importSummary({
    imported: plan.added + plan.replaced,
    skipped: plan.skipped,
    ignored: pending.batch.ignored,
  })
  try {
    if (pending.target === "current") {
      await writeFiles(plan.writes)
    } else {
      const project = await createImportedProject(
        pending.projectName,
        plan.writes satisfies ReadonlyArray<ProjectFile>
      )
      pending.openProject(project.id)
    }
    toast(summary)
  } catch (error) {
    console.error("Import failed", error)
    toast.error("The import failed. Nothing was changed.")
  }
}

// The project store: every project in this browser, its files and the
// state of its window. One layout, whatever the backend:
//
//   projects/<id>/project.json   { schema: 1, name, createdAt, updatedAt, ui }
//   projects/<id>/files/<name>.kul
//
// The project list is the directory listing; there is no index to drift. A
// project exists once its project.json does, so it is written last.
//
// Only the main thread writes, and only the tab that owns a project
// (ownership.ts) writes to it. Within the tab, operations on one project
// run one at a time, in the order they were called.

import type { DirectoryPath, StorageBackend } from "./backend"
import { parseCanvasCamera } from "./canvas-camera"
import type { CanvasCamera } from "./canvas-camera"
import { parseEditorWindow } from "./editor-window"
import type { EditorWindowLayout } from "./editor-window"
import {
  characterCount,
  fileNameProblem,
  inFileOrder,
  sameFileName,
} from "./file-names"
import type { FileNameProblem } from "./file-names"

export const PROJECT_SCHEMA = 1

/** The longest project name, in characters. */
export const MAX_PROJECT_NAME_LENGTH = 100

export interface ProjectSummary {
  id: string
  name: string
  /** ISO 8601 timestamps. */
  createdAt: string
  /** When the project's name or files last changed. */
  updatedAt: string
}

export interface ProjectFile {
  name: string
  text: string
}

/** A project's window state, restored when it opens again. */
export interface ProjectUi {
  /** The editor window's layout; null until it is first moved. */
  editorWindow: EditorWindowLayout | null
  /** Open files, in tab order. */
  openTabs: Array<string>
  activeFile: string | null
  /** The canvas camera; null until the tree is first drawn. */
  camera: CanvasCamera | null
}

export const EMPTY_UI: ProjectUi = {
  editorWindow: null,
  openTabs: [],
  activeFile: null,
  camera: null,
}

export type StoreErrorCode =
  | "project-not-found"
  | "file-not-found"
  | "invalid-project-name"
  | FileNameProblem

export class StoreError extends Error {
  readonly code: StoreErrorCode

  constructor(code: StoreErrorCode, message: string) {
    super(message)
    this.name = "StoreError"
    this.code = code
  }
}

export interface ProjectStore {
  /** Every project, most recently edited first. */
  listProjects: () => Promise<Array<ProjectSummary>>
  /** One project, or `undefined` when it does not exist. */
  readProject: (id: string) => Promise<ProjectSummary | undefined>
  createProject: (
    name: string,
    files: ReadonlyArray<ProjectFile>,
    ui?: ProjectUi
  ) => Promise<ProjectSummary>
  renameProject: (id: string, name: string) => Promise<ProjectSummary>
  /** Copy a project's files and window state into a new project. */
  duplicateProject: (id: string, name: string) => Promise<ProjectSummary>
  deleteProject: (id: string) => Promise<void>
  /** A project's files, in file order. */
  readFiles: (id: string) => Promise<Array<ProjectFile>>
  /** Create a file or replace its text. */
  writeFile: (id: string, name: string, text: string) => Promise<void>
  /** Rename a file; open tabs and the active file follow it. */
  renameFile: (id: string, from: string, to: string) => Promise<void>
  /** Delete a file and close its tab. */
  deleteFile: (id: string, name: string) => Promise<void>
  readUi: (id: string) => Promise<ProjectUi>
  writeUi: (id: string, ui: ProjectUi) => Promise<void>
}

export interface ProjectStoreOptions {
  /** The current time as ISO 8601. */
  now?: () => string
  /** A new project id. */
  newId?: () => string
}

interface ProjectRecord {
  schema: typeof PROJECT_SCHEMA
  name: string
  createdAt: string
  updatedAt: string
  ui: ProjectUi
}

const PROJECTS: DirectoryPath = ["projects"]
const RECORD = "project.json"
const projectDir = (id: string): DirectoryPath => [...PROJECTS, id]
const filesDir = (id: string): DirectoryPath => [...PROJECTS, id, "files"]

/** The project store over `backend`. */
export function createProjectStore(
  backend: StorageBackend,
  options: ProjectStoreOptions = {}
): ProjectStore {
  const now = options.now ?? (() => new Date().toISOString())
  const newId = options.newId ?? (() => crypto.randomUUID())

  // One queue per project: each operation starts when the previous one on
  // the same project has settled.
  const queues = new Map<string, Promise<unknown>>()
  function serial<T>(id: string, operation: () => Promise<T>): Promise<T> {
    const previous = queues.get(id) ?? Promise.resolve()
    const next = previous.then(operation, operation)
    const settled = next.catch(() => undefined)
    queues.set(id, settled)
    void settled.then(() => {
      if (queues.get(id) === settled) queues.delete(id)
    })
    return next
  }

  async function readRecord(id: string): Promise<ProjectRecord | undefined> {
    const text = await backend.readText(projectDir(id), RECORD)
    return text === undefined ? undefined : parseRecord(text)
  }

  async function requireRecord(id: string): Promise<ProjectRecord> {
    const record = await readRecord(id)
    if (!record) {
      throw new StoreError("project-not-found", `No project ${id}`)
    }
    return record
  }

  function writeRecord(id: string, record: ProjectRecord): Promise<void> {
    return backend.writeText(
      projectDir(id),
      RECORD,
      `${JSON.stringify(record, null, 2)}\n`
    )
  }

  async function touch(id: string, record: ProjectRecord): Promise<void> {
    await writeRecord(id, { ...record, updatedAt: now() })
  }

  async function fileNames(id: string): Promise<Array<string>> {
    const names = await backend.listFiles(filesDir(id))
    return names.filter((name) => fileNameProblem(name, []) === undefined)
  }

  async function readAll(id: string): Promise<Array<ProjectFile>> {
    const files = await Promise.all(
      (await fileNames(id)).map(async (name) => ({
        name,
        text: (await backend.readText(filesDir(id), name)) ?? "",
      }))
    )
    return inFileOrder(files)
  }

  async function create(
    name: string,
    files: ReadonlyArray<ProjectFile>,
    ui: ProjectUi
  ): Promise<ProjectSummary> {
    const projectName = checkProjectName(name)
    const seen: Array<string> = []
    for (const file of files) {
      checkFileName(file.name, seen)
      seen.push(file.name)
    }
    const id = newId()
    for (const file of files) {
      await backend.writeText(filesDir(id), file.name, file.text)
    }
    const at = now()
    const record: ProjectRecord = {
      schema: PROJECT_SCHEMA,
      name: projectName,
      createdAt: at,
      updatedAt: at,
      ui: keepFiles(ui, seen),
    }
    await writeRecord(id, record)
    return summary(id, record)
  }

  return {
    async listProjects() {
      const ids = await backend.listDirectories(PROJECTS)
      const projects = await Promise.all(
        ids.map(async (id) => {
          const record = await readRecord(id)
          return record && summary(id, record)
        })
      )
      return projects
        .filter((project) => project !== undefined)
        .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))
    },

    async readProject(id) {
      const record = await readRecord(id)
      return record && summary(id, record)
    },

    createProject: (name, files, ui = EMPTY_UI) => create(name, files, ui),

    renameProject: (id, name) =>
      serial(id, async () => {
        const projectName = checkProjectName(name)
        const record = await requireRecord(id)
        const next = { ...record, name: projectName, updatedAt: now() }
        await writeRecord(id, next)
        return summary(id, next)
      }),

    duplicateProject: (id, name) =>
      serial(id, async () => {
        const record = await requireRecord(id)
        return create(name, await readAll(id), record.ui)
      }),

    deleteProject: (id) =>
      serial(id, () => backend.removeDirectory(projectDir(id))),

    readFiles: (id) =>
      serial(id, async () => {
        await requireRecord(id)
        return readAll(id)
      }),

    writeFile: (id, name, text) =>
      serial(id, async () => {
        const record = await requireRecord(id)
        const names = await fileNames(id)
        const existing = names.find((other) => sameFileName(other, name))
        if (existing !== name) checkFileName(name, names)
        await backend.writeText(filesDir(id), name, text)
        await touch(id, record)
      }),

    renameFile: (id, from, to) =>
      serial(id, async () => {
        const record = await requireRecord(id)
        const names = await fileNames(id)
        if (!names.includes(from)) {
          throw new StoreError("file-not-found", `No file ${from}`)
        }
        checkFileName(to, names, from)
        if (to !== from) {
          const text = (await backend.readText(filesDir(id), from)) ?? ""
          if (sameFileName(from, to)) {
            // A change of case only: go through a name that cannot clash,
            // in case the storage underneath ignores case.
            const scratch = `${crypto.randomUUID()}.renaming`
            await backend.writeText(filesDir(id), scratch, text)
            await backend.removeFile(filesDir(id), from)
            await backend.writeText(filesDir(id), to, text)
            await backend.removeFile(filesDir(id), scratch)
          } else {
            await backend.writeText(filesDir(id), to, text)
            await backend.removeFile(filesDir(id), from)
          }
        }
        const rename = (name: string) => (name === from ? to : name)
        await touch(id, {
          ...record,
          ui: {
            ...record.ui,
            openTabs: record.ui.openTabs.map(rename),
            activeFile:
              record.ui.activeFile === null
                ? null
                : rename(record.ui.activeFile),
          },
        })
      }),

    deleteFile: (id, name) =>
      serial(id, async () => {
        const record = await requireRecord(id)
        if (!(await fileNames(id)).includes(name)) {
          throw new StoreError("file-not-found", `No file ${name}`)
        }
        await backend.removeFile(filesDir(id), name)
        const openTabs = record.ui.openTabs.filter((tab) => tab !== name)
        await touch(id, {
          ...record,
          ui: {
            ...record.ui,
            openTabs,
            activeFile:
              record.ui.activeFile === name
                ? (openTabs[0] ?? null)
                : record.ui.activeFile,
          },
        })
      }),

    readUi: (id) => serial(id, async () => (await requireRecord(id)).ui),

    writeUi: (id, ui) =>
      serial(id, async () => {
        const record = await requireRecord(id)
        await writeRecord(id, {
          ...record,
          ui: keepFiles(ui, await fileNames(id)),
        })
      }),
  }
}

function summary(id: string, record: ProjectRecord): ProjectSummary {
  return {
    id,
    name: record.name,
    createdAt: record.createdAt,
    updatedAt: record.updatedAt,
  }
}

/** The trimmed project name, or a StoreError when it can't be used. */
export function checkProjectName(name: string): string {
  const trimmed = name.trim()
  if (trimmed === "" || characterCount(trimmed) > MAX_PROJECT_NAME_LENGTH) {
    throw new StoreError(
      "invalid-project-name",
      `A project name has 1 to ${MAX_PROJECT_NAME_LENGTH} characters`
    )
  }
  return trimmed
}

function checkFileName(
  name: string,
  existing: Iterable<string>,
  renaming?: string
): void {
  const problem = fileNameProblem(name, existing, renaming)
  if (problem) throw new StoreError(problem, `Cannot use the name ${name}`)
}

/** `ui` with every reference to a file the project lacks dropped. */
function keepFiles(ui: ProjectUi, names: ReadonlyArray<string>): ProjectUi {
  const openTabs = ui.openTabs.filter(
    (tab, index) => names.includes(tab) && ui.openTabs.indexOf(tab) === index
  )
  const activeFile =
    ui.activeFile !== null && names.includes(ui.activeFile)
      ? ui.activeFile
      : null
  return { ...ui, openTabs, activeFile }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function parseUi(value: unknown): ProjectUi {
  if (!isRecord(value)) return EMPTY_UI
  const openTabs = Array.isArray(value.openTabs)
    ? value.openTabs.filter((tab): tab is string => typeof tab === "string")
    : []
  return {
    editorWindow: parseEditorWindow(value.editorWindow),
    openTabs,
    activeFile: typeof value.activeFile === "string" ? value.activeFile : null,
    camera: parseCanvasCamera(value.camera),
  }
}

/** A project.json's contents, or `undefined` when it isn't one. */
function parseRecord(text: string): ProjectRecord | undefined {
  let value: unknown
  try {
    value = JSON.parse(text)
  } catch {
    return undefined
  }
  if (
    !isRecord(value) ||
    value.schema !== PROJECT_SCHEMA ||
    typeof value.name !== "string" ||
    typeof value.createdAt !== "string" ||
    typeof value.updatedAt !== "string"
  ) {
    return undefined
  }
  return {
    schema: PROJECT_SCHEMA,
    name: value.name,
    createdAt: value.createdAt,
    updatedAt: value.updatedAt,
    ui: parseUi(value.ui),
  }
}

// The projects in this browser: the list the project switcher shows, and
// the operations that make, copy and remove whole projects. Every tab keeps
// its list current through a BroadcastChannel.

import { createStore } from "@tanstack/react-store"

import { EXAMPLES, FIRST_RUN_EXAMPLE, findExample } from "@/project/examples"
import type { Example } from "@/project/examples"
import { readLastProject } from "@/store/last-project"
import { openProjectStore } from "@/store/open-store"
import { EMPTY_UI } from "@/store/project-store"
import { takeHandedOffSnippet } from "@/store/try-it"
import type { TryItSnippet } from "@/store/try-it"
import type {
  ProjectFile,
  ProjectStore,
  ProjectSummary,
  ProjectUi,
} from "@/store/project-store"

import { setStorageSaved } from "./durability-state"

export interface ProjectsState {
  /** Every project, most recently edited first; undefined until read. */
  projects: ReadonlyArray<ProjectSummary> | undefined
}

export const projectsStore = createStore<ProjectsState>({
  projects: undefined,
})

const CHANNEL = "kul-projects"
let channel: BroadcastChannel | undefined

/** Open the store and start following other tabs' project changes. */
export async function connectProjects(): Promise<ProjectStore> {
  const { store, saved } = await openProjectStore()
  setStorageSaved(saved)
  if (!channel && typeof BroadcastChannel !== "undefined") {
    channel = new BroadcastChannel(CHANNEL)
    channel.onmessage = () => void refreshProjects()
  }
  return store
}

export async function refreshProjects(): Promise<void> {
  const store = await connectProjects()
  const projects = await store.listProjects()
  projectsStore.setState(() => ({ projects }))
}

/** Re-read the list here and tell the other tabs to. */
export async function projectsChanged(): Promise<void> {
  channel?.postMessage("changed")
  await refreshProjects()
}

/**
 * `name`, or `name 2`, `name 3`, … whichever no project has yet, so
 * projects stay easy to tell apart in the switcher.
 */
export function uniqueProjectName(
  name: string,
  taken: ReadonlyArray<string>
): string {
  if (!taken.includes(name)) return name
  for (let n = 2; ; n++) {
    const candidate = `${name} ${n}`
    if (!taken.includes(candidate)) return candidate
  }
}

async function takenNames(store: ProjectStore): Promise<Array<string>> {
  return (await store.listProjects()).map((project) => project.name)
}

function uiOpening(files: ReadonlyArray<ProjectFile>): ProjectUi {
  const first = files[0]?.name
  return first
    ? { ...EMPTY_UI, openTabs: [first], activeFile: first }
    : EMPTY_UI
}

async function create(
  name: string,
  files: ReadonlyArray<ProjectFile>
): Promise<ProjectSummary> {
  const store = await connectProjects()
  const project = await store.createProject(
    uniqueProjectName(name, await takenNames(store)),
    files,
    uiOpening(files)
  )
  await projectsChanged()
  return project
}

export function createProjectFromExample(
  example: Example
): Promise<ProjectSummary> {
  return create(example.projectName, example.files)
}

/** A new project with one empty file to start writing in. */
export function createBlankProject(): Promise<ProjectSummary> {
  return create("Untitled project", [{ name: "family.kul", text: "" }])
}

/**
 * A new project holding a snippet's files from the docs ("Try it"), named
 * after the page it came from.
 */
export function createScratchProject(
  snippet: TryItSnippet
): Promise<ProjectSummary> {
  return create(`Try it: ${snippet.page}`, snippet.files)
}

export function createImportedProject(
  name: string,
  files: ReadonlyArray<ProjectFile>
): Promise<ProjectSummary> {
  return create(name, files)
}

export async function duplicateProject(
  project: ProjectSummary
): Promise<ProjectSummary> {
  const store = await connectProjects()
  const copy = await store.duplicateProject(
    project.id,
    uniqueProjectName(`${project.name} (copy)`, await takenNames(store))
  )
  await projectsChanged()
  return copy
}

export async function deleteProject(id: string): Promise<void> {
  const store = await connectProjects()
  await store.deleteProject(id)
  await projectsChanged()
}

let arriving: Promise<ProjectSummary> | undefined

/**
 * The project /app opens: a new one holding the snippet a docs page handed
 * over, if any; else the one opened last, else the latest edited. On a
 * first visit there is none, and the first-run example becomes one.
 */
export function startProject(): Promise<ProjectSummary> {
  // The snippet is taken once, so a second call while the first is still
  // creating its project answers with that same project.
  arriving ??= pickStartProject().finally(() => {
    arriving = undefined
  })
  return arriving
}

async function pickStartProject(): Promise<ProjectSummary> {
  const snippet = takeHandedOffSnippet()
  if (snippet) return createScratchProject(snippet)
  const store = await connectProjects()
  const pick = async () => {
    const projects = await store.listProjects()
    const last = readLastProject()
    return projects.find((project) => project.id === last) ?? projects[0]
  }
  const existing = await pick()
  if (existing) return existing
  // Two tabs opening Kul for the first time at once seed one project.
  const seed = async () => {
    const again = await pick()
    if (again) return again
    const example = findExample(FIRST_RUN_EXAMPLE) ?? EXAMPLES[0]
    if (!example) throw new Error("The app was built without examples")
    return createProjectFromExample(example)
  }
  if (typeof navigator.locks === "undefined") return seed()
  return navigator.locks.request("kul-first-run", seed)
}

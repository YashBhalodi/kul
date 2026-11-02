// The open project: its files and window state, this tab's access to it,
// and every change the app makes to it. Changes show at once and are
// written through the save queue; only the tab that owns the project
// writes, and the others reload when it does.

import { Debouncer } from "@tanstack/pacer"
import { createStore } from "@tanstack/react-store"

import { inFileOrder } from "@/store/file-names"
import { rememberLastProject } from "@/store/last-project"
import { claimProject } from "@/store/ownership"
import type { Access, ProjectClaim } from "@/store/ownership"
import { EMPTY_UI } from "@/store/project-store"
import type {
  ProjectFile,
  ProjectStore,
  ProjectSummary,
  ProjectUi,
} from "@/store/project-store"

import { noteEdit } from "./durability-state"
import {
  connectProjects,
  projectsChanged,
  projectsStore,
  refreshProjects,
} from "./projects"
import { createSaveQueue } from "./save-queue"
import type { SaveQueue, SaveStatus } from "./save-queue"

export interface SessionState {
  status: "closed" | "opening" | "missing" | "open"
  projectId: string | undefined
  project: ProjectSummary | undefined
  /** The project's files, in file order. */
  files: ReadonlyArray<ProjectFile>
  ui: ProjectUi
  access: Access
  save: SaveStatus
}

const CLOSED: SessionState = {
  status: "closed",
  projectId: undefined,
  project: undefined,
  files: [],
  ui: EMPTY_UI,
  access: "owner",
  save: "idle",
}

export const sessionStore = createStore<SessionState>(CLOSED)

interface Session {
  id: string
  store: ProjectStore
  claim: ProjectClaim
  saves: SaveQueue
  /** Writes the window state once it stops changing. */
  uiWriter: Debouncer<() => void>
  disconnect: () => void
}

/** How long window-state changes (moving the editor, say) wait to be written. */
const UI_WRITE_WAIT_MS = 300

let session: Session | undefined
// Opening and closing run one at a time, in order.
let transition: Promise<void> = Promise.resolve()

function update(change: (state: SessionState) => Partial<SessionState>) {
  sessionStore.setState((state) => ({ ...state, ...change(state) }))
}

/** Open the project `id` in this tab, closing whichever was open. */
export function openProject(id: string): Promise<void> {
  transition = transition.then(() => open(id)).catch(reportFailure)
  return transition
}

/** Close the open project, writing anything unsaved first. */
export function closeProject(): Promise<void> {
  transition = transition.then(close).catch(reportFailure)
  return transition
}

function reportFailure(error: unknown) {
  console.error("Kul could not open the project", error)
  update(() => ({ status: "missing" }))
}

async function open(id: string): Promise<void> {
  if (session?.id === id) return
  await close()
  update(() => ({ ...CLOSED, status: "opening", projectId: id }))
  const store = await connectProjects()
  const project = await store.readProject(id)
  if (!project) {
    update(() => ({ status: "missing" }))
    return
  }
  rememberLastProject(id)
  void refreshProjects()
  const claim = await claimProject(id)
  const saves = createSaveQueue({
    writeTexts: async (texts) => {
      for (const [name, text] of texts) await store.writeFile(id, name, text)
      claim.announceChange()
    },
    onStatus: (save) => update(() => ({ save })),
  })

  const uiWriter = new Debouncer(
    () => {
      if (claim.access !== "owner") return
      const ui = sessionStore.state.ui
      void saves.run(() => store.writeUi(id, ui)).catch(() => undefined)
    },
    { wait: UI_WRITE_WAIT_MS }
  )
  const flush = () => {
    uiWriter.flush()
    void saves.flush()
  }
  const onVisibility = () => {
    if (document.visibilityState === "hidden") flush()
  }
  document.addEventListener("visibilitychange", onVisibility)
  window.addEventListener("pagehide", flush)
  const stopAccess = claim.onAccessChange((access) => {
    // A tab that lost the project drops what it had not written yet: the
    // new owner has already read the files.
    if (access === "read-only") saves.discard()
    update(() => ({ access }))
    void reload()
  })
  const stopRemote = claim.onRemoteChange(() => void reload())
  // Another tab may delete the project; the project list says so.
  const listing = projectsStore.subscribe(({ projects }) => {
    if (projects && !projects.some((listed) => listed.id === id)) {
      void reload()
    }
  })

  session = {
    id,
    store,
    claim,
    saves,
    uiWriter,
    disconnect: () => {
      document.removeEventListener("visibilitychange", onVisibility)
      window.removeEventListener("pagehide", flush)
      stopAccess()
      stopRemote()
      listing.unsubscribe()
    },
  }
  const [files, ui] = await Promise.all([store.readFiles(id), store.readUi(id)])
  update(() => ({
    status: "open",
    project,
    files,
    ui,
    access: claim.access,
    save: "idle",
  }))
}

async function close(): Promise<void> {
  const closing = session
  if (!closing) return
  session = undefined
  closing.disconnect()
  if (closing.claim.access === "owner") {
    closing.uiWriter.flush()
    await closing.saves.flush()
  } else {
    closing.uiWriter.cancel()
    closing.saves.discard()
  }
  await closing.claim.release()
  sessionStore.setState(() => CLOSED)
}

/** Read the open project again, as another tab left it. */
async function reload(): Promise<void> {
  const current = session
  if (!current) return
  const project = await current.store.readProject(current.id)
  if (session !== current) return
  if (!project) {
    update(() => ({ status: "missing", project: undefined, files: [] }))
    return
  }
  const [files, ui] = await Promise.all([
    current.store.readFiles(current.id),
    current.store.readUi(current.id),
  ])
  if (session !== current) return
  update(() => ({ project, files, ui }))
}

/** Take the open project over from the tab that owns it. */
export async function takeOverProject(): Promise<void> {
  const current = session
  if (!current) return
  await current.claim.takeOver()
  await reload()
}

/** Write everything unsaved now. */
export async function flushProject(): Promise<void> {
  session?.uiWriter.flush()
  await session?.saves.flush()
}

/**
 * Run a change to the open project: show it, then write it. Does nothing
 * unless this tab owns the project.
 */
function change(
  show: (state: SessionState) => Partial<SessionState>,
  write: (store: ProjectStore, id: string) => Promise<void>
): Promise<void> {
  const current = session
  if (!current || current.claim.access !== "owner") return Promise.resolve()
  update(show)
  noteEdit()
  return current.saves.run(async () => {
    await write(current.store, current.id)
    current.claim.announceChange()
  })
}

function withFile(
  files: ReadonlyArray<ProjectFile>,
  file: ProjectFile
): Array<ProjectFile> {
  return inFileOrder([...files.filter((f) => f.name !== file.name), file])
}

function openTab(ui: ProjectUi, name: string): ProjectUi {
  return {
    ...ui,
    openTabs: ui.openTabs.includes(name) ? ui.openTabs : [...ui.openTabs, name],
    activeFile: name,
  }
}

/** Add a file and make it the active one. */
export function createFile(name: string, text = ""): Promise<void> {
  return change(
    (state) => ({
      files: withFile(state.files, { name, text }),
      ui: openTab(state.ui, name),
    }),
    async (store, id) => {
      await store.writeFile(id, name, text)
      await store.writeUi(id, openTab(await store.readUi(id), name))
    }
  )
}

/** Record a file's new text; it is written once typing pauses. */
export function setFileText(name: string, text: string): void {
  const current = session
  if (!current || current.claim.access !== "owner") return
  update((state) => ({ files: withFile(state.files, { name, text }) }))
  noteEdit()
  current.saves.change(name, text)
}

export function renameFile(from: string, to: string): Promise<void> {
  const rename = (name: string) => (name === from ? to : name)
  return change(
    (state) => ({
      files: inFileOrder(
        state.files.map((file) => ({ ...file, name: rename(file.name) }))
      ),
      ui: {
        ...state.ui,
        openTabs: state.ui.openTabs.map(rename),
        activeFile: state.ui.activeFile && rename(state.ui.activeFile),
      },
    }),
    (store, id) => store.renameFile(id, from, to)
  )
}

export function deleteFile(name: string): Promise<void> {
  return change(
    (state) => {
      const openTabs = state.ui.openTabs.filter((tab) => tab !== name)
      return {
        files: state.files.filter((file) => file.name !== name),
        ui: {
          ...state.ui,
          openTabs,
          activeFile:
            state.ui.activeFile === name
              ? (openTabs[0] ?? null)
              : state.ui.activeFile,
        },
      }
    },
    (store, id) => store.deleteFile(id, name)
  )
}

/** Write imported files, replacing any with the same name. */
export function writeFiles(files: ReadonlyArray<ProjectFile>): Promise<void> {
  return change(
    (state) => ({
      files: files.reduce<ReadonlyArray<ProjectFile>>(withFile, state.files),
    }),
    async (store, id) => {
      for (const file of files) await store.writeFile(id, file.name, file.text)
    }
  )
}

/** Make a file the active one, opening its tab. */
export function selectFile(name: string): Promise<void> {
  // Window state is this tab's view; a read-only tab keeps it to itself.
  return writeUiNow(openTab(sessionStore.state.ui, name))
}

/** Close a file's tab; the next tab along becomes the active one. */
export function closeTab(name: string): Promise<void> {
  const { ui } = sessionStore.state
  const index = ui.openTabs.indexOf(name)
  if (index === -1) return Promise.resolve()
  const openTabs = ui.openTabs.filter((tab) => tab !== name)
  const activeFile =
    ui.activeFile === name
      ? (openTabs[Math.min(index, openTabs.length - 1)] ?? null)
      : ui.activeFile
  return writeUiNow({ ...ui, openTabs, activeFile })
}

/**
 * Change the window state (the editor's layout, say). It shows at once and
 * is written once it stops changing; a read-only tab keeps it to itself.
 */
export function updateUi(change: (ui: ProjectUi) => ProjectUi): void {
  const current = session
  if (!current) return
  update((state) => ({ ui: change(state.ui) }))
  current.uiWriter.maybeExecute()
}

function writeUiNow(ui: ProjectUi): Promise<void> {
  const current = session
  if (!current) return Promise.resolve()
  update(() => ({ ui }))
  if (current.claim.access !== "owner") return Promise.resolve()
  current.uiWriter.cancel()
  return current.saves.run(() => current.store.writeUi(current.id, ui))
}

export async function renameProject(name: string): Promise<void> {
  await change(
    (state) => ({
      project: state.project && { ...state.project, name: name.trim() },
    }),
    async (store, id) => {
      const project = await store.renameProject(id, name)
      update(() => ({ project }))
    }
  )
  await projectsChanged()
}

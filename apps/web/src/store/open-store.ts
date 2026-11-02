// The app's one project store. It lives in the origin private file system
// when the browser can write there; otherwise (an older browser, or a
// private window that refuses storage) it lives in memory, and the app says
// that nothing outlasts the tab.

import type { StorageBackend } from "./backend"
import { createMemoryBackend } from "./memory-backend"
import { createOpfsBackend } from "./opfs-backend"
import { createProjectStore } from "./project-store"
import type { ProjectStore } from "./project-store"

export interface OpenedStore {
  store: ProjectStore
  /** Whether projects outlast the tab. */
  saved: boolean
}

const PROBE = ".kul-probe"

async function openOpfs(): Promise<StorageBackend> {
  const root = await navigator.storage.getDirectory()
  const backend = createOpfsBackend(root)
  // Some browsers open the directory but cannot write to it from the main
  // thread; find out before any work depends on it.
  await backend.writeText([], PROBE, "ok")
  if ((await backend.readText([], PROBE)) !== "ok") {
    throw new Error("The origin private file system did not keep a write")
  }
  await backend.removeFile([], PROBE)
  return backend
}

let opened: Promise<OpenedStore> | undefined

export function openProjectStore(): Promise<OpenedStore> {
  opened ??= openOpfs().then(
    (backend) => ({ store: createProjectStore(backend), saved: true }),
    () => ({ store: createProjectStore(createMemoryBackend()), saved: false })
  )
  return opened
}

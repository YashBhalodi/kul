import { afterAll } from "vitest"

import { createOpfsBackend } from "./opfs-backend"
import { projectStoreContract } from "./project-store.contract"

// Each test gets a fresh folder inside the origin's real OPFS.
const RUN = `contract-${crypto.randomUUID()}`

async function testRoot(): Promise<FileSystemDirectoryHandle> {
  const origin = await navigator.storage.getDirectory()
  const run = await origin.getDirectoryHandle(RUN, { create: true })
  return run.getDirectoryHandle(crypto.randomUUID(), { create: true })
}

afterAll(async () => {
  const origin = await navigator.storage.getDirectory()
  await origin.removeEntry(RUN, { recursive: true })
})

projectStoreContract("the origin private file system", async () =>
  createOpfsBackend(await testRoot())
)

// The origin private file system (OPFS) as a storage backend, through the
// asynchronous main-thread API: small text files need nothing faster.

import type { DirectoryPath, StorageBackend } from "./backend"

function isNotFound(error: unknown): boolean {
  return (
    error instanceof DOMException &&
    (error.name === "NotFoundError" || error.name === "TypeMismatchError")
  )
}

/** A backend rooted at `root`: the origin's OPFS root, or a folder in it. */
export function createOpfsBackend(
  root: FileSystemDirectoryHandle
): StorageBackend {
  async function find(
    path: DirectoryPath
  ): Promise<FileSystemDirectoryHandle | undefined> {
    let dir = root
    try {
      for (const name of path) dir = await dir.getDirectoryHandle(name)
    } catch (error) {
      if (isNotFound(error)) return undefined
      throw error
    }
    return dir
  }

  async function ensure(
    path: DirectoryPath
  ): Promise<FileSystemDirectoryHandle> {
    let dir = root
    for (const name of path) {
      dir = await dir.getDirectoryHandle(name, { create: true })
    }
    return dir
  }

  async function list(
    path: DirectoryPath,
    kind: FileSystemHandleKind
  ): Promise<Array<string>> {
    const dir = await find(path)
    if (!dir) return []
    const names: Array<string> = []
    for await (const [name, handle] of dir.entries()) {
      if (handle.kind === kind) names.push(name)
    }
    return names
  }

  return {
    listDirectories: (path) => list(path, "directory"),
    listFiles: (path) => list(path, "file"),

    async readText(path, name) {
      const dir = await find(path)
      if (!dir) return undefined
      try {
        const handle = await dir.getFileHandle(name)
        return await (await handle.getFile()).text()
      } catch (error) {
        if (isNotFound(error)) return undefined
        throw error
      }
    },

    async writeText(path, name, text) {
      const dir = await ensure(path)
      const handle = await dir.getFileHandle(name, { create: true })
      // A writable stream writes to a swap file and replaces the file on
      // close, so a reader never sees half a write.
      const writable = await handle.createWritable()
      try {
        await writable.write(text)
        await writable.close()
      } catch (error) {
        await writable.abort().catch(() => undefined)
        throw error
      }
    },

    async removeFile(path, name) {
      const dir = await find(path)
      try {
        await dir?.removeEntry(name)
      } catch (error) {
        if (!isNotFound(error)) throw error
      }
    },

    async removeDirectory(path) {
      const parent = await find(path.slice(0, -1))
      const last = path.at(-1)
      if (!parent || last === undefined) return
      try {
        await parent.removeEntry(last, { recursive: true })
      } catch (error) {
        if (!isNotFound(error)) throw error
      }
    },
  }
}

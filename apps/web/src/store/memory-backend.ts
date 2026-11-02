import type { DirectoryPath, StorageBackend } from "./backend"

interface Directory {
  directories: Map<string, Directory>
  files: Map<string, string>
}

function emptyDirectory(): Directory {
  return { directories: new Map(), files: new Map() }
}

/** A storage backend that lives in memory and ends with the page. */
export function createMemoryBackend(): StorageBackend {
  const root = emptyDirectory()

  function find(path: DirectoryPath): Directory | undefined {
    let dir: Directory | undefined = root
    for (const name of path) dir = dir?.directories.get(name)
    return dir
  }

  function ensure(path: DirectoryPath): Directory {
    let dir = root
    for (const name of path) {
      let next = dir.directories.get(name)
      if (!next) {
        next = emptyDirectory()
        dir.directories.set(name, next)
      }
      dir = next
    }
    return dir
  }

  return {
    listDirectories: (path) =>
      Promise.resolve([...(find(path)?.directories.keys() ?? [])]),
    listFiles: (path) => Promise.resolve([...(find(path)?.files.keys() ?? [])]),
    readText: (path, name) => Promise.resolve(find(path)?.files.get(name)),
    writeText: (path, name, text) => {
      ensure(path).files.set(name, text)
      return Promise.resolve()
    },
    removeFile: (path, name) => {
      find(path)?.files.delete(name)
      return Promise.resolve()
    },
    removeDirectory: (path) => {
      const parent = find(path.slice(0, -1))
      const last = path.at(-1)
      if (parent && last !== undefined) parent.directories.delete(last)
      return Promise.resolve()
    },
  }
}

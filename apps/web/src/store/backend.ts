// Where the project store keeps its bytes: a tree of directories holding
// text files. The browser's origin private file system is the real one
// (opfs-backend.ts); memory-backend.ts holds the same tree in memory for a
// window that cannot store files, and for tests that run in Node.

/** A directory, as the names of the directories leading to it. */
export type DirectoryPath = ReadonlyArray<string>

export interface StorageBackend {
  /** The directories directly inside `dir`; none when `dir` is missing. */
  listDirectories: (dir: DirectoryPath) => Promise<Array<string>>
  /** The files directly inside `dir`; none when `dir` is missing. */
  listFiles: (dir: DirectoryPath) => Promise<Array<string>>
  /** The text of a file, or `undefined` when it does not exist. */
  readText: (dir: DirectoryPath, name: string) => Promise<string | undefined>
  /** Create or replace a file, creating its directories as needed. */
  writeText: (dir: DirectoryPath, name: string, text: string) => Promise<void>
  /** Remove a file; removing a missing file does nothing. */
  removeFile: (dir: DirectoryPath, name: string) => Promise<void>
  /** Remove a directory and everything in it; a missing one is fine. */
  removeDirectory: (dir: DirectoryPath) => Promise<void>
}

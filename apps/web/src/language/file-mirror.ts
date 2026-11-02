// The main thread's copy of the file set the worker holds: every file's
// latest text and the version it was sent with. It decides whether an
// answer is current, and it is what a restarted worker is given back.

import type { FileVersion } from "kul-wasm"

import type { FileVersions, VersionedFile } from "./protocol"

export class FileMirror {
  #files = new Map<string, VersionedFile>()
  #nextVersion = 1

  /** Record a file's text; returns the version it is sent with. */
  set(name: string, text: string): number {
    const version = this.#nextVersion++
    this.#files.set(name, { name, text, version })
    return version
  }

  delete(name: string): boolean {
    return this.#files.delete(name)
  }

  /** Replace the whole set; returns the files with their new versions. */
  replace(
    files: ReadonlyArray<{ name: string; text: string }>
  ): Array<VersionedFile> {
    this.#files.clear()
    for (const file of files) this.set(file.name, file.text)
    return this.snapshot()
  }

  get(name: string): VersionedFile | undefined {
    return this.#files.get(name)
  }

  /** The latest version of `name`, or undefined when there is no such file. */
  version(name: string): number | undefined {
    return this.#files.get(name)?.version
  }

  /** Every file, as last sent. */
  snapshot(): Array<VersionedFile> {
    return [...this.#files.values()]
  }

  /** Whether an answer computed from `versions` reflects every edit sent. */
  isCurrent(versions: FileVersions): boolean {
    const names = Object.keys(versions)
    if (names.length !== this.#files.size) return false
    return names.every(
      (name) => this.#files.get(name)?.version === versions[name]
    )
  }

  /** Whether an analysis of `files` reflects every edit sent. */
  isCurrentAnalysis(files: ReadonlyArray<FileVersion>): boolean {
    return this.isCurrent(versionsOf(files))
  }
}

export function versionsOf(files: ReadonlyArray<FileVersion>): FileVersions {
  return Object.fromEntries(files.map((file) => [file.name, file.version]))
}

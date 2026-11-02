import { createStore } from "@tanstack/react-store"

/**
 * What the file list is asking for: a name for a new file, a new name for
 * a file, or confirmation to delete one.
 */
export interface FileListState {
  creating?: boolean
  renaming?: string
  deleting?: string
}

export const fileListStore = createStore<FileListState>({})

export function startNewFile(): void {
  fileListStore.setState(() => ({ creating: true }))
}

export function startRenamingFile(name: string): void {
  fileListStore.setState(() => ({ renaming: name }))
}

export function askToDeleteFile(name: string): void {
  fileListStore.setState(() => ({ deleting: name }))
}

export function endFileListAction(): void {
  fileListStore.setState(() => ({}))
}

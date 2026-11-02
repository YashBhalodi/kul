import { useEffect, useRef } from "react"

import { readFileList, startImport } from "./import-flow"
import type { ImportTarget } from "./import-flow"
import { useOpenProject } from "./use-open-project"

export type PickKind = "files" | "folder"

let pick: ((kind: PickKind, target: ImportTarget) => void) | undefined

/** Open the browser's picker for files (and .zip archives) or a folder. */
export function pickImport(kind: PickKind, target: ImportTarget): void {
  pick?.(kind, target)
}

/** The hidden file inputs behind `pickImport`. Mount once, in the app. */
export function ImportPicker() {
  const openProject = useOpenProject()
  const files = useRef<HTMLInputElement>(null)
  const folder = useRef<HTMLInputElement>(null)
  const target = useRef<ImportTarget>("current")

  useEffect(() => {
    pick = (kind, to) => {
      target.current = to
      const input = kind === "files" ? files.current : folder.current
      if (!input) return
      input.value = ""
      input.click()
    }
    return () => {
      pick = undefined
    }
  }, [])

  const onChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const picked = [...(event.currentTarget.files ?? [])]
    if (picked.length === 0) return
    void readFileList(picked).then((read) =>
      startImport(read, target.current, openProject)
    )
  }

  return (
    <>
      <input
        ref={files}
        type="file"
        multiple
        accept=".kul,.zip,application/zip"
        hidden
        data-testid="import-files-input"
        onChange={onChange}
      />
      <input
        ref={folder}
        type="file"
        // A folder picker: every file inside it, with its path.
        {...{ webkitdirectory: "" }}
        hidden
        data-testid="import-folder-input"
        onChange={onChange}
      />
    </>
  )
}

import { useSelector } from "@tanstack/react-store"
import { UploadIcon } from "lucide-react"
import { useRef, useState } from "react"
import type { ReactNode } from "react"

import { carriesFiles, readDataTransfer, startImport } from "./import-flow"
import { sessionStore } from "./project-session"
import { useOpenProject } from "./use-open-project"

/**
 * Files, folders and .zip archives dropped anywhere on the canvas are
 * imported into the open project, or as a new project when this tab
 * can't change it.
 */
export function ImportDropZone({ children }: { children: ReactNode }) {
  const openProject = useOpenProject()
  const name = useSelector(sessionStore, (state) => state.project?.name)
  const writable = useSelector(
    sessionStore,
    (state) => state.status === "open" && state.access === "owner"
  )
  const [over, setOver] = useState(false)
  // dragenter and dragleave fire for every child crossed; count them.
  const depth = useRef(0)

  return (
    <div
      className="relative flex-1"
      onDragEnter={(event) => {
        if (!carriesFiles(event.dataTransfer)) return
        depth.current++
        setOver(true)
      }}
      onDragLeave={() => {
        depth.current = Math.max(0, depth.current - 1)
        if (depth.current === 0) setOver(false)
      }}
      onDragOver={(event) => {
        if (!carriesFiles(event.dataTransfer)) return
        event.preventDefault()
        event.dataTransfer.dropEffect = "copy"
      }}
      onDrop={(event) => {
        if (!carriesFiles(event.dataTransfer)) return
        event.preventDefault()
        depth.current = 0
        setOver(false)
        void readDataTransfer(event.dataTransfer).then((picked) =>
          startImport(picked, writable ? "current" : "new", openProject)
        )
      }}
    >
      {children}
      {over && (
        <div
          data-testid="drop-overlay"
          className="pointer-events-none absolute inset-3 z-30 flex items-center justify-center rounded-2xl border-2 border-dashed border-primary bg-background/80 backdrop-blur-sm"
        >
          <div className="flex flex-col items-center gap-2 text-center">
            <UploadIcon className="size-6 text-primary" aria-hidden />
            <p className="text-base font-medium">
              {writable && name
                ? `Drop to import into ${name}`
                : "Drop to import as a new project"}
            </p>
            <p className="text-sm text-muted-foreground">
              .kul files, folders and .zip archives. Other files are ignored.
            </p>
          </div>
        </div>
      )}
    </div>
  )
}

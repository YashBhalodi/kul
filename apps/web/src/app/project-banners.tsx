import { useSelector } from "@tanstack/react-store"
import { DownloadIcon, TriangleAlertIcon, XIcon } from "lucide-react"
import type { ReactNode } from "react"

import { Button } from "@/components/ui/button"
import { downloadProject } from "./download"
import { canvasSizeStore, currentLayout } from "./editor/editor-layout"
import { freeCanvasInsets } from "./editor/window-geometry"
import { sessionStore } from "./project-session"
import { dismissReminder, durabilityStore } from "./durability-state"

/**
 * Notices about the open project's storage, at the top of the canvas: that
 * this window can't store files, or that the browser may clear them. (The
 * editor window says when another tab owns the project.)
 */
export function ProjectBanners() {
  const { storageSaved, reminderVisible } = useSelector(durabilityStore)
  const editorWindow = useSelector(
    sessionStore,
    (state) => state.status === "open" && state.ui.editorWindow
  )
  const canvas = useSelector(canvasSizeStore)
  // Beside the editor window, never over it.
  const insets =
    editorWindow === false
      ? { left: 0, right: 0 }
      : freeCanvasInsets(currentLayout(editorWindow, canvas), canvas)
  return (
    <div
      style={{ left: insets.left, right: insets.right }}
      className="pointer-events-none absolute top-4 z-20 flex flex-col items-center gap-2 px-4"
    >
      {!storageSaved && (
        <Banner
          testId="storage-banner"
          icon={<TriangleAlertIcon />}
          actions={<BackupButton />}
        >
          This browser window can’t save files. Download a backup before you
          close it.
        </Banner>
      )}
      {storageSaved && reminderVisible && (
        <Banner
          testId="persistence-banner"
          icon={<TriangleAlertIcon />}
          actions={
            <>
              <BackupButton />
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label="Dismiss"
                onClick={dismissReminder}
              >
                <XIcon />
              </Button>
            </>
          }
        >
          Your browser may clear these files. Export a backup.
        </Banner>
      )}
    </div>
  )
}

function BackupButton() {
  return (
    <Button size="sm" variant="outline" onClick={() => void downloadProject()}>
      <DownloadIcon />
      Download backup
    </Button>
  )
}

function Banner({
  testId,
  icon,
  actions,
  children,
}: {
  testId: string
  icon: ReactNode
  actions: ReactNode
  children: ReactNode
}) {
  return (
    <div
      role="status"
      data-testid={testId}
      className="pointer-events-auto flex max-w-2xl items-center gap-3 rounded-xl border bg-popover py-2 pr-2 pl-3.5 text-sm text-popover-foreground shadow-lg [&>svg]:size-4 [&>svg]:shrink-0 [&>svg]:text-muted-foreground"
    >
      {icon}
      <p className="flex-1">{children}</p>
      <div className="flex shrink-0 items-center gap-1.5">{actions}</div>
    </div>
  )
}

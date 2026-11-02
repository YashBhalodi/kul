import { useSelector } from "@tanstack/react-store"
import { DownloadIcon, EyeIcon, TriangleAlertIcon, XIcon } from "lucide-react"
import type { ReactNode } from "react"

import { Button } from "@/components/ui/button"
import { downloadProject } from "./download"
import { dismissReminder, durabilityStore } from "./durability-state"
import { sessionStore, takeOverProject } from "./project-session"

/**
 * Notices about the open project, stacked at the top of the canvas: that
 * another tab owns it, that this window can't store files, or that the
 * browser may clear them.
 */
export function ProjectBanners() {
  const readOnly = useSelector(
    sessionStore,
    (state) => state.status === "open" && state.access === "read-only"
  )
  const { storageSaved, reminderVisible } = useSelector(durabilityStore)
  return (
    <div className="pointer-events-none absolute inset-x-0 top-4 z-20 flex flex-col items-center gap-2 px-4">
      {readOnly && (
        <Banner
          testId="read-only-banner"
          icon={<EyeIcon />}
          actions={
            <Button size="sm" onClick={() => void takeOverProject()}>
              Use here instead
            </Button>
          }
        >
          This project is open in another tab. Changes made there show here.
        </Banner>
      )}
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

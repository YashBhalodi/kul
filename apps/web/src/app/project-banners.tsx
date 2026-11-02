import { useSelector } from "@tanstack/react-store"
import {
  ChevronRightIcon,
  CircleAlertIcon,
  DownloadIcon,
  TriangleAlertIcon,
  XIcon,
} from "lucide-react"
import type { ReactNode } from "react"

import { Button } from "@/components/ui/button"
import { downloadProject } from "./download"
import {
  canvasSizeStore,
  currentLayout,
  showProblems,
} from "./editor/editor-layout"
import { countProblems } from "./editor/problems"
import { analysisStore } from "./engine-status"
import { freeCanvasInsets } from "./editor/window-geometry"
import { sessionStore } from "./project-session"
import { dismissReminder, durabilityStore } from "./durability-state"

/**
 * Notices at the top of the canvas: that the tree on show is the last one
 * drawn before the project's errors, that this window can't store files,
 * or that the browser may clear them. (The editor window says when another
 * tab owns the project.)
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
      <StaleTreeBanner />
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

/**
 * "3 errors — showing the last valid tree", while the project has errors
 * and the canvas still shows the tree from before them. It opens the
 * problems strip.
 */
function StaleTreeBanner() {
  const errors = useSelector(analysisStore, (state) =>
    state.report && state.report.svg === undefined
      ? countProblems(state.report.diagnostics).errors
      : 0
  )
  const showsTree = useSelector(
    analysisStore,
    (state) => state.lastSvg?.includes('class="kul-card"') ?? false
  )
  if (errors === 0 || !showsTree) return null
  return (
    <button
      type="button"
      data-testid="stale-tree-banner"
      onClick={showProblems}
      className="pointer-events-auto flex items-center gap-2 rounded-full border bg-popover py-1.5 pr-2.5 pl-3 text-sm text-popover-foreground shadow-lg outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50"
    >
      <CircleAlertIcon className="size-4 text-destructive" aria-hidden />
      <span>
        <span className="font-medium">
          {errors} {errors === 1 ? "error" : "errors"}
        </span>
        {" — showing the last valid tree"}
      </span>
      <ChevronRightIcon className="size-4 text-muted-foreground" aria-hidden />
    </button>
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

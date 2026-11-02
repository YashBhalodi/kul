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
 * tab owns the project.) They stack as rows of one card, as wide as the
 * widest of them.
 */
export function ProjectBanners() {
  const { storageSaved, reminderVisible } = useSelector(durabilityStore)
  const staleErrors = useStaleTreeErrors()
  const editorWindow = useSelector(
    sessionStore,
    (state) => state.status === "open" && state.ui.editorWindow
  )
  const canvas = useSelector(canvasSizeStore)
  const storage = !storageSaved
  const reminder = storageSaved && reminderVisible
  if (staleErrors === 0 && !storage && !reminder) return null
  // Beside the editor window, never over it.
  const insets =
    editorWindow === false
      ? { left: 0, right: 0 }
      : freeCanvasInsets(currentLayout(editorWindow, canvas), canvas)
  return (
    <div
      style={{ left: insets.left, right: insets.right }}
      className="pointer-events-none absolute top-4 z-20 flex justify-center px-4"
    >
      <div className="pointer-events-auto flex max-w-2xl min-w-0 flex-col divide-y overflow-hidden rounded-xl border bg-popover text-sm text-popover-foreground shadow-lg">
        {staleErrors > 0 && <StaleTreeBanner errors={staleErrors} />}
        {storage && (
          <Banner
            testId="storage-banner"
            icon={<TriangleAlertIcon />}
            actions={<BackupButton />}
          >
            This browser window can’t save files. Download a backup before you
            close it.
          </Banner>
        )}
        {reminder && (
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
    </div>
  )
}

/**
 * The project's error count while the canvas still shows the tree from
 * before those errors; 0 otherwise.
 */
function useStaleTreeErrors(): number {
  const errors = useSelector(analysisStore, (state) =>
    state.report && state.report.svg === undefined
      ? countProblems(state.report.diagnostics).errors
      : 0
  )
  const showsTree = useSelector(
    analysisStore,
    (state) => state.lastSvg?.includes('class="kul-card"') ?? false
  )
  return showsTree ? errors : 0
}

/** The shape every row shares: an icon, its words, and what it offers. */
const ROW =
  "flex min-h-12 items-center gap-3 py-2 pr-2 pl-3.5 [&>svg]:size-4 [&>svg]:shrink-0"

/**
 * "3 errors — showing the last valid tree". It opens the problems strip.
 */
function StaleTreeBanner({ errors }: { errors: number }) {
  return (
    <button
      type="button"
      data-testid="stale-tree-banner"
      onClick={showProblems}
      className={`${ROW} w-full text-left outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:ring-inset`}
    >
      <CircleAlertIcon className="text-destructive" aria-hidden />
      <span className="flex-1">
        <span className="font-medium">
          {errors} {errors === 1 ? "error" : "errors"}
        </span>
        {" — showing the last valid tree"}
      </span>
      <span className="flex size-8 shrink-0 items-center justify-center text-muted-foreground">
        <ChevronRightIcon className="size-4" aria-hidden />
      </span>
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
      className={`${ROW} [&>svg]:text-muted-foreground`}
    >
      {icon}
      <p className="flex-1">{children}</p>
      <div className="flex shrink-0 items-center gap-1.5">{actions}</div>
    </div>
  )
}

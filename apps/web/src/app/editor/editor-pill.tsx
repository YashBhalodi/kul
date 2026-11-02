import { useFormatForDisplay } from "@tanstack/react-hotkeys"
import { useSelector } from "@tanstack/react-store"
import { cn } from "cn"
import { CodeIcon } from "lucide-react"

import { Kbd } from "@/components/ui/kbd"
import { analysisStore } from "../engine-status"
import { sessionStore } from "../project-session"
import { appHotkeys } from "../shortcuts"
import { currentLayout, setMinimized } from "./editor-layout"
import { editorUiStore } from "./editor-store"
import { countProblems } from "./problems"

/**
 * The minimized editor: a pill at the bottom left with the active file's
 * name and the project's problem count. It flashes a file's name when the
 * canvas reveals something in it.
 */
export function EditorPill() {
  const format = useFormatForDisplay()
  const minimized = useSelector(
    sessionStore,
    (state) => currentLayout(state.ui.editorWindow).minimized
  )
  const activeFile = useSelector(sessionStore, (state) => state.ui.activeFile)
  const diagnostics = useSelector(
    analysisStore,
    (state) => state.report?.diagnostics ?? []
  )
  const { pillFlash, pillFile } = useSelector(editorUiStore)
  if (!minimized) return null
  const { errors, warnings } = countProblems(diagnostics)
  const file = pillFile ?? activeFile
  const problems =
    errors > 0
      ? `${errors} ${errors === 1 ? "error" : "errors"}`
      : warnings > 0
        ? `${warnings} ${warnings === 1 ? "warning" : "warnings"}`
        : undefined

  return (
    <button
      type="button"
      data-testid="editor-pill"
      onClick={() => setMinimized(false)}
      className="pointer-events-auto absolute bottom-4 left-4 flex h-9 items-center gap-2 rounded-full border bg-card py-1 pr-2 pl-3 text-sm shadow-lg outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50"
    >
      <CodeIcon className="size-4 text-muted-foreground" aria-hidden />
      <span className="sr-only">Show the editor:</span>
      <span
        key={pillFlash}
        data-testid="editor-pill-file"
        className={cn(
          "max-w-48 truncate rounded-sm px-1 font-mono text-[0.8125rem]",
          pillFlash > 0 && "animate-pill-flash"
        )}
      >
        {file ?? "No file open"}
      </span>
      {problems && (
        <span
          data-testid="editor-pill-problems"
          className={cn(
            "rounded-full px-2 py-0.5 text-xs font-medium",
            errors > 0
              ? "bg-destructive/12 text-destructive"
              : "bg-warning/15 text-warning"
          )}
        >
          {problems}
        </span>
      )}
      <Kbd>{format(appHotkeys["editor.toggle"].defaultHotkey)}</Kbd>
    </button>
  )
}

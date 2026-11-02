import { useSelector } from "@tanstack/react-store"
import { cn } from "cn"
import {
  ChevronUpIcon,
  CircleAlertIcon,
  CircleCheckIcon,
  InfoIcon,
  TriangleAlertIcon,
} from "lucide-react"
import { useId } from "react"

import type { Severity } from "@/language/protocol"
import { analysisStore } from "../engine-status"
import { sessionStore } from "../project-session"
import { showLocation } from "./editor-controller"
import { editorUiStore, setProblemsOpen } from "./editor-store"
import { InlineCode } from "./inline-code"
import { countProblems, describeProblems, listProblems } from "./problems"

const SEVERITY_ICON: Readonly<Record<Severity, typeof CircleAlertIcon>> = {
  error: CircleAlertIcon,
  warning: TriangleAlertIcon,
  note: InfoIcon,
}

const SEVERITY_COLOUR: Readonly<Record<Severity, string>> = {
  error: "text-destructive",
  warning: "text-warning",
  note: "text-link",
}

/**
 * The bottom of the editor: the whole project's error and warning counts.
 * Expanded, it lists every problem; choosing one opens it in the editor.
 */
export function ProblemsStrip() {
  const listId = useId()
  const diagnostics = useSelector(
    analysisStore,
    (state) => state.report?.diagnostics ?? []
  )
  const files = useSelector(sessionStore, (state) => state.files)
  const open = useSelector(editorUiStore, (state) => state.problemsOpen)
  const counts = countProblems(diagnostics)
  const problems = open ? listProblems(diagnostics, files) : []
  const clean = counts.errors === 0 && counts.warnings === 0

  return (
    <div data-testid="problems-strip" className="shrink-0 border-t bg-muted/40">
      <button
        type="button"
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => setProblemsOpen(!open)}
        className="flex h-8 w-full items-center gap-2 px-3 text-xs text-muted-foreground outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:ring-inset"
      >
        {clean ? (
          <CircleCheckIcon className="size-3.5 text-primary" aria-hidden />
        ) : (
          <CircleAlertIcon
            className={cn(
              "size-3.5",
              counts.errors > 0 ? "text-destructive" : "text-warning"
            )}
            aria-hidden
          />
        )}
        <span
          data-testid="problems-count"
          className="font-medium text-foreground"
        >
          {describeProblems(counts)}
        </span>
        <span className="sr-only">
          {open ? "Hide the list" : "Show the list"}
        </span>
        <ChevronUpIcon
          aria-hidden
          className={cn(
            "ml-auto size-3.5 transition-transform",
            open && "rotate-180"
          )}
        />
      </button>
      {open && (
        <ul
          id={listId}
          aria-label="Problems"
          className="max-h-44 overflow-y-auto border-t bg-card py-1"
        >
          {problems.length === 0 && (
            <li className="px-3 py-2 text-xs text-muted-foreground">
              Nothing to fix. The project checks out.
            </li>
          )}
          {problems.map((problem, index) => {
            const Icon = SEVERITY_ICON[problem.diagnostic.severity]
            const { location } = problem
            const where = location
              ? `${location.file}:${problem.line ?? 1}`
              : "Project"
            const body = (
              <>
                <Icon
                  aria-hidden
                  className={cn(
                    "mt-0.5 size-3.5 shrink-0",
                    SEVERITY_COLOUR[problem.diagnostic.severity]
                  )}
                />
                <span className="shrink-0 font-mono text-muted-foreground">
                  {problem.diagnostic.code}
                </span>
                <span className="min-w-0 flex-1 text-foreground">
                  <InlineCode text={problem.diagnostic.message} />
                </span>
                <span className="shrink-0 font-mono text-muted-foreground">
                  {where}
                </span>
              </>
            )
            return (
              <li key={`${where}-${problem.diagnostic.code}-${index}`}>
                {location ? (
                  <button
                    type="button"
                    data-testid="problem"
                    onClick={() => showLocation(location)}
                    className="flex w-full items-start gap-2 px-3 py-1.5 text-left text-xs outline-none hover:bg-muted focus-visible:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:ring-inset"
                  >
                    {body}
                  </button>
                ) : (
                  <div
                    data-testid="problem"
                    className="flex items-start gap-2 px-3 py-1.5 text-xs"
                  >
                    {body}
                  </div>
                )}
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}

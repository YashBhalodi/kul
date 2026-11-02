import { useSelector } from "@tanstack/react-store"
import { cn } from "cn"
import {
  CircleAlertIcon,
  CircleCheckIcon,
  TriangleAlertIcon,
} from "lucide-react"

import { Button } from "@/components/ui/button"
import { analysisStore } from "../engine-status"
import { sessionStore } from "../project-session"
import { showProblems } from "./editor-layout"
import { countProblems, describeProblems } from "./problems"

/** The top bar's problem count; it opens the problems strip. */
export function ProblemsButton() {
  const open = useSelector(sessionStore, (state) => state.status === "open")
  const report = useSelector(analysisStore, (state) => state.report)
  if (!open || !report) return null
  const counts = countProblems(report.diagnostics)
  const Icon =
    counts.errors > 0
      ? CircleAlertIcon
      : counts.warnings > 0
        ? TriangleAlertIcon
        : CircleCheckIcon
  return (
    <Button
      variant="ghost"
      size="sm"
      data-testid="problems-button"
      onClick={showProblems}
      className="gap-1.5 text-muted-foreground"
    >
      <Icon
        aria-hidden
        className={cn(
          counts.errors > 0
            ? "text-destructive"
            : counts.warnings > 0
              ? "text-warning"
              : "text-primary"
        )}
      />
      <span className="tabular-nums">{describeProblems(counts)}</span>
    </Button>
  )
}

import { useSelector } from "@tanstack/react-store"
import { CircleAlertIcon, EyeIcon } from "lucide-react"

import { sessionStore } from "./project-session"

/** A quiet word on whether the project's changes are saved. */
export function SaveStatus() {
  const save = useSelector(sessionStore, (state) => state.save)
  const access = useSelector(sessionStore, (state) => state.access)
  const status = useSelector(sessionStore, (state) => state.status)
  if (status !== "open") return null
  return (
    <p
      role="status"
      aria-live="polite"
      data-testid="save-status"
      data-state={access === "read-only" ? "read-only" : save}
      className="flex items-center gap-1.5 text-xs text-muted-foreground"
    >
      {access === "read-only" ? (
        <>
          <EyeIcon className="size-3.5" aria-hidden />
          Read-only
        </>
      ) : save === "saving" ? (
        "Saving…"
      ) : save === "saved" ? (
        "Saved"
      ) : save === "failed" ? (
        <>
          <CircleAlertIcon className="size-3.5 text-destructive" aria-hidden />
          <span className="text-foreground">Couldn’t save</span>
        </>
      ) : null}
    </p>
  )
}

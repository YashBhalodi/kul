import { useSelector } from "@tanstack/react-store"
import { CircleAlertIcon, LoaderCircleIcon } from "lucide-react"
import { useEffect } from "react"

import { KulMark } from "@/components/kul-mark"
import { Button } from "@/components/ui/button"
import { bootEngine, engineStatusStore } from "./engine-status"

/** The canvas: the dot-grid ground the family tree is drawn on. */
export function CanvasSurface() {
  useEffect(bootEngine, [])
  return (
    <div
      data-testid="canvas"
      className="canvas-grid absolute inset-0 overflow-hidden bg-canvas"
    >
      <div className="flex size-full items-center justify-center p-8">
        <div className="flex max-w-sm flex-col items-center gap-4 text-center">
          <div className="flex size-14 items-center justify-center rounded-2xl border bg-card shadow-sm">
            <KulMark className="size-7" />
          </div>
          <h1 className="text-lg font-semibold tracking-tight">
            Your family tree appears here
          </h1>
          <p className="text-sm leading-relaxed text-muted-foreground">
            Kul reads the people, marriages and children in your project’s .kul
            files and draws them as a tree that updates as you type.
          </p>
        </div>
      </div>
      <EngineStatusBadge />
    </div>
  )
}

function EngineStatusBadge() {
  const status = useSelector(engineStatusStore)
  return (
    <div
      role="status"
      data-testid="engine-status"
      data-state={status.state}
      className="absolute bottom-4 left-4 flex items-center gap-2 rounded-full border bg-card/95 py-1 pr-3 pl-2.5 text-xs text-muted-foreground shadow-sm"
    >
      {status.state === "starting" && (
        <>
          <LoaderCircleIcon className="size-3.5 animate-spin" aria-hidden />
          Starting the Kul engine…
        </>
      )}
      {status.state === "ready" && (
        <>
          <span className="size-2 rounded-full bg-primary" aria-hidden />
          Kul {status.info.core} · language {status.info.language}
        </>
      )}
      {status.state === "failed" && (
        <>
          <CircleAlertIcon className="size-3.5 text-destructive" aria-hidden />
          <span className="text-foreground">The Kul engine didn’t start.</span>
          <Button
            variant="link"
            size="xs"
            className="h-auto px-0"
            onClick={bootEngine}
          >
            Try again
          </Button>
        </>
      )}
    </div>
  )
}

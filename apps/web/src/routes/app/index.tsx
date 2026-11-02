import { createFileRoute } from "@tanstack/react-router"

import { CanvasSurface } from "@/app/canvas-surface"

export const Route = createFileRoute("/app/")({
  component: CanvasSurface,
})

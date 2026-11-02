import { createFileRoute } from "@tanstack/react-router"

import { StartView } from "@/app/project-view"

export const Route = createFileRoute("/app/")({
  component: StartView,
})

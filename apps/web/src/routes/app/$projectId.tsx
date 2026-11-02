import { createFileRoute } from "@tanstack/react-router"

import { ProjectView } from "@/app/project-view"

export const Route = createFileRoute("/app/$projectId")({
  component: function ProjectRoute() {
    const { projectId } = Route.useParams()
    return <ProjectView projectId={projectId} />
  },
})

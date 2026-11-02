import { Link } from "@tanstack/react-router"
import { useSelector } from "@tanstack/react-store"
import { FolderSearchIcon } from "lucide-react"
import { useEffect } from "react"

import { buttonVariants } from "@/components/ui/button"
import { CanvasSurface, CanvasMessage } from "./canvas-surface"
import { FilesPanel } from "./files-panel"
import { closeProject, openProject, sessionStore } from "./project-session"
import { ProjectBanners } from "./project-banners"
import { startProject } from "./projects"
import { TreePreview } from "./tree-preview"
import { useOpenProject } from "./use-open-project"

/** /app/: open the last project, or seed the first one on a first visit. */
export function StartView() {
  const openProject = useOpenProject()
  useEffect(() => {
    let current = true
    void startProject().then((project) => {
      if (current) openProject(project.id, { replace: true })
    })
    return () => {
      current = false
    }
  }, [openProject])
  return <CanvasSurface />
}

/** /app/<id>: the project, its files and its tree over the canvas. */
export function ProjectView({ projectId }: { projectId: string }) {
  const status = useSelector(sessionStore, (state) => state.status)

  useEffect(() => {
    void openProject(projectId)
  }, [projectId])
  // Leaving the app closes the project, writing what is unsaved.
  useEffect(() => () => void closeProject(), [])

  return (
    <CanvasSurface>
      {status === "open" && (
        <>
          <TreePreview />
          <FilesPanel />
        </>
      )}
      {status === "missing" && <MissingProject />}
      <ProjectBanners />
    </CanvasSurface>
  )
}

function MissingProject() {
  return (
    <CanvasMessage
      testId="project-missing"
      icon={<FolderSearchIcon className="size-7 text-muted-foreground" />}
      title="This project isn’t in this browser"
      actions={
        <Link to="/app/" className={buttonVariants()}>
          Open your projects
        </Link>
      }
    >
      Projects live in the browser they were made in. It may have been deleted,
      or made in another browser or on another computer.
    </CanvasMessage>
  )
}

import { Link } from "@tanstack/react-router"
import { useSelector } from "@tanstack/react-store"
import { FolderSearchIcon } from "lucide-react"
import { useEffect } from "react"

import { buttonVariants } from "@/components/ui/button"
import { CanvasControls } from "./canvas/canvas-controls"
import { DetailsPanel } from "./canvas/details-panel"
import { TreeCanvas } from "./canvas/tree-canvas"
import { CanvasSurface, CanvasMessage } from "./canvas-surface"
import { EditorWindow } from "./editor/editor-window"
import { closeProject, openProject, sessionStore } from "./project-session"
import { ProjectBanners } from "./project-banners"
import { startProject } from "./projects"
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

/**
 * /app/<id>: the project's tree on the canvas, with the details panel, the
 * editor window over it, and the canvas's controls in the corner.
 */
export function ProjectView({ projectId }: { projectId: string }) {
  const status = useSelector(sessionStore, (state) => state.status)

  useEffect(() => {
    void openProject(projectId)
  }, [projectId])
  // Leaving the app closes the project, writing what is unsaved.
  useEffect(() => () => void closeProject(), [])

  return (
    <CanvasSurface corner={status === "open" ? <CanvasControls /> : undefined}>
      {status === "open" && (
        <>
          <TreeCanvas />
          <DetailsPanel />
          <EditorWindow />
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

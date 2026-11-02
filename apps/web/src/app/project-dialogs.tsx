import { useSelector } from "@tanstack/react-store"
import { useId, useState } from "react"

import { ConfirmDeleteDialog } from "@/components/confirm-delete-dialog"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { characterCount } from "@/store/file-names"
import { MAX_PROJECT_NAME_LENGTH } from "@/store/project-store"
import { appStore, dialogControl } from "./app-store"
import { closeProject, renameProject, sessionStore } from "./project-session"
import { deleteProject, projectsStore, refreshProjects } from "./projects"
import { useOpenProject } from "./use-open-project"
import { useNavigate } from "@tanstack/react-router"

/** Rename the open project. */
export function RenameProjectDialog() {
  const control = dialogControl("rename-project")
  const open = useSelector(appStore, control.open)
  const name = useSelector(sessionStore, (state) => state.project?.name ?? "")
  return (
    <Dialog open={open} onOpenChange={control.onOpenChange}>
      <DialogContent>
        {/* Mounted per opening, so the field starts from the current name. */}
        {open && (
          <RenameProjectForm
            initial={name}
            onDone={() => control.onOpenChange(false)}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

function RenameProjectForm({
  initial,
  onDone,
}: {
  initial: string
  onDone: () => void
}) {
  const [value, setValue] = useState(initial)
  const errorId = useId()
  const trimmed = value.trim()
  const problem =
    trimmed === ""
      ? "Enter a name."
      : characterCount(trimmed) > MAX_PROJECT_NAME_LENGTH
        ? `Names can be at most ${MAX_PROJECT_NAME_LENGTH} characters long.`
        : undefined
  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        if (problem) return
        onDone()
        if (trimmed !== initial) void renameProject(trimmed)
      }}
    >
      <DialogHeader>
        <DialogTitle>Rename project</DialogTitle>
        <DialogDescription>
          The name shows in the project list and names its downloads.
        </DialogDescription>
      </DialogHeader>
      <div className="grid gap-1.5">
        <Input
          aria-label="Project name"
          aria-invalid={problem !== undefined}
          aria-describedby={problem ? errorId : undefined}
          value={value}
          autoFocus
          onFocus={(event) => event.currentTarget.select()}
          onChange={(event) => setValue(event.currentTarget.value)}
        />
        {problem && (
          <p id={errorId} className="text-xs text-destructive">
            {problem}
          </p>
        )}
      </div>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onDone}>
          Cancel
        </Button>
        <Button type="submit" disabled={problem !== undefined}>
          Rename
        </Button>
      </DialogFooter>
    </form>
  )
}

/** Delete the open project, then open another one. */
export function DeleteProjectDialog() {
  const navigate = useNavigate()
  const openProject = useOpenProject()
  const control = dialogControl("delete-project")
  const open = useSelector(appStore, control.open)
  const project = useSelector(sessionStore, (state) => state.project)
  const fileCount = useSelector(sessionStore, (state) => state.files.length)
  if (!project) return null
  return (
    <ConfirmDeleteDialog
      open={open}
      onOpenChange={control.onOpenChange}
      title={`Delete “${project.name}”?`}
      description={`Its ${fileCount === 1 ? "file is" : `${fileCount} files are`} removed from this browser. You can’t undo this.`}
      confirmLabel="Delete project"
      onConfirm={() => {
        void (async () => {
          await closeProject()
          await deleteProject(project.id)
          await refreshProjects()
          const next = projectsStore.state.projects?.[0]
          if (next) {
            openProject(next.id, { replace: true })
          } else {
            await navigate({ to: "/app/", replace: true })
          }
        })()
      }}
    />
  )
}

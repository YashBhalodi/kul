import { useSelector } from "@tanstack/react-store"
import { useMemo } from "react"

import { EXAMPLES } from "@/project/examples"
import { appStore } from "./app-store"
import { useCommands } from "./commands"
import type { Command } from "./commands"
import { downloadFile, downloadProject } from "./download"
import {
  askToDeleteFile,
  startNewFile,
  startRenamingFile,
} from "./file-list-state"
import { pickImport } from "./import-picker"
import { sessionStore } from "./project-session"
import {
  createBlankProject,
  createProjectFromExample,
  duplicateProject,
  projectsStore,
} from "./projects"
import { useOpenProject } from "./use-open-project"

/** Register the palette's project and file commands. */
export function useProjectCommands(): void {
  const open = useOpenProject()
  const project = useSelector(sessionStore, (state) => state.project)
  const owner = useSelector(
    sessionStore,
    (state) => state.status === "open" && state.access === "owner"
  )
  const activeFile = useSelector(sessionStore, (state) => state.ui.activeFile)
  const projects = useSelector(projectsStore, (state) => state.projects)

  const commands = useMemo((): ReadonlyArray<Command> => {
    const openNew = (made: Promise<{ id: string }>) =>
      void made.then((created) => open(created.id))
    const list: Array<Command> = []

    if (owner) {
      list.push({
        id: "file.new",
        title: "New file",
        group: "Files",
        keywords: ["create", "add"],
        run: startNewFile,
      })
      if (activeFile) {
        list.push(
          {
            id: "file.rename",
            title: `Rename ${activeFile}`,
            group: "Files",
            hotkey: "file.rename",
            run: () => startRenamingFile(activeFile),
          },
          {
            id: "file.delete",
            title: `Delete ${activeFile}`,
            group: "Files",
            keywords: ["remove"],
            run: () => askToDeleteFile(activeFile),
          }
        )
      }
    }
    if (project && activeFile) {
      list.push({
        id: "file.download",
        title: `Download ${activeFile}`,
        group: "Files",
        keywords: ["export", "save"],
        run: () => downloadFile(activeFile),
      })
    }

    for (const other of projects ?? []) {
      if (other.id === project?.id) continue
      list.push({
        id: `project.open.${other.id}`,
        title: `Switch to ${other.name}`,
        group: "Projects",
        keywords: ["open", "project"],
        run: () => open(other.id),
      })
    }
    list.push({
      id: "project.new",
      title: "New blank project",
      group: "Projects",
      keywords: ["create"],
      run: () => openNew(createBlankProject()),
    })
    for (const example of EXAMPLES) {
      list.push({
        id: `project.example.${example.slug}`,
        title: `New from example: ${example.title}`,
        group: "Projects",
        keywords: ["example", "tutorial", "create"],
        run: () => openNew(createProjectFromExample(example)),
      })
    }
    if (owner) {
      list.push(
        {
          id: "project.import.files",
          title: "Import files or a .zip into this project",
          group: "Projects",
          keywords: ["upload", "open"],
          run: () => pickImport("files", "current"),
        },
        {
          id: "project.import.folder",
          title: "Import a folder into this project",
          group: "Projects",
          keywords: ["upload", "open", "directory"],
          run: () => pickImport("folder", "current"),
        }
      )
    }
    list.push(
      {
        id: "project.import.new",
        title: "Import files or a .zip as a new project",
        group: "Projects",
        keywords: ["upload", "open"],
        run: () => pickImport("files", "new"),
      },
      {
        id: "project.import.new-folder",
        title: "Import a folder as a new project",
        group: "Projects",
        keywords: ["upload", "open", "directory"],
        run: () => pickImport("folder", "new"),
      }
    )
    if (project) {
      list.push(
        {
          id: "project.export",
          title: "Download project (.zip)",
          group: "Projects",
          keywords: ["export", "backup", "zip"],
          run: () => void downloadProject(),
        },
        {
          id: "project.duplicate",
          title: "Duplicate project",
          group: "Projects",
          keywords: ["copy"],
          run: () => openNew(duplicateProject(project)),
        }
      )
      if (owner) {
        list.push(
          {
            id: "project.rename",
            title: "Rename project",
            group: "Projects",
            run: () => appStore.actions.show("rename-project"),
          },
          {
            id: "project.delete",
            title: "Delete project",
            group: "Projects",
            keywords: ["remove"],
            run: () => appStore.actions.show("delete-project"),
          }
        )
      }
    }
    list.push({
      id: "app.settings",
      title: "Settings",
      group: "App",
      keywords: ["storage", "space", "backup", "persist"],
      run: () => appStore.actions.show("settings"),
    })
    return list
  }, [open, project, owner, activeFile, projects])

  useCommands(commands)
}

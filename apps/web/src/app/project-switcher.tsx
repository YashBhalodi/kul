import { useSelector } from "@tanstack/react-store"
import {
  BookOpenTextIcon,
  CheckIcon,
  ChevronsUpDownIcon,
  CopyIcon,
  FileUpIcon,
  FolderPlusIcon,
  FolderUpIcon,
  PencilIcon,
  PlusIcon,
  Trash2Icon,
  UploadIcon,
} from "lucide-react"
import { useState } from "react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { EXAMPLES } from "@/project/examples"
import { appStore } from "./app-store"
import { carriesFiles, readDataTransfer, startImport } from "./import-flow"
import { pickImport } from "./import-picker"
import { sessionStore } from "./project-session"
import {
  createBlankProject,
  createProjectFromExample,
  duplicateProject,
  projectsStore,
  refreshProjects,
} from "./projects"
import { editedAgo } from "./relative-time"
import { useOpenProject } from "./use-open-project"

/**
 * The project switcher: the open project's name, and a menu to switch,
 * create, import, rename, copy and delete projects. Dropping files on it
 * imports them as a new project.
 */
export function ProjectSwitcher() {
  const open = useOpenProject()
  const project = useSelector(sessionStore, (state) => state.project)
  const readOnly = useSelector(
    sessionStore,
    (state) => state.access === "read-only"
  )
  const projects = useSelector(projectsStore, (state) => state.projects)
  const [dropping, setDropping] = useState(false)

  const openNew = (made: Promise<{ id: string }>) =>
    void made.then((created) => open(created.id))

  return (
    <DropdownMenu
      onOpenChange={(isOpen) => {
        if (isOpen) void refreshProjects()
      }}
    >
      <DropdownMenuTrigger
        render={
          <Button
            variant="ghost"
            data-testid="project-switcher"
            data-dropping={dropping || undefined}
            className="max-w-72 gap-1.5 px-2 font-semibold data-dropping:bg-primary/10 data-dropping:ring-2 data-dropping:ring-primary"
            onDragOver={(event) => {
              if (!carriesFiles(event.dataTransfer)) return
              event.preventDefault()
              event.stopPropagation()
              event.dataTransfer.dropEffect = "copy"
              setDropping(true)
            }}
            onDragLeave={() => setDropping(false)}
            onDrop={(event) => {
              if (!carriesFiles(event.dataTransfer)) return
              event.preventDefault()
              event.stopPropagation()
              setDropping(false)
              void readDataTransfer(event.dataTransfer).then((picked) =>
                startImport(picked, "new", open)
              )
            }}
          />
        }
      >
        <span className="truncate">
          {dropping
            ? "Drop to import as a new project"
            : (project?.name ?? "Projects")}
        </span>
        <ChevronsUpDownIcon className="text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-80">
        <DropdownMenuGroup>
          <DropdownMenuLabel>Projects</DropdownMenuLabel>
          <div className="max-h-72 overflow-y-auto">
            {(projects ?? []).map((item) => (
              <DropdownMenuItem
                key={item.id}
                data-testid="project-option"
                onClick={() => {
                  if (item.id !== project?.id) open(item.id)
                }}
                className="items-start py-2"
              >
                <CheckIcon
                  className={
                    item.id === project?.id ? "mt-0.5" : "invisible mt-0.5"
                  }
                  aria-hidden
                />
                <span className="flex min-w-0 flex-col">
                  <span className="truncate font-medium">{item.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {editedAgo(item.updatedAt)}
                  </span>
                </span>
              </DropdownMenuItem>
            ))}
          </div>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuItem onClick={() => openNew(createBlankProject())}>
            <PlusIcon />
            New blank project
          </DropdownMenuItem>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>
              <BookOpenTextIcon />
              New from example
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent className="w-80">
              {EXAMPLES.map((example) => (
                <DropdownMenuItem
                  key={example.slug}
                  onClick={() => openNew(createProjectFromExample(example))}
                >
                  {example.title}
                </DropdownMenuItem>
              ))}
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger disabled={readOnly || !project}>
              <UploadIcon />
              Import into this project
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent className="w-56">
              <DropdownMenuItem onClick={() => pickImport("files", "current")}>
                <FileUpIcon />
                Files or a .zip…
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => pickImport("folder", "current")}>
                <FolderUpIcon />A folder…
              </DropdownMenuItem>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
          <DropdownMenuSub>
            <DropdownMenuSubTrigger>
              <FolderPlusIcon />
              Import as a new project
            </DropdownMenuSubTrigger>
            <DropdownMenuSubContent className="w-56">
              <DropdownMenuItem onClick={() => pickImport("files", "new")}>
                <FileUpIcon />
                Files or a .zip…
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => pickImport("folder", "new")}>
                <FolderUpIcon />A folder…
              </DropdownMenuItem>
            </DropdownMenuSubContent>
          </DropdownMenuSub>
        </DropdownMenuGroup>
        {project && (
          <>
            <DropdownMenuSeparator />
            <DropdownMenuGroup>
              <DropdownMenuItem
                disabled={readOnly}
                onClick={() => appStore.actions.show("rename-project")}
              >
                <PencilIcon />
                Rename project…
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => openNew(duplicateProject(project))}
              >
                <CopyIcon />
                Duplicate project
              </DropdownMenuItem>
              <DropdownMenuItem
                variant="destructive"
                disabled={readOnly}
                onClick={() => appStore.actions.show("delete-project")}
              >
                <Trash2Icon />
                Delete project…
              </DropdownMenuItem>
            </DropdownMenuGroup>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

import { useHotkey } from "@tanstack/react-hotkeys"
import { useSelector } from "@tanstack/react-store"
import { cn } from "cn"
import {
  DownloadIcon,
  FilePlusIcon,
  MoreHorizontalIcon,
  PencilIcon,
  Trash2Icon,
} from "lucide-react"

import { ConfirmDeleteDialog } from "@/components/confirm-delete-dialog"
import { FileNameInput } from "@/components/file-name-input"
import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuShortcut,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { downloadFile } from "../download"
import { analysisStore } from "../engine-status"
import {
  askToDeleteFile,
  endFileListAction,
  fileListStore,
  startNewFile,
  startRenamingFile,
} from "../file-list-state"
import {
  createFile,
  deleteFile,
  selectFile,
  sessionStore,
} from "../project-session"
import { appHotkeys } from "../shortcuts"
import { renameProjectFile } from "./editor-controller"
import { problemsByFile } from "./problems"
import type { ProblemCounts } from "./problems"

/**
 * The editor's left rail: the project's files in file order. Pick one to
 * edit it; add, rename, download and delete files. A dot marks a file
 * with errors or warnings.
 */
export function EditorRail() {
  const files = useSelector(sessionStore, (state) => state.files)
  const activeFile = useSelector(sessionStore, (state) => state.ui.activeFile)
  const readOnly = useSelector(
    sessionStore,
    (state) => state.access === "read-only"
  )
  const diagnostics = useSelector(
    analysisStore,
    (state) => state.report?.diagnostics
  )
  const problems = problemsByFile(diagnostics ?? [])
  const editing = useSelector(fileListStore)
  const names = files.map((file) => file.name)

  useHotkey(appHotkeys["file.rename"], () => {
    if (activeFile && !readOnly) startRenamingFile(activeFile)
  })

  return (
    <nav
      aria-label="Files"
      data-testid="files-panel"
      className="flex w-56 shrink-0 flex-col bg-muted/30"
    >
      <header className="flex h-9 shrink-0 items-center gap-2 pr-1 pl-3">
        <h3 className="text-xs font-medium text-muted-foreground">Files</h3>
        <span className="text-xs text-muted-foreground tabular-nums">
          {files.length}
        </span>
        {!readOnly && (
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon-xs"
                  className="ml-auto"
                  aria-label="New file"
                  onClick={startNewFile}
                />
              }
            >
              <FilePlusIcon />
            </TooltipTrigger>
            <TooltipContent>New file</TooltipContent>
          </Tooltip>
        )}
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-1.5 pb-1.5">
        {files.length === 0 && !editing.creating && (
          <p className="px-2 py-2 text-xs leading-relaxed text-muted-foreground">
            This project has no files yet.
          </p>
        )}
        <ul className="flex flex-col gap-0.5">
          {files.map((file) =>
            editing.renaming === file.name ? (
              <li key={file.name} className="py-0.5">
                <FileNameInput
                  label={`New name for ${file.name}`}
                  existing={names}
                  renaming={file.name}
                  onSubmit={(name) => {
                    endFileListAction()
                    void renameProjectFile(file.name, name)
                  }}
                  onCancel={endFileListAction}
                />
              </li>
            ) : (
              <FileRow
                key={file.name}
                name={file.name}
                active={file.name === activeFile}
                readOnly={readOnly}
                problems={problems.get(file.name)}
              />
            )
          )}
          {editing.creating && (
            <li className="py-0.5">
              <FileNameInput
                label="New file name"
                existing={names}
                onSubmit={(name) => {
                  endFileListAction()
                  void createFile(name)
                }}
                onCancel={endFileListAction}
              />
            </li>
          )}
        </ul>
      </div>

      <ConfirmDeleteDialog
        open={editing.deleting !== undefined}
        onOpenChange={(open) => {
          if (!open) endFileListAction()
        }}
        title={`Delete ${editing.deleting ?? "file"}?`}
        description="The file is removed from this project. You can’t undo this."
        confirmLabel="Delete file"
        onConfirm={() => {
          if (editing.deleting) void deleteFile(editing.deleting)
        }}
      />
    </nav>
  )
}

function problemLabel(problems: ProblemCounts | undefined): string | undefined {
  if (!problems) return undefined
  if (problems.errors > 0) {
    return problems.errors === 1 ? "1 error" : `${problems.errors} errors`
  }
  if (problems.warnings > 0) {
    return problems.warnings === 1
      ? "1 warning"
      : `${problems.warnings} warnings`
  }
  return undefined
}

function FileRow({
  name,
  active,
  readOnly,
  problems,
}: {
  name: string
  active: boolean
  readOnly: boolean
  problems: ProblemCounts | undefined
}) {
  const rename = () => startRenamingFile(name)
  const label = problemLabel(problems)
  return (
    <li
      data-testid="file-row"
      data-name={name}
      data-active={active || undefined}
      data-problems={
        problems?.errors ? "error" : problems?.warnings ? "warning" : undefined
      }
      className="group/file relative flex items-center rounded-md hover:bg-muted data-active:bg-accent"
    >
      <button
        type="button"
        aria-current={active ? "true" : undefined}
        aria-label={label ? `${name}, ${label}` : undefined}
        onClick={() => void selectFile(name)}
        onDoubleClick={readOnly ? undefined : rename}
        className={cn(
          "flex h-8 min-w-0 flex-1 items-center gap-2 rounded-md pr-7 pl-2 text-left outline-none focus-visible:ring-3 focus-visible:ring-ring/50",
          active ? "font-medium text-foreground" : "text-muted-foreground"
        )}
      >
        <span className="truncate font-mono text-[0.8125rem]">{name}</span>
        {label && (
          <span
            aria-hidden
            className={cn(
              "size-1.5 shrink-0 rounded-full",
              problems?.errors ? "bg-destructive" : "bg-warning"
            )}
          />
        )}
      </button>
      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button
              variant="ghost"
              size="icon-xs"
              aria-label={`Actions for ${name}`}
              className="absolute right-1 opacity-0 group-hover/file:opacity-100 group-data-active/file:opacity-100 focus-visible:opacity-100 aria-expanded:opacity-100"
            />
          }
        >
          <MoreHorizontalIcon />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-48">
          {!readOnly && (
            <DropdownMenuItem onClick={rename}>
              <PencilIcon />
              Rename
              <DropdownMenuShortcut>F2</DropdownMenuShortcut>
            </DropdownMenuItem>
          )}
          <DropdownMenuItem onClick={() => downloadFile(name)}>
            <DownloadIcon />
            Download
          </DropdownMenuItem>
          {!readOnly && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                variant="destructive"
                onClick={() => askToDeleteFile(name)}
              >
                <Trash2Icon />
                Delete…
              </DropdownMenuItem>
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </li>
  )
}

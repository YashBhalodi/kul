import { useHotkey } from "@tanstack/react-hotkeys"
import { useSelector } from "@tanstack/react-store"
import { cn } from "cn"
import {
  DownloadIcon,
  FileIcon,
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
import { downloadFile } from "./download"
import {
  askToDeleteFile,
  endFileListAction,
  fileListStore,
  startNewFile,
  startRenamingFile,
} from "./file-list-state"
import {
  createFile,
  deleteFile,
  renameFile,
  selectFile,
  sessionStore,
} from "./project-session"
import { appHotkeys } from "./shortcuts"

/**
 * The project's files, in file order: pick the active one, add, rename,
 * download and delete files.
 */
export function FilesPanel() {
  const files = useSelector(sessionStore, (state) => state.files)
  const activeFile = useSelector(sessionStore, (state) => state.ui.activeFile)
  const readOnly = useSelector(
    sessionStore,
    (state) => state.access === "read-only"
  )
  const editing = useSelector(fileListStore)
  const names = files.map((file) => file.name)

  useHotkey(appHotkeys["file.rename"], () => {
    if (activeFile && !readOnly) startRenamingFile(activeFile)
  })

  return (
    <section
      aria-label="Files"
      data-testid="files-panel"
      className="absolute top-4 left-4 z-10 flex max-h-[calc(100%-2rem)] w-64 flex-col rounded-xl border bg-card/95 shadow-lg backdrop-blur-sm"
    >
      <header className="flex h-10 shrink-0 items-center gap-2 border-b pr-1.5 pl-3">
        <h2 className="text-xs font-medium text-muted-foreground">Files</h2>
        <span className="text-xs text-muted-foreground tabular-nums">
          {files.length}
        </span>
        {!readOnly && (
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon-sm"
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

      <div className="min-h-0 overflow-y-auto p-1.5">
        {files.length === 0 && !editing.creating && (
          <p className="px-2 py-3 text-sm text-muted-foreground">
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
                    void renameFile(file.name, name)
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
    </section>
  )
}

function FileRow({
  name,
  active,
  readOnly,
}: {
  name: string
  active: boolean
  readOnly: boolean
}) {
  const rename = () => startRenamingFile(name)
  return (
    <li
      data-testid="file-row"
      data-name={name}
      data-active={active || undefined}
      className="group/file relative flex items-center rounded-md hover:bg-muted data-active:bg-accent"
    >
      <button
        type="button"
        aria-current={active ? "true" : undefined}
        onClick={() => void selectFile(name)}
        onDoubleClick={readOnly ? undefined : rename}
        className={cn(
          "flex h-8 min-w-0 flex-1 items-center gap-2 rounded-md pr-8 pl-2 text-left text-sm outline-none focus-visible:ring-3 focus-visible:ring-ring/50",
          active ? "font-medium text-foreground" : "text-muted-foreground"
        )}
      >
        <FileIcon className="size-3.5 shrink-0" aria-hidden />
        <span className="truncate font-mono text-[0.8125rem]">{name}</span>
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

import { useSelector } from "@tanstack/react-store"
import {
  DownloadIcon,
  FileDownIcon,
  FolderArchiveIcon,
  NetworkIcon,
} from "lucide-react"

import { Button } from "@/components/ui/button"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { downloadFile, downloadProject, downloadTree } from "./download"
import { analysisStore } from "./engine-status"
import { sessionStore } from "./project-session"

/** Downloads of the open project. */
export function ExportMenu() {
  const open = useSelector(sessionStore, (state) => state.status === "open")
  const activeFile = useSelector(sessionStore, (state) => state.ui.activeFile)
  const drawable = useSelector(
    analysisStore,
    (state) => state.report?.svg?.includes('class="kul-card"') ?? false
  )
  if (!open) return null
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={<Button variant="ghost" size="sm" data-testid="export-menu" />}
      >
        <DownloadIcon />
        Export
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-60">
        <DropdownMenuItem onClick={() => void downloadProject()}>
          <FolderArchiveIcon />
          Download project (.zip)
        </DropdownMenuItem>
        <DropdownMenuItem
          disabled={activeFile === null}
          onClick={() => {
            if (activeFile) downloadFile(activeFile)
          }}
        >
          <FileDownIcon />
          <span className="truncate">Download {activeFile ?? "file"}</span>
        </DropdownMenuItem>
        <DropdownMenuItem
          disabled={!drawable}
          onClick={() => void downloadTree()}
          data-testid="download-tree"
        >
          <NetworkIcon />
          Download tree (.svg)
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

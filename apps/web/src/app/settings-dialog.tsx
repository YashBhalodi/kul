import { useSelector } from "@tanstack/react-store"
import { HardDriveIcon, ShieldCheckIcon, ShieldIcon } from "lucide-react"
import { useEffect, useState } from "react"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { readPersistence, readStorageUsage } from "@/store/durability"
import type { StorageUsage } from "@/store/durability"
import { appStore, dialogControl } from "./app-store"
import { formatBytes } from "./format-bytes"
import { askToPersist, durabilityStore } from "./durability-state"
import { projectsStore } from "./projects"

/** Settings: where projects are kept and how safely. */
export function SettingsDialog() {
  const control = dialogControl("settings")
  const open = useSelector(appStore, control.open)
  return (
    <Dialog open={open} onOpenChange={control.onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>
            Kul keeps your projects in this browser, on this computer.
          </DialogDescription>
        </DialogHeader>
        {open && <StorageSettings />}
      </DialogContent>
    </Dialog>
  )
}

function StorageSettings() {
  const durability = useSelector(durabilityStore)
  const projectCount = useSelector(
    projectsStore,
    (state) => state.projects?.length
  )
  const [usage, setUsage] = useState<StorageUsage>()

  useEffect(() => {
    void readStorageUsage().then(setUsage)
    void readPersistence().then((persistence) =>
      durabilityStore.setState((state) => ({ ...state, persistence }))
    )
  }, [])

  const kept = durability.persistence === "persisted"
  return (
    <div className="flex flex-col gap-4" data-testid="storage-settings">
      <section className="flex gap-3 rounded-lg border p-3">
        <HardDriveIcon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        <div className="flex flex-col gap-1">
          <h3 className="text-sm font-medium">Storage</h3>
          <p
            className="text-sm text-muted-foreground"
            data-testid="storage-usage"
          >
            {!durability.storageSaved
              ? "This window can’t store files, so projects last only until the tab closes."
              : usage
                ? `${projectCount ?? 0} ${projectCount === 1 ? "project" : "projects"}, using ${formatBytes(usage.usage)} of about ${formatBytes(usage.quota)} available.`
                : "Reading how much space is used…"}
          </p>
        </div>
      </section>
      <section className="flex gap-3 rounded-lg border p-3">
        {kept ? (
          <ShieldCheckIcon className="mt-0.5 size-4 shrink-0 text-primary" />
        ) : (
          <ShieldIcon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        )}
        <div className="flex flex-col gap-1">
          <h3 className="text-sm font-medium">Keeping your files</h3>
          <p
            className="text-sm text-muted-foreground"
            data-testid="persistence"
          >
            {kept
              ? "The browser has agreed to keep your projects until you delete them."
              : durability.persistence === "unsupported"
                ? "This browser can’t promise to keep your projects. Download a backup now and then."
                : "The browser may clear your projects if it runs short of space. Download a backup now and then."}
          </p>
          {!kept && durability.persistence !== "unsupported" && (
            <Button
              variant="outline"
              size="sm"
              className="mt-1 self-start"
              onClick={() => void askToPersist()}
            >
              Ask the browser to keep them
            </Button>
          )}
        </div>
      </section>
    </div>
  )
}

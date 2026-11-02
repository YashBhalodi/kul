import { useSelector } from "@tanstack/react-store"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { cancelImport, importStore, resolveImport } from "./import-flow"

const SHOWN = 6

/** Asks what to do when imported files have names the project has. */
export function ImportCollisionDialog() {
  const pending = useSelector(importStore, (state) => state.pending)
  const collisions = pending?.collisions ?? []
  const many = collisions.length !== 1
  return (
    <Dialog
      open={pending !== undefined}
      onOpenChange={(open) => {
        if (!open) cancelImport()
      }}
    >
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {many
              ? `${collisions.length} files have names already in use`
              : "A file has a name already in use"}
          </DialogTitle>
          <DialogDescription>
            {pending?.target === "new"
              ? "The files you picked share these names. Choose what to do with the later ones."
              : "This project already has files with these names, ignoring case. Choose what to do with the imported ones."}
          </DialogDescription>
        </DialogHeader>
        <ul className="flex flex-col gap-1 rounded-lg border bg-muted/40 p-2">
          {collisions.slice(0, SHOWN).map((name, index) => (
            <li key={`${name}-${index}`} className="truncate font-mono text-xs">
              {name}
            </li>
          ))}
          {collisions.length > SHOWN && (
            <li className="text-xs text-muted-foreground">
              and {collisions.length - SHOWN} more
            </li>
          )}
        </ul>
        <DialogFooter>
          <Button variant="outline" onClick={() => resolveImport("skip")}>
            Skip {many ? "them" : "it"}
          </Button>
          <Button variant="outline" onClick={() => resolveImport("replace")}>
            Replace
          </Button>
          <Button onClick={() => resolveImport("keep-both")}>Keep both</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

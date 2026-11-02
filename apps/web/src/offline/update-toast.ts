import { useSelector } from "@tanstack/react-store"
import { useEffect } from "react"
import { toast } from "sonner"

import { applyUpdate, updateStore } from "./updates"

const TOAST_ID = "kul-update"

/**
 * Show the update toast while a new build is waiting, in the page's
 * toaster. Call it from a component that renders the toaster: a parent's
 * effect runs after the toaster's own, so the toaster is listening.
 */
export function useUpdateToast() {
  const ready = useSelector(updateStore, (state) => state.ready)
  useEffect(() => (ready ? showUpdateToast() : undefined), [ready])
}

function showUpdateToast(): () => void {
  toast("A new version of Kul is ready", {
    id: TOAST_ID,
    duration: Infinity,
    action: {
      label: "Reload",
      onClick: (event) => {
        // Keep the toast up until the page reloads.
        event.preventDefault()
        applyUpdate().catch(() => {
          toast.error(
            "Your changes couldn’t be saved, so Kul didn’t reload. Try again."
          )
        })
      },
    },
  })
  return () => {
    toast.dismiss(TOAST_ID)
  }
}

import { useLocation } from "@tanstack/react-router"
import { useSelector } from "@tanstack/react-store"
import { Suspense, lazy } from "react"

import { updateStore } from "./updates"

// The site's pages have no toaster of their own; theirs loads only once an
// update is waiting, so no page pays for it up front.
const SiteUpdateToast = lazy(() => import("./site-update-toast"))

/**
 * The update toast on the landing page and the docs. The app shows it in
 * its own toaster (`useUpdateToast` in the app shell), so it never stacks
 * beside the app's toasts.
 */
export function UpdatePrompt() {
  const ready = useSelector(updateStore, (state) => state.ready)
  const inApp = useLocation({
    select: ({ pathname }) =>
      pathname === "/app" || pathname.startsWith("/app/"),
  })
  if (!ready || inApp) return null
  return (
    <Suspense>
      <SiteUpdateToast />
    </Suspense>
  )
}

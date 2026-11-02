// The page's side of the service worker (src/sw/sw.ts): register it, look
// for a new build now and then, and when one is waiting, say so
// (`updateStore`). The page moves to the new build only when the person
// asks (`applyUpdate`), and only after every task registered with
// `beforeUpdateReload` (the open project's pending writes) has finished.

import { createStore } from "@tanstack/react-store"
import type { Workbox } from "workbox-window"

export interface UpdateState {
  /** A new build is installed and the page can reload into it. */
  ready: boolean
}

export const updateStore = createStore<UpdateState>({ ready: false })

/** How often an open page asks whether a new build is out. */
const CHECK_EVERY_MS = 60 * 60 * 1000

let workbox: Workbox | undefined
let applying = false
const beforeReload = new Set<() => Promise<void>>()

/**
 * Run `task`, and wait for it, before an update reloads the page. Returns
 * the function that unregisters it.
 */
export function beforeUpdateReload(task: () => Promise<void>): () => void {
  beforeReload.add(task)
  return () => {
    beforeReload.delete(task)
  }
}

/**
 * Register the service worker, once per page. Only the production build
 * has one: the dev server serves no /sw.js.
 */
export function startServiceWorker(): void {
  if (!import.meta.env.PROD || workbox || !("serviceWorker" in navigator))
    return
  void import("workbox-window").then(({ Workbox }) => {
    if (workbox) return
    const wb = new Workbox("/sw.js")
    workbox = wb
    wb.addEventListener("waiting", () => {
      void noteWaitingWorker()
    })
    // The new build took over: from this page (reload into it), or from
    // another tab, in which case this page runs the old build under the
    // new worker until the person reloads it.
    wb.addEventListener("controlling", () => {
      if (applying) reloadOnce()
      else updateStore.setState(() => ({ ready: true }))
    })
    // A page no worker controls (after a hard reload) gets no `controlling`
    // event; the worker activating is the cue instead.
    wb.addEventListener("activated", () => {
      if (applying) reloadOnce()
    })
    void wb.register()
    const check = () => {
      if (document.visibilityState === "visible") void wb.update()
    }
    document.addEventListener("visibilitychange", check)
    window.setInterval(check, CHECK_EVERY_MS)
  })
}

/**
 * workbox-window also reports a first install that is slow to activate as
 * waiting. Only a worker waiting behind the one that controls this page is
 * a new build for it.
 */
async function noteWaitingWorker() {
  const registration = await navigator.serviceWorker.getRegistration()
  if (navigator.serviceWorker.controller && registration?.waiting)
    updateStore.setState(() => ({ ready: true }))
}

/**
 * Move this page to the new build: finish the pending work, then let the
 * waiting worker take over (the `controlling` handler reloads). If another
 * tab already let it, reload now. Does nothing while a move is under way;
 * a failed task leaves the page as it is.
 */
export async function applyUpdate(): Promise<void> {
  if (applying || !workbox) return
  applying = true
  try {
    await Promise.all([...beforeReload].map((task) => task()))
  } catch (error) {
    applying = false
    throw error
  }
  const registration = await navigator.serviceWorker.getRegistration()
  if (registration?.waiting) workbox.messageSkipWaiting()
  else reloadOnce()
}

let reloading = false

function reloadOnce() {
  if (reloading) return
  reloading = true
  window.location.reload()
}

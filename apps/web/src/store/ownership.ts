// One tab owns a project: the tab holding the Web Lock `kul-project-<id>`
// writes it; any other tab showing it is read-only. A read-only tab can take
// the project over, which steals the lock; the tab that held it learns so
// and turns read-only. The owner announces each write on a BroadcastChannel
// of the same name, so read-only tabs reload what changed.

export type Access = "owner" | "read-only"

export interface ProjectClaim {
  readonly access: Access
  /** Call `listener` whenever this tab's access changes. */
  onAccessChange: (listener: (access: Access) => void) => () => void
  /** Call `listener` when the owning tab has written the project. */
  onRemoteChange: (listener: () => void) => () => void
  /** Take the project from the tab that owns it. */
  takeOver: () => Promise<void>
  /** Tell the other tabs the project changed (call after each write). */
  announceChange: () => void
  /** Give the project up: release the lock and stop listening. */
  release: () => Promise<void>
}

type Message = { type: "changed" } | { type: "taken" }

export function projectLockName(projectId: string): string {
  return `kul-project-${projectId}`
}

/**
 * Claim the project: own it when no other tab does, otherwise watch it
 * read-only. Without Web Locks every tab owns what it opens.
 */
export async function claimProject(projectId: string): Promise<ProjectClaim> {
  const name = projectLockName(projectId)
  const channel = new BroadcastChannel(name)
  const accessListeners = new Set<(access: Access) => void>()
  const changeListeners = new Set<() => void>()
  let access: Access = "read-only"
  let released = false
  // Resolving this ends the lock callback, which releases the lock.
  let unlock: (() => void) | undefined
  // Settles once the lock request has finished, held or not.
  let request: Promise<unknown> = Promise.resolve()

  function setAccess(next: Access) {
    if (next === access) return
    access = next
    for (const listener of accessListeners) listener(next)
  }

  /** Ask for the lock; resolves to whether this tab now holds it. */
  function lock(options: LockOptions): Promise<boolean> {
    return new Promise((acquired) => {
      request = navigator.locks
        .request(name, options, (held) => {
          if (!held) {
            acquired(false)
            return undefined
          }
          acquired(true)
          return new Promise<void>((resolve) => (unlock = resolve))
        })
        .catch(() => {
          // Another tab stole the lock.
          unlock = undefined
          acquired(false)
          if (!released) setAccess("read-only")
        })
    })
  }

  channel.onmessage = (event: MessageEvent<Message>) => {
    if (released) return
    if (event.data.type === "changed") {
      for (const listener of changeListeners) listener()
    } else if (access === "owner") {
      // Another tab took the project.
      unlock?.()
      unlock = undefined
      setAccess("read-only")
    }
  }

  if (typeof navigator.locks === "undefined") {
    access = "owner"
  } else {
    access = (await lock({ ifAvailable: true })) ? "owner" : "read-only"
  }

  return {
    get access() {
      return access
    },
    onAccessChange(listener) {
      accessListeners.add(listener)
      return () => accessListeners.delete(listener)
    },
    onRemoteChange(listener) {
      changeListeners.add(listener)
      return () => changeListeners.delete(listener)
    },
    async takeOver() {
      if (released || access === "owner") return
      if (typeof navigator.locks !== "undefined") {
        await lock({ steal: true })
      }
      channel.postMessage({ type: "taken" } satisfies Message)
      setAccess("owner")
    },
    announceChange() {
      if (!released && access === "owner") {
        channel.postMessage({ type: "changed" } satisfies Message)
      }
    },
    async release() {
      if (released) return
      released = true
      unlock?.()
      unlock = undefined
      channel.close()
      accessListeners.clear()
      changeListeners.clear()
      await request
    },
  }
}

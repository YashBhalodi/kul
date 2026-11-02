// How safe the projects are: whether this window can store them at all,
// and whether the browser has promised to keep them. The first edit of a
// page asks for that promise; without it, a dismissible reminder suggests
// a backup.

import { createStore } from "@tanstack/react-store"

import {
  dismissBackupReminder,
  isBackupReminderDismissed,
} from "@/prefs/backup-reminder"
import { requestPersistence } from "@/store/durability"
import type { Persistence } from "@/store/durability"

export interface DurabilityState {
  /** False when projects live in memory and end with the tab. */
  storageSaved: boolean
  persistence: Persistence | "unknown"
  reminderVisible: boolean
}

export const durabilityStore = createStore<DurabilityState>({
  storageSaved: true,
  persistence: "unknown",
  reminderVisible: false,
})

let asked = false

export function setStorageSaved(storageSaved: boolean): void {
  durabilityStore.setState((state) => ({ ...state, storageSaved }))
}

/** Call after every edit; the first one asks the browser to keep storage. */
export function noteEdit(): void {
  if (asked) return
  asked = true
  void askToPersist()
}

/** Ask the browser to keep this origin's storage, and record the answer. */
export async function askToPersist(): Promise<Persistence> {
  const persistence = await requestPersistence()
  durabilityStore.setState((state) => ({
    ...state,
    persistence,
    reminderVisible:
      persistence === "best-effort" && !isBackupReminderDismissed(),
  }))
  return persistence
}

export function dismissReminder(): void {
  dismissBackupReminder()
  durabilityStore.setState((state) => ({ ...state, reminderVisible: false }))
}

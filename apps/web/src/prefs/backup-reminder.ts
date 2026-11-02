// Whether the person has dismissed the reminder that this browser may
// clear their projects. Once dismissed, it stays away in this browser.

const DISMISSED_KEY = "kul-backup-reminder-dismissed"

export function isBackupReminderDismissed(): boolean {
  try {
    return window.localStorage.getItem(DISMISSED_KEY) === "1"
  } catch {
    return false
  }
}

export function dismissBackupReminder(): void {
  try {
    window.localStorage.setItem(DISMISSED_KEY, "1")
  } catch {
    // Storage can be unavailable; the reminder is then gone for this page.
  }
}

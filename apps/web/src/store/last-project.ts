// The project opened last, so /app returns to it.

const LAST_PROJECT_KEY = "kul-last-project"

export function readLastProject(): string | undefined {
  try {
    return window.localStorage.getItem(LAST_PROJECT_KEY) ?? undefined
  } catch {
    return undefined
  }
}

export function rememberLastProject(id: string): void {
  try {
    window.localStorage.setItem(LAST_PROJECT_KEY, id)
  } catch {
    // Storage can be unavailable; /app then opens the latest project.
  }
}

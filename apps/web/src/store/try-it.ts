// "Try it" on a docs code block opens the snippet in the app as a new
// project. The snippet travels in this tab's sessionStorage, never in the
// URL: the docs page leaves it here, and /app takes it (once) on arrival.

const KEY = "kul-try-it"

/** A snippet on its way from the docs to the app. */
export interface TryItSnippet {
  /** The docs page it came from, which names the new project. */
  page: string
  /** The snippet's files: usually one, more for a multi-file example. */
  files: Array<{ name: string; text: string }>
}

function isSnippet(value: unknown): value is TryItSnippet {
  if (typeof value !== "object" || value === null) return false
  const { page, files } = value as Record<string, unknown>
  return (
    typeof page === "string" &&
    Array.isArray(files) &&
    files.length > 0 &&
    files.every(
      (file: unknown) =>
        typeof file === "object" &&
        file !== null &&
        typeof (file as Record<string, unknown>).name === "string" &&
        typeof (file as Record<string, unknown>).text === "string"
    )
  )
}

/** Leave `snippet` for the app to pick up. */
export function handOffSnippet(snippet: TryItSnippet): void {
  try {
    sessionStorage.setItem(KEY, JSON.stringify(snippet))
  } catch {
    // Storage is off: the app opens as usual, without the snippet.
  }
}

/** The snippet left for the app, removed so it is used once. */
export function takeHandedOffSnippet(): TryItSnippet | undefined {
  try {
    const text = sessionStorage.getItem(KEY)
    if (text === null) return undefined
    sessionStorage.removeItem(KEY)
    const value: unknown = JSON.parse(text)
    return isSnippet(value) ? value : undefined
  } catch {
    return undefined
  }
}

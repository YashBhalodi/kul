// What the service worker answers from where. Pure, so it is tested in Node.

/**
 * Whether `pathname` is a content-hashed build asset. Its URL names one
 * version of its content for ever, so the precache answers it first.
 */
export function isHashedAsset(pathname: string): boolean {
  return pathname.startsWith("/assets/")
}

/**
 * The precached file that answers `pathname` when the network can't: the
 * SPA shell for every path in the app, a directory's `index.html`, and
 * otherwise the file itself.
 */
export function precachedFileFor(pathname: string): string {
  if (pathname === "/app" || pathname.startsWith("/app/")) return "/_shell.html"
  if (pathname.endsWith("/")) return `${pathname}index.html`
  const name = pathname.slice(pathname.lastIndexOf("/") + 1)
  return name.includes(".") ? pathname : `${pathname}/index.html`
}

/** The message workbox-window's `messageSkipWaiting()` posts. */
export function isSkipWaiting(data: unknown): boolean {
  return (
    typeof data === "object" &&
    data !== null &&
    "type" in data &&
    data.type === "SKIP_WAITING"
  )
}

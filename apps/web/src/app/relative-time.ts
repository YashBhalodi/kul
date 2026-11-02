const MINUTE = 60_000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

const relative = new Intl.RelativeTimeFormat("en", { numeric: "auto" })
const date = new Intl.DateTimeFormat("en", {
  day: "numeric",
  month: "short",
  year: "numeric",
})

/** "Edited just now", "Edited 5 minutes ago", …, "Edited Mar 3, 2026". */
export function editedAgo(iso: string, now: number = Date.now()): string {
  const at = Date.parse(iso)
  if (Number.isNaN(at)) return "Edited"
  const ago = Math.max(0, now - at)
  if (ago < MINUTE) return "Edited just now"
  if (ago < HOUR)
    return `Edited ${relative.format(-Math.floor(ago / MINUTE), "minute")}`
  if (ago < DAY)
    return `Edited ${relative.format(-Math.floor(ago / HOUR), "hour")}`
  if (ago < 7 * DAY)
    return `Edited ${relative.format(-Math.floor(ago / DAY), "day")}`
  return `Edited ${date.format(at)}`
}

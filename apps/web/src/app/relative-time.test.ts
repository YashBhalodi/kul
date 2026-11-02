import { describe, expect, it } from "vitest"

import { editedAgo } from "./relative-time"

const NOW = Date.parse("2026-03-10T12:00:00Z")
const before = (ms: number) => new Date(NOW - ms).toISOString()

describe("editedAgo", () => {
  it("says how long ago, coarsening with age", () => {
    expect(editedAgo(before(10_000), NOW)).toBe("Edited just now")
    expect(editedAgo(before(5 * 60_000), NOW)).toBe("Edited 5 minutes ago")
    expect(editedAgo(before(3 * 3_600_000), NOW)).toBe("Edited 3 hours ago")
    expect(editedAgo(before(26 * 3_600_000), NOW)).toBe("Edited yesterday")
    expect(editedAgo(before(9 * 86_400_000), NOW)).toBe("Edited Mar 1, 2026")
  })

  it("treats a time in the future as now", () => {
    expect(editedAgo(before(-60_000), NOW)).toBe("Edited just now")
  })
})

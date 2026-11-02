import { describe, expect, it, vi } from "vitest"

import { claimProject } from "./ownership"
import type { Access } from "./ownership"

// Two claims in one page stand in for two tabs: Web Locks and
// BroadcastChannel behave the same between them.

describe("claimProject", () => {
  it("makes the first claim the owner and the next one read-only", async () => {
    const id = crypto.randomUUID()
    const first = await claimProject(id)
    const second = await claimProject(id)
    try {
      expect(first.access).toBe("owner")
      expect(second.access).toBe("read-only")
    } finally {
      await first.release()
      await second.release()
    }
  })

  it("hands the project over when a read-only tab takes it", async () => {
    const id = crypto.randomUUID()
    const first = await claimProject(id)
    const second = await claimProject(id)
    const firstAccess: Array<Access> = []
    first.onAccessChange((access) => firstAccess.push(access))
    try {
      await second.takeOver()
      expect(second.access).toBe("owner")
      await vi.waitFor(() => expect(first.access).toBe("read-only"))
      expect(firstAccess).toEqual(["read-only"])

      // And back again.
      await first.takeOver()
      expect(first.access).toBe("owner")
      await vi.waitFor(() => expect(second.access).toBe("read-only"))
    } finally {
      await first.release()
      await second.release()
    }
  })

  it("tells read-only tabs when the owner writes", async () => {
    const id = crypto.randomUUID()
    const owner = await claimProject(id)
    const viewer = await claimProject(id)
    const changed = vi.fn()
    viewer.onRemoteChange(changed)
    try {
      // A read-only tab's announcements go nowhere.
      viewer.announceChange()
      owner.announceChange()
      await vi.waitFor(() => expect(changed).toHaveBeenCalledTimes(1))
    } finally {
      await owner.release()
      await viewer.release()
    }
  })

  it("frees the project for the next claim on release", async () => {
    const id = crypto.randomUUID()
    const first = await claimProject(id)
    await first.release()
    const next = await claimProject(id)
    try {
      expect(next.access).toBe("owner")
    } finally {
      await next.release()
    }
  })

  it("keeps projects independent", async () => {
    const a = await claimProject(crypto.randomUUID())
    const b = await claimProject(crypto.randomUUID())
    try {
      expect([a.access, b.access]).toEqual(["owner", "owner"])
    } finally {
      await a.release()
      await b.release()
    }
  })
})

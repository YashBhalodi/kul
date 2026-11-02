import { describe, expect, it } from "vitest"

import { isHashedAsset, isSkipWaiting, precachedFileFor } from "./routing"

describe("precachedFileFor", () => {
  it("answers every app path with the SPA shell", () => {
    for (const path of ["/app", "/app/", "/app/3f2a/", "/app/3f2a"]) {
      expect(precachedFileFor(path)).toBe("/_shell.html")
    }
  })

  it("answers a directory with its index.html", () => {
    expect(precachedFileFor("/")).toBe("/index.html")
    expect(precachedFileFor("/docs/")).toBe("/docs/index.html")
    expect(precachedFileFor("/docs/reference/cheat-sheet/")).toBe(
      "/docs/reference/cheat-sheet/index.html"
    )
    expect(precachedFileFor("/docs")).toBe("/docs/index.html")
  })

  it("answers a file with itself", () => {
    expect(precachedFileFor("/llms.txt")).toBe("/llms.txt")
    expect(precachedFileFor("/docs/reference/cheat-sheet.md")).toBe(
      "/docs/reference/cheat-sheet.md"
    )
    expect(precachedFileFor("/docs/search.json")).toBe("/docs/search.json")
  })

  it("does not take /application for the app", () => {
    expect(precachedFileFor("/application")).toBe("/application/index.html")
  })
})

describe("isHashedAsset", () => {
  it("is true only under /assets/", () => {
    expect(isHashedAsset("/assets/kul_wasm_bg-BRaD1Xnl.wasm")).toBe(true)
    expect(isHashedAsset("/favicon.svg")).toBe(false)
    expect(isHashedAsset("/docs/assets/x.js")).toBe(false)
  })
})

describe("isSkipWaiting", () => {
  it("recognises only the skip-waiting message", () => {
    expect(isSkipWaiting({ type: "SKIP_WAITING" })).toBe(true)
    expect(isSkipWaiting({ type: "GET_VERSION" })).toBe(false)
    expect(isSkipWaiting("SKIP_WAITING")).toBe(false)
    expect(isSkipWaiting(null)).toBe(false)
  })
})

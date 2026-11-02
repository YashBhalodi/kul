import { describe, expect, it } from "vitest"

import { APP_VERSION } from "@/site"
import { startEngine } from "./engine"

describe("startEngine", () => {
  it("loads kul-wasm in the language worker and reports its versions", async () => {
    const engine = await startEngine()
    try {
      // The web app and the engine release in lockstep.
      expect(engine.info.core).toBe(APP_VERSION)
      expect(engine.info.language).toMatch(/^\d+\.\d+$/)
    } finally {
      engine.terminate()
    }
  })
})

import { createStore } from "@tanstack/react-store"

import type { Engine, EngineInfo } from "@/language/engine"

export type EngineStatus =
  | { state: "starting" }
  | { state: "ready"; info: EngineInfo }
  | { state: "failed"; message: string }

export const engineStatusStore = createStore<EngineStatus>({
  state: "starting",
})

let engine: Promise<Engine> | undefined

/**
 * Start the language engine once per page and track its status. The engine
 * module is imported dynamically, so only the app route ever loads it.
 */
export function bootEngine(): void {
  if (engine) return
  engineStatusStore.setState(() => ({ state: "starting" }))
  engine = import("@/language/engine").then((module) => module.startEngine())
  engine.then(
    ({ info }) => engineStatusStore.setState(() => ({ state: "ready", info })),
    (error: unknown) => {
      engine = undefined
      engineStatusStore.setState(() => ({
        state: "failed",
        message: error instanceof Error ? error.message : String(error),
      }))
    }
  )
}

/** The running engine, starting it if needed. */
export function getEngine(): Promise<Engine> {
  bootEngine()
  if (!engine) throw new Error("The language engine is not running")
  return engine
}

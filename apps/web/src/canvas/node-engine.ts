// The Kul engine in Node, for tests that check the canvas against what the
// engine draws: the built kul-wasm module, loaded synchronously.

import { readFileSync } from "node:fs"
import { createRequire } from "node:module"

import { Workspace, initSync } from "kul-wasm"

let loaded = false

/** A workspace holding `files` (name → text), ready to analyze. */
export function workspaceWith(files: Record<string, string>): Workspace {
  if (!loaded) {
    const wasm = createRequire(import.meta.url).resolve(
      "kul-wasm/kul_wasm_bg.wasm"
    )
    initSync({ module: readFileSync(wasm) })
    loaded = true
  }
  const workspace = new Workspace()
  for (const [name, text] of Object.entries(files)) {
    workspace.setFile(name, text, 1)
  }
  return workspace
}

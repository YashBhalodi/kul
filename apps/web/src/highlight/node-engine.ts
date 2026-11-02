// The Kul engine in Node: the built kul-wasm module, loaded synchronously.
// The build uses it to highlight and check Kul code in the docs and on the
// landing page; tests use it to compare the app with what the engine says.

import { readFileSync } from "node:fs"
import { createRequire } from "node:module"

import { Workspace, diagnosticCodes, initSync } from "kul-wasm"
import type { DiagnosticCode } from "kul-wasm"

let loaded = false

function load(): void {
  if (loaded) return
  const wasm = createRequire(import.meta.url).resolve(
    "kul-wasm/kul_wasm_bg.wasm"
  )
  initSync({ module: readFileSync(wasm) })
  loaded = true
}

/** A workspace holding `files` (name → text), ready to analyze. */
export function workspaceWith(files: Record<string, string>): Workspace {
  load()
  const workspace = new Workspace()
  for (const [name, text] of Object.entries(files)) {
    workspace.setFile(name, text, 1)
  }
  return workspace
}

/** Every diagnostic code the engine can report, in code order. */
export function engineDiagnosticCodes(): Array<DiagnosticCode> {
  load()
  return diagnosticCodes()
}

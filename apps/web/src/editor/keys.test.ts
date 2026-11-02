import { describe, expect, it } from "vitest"

import { appHotkeys } from "@/app/shortcuts"
import { codeMirrorKey } from "./keys"

describe("codeMirrorKey", () => {
  it("joins modifiers with dashes and lowercases a letter", () => {
    expect(codeMirrorKey("Mod+Z")).toBe("Mod-z")
    expect(codeMirrorKey("Mod+Shift+Z")).toBe("Mod-Shift-z")
  })

  it("keeps named keys as they are", () => {
    expect(codeMirrorKey("Shift+F12")).toBe("Shift-F12")
    expect(codeMirrorKey("Control+Space")).toBe("Control-Space")
  })

  it("gives undo and redo the keys the table declares", () => {
    expect(codeMirrorKey(appHotkeys["editor.undo"].defaultHotkey)).toBe("Mod-z")
    expect(codeMirrorKey(appHotkeys["editor.redo"].defaultHotkey)).toBe(
      "Mod-Shift-z"
    )
    expect(codeMirrorKey(appHotkeys["editor.redoAgain"].defaultHotkey)).toBe(
      "Mod-y"
    )
  })
})

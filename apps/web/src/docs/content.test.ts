import { globSync, readFileSync } from "node:fs"
import { fileURLToPath } from "node:url"

import { describe, expect, it } from "vitest"

import { appHotkeys } from "@/app/shortcuts"
import { engineDiagnosticCodes } from "@/highlight/node-engine"
import {
  diagnosticsDrift,
  documentedCodes,
  hotkeyMarkup,
  shortcutRows,
  shortcutsDrift,
} from "./content-checks"

const CONTENT = fileURLToPath(new URL("../../content/docs/", import.meta.url))
const read = (file: string) => readFileSync(`${CONTENT}${file}`, "utf8")

describe("the diagnostics reference", () => {
  it("documents exactly the codes the engine defines", () => {
    const codes = engineDiagnosticCodes().map((entry) => entry.code)
    expect(codes.length).toBeGreaterThan(30)
    const page = read("reference/diagnostics.mdx")
    expect(diagnosticsDrift(page, codes)).toEqual([])
    expect(documentedCodes(page)).toEqual(codes)
  })

  it("reports a missing entry, an invented one and a missing example", () => {
    const page = [
      "### KUL-R03: Missing name [#kul-r03]",
      "",
      "No example here.",
      "",
      "### KUL-R99: Not a code [#kul-r99]",
      "",
      "```kul expect-error=KUL-R99",
      "```",
    ].join("\n")
    expect(diagnosticsDrift(page, ["KUL-R03", "KUL-R04"])).toEqual([
      "KUL-R99 is documented, but the engine defines no such code",
      "KUL-R04 has no entry in the reference",
      "KUL-R03's entry needs an example fenced ```kul expect-error=KUL-R03",
    ])
  })
})

describe("the keyboard shortcuts page", () => {
  const declared = Object.values(appHotkeys).map((entry) => ({
    hotkey: String(entry.hotkey),
    name: entry.meta?.name ?? "",
  }))

  it("lists every shortcut the app declares, and no other", () => {
    const page = read("app/keyboard-shortcuts.mdx")
    expect(shortcutsDrift(page, declared)).toEqual([])
    expect(shortcutRows(page)).toHaveLength(declared.length)
  })

  it("writes keys the way the page does", () => {
    expect(hotkeyMarkup("Mod+Shift+O")).toBe(
      "<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>O</kbd>"
    )
    expect(hotkeyMarkup("+")).toBe("<kbd>+</kbd>")
    expect(hotkeyMarkup("Escape")).toBe("<kbd>Esc</kbd>")
  })
})

describe("the docs pages", () => {
  const pages = globSync("**/*.mdx", { cwd: CONTENT }).sort()

  it("cover every part of the outline", () => {
    expect(pages).toEqual(
      [
        "index.mdx",
        "quick-start.mdx",
        "guide/people.mdx",
        "guide/dates.mdx",
        "guide/marriages.mdx",
        "guide/children.mdx",
        "guide/multiple-marriages.mdx",
        "guide/disconnected-lineages.mdx",
        "guide/files.mdx",
        "guide/comments-and-formatting.mdx",
        "reading-the-tree.mdx",
        "app/editor.mdx",
        "app/canvas.mdx",
        "app/projects.mdx",
        "app/files-and-backups.mdx",
        "app/keyboard-shortcuts.mdx",
        "reference/cheat-sheet.mdx",
        "reference/fields.mdx",
        "reference/diagnostics.mdx",
        "command-line.mdx",
      ].sort()
    )
  })

  it("each have a title and a one-sentence description", () => {
    for (const page of pages) {
      const front = /^---\n([\s\S]*?)\n---\n/.exec(read(page))?.[1] ?? ""
      expect(front, page).toMatch(/^title: \S/m)
      const description = /^description: "?(.+?)"?$/m.exec(front)?.[1] ?? ""
      expect(description.length, page).toBeGreaterThan(30)
      expect(description, page).toMatch(/\.$/)
    }
  })
})

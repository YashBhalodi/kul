// The project store's contract, run against every backend: in memory under
// Node (project-store.test.ts) and on a real origin private file system in
// Chromium and WebKit (project-store.browser.test.ts).

import { beforeEach, describe, expect, it } from "vitest"

import type { StorageBackend } from "./backend"
import { EMPTY_UI, StoreError, createProjectStore } from "./project-store"
import type { ProjectStore } from "./project-store"

function clock(): () => string {
  let tick = Date.parse("2026-01-01T00:00:00.000Z")
  return () => new Date((tick += 1000)).toISOString()
}

async function expectStoreError(promise: Promise<unknown>, code: string) {
  const error: unknown = await promise.then(
    () => undefined,
    (reason: unknown) => reason
  )
  expect(error).toBeInstanceOf(StoreError)
  expect((error as StoreError).code).toBe(code)
}

export function projectStoreContract(
  backendName: string,
  makeBackend: () => Promise<StorageBackend>
): void {
  describe(`the project store on ${backendName}`, () => {
    let backend: StorageBackend
    let store: ProjectStore

    beforeEach(async () => {
      backend = await makeBackend()
      store = createProjectStore(backend, { now: clock() })
    })

    it("starts with no projects", async () => {
      expect(await store.listProjects()).toEqual([])
    })

    it("creates a project with its files, in file order", async () => {
      const project = await store.createProject("  The Okeke family ", [
        { name: "b.kul", text: "person b" },
        { name: "a.kul", text: "person a" },
      ])
      expect(project.name).toBe("The Okeke family")
      expect(await store.readProject(project.id)).toEqual(project)
      expect(await store.readFiles(project.id)).toEqual([
        { name: "a.kul", text: "person a" },
        { name: "b.kul", text: "person b" },
      ])
      expect(await store.readUi(project.id)).toEqual(EMPTY_UI)
    })

    it("lists projects, most recently edited first", async () => {
      const first = await store.createProject("First", [])
      const second = await store.createProject("Second", [])
      expect((await store.listProjects()).map((p) => p.name)).toEqual([
        "Second",
        "First",
      ])
      await store.writeFile(first.id, "a.kul", "")
      expect((await store.listProjects()).map((p) => p.id)).toEqual([
        first.id,
        second.id,
      ])
    })

    it("orders files by the bytes of their UTF-8 names", async () => {
      // "😀" (U+1F600) sorts after "ｚ" (U+FF5A) in UTF-8, though not as
      // UTF-16 code units; uppercase sorts before lowercase.
      const names = ["😀.kul", "ｚ.kul", "b.kul", "B2.kul", "10.kul", "9.kul"]
      const project = await store.createProject(
        "Order",
        names.map((name) => ({ name, text: "" }))
      )
      expect((await store.readFiles(project.id)).map((f) => f.name)).toEqual([
        "10.kul",
        "9.kul",
        "B2.kul",
        "b.kul",
        "ｚ.kul",
        "😀.kul",
      ])
    })

    it("refuses invalid and clashing file names", async () => {
      await expectStoreError(
        store.createProject("Bad", [{ name: "notes.txt", text: "" }]),
        "extension"
      )
      await expectStoreError(
        store.createProject("Clash", [
          { name: "a.kul", text: "" },
          { name: "A.kul", text: "" },
        ]),
        "taken"
      )
      expect(await store.listProjects()).toEqual([])

      const project = await store.createProject("P", [
        { name: "Smith.kul", text: "" },
      ])
      await expectStoreError(
        store.writeFile(project.id, "smith.kul", ""),
        "taken"
      )
      await expectStoreError(
        store.writeFile(project.id, "a/b.kul", ""),
        "separator"
      )
    })

    it("writes, renames and deletes files, bumping the edit time", async () => {
      const project = await store.createProject("P", [
        { name: "a.kul", text: "one" },
      ])
      await store.writeFile(project.id, "a.kul", "two")
      await store.writeFile(project.id, "b.kul", "three")
      await store.renameFile(project.id, "a.kul", "c.kul")
      expect(await store.readFiles(project.id)).toEqual([
        { name: "b.kul", text: "three" },
        { name: "c.kul", text: "two" },
      ])
      await store.deleteFile(project.id, "b.kul")
      expect(await store.readFiles(project.id)).toEqual([
        { name: "c.kul", text: "two" },
      ])
      const after = await store.readProject(project.id)
      expect(after && after.updatedAt > project.updatedAt).toBe(true)
      expect(after?.createdAt).toBe(project.createdAt)
    })

    it("renames a file to a change of case only", async () => {
      const project = await store.createProject("P", [
        { name: "smith.kul", text: "person s" },
        { name: "other.kul", text: "" },
      ])
      await store.renameFile(project.id, "smith.kul", "Smith.kul")
      expect(await store.readFiles(project.id)).toEqual([
        { name: "Smith.kul", text: "person s" },
        { name: "other.kul", text: "" },
      ])
      await expectStoreError(
        store.renameFile(project.id, "Smith.kul", "OTHER.kul"),
        "taken"
      )
      await expectStoreError(
        store.renameFile(project.id, "missing.kul", "x.kul"),
        "file-not-found"
      )
    })

    it("keeps the window state, following renamed and deleted files", async () => {
      const project = await store.createProject(
        "P",
        [
          { name: "a.kul", text: "" },
          { name: "b.kul", text: "" },
        ],
        {
          editorWindow: {
            x: 10,
            y: 20,
            width: 400,
            height: 300,
            dock: "left",
            minimized: false,
          },
          openTabs: ["a.kul", "b.kul"],
          activeFile: "a.kul",
          camera: { x: 1, y: 2, zoom: 0.5 },
        }
      )
      await store.renameFile(project.id, "a.kul", "first.kul")
      expect(await store.readUi(project.id)).toEqual({
        editorWindow: {
          x: 10,
          y: 20,
          width: 400,
          height: 300,
          dock: "left",
          minimized: false,
        },
        openTabs: ["first.kul", "b.kul"],
        activeFile: "first.kul",
        camera: { x: 1, y: 2, zoom: 0.5 },
      })
      await store.deleteFile(project.id, "first.kul")
      const ui = await store.readUi(project.id)
      expect(ui.openTabs).toEqual(["b.kul"])
      expect(ui.activeFile).toBe("b.kul")

      // A window-state write is not an edit, and drops unknown files.
      const before = await store.readProject(project.id)
      await store.writeUi(project.id, {
        ...ui,
        openTabs: ["b.kul", "gone.kul"],
        activeFile: "gone.kul",
      })
      expect(await store.readUi(project.id)).toEqual({
        ...ui,
        openTabs: ["b.kul"],
        activeFile: null,
      })
      expect(await store.readProject(project.id)).toEqual(before)
    })

    it("renames, duplicates and deletes projects", async () => {
      const project = await store.createProject(
        "Okeke",
        [{ name: "a.kul", text: "person a" }],
        { ...EMPTY_UI, activeFile: "a.kul", openTabs: ["a.kul"] }
      )
      const renamed = await store.renameProject(project.id, "The Okekes")
      expect(renamed.name).toBe("The Okekes")
      await expectStoreError(
        store.renameProject(project.id, "   "),
        "invalid-project-name"
      )

      const copy = await store.duplicateProject(project.id, "The Okekes (copy)")
      expect(copy.id).not.toBe(project.id)
      expect(await store.readFiles(copy.id)).toEqual(
        await store.readFiles(project.id)
      )
      expect((await store.readUi(copy.id)).activeFile).toBe("a.kul")

      // The copy is independent of the original.
      await store.writeFile(copy.id, "a.kul", "changed")
      expect(await store.readFiles(project.id)).toEqual([
        { name: "a.kul", text: "person a" },
      ])

      await store.deleteProject(project.id)
      expect(await store.readProject(project.id)).toBeUndefined()
      expect((await store.listProjects()).map((p) => p.id)).toEqual([copy.id])
      await expectStoreError(store.readFiles(project.id), "project-not-found")
    })

    it("runs one project's operations in the order they were called", async () => {
      const project = await store.createProject("P", [])
      await Promise.all([
        store.writeFile(project.id, "a.kul", "1"),
        store.writeFile(project.id, "a.kul", "2"),
        store.renameFile(project.id, "a.kul", "b.kul"),
        store.writeFile(project.id, "b.kul", "3"),
      ])
      expect(await store.readFiles(project.id)).toEqual([
        { name: "b.kul", text: "3" },
      ])
    })

    it("ignores folders without a readable project.json", async () => {
      await backend.writeText(["projects", "broken"], "project.json", "{")
      await backend.writeText(["projects", "half", "files"], "a.kul", "")
      const project = await store.createProject("Real", [])
      expect((await store.listProjects()).map((p) => p.id)).toEqual([
        project.id,
      ])
    })

    it("lists only .kul files", async () => {
      const project = await store.createProject("P", [
        { name: "a.kul", text: "" },
      ])
      await backend.writeText(
        ["projects", project.id, "files"],
        "notes.txt",
        ""
      )
      expect((await store.readFiles(project.id)).map((f) => f.name)).toEqual([
        "a.kul",
      ])
    })
  })
}

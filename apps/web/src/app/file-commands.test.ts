import { describe, expect, it } from "vitest"

import { openFileCommands } from "./file-commands"

describe("openFileCommands", () => {
  it("offers to open each file, in the order given, in the Files group", () => {
    const commands = openFileCommands(["01-a.kul", "02-b.kul"], () => undefined)
    expect(
      commands.map((command) => [command.id, command.title, command.group])
    ).toEqual([
      ["file.open.01-a.kul", "Open 01-a.kul", "Files"],
      ["file.open.02-b.kul", "Open 02-b.kul", "Files"],
    ])
  })

  it("opens the file it names", () => {
    const opened: Array<string> = []
    const [, second] = openFileCommands(["a.kul", "b.kul"], (name) =>
      opened.push(name)
    )
    second?.run()
    expect(opened).toEqual(["b.kul"])
  })

  it("offers nothing for a project with no files", () => {
    expect(openFileCommands([], () => undefined)).toEqual([])
  })
})

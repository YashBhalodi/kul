import { afterEach, describe, expect, it } from "vitest"

import { commandStore, groupCommands, registerCommands } from "./commands"
import type { Command } from "./commands"

function command(id: string, group: string): Command {
  return { id, title: id, group, run: () => undefined }
}

afterEach(() => commandStore.setState(() => new Map()))

describe("registerCommands", () => {
  it("lists registered commands until they are unregistered", () => {
    const unregister = registerCommands([command("a", "G"), command("b", "G")])
    expect([...commandStore.state.keys()]).toEqual(["a", "b"])
    unregister()
    expect(commandStore.state.size).toBe(0)
  })

  it("replaces a command registered under the same id", () => {
    registerCommands([command("a", "Old")])
    registerCommands([command("a", "New")])
    expect(commandStore.state.get("a")?.group).toBe("New")
  })

  it("leaves a replacement in place when the replaced registration ends", () => {
    const first = registerCommands([command("a", "Old")])
    registerCommands([command("a", "New")])
    first()
    expect(commandStore.state.get("a")?.group).toBe("New")
  })
})

describe("groupCommands", () => {
  it("groups commands in order of each group's first command", () => {
    registerCommands([
      command("theme", "Appearance"),
      command("docs", "Help"),
      command("dark", "Appearance"),
    ])
    expect(
      groupCommands(commandStore.state).map((group) => [
        group.name,
        group.commands.map((c) => c.id),
      ])
    ).toEqual([
      ["Appearance", ["theme", "dark"]],
      ["Help", ["docs"]],
    ])
  })
})

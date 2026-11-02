import type { Command } from "./commands"

/**
 * One palette command per file of the open project, in file order:
 * "Open family.kul" opens the file in the editor (`open`).
 */
export function openFileCommands(
  names: ReadonlyArray<string>,
  open: (name: string) => unknown
): Array<Command> {
  return names.map((name) => ({
    id: `file.open.${name}`,
    title: `Open ${name}`,
    group: "Files",
    keywords: ["go to", "switch", "tab", "show"],
    run: () => void open(name),
  }))
}

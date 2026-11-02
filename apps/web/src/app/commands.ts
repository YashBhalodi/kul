// The command palette's commands. A component registers the commands it can
// run with `useCommands` while it is mounted, so a command is listed exactly
// when the state it acts on exists.

import { createStore } from "@tanstack/react-store"
import { useEffect } from "react"

import type { AppHotkeyId } from "./shortcuts"

export interface Command {
  /** Stable id, unique across the app. */
  id: string
  /** Sentence-case label, as the palette shows it. */
  title: string
  /** The palette group the command is listed under. */
  group: string
  /** Extra words the palette's search matches. */
  keywords?: ReadonlyArray<string>
  /** The shortcut that runs the same action, shown beside the title. */
  hotkey?: AppHotkeyId
  run: () => void
}

/** Commands by id, in registration order. */
export const commandStore = createStore<ReadonlyMap<string, Command>>(new Map())

/**
 * Add commands; a command with an id already registered replaces it.
 * Returns the function that removes exactly these registrations.
 */
export function registerCommands(commands: ReadonlyArray<Command>): () => void {
  commandStore.setState((current) => {
    const next = new Map(current)
    for (const command of commands) next.set(command.id, command)
    return next
  })
  return () =>
    commandStore.setState((current) => {
      const next = new Map(current)
      for (const command of commands) {
        if (next.get(command.id) === command) next.delete(command.id)
      }
      return next
    })
}

/** Register `commands` while the calling component is mounted. */
export function useCommands(commands: ReadonlyArray<Command>): void {
  useEffect(() => registerCommands(commands), [commands])
}

export interface CommandGroup {
  name: string
  commands: Array<Command>
}

/** Commands grouped for display, groups in order of first registration. */
export function groupCommands(
  commands: ReadonlyMap<string, Command>
): Array<CommandGroup> {
  const groups = new Map<string, CommandGroup>()
  for (const command of commands.values()) {
    let group = groups.get(command.group)
    if (!group) {
      group = { name: command.group, commands: [] }
      groups.set(command.group, group)
    }
    group.commands.push(command)
  }
  return [...groups.values()]
}

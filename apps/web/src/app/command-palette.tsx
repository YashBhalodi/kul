import { useFormatForDisplay } from "@tanstack/react-hotkeys"
import { useSelector } from "@tanstack/react-store"
import { useRef } from "react"

import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from "@/components/ui/command"
import { appStore, dialogControl } from "./app-store"
import { commandStore, groupCommands } from "./commands"
import { appHotkeys } from "./shortcuts"

/** The command palette: every registered command, searchable. */
export function CommandPalette() {
  const control = dialogControl("palette")
  const open = useSelector(appStore, control.open)
  const commands = useSelector(commandStore)
  const format = useFormatForDisplay()
  const input = useRef<HTMLInputElement>(null)
  return (
    <CommandDialog
      open={open}
      onOpenChange={control.onOpenChange}
      title="Command palette"
      description="Search for a command to run"
      className="sm:max-w-lg"
      initialFocus={input}
    >
      <Command>
        <CommandInput ref={input} placeholder="Type a command or search…" />
        <CommandList className="max-h-96">
          <CommandEmpty>No command matches that search.</CommandEmpty>
          {groupCommands(commands).map((group) => (
            <CommandGroup key={group.name} heading={group.name}>
              {group.commands.map((command) => {
                const hotkey =
                  command.hotkey && appHotkeys[command.hotkey].hotkey
                return (
                  <CommandItem
                    key={command.id}
                    value={command.id}
                    keywords={[command.title, ...(command.keywords ?? [])]}
                    onSelect={() => {
                      appStore.actions.hide("palette")
                      command.run()
                    }}
                  >
                    {command.title}
                    {hotkey && (
                      <CommandShortcut>{format(hotkey)}</CommandShortcut>
                    )}
                  </CommandItem>
                )
              })}
            </CommandGroup>
          ))}
        </CommandList>
      </Command>
    </CommandDialog>
  )
}

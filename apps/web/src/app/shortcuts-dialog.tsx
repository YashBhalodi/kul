import { useAppHotkeyList, useFormatForDisplay } from "@tanstack/react-hotkeys"
import { useSelector } from "@tanstack/react-store"

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Kbd } from "@/components/ui/kbd"
import { appStore } from "./app-store"

/** The cheat sheet: every declared shortcut, by group. */
export function ShortcutsDialog() {
  const open = useSelector(appStore, (state) => state.shortcutsOpen)
  const items = useAppHotkeyList()
  const format = useFormatForDisplay()

  const groups = new Map<string, Array<{ name: string; keys: string }>>()
  for (const { appHotkey } of items) {
    if (!appHotkey.hotkey) continue
    const group = appHotkey.meta?.group ?? "General"
    const rows = groups.get(group) ?? []
    rows.push({
      name: appHotkey.meta?.name ?? appHotkey.id,
      keys: [format(appHotkey.hotkey)].flat(2).join(" then "),
    })
    groups.set(group, rows)
  }

  return (
    <Dialog open={open} onOpenChange={appStore.actions.setShortcutsOpen}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Keyboard shortcuts</DialogTitle>
          <DialogDescription>
            Press ? outside a text field to open this list.
          </DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-5">
          {[...groups].map(([group, rows]) => (
            <section key={group} className="flex flex-col gap-2">
              <h3 className="text-xs font-medium text-muted-foreground">
                {group}
              </h3>
              <dl className="flex flex-col gap-1.5">
                {rows.map((row) => (
                  <div
                    key={row.name}
                    className="flex items-center justify-between gap-4 text-sm"
                  >
                    <dt>{row.name}</dt>
                    <dd>
                      <Kbd className="text-foreground">{row.keys}</Kbd>
                    </dd>
                  </div>
                ))}
              </dl>
            </section>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  )
}

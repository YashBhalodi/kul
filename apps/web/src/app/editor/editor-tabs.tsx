import { useHotkey } from "@tanstack/react-hotkeys"
import { useSelector } from "@tanstack/react-store"
import { cn } from "cn"
import { XIcon } from "lucide-react"

import { closeTab, selectFile, sessionStore } from "../project-session"
import { appHotkeys } from "../shortcuts"

/** The open files, as tabs over the editor. */
export function EditorTabs() {
  const tabs = useSelector(sessionStore, (state) => state.ui.openTabs)
  const activeFile = useSelector(sessionStore, (state) => state.ui.activeFile)

  useHotkey(appHotkeys["tab.close"], () => {
    if (activeFile) void closeTab(activeFile)
  })

  if (tabs.length === 0) return null
  return (
    <nav
      aria-label="Open files"
      className="flex h-9 shrink-0 items-end overflow-x-auto border-b bg-muted/40 px-1"
    >
      <ul className="flex items-end gap-px">
        {tabs.map((name) => {
          const active = name === activeFile
          return (
            <li
              key={name}
              data-testid="editor-tab"
              data-name={name}
              data-active={active || undefined}
              className="group/tab -mb-px flex h-8 shrink-0 items-center rounded-t-md border border-b-0 border-transparent data-active:border-border data-active:bg-card"
            >
              <button
                type="button"
                aria-current={active ? "page" : undefined}
                onClick={() => void selectFile(name)}
                onAuxClick={(event) => {
                  if (event.button === 1) void closeTab(name)
                }}
                className={cn(
                  "h-full rounded-t-md pr-1 pl-3 font-mono text-[0.8125rem] outline-none focus-visible:ring-3 focus-visible:ring-ring/50",
                  active
                    ? "text-foreground"
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                {name}
              </button>
              <button
                type="button"
                aria-label={`Close ${name}`}
                onClick={() => void closeTab(name)}
                className="mr-1 flex size-5 items-center justify-center rounded-sm text-muted-foreground opacity-0 outline-none group-hover/tab:opacity-100 group-data-active/tab:opacity-100 hover:bg-muted hover:text-foreground focus-visible:opacity-100 focus-visible:ring-3 focus-visible:ring-ring/50"
              >
                <XIcon className="size-3.5" />
              </button>
            </li>
          )
        })}
      </ul>
    </nav>
  )
}

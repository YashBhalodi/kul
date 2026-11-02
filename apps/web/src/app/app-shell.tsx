import { HotkeysProvider, useHotkey } from "@tanstack/react-hotkeys"
import { useNavigate } from "@tanstack/react-router"
import { useMemo } from "react"
import type { ReactNode } from "react"

import { Toaster } from "@/components/ui/sonner"
import { TooltipProvider } from "@/components/ui/tooltip"
import { setThemePreference, toggleTheme } from "@/prefs/theme"
import { appStore } from "./app-store"
import { CommandPalette } from "./command-palette"
import { useCommands } from "./commands"
import type { Command } from "./commands"
import { DesktopGate } from "./desktop-gate"
import { ImportCollisionDialog } from "./import-dialog"
import { ImportDropZone } from "./import-drop-zone"
import { ImportPicker } from "./import-picker"
import { useProjectCommands } from "./project-commands"
import { DeleteProjectDialog, RenameProjectDialog } from "./project-dialogs"
import { SettingsDialog } from "./settings-dialog"
import { ShortcutsDialog } from "./shortcuts-dialog"
import { appHotkeys } from "./shortcuts"
import { TopBar } from "./top-bar"

/**
 * The app: the top bar over the canvas, the command palette, the dialogs
 * and the import machinery. Smaller screens get the desktop-only notice
 * instead.
 */
export function AppShell({ children }: { children: ReactNode }) {
  return (
    <HotkeysProvider appHotkeys={appHotkeys}>
      <TooltipProvider delay={400}>
        <DesktopGate>
          <AppFrame>{children}</AppFrame>
        </DesktopGate>
      </TooltipProvider>
    </HotkeysProvider>
  )
}

function AppFrame({ children }: { children: ReactNode }) {
  useHotkey(appHotkeys["palette.open"], appStore.actions.togglePalette)
  useHotkey(appHotkeys["shortcuts.open"], showShortcuts)
  useProjectCommands()
  useCommands(useShellCommands())
  return (
    <div className="flex h-svh flex-col overflow-hidden bg-canvas">
      <TopBar />
      <main className="relative flex flex-1">
        <ImportDropZone>{children}</ImportDropZone>
      </main>
      <CommandPalette />
      <ShortcutsDialog />
      <SettingsDialog />
      <RenameProjectDialog />
      <DeleteProjectDialog />
      <ImportCollisionDialog />
      <ImportPicker />
      <Toaster position="bottom-right" />
    </div>
  )
}

const showShortcuts = () => appStore.actions.show("shortcuts")

function useShellCommands(): ReadonlyArray<Command> {
  const navigate = useNavigate()
  return useMemo(
    () => [
      {
        id: "theme.toggle",
        title: "Toggle theme",
        group: "Appearance",
        keywords: ["dark", "light", "mode"],
        run: toggleTheme,
      },
      {
        id: "theme.system",
        title: "Use the system theme",
        group: "Appearance",
        run: () => setThemePreference("system"),
      },
      {
        id: "theme.light",
        title: "Use the light theme",
        group: "Appearance",
        run: () => setThemePreference("light"),
      },
      {
        id: "theme.dark",
        title: "Use the dark theme",
        group: "Appearance",
        run: () => setThemePreference("dark"),
      },
      {
        id: "help.shortcuts",
        title: "Keyboard shortcuts",
        group: "Help",
        hotkey: "shortcuts.open",
        run: showShortcuts,
      },
      {
        id: "help.docs",
        title: "Open the docs",
        group: "Help",
        keywords: ["learn", "guide", "help"],
        run: () => void navigate({ to: "/docs/" }),
      },
      {
        id: "help.home",
        title: "Go to the home page",
        group: "Help",
        run: () => void navigate({ to: "/" }),
      },
    ],
    [navigate]
  )
}

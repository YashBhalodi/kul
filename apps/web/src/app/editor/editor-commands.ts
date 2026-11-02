import { useSelector } from "@tanstack/react-store"
import { useMemo } from "react"

import { useCommands } from "../commands"
import type { Command } from "../commands"
import { closeTab, sessionStore } from "../project-session"
import {
  editorFeatureCommands,
  formatAllFiles,
  formatFile,
  runOnEditor,
} from "./editor-controller"
import { showProblems, toggleEditor } from "./editor-layout"

/** Register the editor's palette commands while it is mounted. */
export function useEditorCommands(): void {
  const activeFile = useSelector(sessionStore, (state) => state.ui.activeFile)
  const owner = useSelector(sessionStore, (state) => state.access === "owner")

  const commands = useMemo((): ReadonlyArray<Command> => {
    const list: Array<Command> = [
      {
        id: "editor.toggle",
        title: "Show or hide the editor",
        group: "Editor",
        keywords: ["minimize", "restore", "toggle"],
        hotkey: "editor.toggle",
        run: toggleEditor,
      },
      {
        id: "editor.problems",
        title: "Show problems",
        group: "Editor",
        keywords: ["errors", "warnings", "diagnostics"],
        run: showProblems,
      },
    ]
    if (activeFile) {
      list.push(
        {
          id: "editor.symbols",
          title: `Go to a person or marriage in ${activeFile}`,
          group: "Editor",
          keywords: ["symbol", "outline", "jump"],
          hotkey: "editor.symbols",
          run: () => runOnEditor(editorFeatureCommands.showSymbols),
        },
        {
          id: "tab.close",
          title: `Close ${activeFile}`,
          group: "Editor",
          keywords: ["tab"],
          hotkey: "tab.close",
          run: () => void closeTab(activeFile),
        }
      )
      if (owner) {
        list.push({
          id: "editor.format",
          title: `Format ${activeFile}`,
          group: "Editor",
          keywords: ["tidy", "indent"],
          hotkey: "editor.format",
          run: () => void formatFile(activeFile),
        })
      }
    }
    if (owner) {
      list.push({
        id: "editor.format-all",
        title: "Format all files",
        group: "Editor",
        keywords: ["tidy", "indent", "project"],
        run: () => void formatAllFiles(),
      })
    }
    return list
  }, [activeFile, owner])

  useCommands(commands)
}

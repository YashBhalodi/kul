import { useSelector } from "@tanstack/react-store"
import { useMemo } from "react"

import { useCommands } from "../commands"
import type { Command } from "../commands"
import { downloadTree } from "../download"
import { analysisStore } from "../engine-status"
import { clearCanvasSelection } from "./canvas-linking"
import {
  fitTree,
  legendStore,
  marksStore,
  toggleLegend,
  zoomStep,
  zoomToActualSize,
} from "./canvas-store"

/** Register the canvas's palette commands while it is mounted. */
export function useCanvasCommands(): void {
  const legendOpen = useSelector(legendStore, (state) => state.open)
  const selected = useSelector(
    marksStore,
    (marks) => marks.selected !== undefined
  )
  const drawable = useSelector(
    analysisStore,
    (state) => state.report?.svg?.includes('class="kul-card"') ?? false
  )

  const commands = useMemo((): ReadonlyArray<Command> => {
    const list: Array<Command> = [
      {
        id: "canvas.fit",
        title: "Fit the tree to the canvas",
        group: "Canvas",
        keywords: ["zoom", "view", "whole"],
        hotkey: "canvas.fit",
        run: fitTree,
      },
      {
        id: "canvas.actualSize",
        title: "Show the tree at its actual size",
        group: "Canvas",
        keywords: ["zoom", "100%", "reset"],
        run: zoomToActualSize,
      },
      {
        id: "canvas.zoomIn",
        title: "Zoom in",
        group: "Canvas",
        hotkey: "canvas.zoomIn",
        run: () => zoomStep(1),
      },
      {
        id: "canvas.zoomOut",
        title: "Zoom out",
        group: "Canvas",
        hotkey: "canvas.zoomOut",
        run: () => zoomStep(-1),
      },
      {
        id: "canvas.legend",
        title: legendOpen ? "Hide the legend" : "Show the legend",
        group: "Canvas",
        keywords: ["key", "colours"],
        run: toggleLegend,
      },
    ]
    if (selected) {
      list.push({
        id: "canvas.clearSelection",
        title: "Clear the selection",
        group: "Canvas",
        hotkey: "canvas.clearSelection",
        run: clearCanvasSelection,
      })
    }
    if (drawable) {
      list.push({
        id: "canvas.download",
        title: "Download the tree (.svg)",
        group: "Canvas",
        keywords: ["export", "image", "svg"],
        run: () => void downloadTree(),
      })
    }
    return list
  }, [legendOpen, selected, drawable])

  useCommands(commands)
}

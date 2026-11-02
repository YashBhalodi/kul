import { useSelector } from "@tanstack/react-store"
import { useLayoutEffect, useRef, useState } from "react"

import { canvasSizeStore, currentLayout } from "./editor/editor-layout"
import { freeCanvasInsets } from "./editor/window-geometry"
import { analysisStore } from "./engine-status"
import { sessionStore } from "./project-session"

// Room kept clear around the fitted tree, in CSS pixels.
const MARGIN = 48

/**
 * The open project's family tree, drawn whole and fitted to the canvas
 * beside the editor window (never past 100%). It redraws when the files
 * change, keeping the last tree while the project has errors.
 */
export function TreePreview() {
  const lastSvg = useSelector(analysisStore, (state) => state.lastSvg)
  const stale = useSelector(
    analysisStore,
    (state) => state.report !== undefined && state.report.svg === undefined
  )
  if (!lastSvg) return null
  return <FittedSvg svg={lastSvg} stale={stale} />
}

function FittedSvg({ svg, stale }: { svg: string; stale: boolean }) {
  const frame = useRef<HTMLDivElement>(null)
  const editorWindow = useSelector(
    sessionStore,
    (state) => state.ui.editorWindow
  )
  const canvas = useSelector(canvasSizeStore)
  const insets = freeCanvasInsets(currentLayout(editorWindow, canvas), canvas)
  const [scale, setScale] = useState(1)

  useLayoutEffect(() => {
    const element = frame.current
    const drawing = element?.querySelector("svg")
    if (!element || !drawing) return
    const width = Number(drawing.getAttribute("width")) || 1
    const height = Number(drawing.getAttribute("height")) || 1
    const fit = () =>
      setScale(
        Math.min(
          1,
          (element.clientWidth - 2 * MARGIN) / width,
          (element.clientHeight - 2 * MARGIN) / height
        )
      )
    fit()
    const observer = new ResizeObserver(fit)
    observer.observe(element)
    return () => observer.disconnect()
  }, [svg])

  return (
    <div
      ref={frame}
      data-testid="tree"
      data-stale={stale || undefined}
      style={{ left: insets.left, right: insets.right }}
      className="tree-preview absolute inset-y-0 flex items-center justify-center overflow-hidden transition-opacity data-stale:opacity-40"
    >
      <div
        // The SVG comes from the engine, which escapes every text it draws.
        dangerouslySetInnerHTML={{ __html: svg }}
        style={{ transform: `scale(${scale})` }}
        className="shrink-0"
      />
    </div>
  )
}

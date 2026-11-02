import { useSelector } from "@tanstack/react-store"
import { useEffect, useLayoutEffect, useRef, useState } from "react"

import type { TreeRender } from "@/language/engine"
import { getEngine } from "./engine-status"
import { sessionStore } from "./project-session"

// Room kept clear around the fitted tree, in CSS pixels.
const MARGIN = 48

/**
 * The open project's family tree, drawn whole and fitted to the canvas
 * (never past 100%). It redraws when the files change, keeping the last
 * tree while the project has errors.
 */
export function TreePreview() {
  const files = useSelector(sessionStore, (state) => state.files)
  const [render, setRender] = useState<TreeRender>()
  const [lastSvg, setLastSvg] = useState<string>()

  useEffect(() => {
    let current = true
    void getEngine()
      .then((engine) => engine.api.render(files))
      .then((result) => {
        if (!current) return
        setRender(result)
        if (result.svg) setLastSvg(result.svg)
      })
      .catch(() => undefined)
    return () => {
      current = false
    }
  }, [files])

  if (!lastSvg) return null
  return (
    <FittedSvg
      svg={lastSvg}
      stale={render !== undefined && render.svg === undefined}
    />
  )
}

function FittedSvg({ svg, stale }: { svg: string; stale: boolean }) {
  const frame = useRef<HTMLDivElement>(null)
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
      className="tree-preview absolute inset-0 flex items-center justify-center overflow-hidden transition-opacity data-stale:opacity-40"
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

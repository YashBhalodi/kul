import { matchesKeyboardEvent } from "@tanstack/react-hotkeys"
import { useSelector } from "@tanstack/react-store"
import { CircleAlertIcon, UserRoundPlusIcon } from "lucide-react"
import { useEffect, useLayoutEffect, useRef } from "react"
import type { KeyboardEvent, PointerEvent, ReactNode } from "react"

import { applyWheel, gridFor, panBy, zoomAround } from "@/canvas/camera"
import type { Camera, Point } from "@/canvas/camera"
import { reapply } from "@/canvas/selection"
import { Button } from "@/components/ui/button"
import { CanvasMessage } from "../canvas-surface"
import {
  canvasSizeStore,
  currentLayout,
  showProblems,
} from "../editor/editor-layout"
import { freeCanvasInsets } from "../editor/window-geometry"
import { countProblems } from "../editor/problems"
import { analysisStore } from "../engine-status"
import { sessionStore } from "../project-session"
import { appHotkeys } from "../shortcuts"
import type { AppHotkeyId } from "../shortcuts"
import {
  chooseEntity,
  clearCanvasSelection,
  connectLinking,
  jumpToOwnCard,
} from "./canvas-linking"
import {
  cameraStore,
  connectCanvas,
  drawnTreeStore,
  fitTree,
  marksStore,
  panStep,
  placeCamera,
  setCamera,
  zoomStep,
} from "./canvas-store"
import { addFirstPerson } from "./first-person"
import { decorateTree, paintMarks, targetOf, treeContents } from "./tree-dom"

/** How far a press must move before it pans rather than clicks, in px. */
const DRAG_THRESHOLD = 4

/** The canvas's own keys, from the app's shortcut table. */
const CANVAS_KEYS: ReadonlyArray<readonly [AppHotkeyId, () => void]> = [
  ["canvas.fit", fitTree],
  ["canvas.zoomOut", () => zoomStep(-1, lastPointer)],
  ["canvas.panLeft", () => panStep(-1, 0)],
  ["canvas.panRight", () => panStep(1, 0)],
  ["canvas.panUp", () => panStep(0, -1)],
  ["canvas.panDown", () => panStep(0, 1)],
  ["canvas.clearSelection", clearCanvasSelection],
]

/** Where the pointer last was over the canvas, for zooming with keys. */
let lastPointer: Point | undefined

/**
 * The family tree on the canvas: the engine's latest drawing, under a
 * camera you move by dragging, scrolling, pinching and the keyboard. It
 * keeps the last good tree (dimmed) while the project has errors.
 */
export function TreeCanvas() {
  const viewport = useRef<HTMLDivElement>(null)
  const layer = useRef<HTMLDivElement>(null)
  const host = useRef<HTMLDivElement>(null)
  const drag = useRef<{
    pointer: number
    start: Point
    camera: Camera
    moved: boolean
    target: Element | null
  }>(undefined)

  const svg = useSelector(analysisStore, (state) => state.lastSvg)
  const errors = useSelector(analysisStore, (state) =>
    state.report && state.report.svg === undefined
      ? countProblems(state.report.diagnostics).errors
      : 0
  )
  const clean = useSelector(
    analysisStore,
    (state) => state.report?.svg !== undefined
  )
  const owner = useSelector(sessionStore, (state) => state.access === "owner")
  const drawsSomeone = svg?.includes('class="kul-card"') ?? false

  useEffect(() => {
    connectCanvas()
    connectLinking()
  }, [])

  // The camera moves the tree and the dot grid under it.
  useLayoutEffect(() => {
    const apply = (camera: Camera) => {
      if (layer.current) {
        layer.current.style.transform = `translate(${camera.x}px, ${camera.y}px) scale(${camera.zoom})`
      }
      if (viewport.current) {
        const grid = gridFor(camera)
        viewport.current.style.backgroundSize = `${grid.spacing}px ${grid.spacing}px`
        viewport.current.style.backgroundPosition = `${grid.x}px ${grid.y}px`
      }
    }
    apply(cameraStore.state)
    const subscription = cameraStore.subscribe(apply)
    return () => subscription.unsubscribe()
  }, [])

  // The canvas's size, for fitting and revealing.
  useLayoutEffect(() => {
    const element = viewport.current
    if (!element) return
    const measure = () =>
      canvasSizeStore.setState((size) =>
        size.width === element.clientWidth &&
        size.height === element.clientHeight
          ? size
          : { width: element.clientWidth, height: element.clientHeight }
      )
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  // Swap in each new drawing, then mark it and place the camera.
  useLayoutEffect(() => {
    const element = host.current
    if (!element) return
    // Each swap is measured (the latency spec reads it); only the latest
    // measure is kept.
    performance.clearMeasures("kul:canvas-swap")
    performance.mark("kul:canvas-swap-start")
    element.innerHTML = svg ?? ""
    const drawn = element.querySelector("svg") ?? undefined
    if (drawn) {
      decorateTree(drawn)
      marksStore.setState((marks) => reapply(marks, treeContents(drawn)))
      paintMarks(drawn, marksStore.state)
    }
    drawnTreeStore.setState(() => (drawsSomeone ? drawn : undefined))
    performance.measure("kul:canvas-swap", "kul:canvas-swap-start")
    performance.clearMarks("kul:canvas-swap-start")
    if (drawn) placeCamera(drawn, drawsSomeone)
  }, [svg, drawsSomeone])

  // Repaint the marks as they change.
  useEffect(() => {
    const subscription = marksStore.subscribe((marks) => {
      const drawn = drawnTreeStore.state
      if (drawn) paintMarks(drawn, marks)
    })
    return () => subscription.unsubscribe()
  }, [])

  // Scrolling and pinching; the listener is not passive, so the page
  // itself never zooms.
  useEffect(() => {
    const element = viewport.current
    if (!element) return
    const onWheel = (event: WheelEvent) => {
      event.preventDefault()
      const at = pointIn(element, event)
      setCamera(
        applyWheel(cameraStore.state, event, at, {
          width: element.clientWidth,
          height: element.clientHeight,
        })
      )
    }
    // Safari reports a trackpad pinch as gesture events with a scale.
    let gestureStart: { camera: Camera; at: Point } | undefined
    const onGestureStart = (event: Event) => {
      event.preventDefault()
      gestureStart = {
        camera: cameraStore.state,
        at: pointIn(element, event as unknown as MouseEvent),
      }
    }
    const onGestureChange = (event: Event) => {
      event.preventDefault()
      if (!gestureStart) return
      const { camera, at } = gestureStart
      const scale = (event as unknown as { scale: number }).scale
      setCamera(zoomAround(camera, camera.zoom * scale, at))
    }
    element.addEventListener("wheel", onWheel, { passive: false })
    element.addEventListener("gesturestart", onGestureStart)
    element.addEventListener("gesturechange", onGestureChange)
    return () => {
      element.removeEventListener("wheel", onWheel)
      element.removeEventListener("gesturestart", onGestureStart)
      element.removeEventListener("gesturechange", onGestureChange)
    }
  }, [])

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 && event.button !== 1) return
    event.currentTarget.setPointerCapture(event.pointerId)
    drag.current = {
      pointer: event.pointerId,
      start: { x: event.clientX, y: event.clientY },
      camera: cameraStore.state,
      moved: false,
      target: event.target as Element,
    }
  }

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    lastPointer = pointIn(event.currentTarget, event)
    const current = drag.current
    if (current?.pointer !== event.pointerId) return
    const dx = event.clientX - current.start.x
    const dy = event.clientY - current.start.y
    if (!current.moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) return
    current.moved = true
    setCamera(panBy(current.camera, dx, dy))
  }

  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    const current = drag.current
    if (current?.pointer !== event.pointerId) return
    drag.current = undefined
    if (current.moved || event.button !== 0) return
    activate(targetOf(current.target))
  }

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const target = event.target as Element
    if (
      target.classList.contains("kul-ghost-badge") &&
      (event.key === "Enter" || event.key === " ")
    ) {
      event.preventDefault()
      activate(targetOf(target))
      return
    }
    // "+" is Shift+= on most layouts; "=" alone zooms in too.
    if (
      matchesKeyboardEvent(
        event.nativeEvent,
        appHotkeys["canvas.zoomIn"].defaultHotkey
      ) ||
      event.key === "="
    ) {
      event.preventDefault()
      zoomStep(1, lastPointer)
      return
    }
    for (const [id, run] of CANVAS_KEYS) {
      if (
        matchesKeyboardEvent(event.nativeEvent, appHotkeys[id].defaultHotkey)
      ) {
        event.preventDefault()
        run()
        return
      }
    }
  }

  const empty = !drawsSomeone
  return (
    <>
      <div
        ref={viewport}
        role="region"
        aria-label="Family tree"
        tabIndex={0}
        data-testid="tree-viewport"
        className="canvas-grid absolute inset-0 cursor-grab touch-none overflow-hidden outline-none select-none active:cursor-grabbing"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={() => (drag.current = undefined)}
        onPointerLeave={() => (lastPointer = undefined)}
        onKeyDown={onKeyDown}
      >
        <div
          ref={layer}
          data-testid="tree-layer"
          className="absolute top-0 left-0 origin-top-left"
        >
          <div
            ref={host}
            data-testid="tree"
            data-stale={(errors > 0 && !empty) || undefined}
            className="kul-tree transition-opacity data-stale:opacity-40"
          />
        </div>
      </div>
      {empty && (errors > 0 || clean) && (
        <BesideEditor>
          {errors > 0 ? (
            <CanvasMessage
              testId="tree-errors"
              icon={<CircleAlertIcon className="size-7 text-destructive" />}
              title="The tree can’t be drawn yet"
              actions={
                <Button variant="outline" onClick={showProblems}>
                  Show problems
                </Button>
              }
            >
              {errors === 1
                ? "Fix the error in your files and your family appears here."
                : `Fix the ${errors} errors in your files and your family appears here.`}
            </CanvasMessage>
          ) : (
            <CanvasMessage
              testId="tree-empty"
              icon={<UserRoundPlusIcon className="size-7 text-primary" />}
              title="Your family tree starts here"
              actions={
                <Button disabled={!owner} onClick={() => void addFirstPerson()}>
                  Add your first person
                </Button>
              }
            >
              Describe your family in the editor, one person at a time, and the
              tree draws itself as you type.
            </CanvasMessage>
          )}
        </BesideEditor>
      )}
    </>
  )
}

/** Canvas content placed in the room beside the editor window. */
function BesideEditor({ children }: { children: ReactNode }) {
  const editorWindow = useSelector(
    sessionStore,
    (state) => state.ui.editorWindow
  )
  const canvas = useSelector(canvasSizeStore)
  const insets = freeCanvasInsets(currentLayout(editorWindow, canvas), canvas)
  return (
    <div
      style={{ left: insets.left, right: insets.right }}
      className="pointer-events-none absolute inset-y-0 z-[1] *:pointer-events-auto"
    >
      {children}
    </div>
  )
}

/** A pointer event's position in px from `element`'s top-left corner. */
function pointIn(
  element: Element,
  event: { clientX: number; clientY: number }
): Point {
  const box = element.getBoundingClientRect()
  return { x: event.clientX - box.left, y: event.clientY - box.top }
}

/** Act on a click (or Enter) on the tree. */
function activate(target: ReturnType<typeof targetOf>) {
  switch (target.kind) {
    case "badge":
      jumpToOwnCard(target.personId)
      return
    case "entity":
      chooseEntity(target.entity)
      return
    case "background":
      clearCanvasSelection()
      return
  }
}

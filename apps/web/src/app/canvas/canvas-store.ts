// The canvas's state: the camera, the marks (the selection and the link),
// the legend's toggle, the selected entity's details, and the tree on
// screen. The camera is saved with the project; the rest is this page's.

import { Debouncer } from "@tanstack/pacer"
import { createStore } from "@tanstack/react-store"

import {
  MIN_ZOOM,
  PAN_STEP,
  ZOOM_STEP,
  between,
  fitCamera,
  panBy,
  sameCamera,
  zoomAround,
  zoomBy,
} from "@/canvas/camera"
import type { Box, Camera, Margins, Point, Size } from "@/canvas/camera"
import type { DetailsModel } from "@/canvas/details"
import { centreInView, revealCamera, visibleRegion } from "@/canvas/reveal"
import { NO_MARKS } from "@/canvas/selection"
import type { CanvasMarks, EntityRef } from "@/canvas/selection"
import type { EditorWindowLayout } from "@/store/editor-window"
import { canvasSizeStore, currentLayout } from "../editor/editor-layout"
import { windowRect } from "../editor/window-geometry"
import { sessionStore, updateUi } from "../project-session"
import type { SessionState } from "../project-session"
import { entityBox, treeSize } from "./tree-dom"

const HOME: Camera = { x: 0, y: 0, zoom: 1 }

export const cameraStore = createStore<Camera>(HOME)

export const marksStore = createStore<CanvasMarks>(NO_MARKS)

export const legendStore = createStore<{ open: boolean }>({ open: false })

export function toggleLegend(): void {
  legendStore.setState((state) => ({ open: !state.open }))
}

/** The details panel's content: the model of the entity it last described. */
export const detailsStore = createStore<DetailsModel | undefined>(undefined)

/** The tree on screen, once drawn. */
export const drawnTreeStore = createStore<SVGSVGElement | undefined>(undefined)

// ---- Where the chrome sits ------------------------------------------------

/** The details panel's width, fixed so the tree's room doesn't jump about. */
export const DETAILS_WIDTH = 360
/** The room kept between the canvas's corner controls and its edges. */
const CHROME_INSET = 16

/**
 * The details panel's box on the canvas: along the right edge, or beside
 * the editor window when that is docked there.
 */
export function detailsPanelBox(canvas: Size, layout: EditorWindowLayout): Box {
  const docked =
    layout.dock === "right" && !layout.minimized
      ? windowRect(layout, canvas).width
      : 0
  return {
    x: canvas.width - docked - DETAILS_WIDTH,
    y: 0,
    width: DETAILS_WIDTH,
    height: canvas.height,
  }
}

/** How far the corner controls keep from the canvas's right edge. */
export function controlsRightInset(
  canvas: Size,
  layout: EditorWindowLayout,
  panelOpen: boolean
): number {
  if (panelOpen) {
    return canvas.width - detailsPanelBox(canvas, layout).x + CHROME_INSET
  }
  if (layout.dock === "right" && !layout.minimized) {
    return windowRect(layout, canvas).width + CHROME_INSET
  }
  return CHROME_INSET
}

/** What floats over the canvas and hides the tree behind it. */
export function occluders(): Array<Box> {
  const canvas = canvasSizeStore.state
  const layout = currentLayout()
  const boxes: Array<Box> = []
  if (!layout.minimized) boxes.push(windowRect(layout, canvas))
  if (marksStore.state.selected) boxes.push(detailsPanelBox(canvas, layout))
  return boxes
}

/** Room kept around a fitted tree, clear of the banners and controls. */
const FIT_MARGINS: Margins = { top: 64, right: 48, bottom: 80, left: 48 }

// ---- Moving the camera ----------------------------------------------------

const ANIMATION_MS = 260
let animation: number | undefined

function stopAnimation() {
  if (animation !== undefined) cancelAnimationFrame(animation)
  animation = undefined
}

function prefersReducedMotion(): boolean {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches
}

/** Show the tree through `camera` at once (a drag, a scroll, a key). */
export function setCamera(camera: Camera): void {
  stopAnimation()
  if (!sameCamera(camera, cameraStore.state)) {
    cameraStore.setState(() => camera)
  }
}

/** Glide to `camera` (a reveal or a jump), or go at once if motion is reduced. */
export function glideTo(camera: Camera): void {
  stopAnimation()
  const from = cameraStore.state
  if (prefersReducedMotion() || sameCamera(from, camera)) {
    setCamera(camera)
    return
  }
  const start = performance.now()
  const frame = (now: number) => {
    const t = Math.min(1, (now - start) / ANIMATION_MS)
    const eased = 1 - Math.pow(1 - t, 3)
    cameraStore.setState(() => between(from, camera, eased))
    animation = t < 1 ? requestAnimationFrame(frame) : undefined
  }
  animation = requestAnimationFrame(frame)
}

/** The middle of the part of the canvas the tree can be seen in. */
function visibleCentre(): Point {
  const region = visibleRegion(canvasSizeStore.state, occluders())
  return { x: region.x + region.width / 2, y: region.y + region.height / 2 }
}

/** The camera that shows the whole tree beside the canvas's chrome. */
function fittedCamera(svg: SVGSVGElement): Camera {
  const canvas = canvasSizeStore.state
  return fitCamera(
    treeSize(svg),
    visibleRegion(canvas, occluders()),
    FIT_MARGINS
  )
}

/** Fit the whole tree into view (Fit, or `0`). */
export function fitTree(): void {
  const svg = drawnTreeStore.state
  if (svg) glideTo(fittedCamera(svg))
}

/** Show the tree at its own size, around the middle of the view (100%). */
export function zoomToActualSize(): void {
  glideTo(zoomAround(cameraStore.state, 1, visibleCentre()))
}

/** Zoom one step in (1) or out (-1), around `at` or the middle of the view. */
export function zoomStep(direction: 1 | -1, at?: Point): void {
  const factor = direction === 1 ? ZOOM_STEP : 1 / ZOOM_STEP
  setCamera(zoomBy(cameraStore.state, factor, at ?? visibleCentre()))
}

/** Pan one step; `dx, dy` are -1, 0 or 1 in the direction to look. */
export function panStep(dx: number, dy: number): void {
  setCamera(panBy(cameraStore.state, -dx * PAN_STEP, -dy * PAN_STEP))
}

/** Bring `entity` into view if it is off screen or behind chrome. */
export function revealEntity(entity: EntityRef): void {
  const svg = drawnTreeStore.state
  const box = svg && entityBox(svg, entity)
  if (!box) return
  const camera = revealCamera(
    cameraStore.state,
    box,
    canvasSizeStore.state,
    occluders()
  )
  if (camera) glideTo(camera)
}

/** Centre `entity` across the view, wherever it is. */
export function centreEntity(entity: EntityRef): void {
  const svg = drawnTreeStore.state
  const box = svg && entityBox(svg, entity)
  if (!box) return
  glideTo(
    centreInView(cameraStore.state, box, canvasSizeStore.state, occluders())
  )
}

// ---- One project at a time ------------------------------------------------

/** Whether the open project's camera has been placed (restored or fitted). */
let placed = false
let openProjectId: string | undefined

/**
 * Place the camera on a newly drawn tree: the first tree a project draws
 * (with someone in it) is fitted, and after that the camera stays put.
 */
export function placeCamera(svg: SVGSVGElement, drawsSomeone: boolean): void {
  if (placed || !drawsSomeone) return
  placed = true
  setCamera(fittedCamera(svg))
}

/** Writes the camera into the project once it stops moving. */
const cameraWriter = new Debouncer(
  (projectId: string, camera: Camera) => {
    if (sessionStore.state.projectId !== projectId) return
    updateUi((ui) => ({ ...ui, camera }))
  },
  { wait: 400 }
)

function followProject(state: SessionState) {
  if (state.status !== "open") {
    if (state.projectId !== openProjectId) reset(undefined)
    return
  }
  if (state.projectId === openProjectId) return
  reset(state.projectId)
  const stored = state.ui.camera
  if (stored && stored.zoom >= MIN_ZOOM) {
    placed = true
    setCamera(stored)
  }
}

function reset(projectId: string | undefined) {
  cameraWriter.cancel()
  openProjectId = projectId
  placed = false
  setCamera(HOME)
  marksStore.setState(() => NO_MARKS)
  detailsStore.setState(() => undefined)
}

let connected = false

/** Keep the canvas in step with the open project, once per page. */
export function connectCanvas(): void {
  if (connected) return
  connected = true
  followProject(sessionStore.state)
  sessionStore.subscribe(followProject)
  cameraStore.subscribe((camera) => {
    if (placed && openProjectId) {
      cameraWriter.maybeExecute(openProjectId, camera)
    }
  })
}

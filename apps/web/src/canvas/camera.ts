// The canvas camera: where the family tree sits on screen, and how large.
// A point (x, y) of the tree, in the SVG's own units, is drawn at
// (camera.x + x * zoom, camera.y + y * zoom) in px from the canvas's
// top-left corner. Every function here is pure; the canvas applies the
// result as a CSS transform.

export interface Point {
  x: number
  y: number
}

export interface Size {
  width: number
  height: number
}

/** A rectangle: its top-left corner and its size. */
export interface Box extends Point, Size {}

export interface Camera {
  /** Where the tree's origin is drawn, in px from the canvas's corner. */
  x: number
  y: number
  /** Screen px per tree unit: 1 is 100%. */
  zoom: number
}

export const MIN_ZOOM = 0.1
export const MAX_ZOOM = 8
/** How much one press of zoom in or zoom out changes the zoom. */
export const ZOOM_STEP = 1.25
/** How far one press of an arrow key pans, in px. */
export const PAN_STEP = 64

export function clampZoom(zoom: number): number {
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom))
}

/** The camera moved by `dx, dy` screen px. */
export function panBy(camera: Camera, dx: number, dy: number): Camera {
  return { ...camera, x: camera.x + dx, y: camera.y + dy }
}

/**
 * The camera at `zoom` (clamped to the range), keeping the part of the
 * tree under the screen point `at` where it is.
 */
export function zoomAround(camera: Camera, zoom: number, at: Point): Camera {
  const next = clampZoom(zoom)
  const scale = next / camera.zoom
  return {
    x: at.x - (at.x - camera.x) * scale,
    y: at.y - (at.y - camera.y) * scale,
    zoom: next,
  }
}

/** The camera zoomed by `factor` around the screen point `at`. */
export function zoomBy(camera: Camera, factor: number, at: Point): Camera {
  return zoomAround(camera, camera.zoom * factor, at)
}

/** Where a box of the tree is drawn on screen. */
export function toScreen(camera: Camera, box: Box): Box {
  return {
    x: camera.x + box.x * camera.zoom,
    y: camera.y + box.y * camera.zoom,
    width: box.width * camera.zoom,
    height: box.height * camera.zoom,
  }
}

/** The camera that draws the centre of `box` (tree units) at screen point `at`. */
export function centreOn(camera: Camera, box: Box, at: Point): Camera {
  return {
    ...camera,
    x: at.x - (box.x + box.width / 2) * camera.zoom,
    y: at.y - (box.y + box.height / 2) * camera.zoom,
  }
}

/** Room kept around the tree when it is fitted, in px. */
export interface Margins {
  top: number
  right: number
  bottom: number
  left: number
}

/**
 * The camera that shows the whole tree (`content`, in tree units) centred
 * in `region` (screen px) inside `margins`. It never enlarges past 100%:
 * a small tree is shown at its own size rather than blown up.
 */
export function fitCamera(
  content: Size,
  region: Box,
  margins: Margins
): Camera {
  const width = Math.max(1, region.width - margins.left - margins.right)
  const height = Math.max(1, region.height - margins.top - margins.bottom)
  const zoom = clampZoom(
    Math.min(
      1,
      width / Math.max(1, content.width),
      height / Math.max(1, content.height)
    )
  )
  return centreOn(
    { x: 0, y: 0, zoom },
    { x: 0, y: 0, width: content.width, height: content.height },
    {
      x: region.x + margins.left + width / 2,
      y: region.y + margins.top + height / 2,
    }
  )
}

/** A wheel event's movement, as the canvas reads it. */
export interface WheelInput {
  deltaX: number
  deltaY: number
  /** 0 = px, 1 = lines, 2 = pages (WheelEvent.deltaMode). */
  deltaMode: number
  /** A pinch on a trackpad arrives as a wheel event with Ctrl held. */
  ctrlKey: boolean
  metaKey: boolean
}

/** Px per line, for wheels that count lines (Firefox with a mouse). */
const LINE_HEIGHT = 16
/** How strongly one px of wheel movement zooms. */
const WHEEL_ZOOM_RATE = 0.01
/** The most one wheel event may move, in px, so a notched wheel zooms in steps. */
const WHEEL_ZOOM_LIMIT = 40

/**
 * The camera after a wheel event at the screen point `at`: scrolling pans;
 * pinching, or scrolling with Ctrl or ⌘ held, zooms around the pointer.
 */
export function applyWheel(
  camera: Camera,
  input: WheelInput,
  at: Point,
  page: Size
): Camera {
  const unit =
    input.deltaMode === 1
      ? LINE_HEIGHT
      : input.deltaMode === 2
        ? page.height
        : 1
  const dx = input.deltaX * unit
  const dy = input.deltaY * unit
  if (input.ctrlKey || input.metaKey) {
    const step = Math.max(-WHEEL_ZOOM_LIMIT, Math.min(WHEEL_ZOOM_LIMIT, dy))
    return zoomBy(camera, Math.exp(-step * WHEEL_ZOOM_RATE), at)
  }
  return panBy(camera, -dx, -dy)
}

/** The dot grid's spacing at 100%, in px. */
export const GRID_SPACING = 20

/**
 * The dot grid under the tree: it moves and scales with the camera, and
 * its spacing doubles or halves to stay between 14 and 28 px whatever the
 * zoom, so the ground never turns into a blur or a desert.
 */
export function gridFor(camera: Camera): { spacing: number } & Point {
  let spacing = GRID_SPACING * camera.zoom
  while (spacing < 14) spacing *= 2
  while (spacing >= 28) spacing /= 2
  const offset = (value: number) => ((value % spacing) + spacing) % spacing
  return { spacing, x: offset(camera.x), y: offset(camera.y) }
}

/** The camera `t` of the way (0–1) from `from` to `to`, for animated pans. */
export function between(from: Camera, to: Camera, t: number): Camera {
  // Zoom moves geometrically, so a zoom from 10% to 800% feels even.
  const zoom = from.zoom * Math.pow(to.zoom / from.zoom, t)
  return {
    x: from.x + (to.x - from.x) * t,
    y: from.y + (to.y - from.y) * t,
    zoom,
  }
}

/** Whether two cameras show the same view. */
export function sameCamera(a: Camera, b: Camera): boolean {
  return a.x === b.x && a.y === b.y && a.zoom === b.zoom
}

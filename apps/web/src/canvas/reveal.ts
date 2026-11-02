// Bringing part of the tree into view without parking it behind, or hard
// against, the chrome that floats over the canvas: the editor window and
// the details panel. Each occluder's box is given (measured from the
// layout that places it), never assumed to be on one particular side.

import { centreOn, toScreen } from "./camera"
import type { Box, Camera, Size } from "./camera"

/** How far inside the canvas's edges a box must sit to count as in view. */
export const VIEW_MARGIN = 16

/** The room the tree keeps from the chrome floating over it, in px. */
export const CHROME_GAP = 24

/** The room a revealed box keeps from the canvas's own edges, in px. */
export const REVEAL_MARGIN = 48

/** The part of `box` that lies on the canvas, or undefined if none does. */
function onCanvas(box: Box, canvas: Size): Box | undefined {
  const left = Math.max(0, box.x)
  const top = Math.max(0, box.y)
  const right = Math.min(canvas.width, box.x + box.width)
  const bottom = Math.min(canvas.height, box.y + box.height)
  if (right <= left || bottom <= top) return undefined
  return { x: left, y: top, width: right - left, height: bottom - top }
}

function overlaps(a: Box, b: Box): boolean {
  return (
    a.x < b.x + b.width &&
    b.x < a.x + a.width &&
    a.y < b.y + b.height &&
    b.y < a.y + a.height
  )
}

/** `box` grown by `by` on every side. */
function grow(box: Box, by: number): Box {
  return {
    x: box.x - by,
    y: box.y - by,
    width: box.width + 2 * by,
    height: box.height + 2 * by,
  }
}

/**
 * The widest full-height strip that no occluder (nor the `CHROME_GAP` beside
 * it) covers, kept `edge` px inside the canvas's own edges. Occluders block
 * the whole of their width, top to bottom (the editor and the panel are
 * tall). When they leave no strip at all, the whole canvas: centring behind
 * chrome beats not moving.
 */
function freeStrip(
  canvas: Size,
  occluders: ReadonlyArray<Box>,
  edge: number
): Box {
  const spans = occluders
    .map((box) => onCanvas(box, canvas))
    .filter((box): box is Box => box !== undefined)
    .map((box) => [box.x - CHROME_GAP, box.x + box.width + CHROME_GAP] as const)
    .sort((a, b) => a[0] - b[0])
  let best: readonly [number, number] | undefined
  let from = edge
  const consider = (end: number) => {
    const to = Math.min(end, canvas.width - edge)
    if (to > from && (!best || to - from > best[1] - best[0])) {
      best = [from, to]
    }
  }
  for (const [start, end] of spans) {
    consider(start)
    from = Math.max(from, end)
  }
  consider(canvas.width)
  const [left, right] = best ?? [edge, canvas.width - edge]
  return {
    x: left,
    y: edge,
    width: right - left,
    height: canvas.height - 2 * edge,
  }
}

/**
 * The part of the canvas the tree is seen in: the widest full-height strip
 * between the occluders and the canvas's sides, `CHROME_GAP` clear of each
 * occluder.
 */
export function visibleRegion(
  canvas: Size,
  occluders: ReadonlyArray<Box>
): Box {
  return freeStrip(canvas, occluders, 0)
}

/**
 * Whether a box on screen can be seen: on the canvas, clear of its edges,
 * and `CHROME_GAP` clear of every occluder.
 */
export function inView(
  screen: Box,
  canvas: Size,
  occluders: ReadonlyArray<Box>
): boolean {
  const inside =
    screen.x >= VIEW_MARGIN &&
    screen.y >= VIEW_MARGIN &&
    screen.x + screen.width <= canvas.width - VIEW_MARGIN &&
    screen.y + screen.height <= canvas.height - VIEW_MARGIN
  return (
    inside &&
    !occluders.some((occluder) => overlaps(screen, grow(occluder, CHROME_GAP)))
  )
}

/**
 * The camera that centres `box` (tree units) across the visible region.
 * Only the horizontal axis has to move: the box keeps its height on screen
 * unless it is off the top or bottom, when it is centred vertically too.
 */
export function centreInView(
  camera: Camera,
  box: Box,
  canvas: Size,
  occluders: ReadonlyArray<Box>
): Camera {
  const region = visibleRegion(canvas, occluders)
  const screen = toScreen(camera, box)
  const fitsVertically =
    screen.y >= VIEW_MARGIN &&
    screen.y + screen.height <= canvas.height - VIEW_MARGIN
  return centreOn(camera, box, {
    x: region.x + region.width / 2,
    y: fitsVertically
      ? screen.y + screen.height / 2
      : region.y + region.height / 2,
  })
}

/**
 * How far to move a span [start, end) on one axis so it lies inside
 * [low, high): the least move that does it, or, when the span is too long
 * to fit, the move that centres it.
 */
function shiftInto(
  start: number,
  end: number,
  low: number,
  high: number
): number {
  if (end - start > high - low) return (low + high) / 2 - (start + end) / 2
  if (start < low) return low - start
  if (end > high) return high - end
  return 0
}

/**
 * The camera that brings `box` (tree units) into view, or undefined when
 * it already is. A box off screen, behind an occluder or hard against one
 * moves the least distance that puts it in the visible region,
 * `REVEAL_MARGIN` inside the canvas's edges, so the tree around it stays
 * where the reader left it.
 */
export function revealCamera(
  camera: Camera,
  box: Box,
  canvas: Size,
  occluders: ReadonlyArray<Box>
): Camera | undefined {
  const screen = toScreen(camera, box)
  if (inView(screen, canvas, occluders)) return undefined
  const room = freeStrip(canvas, occluders, REVEAL_MARGIN)
  return {
    ...camera,
    x:
      camera.x +
      shiftInto(screen.x, screen.x + screen.width, room.x, room.x + room.width),
    y:
      camera.y +
      shiftInto(
        screen.y,
        screen.y + screen.height,
        room.y,
        room.y + room.height
      ),
  }
}

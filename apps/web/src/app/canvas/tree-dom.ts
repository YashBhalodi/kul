// The drawn tree in the page: the engine's SVG, read through the kul-svg
// contract (the `kul-*` classes and their data-* attributes), plus the
// canvas's own additions to it: the ghost cards' return badges and wider
// hit areas for marriage lines.

import type { Box, Size } from "@/canvas/camera"
import type { CanvasMarks, EntityRef, TreeContents } from "@/canvas/selection"

const SVG_NS = "http://www.w3.org/2000/svg"

const PERSON_CARDS = ".kul-card[data-person-id]"
const MARRIAGE_EDGES = '.kul-edge[data-link-kind="marriage"][data-marriage-id]'

/** The people and marriages a drawn tree shows. */
export function treeContents(svg: SVGSVGElement): TreeContents {
  const persons = new Set<string>()
  for (const card of svg.querySelectorAll(PERSON_CARDS)) {
    persons.add(card.getAttribute("data-person-id") ?? "")
  }
  const marriages = new Set<string>()
  for (const edge of svg.querySelectorAll(MARRIAGE_EDGES)) {
    marriages.add(edge.getAttribute("data-marriage-id") ?? "")
  }
  return { persons, marriages }
}

/** The tree's own size, in its units. */
export function treeSize(svg: SVGSVGElement): Size {
  return {
    width: Number(svg.getAttribute("width")) || 0,
    height: Number(svg.getAttribute("height")) || 0,
  }
}

/** Every element that draws `entity`: a person's cards, a marriage's lines. */
export function entityElements(
  svg: SVGSVGElement,
  entity: EntityRef
): Array<Element> {
  const id = CSS.escape(entity.id)
  return [
    ...svg.querySelectorAll(
      entity.kind === "person"
        ? `.kul-card[data-person-id="${id}"]`
        : `.kul-edge[data-link-kind="marriage"][data-marriage-id="${id}"]`
    ),
  ]
}

/** A card's box, from its rectangle. */
function cardBox(card: Element): Box | undefined {
  const rect = card.querySelector("rect")
  if (!rect) return undefined
  const read = (name: string) => Number(rect.getAttribute(name))
  return {
    x: read("x"),
    y: read("y"),
    width: read("width"),
    height: read("height"),
  }
}

function union(boxes: ReadonlyArray<Box>): Box | undefined {
  if (boxes.length === 0) return undefined
  const left = Math.min(...boxes.map((box) => box.x))
  const top = Math.min(...boxes.map((box) => box.y))
  const right = Math.max(...boxes.map((box) => box.x + box.width))
  const bottom = Math.max(...boxes.map((box) => box.y + box.height))
  return { x: left, y: top, width: right - left, height: bottom - top }
}

function centreDistance(a: Box, b: Box): number {
  return Math.hypot(
    a.x + a.width / 2 - (b.x + b.width / 2),
    a.y + a.height / 2 - (b.y + b.height / 2)
  )
}

/**
 * Where `entity` is drawn, in the tree's units: a person's own (canonical)
 * card; a marriage's lines together with the spouses' cards beside them.
 */
export function entityBox(
  svg: SVGSVGElement,
  entity: EntityRef
): Box | undefined {
  const elements = entityElements(svg, entity)
  if (entity.kind === "person") {
    const canonical =
      elements.find((card) => card.getAttribute("data-kind") === "canonical") ??
      elements[0]
    return canonical && cardBox(canonical)
  }
  const edges = elements.filter(
    (edge): edge is SVGGraphicsElement => "getBBox" in edge
  )
  const lines = union(edges.map((edge) => edge.getBBox()))
  if (!lines) return undefined
  // A spouse can be drawn more than once (a past record); take the card
  // nearest the marriage's lines.
  const spouses = edges.flatMap((edge) =>
    ["data-host-id", "data-joining-id"].map((name) => edge.getAttribute(name))
  )
  const cards = [...new Set(spouses)].flatMap((id) => {
    if (!id) return []
    const boxes = entityElements(svg, { kind: "person", id })
      .map(cardBox)
      .filter((box): box is Box => box !== undefined)
      .sort((a, b) => centreDistance(a, lines) - centreDistance(b, lines))
    return boxes.slice(0, 1)
  })
  return union([lines, ...cards])
}

/** A person's own card (not a past record of them). */
export function canonicalCard(
  svg: SVGSVGElement,
  personId: string
): Element | undefined {
  return entityElements(svg, { kind: "person", id: personId }).find(
    (card) => card.getAttribute("data-kind") === "canonical"
  )
}

/** Mark the selected and the linked entity's elements for the tree's CSS. */
export function paintMarks(svg: SVGSVGElement, marks: CanvasMarks): void {
  for (const marked of svg.querySelectorAll("[data-selected], [data-linked]")) {
    marked.removeAttribute("data-selected")
    marked.removeAttribute("data-linked")
  }
  if (marks.linked) {
    for (const element of entityElements(svg, marks.linked)) {
      element.setAttribute("data-linked", "")
    }
  }
  if (marks.selected) {
    for (const element of entityElements(svg, marks.selected)) {
      element.setAttribute("data-selected", "")
    }
  }
}

/** Why a past record is drawn, as its badge's tooltip says. */
const GHOST_REASONS: Readonly<Record<string, string>> = {
  "past-marriage": "a marriage that ended",
  "past-adoption": "an adoption that ended",
  "past-birth": "their birth family",
}

/**
 * Add the canvas's own elements to a freshly drawn tree: on each ghost
 * card (a past record of a person) a return badge that leads to the
 * person's own card, and along each marriage line a wider, invisible line
 * that is easier to click. Each person's own card and each marriage's
 * line become buttons in the tab order, so the keyboard selects what a
 * click does. A ghost card does not: its badge is its button, and the
 * person's own card is reachable.
 */
export function decorateTree(svg: SVGSVGElement): void {
  for (const card of svg.querySelectorAll(
    '.kul-card[data-kind="canonical"][data-person-id]'
  )) {
    card.setAttribute("role", "button")
    card.setAttribute("tabindex", "0")
    card.setAttribute("aria-label", cardLabel(card))
  }
  for (const ghost of svg.querySelectorAll(
    '.kul-card[data-kind="ghost"][data-person-id]'
  )) {
    const box = cardBox(ghost)
    if (!box) continue
    const name = ghost.querySelector(".kul-label-name")?.textContent ?? ""
    const reason =
      GHOST_REASONS[ghost.getAttribute("data-ghost-reason") ?? ""] ??
      "a past record"
    const badge = document.createElementNS(SVG_NS, "g")
    badge.setAttribute("class", "kul-ghost-badge")
    badge.setAttribute("role", "button")
    badge.setAttribute("tabindex", "0")
    badge.setAttribute("aria-label", `Go to ${name}’s own card`)
    const target = document.createElementNS(SVG_NS, "rect")
    target.setAttribute("class", "kul-ghost-badge-target")
    target.setAttribute("x", String(box.x + box.width - 30))
    target.setAttribute("y", String(box.y + 4))
    target.setAttribute("width", "26")
    target.setAttribute("height", "26")
    target.setAttribute("rx", "6")
    const glyph = document.createElementNS(SVG_NS, "text")
    glyph.setAttribute("x", String(box.x + box.width - 17))
    glyph.setAttribute("y", String(box.y + 17))
    glyph.setAttribute("text-anchor", "middle")
    glyph.setAttribute("dominant-baseline", "central")
    glyph.textContent = "↺"
    const title = document.createElementNS(SVG_NS, "title")
    title.textContent = `A past record of ${name}, from ${reason}. Go to their own card.`
    badge.append(target, glyph, title)
    ghost.append(badge)
  }
  for (const edge of svg.querySelectorAll(MARRIAGE_EDGES)) {
    const hit = document.createElementNS(SVG_NS, "path")
    hit.setAttribute("class", "kul-hit")
    hit.setAttribute(
      "data-marriage-id",
      edge.getAttribute("data-marriage-id") ?? ""
    )
    hit.setAttribute("d", edge.getAttribute("d") ?? "")
    hit.setAttribute("role", "button")
    hit.setAttribute("tabindex", "0")
    hit.setAttribute("aria-label", marriageLabel(svg, edge))
    edge.after(hit)
  }
}

/** A card's accessible name: the text it shows, in order. */
function cardLabel(card: Element): string {
  return [...card.querySelectorAll("text")]
    .map((text) => text.textContent.trim())
    .filter((text) => text !== "")
    .join(" ")
}

/** "Marriage of Chinua Okeke and Adaeze Okeke", from the spouses' cards. */
function marriageLabel(svg: SVGSVGElement, edge: Element): string {
  const name = (id: string | null): string => {
    if (!id) return ""
    const label = svg.querySelector(
      `.kul-card[data-person-id="${CSS.escape(id)}"] .kul-label-name`
    )
    return label?.textContent.trim() || id
  }
  const spouses = [
    name(edge.getAttribute("data-host-id")),
    name(edge.getAttribute("data-joining-id")),
  ].filter((spouse) => spouse !== "")
  return spouses.length > 0
    ? `Marriage of ${spouses.join(" and ")}`
    : "Marriage"
}

/** What a press on the tree landed on. */
export type TreeTarget =
  | { kind: "badge"; personId: string }
  | { kind: "entity"; entity: EntityRef }
  | { kind: "background" }

export function targetOf(element: Element | null): TreeTarget {
  const badge = element?.closest(".kul-ghost-badge")
  const card = element?.closest(PERSON_CARDS)
  const personId = card?.getAttribute("data-person-id")
  if (badge && personId) return { kind: "badge", personId }
  if (personId) {
    return { kind: "entity", entity: { kind: "person", id: personId } }
  }
  const marriage = element
    ?.closest(`.kul-hit, ${MARRIAGE_EDGES}`)
    ?.getAttribute("data-marriage-id")
  if (marriage) {
    return { kind: "entity", entity: { kind: "marriage", id: marriage } }
  }
  return { kind: "background" }
}

/** Briefly pulse an element, to show where a jump landed. */
export function pulse(element: Element): void {
  element.removeAttribute("data-pulse")
  // Reading layout restarts the animation when it is already running.
  void (element as SVGGraphicsElement).getBoundingClientRect()
  element.setAttribute("data-pulse", "")
  element.addEventListener(
    "animationend",
    () => element.removeAttribute("data-pulse"),
    { once: true }
  )
}

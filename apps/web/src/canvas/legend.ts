// The canvas legend: a key to the drawing's visual vocabulary, built from
// the categories the current tree actually draws. Its order and labels are
// the canonical legend table in docs/canonical-ui-pattern.md, which the
// downloaded SVG's baked legend (kul-svg) follows too; a test holds the two
// together. Each swatch is a miniature of the real element, carrying its
// class and data-* attributes, so the tree's own CSS paints it.

/** A swatch: a card outline or an edge, as the tree draws them. */
export type LegendSwatch =
  | {
      element: "card"
      kind: "canonical" | "ghost"
      gender?: "male" | "female" | "other"
    }
  | {
      element: "edge"
      linkKind: "birth" | "adoption" | "marriage"
      ended?: boolean
    }

export interface LegendRow {
  key: string
  label: string
  /** Matches an element of the rendered tree when it draws this category. */
  selector: string
  swatch: LegendSwatch
}

export const LEGEND_ROWS: ReadonlyArray<LegendRow> = [
  {
    key: "gender-male",
    label: "Male",
    selector: '.kul-card[data-gender="male"]',
    swatch: { element: "card", kind: "canonical", gender: "male" },
  },
  {
    key: "gender-female",
    label: "Female",
    selector: '.kul-card[data-gender="female"]',
    swatch: { element: "card", kind: "canonical", gender: "female" },
  },
  {
    key: "gender-other",
    label: "Other",
    selector: '.kul-card[data-gender="other"]',
    swatch: { element: "card", kind: "canonical", gender: "other" },
  },
  {
    key: "past-record",
    label: "Past record",
    selector: '.kul-card[data-kind="ghost"]',
    swatch: { element: "card", kind: "ghost" },
  },
  {
    key: "birth",
    label: "Birth",
    selector: '.kul-edge[data-link-kind="birth"]',
    swatch: { element: "edge", linkKind: "birth" },
  },
  {
    key: "adoption",
    label: "Adoption",
    selector: '.kul-edge[data-link-kind="adoption"]',
    swatch: { element: "edge", linkKind: "adoption" },
  },
  {
    key: "marriage",
    label: "Marriage",
    selector:
      '.kul-edge[data-link-kind="marriage"]:not([data-is-ended="true"])',
    swatch: { element: "edge", linkKind: "marriage" },
  },
  {
    key: "ended-marriage",
    label: "Ended marriage",
    selector: '.kul-edge[data-link-kind="marriage"][data-is-ended="true"]',
    swatch: { element: "edge", linkKind: "marriage", ended: true },
  },
]

/** The rows for the categories a tree draws, in the legend's order. */
export function legendRows(
  draws: (selector: string) => boolean
): ReadonlyArray<LegendRow> {
  return LEGEND_ROWS.filter((row) => draws(row.selector))
}

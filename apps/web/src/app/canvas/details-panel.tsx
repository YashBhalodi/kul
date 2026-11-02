import { useSelector } from "@tanstack/react-store"
import { CodeIcon, XIcon } from "lucide-react"
import type { KeyboardEvent } from "react"

import { sameEntity } from "@/canvas/selection"
import type { DetailsLink, DetailsModel } from "@/canvas/details"
import { Button } from "@/components/ui/button"
import { canvasSizeStore, currentLayout } from "../editor/editor-layout"
import { analysisStore } from "../engine-status"
import { sessionStore } from "../project-session"
import {
  chooseEntity,
  clearCanvasSelection,
  goToSource,
} from "./canvas-linking"
import { detailsPanelBox, detailsStore, marksStore } from "./canvas-store"

/**
 * The selected person or marriage, described: a side panel over the
 * canvas's right edge that leaves the rest of the canvas to use. It never
 * takes the keyboard; every related entity in it selects that entity.
 */
export function DetailsPanel() {
  const selected = useSelector(marksStore, (marks) => marks.selected)
  const model = useSelector(detailsStore)
  const canvas = useSelector(canvasSizeStore)
  const editorWindow = useSelector(
    sessionStore,
    (state) => state.ui.editorWindow
  )
  const hasErrors = useSelector(
    analysisStore,
    (state) => state.report !== undefined && state.report.svg === undefined
  )
  if (!selected) return null
  const box = detailsPanelBox(canvas, currentLayout(editorWindow, canvas))
  const shown = model && sameEntity(model.entity, selected) ? model : undefined

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault()
      clearCanvasSelection()
    }
  }

  return (
    <aside
      aria-label="Details"
      data-testid="details-panel"
      data-entity-kind={selected.kind}
      data-entity-id={selected.id}
      onKeyDown={onKeyDown}
      style={{ left: box.x, width: box.width }}
      className="absolute inset-y-0 z-[5] flex animate-in flex-col border-l bg-popover text-popover-foreground shadow-xl duration-200 fade-in-0 slide-in-from-right-4"
    >
      <header className="flex items-start gap-2 border-b px-5 pt-4 pb-3.5">
        <div className="min-w-0 flex-1">
          <p className="text-xs font-medium text-muted-foreground">
            {shown?.kind ??
              (selected.kind === "person" ? "Person" : "Marriage")}
          </p>
          <h2
            data-testid="details-title"
            className="mt-0.5 text-lg leading-snug font-semibold tracking-tight text-balance"
          >
            {shown?.title ?? selected.id}
          </h2>
          <p className="mt-0.5 truncate font-mono text-xs text-muted-foreground">
            {selected.id}
          </p>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Close the details"
          onClick={clearCanvasSelection}
        >
          <XIcon />
        </Button>
      </header>
      <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-5 py-4">
        <Button
          variant="outline"
          size="sm"
          className="self-start"
          onClick={() => goToSource(selected)}
          data-testid="go-to-source"
        >
          <CodeIcon />
          Go to source
        </Button>
        {shown ? (
          <Description model={shown} />
        ) : (
          hasErrors && (
            <p className="text-sm leading-relaxed text-muted-foreground">
              Fix the errors in your files to see these details.
            </p>
          )
        )}
      </div>
    </aside>
  )
}

function Description({ model }: { model: DetailsModel }) {
  return (
    <>
      {model.facts.length > 0 && (
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 text-sm">
          {model.facts.map((fact) => (
            <div key={fact.label} className="contents">
              <dt className="text-muted-foreground">{fact.label}</dt>
              <dd className="font-medium">{fact.value}</dd>
            </div>
          ))}
        </dl>
      )}
      {model.groups.map((group) => (
        <section key={group.title} className="flex flex-col gap-1.5">
          <h3 className="text-xs font-medium text-muted-foreground">
            {group.title}
          </h3>
          <ul className="flex flex-col gap-1">
            {group.rows.map((row) => (
              <li
                key={`${row.link.entity.kind}:${row.link.entity.id}:${row.also?.entity.id ?? ""}`}
                className="flex flex-wrap items-baseline gap-x-2 text-sm"
              >
                <EntityLink link={row.link} />
                {row.note && (
                  <span className="text-muted-foreground">{row.note}</span>
                )}
                {row.also && <EntityLink link={row.also} subtle />}
              </li>
            ))}
          </ul>
        </section>
      ))}
    </>
  )
}

function EntityLink({
  link,
  subtle = false,
}: {
  link: DetailsLink
  subtle?: boolean
}) {
  return (
    <button
      type="button"
      onClick={() => chooseEntity(link.entity)}
      className={
        subtle
          ? "rounded-sm text-xs text-link underline-offset-2 outline-none hover:underline focus-visible:ring-3 focus-visible:ring-ring/50"
          : "rounded-sm font-medium text-link underline-offset-2 outline-none hover:underline focus-visible:ring-3 focus-visible:ring-ring/50"
      }
    >
      {link.label}
    </button>
  )
}

import { SearchIcon } from "lucide-react"
import {
  Suspense,
  lazy,
  useEffect,
  useState,
  useSyncExternalStore,
} from "react"

import { Kbd } from "@/components/ui/kbd"

// The dialog and its search engine load the first time search opens.
const DocsSearchDialog = lazy(() =>
  import("./search-dialog").then((module) => ({
    default: module.DocsSearchDialog,
  }))
)

const noSubscription = () => () => undefined

/** "⌘K" on a Mac, "Ctrl K" elsewhere (and in the prerendered page). */
function useShortcutLabel(): string {
  return useSyncExternalStore(
    noSubscription,
    () => (/Mac|iPhone|iPad/.test(navigator.userAgent) ? "⌘K" : "Ctrl K"),
    () => "Ctrl K"
  )
}

/** "Search the docs": the header's button, and Ctrl+K / ⌘K anywhere. */
export function SearchButton() {
  const shortcut = useShortcutLabel()
  const [open, setOpen] = useState(false)
  const [used, setUsed] = useState(false)
  const show = (value: boolean) => {
    if (value) setUsed(true)
    setOpen(value)
  }

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault()
        setUsed(true)
        setOpen((value) => !value)
      }
    }
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [])

  return (
    <>
      <button
        type="button"
        data-testid="docs-search"
        onClick={() => show(true)}
        className="inline-flex h-8 items-center gap-2 rounded-lg border bg-muted/50 px-2 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none sm:w-52 [&_svg]:size-4"
      >
        <SearchIcon aria-hidden="true" />
        <span className="sr-only sm:not-sr-only">Search</span>
        <Kbd aria-hidden="true" className="ml-auto hidden sm:inline-flex">
          {shortcut}
        </Kbd>
      </button>
      {used && (
        <Suspense fallback={null}>
          <DocsSearchDialog open={open} onOpenChange={show} />
        </Suspense>
      )}
    </>
  )
}

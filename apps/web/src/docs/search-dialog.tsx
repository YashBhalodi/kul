import { Dialog } from "@base-ui/react/dialog"
import { TanstackProvider } from "fumadocs-core/framework/tanstack"
import { useStaticSearch } from "fumadocs-core/search/client"
import {
  SearchDialog,
  SearchDialogClose,
  SearchDialogContent,
  SearchDialogHeader,
  SearchDialogIcon,
  SearchDialogInput,
  SearchDialogList,
  SearchDialogOverlay,
} from "fumadocs-ui/components/dialog/search"
import { useMemo } from "react"
import { createPortal } from "react-dom"

/** The prerendered search index (routes/docs/search[.]json.ts). */
export const SEARCH_INDEX = "/docs/search.json"

/**
 * Docs search, computed in the browser from the prerendered index: it
 * needs no server and works offline once the index is cached.
 */
export function DocsSearchDialog({
  open,
  onOpenChange,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const search = useStaticSearch({ from: SEARCH_INDEX })
  const dialogHandle = useMemo(() => Dialog.createHandle(), [])
  return (
    // Results are links, which Fumadocs follows through the router.
    <TanstackProvider>
      <SearchDialog
        {...search}
        open={open}
        onOpenChange={onOpenChange}
        dialogHandle={dialogHandle}
      >
        {/* Fumadocs leaves the scrim where the dialog is rendered: in the
            site header, whose backdrop blur would make it the scrim's
            containing block and shrink it to the header. On the body it
            covers the page, under the dialog (z-50) and over the header. */}
        {createPortal(<SearchDialogOverlay className="z-40" />, document.body)}
        <SearchDialogContent>
          <SearchDialogHeader>
            <SearchDialogIcon />
            <SearchDialogInput />
            {/* The visible "Esc" is part of the button's name, as it should be. */}
            <SearchDialogClose aria-label="Esc: close search">
              Esc
            </SearchDialogClose>
          </SearchDialogHeader>
          <SearchDialogList />
        </SearchDialogContent>
      </SearchDialog>
    </TanstackProvider>
  )
}

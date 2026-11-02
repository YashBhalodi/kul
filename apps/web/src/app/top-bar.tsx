import { useFormatForDisplay } from "@tanstack/react-hotkeys"
import { Link } from "@tanstack/react-router"
import { BookOpenIcon, SearchIcon } from "lucide-react"

import { KulMark } from "@/components/kul-mark"
import { ThemeToggle } from "@/components/theme-toggle"
import { Button, buttonVariants } from "@/components/ui/button"
import { Kbd } from "@/components/ui/kbd"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { appStore } from "./app-store"
import { ProblemsButton } from "./editor/problems-button"
import { ExportMenu } from "./export-menu"
import { ProjectSwitcher } from "./project-switcher"
import { SaveStatus } from "./save-status"
import { appHotkeys } from "./shortcuts"

/** The app's slim top bar above the canvas. */
export function TopBar() {
  const format = useFormatForDisplay()
  return (
    <header className="relative z-20 flex h-12 shrink-0 items-center gap-3 border-b bg-background px-3">
      <Tooltip>
        <TooltipTrigger
          render={
            <Link
              to="/"
              aria-label="Kul home"
              className="flex size-8 items-center justify-center rounded-lg hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
            />
          }
        >
          <KulMark className="size-5" />
        </TooltipTrigger>
        <TooltipContent>Kul home</TooltipContent>
      </Tooltip>
      <span className="text-border" aria-hidden>
        /
      </span>
      <ProjectSwitcher />
      <SaveStatus />
      <ProblemsButton />

      <div className="ml-auto flex items-center gap-1.5">
        <ExportMenu />
        <Button
          variant="outline"
          className="w-56 justify-start gap-2 text-muted-foreground"
          onClick={appStore.actions.togglePalette}
          data-testid="palette-trigger"
        >
          <SearchIcon />
          <span className="flex-1 text-left">Commands</span>
          <Kbd>{format(appHotkeys["palette.open"].defaultHotkey)}</Kbd>
        </Button>
        <Tooltip>
          <TooltipTrigger
            render={
              <Link
                to="/docs/"
                aria-label="Docs"
                className={buttonVariants({ variant: "ghost", size: "icon" })}
              />
            }
          >
            <BookOpenIcon />
          </TooltipTrigger>
          <TooltipContent>Docs</TooltipContent>
        </Tooltip>
        <ThemeToggle />
      </div>
    </header>
  )
}

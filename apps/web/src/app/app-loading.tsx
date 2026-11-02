import { KulMark } from "@/components/kul-mark"

/** What the SPA shell shows while the app's code loads. */
export function AppLoading() {
  return (
    <div
      data-testid="app-loading"
      className="canvas-grid flex h-svh items-center justify-center bg-canvas"
    >
      <div className="flex flex-col items-center gap-3 text-sm text-muted-foreground">
        <KulMark className="size-8 animate-pulse" />
        <p>Opening Kul…</p>
      </div>
    </div>
  )
}

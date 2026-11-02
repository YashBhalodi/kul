import { Outlet, createFileRoute } from "@tanstack/react-router"

import { AppLoading } from "@/app/app-loading"
import { AppShell } from "@/app/app-shell"

// The app is client-only: the prerendered SPA shell (/_shell.html, which
// _redirects serves for every /app/* path) shows AppLoading until the client
// renders the route.
export const Route = createFileRoute("/app")({
  ssr: false,
  head: () => ({ meta: [{ title: "Kul" }] }),
  pendingComponent: AppLoading,
  component: () => (
    <AppShell>
      <Outlet />
    </AppShell>
  ),
})

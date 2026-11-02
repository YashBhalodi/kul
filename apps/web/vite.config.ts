import tailwindcss from "@tailwindcss/vite"
import { tanstackStart } from "@tanstack/react-start/plugin/vite"
import viteReact from "@vitejs/plugin-react"
import { defineConfig } from "vite"

import { docsPages, markdownPath } from "./src/docs/pages.ts"

// The site is fully static (dist/client). The landing page, the docs, their
// Markdown variants, the docs search index and the llms files are
// prerendered; /app is client-only and served by the SPA shell
// (/_shell.html), which _redirects maps every /app/* path to.
const SHELL_OUTPUT = "/_shell"

export default defineConfig({
  resolve: { tsconfigPaths: true },
  // The CSP allows fonts and scripts only from 'self', so no asset is ever
  // inlined as a data: URL.
  build: { assetsInlineLimit: 0 },
  // The prerender crawls the build through a Vite preview server. Pin it to
  // IPv4: where `localhost` resolves to both ::1 and 127.0.0.1, the server
  // can bind one and the crawler's requests reach the other.
  preview: { host: "127.0.0.1" },
  plugins: [
    tailwindcss(),
    tanstackStart({
      spa: {
        enabled: true,
        // The shell is rendered at /app, the only client-only route, so the
        // landing page keeps "/" for its own prerender.
        maskPath: "/app",
        prerender: { outputPath: SHELL_OUTPUT },
      },
      prerender: {
        enabled: true,
        crawlLinks: true,
        failOnError: true,
        // Links into the app are crawled from the landing page and docs;
        // only the shell itself renders /app.
        filter: (page) =>
          page.prerender?.outputPath === SHELL_OUTPUT ||
          !(page.path === "/app" || page.path.startsWith("/app/")),
      },
      pages: [
        { path: "/" },
        { path: "/llms.txt" },
        { path: "/llms-full.txt" },
        { path: "/docs/search.json" },
        ...docsPages.map((page) => ({ path: markdownPath(page) })),
      ],
    }),
    viteReact(),
  ],
})

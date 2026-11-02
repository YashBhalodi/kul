import tailwindcss from "@tailwindcss/vite"
import { tanstackStart } from "@tanstack/react-start/plugin/vite"
import viteReact from "@vitejs/plugin-react"
import { readFileSync } from "node:fs"

import { rehypeCodeDefaultOptions } from "fumadocs-core/mdx-plugins/rehype-code"
import { fumadocsMdx } from "fumadocs-mdx/vite"
import { defineConfig } from "vite"
import type { Plugin } from "vite"

import { siteVersion } from "./scripts/site-version.mjs"
import { docsPagePaths } from "./scripts/docs-pages.mjs"
import { diagnosticsDrift } from "./src/docs/content-checks.ts"
import { engineDiagnosticCodes } from "./src/highlight/node-engine.ts"
import { rehypeKul } from "./src/highlight/rehype-kul.ts"

// The site is fully static (dist/client). The landing page, the docs, their
// Markdown variants, the docs search index and the llms files are
// prerendered; /app is client-only and served by the SPA shell
// (/_shell.html), which _redirects maps every /app/* path to.
const SHELL_OUTPUT = "/_shell"

/**
 * Fails the build when the diagnostics reference and the engine disagree:
 * a code without an entry, or an entry for a code that doesn't exist.
 */
function diagnosticsReference(): Plugin {
  const page = new URL(
    "./content/docs/reference/diagnostics.mdx",
    import.meta.url
  )
  return {
    name: "kul-diagnostics-reference",
    buildStart() {
      const codes = engineDiagnosticCodes().map((entry) => entry.code)
      const problems = diagnosticsDrift(readFileSync(page, "utf8"), codes)
      if (problems.length > 0) {
        this.error(
          `the diagnostics reference is out of date:\n${problems.join("\n")}`
        )
      }
    },
  }
}

export default defineConfig({
  resolve: { tsconfigPaths: true },
  define: {
    "import.meta.env.KUL_SITE_VERSION": JSON.stringify(siteVersion()),
  },
  // The CSP allows fonts and scripts only from 'self', so no asset is ever
  // inlined as a data: URL.
  build: { assetsInlineLimit: 0 },
  environments: {
    client: {
      build: {
        rolldownOptions: {
          output: {
            // React and the router, which every page runs, as one file
            // rather than a dozen small ones: a page's first load fetches
            // fewer files before it can show anything.
            codeSplitting: {
              groups: [
                {
                  name: "framework",
                  test: /node_modules[\\/](react|react-dom|scheduler|seroval|seroval-plugins|@tanstack[\\/](react-router|router-core|history|react-store|store|start-client-core|react-start-client))[\\/]/,
                },
              ],
            },
          },
        },
      },
    },
  },
  // The prerender crawls the build through a Vite preview server. Pin it to
  // IPv4: where `localhost` resolves to both ::1 and 127.0.0.1, the server
  // can bind one and the crawler's requests reach the other.
  preview: { host: "127.0.0.1" },
  plugins: [
    diagnosticsReference(),
    // The docs' MDX: Kul blocks are highlighted and checked by the engine
    // (rehype-kul) before the default highlighter handles everything else.
    fumadocsMdx({
      index: false,
      globalOptions: {
        mdxOptions: {
          rehypePlugins: (plugins) => [rehypeKul, ...plugins],
          // The code element names its language, which titles the block.
          rehypeCodeOptions: {
            ...rehypeCodeDefaultOptions,
            addLanguageClass: true,
          },
        },
      },
    }),
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
        ...docsPagePaths().map((path) => ({ path })),
      ],
    }),
    viteReact(),
  ],
})

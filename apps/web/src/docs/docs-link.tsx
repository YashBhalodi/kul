import { Link } from "@tanstack/react-router"
import type { AnchorHTMLAttributes } from "react"

const DOCS_PAGE = /^(\/docs\/(?:[a-z0-9-]+\/)*)(?:#(.*))?$/

/**
 * A link in the docs. Links to docs pages go through the router, so moving
 * between pages doesn't reload the site; anything else (a file such as
 * /llms.txt, the app, another site) is a plain link.
 */
export function DocsLink({
  href = "",
  ...props
}: AnchorHTMLAttributes<HTMLAnchorElement>) {
  const page = DOCS_PAGE.exec(href)
  if (!page) return <a href={href} {...props} />
  // Every docs page is a match of the /docs/$ route; the router takes the
  // page's own path, which the source built (always with its slash).
  const path = page[1] as "/docs/"
  // A page is current only at its own address, not at its sub-pages'.
  return (
    <Link
      to={path}
      hash={page[2]}
      activeOptions={{ exact: true, includeHash: false }}
      {...props}
    />
  )
}

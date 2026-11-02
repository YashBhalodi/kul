import { createFileRoute } from "@tanstack/react-router"
import content from "virtual:kul-landing"

import { LandingPage } from "@/landing/landing-page"
import { SITE_ORIGIN } from "@/site"

const TITLE = "Kul: describe a family in plain text, get the family tree"
const DESCRIPTION =
  "Kul is a small plain-text language for families: people, marriages, births and adoptions. Write it the way you'd tell it, and Kul draws the family tree."

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [
      { title: TITLE },
      { name: "description", content: DESCRIPTION },
      { property: "og:type", content: "website" },
      { property: "og:site_name", content: "Kul" },
      { property: "og:title", content: TITLE },
      { property: "og:description", content: DESCRIPTION },
      { property: "og:url", content: `${SITE_ORIGIN}/` },
      { property: "og:image", content: `${SITE_ORIGIN}/og.png` },
      { property: "og:image:type", content: "image/png" },
      { property: "og:image:width", content: "1200" },
      { property: "og:image:height", content: "630" },
      {
        property: "og:image:alt",
        content: `The ${content.sample.familyName} family tree, drawn by Kul`,
      },
      { name: "twitter:card", content: "summary_large_image" },
    ],
    links: [{ rel: "canonical", href: `${SITE_ORIGIN}/` }],
  }),
  component: Landing,
})

function Landing() {
  return <LandingPage content={content} />
}

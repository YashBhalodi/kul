import { Link, createFileRoute } from "@tanstack/react-router"

import { SiteFooter, SiteHeader } from "@/components/site-chrome"
import { buttonVariants } from "@/components/ui/button"
import { SITE_ORIGIN } from "@/site"

export const Route = createFileRoute("/")({
  head: () => ({
    meta: [{ title: "Kul — describe a family in plain text" }],
    links: [{ rel: "canonical", href: `${SITE_ORIGIN}/` }],
  }),
  component: Landing,
})

const SAMPLE = `person ramesh  name:"Ramesh Sharma"  gender:male    born:1925
person sita    name:"Sita Sharma"    gender:female  born:~1928
person alice   name:"Alice Sharma"   gender:female  born:1950-04
  birth m_ramesh_sita

marriage m_ramesh_sita ramesh sita  start:1948-06-10`

const POINTS = [
  {
    title: "Write it the way you’d tell it",
    body: "People, the marriages between them, and the children of each marriage. Parents, siblings and cousins follow from those links.",
  },
  {
    title: "Honest about what you don’t know",
    body: "A date can be a day, a month, a year or roughly a year. Leave out what the records don’t say.",
  },
  {
    title: "The text is the record",
    body: "Plain .kul files you can read, compare and keep. Kul checks them as you type and draws the tree from them.",
  },
]

function Landing() {
  return (
    <div className="flex min-h-svh flex-col">
      <SiteHeader />
      <main className="flex-1">
        <section className="mx-auto grid max-w-6xl grid-cols-[minmax(0,1fr)] gap-12 px-4 pt-16 pb-20 sm:px-6 md:pt-24 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)] lg:items-center">
          <div className="flex flex-col gap-6">
            <h1 className="text-4xl font-semibold tracking-tight text-balance sm:text-5xl">
              Describe a family in plain text. Kul draws the tree.
            </h1>
            <p className="max-w-xl text-lg leading-relaxed text-muted-foreground">
              Kul is a small language for families: who people are, whom they
              married and which children came from each marriage. Write it down
              once and get a tree that stays true to the text.
            </p>
            <div className="flex flex-wrap gap-3">
              <a href="/app" className={buttonVariants({ size: "lg" })}>
                Open Kul
              </a>
              <Link
                to="/docs/"
                className={buttonVariants({ size: "lg", variant: "outline" })}
              >
                Learn the language
              </Link>
            </div>
          </div>
          <figure className="min-w-0 overflow-hidden rounded-xl border bg-code shadow-sm">
            <figcaption className="border-b px-4 py-2.5 font-mono text-sm text-muted-foreground">
              sharma.kul
            </figcaption>
            <pre
              tabIndex={0}
              className="overflow-x-auto p-4 text-sm leading-relaxed text-code-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
            >
              <code>{SAMPLE}</code>
            </pre>
          </figure>
        </section>
        <section
          aria-labelledby="why"
          className="border-t bg-muted/40 py-16 sm:py-20"
        >
          <div className="mx-auto max-w-6xl px-4 sm:px-6">
            <h2 id="why" className="text-2xl font-semibold tracking-tight">
              Why a language
            </h2>
            <ul className="mt-8 grid gap-8 md:grid-cols-3">
              {POINTS.map((point) => (
                <li key={point.title} className="flex flex-col gap-2">
                  <h3 className="text-base font-semibold">{point.title}</h3>
                  <p className="leading-relaxed text-muted-foreground">
                    {point.body}
                  </p>
                </li>
              ))}
            </ul>
          </div>
        </section>
      </main>
      <SiteFooter />
    </div>
  )
}

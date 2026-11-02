import { Link } from "@tanstack/react-router"
import { cn } from "cn"
import {
  ArrowRightIcon,
  CircleAlertIcon,
  DatabaseIcon,
  PenToolIcon,
  ShuffleIcon,
} from "lucide-react"
import type { ReactNode } from "react"

import { SiteFooter, SiteHeader } from "@/components/site-chrome"
import { buttonVariants } from "@/components/ui/button"
import { DocsLink } from "@/docs/docs-link"
import { REPO_URL } from "@/site"
import { KulLines, Prose } from "./kul-lines"
import type {
  LandingContent,
  LandingFeature,
  LandingSample,
  SampleNote,
} from "./landing-data"

const CONTAINER = "mx-auto w-full max-w-6xl px-4 sm:px-6"

function Section({
  id,
  eyebrow,
  title,
  intro,
  className,
  children,
}: {
  id: string
  eyebrow: string
  title: string
  intro?: ReactNode
  className?: string
  children: ReactNode
}) {
  return (
    <section aria-labelledby={id} className={cn("py-20 sm:py-24", className)}>
      <div className={CONTAINER}>
        <div className="max-w-3xl">
          <p className="text-sm font-semibold text-primary">{eyebrow}</p>
          <h2
            id={id}
            className="mt-2 text-3xl font-semibold tracking-tight text-balance sm:text-4xl"
          >
            {title}
          </h2>
          {intro && (
            <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
              {intro}
            </p>
          )}
        </div>
        {children}
      </div>
    </section>
  )
}

function Hero() {
  return (
    <section className="relative overflow-hidden border-b">
      <div
        aria-hidden="true"
        className="canvas-grid absolute inset-0 [mask-image:radial-gradient(ellipse_75%_70%_at_50%_0%,black,transparent)]"
      />
      <div className="relative mx-auto max-w-4xl px-4 pt-16 pb-20 text-center sm:px-6 sm:pt-24 sm:pb-24">
        <p className="text-sm font-semibold text-primary">
          A language for family trees
        </p>
        <h1 className="mt-4 text-4xl font-semibold tracking-tight text-balance sm:text-6xl">
          Describe a family in plain text. Kul draws the tree.
        </h1>
        <p className="mx-auto mt-6 max-w-2xl text-lg leading-relaxed text-pretty text-muted-foreground sm:text-xl">
          Kul is a small language for writing down a family: who the people are,
          who married whom, and which marriage each child was born or adopted
          into. You keep a record you can read line by line, and the tree always
          matches it.
        </p>
        <div className="mt-9 flex flex-wrap justify-center gap-3">
          <a
            href="/app"
            className={cn(
              buttonVariants({ size: "lg" }),
              "h-11 gap-2 px-5 text-base"
            )}
          >
            Open Kul
            <ArrowRightIcon aria-hidden="true" />
          </a>
          <Link
            to="/docs/"
            className={cn(
              buttonVariants({ size: "lg", variant: "outline" }),
              "h-11 px-5 text-base"
            )}
          >
            Learn the language
          </Link>
        </div>
        <p className="mt-5 text-sm text-muted-foreground">
          Free and open source, under the MIT licence.
        </p>
      </div>
    </section>
  )
}

const PROBLEMS = [
  {
    icon: DatabaseIcon,
    title: "Apps keep it in their database",
    body: "Genealogy apps hold your family in their own forms and storage. You can say what their fields allow, and getting it back out means an export file nobody reads.",
  },
  {
    icon: PenToolIcon,
    title: "A drawing is only a picture",
    body: "A diagram tool draws boxes and lines, but it doesn't know that one line is a marriage and one box is somebody's daughter. When the family changes, you redraw it by hand.",
  },
  {
    icon: ShuffleIcon,
    title: "Real families are messy",
    body: `A grandmother born "around 1925". A second marriage after a divorce. A child adopted at six. A household with more than one wife. A branch you can't connect to the rest yet.`,
  },
]

function Problem() {
  return (
    <Section
      id="problem"
      eyebrow="The problem"
      title="Family history doesn't fit in a form"
      intro="Writing a family down sounds simple until you try it with a real one."
    >
      <ul className="mt-12 grid gap-4 lg:grid-cols-3">
        {PROBLEMS.map(({ icon: Icon, title, body }) => (
          <li key={title} className="rounded-xl border bg-card p-6 shadow-xs">
            <span className="flex size-10 items-center justify-center rounded-lg bg-muted text-foreground">
              <Icon aria-hidden="true" className="size-5" />
            </span>
            <h3 className="mt-5 text-lg font-semibold">{title}</h3>
            <p className="mt-2 leading-relaxed text-muted-foreground">{body}</p>
          </li>
        ))}
      </ul>
    </Section>
  )
}

const SOURCE_OF_TRUTH = [
  {
    title: "Readable",
    body: "It reads like careful notes. Anyone can open the file and check it against what they were told.",
  },
  {
    title: "Diffable",
    body: "A change is a changed line. Keep the files in git and you can see who added which aunt, and when.",
  },
  {
    title: "Yours",
    body: "Plain `.kul` files you can copy, send to a cousin or open in any text editor.",
  },
]

function Idea() {
  return (
    <section
      aria-labelledby="idea"
      className="border-y bg-muted/50 py-20 sm:py-24"
    >
      <div className={cn(CONTAINER, "grid gap-12 lg:grid-cols-2 lg:gap-16")}>
        <div>
          <p className="text-sm font-semibold text-primary">The idea</p>
          <h2
            id="idea"
            className="mt-2 text-3xl font-semibold tracking-tight text-balance sm:text-4xl"
          >
            Write it down the way you'd tell it
          </h2>
          <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
            In Kul you describe the family as text: each person, each marriage,
            and the marriage each child came from. Kul works out the rest
            (parents, siblings, cousins, in-laws) and draws the tree.
          </p>
          <p className="mt-4 text-lg leading-relaxed text-muted-foreground">
            The text is the source of truth. Kul draws the tree from it, so to
            fix a date or add a cousin you edit a line.
          </p>
        </div>
        <dl className="flex flex-col divide-y rounded-xl border bg-card shadow-xs">
          {SOURCE_OF_TRUTH.map((point) => (
            <div key={point.title} className="px-6 py-5">
              <dt className="font-semibold">{point.title}</dt>
              <dd className="mt-1 leading-relaxed text-muted-foreground">
                <Prose text={point.body} />
              </dd>
            </div>
          ))}
        </dl>
      </div>
    </section>
  )
}

function noteText(
  note: SampleNote,
  sample: LandingSample
): {
  label: string
  body: string
} {
  const { facts } = sample
  switch (note.kind) {
    case "person":
      return {
        label: "person",
        body: `declares someone. \`${facts.personId}\` is the id the rest of the file uses for them, and \`name:\` is what the tree shows.`,
      }
    case "partialDate":
      return {
        label: facts.partialDate,
        body: `is a partial date: a month and a year, no day. Write what the records say, and \`${facts.circaDate}\` for a guess.`,
      }
    case "marriage":
      return {
        label: "marriage",
        body: `joins two people, here \`${facts.spouses[0]}\` and \`${facts.spouses[1]}\`, from the day it started. A marriage that ended also says when and why.`,
      }
    case "birth":
      return {
        label: "birth",
        body: `under a person names the marriage they were born into, here \`${facts.birthMarriage}\`. Parents, siblings and grandparents all follow from lines like this.`,
      }
  }
}

function HowItWorks({ sample }: { sample: LandingSample }) {
  return (
    <Section
      id="how-it-works"
      eyebrow="How it works"
      title="The text, and the tree Kul draws from it"
      intro={`This is the ${sample.familyName} family, one of the examples that come with Kul: ${String(sample.lines.length)} lines once the comments are taken out. Kul drew the tree from those lines when this page was built.`}
    >
      <div className="mt-12 grid gap-4 lg:grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)]">
        <figure className="flex min-w-0 flex-col overflow-hidden rounded-xl border bg-code shadow-xs">
          <figcaption className="flex items-center justify-between gap-3 border-b bg-card px-4 py-2.5 text-sm">
            <span className="truncate font-mono font-medium">
              {sample.file}
            </span>
            <span className="hidden truncate text-muted-foreground sm:inline">
              examples/{sample.example}
            </span>
          </figcaption>
          <KulLines
            lines={sample.lines}
            numbered
            marks={sample.notes}
            className="flex-1 px-3 py-4 text-[0.8125rem] sm:px-4"
          />
        </figure>
        <figure className="flex min-w-0 flex-col overflow-hidden rounded-xl border bg-canvas shadow-xs">
          <figcaption className="border-b bg-card px-4 py-2.5 text-sm font-medium">
            The tree Kul draws
          </figcaption>
          <div
            role="img"
            aria-label={`The ${sample.familyName} family tree, drawn by Kul from the file`}
            className="canvas-grid kul-tree landing-tree flex flex-1 items-center justify-center p-6 sm:p-8"
            // The engine's SVG, rendered at build time from the file:
            // classes and data-* attributes only, coloured by the token sheet.
            dangerouslySetInnerHTML={{ __html: sample.svg }}
          />
        </figure>
      </div>
      <ol className="mt-8 grid gap-x-8 gap-y-6 sm:grid-cols-2 lg:grid-cols-4">
        {sample.notes.map((note) => {
          const { label, body } = noteText(note, sample)
          return (
            <li key={note.number} className="flex gap-3">
              <span className="kul-note-number mt-0.5" aria-hidden="true">
                {note.number}
              </span>
              <p className="leading-relaxed text-muted-foreground">
                <code className="landing-code font-semibold text-foreground">
                  {label}
                </code>{" "}
                <Prose text={body} />
              </p>
            </li>
          )
        })}
      </ol>
    </Section>
  )
}

function FeatureCard({ feature }: { feature: LandingFeature }) {
  const failing = feature.diagnostics.length > 0
  return (
    <li
      className={cn(
        "flex flex-col rounded-xl border bg-card p-6 shadow-xs",
        failing && "md:col-span-2"
      )}
      data-feature={feature.id}
    >
      <h3 className="text-lg font-semibold">{feature.title}</h3>
      <p className="mt-2 leading-relaxed text-muted-foreground">
        <Prose text={feature.body} />
      </p>
      <div className="mt-5">
        <div className="overflow-hidden rounded-lg border bg-code">
          {feature.fileName && (
            <p className="border-b px-4 py-1.5 font-mono text-xs text-muted-foreground">
              {feature.fileName}
            </p>
          )}
          <KulLines
            lines={feature.lines}
            numbered={failing}
            className="px-4 py-3 text-[0.8125rem]"
          />
          {failing && (
            <ul
              aria-label="What Kul reports"
              className="flex flex-col gap-1.5 border-t bg-muted px-4 py-3 text-sm"
            >
              {feature.diagnostics.map((diagnostic) => (
                <li
                  key={`${diagnostic.code}:${String(diagnostic.line)}`}
                  className="flex gap-2"
                  data-landing-diagnostic=""
                >
                  <CircleAlertIcon
                    aria-hidden="true"
                    className="mt-0.5 size-4 shrink-0 text-destructive"
                  />
                  <span>
                    <span className="font-mono font-medium">
                      {diagnostic.code}
                    </span>
                    <span className="text-muted-foreground">
                      {" "}
                      · line {diagnostic.line}
                    </span>{" "}
                    <Prose text={diagnostic.message} />
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </li>
  )
}

function Features({ features }: { features: ReadonlyArray<LandingFeature> }) {
  return (
    <Section
      id="features"
      eyebrow="What the language handles"
      title="The awkward parts of real families"
      intro="Each of these takes a line or two of Kul. Kul checked every example on this page when the page was built."
      className="border-t"
    >
      <ul className="mt-12 grid gap-4 md:grid-cols-2">
        {features.map((feature) => (
          <FeatureCard key={feature.id} feature={feature} />
        ))}
      </ul>
    </Section>
  )
}

const PRINCIPLES = [
  {
    title: "Small and readable",
    body: "Two statements, `person` and `marriage`, plus a `birth` or `adoption` line under a child. You never write down siblings, cousins or in-laws; Kul works them out.",
  },
  {
    title:
      "Lenient where records are uncertain, strict where meaning is ambiguous",
    body: "A missing or approximate date is fine. A child who points at a marriage that doesn't exist is an error, and Kul tells you which line and why.",
  },
  {
    title: "One canonical way to draw a family",
    body: "The same text always gives the same tree. There are no boxes to drag around, so the picture can't drift from the record, and everyone who opens the file sees the same family.",
  },
]

function Principles() {
  return (
    <Section
      id="principles"
      eyebrow="Design principles"
      title="The rules behind the language"
      className="border-t bg-muted/50"
    >
      <ul className="mt-12 grid gap-10 lg:grid-cols-3 lg:gap-8">
        {PRINCIPLES.map((principle) => (
          <li key={principle.title} className="border-t-2 border-primary pt-5">
            <h3 className="text-lg font-semibold text-balance">
              {principle.title}
            </h3>
            <p className="mt-2 leading-relaxed text-muted-foreground">
              <Prose text={principle.body} />
            </p>
          </li>
        ))}
      </ul>
    </Section>
  )
}

const START_LINK =
  "group flex h-full flex-col rounded-xl border bg-card p-6 shadow-xs transition-colors hover:border-primary/60 hover:bg-accent focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"

function StartCard({
  title,
  body,
  primary = false,
}: {
  title: string
  body: string
  primary?: boolean
}) {
  return (
    <>
      <span className={cn("text-lg font-semibold", primary && "text-primary")}>
        {title}
        <ArrowRightIcon
          aria-hidden="true"
          className="ml-1.5 inline size-4 align-[-0.125em] transition-transform group-hover:translate-x-0.5"
        />
      </span>
      <span className="mt-2 leading-relaxed text-muted-foreground">{body}</span>
    </>
  )
}

function GetStarted() {
  return (
    <Section
      id="get-started"
      eyebrow="Get started"
      title="Try it on your own family"
      className="border-t"
    >
      <ul className="mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <li>
          <a href="/app" className={START_LINK}>
            <StartCard
              primary
              title="Open Kul"
              body="Start from an example family in the browser and make it yours."
            />
          </a>
        </li>
        <li>
          <DocsLink href="/docs/quick-start/" className={START_LINK}>
            <StartCard
              title="Quick start"
              body="Build a small family step by step, from the first person to the first tree."
            />
          </DocsLink>
        </li>
        <li>
          <a href={`${REPO_URL}/releases`} className={START_LINK}>
            <StartCard
              title="The kul command line"
              body="Check, format and export Kul files from scripts, CI and AI coding agents. Download it from GitHub Releases."
            />
          </a>
        </li>
        <li>
          <a href="/llms.txt" className={START_LINK}>
            <StartCard
              title="llms.txt"
              body="An index of the docs for AI assistants, so one can write Kul for you."
            />
          </a>
        </li>
      </ul>
    </Section>
  )
}

/** The landing page, from the content the build made with the engine. */
export function LandingPage({ content }: { content: LandingContent }) {
  return (
    <div className="flex min-h-svh flex-col">
      <SiteHeader />
      <main className="flex-1">
        <Hero />
        <Problem />
        <Idea />
        <HowItWorks sample={content.sample} />
        <Features features={content.features} />
        <Principles />
        <GetStarted />
      </main>
      <SiteFooter />
    </div>
  )
}

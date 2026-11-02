// "What the language handles" on the landing page: each point with a line
// or two of Kul. The build checks every example with the engine, together
// with the lines it hides (the people a marriage needs, the file a
// reference points into), and highlights the lines it shows.
// `code spans` in the copy are set as code.

/** A file of a feature's example project. */
export interface FeatureFile {
  name: string
  /** Lines the example needs but doesn't show, before the shown ones. */
  before?: string
  /** The lines the page shows. One file of a feature has them. */
  shown?: string
  /** Lines the example needs but doesn't show, after the shown ones. */
  after?: string
}

export interface FeatureSource {
  id: string
  title: string
  body: string
  files: ReadonlyArray<FeatureFile>
  /** Show the shown file's name above its lines. */
  showFileName?: boolean
  /** The diagnostic codes the example must produce; none when omitted. */
  expectErrors?: ReadonlyArray<string>
}

const FOUNDERS = `person chinua  name:"Chinua Okeke"  gender:male    born:~1925
person adaeze  name:"Adaeze Okeke"  gender:female  born:1930-03

marriage m_chinua_adaeze chinua adaeze  start:1948-11-02`

export const FEATURES: ReadonlyArray<FeatureSource> = [
  {
    id: "dates",
    title: "Dates as exact as your records",
    body: "A day, a month or only a year, and `~` when it's a guess. Write what you know; Kul compares rough dates generously and never asks you to invent a day.",
    files: [
      {
        name: "family.kul",
        shown: `person kamala  name:"Kamala Rao"  gender:female  born:~1933  died:1960-05`,
      },
    ],
  },
  {
    id: "death",
    title: "Death isn't divorce",
    body: "`died:` records a death and nothing else, so Adaeze stays married to Chinua on the tree after he dies. A marriage ends only when it says so, with `end:` and an `end_reason:` such as `divorce`.",
    files: [
      {
        name: "family.kul",
        shown: `person chinua  name:"Chinua Okeke"  gender:male  born:~1925  died:1998-05-12
marriage m_chinua_adaeze chinua adaeze  start:1948-11-02`,
        after: `person adaeze  name:"Adaeze Okeke"  gender:female  born:1930-03`,
      },
    ],
  },
  {
    id: "adoption",
    title: "Birth and adoption",
    body: "A child is born into one marriage and can be adopted into another at any age. Kul keeps both links and draws an adoption as its own kind of line.",
    files: [
      {
        name: "family.kul",
        before: `person eduardo  name:"Eduardo Reyes"   gender:male    born:1962-03-11
person luz      name:"Luz Reyes"       gender:female  born:1965-12-02
marriage m_eduardo_luz eduardo luz  start:1988-07-09
person carlos   name:"Carlos Mendoza"  gender:male    born:1973-02-18
person rosa     name:"Rosa Mendoza"    gender:female  born:1975-09-24
marriage m_carlos_rosa carlos rosa  start:2000-01-15
`,
        shown: `person dalisay  name:"Dalisay Mendoza"  gender:female  born:1990-08-03
  birth m_eduardo_luz
  adoption m_carlos_rosa  start:2002-11-30`,
      },
    ],
  },
  {
    id: "marriages",
    title: "More than one marriage",
    body: "People remarry after a divorce or a death, and some households hold several marriages at once. Each marriage keeps its own dates and its own children.",
    files: [
      {
        name: "family.kul",
        before: `person khalid  name:"Khalid Al-Rashid"  gender:male    born:1950-03-29
person aisha   name:"Aisha Al-Rashid"   gender:female  born:1955-07-11
person layla   name:"Layla Al-Rashid"   gender:female  born:1958-02-23
`,
        shown: `marriage m_khalid_aisha  khalid aisha  start:1975-05-04
marriage m_khalid_layla  khalid layla  start:1980-09-16`,
      },
    ],
  },
  {
    id: "lineages",
    title: "Families that don't connect yet",
    body: "Unrelated families sit side by side in one project, and someone with no known relatives still gets a card. Link them up when you find out how they fit.",
    files: [
      {
        name: "family.kul",
        before: `person jinho   name:"Jin-ho Park"   gender:male    born:1960-04-12
person soojin  name:"Soo-jin Park"  gender:female  born:1963-08-25
marriage m_jinho_soojin jinho soojin  start:1988-05-14
`,
        shown: `person nadia  name:"Nadia Haddad"  gender:female  born:~1985`,
      },
    ],
  },
  {
    id: "files",
    title: "One family, many files",
    body: "Split a big family into as many `.kul` files as you like. They share one set of ids, so a child in one file can belong to a marriage declared in another.",
    showFileName: true,
    files: [
      { name: "founders.kul", before: FOUNDERS },
      {
        name: "children.kul",
        shown: `person emeka  name:"Emeka Okeke"  gender:male  born:1950
  birth m_chinua_adaeze`,
      },
    ],
  },
  {
    id: "mistakes",
    title: "Mistakes explained as you type",
    body: "Kul checks the whole project while you write and says what's wrong in plain words, on the line where it happens. This is its message for a misspelt marriage id:",
    expectErrors: ["KUL-R02"],
    files: [
      { name: "founders.kul", before: FOUNDERS },
      {
        name: "children.kul",
        shown: `person emeka  name:"Emeka Okeke"  gender:male  born:1950
  birth m_chinua_adeze`,
      },
    ],
  },
]

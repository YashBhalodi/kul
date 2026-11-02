// The landing page's build-time content, as the page receives it from the
// `virtual:kul-landing` module. Everything here is produced by the engine
// when the site is built (src/landing/landing-content.ts).

import type { CodeLine } from "@/highlight/highlight-kul"

/** A numbered callout on one token of the sample. */
export interface SampleNote {
  kind: "person" | "partialDate" | "marriage" | "birth"
  /** 1-based, in line order. */
  number: number
  /** 0-based line and run index of the token in `lines`. */
  line: number
  run: number
}

/** "How it works": an example project's file beside the tree it draws. */
export interface LandingSample {
  /** The example's directory under examples/. */
  example: string
  /** The file shown. */
  file: string
  /** The family's surname, from its `family:` fields. */
  familyName: string
  /** The file's highlighted lines, comments left out. */
  lines: Array<CodeLine>
  notes: Array<SampleNote>
  /** Tokens the callouts quote. */
  facts: {
    personId: string
    spouses: [string, string]
    birthMarriage: string
    partialDate: string
    circaDate: string
  }
  /** The engine's theme-agnostic SVG of the tree. */
  svg: string
}

/** What the engine reports for an example that shows a mistake. */
export interface FeatureDiagnostic {
  code: string
  message: string
  /** 1-based, within the shown lines. */
  line: number
}

/** A point of "What the language handles", with its example. */
export interface LandingFeature {
  id: string
  title: string
  body: string
  /** The shown file's name, when the point is about files. */
  fileName?: string
  lines: Array<CodeLine>
  diagnostics: Array<FeatureDiagnostic>
}

export interface LandingContent {
  sample: LandingSample
  features: Array<LandingFeature>
}

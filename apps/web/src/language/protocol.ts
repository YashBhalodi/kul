// What the language worker and the main thread say to each other. Every
// answer carries the file versions it was computed from, so the main
// thread can tell a current answer from one that a newer edit has
// overtaken.

import type { Analysis, Token } from "kul-wasm"

export type {
  Analysis,
  CompletionItem,
  Diagnostic,
  Entity,
  EntityDetail,
  FileEdit,
  Hover,
  Location,
  OutlineSymbol,
  QuickFix,
  RenameResult,
  Severity,
  Span,
  SvgTheme,
  TextEdit,
  Token,
  TokenKind,
} from "kul-wasm"

export interface EngineInfo {
  /** The kul-core version the engine was built from. */
  core: string
  /** The Kul language version it implements. */
  language: string
}

/** A project file with the version of its text. */
export interface VersionedFile {
  name: string
  text: string
  version: number
}

/** The version of every file an answer was computed from, by name. */
export type FileVersions = Readonly<Record<string, number>>

/** An answer, with the file versions it was computed from. */
export interface Answer<T> {
  versions: FileVersions
  value: T
}

/** One file's highlighting from the lexer, for one version of its text. */
export interface LexicalTokens {
  file: string
  version: number
  tokens: Array<Token>
}

/**
 * One analysis of the whole project, pushed by the worker once edits
 * pause: the diagnostics, the tree, and every file's semantic tokens.
 */
export interface AnalysisReport extends Analysis {
  /** Semantic tokens by file name, for the versions in `files`. */
  semantic: Record<string, Array<Token>>
}

/** How long the worker waits after the last edit before analyzing. */
export const ANALYSIS_WAIT_MS = 200

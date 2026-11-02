/* tslint:disable */
/* eslint-disable */
/**
 * A declared entity, by id. The id keys `data-person-id` /
 * `data-marriage-id` in the rendered SVG.
 */
export interface Entity {
    /**
     * The declaration\'s id.
     */
    id: string;
    /**
     * Person or marriage.
     */
    kind: EntityKind;
}

/**
 * A file name and the version of its text.
 */
export interface FileVersion {
    /**
     * The file\'s name.
     */
    name: string;
    /**
     * The version passed with its text to `setFile`.
     */
    version: number;
}

/**
 * A half-open range `[start, end)` of offsets into one file\'s text.
 *
 * In JavaScript the offsets are UTF-16 code units, the unit CodeMirror and
 * `String` indexing use.
 */
export interface Span {
    /**
     * Offset of the first character in the range.
     */
    start: number;
    /**
     * Offset just past the last character in the range.
     */
    end: number;
}

/**
 * A neighbouring person plus the parenthood link that reaches them.
 *
 * One row per **link**, not per person: a person reached both by birth and by
 * a later adoption appears twice, with a different `link` each time. That is
 * the same path-identity discipline the kin-set results carry (ADR-0026) —
 * the engine does not collapse two distinct ties into one row.
 */
export interface LinkedPerson {
    /**
     * The neighbour, in the export shape.
     */
    person: ExportedPerson;
    /**
     * The `birth` / `adoption` link connecting them to the subject, carrying
     * its `biological` / `adoptive` kind and an adoption\'s own dates.
     */
    link: ExportedParenthoodLink;
}

/**
 * A range in a named project file.
 */
export interface Location {
    /**
     * The file\'s name, as the project names it (`family.kul`).
     */
    file: string;
    /**
     * The range in that file.
     */
    span: Span;
}

/**
 * A secondary place a [`Diagnostic`] points at.
 */
export interface Related {
    /**
     * What this place is (`first declared here as a person`).
     */
    message: string;
    /**
     * The place.
     */
    location: Location;
}

/**
 * Hover content for one position.
 */
export interface Hover {
    /**
     * The content, as Markdown.
     */
    markdown: string;
    /**
     * The range the content describes (the whole id, field or keyword).
     */
    span: Span;
}

/**
 * How `renderSvg` themes the tree.
 */
export type SvgTheme = "agnostic" | "fileExport";

/**
 * How serious a [`Diagnostic`] is.
 */
export type Severity = "error" | "warning" | "note";

/**
 * One analysis of the whole project: what was analyzed, what is wrong with
 * it, and the rendered tree.
 */
export interface Analysis {
    /**
     * Every file, in file order, with the version of the text analyzed —
     * compare it with the latest version sent to discard a stale answer.
     */
    files: FileVersion[];
    /**
     * Every diagnostic of the project. Filter by `location.file` for one
     * file\'s; a diagnostic without a location concerns the whole project.
     */
    diagnostics: Diagnostic[];
    /**
     * The theme-agnostic SVG of the family tree (semantic CSS classes, no
     * colours). Absent when the project has errors, or cannot be drawn.
     */
    svg?: string;
}

/**
 * One completion.
 */
export interface CompletionItem {
    /**
     * What the list shows, and what is inserted when `insert` is absent.
     */
    label: string;
    /**
     * What it inserts.
     */
    kind: CompletionKind;
    /**
     * A one-line description (a field\'s meaning, a person\'s name and birth
     * year, a marriage\'s spouses and years).
     */
    detail: string;
    /**
     * Text to insert instead of the label.
     */
    insert?: Insertion;
    /**
     * Whether the list should select this item first.
     */
    preselect: boolean;
}

/**
 * One entity\'s own fields plus its relational neighbourhood — the answer to
 * a single [`DetailTarget`].
 *
 * A tagged union over the three selectable entity kinds, so a consumer\'s
 * panel variant maps 1:1 onto a variant here. Absent fields are **omitted**
 * from the wire (the export shapes\' `skip_serializing_if`), never sent as an
 * empty placeholder.
 */
export type EntityDetail = { kind: "person"; person: ExportedPerson; parents: LinkedPerson[]; marriages: MarriageTie[]; children: LinkedPerson[] } | { kind: "marriage"; marriage: ExportedMarriage; spouses: ExportedPerson[]; children: LinkedPerson[] } | { kind: "adoption"; adoption: ExportedParenthoodLink; child: ExportedPerson; parents: ExportedPerson[] };

/**
 * One highlighted token.
 */
export interface Token {
    /**
     * The token\'s text range. Never empty and never spans a line break.
     */
    span: Span;
    /**
     * What it is.
     */
    kind: TokenKind;
}

/**
 * One marriage a person is a spouse in, with the person on the other side.
 *
 * Each marriage is its own row rather than a joined spouse list, because the
 * marriage\'s `start` / `end` / `endReason` are what explain a person\'s
 * history (ADR-0035).
 */
export interface MarriageTie {
    /**
     * The marriage, in the export shape.
     */
    marriage: ExportedMarriage;
    /**
     * The other spouse. Absent only when the marriage names no second
     * person — which a project that passes its checks cannot do (R02, R04) —
     * so consumers omit the field rather than rendering a placeholder.
     */
    spouse?: ExportedPerson;
}

/**
 * One outline entry.
 */
export interface OutlineSymbol {
    /**
     * The label: a display name, or the id when there is none.
     */
    name: string;
    /**
     * A short date summary (`1900–1990`, `b. 1950`), when there are dates.
     */
    detail?: string;
    /**
     * What it stands for.
     */
    kind: SymbolKind;
    /**
     * The whole statement, sub-statements included.
     */
    span: Span;
    /**
     * The part to select when jumping to it: the id, or the keyword of a
     * sub-statement.
     */
    selection: Span;
    /**
     * A person\'s `birth` and `adoption` sub-statements.
     */
    children: OutlineSymbol[];
}

/**
 * One problem the parser, resolver or validator found.
 */
export interface Diagnostic {
    /**
     * Stable code: `KUL-Lxx` (lexer), `KUL-Pxx` (parser), `KUL-Rxx`
     * (validator rule), `KUL-Vxx` (render).
     */
    code: string;
    /**
     * How serious it is.
     */
    severity: Severity;
    /**
     * The message, written for the author.
     */
    message: string;
    /**
     * Where it is. Absent for a problem with the project as a whole.
     */
    location?: Location;
    /**
     * Other places that explain it (a duplicate\'s first declaration).
     */
    related: Related[];
}

/**
 * One repair for one diagnostic. Its edits all apply to the file the fix
 * was asked for.
 */
export interface QuickFix {
    /**
     * What the fix does (`Add `gender:female``).
     */
    title: string;
    /**
     * The diagnostic it repairs.
     */
    diagnostic: Diagnostic;
    /**
     * Its edits to the file, in source order.
     */
    edits: TextEdit[];
}

/**
 * Parenthood kind. Wire form: `\"biological\"` / `\"adoptive\"`.
 */
export type ParenthoodLinkKind = "biological" | "adoptive";

/**
 * Projected date: `value` (no circa marker), `precision`
 * (year/month/day), and `circa` flag.
 */
export interface ExportedDate {
    value: string;
    precision: string;
    circa: boolean;
}

/**
 * Replace the text in `span` with `text`. An empty span inserts.
 */
export interface TextEdit {
    /**
     * The range to replace.
     */
    span: Span;
    /**
     * The replacement text.
     */
    text: string;
}

/**
 * Text to insert in place of an item\'s label, with where the cursor lands.
 */
export interface Insertion {
    /**
     * The text.
     */
    text: string;
    /**
     * Offset into `text` where the cursor goes after inserting it.
     */
    cursor: number;
}

/**
 * The answer to `rename`: the edits to apply, or why the rename was
 * refused.
 */
export type RenameResult = { ok: true; edits: FileEdit[] } | { ok: false; message: string };

/**
 * The edits one change makes to one file, in source order. Ranges are
 * against the file\'s text before any of the edits apply, and never overlap.
 */
export interface FileEdit {
    /**
     * The file\'s name.
     */
    file: string;
    /**
     * Its edits, in source order.
     */
    edits: TextEdit[];
}

/**
 * The two kinds of entity a Kul project declares.
 */
export type EntityKind = "person" | "marriage";

/**
 * What a [`CompletionItem`] inserts.
 */
export type CompletionKind = "keyword" | "field" | "enumValue" | "value" | "marriage" | "person";

/**
 * What a highlighted token is.
 *
 * Lexical tokens are classified from the lexer alone; semantic tokens from
 * the checked syntax tree, and they refine the lexical ones where both cover
 * the same text.
 */
export type TokenKind = "keyword" | "fieldName" | "declaration" | "reference" | "enumValue" | "date" | "string" | "comment";

/**
 * What an [`OutlineSymbol`] stands for.
 */
export type SymbolKind = "person" | "marriage" | "birth" | "adoption";

export interface ExportedMarriage {
    id: string;
    /**
     * Two spouse ids, in declaration order. Both resolve to entries in
     * `persons` (export refuses otherwise).
     */
    spouses: [string, string];
    start?: ExportedDate;
    end?: ExportedDate;
    endReason?: string;
    /**
     * `[byte_start, byte_end]`. Present iff `with_positions`.
     */
    span?: [number, number];
}

export interface ExportedParenthoodLink {
    marriageId: string;
    childId: string;
    kind: ParenthoodLinkKind;
    /**
     * `start:` of an adoption. Absent for biological links.
     */
    start?: ExportedDate;
    /**
     * `end:` of an adoption. Absent for biological links.
     */
    end?: ExportedDate;
    /**
     * `[byte_start, byte_end]`. Present iff `with_positions`.
     */
    span?: [number, number];
}

export interface ExportedPerson {
    id: string;
    name: string;
    family?: string;
    given?: string;
    gender: string;
    born?: ExportedDate;
    died?: ExportedDate;
    /**
     * `[byte_start, byte_end]`. Present iff `with_positions`.
     */
    span?: [number, number];
}


/**
 * The version of the Kul toolchain this module was built from.
 */
export function KUL_CORE_VERSION(): string;

/**
 * The version of the Kul language this module implements.
 */
export function KUL_LANGUAGE_VERSION(): string;

/**
 * One project's `.kul` files, with the language features over them.
 *
 * Files are keyed by name; the project's file order is the byte-wise order
 * of those names. Edits replace a file's whole text. Analysis runs on
 * demand and is cached until the next edit.
 */
export class Workspace {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Check the project: its diagnostics, and its tree when it has none
     * that are errors.
     */
    analyze(): Analysis;
    /**
     * The completions at `offset` in `file`: the full set for the position,
     * to be filtered by the word already typed.
     */
    complete(file: string, offset: number): CompletionItem[];
    /**
     * The declaration the reference at `offset` in `file` names.
     */
    definition(file: string, offset: number): Location | undefined;
    /**
     * Remove the file `name`. Returns `false` when there is no such file.
     */
    deleteFile(name: string): boolean;
    /**
     * The facts about the person or marriage `id`, for a details panel;
     * `undefined` when there is no such entity or the project has errors.
     */
    details(id: string): EntityDetail | undefined;
    /**
     * The entity whose id sits at `offset` in `file`.
     */
    entityAt(file: string, offset: number): Entity | undefined;
    /**
     * The edits that format `file` canonically: none when it already is,
     * `undefined` when it has syntax errors.
     */
    format(file: string): TextEdit[] | undefined;
    /**
     * Hover content for `offset` in `file`.
     */
    hover(file: string, offset: number): Hover | undefined;
    /**
     * Highlighting for `file` from the lexer alone. Needs no analysis, so it
     * is cheap enough for every keystroke.
     */
    lexicalTokens(file: string): Token[];
    /**
     * The declaration of the person or marriage `id`.
     */
    locate(id: string): Location | undefined;
    /**
     * An empty project.
     */
    constructor();
    /**
     * The range a rename at `offset` in `file` would change, or `undefined`
     * when nothing there can be renamed.
     */
    prepareRename(file: string, offset: number): Span | undefined;
    /**
     * The quick fixes for diagnostics in `file` touching `start..end`.
     */
    quickFixes(file: string, start: number, end: number): QuickFix[];
    /**
     * Every use of the id at `offset` in `file`, optionally with its
     * declaration; `undefined` when the cursor is not on an id.
     */
    references(file: string, offset: number, includeDeclaration: boolean): Location[] | undefined;
    /**
     * Rename the id at `offset` in `file` to `newName`, across the project.
     */
    rename(file: string, offset: number, newName: string): RenameResult;
    /**
     * Rename the file `from` to `to`, keeping its text and version. Returns
     * `false`, changing nothing, when `from` does not exist or `to` does.
     */
    renameFile(from: string, to: string): boolean;
    /**
     * The SVG of the family tree in `theme`, or `undefined` when the project
     * has errors or cannot be drawn.
     */
    renderSvg(theme: SvgTheme): string | undefined;
    /**
     * Highlighting for `file` from its syntax tree. Paint these over
     * `lexicalTokens` where both cover the same text.
     */
    semanticTokens(file: string): Token[];
    /**
     * Add the file `name`, or replace its text. `version` tags the text: an
     * analysis reports the version it saw for each file.
     */
    setFile(name: string, text: string, version: number): void;
    /**
     * The outline of `file`.
     */
    symbols(file: string): OutlineSymbol[];
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly KUL_CORE_VERSION: (a: number) => void;
    readonly KUL_LANGUAGE_VERSION: (a: number) => void;
    readonly __wbg_workspace_free: (a: number, b: number) => void;
    readonly workspace_analyze: (a: number) => number;
    readonly workspace_complete: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly workspace_definition: (a: number, b: number, c: number, d: number) => number;
    readonly workspace_deleteFile: (a: number, b: number, c: number) => number;
    readonly workspace_details: (a: number, b: number, c: number) => number;
    readonly workspace_entityAt: (a: number, b: number, c: number, d: number) => number;
    readonly workspace_format: (a: number, b: number, c: number, d: number) => void;
    readonly workspace_hover: (a: number, b: number, c: number, d: number) => number;
    readonly workspace_lexicalTokens: (a: number, b: number, c: number, d: number) => void;
    readonly workspace_locate: (a: number, b: number, c: number) => number;
    readonly workspace_new: () => number;
    readonly workspace_prepareRename: (a: number, b: number, c: number, d: number) => number;
    readonly workspace_quickFixes: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workspace_references: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workspace_rename: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly workspace_renameFile: (a: number, b: number, c: number, d: number, e: number) => number;
    readonly workspace_renderSvg: (a: number, b: number, c: number) => void;
    readonly workspace_semanticTokens: (a: number, b: number, c: number, d: number) => void;
    readonly workspace_setFile: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly workspace_symbols: (a: number, b: number, c: number, d: number) => void;
    readonly __wbindgen_export: (a: number, b: number) => number;
    readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

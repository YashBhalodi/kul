// TypeScript consumer compile-test for the `kul-wasm` module.
//
// Runs under `tsc --noEmit` in `just wasm-check`. Catches the case where the
// generated `.d.ts` compiles against itself but isn't usable from real
// consumer code: a language worker driving a `Workspace`, an editor mapping
// its results, a canvas reading `analyze()` and `details()`.

import init, {
    KUL_CORE_VERSION,
    KUL_LANGUAGE_VERSION,
    Workspace,
    diagnosticCodes,
    type DiagnosticCode,
    type Analysis,
    type CompletionItem,
    type Diagnostic,
    type EntityDetail,
    type FileEdit,
    type Location,
    type OutlineSymbol,
    type Severity,
    type Span,
    type SvgTheme,
    type Token,
    type TokenKind,
} from '../../pkg/kul_wasm.js';

// Loading: the default export fetches and instantiates the module.
export async function load(url: URL): Promise<string> {
    await init({ module_or_path: url });
    return `${KUL_CORE_VERSION()} implementing Kul ${KUL_LANGUAGE_VERSION()}`;
}

// The docs check their diagnostics reference against the catalogue.
const catalogue: DiagnosticCode[] = diagnosticCodes();
const documented: string[] = catalogue.map((entry) => `${entry.code}: ${entry.summary}`);

// A worker keeps one workspace and tags every text with a version.
const ws = new Workspace();
ws.setFile('family.kul', 'person alice name:"Alice" gender:female\n', 1);
ws.setFile('children.kul', '', 1);
const renamedFile: boolean = ws.renameFile('children.kul', 'kids.kul');
const deleted: boolean = ws.deleteFile('kids.kul');
// @ts-expect-error — every text carries its version.
ws.setFile('family.kul', 'person bob');

// analyze(): per-file versions for stale-answer checks, project diagnostics,
// and the svg only when the tree can be drawn.
const analysis: Analysis = ws.analyze();
const latest = new Map<string, number>([['family.kul', 1]]);
const fresh: boolean = analysis.files.every((f) => latest.get(f.name) === f.version);
const forFile = (file: string): Diagnostic[] =>
    analysis.diagnostics.filter((d) => d.location?.file === file);
const projectWide: Diagnostic[] = analysis.diagnostics.filter((d) => d.location === undefined);
const errors: number = analysis.diagnostics.filter((d) => d.severity === 'error').length;
const severities: Severity[] = ['error', 'warning', 'note'];
const svg: string = analysis.svg ?? '<svg/>';
const relatedFiles: string[] = analysis.diagnostics.flatMap((d) => d.related.map((r) => r.location.file));

// Offsets are UTF-16 code units: they index JS strings directly.
const text = 'person alice name:"Alice" gender:female\n';
const slice = (span: Span): string => text.slice(span.start, span.end);

const hovered: string | undefined = ws.hover('family.kul', 8)?.markdown;
const hoverRange: string = slice(ws.hover('family.kul', 8)?.span ?? { start: 0, end: 0 });

const items: CompletionItem[] = ws.complete('family.kul', 34);
const applied: string[] = items.map((item) =>
    item.insert === undefined ? item.label : item.insert.text.slice(0, item.insert.cursor),
);
const preselected: CompletionItem | undefined = items.find((item) => item.preselect);
const isId: boolean = items.some((item) => item.kind === 'person' || item.kind === 'marriage');

const target: Location | undefined = ws.definition('family.kul', 8);
const uses: Location[] = ws.references('family.kul', 8, true) ?? [];
const declarationSite: string | undefined = ws.locate('alice')?.file;

// rename(): a discriminated union on `ok`.
const editable: Span | undefined = ws.prepareRename('family.kul', 8);
const result = ws.rename('family.kul', 8, 'alicia');
let edits: FileEdit[] = [];
if (result.ok) {
    edits = result.edits;
    // @ts-expect-error — a successful rename carries no message.
    void result.message;
} else {
    const why: string = result.message;
    void why;
}
const touched: string[] = edits.map((edit) => edit.file);

const outline: OutlineSymbol[] = ws.symbols('family.kul');
const flat: string[] = outline.flatMap((s) => [s.name, ...s.children.map((c) => c.name)]);

const fixes = ws.quickFixes('family.kul', 0, text.length);
const fixTitles: string[] = fixes.map((fix) => `${fix.diagnostic.code}: ${fix.title}`);
const formatting = ws.format('family.kul');
const canonical: boolean = formatting !== undefined && formatting.length === 0;

// Tokens map onto CSS classes; the switch must cover every kind.
function cssClass(kind: TokenKind): string {
    switch (kind) {
        case 'keyword':
        case 'fieldName':
        case 'declaration':
        case 'reference':
        case 'enumValue':
        case 'date':
        case 'string':
        case 'comment':
            return `tok-${kind}`;
    }
}
const painted: string[] = [...ws.lexicalTokens('family.kul'), ...ws.semanticTokens('family.kul')].map(
    (token: Token) => cssClass(token.kind),
);

// The canvas: which entity the cursor is on, and its details.
const linked = ws.entityAt('family.kul', 8);
const linkedId: string | undefined = linked?.kind === 'person' ? linked.id : undefined;

function describe(detail: EntityDetail): string {
    switch (detail.kind) {
        case 'person':
            return `${detail.person.name}: ${detail.parents.length} parents, ${detail.marriages
                .map((tie) => `${tie.spouse?.name ?? '?'} (${tie.marriage.start?.value ?? ''})`)
                .join(', ')}, ${detail.children.length} children`;
        case 'marriage':
            return `${detail.spouses.map((p) => p.name).join(' & ')}, ${detail.marriage.endReason ?? 'ongoing'}`;
        case 'adoption':
            return `${detail.child.name} adopted ${detail.adoption.start?.value ?? ''}`;
    }
}
const panel: string = describe(ws.details('alice') ?? ws.details('m')!);
const adoptive: boolean =
    ws.details('alice')?.kind === 'person' &&
    (ws.details('alice') as Extract<EntityDetail, { kind: 'person' }>).parents.some(
        (p) => p.link.kind === 'adoptive',
    );

// renderSvg(): two themes, nothing else.
const themes: SvgTheme[] = ['agnostic', 'fileExport'];
const download: string | undefined = ws.renderSvg('fileExport');
// @ts-expect-error — no such theme.
ws.renderSvg('dark');

ws.free();

export const used = [
    renamedFile,
    deleted,
    fresh,
    forFile,
    projectWide,
    errors,
    severities,
    svg,
    relatedFiles,
    hovered,
    hoverRange,
    applied,
    preselected,
    isId,
    target,
    uses,
    declarationSite,
    editable,
    touched,
    flat,
    fixTitles,
    canonical,
    painted,
    linkedId,
    panel,
    adoptive,
    themes,
    download,
    documented,
];

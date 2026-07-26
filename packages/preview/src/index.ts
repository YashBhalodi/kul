export {
    previewHtml,
    getNonce,
    CHROME_LANG,
    MOUNT_POINT_ID,
    ENGINE_MODULE_ATTR,
    ENGINE_WASM_ATTR,
} from "./html.js";
export type { PreviewHtmlOptions } from "./html.js";

export { mountPreview } from "./mount.js";
export type { MountOptions } from "./mount.js";

export {
    createVscodeAdapter,
    createVscodeLocaleStore,
    installVscodeInboundBridge,
} from "./adapter-vscode.js";

// The locale toggle and the store its choice persists in (ADR-0033).
export { createLocaleController, LOCALE_TOGGLE_HTML } from "./locale.js";
export type { LocaleController } from "./locale.js";
export { createMemoryLocaleStore } from "./locale-store.js";
export type { LocaleStore } from "./locale-store.js";

export { isEntityKind, isProjectSnapshot, isRevealTarget } from "./wire-guards.js";

// The query transport (ADR-0034 / ADR-0040).
export { createQueryEngine, loadEngineModule } from "./engine.js";
export type {
    EngineModule,
    EngineModuleLoader,
    EngineSource,
    ProjectSnapshot,
    QueryEngine,
} from "./engine.js";
export { isQueryOk } from "./engine-wire.js";
export type {
    DetailLookupResult,
    DetailTarget,
    EmptyReason,
    EntityDetail,
    ExportedDiagnostic,
    ExportedMarriage,
    ExportedPerson,
    KinPattern,
    LinkedPerson,
    Manifest,
    MarriageTie,
    Member,
    Projection,
    Query,
    QueryEnvelope,
    QueryResult,
    ResolveResult,
    WasmInputFile,
} from "./engine-wire.js";
export { selectBoundNodes } from "./result-binding.js";
export type { ResultBinding } from "./result-binding.js";

// Explore kin (ADR-0043) — the kin-set catalogue, the list it renders and the
// paint an answered set puts on the tree.
export { KIN_SETS, kinQuery, kinSetById } from "./kin-sets.js";
export type { KinSet } from "./kin-sets.js";
export { KIN_LIST_TITLE, buildKinList, emptyKinMessage } from "./kin-list.js";
export type { KinListModel, KinListState, KinRowModel } from "./kin-list.js";
export {
    KIN_DIM_SOURCE,
    RESULT_CLASS,
    clearKinPaint,
    paintKinResults,
} from "./kin-paint.js";
export type { KinAnswer } from "./kin-paint.js";
// The dim: one class, one owner, union over every paint source (ADR-0043).
export { DIM_CLASS, createDimRegistry } from "./dim.js";
export type { DimRegistry, DimSource } from "./dim.js";
export { TOAST_DURATION_MS, createToaster } from "./toast.js";
export type { Toaster } from "./toast.js";

// The selection seam (ADR-0035, ADR-0042) — one selection over three entity
// kinds, and the surface every later piece of query chrome hangs off.
export {
    SELECTION_CLASS,
    clearSelectionPaint,
    createSelectionStore,
    isSameSelection,
    paintSelection,
    selectionAnchorPerson,
    selectionAtElement,
    selectionNodes,
} from "./selection.js";
export type {
    EntitySelection,
    SelectionListener,
    SelectionStore,
} from "./selection.js";
export { REVEAL_LABEL, buildDetailPanel, createDetailPanel } from "./detail-panel.js";
export type {
    DetailPanel,
    DetailPanelModel,
    DetailPanelView,
    PanelField,
    PanelRow,
    PanelSection,
} from "./detail-panel.js";
// The docked tag (ADR-0044) — the whisper grammar itself: a tag positioned
// against an entity's screen box on the float layer. Decides nothing about when
// one appears or what it says, so a second consumer supplies its own content and
// its own trigger without touching the hover lens.
export { DOCKED_TAG_CLASS, openDockedTag } from "./docked-tag.js";
export type { DockedTag, DockedTagOptions } from "./docked-tag.js";

// The hover lens (#302, ADR-0044) — relationship resolution with no click.
export {
    LENS_CLASS,
    LENS_DEBOUNCE_MS,
    LENS_PATH_CLASS,
    NOT_RELATED_DISCONNECTED,
    NOT_RELATED_WITHIN_BOUNDS,
    VIEWPOINT_TITLE,
    createHoverLens,
    emptinessWhisper,
    resolutionPathBindings,
} from "./hover-lens.js";
export type { HoverLens, HoverLensOptions } from "./hover-lens.js";
export { SYNC_SUSPENDED_HINT, createQuerySurface } from "./query-surface.js";
export type {
    QuerySurface,
    QuerySurfaceOptions,
    SyncSuspensionReason,
} from "./query-surface.js";
export { canonicalCardFor, panToElement, visibleCentre } from "./highlight.js";
export type { HighlightPanZoom, ScreenBox } from "./highlight.js";

export type {
    EntityRef,
    ErrorRow,
    HostAdapter,
    LspPosition,
    LspRange,
    PreviewHandle,
    RevealTarget,
    WireHighlightEntityMessage,
    WireInboundMessage,
    WireOutboundMessage,
    WireRenderErrorMessage,
    WireRenderMessage,
    WireRevealRequestMessage,
} from "./types.js";

// Re-exports for direct consumption by tests / future webapp.
export { LEGEND_ROWS, legendSwatchInnerSvg, presentLegendRows } from "./legend.js";
export type { LegendRow } from "./legend.js";

// The phrasing layer (ADR-0033) — descriptor → word. A pure module with no DOM
// imports, so extracting it to its own package stays mechanical.
export { PACKS, packFor, phrase } from "./phrasing/index.js";
export type {
    LanguagePack,
    Phrase,
    PhraseKind,
    PhrasingKey,
    RelationshipDescriptor,
} from "./phrasing/index.js";

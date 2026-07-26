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
    EntityDetail,
    ExportedDiagnostic,
    ExportedMarriage,
    ExportedPerson,
    LinkedPerson,
    Manifest,
    MarriageTie,
    QueryEnvelope,
    WasmInputFile,
} from "./engine-wire.js";
export { selectBoundNodes } from "./result-binding.js";
export type { ResultBinding } from "./result-binding.js";

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
export { buildTooltip } from "./tooltip.js";
export type { TooltipModel } from "./tooltip.js";
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

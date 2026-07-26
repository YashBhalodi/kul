export { previewHtml, getNonce, MOUNT_POINT_ID } from "./html.js";
export type { PreviewHtmlOptions } from "./html.js";

export { mountPreview } from "./mount.js";

export { createVscodeAdapter, installVscodeInboundBridge } from "./adapter-vscode.js";

export { isEntityKind, isRevealTarget } from "./wire-guards.js";

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

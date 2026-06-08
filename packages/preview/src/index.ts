export { previewHtml, getNonce, MOUNT_POINT_ID } from "./html.js";
export type { PreviewHtmlOptions } from "./html.js";

export { mountPreview } from "./mount.js";

export { createVscodeAdapter, installVscodeInboundBridge } from "./adapter-vscode.js";

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

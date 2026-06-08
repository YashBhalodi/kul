/** LSP-style position; 0-based line + character. */
export interface LspPosition {
    line: number;
    character: number;
}

export interface LspRange {
    start: LspPosition;
    end: LspPosition;
}

/** Click target the host should reveal in its source editor. */
export type RevealTarget =
    | { kind: "entity"; id: string }
    | { kind: "location"; uri: string; range: LspRange };

/** Entity the preview is currently highlighting (selection-sync). */
export interface EntityRef {
    id: string;
    kind: "person" | "marriage";
}

/** One error-popover row (#203). */
export interface ErrorRow {
    message: string;
    code?: string;
    /** Anchored diagnostics carry uri + range; transport-failure rows omit them. */
    uri?: string;
    range?: LspRange;
}

/**
 * The host's outbound seam. The chrome calls a single method, discriminated by
 * `kind`. Card / marriage clicks fire `entity`; error-popover rows fire
 * `location`.
 */
export interface HostAdapter {
    onRevealRequest(target: RevealTarget): void;
}

/**
 * Handle returned by {@link mountPreview}. The host drives the chrome
 * imperatively through these methods; the VSCode adapter wraps inbound
 * `webview.postMessage` events into the equivalent calls.
 */
export interface PreviewHandle {
    render(svg: string): void;
    showErrors(errors: ErrorRow[]): void;
    highlightEntity(ref: EntityRef | null): void;
    dispose(): void;
}

// --- Wire format (VSCode webview ↔ extension) ---------------------------

export interface WireRenderMessage {
    type: "render";
    svg: string;
}

export interface WireRenderErrorMessage {
    type: "renderError";
    errors: ErrorRow[];
}

export interface WireHighlightEntityMessage {
    type: "highlightEntity";
    /** Null clears the highlight; a non-null id requires `kind`. */
    id: string | null;
    kind?: "person" | "marriage";
}

/** All inbound messages the chrome consumes. */
export type WireInboundMessage =
    | WireRenderMessage
    | WireRenderErrorMessage
    | WireHighlightEntityMessage;

/** Outbound messages the chrome posts back to the host. */
export interface WireRevealRequestMessage {
    type: "revealRequest";
    target: RevealTarget;
}

export type WireOutboundMessage = WireRevealRequestMessage;

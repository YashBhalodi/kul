import type {
    DetailLookupResult,
    DetailTarget,
    Query,
    QueryEnvelope,
    QueryResult,
    ResolveResult,
} from "./engine-wire.js";
import type { ProjectSnapshot } from "./engine.js";
import type { LocaleController } from "./locale.js";

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
    /**
     * Swap in a rendered picture, optionally with the project source it came
     * from. The engine is stateless, so a query needs that source; a render
     * without it leaves the chrome unable to answer queries about the picture
     * it just drew (ADR-0034).
     */
    render(svg: string, project?: ProjectSnapshot | null): void;
    showErrors(errors: ErrorRow[]): void;
    highlightEntity(ref: EntityRef | null): void;
    /**
     * Batched detail lookup against the project the current picture came from
     * (ADR-0037). Loads the engine on first call and never before.
     *
     * Resolves `null` when there is nothing to ask — no project has been
     * rendered yet, or no engine was supplied. An envelope's error arm means
     * the project failed its checks; its diagnostics are surfaced in the error
     * popover as well as returned, because a failing project yields no partial
     * answer (ADR-0009).
     */
    queryDetail(
        targets: DetailTarget[],
    ): Promise<QueryEnvelope<DetailLookupResult> | null>;
    /**
     * Evaluate one kin-set {@link Query} against the same project, with the
     * same `null` and error-arm behaviour as {@link PreviewHandle.queryDetail}.
     * The chrome builds every Query value it asks (ADR-0025, ADR-0043); this is
     * the one place they are evaluated.
     */
    queryKin(query: Query): Promise<QueryEnvelope<QueryResult> | null>;
    /**
     * Every way `xId` and `yId` are related, as terminology-neutral descriptors
     * relative to `xId` (ADR-0028). Same transport policy as
     * {@link PreviewHandle.queryDetail}: `null` when there is nothing to ask,
     * an error arm when the project fails its checks.
     *
     * The hover lens is this call's reason for existing, and it is why the
     * engine runs in the webview at all (ADR-0034) — but the verb is on the
     * handle rather than hidden inside the chrome, because a host with a
     * rendered project can legitimately ask it.
     */
    queryResolve(
        xId: string,
        yId: string,
    ): Promise<QueryEnvelope<ResolveResult> | null>;
    /**
     * The reader's language choice and everything phrased against it. Read it
     * to know which pack the chrome is phrasing in; bind an element to a
     * relationship and it re-phrases whenever the toggle flips (ADR-0033).
     */
    locale: LocaleController;
    dispose(): void;
}

// --- Wire format (VSCode webview ↔ extension) ---------------------------

export interface WireRenderMessage {
    type: "render";
    svg: string;
    /**
     * The project the SVG was rendered from. Carried with every render because
     * the WASM query surface is stateless — the source the webview queries has
     * to be the source the picture came from (ADR-0034). Absent from a host
     * that ships no engine.
     */
    project?: ProjectSnapshot;
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

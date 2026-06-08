import type {
    HostAdapter,
    PreviewHandle,
    WireInboundMessage,
} from "./types.js";

/** Subset of the VSCode webview API the adapter needs. */
interface VsCodeWebviewApi {
    postMessage(message: unknown): void;
}

declare global {
    interface Window {
        acquireVsCodeApi?(): VsCodeWebviewApi;
    }
}

/**
 * VSCode-webview-context {@link HostAdapter}. Outbound `onRevealRequest` posts
 * a `{type: 'revealRequest', target}` message to the extension via
 * `acquireVsCodeApi().postMessage`. Inbound messages dispatched to the
 * supplied {@link PreviewHandle} are wired separately via
 * {@link installVscodeInboundBridge} (split so unit tests can drive the
 * adapter without a live `window.message` channel).
 */
export function createVscodeAdapter(): HostAdapter {
    const vscode =
        typeof window !== "undefined" && typeof window.acquireVsCodeApi === "function"
            ? window.acquireVsCodeApi()
            : null;
    return {
        onRevealRequest(target) {
            vscode?.postMessage({ type: "revealRequest", target });
        },
    };
}

/**
 * Wire `window.message` events into `handle`. Returns a teardown that removes
 * the listener.
 */
export function installVscodeInboundBridge(handle: PreviewHandle): () => void {
    function onMessage(event: MessageEvent): void {
        const msg = event.data as WireInboundMessage | null;
        if (!msg || typeof msg !== "object") {
            return;
        }
        if (msg.type === "render") {
            handle.render(msg.svg);
        } else if (msg.type === "renderError") {
            handle.showErrors(Array.isArray(msg.errors) ? msg.errors : []);
        } else if (msg.type === "highlightEntity") {
            if (msg.id && msg.kind) {
                handle.highlightEntity({ id: msg.id, kind: msg.kind });
            } else {
                handle.highlightEntity(null);
            }
        }
    }
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
}

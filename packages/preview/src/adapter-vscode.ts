import type {
    HostAdapter,
    PreviewHandle,
    WireInboundMessage,
} from "./types.js";
import { isEntityKind } from "./wire-guards.js";

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
 *
 * TRUST BOUNDARY. `event.data` is untrusted input that has crossed the webview
 * `postMessage` channel, so every field is validated for shape before it is
 * handed to `handle` and `kind` is checked against its union (drift fails safe
 * to a cleared highlight rather than a mis-branched one).
 *
 * This bridge intentionally does NOT check `event.origin` / `event.source`.
 * That is sound only in the VSCode single-webview host, where the webview is
 * the sole possible sender. `@kullang/preview` is exported for reuse in other
 * embeddings (iframes, multi-webview shells) where that assumption does not
 * hold — any such host MUST either verify `event.origin`/`event.source` against
 * its own webview, or thread a per-load secret through the messages and check
 * it here, before trusting a payload. Shape validation alone does not
 * authenticate the sender.
 */
export function installVscodeInboundBridge(handle: PreviewHandle): () => void {
    function onMessage(event: MessageEvent): void {
        const msg = event.data as WireInboundMessage | null;
        if (!msg || typeof msg !== "object") {
            return;
        }
        if (msg.type === "render") {
            if (typeof msg.svg === "string") {
                handle.render(msg.svg);
            }
        } else if (msg.type === "renderError") {
            handle.showErrors(Array.isArray(msg.errors) ? msg.errors : []);
        } else if (msg.type === "highlightEntity") {
            // Fail safe: a highlight requires a non-empty string id AND a kind
            // inside the "person" | "marriage" union. Anything else — a null
            // id, a missing kind, or a kind that has drifted off the union —
            // clears the highlight rather than guessing a branch.
            if (typeof msg.id === "string" && msg.id.length > 0 && isEntityKind(msg.kind)) {
                handle.highlightEntity({ id: msg.id, kind: msg.kind });
            } else {
                handle.highlightEntity(null);
            }
        }
    }
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
}

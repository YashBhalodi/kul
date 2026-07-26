import type { LocaleStore } from "./locale-store.js";
import type {
    HostAdapter,
    PreviewHandle,
    WireInboundMessage,
} from "./types.js";
import { isEntityKind, isProjectSnapshot } from "./wire-guards.js";

/** Subset of the VSCode webview API the adapter needs. */
interface VsCodeWebviewApi {
    postMessage(message: unknown): void;
    /** Webview-local persistent state — survives a panel being hidden/restored. */
    getState(): unknown;
    setState(state: unknown): void;
}

declare global {
    interface Window {
        acquireVsCodeApi?(): VsCodeWebviewApi;
    }
}

/**
 * `acquireVsCodeApi` may be called **once** per webview load; a second call
 * throws. Two consumers need the handle now — the reveal channel and the
 * locale store — so it is acquired lazily and cached rather than re-acquired.
 */
let acquired: VsCodeWebviewApi | null | undefined;

function vscodeApi(): VsCodeWebviewApi | null {
    if (acquired === undefined) {
        acquired =
            typeof window !== "undefined" && typeof window.acquireVsCodeApi === "function"
                ? window.acquireVsCodeApi()
                : null;
    }
    return acquired;
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
    return {
        onRevealRequest(target) {
            vscodeApi()?.postMessage({ type: "revealRequest", target });
        },
    };
}

/** Key the locale takes inside the webview's single state object. */
const LOCALE_STATE_KEY = "locale";

/**
 * The VSCode-backed {@link LocaleStore}: the reader's language choice rides
 * `getState` / `setState`, so it survives the panel being hidden and restored
 * (ADR-0033).
 *
 * `HostAdapter` is untouched — this is a second, independent seam, which is
 * what keeps the preview host-agnostic: an embedding with no durable state
 * simply never constructs this and gets the in-memory default.
 *
 * The webview has exactly **one** state object, so the write merges rather
 * than replaces; any other chrome that later persists something keeps it.
 */
export function createVscodeLocaleStore(): LocaleStore {
    return {
        read() {
            const state = vscodeApi()?.getState();
            if (state === null || typeof state !== "object") {
                return null;
            }
            const held = (state as Record<string, unknown>)[LOCALE_STATE_KEY];
            return typeof held === "string" ? held : null;
        },
        write(code) {
            const api = vscodeApi();
            if (!api) {
                return;
            }
            const state = api.getState();
            const base = state !== null && typeof state === "object" ? state : {};
            api.setState({ ...base, [LOCALE_STATE_KEY]: code });
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
                // A malformed project drops to null rather than failing the
                // render: the picture is still worth showing, it just cannot
                // be queried until the next well-formed render arrives.
                handle.render(
                    msg.svg,
                    isProjectSnapshot(msg.project) ? msg.project : null,
                );
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

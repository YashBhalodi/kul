// Runtime validators for the cross-boundary wire format. These live beside the
// wire types (types.ts) because they encode the same contract at runtime that
// the types encode at compile time: any host that receives a `postMessage`
// across the webview trust boundary should narrow the untrusted `unknown`
// payload through these guards before acting on it.

import type { EntityRef, LspPosition, LspRange, RevealTarget } from "./types.js";

/**
 * True iff `value` is a valid {@link EntityRef} kind. Used to fail safe when an
 * inbound `highlightEntity` message carries a `kind` outside the union: an
 * unexpected value must clear the highlight, not silently pick a branch.
 */
export function isEntityKind(value: unknown): value is EntityRef["kind"] {
    return value === "person" || value === "marriage";
}

function isLspPosition(value: unknown): value is LspPosition {
    if (!value || typeof value !== "object") {
        return false;
    }
    const p = value as { line?: unknown; character?: unknown };
    return typeof p.line === "number" && typeof p.character === "number";
}

function isLspRange(value: unknown): value is LspRange {
    if (!value || typeof value !== "object") {
        return false;
    }
    const r = value as { start?: unknown; end?: unknown };
    return isLspPosition(r.start) && isLspPosition(r.end);
}

/**
 * True iff `value` is a well-formed {@link RevealTarget}. The host dispatches a
 * reveal target into async editor code, so a malformed payload must be rejected
 * up front rather than cast blindly and awaited (which risks an unhandled
 * rejection). Every discriminant branch validates its own fields.
 */
export function isRevealTarget(value: unknown): value is RevealTarget {
    if (!value || typeof value !== "object") {
        return false;
    }
    const t = value as { kind?: unknown };
    if (t.kind === "entity") {
        return typeof (value as { id?: unknown }).id === "string";
    }
    if (t.kind === "location") {
        const loc = value as { uri?: unknown; range?: unknown };
        return typeof loc.uri === "string" && isLspRange(loc.range);
    }
    return false;
}

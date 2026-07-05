import type * as vscode from "vscode";
import { describe, expect, it, vi } from "vitest";

import { postToPreview } from "./post-to-preview";

// The `vscode` import in the helper is type-only, so the panel is stubbed
// with just the `webview.postMessage` surface the helper actually touches.
function livePanel(postMessage: (m: unknown) => unknown): vscode.WebviewPanel {
    return { webview: { postMessage } } as unknown as vscode.WebviewPanel;
}

describe("postToPreview", () => {
    it("returns false without logging when the panel is already gone", async () => {
        const logError = vi.fn();
        const posted = await postToPreview(() => undefined, { type: "x" }, logError);
        expect(posted).toBe(false);
        expect(logError).not.toHaveBeenCalled();
    });

    it("forwards the message verbatim and returns true on a live panel", async () => {
        const postMessage = vi.fn().mockResolvedValue(true);
        const logError = vi.fn();
        const message = { type: "render", svg: "<svg/>" };
        const posted = await postToPreview(
            () => livePanel(postMessage),
            message,
            logError,
        );
        expect(posted).toBe(true);
        expect(postMessage).toHaveBeenCalledTimes(1);
        expect(postMessage).toHaveBeenCalledWith(message);
        expect(logError).not.toHaveBeenCalled();
    });

    it("swallows an async rejection (Webview is disposed), logs once, returns false", async () => {
        const postMessage = vi
            .fn()
            .mockRejectedValue(new Error("Webview is disposed"));
        const logError = vi.fn();
        const posted = await postToPreview(
            () => livePanel(postMessage),
            { type: "highlightEntity", id: null },
            logError,
        );
        expect(posted).toBe(false);
        expect(logError).toHaveBeenCalledTimes(1);
        expect(logError.mock.calls[0][0]).toContain("Webview is disposed");
    });

    it("swallows a synchronous throw the same way", async () => {
        const postMessage = vi.fn(() => {
            throw new Error("sync boom");
        });
        const logError = vi.fn();
        const posted = await postToPreview(
            () => livePanel(postMessage),
            { type: "renderError", errors: [] },
            logError,
        );
        expect(posted).toBe(false);
        expect(logError).toHaveBeenCalledTimes(1);
        expect(logError.mock.calls[0][0]).toContain("sync boom");
    });
});

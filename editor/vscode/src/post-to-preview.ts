import type * as vscode from "vscode";

/**
 * Post to the preview webview iff the panel is still alive. The panel can be
 * disposed while an LSP request is in flight (the user closes the tab), and
 * TypeScript's narrowing of the module-level `previewPanel` does not survive
 * an `await` — so every post re-reads the current panel through `getPanel`
 * and tolerates the dispose-after-check window with a try/catch. VSCode throws
 * "Webview is disposed" from `postMessage` on a dead panel; several call sites
 * run fire-and-forget (`void syncSelection(...)`), so an unguarded throw here
 * would surface as an unhandled promise rejection in the extension host.
 *
 * Returns true when the message was posted, false when the panel was gone or
 * disposed mid-post. Never throws.
 */
export async function postToPreview(
    getPanel: () => vscode.WebviewPanel | undefined,
    message: unknown,
    logError: (text: string) => void,
): Promise<boolean> {
    const panel = getPanel();
    if (!panel) {
        return false;
    }
    try {
        await panel.webview.postMessage(message);
        return true;
    } catch (err) {
        // Disposed between the check and the post — benign; log and move on.
        logError(
            `preview postMessage skipped: ${
                err instanceof Error ? err.message : String(err)
            }`,
        );
        return false;
    }
}

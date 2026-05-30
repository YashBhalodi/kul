import * as fs from "node:fs";
import * as path from "node:path";

import * as vscode from "vscode";
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from "vscode-languageclient/node";

import { getNonce, previewHtml } from "./preview-html";

let client: LanguageClient | undefined;
let previewPanel: vscode.WebviewPanel | undefined;
let previewDebounce: NodeJS.Timeout | undefined;
let previewListener: vscode.Disposable | undefined;
let previewUri: vscode.Uri | undefined;

const PREVIEW_DEBOUNCE_MS = 300;

export async function activate(
    context: vscode.ExtensionContext,
): Promise<void> {
    const serverPath = resolveServerPath(context);
    if (!serverPath) {
        await vscode.window.showErrorMessage(
            "Kul: kul-lsp binary not found. Set the `kul.serverPath` setting to the absolute path of your kul-lsp binary, or install the bundled extension version. See the README for details.",
        );
        return;
    }

    const env = { ...process.env };
    if (!env.RUST_LOG) {
        env.RUST_LOG = "kul_lsp=info";
    }

    const serverOptions: ServerOptions = {
        run: {
            command: serverPath,
            args: [],
            options: { env },
        },
        debug: {
            command: serverPath,
            args: [],
            options: { env: { ...env, RUST_LOG: "kul_lsp=debug" } },
        },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "kul" }],
        outputChannelName: "Kul LSP",
    };

    client = new LanguageClient(
        "kul",
        "Kul LSP",
        serverOptions,
        clientOptions,
    );

    try {
        await client.start();
    } catch (err) {
        const message =
            err instanceof Error ? err.message : String(err);
        await vscode.window.showErrorMessage(
            `Kul LSP failed to start: ${message}. Check the "Kul LSP" output channel for details.`,
        );
    }

    context.subscriptions.push(
        vscode.commands.registerCommand("kul.export.json", () =>
            runExport("json"),
        ),
        vscode.commands.registerCommand("kul.export.cytoscape", () =>
            runExport("cytoscape"),
        ),
        vscode.commands.registerCommand("kul.preview.show", () =>
            showPreview(context),
        ),
    );
}

type ExportFormat = "json" | "cytoscape";

interface ExportEnvelope {
    ok: boolean;
    diagnostics?: { code: string }[];
}

async function runExport(format: ExportFormat): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "kul") {
        await vscode.window.showWarningMessage(
            "Kul export only works on .kul files.",
        );
        return;
    }
    if (!client) {
        await vscode.window.showWarningMessage(
            "Kul LSP is not running — open a `.kul` file to start the server.",
        );
        return;
    }

    let envelope: ExportEnvelope;
    try {
        envelope = await client.sendRequest<ExportEnvelope>("kul/export", {
            uri: editor.document.uri.toString(),
            format,
            withPositions: false,
        });
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        await vscode.window.showErrorMessage(
            `Kul export failed: ${message}`,
        );
        return;
    }

    if (!envelope.ok) {
        const count = envelope.diagnostics?.length ?? 0;
        await vscode.window.showWarningMessage(
            `Kul export failed: ${count} issue${count === 1 ? "" : "s"} — fix the errors in the Problems panel and try again.`,
        );
        return;
    }

    const defaultName = defaultExportFilename(editor.document.uri, format);
    const target = await vscode.window.showSaveDialog({
        defaultUri: defaultName,
        filters: { JSON: ["json"] },
        saveLabel: "Export",
    });
    if (!target) {
        return;
    }
    const body = JSON.stringify(envelope, null, 2);
    try {
        await vscode.workspace.fs.writeFile(target, Buffer.from(body, "utf8"));
        await vscode.window.showInformationMessage(
            `Kul: exported ${path.basename(target.fsPath)}`,
        );
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        await vscode.window.showErrorMessage(
            `Kul: could not write export file: ${message}`,
        );
    }
}

function defaultExportFilename(
    source: vscode.Uri,
    format: ExportFormat,
): vscode.Uri {
    const dir = path.dirname(source.fsPath);
    const stem = path.basename(source.fsPath, path.extname(source.fsPath));
    const suffix = format === "cytoscape" ? ".cytoscape.json" : ".json";
    return vscode.Uri.file(path.join(dir, `${stem}${suffix}`));
}

interface RenderResponse {
    ok: boolean;
    svg?: string;
    diagnostics?: { code: string }[];
}

/** LSP-protocol position (0-based line/character). */
interface LspPosition {
    line: number;
    character: number;
}

/** LSP-protocol location returned by `kul/locate`. */
interface LspLocation {
    uri: string;
    range: { start: LspPosition; end: LspPosition };
}

interface LocateResponse {
    location: LspLocation | null;
}

/**
 * Message posted by the webview when a person card or marriage bar is
 * clicked (issue #135). The id is a project-wide entity id; the uri is
 * carried by `previewUri`, so the message stays minimal.
 */
interface RevealSourceMessage {
    type: "revealSource";
    id: string;
}

/**
 * Resolve a clicked entity id to its declaration via `kul/locate` and
 * reveal it in an editor. A null location (stale id, no live
 * declaration) is a silent no-op — no dialog, debug log only.
 */
async function revealSource(id: string): Promise<void> {
    if (!client || !previewUri) {
        return;
    }
    let response: LocateResponse;
    try {
        response = await client.sendRequest<LocateResponse>("kul/locate", {
            uri: previewUri.toString(),
            id,
        });
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        client.outputChannel.appendLine(
            `kul/locate failed for id "${id}": ${message}`,
        );
        return;
    }
    const location = response.location;
    if (!location) {
        client.outputChannel.appendLine(
            `kul/locate: no declaration for id "${id}"`,
        );
        return;
    }
    const targetUri = vscode.Uri.parse(location.uri);
    const selection = new vscode.Range(
        location.range.start.line,
        location.range.start.character,
        location.range.end.line,
        location.range.end.character,
    );
    await vscode.window.showTextDocument(targetUri, { selection });
}

async function showPreview(
    context: vscode.ExtensionContext,
): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "kul") {
        await vscode.window.showWarningMessage(
            "Kul preview only works on .kul files.",
        );
        return;
    }
    if (!client) {
        await vscode.window.showWarningMessage(
            "Kul LSP is not running — open a `.kul` file to start the server.",
        );
        return;
    }

    previewUri = editor.document.uri;

    if (previewPanel) {
        previewPanel.reveal(vscode.ViewColumn.Beside, /* preserveFocus */ true);
        void refreshPreview(previewUri);
        return;
    }

    previewPanel = vscode.window.createWebviewPanel(
        "kul.preview",
        "Kul: Preview",
        { viewColumn: vscode.ViewColumn.Beside, preserveFocus: true },
        {
            enableScripts: true,
            localResourceRoots: [
                vscode.Uri.joinPath(context.extensionUri, "media"),
            ],
        },
    );

    const themeUri = previewPanel.webview.asWebviewUri(
        vscode.Uri.joinPath(context.extensionUri, "media", "preview-themes.css"),
    );
    const cssUri = previewPanel.webview.asWebviewUri(
        vscode.Uri.joinPath(context.extensionUri, "media", "preview.css"),
    );
    const scriptUri = previewPanel.webview.asWebviewUri(
        vscode.Uri.joinPath(
            context.extensionUri,
            "media",
            "vendor",
            "dist",
            "svg-pan-zoom.min.js",
        ),
    );
    previewPanel.webview.html = previewHtml(
        themeUri.toString(),
        cssUri.toString(),
        scriptUri.toString(),
        previewPanel.webview.cspSource,
        getNonce(),
    );

    // Click-to-source (issue #135): the webview posts a `revealSource`
    // message when a card or marriage bar is clicked.
    previewPanel.webview.onDidReceiveMessage((message: unknown) => {
        if (
            message &&
            typeof message === "object" &&
            (message as { type?: unknown }).type === "revealSource"
        ) {
            const { id } = message as RevealSourceMessage;
            if (typeof id === "string" && id.length > 0) {
                void revealSource(id);
            }
        }
    });

    previewPanel.onDidDispose(() => {
        previewPanel = undefined;
        previewListener?.dispose();
        previewListener = undefined;
        if (previewDebounce) {
            clearTimeout(previewDebounce);
            previewDebounce = undefined;
        }
    });

    // Debounced re-render on document changes in the project that
    // owns the active URI.
    previewListener = vscode.workspace.onDidChangeTextDocument((event) => {
        if (!previewUri) {
            return;
        }
        if (event.document.languageId !== "kul") {
            return;
        }
        // Re-render if the changed document is the previewed URI or
        // a sibling .kul in the same directory (project-wide per
        // ADR-0015).
        const previewDir = path.dirname(previewUri.fsPath);
        const changedDir = path.dirname(event.document.uri.fsPath);
        if (changedDir !== previewDir) {
            return;
        }
        if (previewDebounce) {
            clearTimeout(previewDebounce);
        }
        previewDebounce = setTimeout(() => {
            previewDebounce = undefined;
            if (previewUri) {
                void refreshPreview(previewUri);
            }
        }, PREVIEW_DEBOUNCE_MS);
    });

    // Initial render.
    await refreshPreview(previewUri);
}

async function refreshPreview(uri: vscode.Uri): Promise<void> {
    if (!previewPanel || !client) {
        return;
    }
    let response: RenderResponse;
    try {
        response = await client.sendRequest<RenderResponse>("kul/render", {
            uri: uri.toString(),
        });
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        await previewPanel.webview.postMessage({
            type: "renderError",
            message: `Kul render failed: ${message}`,
            diagnosticCount: 0,
        });
        return;
    }
    if (response.ok && response.svg) {
        await previewPanel.webview.postMessage({
            type: "render",
            svg: response.svg,
        });
    } else {
        const count = response.diagnostics?.length ?? 0;
        await previewPanel.webview.postMessage({
            type: "renderError",
            message: `Document has ${count} issue${count === 1 ? "" : "s"} — see the Problems panel.`,
            diagnosticCount: count,
        });
    }
}

export async function deactivate(): Promise<void> {
    previewListener?.dispose();
    previewPanel?.dispose();
    if (previewDebounce) {
        clearTimeout(previewDebounce);
    }
    await client?.stop();
    client = undefined;
}

function resolveServerPath(
    context: vscode.ExtensionContext,
): string | undefined {
    const cfg = vscode.workspace.getConfiguration("kul");
    const userPath = cfg.get<string>("serverPath");
    if (userPath && userPath.trim() !== "") {
        const expanded = expandHome(userPath.trim());
        if (existsAndExecutable(expanded)) {
            return expanded;
        }
        void vscode.window.showWarningMessage(
            `Kul: kul.serverPath is set to "${userPath}" but the file does not exist or is not executable. Falling back to the bundled binary if present.`,
        );
    }

    const bundled = bundledServerPath(context);
    if (bundled && existsAndExecutable(bundled)) {
        return bundled;
    }
    return undefined;
}

function bundledServerPath(
    context: vscode.ExtensionContext,
): string | undefined {
    const subdir = platformSubdir(process.platform, process.arch);
    if (!subdir) {
        return undefined;
    }
    const exe = process.platform === "win32" ? "kul-lsp.exe" : "kul-lsp";
    const p = path.join(context.extensionPath, "server", subdir, exe);
    // vsce's zip layer drops the execute bit, so a marketplace-installed
    // binary lands as -rw-r--r--. Restore +x on Unix before the
    // executable check so the bundled LSP can actually launch.
    if (process.platform !== "win32") {
        try {
            fs.chmodSync(p, 0o755);
        } catch {
            // File may not exist (Fix A means only one platform's binary
            // is bundled per vsix); fall through to the existence check.
        }
    }
    return p;
}

function platformSubdir(
    platform: NodeJS.Platform,
    arch: string,
): string | undefined {
    if (platform === "darwin" && arch === "x64") return "darwin-x64";
    if (platform === "darwin" && arch === "arm64") return "darwin-arm64";
    if (platform === "linux" && arch === "x64") return "linux-x64";
    if (platform === "win32" && arch === "x64") return "win32-x64";
    return undefined;
}

function existsAndExecutable(p: string): boolean {
    try {
        const st = fs.statSync(p);
        if (!st.isFile()) {
            return false;
        }
        if (process.platform !== "win32") {
            // X bit set for owner, group, or other.
            return (st.mode & 0o111) !== 0;
        }
        return true;
    } catch {
        return false;
    }
}

function expandHome(p: string): string {
    if (p.startsWith("~/") || p === "~") {
        const home = process.env.HOME ?? process.env.USERPROFILE ?? "";
        return path.join(home, p.slice(1));
    }
    return p;
}

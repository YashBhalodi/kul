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
let selectionListener: vscode.Disposable | undefined;
let selectionDebounce: NodeJS.Timeout | undefined;
let previewUri: vscode.Uri | undefined;

const PREVIEW_DEBOUNCE_MS = 300;
// Cursor movement fires far more often than edits; tighter debounce keeps
// the highlight live without flooding the server.
const SELECTION_DEBOUNCE_MS = 50;

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
        vscode.commands.registerCommand("kul.export.svg", () =>
            runExportSvg(),
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

// Default SVG filename: `<project-dir-basename>.svg` in the project
// directory (the directory containing the active `.kul` file, per
// ADR-0015 — rendered output is project-wide). Falls back to `tree.svg`
// if the basename is empty (e.g. an unsaved file at the filesystem root).
function defaultSvgExportFilename(source: vscode.Uri): vscode.Uri {
    const dir = path.dirname(source.fsPath);
    const projectName = path.basename(dir);
    const stem = projectName.length > 0 ? projectName : "tree";
    return vscode.Uri.file(path.join(dir, `${stem}.svg`));
}

async function runExportSvg(): Promise<void> {
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

    let response: RenderResponse;
    try {
        response = await client.sendRequest<RenderResponse>("kul/exportSvg", {
            uri: editor.document.uri.toString(),
        });
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        await vscode.window.showErrorMessage(
            `Kul export failed: ${message}`,
        );
        return;
    }

    if (!response.ok || !response.svg) {
        const count = response.diagnostics?.length ?? 0;
        await vscode.window.showWarningMessage(
            `Kul export failed: ${count} issue${count === 1 ? "" : "s"} — fix the errors in the Problems panel and try again.`,
        );
        return;
    }

    const defaultName = defaultSvgExportFilename(editor.document.uri);
    const target = await vscode.window.showSaveDialog({
        defaultUri: defaultName,
        filters: { SVG: ["svg"] },
        saveLabel: "Export",
    });
    if (!target) {
        return;
    }
    try {
        await vscode.workspace.fs.writeFile(
            target,
            Buffer.from(response.svg, "utf8"),
        );
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

interface LspPosition {
    line: number;
    character: number;
}

interface LspRange {
    start: LspPosition;
    end: LspPosition;
}

interface RenderDiagnostic {
    code: string;
    severity: string;
    message: string;
    /** Present only for anchored diagnostics. */
    uri?: string;
    /** LSP range in the primary file; present only for anchored diagnostics. */
    range?: LspRange;
}

interface RenderResponse {
    ok: boolean;
    svg?: string;
    diagnostics?: RenderDiagnostic[];
}

interface LspLocation {
    uri: string;
    range: LspRange;
}

interface LocateResponse {
    location: LspLocation | null;
}

interface EntityAtResponse {
    entity: { id: string; kind: "person" | "marriage" } | null;
}

interface RevealSourceMessage {
    type: "revealSource";
    /** Entity id (kul-card / marriage bar click). */
    id?: string;
    /** Direct location (error popover click — #203). */
    uri?: string;
    range?: LspRange;
}

// Reveal an LSP URI + range directly. Used by the error popover (#203),
// where the diagnostic already carries its own location and the entity-id
// round-trip would be a no-op (errors have no entity to look up).
async function revealLocation(uri: string, range: LspRange): Promise<void> {
    const target = vscode.Uri.parse(uri);
    const selection = new vscode.Range(
        range.start.line,
        range.start.character,
        range.end.line,
        range.end.character,
    );
    await vscode.window.showTextDocument(target, { selection });
}

// A null location (stale id, no live declaration) is a silent no-op.
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

// Resolve a source cursor position to its entity via `kul/entityAt` and tell
// the webview to highlight the matching card/bar. Resolution is project-wide
// (ADR-0015); a cursor off any entity resolves to null, posted as a clear.
async function syncSelection(
    uri: vscode.Uri,
    position: vscode.Position,
): Promise<void> {
    if (!client || !previewPanel) {
        return;
    }
    let response: EntityAtResponse;
    try {
        response = await client.sendRequest<EntityAtResponse>("kul/entityAt", {
            uri: uri.toString(),
            position: { line: position.line, character: position.character },
        });
    } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        client.outputChannel.appendLine(
            `kul/entityAt failed: ${message}`,
        );
        await previewPanel.webview.postMessage({
            type: "highlightEntity",
            id: null,
        });
        return;
    }
    const entity = response.entity;
    await previewPanel.webview.postMessage({
        type: "highlightEntity",
        id: entity ? entity.id : null,
        kind: entity ? entity.kind : undefined,
    });
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
        previewPanel.reveal(vscode.ViewColumn.Beside, true);
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

    previewPanel.webview.onDidReceiveMessage((message: unknown) => {
        if (
            message &&
            typeof message === "object" &&
            (message as { type?: unknown }).type === "revealSource"
        ) {
            const { id, uri, range } = message as RevealSourceMessage;
            if (typeof id === "string" && id.length > 0) {
                void revealSource(id);
            } else if (
                typeof uri === "string" &&
                uri.length > 0 &&
                range &&
                range.start &&
                range.end
            ) {
                void revealLocation(uri, range);
            }
        }
    });

    // VSCode destroys a webview's DOM/JS context when its tab moves to the
    // background (another tab in the same group takes focus). On restore the
    // HTML shell reloads but our bootstrap waits for a `render` message
    // before populating #root and #kul-controls — so without a re-render the
    // panel stays blank until the next save. Push a fresh render on every
    // hidden→visible transition so the preview self-heals.
    let wasVisible = true;
    previewPanel.onDidChangeViewState((event) => {
        const visible = event.webviewPanel.visible;
        if (visible && !wasVisible && previewUri) {
            void refreshPreview(previewUri);
        }
        wasVisible = visible;
    });

    previewPanel.onDidDispose(() => {
        previewPanel = undefined;
        previewListener?.dispose();
        previewListener = undefined;
        selectionListener?.dispose();
        selectionListener = undefined;
        if (previewDebounce) {
            clearTimeout(previewDebounce);
            previewDebounce = undefined;
        }
        if (selectionDebounce) {
            clearTimeout(selectionDebounce);
            selectionDebounce = undefined;
        }
    });

    // Re-render on changes to the previewed URI or sibling .kul files in the
    // same directory (project-wide per ADR-0015).
    previewListener = vscode.workspace.onDidChangeTextDocument((event) => {
        if (!previewUri) {
            return;
        }
        if (event.document.languageId !== "kul") {
            return;
        }
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

    // Only the primary cursor (selections[0]) drives the highlight.
    selectionListener = vscode.window.onDidChangeTextEditorSelection((event) => {
        if (!previewUri) {
            return;
        }
        if (event.textEditor.document.languageId !== "kul") {
            return;
        }
        const previewDir = path.dirname(previewUri.fsPath);
        const eventDir = path.dirname(event.textEditor.document.uri.fsPath);
        if (eventDir !== previewDir) {
            return;
        }
        const editorUri = event.textEditor.document.uri;
        const position = event.selections[0].active;
        if (selectionDebounce) {
            clearTimeout(selectionDebounce);
        }
        selectionDebounce = setTimeout(() => {
            selectionDebounce = undefined;
            void syncSelection(editorUri, position);
        }, SELECTION_DEBOUNCE_MS);
    });

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
        // Transport failure — surface as a single synthetic error row.
        await previewPanel.webview.postMessage({
            type: "renderError",
            errors: [{ message: `Kul render failed: ${message}` }],
        });
        return;
    }
    if (response.ok && response.svg) {
        await previewPanel.webview.postMessage({
            type: "render",
            svg: response.svg,
        });
        // A live-edit re-render rebuilds the SVG and drops the prior
        // `.kul-selected`, so re-resolve the cursor and post it after the swap.
        const editor = vscode.window.activeTextEditor;
        if (editor && editor.document.languageId === "kul") {
            const editorDir = path.dirname(editor.document.uri.fsPath);
            const previewDir = previewUri && path.dirname(previewUri.fsPath);
            if (editorDir === previewDir) {
                void syncSelection(
                    editor.document.uri,
                    editor.selection.active,
                );
            }
        }
    } else {
        // Forward error-severity rows only (#203). The Rust side already
        // filters to severity === "error", but re-filter here so a future
        // payload change can't accidentally surface warnings in this UI.
        const errors = (response.diagnostics ?? [])
            .filter((d) => d.severity === "error")
            .map((d) => ({
                message: d.message,
                code: d.code,
                uri: d.uri,
                range: d.range,
            }));
        await previewPanel.webview.postMessage({
            type: "renderError",
            errors,
        });
    }
}

export async function deactivate(): Promise<void> {
    previewListener?.dispose();
    selectionListener?.dispose();
    previewPanel?.dispose();
    if (previewDebounce) {
        clearTimeout(previewDebounce);
    }
    if (selectionDebounce) {
        clearTimeout(selectionDebounce);
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
    // vsce's zip layer drops the execute bit; restore +x on Unix so the
    // bundled LSP can actually launch.
    if (process.platform !== "win32") {
        try {
            fs.chmodSync(p, 0o755);
        } catch {
            // File may not exist; fall through to the existence check.
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

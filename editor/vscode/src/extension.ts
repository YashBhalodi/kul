import * as fs from "node:fs";
import * as path from "node:path";

import * as vscode from "vscode";
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

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

export async function deactivate(): Promise<void> {
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
    return path.join(context.extensionPath, "server", subdir, exe);
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

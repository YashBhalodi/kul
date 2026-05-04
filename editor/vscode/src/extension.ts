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
            "Kula: kula-lsp binary not found. Set the `kula.serverPath` setting to the absolute path of your kula-lsp binary, or install the bundled extension version. See the README for details.",
        );
        return;
    }

    const env = { ...process.env };
    if (!env.RUST_LOG) {
        env.RUST_LOG = "kula_lsp=info";
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
            options: { env: { ...env, RUST_LOG: "kula_lsp=debug" } },
        },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "kula" }],
        outputChannelName: "Kula LSP",
    };

    client = new LanguageClient(
        "kula",
        "Kula LSP",
        serverOptions,
        clientOptions,
    );

    try {
        await client.start();
    } catch (err) {
        const message =
            err instanceof Error ? err.message : String(err);
        await vscode.window.showErrorMessage(
            `Kula LSP failed to start: ${message}. Check the "Kula LSP" output channel for details.`,
        );
    }
}

export async function deactivate(): Promise<void> {
    await client?.stop();
    client = undefined;
}

function resolveServerPath(
    context: vscode.ExtensionContext,
): string | undefined {
    const cfg = vscode.workspace.getConfiguration("kula");
    const userPath = cfg.get<string>("serverPath");
    if (userPath && userPath.trim() !== "") {
        const expanded = expandHome(userPath.trim());
        if (existsAndExecutable(expanded)) {
            return expanded;
        }
        void vscode.window.showWarningMessage(
            `Kula: kula.serverPath is set to "${userPath}" but the file does not exist or is not executable. Falling back to the bundled binary if present.`,
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
    const exe = process.platform === "win32" ? "kula-lsp.exe" : "kula-lsp";
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

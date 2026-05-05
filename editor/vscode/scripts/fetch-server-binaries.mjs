#!/usr/bin/env node
/**
 * Fetch matched-version `kula-lsp` binaries from the kulalang GitHub
 * Releases and stage them under `editor/vscode/server/<platform>/` so
 * `vsce package` can bundle them. Platform layout matches what the
 * extension's `bundledServerPath` lookup expects:
 *
 *   editor/vscode/server/linux-x64/kula-lsp
 *   editor/vscode/server/darwin-x64/kula-lsp
 *   editor/vscode/server/darwin-arm64/kula-lsp
 *   editor/vscode/server/win32-x64/kula-lsp.exe
 *
 * Usage (from `editor/vscode/`):
 *
 *   node scripts/fetch-server-binaries.mjs            # uses LSP_VERSION env or 0.1.0
 *   LSP_VERSION=0.2.0 node scripts/fetch-server-binaries.mjs
 *
 * Or via the npm script:
 *
 *   npm run fetch-server
 *   npm run package:bundled                           # fetch + package together
 *
 * The script is opt-in. The default `npm run package` produces an
 * unbundled .vsix (good for local-dev install + `kula.serverPath`).
 */

import { execFileSync } from "node:child_process";
import {
    chmodSync,
    copyFileSync,
    existsSync,
    mkdirSync,
    mkdtempSync,
    readFileSync,
    rmSync,
    writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const TARGETS = [
    {
        rustTarget: "x86_64-unknown-linux-gnu",
        platformDir: "linux-x64",
        archive: "tar.gz",
        binaryName: "kula-lsp",
    },
    {
        rustTarget: "x86_64-apple-darwin",
        platformDir: "darwin-x64",
        archive: "tar.gz",
        binaryName: "kula-lsp",
    },
    {
        rustTarget: "aarch64-apple-darwin",
        platformDir: "darwin-arm64",
        archive: "tar.gz",
        binaryName: "kula-lsp",
    },
    {
        rustTarget: "x86_64-pc-windows-msvc",
        platformDir: "win32-x64",
        archive: "zip",
        binaryName: "kula-lsp.exe",
    },
];

const REPO = "YashBhalodi/kulalang";

async function main() {
    const here = dirname(fileURLToPath(import.meta.url));
    const extensionRoot = resolve(here, "..");
    const serverRoot = join(extensionRoot, "server");

    const version = process.env.LSP_VERSION ?? readDefaultVersion(extensionRoot);
    if (!version) {
        die("could not determine LSP version. Set LSP_VERSION=<x.y.z> or ensure package.json's version field is set.");
    }

    const tag = `kula-lsp-v${version}`;
    console.log(`fetching ${REPO} release ${tag}`);

    const tmp = mkdtempSync(join(tmpdir(), "kula-lsp-fetch-"));
    try {
        for (const target of TARGETS) {
            await fetchAndStage(tag, target, tmp, serverRoot);
        }
        console.log(`✓ all ${TARGETS.length} platforms staged under ${serverRoot}`);
    } finally {
        rmSync(tmp, { recursive: true, force: true });
    }
}

function readDefaultVersion(extensionRoot) {
    // Default to the extension's own version. For coordinated releases
    // (v0.1.0 was the first), the LSP and extension share the same version
    // number; if they later drift, set LSP_VERSION explicitly.
    try {
        const pkg = JSON.parse(readFileSync(join(extensionRoot, "package.json"), "utf8"));
        return pkg.version;
    } catch {
        return undefined;
    }
}

async function fetchAndStage(tag, target, tmp, serverRoot) {
    const archiveName = `kula-lsp-${target.rustTarget}.${target.archive}`;
    const url = `https://github.com/${REPO}/releases/download/${tag}/${archiveName}`;
    const archivePath = join(tmp, archiveName);

    console.log(`  ↓ ${target.platformDir}: downloading ${archiveName}`);
    const res = await fetch(url);
    if (!res.ok) {
        die(`download failed (${res.status} ${res.statusText}): ${url}`);
    }
    const buf = Buffer.from(await res.arrayBuffer());
    writeFileSync(archivePath, buf);

    const extractDir = join(tmp, target.platformDir);
    mkdirSync(extractDir, { recursive: true });

    if (target.archive === "tar.gz") {
        execFileSync("tar", ["-xzf", archivePath, "-C", extractDir]);
    } else if (target.archive === "zip") {
        // Modern Windows (and macOS / most Linux) ship `tar` with zip
        // support. Fall back to `unzip` on systems where tar can't.
        try {
            execFileSync("tar", ["-xf", archivePath, "-C", extractDir]);
        } catch {
            execFileSync("unzip", ["-q", archivePath, "-d", extractDir]);
        }
    }

    // Archive contents are at <staging>/kula-lsp-<target>/kula-lsp[.exe].
    const stagingDir = `kula-lsp-${target.rustTarget}`;
    const sourceBinary = join(extractDir, stagingDir, target.binaryName);
    if (!existsSync(sourceBinary)) {
        die(`expected binary at ${sourceBinary} after extracting ${archiveName}`);
    }

    const destDir = join(serverRoot, target.platformDir);
    mkdirSync(destDir, { recursive: true });
    const destBinary = join(destDir, target.binaryName);
    copyFileSync(sourceBinary, destBinary);
    if (process.platform !== "win32") {
        chmodSync(destBinary, 0o755);
    }
    console.log(`    ✓ staged ${destBinary}`);
}

function die(message) {
    console.error(`fetch-server-binaries: ${message}`);
    process.exit(1);
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});

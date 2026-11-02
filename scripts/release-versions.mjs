#!/usr/bin/env node
// The release's version lockstep. The workspace `Cargo.toml`'s
// `[workspace.package] version`, the web app's `apps/web/package.json`
// `version` and, when one is given, the release tag `v<version>` must agree.
//
//   node scripts/release-versions.mjs [--root <dir>] [--tag v<major>.<minor>.<patch>]
//
// Prints the agreed version on stdout. On a disagreement it prints one error
// line per mismatch on stderr and exits 1. `.github/workflows/release.yml`'s
// `verify` job runs it; `scripts/release-versions.test.mjs` tests it.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { parseArgs } from "node:util";

const TAG = /^v(\d+\.\d+\.\d+)$/;
const WEB_PACKAGE = "apps/web/package.json";

/** The `version` key of the `[workspace.package]` table, or undefined. */
export function cargoWorkspaceVersion(toml) {
    let inTable = false;
    for (const line of toml.split(/\r?\n/)) {
        const header = line.match(/^\s*\[([^\]]+)\]\s*(#.*)?$/);
        if (header) {
            inTable = header[1].trim() === "workspace.package";
            continue;
        }
        const version = inTable && line.match(/^\s*version\s*=\s*"([^"]*)"/);
        if (version) return version[1];
    }
    return undefined;
}

/**
 * Check the lockstep under `root`. Returns `{ version, notes }` when every
 * source agrees; throws an Error whose `problems` lists each mismatch.
 */
export function checkVersions({ root, tag }) {
    const problems = [];
    const notes = [];

    const cargo = cargoWorkspaceVersion(readFileSync(join(root, "Cargo.toml"), "utf8"));
    if (!cargo) problems.push("Cargo.toml has no [workspace.package] version");

    const webPath = join(root, WEB_PACKAGE);
    if (existsSync(webPath)) {
        const web = JSON.parse(readFileSync(webPath, "utf8")).version;
        if (web !== cargo) {
            problems.push(`${WEB_PACKAGE} version ${web} != Cargo.toml version ${cargo}`);
        }
    } else {
        notes.push(`${WEB_PACKAGE} does not exist yet, so only Cargo.toml and the tag are compared`);
    }

    if (tag !== undefined) {
        const match = tag.match(TAG);
        if (!match) {
            problems.push(`tag ${tag} is not v<major>.<minor>.<patch>`);
        } else if (match[1] !== cargo) {
            problems.push(`tag ${tag} != Cargo.toml version ${cargo}`);
        }
    }

    if (problems.length > 0) {
        const error = new Error(problems.join("\n"));
        error.problems = problems;
        throw error;
    }
    return { version: cargo, notes };
}

function main() {
    const { values } = parseArgs({
        options: { root: { type: "string", default: "." }, tag: { type: "string" } },
    });
    try {
        const { version, notes } = checkVersions(values);
        for (const note of notes) console.error(`note: ${note}`);
        console.log(version);
    } catch (error) {
        for (const problem of error.problems ?? [error.message]) {
            console.error(`::error::${problem}`);
        }
        process.exit(1);
    }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main();

// Tests for scripts/release-versions.mjs over fixture trees in a temp dir.
// Run: node --test scripts/release-versions.test.mjs

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { after, describe, test } from "node:test";
import { fileURLToPath } from "node:url";

import { cargoWorkspaceVersion, checkVersions } from "./release-versions.mjs";

const SCRIPT = fileURLToPath(new URL("./release-versions.mjs", import.meta.url));
const REPO = dirname(dirname(SCRIPT));
const scratch = mkdtempSync(join(tmpdir(), "release-versions-"));
after(() => rmSync(scratch, { recursive: true, force: true }));

let fixtures = 0;
/** A repo root holding a workspace Cargo.toml and, unless `web` is null, apps/web/package.json. */
function fixture({ cargo = "1.2.3", web = cargo } = {}) {
    const root = join(scratch, String(fixtures++));
    mkdirSync(root);
    writeFileSync(
        join(root, "Cargo.toml"),
        [
            "[workspace]",
            'members = ["crates/a"]',
            "",
            "[workspace.package]",
            `version = "${cargo}"`,
            'edition = "2024"',
            "",
            "[workspace.dependencies]",
            'a = { path = "crates/a", version = "9.9.9" }',
            "",
        ].join("\n"),
    );
    if (web !== null) {
        mkdirSync(join(root, "apps/web"), { recursive: true });
        writeFileSync(join(root, "apps/web/package.json"), JSON.stringify({ name: "web", version: web }));
    }
    return root;
}

function problemsOf(options) {
    try {
        checkVersions(options);
    } catch (error) {
        return error.problems;
    }
    assert.fail("expected the check to fail");
}

describe("cargoWorkspaceVersion", () => {
    test("reads the version of the [workspace.package] table only", () => {
        const toml = [
            "[package]",
            'version = "0.0.1"',
            "[workspace.package] # shared",
            'version = "4.5.6"',
            "[dependencies]",
            'version = "7.7.7"',
        ].join("\n");
        assert.equal(cargoWorkspaceVersion(toml), "4.5.6");
    });

    test("is undefined without the table", () => {
        assert.equal(cargoWorkspaceVersion('[package]\nversion = "1.0.0"\n'), undefined);
    });
});

describe("checkVersions", () => {
    test("agreeing Cargo.toml, web package and tag give the version", () => {
        assert.deepEqual(checkVersions({ root: fixture(), tag: "v1.2.3" }), { version: "1.2.3", notes: [] });
    });

    test("without a tag, the two files are compared", () => {
        assert.equal(checkVersions({ root: fixture() }).version, "1.2.3");
    });

    test("a web package version that differs from Cargo.toml fails", () => {
        assert.deepEqual(problemsOf({ root: fixture({ web: "1.2.4" }), tag: "v1.2.3" }), [
            "apps/web/package.json version 1.2.4 != Cargo.toml version 1.2.3",
        ]);
    });

    test("a tag that differs from Cargo.toml fails", () => {
        assert.deepEqual(problemsOf({ root: fixture(), tag: "v1.3.0" }), [
            "tag v1.3.0 != Cargo.toml version 1.2.3",
        ]);
    });

    test("every mismatch is reported at once", () => {
        assert.equal(problemsOf({ root: fixture({ web: "2.0.0" }), tag: "v3.0.0" }).length, 2);
    });

    test("a tag that is not v<major>.<minor>.<patch> fails", () => {
        for (const tag of ["1.2.3", "v1.2", "v1.2.3-rc.1"]) {
            assert.deepEqual(problemsOf({ root: fixture(), tag }), [
                `tag ${tag} is not v<major>.<minor>.<patch>`,
            ]);
        }
    });

    test("a missing web package compares Cargo.toml with the tag and says so", () => {
        const result = checkVersions({ root: fixture({ web: null }), tag: "v1.2.3" });
        assert.equal(result.version, "1.2.3");
        assert.equal(result.notes.length, 1);
        assert.deepEqual(problemsOf({ root: fixture({ web: null }), tag: "v1.2.4" }), [
            "tag v1.2.4 != Cargo.toml version 1.2.3",
        ]);
    });

    test("this repository is in lockstep", () => {
        assert.match(checkVersions({ root: REPO }).version, /^\d+\.\d+\.\d+$/);
    });
});

describe("command line", () => {
    const run = (...args) => spawnSync(process.execPath, [SCRIPT, ...args], { encoding: "utf8" });

    test("prints the version and exits 0 on agreement", () => {
        const result = run("--root", fixture(), "--tag", "v1.2.3");
        assert.equal(result.status, 0, result.stderr);
        assert.equal(result.stdout, "1.2.3\n");
    });

    test("prints one error annotation per mismatch and exits 1", () => {
        const result = run("--root", fixture({ web: "2.0.0" }), "--tag", "v3.0.0");
        assert.equal(result.status, 1);
        assert.equal(result.stdout, "");
        assert.deepEqual(result.stderr.trim().split("\n"), [
            "::error::apps/web/package.json version 2.0.0 != Cargo.toml version 1.2.3",
            "::error::tag v3.0.0 != Cargo.toml version 1.2.3",
        ]);
    });
});

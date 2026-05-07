#!/usr/bin/env node
// Set `kul.serverPath` in user-level VSCode and Cursor settings.json files.
//
// VSCode's settings.json is JSONC (allows comments and trailing commas), so a
// straight JSON.parse + JSON.stringify round-trip would destroy comments. We
// instead do a regex-based single-key edit on the original text and write
// atomically (temp + rename) so a partial write can never corrupt the file.
//
// Usage: node update-settings.mjs <kul-lsp-absolute-path>

import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

const target = process.argv[2];
if (!target) {
    console.error("usage: update-settings.mjs <kul-lsp-absolute-path>");
    process.exit(2);
}

const SETTINGS_KEY = "kul.serverPath";

const home = os.homedir();
const candidates = (() => {
    if (process.platform === "darwin") {
        const base = path.join(home, "Library", "Application Support");
        return [
            { name: "Cursor", file: path.join(base, "Cursor", "User", "settings.json") },
            { name: "VSCode", file: path.join(base, "Code", "User", "settings.json") },
        ];
    }
    if (process.platform === "win32") {
        const base = process.env.APPDATA || path.join(home, "AppData", "Roaming");
        return [
            { name: "Cursor", file: path.join(base, "Cursor", "User", "settings.json") },
            { name: "VSCode", file: path.join(base, "Code", "User", "settings.json") },
        ];
    }
    // linux + others
    const base = process.env.XDG_CONFIG_HOME || path.join(home, ".config");
    return [
        { name: "Cursor", file: path.join(base, "Cursor", "User", "settings.json") },
        { name: "VSCode", file: path.join(base, "Code", "User", "settings.json") },
    ];
})();

let touched = 0;
for (const { name, file } of candidates) {
    if (!fs.existsSync(file)) continue;
    try {
        const before = fs.readFileSync(file, "utf8");
        const after = setStringKey(before, SETTINGS_KEY, target);
        if (after === before) {
            console.log(`${name}: ${SETTINGS_KEY} already up to date`);
            touched++;
            continue;
        }
        const tmp = `${file}.tmp.${process.pid}`;
        fs.writeFileSync(tmp, after);
        fs.renameSync(tmp, file);
        console.log(`${name}: ${SETTINGS_KEY} -> ${target}`);
        touched++;
    } catch (err) {
        console.warn(`${name}: skipped (${err.message})`);
    }
}

if (touched === 0) {
    console.log("No IDE settings.json found; set kul.serverPath manually.");
}

// Set or insert a top-level string key in JSONC text. Preserves comments and
// surrounding formatting. Handles three cases:
//   1. Key exists with a string value — replace the value.
//   2. Key missing in a `{ ... }` root — insert before the final `}`, adding a
//      leading comma if needed.
//   3. File empty or has no root object — write a fresh single-key object.
function setStringKey(text, key, value) {
    const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    const re = new RegExp(`("${escapedKey}"\\s*:\\s*)"(?:[^"\\\\]|\\\\.)*"`);
    const valueJson = JSON.stringify(value);
    if (re.test(text)) {
        return text.replace(re, `$1${valueJson}`);
    }
    const closeIdx = text.lastIndexOf("}");
    if (closeIdx < 0) {
        return `{\n  ${JSON.stringify(key)}: ${valueJson}\n}\n`;
    }
    const head = text.slice(0, closeIdx);
    const tail = text.slice(closeIdx);
    const indent = (head.match(/\n([ \t]+)\S/) || [, "  "])[1];
    const trimmed = head.replace(/[\s]+$/, "");
    const sep = trimmed.endsWith("{") || trimmed.endsWith(",") ? "" : ",";
    return `${trimmed}${sep}\n${indent}${JSON.stringify(key)}: ${valueJson}\n${tail}`;
}

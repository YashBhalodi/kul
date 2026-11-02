#!/usr/bin/env node
// Compares two behaviour captures for the epic #343 baseline
// (`scripts/pivot/baseline.sh diff`).
//
// Usage: node compare.mjs <baseline-dir> <current-dir>
//
// Each capture directory holds `cli/<project>.json` (from cli-capture.mjs)
// and `service/<project>.json` (from the language-feature dumper). Exits 1
// when any difference falls outside the allowlist, 0 otherwise.
//
// The allowlist, and nothing else:
//   1. removed `KUL-M*` diagnostics: dropped from the baseline before
//      comparing (JSON objects whose `code` matches, human-readable
//      diagnostic blocks headed by one). A case whose baseline lost such a
//      diagnostic may also change its exit code;
//   2. the missing-manifest error ("not a Kul project root"): a CLI case
//      whose baseline failed with it is reported as allowed, not compared;
//   3. `FileId` renumbering where it shows in debug output: every
//      `FileId(n)` reads as `FileId(#)` on both sides.
//
// Extension point for the language-service move: `mapBaselineService` turns
// one baseline service dump (LSP shapes) into the shape the current dumper
// writes. It is the identity while both sides run the same dumper.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const MANIFEST_DIAG = /^KUL-M\d+$/;
const MISSING_ROOT = "not a Kul project root";
const FILE_ID = /FileId\(\d+\)/g;
const MAX_DETAILS = 12;

/** Maps one baseline service dump onto the current dumper's shape. */
export function mapBaselineService(dump) {
  return dump;
}

// ---------------------------------------------------------------- allowlist

function dropRetiredDiagnostics(value, removed) {
  if (Array.isArray(value)) {
    const kept = [];
    for (const item of value) {
      if (item && typeof item === "object" && MANIFEST_DIAG.test(item.code ?? "")) {
        removed.count++;
        continue;
      }
      kept.push(dropRetiredDiagnostics(item, removed));
    }
    return kept;
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([k, v]) => [k, dropRetiredDiagnostics(v, removed)]),
    );
  }
  return value;
}

function normalizeFileIds(value) {
  if (typeof value === "string") return value.replace(FILE_ID, "FileId(#)");
  if (Array.isArray(value)) return value.map(normalizeFileIds);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, normalizeFileIds(v)]));
  }
  return value;
}

/** Drops human-readable diagnostic blocks headed by a `KUL-M*` code line. */
function dropRetiredBlocks(text, removed) {
  const out = [];
  let skipping = false;
  for (const line of text.split("\n")) {
    if (/^KUL-[A-Z]\d+$/.test(line)) {
      skipping = MANIFEST_DIAG.test(line);
      if (skipping) removed.count++;
    }
    if (!skipping) out.push(line);
  }
  return out.join("\n");
}

/** A CLI stream as JSON, JSON lines, or text, with the allowlist applied. */
function normalizeStream(text, { baseline, removed }) {
  const parsed = parseJsonish(text);
  if (parsed !== undefined) {
    const value = baseline ? dropRetiredDiagnostics(parsed, removed) : parsed;
    return normalizeFileIds(value);
  }
  const body = baseline ? dropRetiredBlocks(text, removed) : text;
  return body.trim() === "" ? { jsonl: [] } : normalizeFileIds(body);
}

/**
 * JSON lines (one document per line, so a single-line document too) as
 * `{ jsonl: [...] }`, a multi-line document as `{ json }`, otherwise
 * undefined. An empty stream is an empty JSON-lines stream, so a baseline
 * whose only line was a dropped diagnostic equals an empty current stream.
 */
function parseJsonish(text) {
  const trimmed = text.trim();
  if (trimmed === "") return { jsonl: [] };
  try {
    return { jsonl: trimmed.split("\n").map((l) => JSON.parse(l)) };
  } catch {
    // Not JSON lines; try one pretty-printed document.
  }
  try {
    return { json: JSON.parse(trimmed) };
  } catch {
    return undefined;
  }
}

// ---------------------------------------------------------------- comparison

function diffValues(a, b, path, out) {
  if (out.length >= MAX_DETAILS) return;
  if (Object.is(a, b)) return;
  const ta = Array.isArray(a) ? "array" : a === null ? "null" : typeof a;
  const tb = Array.isArray(b) ? "array" : b === null ? "null" : typeof b;
  if (ta !== tb) {
    out.push(`${path}: ${preview(a)} -> ${preview(b)}`);
    return;
  }
  if (ta === "array") {
    if (a.length !== b.length) out.push(`${path}: length ${a.length} -> ${b.length}`);
    for (let i = 0; i < Math.min(a.length, b.length); i++) diffValues(a[i], b[i], `${path}[${i}]`, out);
    return;
  }
  if (ta === "object") {
    const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort();
    for (const k of keys) {
      if (!(k in a)) out.push(`${path}.${k}: added ${preview(b[k])}`);
      else if (!(k in b)) out.push(`${path}.${k}: removed ${preview(a[k])}`);
      else diffValues(a[k], b[k], `${path}.${k}`, out);
      if (out.length >= MAX_DETAILS) return;
    }
    return;
  }
  if (ta === "string" && a.includes("\n") && b.includes("\n")) {
    const la = a.split("\n");
    const lb = b.split("\n");
    for (let i = 0; i < Math.max(la.length, lb.length) && out.length < MAX_DETAILS; i++) {
      if (la[i] !== lb[i]) out.push(`${path} line ${i + 1}: ${preview(la[i])} -> ${preview(lb[i])}`);
    }
    return;
  }
  out.push(`${path}: ${preview(a)} -> ${preview(b)}`);
}

function preview(v) {
  const s = v === undefined ? "<missing>" : JSON.stringify(v);
  return s.length > 160 ? `${s.slice(0, 157)}...` : s;
}

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function projectFiles(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).filter((f) => f.endsWith(".json")).sort();
}

function compareCli(baseDir, curDir, report) {
  const names = [...new Set([...projectFiles(baseDir), ...projectFiles(curDir)])].sort();
  for (const name of names) {
    const label = `cli/${name.replace(/\.json$/, "")}`;
    if (!existsSync(join(baseDir, name)) || !existsSync(join(curDir, name))) {
      report.diff(label, [`present only in ${existsSync(join(baseDir, name)) ? "baseline" : "current"}`]);
      continue;
    }
    const base = readJson(join(baseDir, name));
    const cur = readJson(join(curDir, name));
    const cases = [...new Set([...Object.keys(base), ...Object.keys(cur)])].sort();
    const details = [];
    let allowed = 0;
    for (const key of cases) {
      const b = base[key];
      const c = cur[key];
      if (!b || !c) {
        details.push(`[${key}] present only in ${b ? "baseline" : "current"}`);
        continue;
      }
      if (`${b.stdout}${b.stderr}`.includes(MISSING_ROOT)) {
        allowed++;
        report.allow(`${label} [${key}]`, "missing-manifest error in the baseline");
        continue;
      }
      const removed = { count: 0 };
      const nb = {
        stdout: normalizeStream(b.stdout, { baseline: true, removed }),
        stderr: normalizeStream(b.stderr, { baseline: true, removed }),
      };
      const nc = {
        stdout: normalizeStream(c.stdout, { baseline: false, removed: { count: 0 } }),
        stderr: normalizeStream(c.stderr, { baseline: false, removed: { count: 0 } }),
      };
      const caseDetails = [];
      diffValues(nb, nc, `[${key}]`, caseDetails);
      if (b.exit !== c.exit && removed.count === 0) {
        caseDetails.push(`[${key}] exit ${b.exit} -> ${c.exit}`);
      }
      if (removed.count > 0 && caseDetails.length === 0) {
        allowed++;
        report.allow(`${label} [${key}]`, `${removed.count} KUL-M diagnostic(s) removed`);
      }
      details.push(...caseDetails);
    }
    if (details.length) report.diff(label, details);
    else report.ok(label, `${cases.length} cases${allowed ? `, ${allowed} allowed` : ""}`);
  }
}

function compareService(baseDir, curDir, report) {
  const names = [...new Set([...projectFiles(baseDir), ...projectFiles(curDir)])].sort();
  for (const name of names) {
    const label = `service/${name.replace(/\.json$/, "")}`;
    if (!existsSync(join(baseDir, name)) || !existsSync(join(curDir, name))) {
      report.diff(label, [`present only in ${existsSync(join(baseDir, name)) ? "baseline" : "current"}`]);
      continue;
    }
    const removed = { count: 0 };
    const base = normalizeFileIds(
      dropRetiredDiagnostics(mapBaselineService(readJson(join(baseDir, name))), removed),
    );
    const cur = normalizeFileIds(readJson(join(curDir, name)));
    const details = [];
    diffValues(base, cur, "$", details);
    if (details.length) report.diff(label, details);
    else {
      report.ok(label, "semantically equal");
      if (removed.count) report.allow(label, `${removed.count} KUL-M diagnostic(s) removed`);
    }
  }
}

function main() {
  const [baseline, current] = process.argv.slice(2);
  if (!baseline || !current) {
    console.error("usage: node compare.mjs <baseline-dir> <current-dir>");
    process.exit(2);
  }
  let failures = 0;
  const report = {
    ok: (label, note) => console.log(`ok       ${label} (${note})`),
    allow: (label, reason) => console.log(`allowed  ${label}: ${reason}`),
    diff: (label, details) => {
      failures++;
      console.log(`DIFF     ${label}`);
      for (const d of details.slice(0, MAX_DETAILS)) console.log(`           ${d}`);
      if (details.length > MAX_DETAILS) console.log(`           ... ${details.length - MAX_DETAILS} more`);
    },
  };
  compareCli(join(baseline, "cli"), join(current, "cli"), report);
  compareService(join(baseline, "service"), join(current, "service"), report);
  if (failures) {
    console.log(`\nbaseline diff: ${failures} unexplained difference(s)`);
    process.exit(1);
  }
  console.log("\nbaseline diff: no differences outside the allowlist");
}

main();

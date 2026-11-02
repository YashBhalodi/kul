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
// The baseline's language features come from the pinned tree's language
// server (lsp-dump, LSP shapes); the current tree's from kul-language-service
// (service-dump, Kul-native shapes). `mapBaselineService` maps the first onto
// the second before comparing, so the comparison is semantic: same results,
// different shapes. LSP positions (line, UTF-16 character) become byte offsets
// into the baseline's own copy of each example (`<baseline>/examples`), URIs
// become file names, the semantic-token delta stream is decoded through the
// legend, and LSP enum numbers become their Kul-native names. Every field of
// either shape is carried or explicitly accounted for; nothing is dropped
// silently, so a real behaviour difference still shows as a DIFF.

import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const MANIFEST_DIAG = /^KUL-M\d+$/;
const MISSING_ROOT = "not a Kul project root";
const FILE_ID = /FileId\(\d+\)/g;
const MAX_DETAILS = 12;

// ---------------------------------------------------------------- LSP -> native

// LSP CompletionItemKind, SymbolKind and DiagnosticSeverity numbers.
const COMPLETION_KIND = { 14: "keyword", 5: "field", 20: "enumValue", 12: "value", 23: "marriage", 6: "person" };
const SYMBOL_KIND = { 13: "person", 24: "marriage" };
const SYMBOL_FIELD = 8; // birth and adoption, told apart by their label
const SEVERITY = { 1: "error", 2: "warning", 3: "note" };
// LSP semantic-token legend names -> Kul-native token kinds.
const TOKEN_KIND = {
  keyword: "keyword",
  property: "fieldName",
  function: "declaration",
  parameter: "reference",
  enum: "enumValue",
  number: "date",
  string: "string",
};

/** Line starts of `text`, for LSP position -> byte offset. */
function lineIndex(text) {
  const lines = text.split("\n");
  const starts = [];
  let at = 0;
  for (const line of lines) {
    starts.push(at);
    at += Buffer.byteLength(line) + 1;
  }
  return { lines, starts };
}

/** LSP `{ line, character }` (UTF-16 units) -> UTF-8 byte offset. */
function toByte(index, { line, character }) {
  let units = 0;
  let bytes = index.starts[line];
  for (const ch of index.lines[line]) {
    if (units >= character || units + ch.length > character) break;
    units += ch.length;
    bytes += Buffer.byteLength(ch);
  }
  return bytes;
}

function fail(message) {
  throw new Error(`mapBaselineService: ${message}`);
}

/**
 * Maps one baseline service dump (LSP shapes) onto service-dump's shape.
 * `sources` maps each of the project's file names to its text; `legend` is
 * the baseline's semantic-token legend.
 */
export function mapBaselineService(dump, sources, legend) {
  const indexes = Object.fromEntries(Object.entries(sources).map(([f, t]) => [f, lineIndex(t)]));
  const fileOf = (uri) => {
    const file = uri.slice(uri.lastIndexOf("/") + 1);
    if (!(file in indexes)) fail(`no source for ${uri}`);
    return file;
  };
  const span = (file, range) => ({
    start: toByte(indexes[file], range.start),
    end: toByte(indexes[file], range.end),
  });
  const location = (loc) => {
    const file = fileOf(loc.uri);
    return { file, span: span(file, loc.range) };
  };
  const diagnostic = (file, d) => {
    if (!(d.severity in SEVERITY)) fail(`diagnostic severity ${d.severity}`);
    return {
      code: d.code,
      severity: SEVERITY[d.severity],
      message: d.message,
      location: { file, span: span(file, d.range) },
      related: (d.relatedInformation ?? []).map((r) => ({ message: r.message, location: location(r.location) })),
    };
  };
  const fileEdits = (edit) =>
    Object.entries(edit.changes ?? {})
      .map(([uri, edits]) => {
        const file = fileOf(uri);
        return { file, edits: edits.map((e) => ({ span: span(file, e.range), text: e.newText })) };
      })
      .sort((a, b) => (a.file < b.file ? -1 : a.file > b.file ? 1 : 0));
  const completion = (item) => {
    if (!(item.kind in COMPLETION_KIND)) fail(`completion kind ${item.kind}`);
    const out = {
      label: item.label,
      kind: COMPLETION_KIND[item.kind],
      detail: item.detail,
      preselect: item.preselect ?? false,
    };
    if (item.insertText !== undefined) {
      if (item.insertTextFormat !== 2) fail(`insert text format ${item.insertTextFormat}`);
      const cursor = item.insertText.indexOf("$0");
      out.insert = { text: item.insertText.replace("$0", ""), cursor: Buffer.byteLength(item.insertText.slice(0, cursor)) };
    }
    return out;
  };
  const symbol = (file, s) => {
    let kind = SYMBOL_KIND[s.kind];
    if (kind === undefined) {
      if (s.kind !== SYMBOL_FIELD) fail(`symbol kind ${s.kind}`);
      kind = s.name.startsWith("birth ") ? "birth" : "adoption";
    }
    const out = {
      name: s.name,
      kind,
      span: span(file, s.range),
      selection: span(file, s.selectionRange),
      children: (s.children ?? []).map((c) => symbol(file, c)),
    };
    if (s.detail != null) out.detail = s.detail;
    return out;
  };
  const semanticTokens = (file, { data }) => {
    const tokens = [];
    let line = 0;
    let character = 0;
    for (let i = 0; i < data.length; i += 5) {
      const [deltaLine, deltaStart, length, type, modifiers] = data.slice(i, i + 5);
      if (modifiers !== 0) fail(`token modifiers ${modifiers}`);
      line += deltaLine;
      character = deltaLine > 0 ? deltaStart : character + deltaStart;
      const kind = TOKEN_KIND[legend.tokenTypes[type]];
      if (kind === undefined) fail(`token type ${type}`);
      tokens.push({
        span: { start: toByte(indexes[file], { line, character }), end: toByte(indexes[file], { line, character: character + length }) },
        kind,
      });
    }
    return tokens;
  };
  const quickFix = (file, action) => {
    if (action.kind !== "quickfix" || action.diagnostics?.length !== 1) fail(`code action ${action.title}`);
    const edits = fileEdits(action.edit);
    if (edits.length !== 1 || edits[0].file !== file) fail(`code action ${action.title} edits another file`);
    return { title: action.title, diagnostic: diagnostic(file, action.diagnostics[0]), edits: edits[0].edits };
  };
  const position = (file, p) => ({
    offset: p.offset,
    hover:
      p.hover === null
        ? null
        : p.hover.contents.kind === "markdown"
          ? { markdown: p.hover.contents.value, span: span(file, p.hover.range) }
          : fail(`hover contents ${p.hover.contents.kind}`),
    completion: p.completion?.map(completion) ?? [],
    definition: p.definition === null ? null : location(p.definition),
    references: p.references?.map(location) ?? null,
    referencesWithDeclaration: p.referencesWithDeclaration?.map(location) ?? null,
    prepareRename: p.prepareRename === null ? null : span(file, p.prepareRename),
    rename: "ok" in p.rename ? { ok: fileEdits(p.rename.ok) } : { error: p.rename.error },
    entityAt: p.entityAt.entity,
  });

  const files = {};
  for (const [file, f] of Object.entries(dump.files)) {
    if (!(file in indexes)) fail(`no source for ${file}`);
    files[file] = {
      diagnostics: f.diagnostics.map((d) => diagnostic(file, d)),
      symbols: f.symbols.map((s) => symbol(file, s)),
      semanticTokens: semanticTokens(file, f.semanticTokens),
      format:
        f.formatting === null
          ? null
          : f.formatting.map((e) => ({ span: span(file, e.range), text: e.newText })),
      quickFixes: f.codeActions.map((a) => quickFix(file, a)),
      positions: f.positions.map((p) => position(file, p)),
    };
  }
  return {
    project: dump.project,
    files,
    locate: Object.fromEntries(
      Object.entries(dump.locate).map(([id, r]) => [id, r.location === null ? null : location(r.location)]),
    ),
  };
}

/** The text of every `.kul` file of one project in a capture's examples copy. */
function projectSources(examples, project) {
  const dir = join(examples, project);
  return Object.fromEntries(
    readdirSync(dir)
      .filter((f) => f.endsWith(".kul"))
      .map((f) => [f, readFileSync(join(dir, f), "utf8")]),
  );
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

function compareService(baseDir, curDir, examples, report) {
  const legend = readJson(join(baseDir, "_legend.json"));
  const dumps = (dir) => projectFiles(dir).filter((f) => f !== "_legend.json");
  const names = [...new Set([...dumps(baseDir), ...dumps(curDir)])].sort();
  for (const name of names) {
    const label = `service/${name.replace(/\.json$/, "")}`;
    if (!existsSync(join(baseDir, name)) || !existsSync(join(curDir, name))) {
      report.diff(label, [`present only in ${existsSync(join(baseDir, name)) ? "baseline" : "current"}`]);
      continue;
    }
    const removed = { count: 0 };
    const sources = projectSources(examples, name.replace(/\.json$/, ""));
    const mapped = mapBaselineService(readJson(join(baseDir, name)), sources, legend);
    const base = normalizeFileIds(dropRetiredDiagnostics(mapped, removed));
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
  compareService(join(baseline, "service"), join(current, "service"), join(baseline, "examples"), report);
  if (failures) {
    console.log(`\nbaseline diff: ${failures} unexplained difference(s)`);
    process.exit(1);
  }
  console.log("\nbaseline diff: no differences outside the allowlist");
}

main();

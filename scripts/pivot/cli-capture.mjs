#!/usr/bin/env node
// Behaviour capture of the `kul` CLI over a directory of example projects,
// for the epic #343 baseline (`scripts/pivot/baseline.sh`).
//
// Usage: node cli-capture.mjs <kul-binary> <examples-root> <out-dir>
//
// Every subdirectory of <examples-root> that holds a `.kul` file is one
// project. Each project writes `<out-dir>/<project>.json`: an object keyed
// by the case label (the argument list), each value `{ exit, stdout, stderr }`.
//
// Cases per project:
//   validate (human, json), format --check, export --format json|svg,
//   query persons (human, json, --count),
//   query person <id> (human, json) for every person plus one unknown id,
//   query marriage <id> (human, json) for every marriage,
//   query kin <id> <relation> --format json for every person and relation,
//   query rel <x> <y> --format json for every unordered pair of persons.
//
// The project directory's absolute path is replaced by `<PROJECT_DIR>` in
// every captured stream, so captures from different checkouts compare.

import { spawnSync } from "node:child_process";
import { mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

const RELATIONS = [
  ["parents"],
  ["children"],
  ["ancestors"],
  ["descendants"],
  ["siblings"],
  ["aunts-uncles"],
  ["nieces-nephews"],
  ["cousins", "--degree", "1"],
  ["cousins", "--degree", "2", "--removed", "1"],
  ["spouses"],
  ["in-laws"],
  ["step-parents"],
  ["step-siblings"],
  ["step-children"],
];

// The one place that decides how a CLI command is pointed at a project.
// Today and after the manifest-less CLI alike, the project is the working
// directory, so the same invocation runs on both sides of the diff.
function invoke(kul, projectDir, args) {
  const r = spawnSync(kul, args, {
    cwd: projectDir,
    encoding: "utf8",
    maxBuffer: 256 * 1024 * 1024,
  });
  if (r.error) throw r.error;
  const scrub = (s) => s.split(projectDir).join("<PROJECT_DIR>");
  return { exit: r.status, stdout: scrub(r.stdout), stderr: scrub(r.stderr) };
}

function captureProject(kul, dir) {
  const cases = {};
  const run = (args) => {
    cases[args.join(" ")] = invoke(kul, dir, args);
    return cases[args.join(" ")];
  };

  run(["validate"]);
  run(["validate", "--format", "json"]);
  run(["format", "--check"]);
  const exported = run(["export", "--format", "json"]);
  run(["export", "--format", "svg"]);
  run(["query", "persons"]);
  run(["query", "persons", "--format", "json"]);
  run(["query", "persons", "--count"]);

  let persons = [];
  let marriages = [];
  if (exported.exit === 0) {
    const graph = JSON.parse(exported.stdout).graph;
    persons = graph.persons.map((p) => p.id).sort();
    marriages = graph.marriages.map((m) => m.id).sort();
  }
  for (const id of [...persons, "no_such_person"]) {
    run(["query", "person", id]);
    run(["query", "person", id, "--format", "json"]);
  }
  for (const id of marriages) {
    run(["query", "marriage", id]);
    run(["query", "marriage", id, "--format", "json"]);
  }
  for (const id of persons) {
    for (const rel of RELATIONS) run(["query", "kin", id, ...rel, "--format", "json"]);
  }
  for (let i = 0; i < persons.length; i++) {
    for (let j = i + 1; j < persons.length; j++) {
      run(["query", "rel", persons[i], persons[j], "--format", "json"]);
    }
  }
  return cases;
}

function main() {
  const [kul, root, out] = process.argv.slice(2);
  if (!kul || !root || !out) {
    console.error("usage: node cli-capture.mjs <kul-binary> <examples-root> <out-dir>");
    process.exit(2);
  }
  mkdirSync(out, { recursive: true });
  const projects = readdirSync(root)
    .map((name) => ({ name, dir: resolve(root, name) }))
    .filter(({ dir }) => statSync(dir).isDirectory())
    .filter(({ dir }) => readdirSync(dir).some((f) => f.endsWith(".kul")))
    .sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  for (const { name, dir } of projects) {
    const cases = captureProject(resolve(kul), dir);
    writeFileSync(join(out, `${name}.json`), `${JSON.stringify(cases, null, 2)}\n`);
    console.error(`cli-capture: ${name} (${Object.keys(cases).length} cases)`);
  }
}

main();

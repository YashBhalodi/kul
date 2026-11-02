// End-to-end smoke test of the built WebAssembly module, driven from Node
// the way a worker drives it: load the `--target web` output with
// `initSync`, fill a `Workspace` and call every feature once. Catches
// WASM-toolchain and JS-glue regressions the Rust tests cannot see, and
// checks UTF-16 offsets from the JavaScript side, slicing JS strings.
//
// Run with `node crates/kul-wasm/tests/node/smoke.mjs` after `just wasm`.

import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { diagnosticCodes, initSync, KUL_CORE_VERSION, KUL_LANGUAGE_VERSION, Workspace } from '../../pkg/kul_wasm.js';

initSync({ module: readFileSync(new URL('../../pkg/kul_wasm_bg.wasm', import.meta.url)) });

assert.match(KUL_CORE_VERSION(), /^\d+\.\d+\.\d+/);

// the diagnostic catalogue: every code with a summary
const catalogue = diagnosticCodes();
assert.ok(catalogue.some((entry) => entry.code === 'KUL-R03' && entry.summary.length > 0));
assert.ok(catalogue.every((entry) => /^KUL-[LPRVQ]\d\d[a-z]?$/.test(entry.code)));
assert.equal(KUL_LANGUAGE_VERSION(), '0.1');

const examples = new URL('../../../../examples/', import.meta.url);
const project = (name) =>
    readdirSync(new URL(`${name}/`, examples))
        .filter((f) => f.endsWith('.kul'))
        .sort()
        .map((f) => [f, readFileSync(new URL(`${name}/${f}`, examples), 'utf8')]);

// --- every example analyzes clean and renders
for (const name of readdirSync(examples).filter((n) => !n.includes('.'))) {
    const files = project(name);
    if (files.length === 0) continue;
    const ws = new Workspace();
    files.forEach(([file, text], i) => ws.setFile(file, text, i + 1));
    const analysis = ws.analyze();
    assert.deepEqual(analysis.diagnostics, [], `${name} has diagnostics`);
    assert.ok(analysis.svg?.startsWith('<svg'), `${name} renders`);
    assert.deepEqual(
        analysis.files.map((f) => f.name),
        files.map(([f]) => f),
        `${name} files in file order`,
    );
    ws.free();
}

// --- a two-file project with non-ASCII text, offsets in UTF-16 units
const family = [
    '# પરિવાર 👨‍👩‍👧 — परिवार',
    'person bhavna name:"ભાવના પટેલ" gender:female born:~1950',
    'person ravi name:"રવિ 🎉" gender:male',
    'marriage m_ravi_bhavna ravi bhavna start:1972 # लग्न 💍',
    '',
].join('\n');
const children = [
    '# 🎉 બાળકો — बच्चे',
    'person kiran name:"કિરણ" gender:other',
    '  birth m_ravi_bhavna',
    '',
].join('\n');
const text = { 'a.kul': family, 'b.kul': children };
const at = (file, needle, skip = 0) => text[file].indexOf(needle) + skip;
const slice = (file, span) => text[file].slice(span.start, span.end);

const ws = new Workspace();
ws.setFile('b.kul', children, 7);
ws.setFile('a.kul', family, 3);

let analysis = ws.analyze();
assert.deepEqual(analysis.files, [
    { name: 'a.kul', version: 3 },
    { name: 'b.kul', version: 7 },
]);
assert.deepEqual(analysis.diagnostics, []);
assert.ok(analysis.svg.includes('data-person-id="kiran"'));

// hover
const ref = at('b.kul', 'birth m_', 6);
const hover = ws.hover('b.kul', ref);
assert.equal(slice('b.kul', hover.span), 'm_ravi_bhavna');
assert.match(hover.markdown, /ભાવના પટેલ/);
assert.equal(ws.hover('a.kul', at('a.kul', '# પરિવાર')), undefined);

// complete
const genders = ws.complete('a.kul', at('a.kul', 'gender:male', 7)).map((c) => c.label);
assert.deepEqual(genders, ['male', 'female', 'other']);
const marriages = ws.complete('b.kul', ref).map((c) => [c.label, c.kind]);
assert.deepEqual(marriages, [['m_ravi_bhavna', 'marriage']]);

// definition, references
const definition = ws.definition('b.kul', ref);
assert.equal(definition.file, 'a.kul');
assert.equal(slice('a.kul', definition.span), 'm_ravi_bhavna');
const references = ws.references('b.kul', ref, true);
assert.deepEqual(references.map((l) => [l.file, slice(l.file, l.span)]), [
    ['a.kul', 'm_ravi_bhavna'],
    ['b.kul', 'm_ravi_bhavna'],
]);
assert.equal(ws.references('a.kul', 0, true), undefined);

// rename across two files
assert.equal(slice('a.kul', ws.prepareRename('a.kul', at('a.kul', 'm_ravi'))), 'm_ravi_bhavna');
const renamed = ws.rename('a.kul', at('a.kul', 'm_ravi'), 'm_rb');
assert.equal(renamed.ok, true);
assert.deepEqual(
    renamed.edits.map((f) => [f.file, f.edits.map((e) => [slice(f.file, e.span), e.text])]),
    [
        ['a.kul', [['m_ravi_bhavna', 'm_rb']]],
        ['b.kul', [['m_ravi_bhavna', 'm_rb']]],
    ],
);
const refused = ws.rename('a.kul', at('a.kul', 'ravi name'), 'bhavna');
assert.equal(refused.ok, false);
assert.match(refused.message, /already used/);

// entity at, locate, details
assert.deepEqual(ws.entityAt('a.kul', at('a.kul', ' ravi bhavna', 1)), { id: 'ravi', kind: 'person' });
const located = ws.locate('kiran');
assert.equal(located.file, 'b.kul');
assert.equal(slice('b.kul', located.span), 'kiran');
assert.equal(ws.locate('nobody'), undefined);
const details = ws.details('m_ravi_bhavna');
assert.equal(details.kind, 'marriage');
assert.deepEqual(details.spouses.map((p) => p.name), ['રવિ 🎉', 'ભાવના પટેલ']);
assert.deepEqual(details.children.map((c) => c.person.id), ['kiran']);
assert.equal(ws.details('bhavna').kind, 'person');

// symbols, tokens
assert.deepEqual(
    ws.symbols('a.kul').map((s) => [s.name, s.kind, slice('a.kul', s.selection)]),
    [
        ['ભાવના પટેલ', 'person', 'bhavna'],
        ['રવિ 🎉', 'person', 'ravi'],
        ['રવિ 🎉 & ભાવના પટેલ', 'marriage', 'm_ravi_bhavna'],
    ],
);
const lexical = ws.lexicalTokens('a.kul').map((t) => [slice('a.kul', t.span), t.kind]);
assert.deepEqual(lexical[0], ['# પરિવાર 👨‍👩‍👧 — परिवार', 'comment']);
assert.ok(lexical.some(([t, k]) => t === '"ભાવના પટેલ"' && k === 'string'));
const semantic = ws.semanticTokens('a.kul').map((t) => [slice('a.kul', t.span), t.kind]);
assert.ok(semantic.some(([t, k]) => t === 'm_ravi_bhavna' && k === 'declaration'));
assert.ok(semantic.every((token) => lexical.some((l) => l[0] === token[0] && l[1] === token[1])));

// renderSvg in both themes
const agnostic = ws.renderSvg('agnostic');
assert.equal(agnostic, analysis.svg);
assert.ok(!agnostic.includes('<style>'));
const download = ws.renderSvg('fileExport');
assert.ok(download.startsWith('<svg') && download.includes('<style>'));

// format, quick fixes, diagnostics after edits (versions tag the answer)
const messy = 'person dev name:"देव 🎉"\n';
ws.setFile('c.kul', messy, 1);
text['c.kul'] = messy;
analysis = ws.analyze();
assert.equal(analysis.svg, undefined);
const r03 = analysis.diagnostics.find((d) => d.code === 'KUL-R03');
assert.equal(r03.location.file, 'c.kul');
assert.equal(slice('c.kul', r03.location.span), 'dev');
assert.equal(ws.renderSvg('fileExport'), undefined);
assert.equal(ws.details('bhavna'), undefined);

const fix = ws.quickFixes('c.kul', 0, messy.length).find((f) => f.title === 'Add `gender:female`');
const [edit] = fix.edits;
const fixed = messy.slice(0, edit.span.start) + edit.text + messy.slice(edit.span.end);
assert.equal(fixed, 'person dev name:"देव 🎉" gender:female\n');
ws.setFile('c.kul', fixed, 2);
const formatted = ws.format('c.kul');
assert.equal(formatted.length, 1);
assert.deepEqual(formatted[0].span, { start: 0, end: fixed.length });
assert.equal(formatted[0].text, 'person dev  name:"देव 🎉"  gender:female\n');
analysis = ws.analyze();
assert.deepEqual(analysis.diagnostics, []);
assert.equal(analysis.files.find((f) => f.name === 'c.kul').version, 2);

// file lifecycle
assert.equal(ws.renameFile('c.kul', 'a.kul'), false);
assert.equal(ws.renameFile('c.kul', '0.kul'), true);
assert.equal(ws.locate('dev').file, '0.kul');
assert.equal(ws.deleteFile('0.kul'), true);
assert.equal(ws.locate('dev'), undefined);
ws.free();

console.log('kul-wasm smoke: ok');

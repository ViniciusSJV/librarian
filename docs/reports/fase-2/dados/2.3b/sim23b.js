// Step 2.3b, Parada 1: prediction for option B (rule) and A reduzida (graph), from the run of the
// current binary (q23b-out/graph.json), the answers of the calls outside bodies
// (q23b-out/fora-do-corpo.json) and the reader's places (topo2.jsonl). Only the calls the options
// touch change; every other answer is today's. Usage: node sim23b.js
'use strict';
const fs = require('fs');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const FIX = 'C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/fixtures/';
const g = JSON.parse(fs.readFileSync('q23b-out/graph.json', 'utf8'));
const { res } = JSON.parse(fs.readFileSync('q23b-out/fora-do-corpo.json', 'utf8'));
const rows = fs.readFileSync('topo2.jsonl', 'utf8').trim().split('\n').map(JSON.parse);
const declared = {}, parent = {};
for (const e of g.edges) { if (e.kind === 'declares') declared[e.to] = e.evidence; else if (e.kind === 'contains') parent[e.to] = e.from; }
const G = { g, declared, parent, where: id => declared[id] ? `${declared[id].path}:${declared[id].start_line}` : '?' };
const test = p => /(^|\/)tests?\/|test_/.test(p);

// Where each call occurrence of the run is written, by the reader.
const place = new Map(rows.filter(r => r.stmt !== 'decorator_expr').map(r => [`${r.path}@${r.start}`, r]));
const occ = [];
for (const e of g.edges) {
  if (e.kind !== 'observes' || !/call_observed$/.test(g.nodes[e.to].kind)) continue;
  const r = place.get(`${e.evidence.path}@${e.evidence.start_byte}`);
  occ.push({ id: e.to, key: L.occKey(e.evidence), owner: e.from, path: e.evidence.path, line: e.evidence.start_line, text: g.nodes[e.to].label.split(/\r?\n/)[0], place: r?.place, fieldInInit: r?.place === 'funcao' && g.nodes[e.from].kind === 'field' });
}
const byKey = new Map(occ.map(o => [o.key, o]));
const answer = new Map(res.filter(r => r.occ).map(r => [r.occ, r]));

// B: decorator links get relation `decorates`; a field's value of an __init__ also answers in the method.
const changesB = [];
for (const o of occ) {
  if (o.place?.startsWith('decorador') && answer.get(o.id)?.targets?.length) {
    const a = answer.get(o.id);
    changesB.push(['relacao', `${o.path}:${o.line}`, o.text, g.nodes[o.owner].label, `${a.resolution} ${a.relation} -> ${a.resolution} decorates`, a.targets.join('; ')]);
  }
  if (o.fieldInInit) changesB.push(['dono', `${o.path}:${o.line}`, o.text, g.nodes[o.owner].label, `também em /api/calls de ${g.nodes[parent[o.owner]].label}`, '']);
}
// A reduzida: top-level decorators move to the module symbol; a call out of every symbol enters.
const changesA = occ.filter(o => /^decorador_(funcao|classe)_topo$/.test(o.place)).map(o => ['dono', `${o.path}:${o.line}`, o.text, g.nodes[o.owner].label, `-> módulo ${o.path}`, '']);
const outside = rows.filter(r => r.place === 'topo' && !r.owner_kind && r.stmt !== 'decorator_expr');

// Population (Part C pool): which of its calls each option touches.
const P = L.pool(G);
const poolB = P.filter(p => { const o = byKey.get(p.key); return o && (o.place?.startsWith('decorador') || o.fieldInInit); });
const poolA = P.filter(p => /^decorador_(funcao|classe)_topo$/.test(byKey.get(p.key)?.place));

// Sample and facts: which entries the options touch.
const touched = [];
for (const lang of ['hc', 'rt']) {
  const fx = JSON.parse(fs.readFileSync(FIX + `amostra-${lang}.json`, 'utf8'));
  for (const c of fx.chamadas) {
    const o = byKey.get(c.ocorrencia);
    if (o && (o.place?.startsWith('decorador') || o.fieldInInit)) touched.push(`${c.id} ${o.place}${o.fieldInInit ? ' (campo)' : ''} ${c.texto} verdade=${JSON.stringify(c.verdade)}`);
  }
}
const fatos = JSON.parse(fs.readFileSync(FIX + 'fatos.json', 'utf8'));
const decoLines = new Set(occ.filter(o => o.place?.startsWith('decorador') || o.fieldInInit).map(o => `${o.line}|${o.text}`));
const factHits = (fatos.fatos || fatos).flatMap(f => (f.chamadas || []).filter(c => decoLines.has(`${c.linha}|${c.texto}`)).map(c => `${f.id} ${c.linha} ${c.texto}`));

const tsv = rows => rows.map(r => r.join('\t')).join('\n');
fs.writeFileSync('mudancas-previstas-2.3b-B.tsv', 'tipo\tlocal\ttexto\tdono no grafo\tmudança\talvos\n' + tsv(changesB) + '\n');
fs.writeFileSync('mudancas-previstas-2.3b-A-reduzida.tsv', 'tipo\tlocal\ttexto\tdono no grafo\tmudança\talvos\n' + tsv(changesA) + '\n');
const count = (xs, f) => xs.reduce((m, x) => (m[f(x)] = (m[f(x)] || 0) + 1, m), {});
console.log(JSON.stringify({
  B: { mudancas: count(changesB, c => `${c[0]} ${test(c[1]) ? 'teste' : 'prod'}`), populacao_tocada: poolB.length, populacao_tocada_resolucoes: count(poolB, p => answer.get(p.occ.id)?.resolution) },
  A_reduzida: { dono_muda: count(changesA, c => test(c[1]) ? 'teste' : 'prod'), entram_no_grafo: outside.length, populacao_sai: poolA.length, populacao_sai_resolucoes: count(poolA, p => answer.get(p.occ.id)?.resolution) },
  amostra_tocada: touched, fatos_tocados: factHits, populacao_total: P.length,
}, null, 1));

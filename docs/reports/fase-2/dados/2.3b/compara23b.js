// Step 2.3b, Parada 2: /api/calls and /api/used-by of every symbol, before (2.3a binary) and after (2.3b
// binary), compared call by call with the prediction made from "before":
//   - every link whose occurrence lies in a decorator (the reader topo23b, fixed) gets `decorates` and no `runs`;
//   - a function or method answers also for the calls of the fields it contains, with `via` = the field;
//   - everything else, used_by included, stays the same.
// Usage: node compara23b.js <dump-antes> <dump-depois> <topo.jsonl> <graph.json of the HC run> <out.json>
'use strict';
const fs = require('fs'), path = require('path');
const [antesDir, depoisDir, topoFile, graphFile, outFile] = process.argv.slice(2);
const keys = ['healthchecks-25597e41', 'renderer-9b78f3a9'];
const decorators = {};
for (const r of fs.readFileSync(topoFile, 'utf8').trim().split('\n').map(JSON.parse)) {
  if (r.stmt === 'decorator_expr') (decorators[r.path] ??= []).push([r.start, r.end]);
}
const g = JSON.parse(fs.readFileSync(graphFile, 'utf8'));
const evidence = {};
for (const e of g.edges) if (e.kind === 'observes') evidence[e.to] = e.evidence;
const inDecorator = occ => { const e = evidence[occ]; return !!e && (decorators[e.path] ?? []).some(([a, b]) => a <= e.start_byte && e.end_byte <= b); };
const fieldsIn = {};
for (const e of g.edges) if (e.kind === 'contains' && g.nodes[e.to].kind === 'field' && /^(method|function)$/.test(g.nodes[e.from].kind)) (fieldsIn[e.from] ??= []).push(e.to);

const out = {};
for (const key of keys) {
  const A = JSON.parse(fs.readFileSync(path.join(antesDir, `${key}.json`), 'utf8')).res;
  const D = JSON.parse(fs.readFileSync(path.join(depoisDir, `${key}.json`), 'utf8')).res;
  const hc = key.startsWith('healthchecks');
  const s = { simbolos: Object.keys(A).length, simbolos_depois: Object.keys(D).length, relacao_decora: 0, runs_tirado: 0, via_links: 0, via_unresolved: 0,
    used_by_iguais: 0, used_by_diferentes: [], fora_da_previsao: [], previsao_nao_cumprida: [] };
  for (const id of Object.keys(A)) {
    const a = A[id], d = D[id];
    if (!d) { s.fora_da_previsao.push(`${a.label}: sumiu`); continue; }
    // used_by: unchanged.
    if (JSON.stringify(a.used_by) === JSON.stringify(d.used_by)) s.used_by_iguais++; else s.used_by_diferentes.push(`${a.label} (${a.at})`);
    // Predicted links and unresolved entries.
    const links = a.links.map(l => {
      if (hc && inDecorator(l[0])) { if (l[4] !== 'decorates') s.relacao_decora++; if (l[6]) s.runs_tirado++; return [l[0], l[1], l[2], l[3], 'decorates', null, null, l[7]]; }
      return l;
    });
    const unresolved = [...a.unresolved];
    for (const f of hc ? fieldsIn[id] ?? [] : []) {
      for (const l of A[f]?.links ?? []) { links.push([l[0], id, l[2], l[3], l[4], f, l[6], l[7]]); s.via_links++; }
      for (const u of A[f]?.unresolved ?? []) { unresolved.push([u[0], id, u[2], f, u[4]]); s.via_unresolved++; }
    }
    const norm = xs => xs.map(x => JSON.stringify(x)).sort();
    const want = norm(links).concat(norm(unresolved)), got = norm(d.links).concat(norm(d.unresolved));
    const wantSet = new Set(want), gotSet = new Set(got);
    for (const x of got) if (!wantSet.has(x)) s.fora_da_previsao.push(`${a.label} (${a.at}): ${x}`);
    for (const x of want) if (!gotSet.has(x)) s.previsao_nao_cumprida.push(`${a.label} (${a.at}): ${x}`);
  }
  out[key] = s;
  console.log(key, JSON.stringify({ ...s, used_by_diferentes: s.used_by_diferentes.length, fora_da_previsao: s.fora_da_previsao.length, previsao_nao_cumprida: s.previsao_nao_cumprida.length }));
}
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));

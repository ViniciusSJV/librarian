// node compara.js : binary "depois" dump vs simulator prediction (simR2.json) vs "antes" dump, call by call.
'use strict';
const fs = require('fs'), path = require('path');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const sim = require('./simR2.json');
const out = {};
const norm = (rows) => { const by = {}; for (const [owner, r] of Object.entries(rows)) {
  for (const l of r.links) { const k = owner + '|' + l.occ; (by[k] ??= { r: l.resolution, t: [], basis: l.basis, n: l.candidates, edge: !!l.edge }).t.push(l.to.label + ' (' + l.to.at + ')'); }
  for (const u of r.unresolved) { const k = owner + '|' + u.occ; if (u.reason !== 'node_limit') by[k] = { r: u.reason, t: [], n: u.candidates }; else (by[k] ??= { r: u.resolution, t: [], n: u.candidates }).t.push(u.to.label + ' (' + u.to.at + ')'); } }
  for (const v of Object.values(by)) v.t.sort(); return by; };
for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  const ga = fs.readFileSync(`antes/${key}.graph.json`, 'utf8'), gd = fs.readFileSync(`depois/${key}.graph.json`, 'utf8');
  const g = JSON.parse(gd);
  const pool = new Set(L.pool({ g }).map(p => p.occ.id));
  const A = norm(JSON.parse(fs.readFileSync(`antes/${key}.calls.json`, 'utf8')).res), D = norm(JSON.parse(fs.readFileSync(`depois/${key}.calls.json`, 'utf8')).res);
  const P = {}; for (const r of sim[key]) { const owner = Object.keys(g.nodes).length && null; }
  // sim rows have occ + owner label; map by occ+owner label
  const simBy = new Map(sim[key].map(r => [r.occ + '|' + r.owner, r]));
  const res = { grafo_igual: ga === gd, chamadas: Object.keys(D).length, iguais_antes: 0, mudaram: 0, previsao_ok: 0, divergencias: [], sem_previsao: 0, campos_antigos_iguais_nao_mudadas: 0 };
  for (const [k, d] of Object.entries(D)) {
    const [owner, occ] = k.split('|');
    const a = A[k]; const s = simBy.get(occ + '|' + g.nodes[owner].label);
    const same = a && a.r === d.r && JSON.stringify(a.t) === JSON.stringify(d.t);
    if (same) res.iguais_antes++; else res.mudaram++;
    if (same && a.n === d.n && a.basis === d.basis && a.edge === d.edge) res.campos_antigos_iguais_nao_mudadas++;
    if (!s) { res.sem_previsao++; continue; }
    const dr = d.r === 'homonym' && d.t.length === 1 ? 'homonym' : d.r;
    if (s.after.r === dr && JSON.stringify(s.after.t) === JSON.stringify(d.t)) res.previsao_ok++;
    else res.divergencias.push({ local: s.at, texto: s.text, populacao: pool.has(occ), teste: s.test, antes: a, previsto: s.after, binario: d });
  }
  out[key] = res;
  console.log(key, JSON.stringify({ ...res, divergencias: res.divergencias.length }));
}
fs.writeFileSync('compara.json', JSON.stringify(out, null, 1));

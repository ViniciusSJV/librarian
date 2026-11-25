// Passo 2.4a, Parada 2: /api/calls e /api/used-by de todos os símbolos, antes (binário do 2.3c) e depois
// (binário do 2.4a), comparados chamada a chamada com a previsão da Parada 1 (dados/2.4/py-mudancas.json,
// locais.json e rs.json). Tudo o mais deve ficar igual, used_by inclusive.
//   node compara24a.js <dump-antes> <dump-depois> <out.json>
'use strict';
const fs = require('fs'), path = require('path');
const [antesDir, depoisDir, outFile] = process.argv.slice(2);
const P1 = path.join(__dirname, '..', '2.4');
const py = JSON.parse(fs.readFileSync(path.join(P1, 'py-mudancas.json'), 'utf8'));
const locais = JSON.parse(fs.readFileSync(path.join(P1, 'locais.json'), 'utf8'));
const rs = JSON.parse(fs.readFileSync(path.join(P1, 'rs.json'), 'utf8'));
// `path:line|text` -> { res, to } (to: the target's label, or null without a candidate).
const PREV = { 'healthchecks-25597e41': new Map(), 'renderer-9b78f3a9': new Map() };
const label = s => s ? s.replace(/ \([^)]*\)$/, '') : null;
for (const x of py.n0_mudam) {
  const how = x.binding.how;
  const res = ['external', 'modulo-chamado'].includes(x.binding.kind) ? 'no_candidate_in_project' : /import/.test(how) ? 'import' : 'scope';
  PREV['healthchecks-25597e41'].set(`${x.at}|${x.text}`, { res, to: res === 'no_candidate_in_project' ? null : label(x.binding.to) });
}
for (const x of locais) if (x.now !== 'no_candidate_in_project') PREV['healthchecks-25597e41'].set(`${x.at}|${x.text}`, { res: 'no_candidate_in_project', to: null });
for (const x of rs) if (/->escopo/.test(x.mudanca)) PREV['renderer-9b78f3a9'].set(`${x.at}|${x.text}`, { res: x.binding.how === 'mesmo módulo' ? 'scope' : 'import', to: label(x.binding.to) });

const out = {};
for (const key of Object.keys(PREV)) {
  const A = JSON.parse(fs.readFileSync(path.join(antesDir, `${key}.json`), 'utf8')).res;
  const D = JSON.parse(fs.readFileSync(path.join(depoisDir, `${key}.json`), 'utf8')).res;
  const g = JSON.parse(fs.readFileSync(path.join(depoisDir, `${key}.graph.json`), 'utf8'));
  const where = {};
  for (const e of g.edges) if (e.kind === 'observes') where[e.to] = `${e.evidence.path}:${e.evidence.start_line}`;
  const name = id => g.nodes[id]?.label ?? id;
  const s = { simbolos: Object.keys(A).length, simbolos_depois: Object.keys(D).length, used_by_iguais: 0, used_by_diferentes: [], previstas_cumpridas: [], fora_da_previsao: [], previsao_nao_cumprida: [], por_resolucao_depois: {} };
  const seen = new Set();
  for (const id of Object.keys(A)) {
    const a = A[id], d = D[id];
    if (!d) { s.fora_da_previsao.push({ simbolo: a.label, motivo: 'sumiu' }); continue; }
    if (JSON.stringify(a.used_by) === JSON.stringify(d.used_by)) s.used_by_iguais++; else s.used_by_diferentes.push(a.label);
    const group = (links, unresolved) => {
      const m = new Map();
      for (const l of links) { const k = l[0]; const v = m.get(k) ?? { res: l[3], rel: l[4], via: l[5], runs: l[6], cands: l[7], rule: l[8], to: [] }; v.to.push(name(l[2])); m.set(k, v); }
      for (const u of unresolved) m.set(u[0], { res: u[2], via: u[3], cands: u[4], to: [] });
      for (const v of m.values()) v.to.sort();
      return m;
    };
    const ga = group(a.links, a.unresolved), gd = group(d.links, d.unresolved);
    for (const occ of new Set([...ga.keys(), ...gd.keys()])) {
      const x = ga.get(occ), y = gd.get(occ);
      if (y && !y.via) s.por_resolucao_depois[y.res] = (s.por_resolucao_depois[y.res] ?? 0) + 1;
      const k = `${where[occ]}|${g.nodes[occ]?.label}`;
      const want = PREV[key].get(k);
      const same = JSON.stringify(x) === JSON.stringify(y);
      if (want) {
        const ok = y && y.res === want.res && (want.to === null ? y.to.length === 0 : y.to.length === 1 && y.to[0] === want.to);
        if (!y?.via) (ok ? s.previstas_cumpridas : s.previsao_nao_cumprida).push({ simbolo: a.label, at: where[occ], texto: g.nodes[occ]?.label, antes: x, depois: y, previsto: want });
        seen.add(k);
      } else if (!same) {
        s.fora_da_previsao.push({ simbolo: a.label, at: where[occ], texto: g.nodes[occ]?.label, antes: x, depois: y });
      }
    }
  }
  for (const k of PREV[key].keys()) if (!seen.has(k)) s.previsao_nao_cumprida.push({ at: k, motivo: 'não achada' });
  out[key] = s;
  console.log(key, `símbolos ${s.simbolos}/${s.simbolos_depois}; previstas ${PREV[key].size}: cumpridas ${s.previstas_cumpridas.length}, não cumpridas ${s.previsao_nao_cumprida.length}; fora da previsão ${s.fora_da_previsao.length}; used_by iguais ${s.used_by_iguais}, diferentes ${s.used_by_diferentes.length}`);
}
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));

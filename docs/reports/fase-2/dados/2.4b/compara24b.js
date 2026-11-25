// Passo 2.4b, Parada 2: /api/calls e used_by de todos os símbolos, antes (binário do 2.4a) e depois (2.4b), contra
// a previsão (py24b.json: as 35+139 que vão a `type`, o F23 e as 23 da regra 2). Tudo o mais deve ficar igual.
//   node compara24b.js <dump-antes> <dump-depois> <out.json>
'use strict';
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const [A0, D0, out] = process.argv.slice(2);
const prev = new Map();
for (const r of require('./py24b.json').rows) {
  if (r.over) prev.set(r.occ, { res: 'homonym', to: r.over.slice().sort() });
  else if (r.r2 && !r.r2.incerta) prev.set(r.occ, { res: 'no_candidate_in_project', to: [] });
  else if (/-> type/.test(r.mud) && r.nome !== 'to_dict') prev.set(r.occ, { res: 'type', to: [r.alvo] });
}
const res = {};
for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  const rd = d => JSON.parse(fs.readFileSync(path.join(d, `${key}.json`), 'utf8')).res;
  const gh = d => crypto.createHash('sha256').update(fs.readFileSync(path.join(d, `${key}.graph.json`))).digest('hex').slice(0, 12);
  const A = rd(A0), D = rd(D0), g = JSON.parse(fs.readFileSync(path.join(D0, `${key}.graph.json`), 'utf8'));
  const where = {}, occ = {};
  for (const e of g.edges) { if (e.kind === 'observes') occ[e.to] = `${e.evidence.path}@${e.evidence.start_byte}-${e.evidence.end_byte}`; if (e.kind === 'declares') where[e.to] = `${e.evidence.path}:${e.evidence.start_line}`; }
  const show = id => `${g.nodes[id].label} (${where[id]})`;
  const group = x => {
    const m = new Map();
    for (const l of x.links) { const v = m.get(l[0]) ?? { res: l[3], rel: l[4], via: l[5], runs: l[6], cands: l[7], rule: l[8], to: [] }; v.to.push(show(l[2])); m.set(l[0], v); }
    for (const u of x.unresolved) m.set(u[0], { res: u[2], via: u[3], cands: u[4], to: [] });
    for (const v of m.values()) v.to.sort();
    return m;
  };
  const s = { simbolos: Object.keys(A).length, grafo: [gh(A0), gh(D0)], used_by_diferentes: 0, previstas: 0, nao_cumpridas: [], fora: [], so_contagem: [], vistas: new Set(), regras: {} };
  if (Object.keys(D).length !== s.simbolos) s.fora.push('número de símbolos');
  for (const id of Object.keys(A)) {
    if (JSON.stringify(A[id].used_by) !== JSON.stringify(D[id].used_by)) s.used_by_diferentes++;
    const a = group(A[id]), d = group(D[id]);
    for (const o of new Set([...a.keys(), ...d.keys()])) {
      const x = a.get(o), y = d.get(o), w = key.startsWith('health') ? prev.get(occ[o]) : null;
      if (w) { s.vistas.add(occ[o]); const ok = y && y.res === w.res && JSON.stringify(y.to) === JSON.stringify(w.to); if (ok) { s.previstas++; s.regras[`${y.res} ${y.rule ?? ''}`] = (s.regras[`${y.res} ${y.rule ?? ''}`] ?? 0) + 1; } else s.nao_cumpridas.push({ occ: occ[o], texto: g.nodes[o].label.slice(0, 80), x, y, w }); } else if (JSON.stringify(x) !== JSON.stringify(y)) {
        const same = x && y && JSON.stringify({ ...x, cands: 0 }) === JSON.stringify({ ...y, cands: 0 });
        (same ? s.so_contagem : s.fora).push({ occ: occ[o], texto: g.nodes[o].label.slice(0, 80), x, y });
      }
    }
  }
  s.vistas = s.vistas.size;
  res[key] = s;
  console.log(key, 'símbolos', s.simbolos, 'grafo', s.grafo.join(' '), 'used_by diferentes', s.used_by_diferentes, '| previstas cumpridas', s.previstas, 'de', key.startsWith('health') ? prev.size : 0, 'vistas', s.vistas, '| não cumpridas', s.nao_cumpridas.length, '| fora', s.fora.length, '| só contagem', s.so_contagem.length, JSON.stringify(s.regras));
}
fs.writeFileSync(out, JSON.stringify(res, null, 1) + '\n');

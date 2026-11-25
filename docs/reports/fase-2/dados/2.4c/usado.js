// Passo 2.4c, Parada 1: (1) o inverso de /api/calls contra o used_by de hoje, para os métodos sobrescritos e para as
// sobrescritas; (2) o que sai do percurso (sem teto de nós) com a regra nova.   node usado.js <dump>
'use strict';
const path = require('path');
const { load } = require('../2.4/load.js');
const P = load(process.argv[2], 'healthchecks-25597e41');
const { N, d, show } = P;
const novo = new Map(require('./py24c.json').filter(x => x.cand).map(x => [x.occ, x.cand]));
const idOf = {};
for (const id in N) if (N[id].category === 'symbol') idOf[show(id)] = id;
const occKey = c => `${c.path}@${c.sb}-${c.eb}`;
// inverso: método -> chamadas que hoje chegam, por link único ou como candidata; e com a regra nova.
const hoje = {}, depois = {};
for (const c of P.calls.values()) {
  for (const t of c.targets) (hoje[t] ??= []).push(c);
  const n = novo.get(occKey(c));
  for (const t of n ? n.map(s => idOf[s]) : c.targets) (depois[t] ??= []).push(c);
}
const fam = {};
for (const [occ, cand] of novo) for (const s of cand) fam[idOf[s]] = occ;
console.log('== (1) used_by de hoje x chamadas que podem despachar, por candidata');
for (const id of Object.keys(fam)) {
  const u = d.res[id].used_by, uo = new Set(u.map(x => x[0]));
  const di = new Set((depois[id] ?? []).map(c => c.occ)), hj = (hoje[id] ?? []).filter(c => c.res !== 'homonym').length;
  const disp = [...P.calls.values()].filter(c => novo.get(occKey(c))?.includes(show(id)));
  const falta = disp.filter(c => !uo.has(c.occ)).map(c => c.path + ':' + c.line), sobra = u.filter(x => !di.has(x[0])).length;
  console.log(show(id), '| used_by hoje', u.length, '| links únicos hoje', hj, '| despacho que falta no used_by:', falta.join(', ') || '-', '| no used_by sem ser alvo possível', sobra);
}
const all = { total: 0, cobertas: 0, linkSemUso: 0 };
for (const c of P.calls.values()) if (c.lang === 'python' && c.targets.length === 1 && c.res !== 'homonym') { all.total++; d.res[c.targets[0]].used_by.some(x => x[0] === c.occ) ? all.cobertas++ : all.linkSemUso++; }
console.log('== geral (Python): links únicos do /api/calls', all.total, '; aparecem no used_by do alvo', all.cobertas, '; não aparecem', all.linkSemUso);
const por = {};
for (const c of P.calls.values()) if (c.lang === 'python' && c.targets.length === 1 && c.res !== 'homonym' && !d.res[c.targets[0]].used_by.some(x => x[0] === c.occ)) { const k = `${c.res} ${c.rule ?? ''}`; por[k] = (por[k] ?? 0) + 1; }
console.log('   por resolução', por);
// (2) percurso: símbolos alcançados (profundidade 6), antes e depois. Candidatas de homonym aparecem, sem ser seguidas.
const links = {};
for (const [o, r] of Object.entries(d.res)) for (const l of r.links) (links[o] ??= []).push(l);
const byOcc = P.calls;
function walk(seed, rule) {
  const seen = new Set([seed]); let fr = [seed];
  for (let k = 0; k < 6 && fr.length; k++) {
    const nx = [];
    for (const s of fr) for (const l of links[s] ?? []) {
      const [occ, , to, res, rel, , runs] = l;
      const n = rule && novo.get(occKey(byOcc.get(occ) ?? {}));
      if (n) { n.forEach(x => seen.add(idOf[x])); continue; }
      if (!seen.has(to)) { seen.add(to); if (res !== 'homonym' && rel === 'calls') nx.push(to); }
      if (rel === 'instantiates' && runs && !seen.has(runs)) { seen.add(runs); nx.push(runs); }
    }
    fr = nx;
  }
  return seen;
}
let mud = 0; const ex = [];
for (const id of Object.keys(d.res)) { const a = walk(id, false), b = walk(id, true); const perde = [...a].filter(x => !b.has(x)), ganha = [...b].filter(x => !a.has(x)); if (perde.length || ganha.length) { mud++; ex.push([show(id), a.size, b.size, perde.map(show), ganha.length]); } }
console.log('== (2) símbolos cujo percurso (profundidade 6) muda:', mud, '; com perda', ex.filter(x => x[3].length).length);
const pk = {}; ex.forEach(x => x[3].forEach(p => pk[p] = (pk[p] ?? 0) + 1)); console.log('   perdidos', pk);
for (const n of ['Channel.notify (', 'Slackalike.notify', 'HttpTransport._request', 'Slackalike.payload', 'Discord.notify', 'create_check (', 'Command.handle_up (', 'notify (hc/api/management']) for (const x of ex.filter(x => x[0].startsWith(n))) console.log('  ', x[0], 'nós', x[1], '->', x[2], 'perde', x[3].join(', ') || '-', 'ganha', x[4]);
console.log('   maior', Math.max(...ex.map(x => x[2])));

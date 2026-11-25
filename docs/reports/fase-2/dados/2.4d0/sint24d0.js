// Passo 2.4d-0, Parada 1: as respostas do sintético, antes (binário atual) e com o protótipo, só das chamadas com receptor.
//   node sint24d0.js <dump-antes> <dump-depois>      (dumps de dumpsint.js)
'use strict';
const fs = require('fs'), path = require('path');
const [A0, D0] = process.argv.slice(2);
const load = dir => {
  const g = JSON.parse(fs.readFileSync(path.join(dir, 'sint.graph.json'), 'utf8')), { res } = JSON.parse(fs.readFileSync(path.join(dir, 'sint.json'), 'utf8'));
  const where = {}, out = new Map();
  for (const e of g.edges) if (e.kind === 'declares') where[e.to] = `${path.basename(e.evidence.path)}:${e.evidence.start_line}`;
  for (const id of Object.keys(res)) {
    const add = (occ, from, v, to) => {
      if (g.nodes[occ].kind !== 'method_call_observed' || from !== id) return;
      const k = `${g.nodes[id].label} | ${g.nodes[occ].label.split('\n')[0]}`, o = out.get(k) ?? { v, to: [] };
      if (to) o.to.push(`${g.nodes[to].label} (${where[to]})`);
      out.set(k, o);
    };
    for (const l of res[id].links) add(l[0], l[1], `${l[3]}${l[8] ? ` [${l[8]}]` : ''} (${l[7]})`, l[2]);
    for (const u of res[id].unresolved) add(u[0], u[1], u[2], null);
  }
  return out;
};
const A = load(A0), D = load(D0), f = x => x ? `${x.v} ${x.to.sort().join('; ')}` : '—';
for (const k of [...A.keys()].sort()) console.log(`${f(A.get(k)) === f(D.get(k)) ? ' ' : '*'} ${k}\n      antes:  ${f(A.get(k))}\n      depois: ${f(D.get(k))}`);

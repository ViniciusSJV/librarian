// Step 2.3a: /api/calls before (dump after 2.3a-0) and after (dump of 2.3a), call by call, compared with the
// simulator's forecast (sim23.js). node compara23.js <before-dir> <after-dir> <sim.json> <out.json>
'use strict';
const fs = require('fs');
const [a, b, simFile, outFile] = process.argv.slice(2);
const sim = JSON.parse(fs.readFileSync(simFile, 'utf8'));
const forecast = new Map(sim.mudancas.map(m => [m.key, m]));
const instancia = new Set([...sim.instancia.hc, ...sim.instancia.rt].map(x => x.key));
const show = t => `${t.label} (${t.at})`;
function index(dir, key) {
  const d = JSON.parse(fs.readFileSync(`${dir}/${key}.calls.json`, 'utf8'));
  const m = new Map();
  for (const r of Object.values(d.res)) {
    for (const l of r.links) {
      const k = `${l.path}@${l.sb}-${l.eb}`;
      const v = m.get(k) ?? { resolution: l.resolution, candidates: l.candidates, basis: l.basis, targets: [], relation: l.relation ?? 'calls', text: l.text, line: l.line, path: l.path };
      v.targets.push(show(l.to)); m.set(k, v);
    }
    for (const u of r.unresolved) {
      const k = `${u.path}@${u.sb}-${u.eb}`;
      if (u.reason === 'node_limit') { const v = m.get(k) ?? { resolution: u.resolution, candidates: u.candidates, targets: [], relation: null, text: u.text, line: u.line, path: u.path }; v.targets.push(show(u.to)); m.set(k, v); continue; }
      m.set(k, { resolution: u.reason, candidates: u.candidates, targets: [], relation: null, text: u.text, line: u.line, path: u.path });
    }
  }
  for (const v of m.values()) v.targets.sort();
  return m;
}
const out = {};
for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  // the dump script keeps only the first line of the text; the dump-calls output has `relation` only after 2.3a
  const A = index(a, key), B = index(b, key);
  const r = { chamadas: B.size, faltam_no_depois: [...A.keys()].filter(k => !B.has(k)).length, novas_no_depois: [...B.keys()].filter(k => !A.has(k)).length,
    mudam: 0, previstas_e_iguais: 0, previstas_diferentes: [], nao_previstas: [], previstas_que_nao_mudaram: [], so_candidates: 0, instancia: { previstas: 0, iguais: 0, divergem: [] } };
  for (const [k, x] of B) {
    const y = A.get(k); if (!y) continue;
    const same = x.resolution === y.resolution && JSON.stringify(x.targets) === JSON.stringify(y.targets);
    const f = forecast.get(k);
    if (x.relation === 'instantiates' || instancia.has(k)) {
      r.instancia.previstas += instancia.has(k) ? 1 : 0;
      if (instancia.has(k) && x.relation === 'instantiates') r.instancia.iguais++;
      else r.instancia.divergem.push({ k, text: x.text, relation: x.relation, previsto: instancia.has(k) });
    }
    if (same) {
      if (x.candidates !== y.candidates) r.so_candidates++;
      if (f) r.previstas_que_nao_mudaram.push({ k, text: x.text, agora: `${x.resolution} (${x.candidates})`, previsto: `${f.depois.resolution} -> ${f.depois.targets.join(' | ')}` });
      continue;
    }
    r.mudam++;
    if (!f) { r.nao_previstas.push({ k, text: x.text, antes: `${y.resolution} (${y.candidates}) ${y.targets.join(' | ')}`, depois: `${x.resolution} (${x.candidates}) ${x.targets.join(' | ')}` }); continue; }
    if (f.depois.resolution === x.resolution && JSON.stringify([...f.depois.targets].sort()) === JSON.stringify(x.targets)) r.previstas_e_iguais++;
    else r.previstas_diferentes.push({ k, text: x.text, previsto: `${f.depois.resolution} ${f.depois.targets.join(' | ')}`, binario: `${x.resolution} ${x.targets.join(' | ')}` });
  }
  out[key] = r;
  console.log(key, JSON.stringify({ chamadas: r.chamadas, mudam: r.mudam, previstas_e_iguais: r.previstas_e_iguais, previstas_diferentes: r.previstas_diferentes.length, nao_previstas: r.nao_previstas.length, previstas_que_nao_mudaram: r.previstas_que_nao_mudaram.length, so_candidates: r.so_candidates, instancia: { previstas: r.instancia.previstas, iguais: r.instancia.iguais, divergem: r.instancia.divergem.length }, faltam: r.faltam_no_depois, novas: r.novas_no_depois }));
}
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));

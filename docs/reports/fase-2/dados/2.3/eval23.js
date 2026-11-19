// Step 2.3, Parada 1: the forecast of subpasso 2.3a for the gate: sample, population and facts,
// from the simulator's output over the dump. node eval23.js <dump-dir> <sim.json> <gate resultado.json>
'use strict';
const fs = require('fs'), path = require('path');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const [dir, simFile, gateFile] = process.argv.slice(2);
const sim = JSON.parse(fs.readFileSync(simFile, 'utf8'));
const gate = JSON.parse(fs.readFileSync(gateFile, 'utf8'));
const changed = new Map(sim.mudancas.map(m => [m.key, m]));
const inst = new Set([...sim.instancia.hc, ...sim.instancia.rt].map(x => x.key));
const CATS = ['correta', 'link_falso', 'candidatos_falsos', 'ambigua', 'sem_candidato'];
const LINKED = new Set(['unique', 'receiver', 'type', 'trait_default', 'import', 'chosen']);
function classify(a, truth) {
  const right = truth.alvo && a.targets.includes(truth.alvo);
  if (a.resolution === 'no_candidate_in_project') return 'sem_candidato';
  if (a.resolution === 'above_candidate_limit') return truth.alvo ? 'ambigua' : 'candidatos_falsos';
  if (a.resolution === 'homonym') return right ? 'ambigua' : 'candidatos_falsos';
  if (a.resolution === 'type' && a.targets.length > 1) return right ? 'ambigua' : 'candidatos_falsos';
  return a.targets.length === 1 && right ? 'correta' : 'link_falso';
}
const out = { amostra: {}, mudam: [], populacao: {}, fatos: [] };
for (const [lang, file] of [['hc', 'amostra-hc.json'], ['rt', 'amostra-rt.json']]) {
  const fx = JSON.parse(fs.readFileSync(path.join(REPO, 'tools/precisao/fixtures', file), 'utf8')).chamadas;
  const now = Object.fromEntries(gate.projetos[lang].amostra.map(s => [s.id, s.categoria]));
  const count = {};
  for (const c of fx) {
    let cat = now[c.id];
    const m = changed.get(c.ocorrencia);
    if (m) {
      const cat2 = classify(m.depois, c.verdade);
      out.mudam.push({ id: c.id, texto: c.texto, de: cat, para: cat2, regra: m.rule, tipo: m.tipo, antes: `${m.antes.resolution} (${m.antes.candidates})`, depois: `${m.depois.resolution}${m.depois.targets.length ? ' → ' + m.depois.targets.join(' | ') : ''}`, sobrescrito: m.depois.sobrescrito_em });
      cat = cat2;
    }
    if (inst.has(c.ocorrencia)) out.mudam.push({ id: c.id, texto: c.texto, de: cat, para: cat, nota: 'relação instancia' });
    count[cat] = (count[cat] ?? 0) + 1;
  }
  out.amostra[lang] = { antes: gate.projetos[lang].amostra_contagem, depois: count };
}
// Population: the gate's pool, from the graph of the dump.
for (const [lang, key] of [['hc', 'healthchecks-25597e41'], ['rt', 'renderer-9b78f3a9']]) {
  const g = JSON.parse(fs.readFileSync(path.join(dir, `${key}.graph.json`), 'utf8'));
  const d = JSON.parse(fs.readFileSync(path.join(dir, `${key}.calls.json`), 'utf8'));
  const res = new Map();
  for (const r of Object.values(d.res)) {
    for (const l of r.links) res.set(`${l.path}@${l.sb}-${l.eb}`, l.resolution);
    for (const u of r.unresolved) if (u.reason !== 'node_limit') res.set(`${u.path}@${u.sb}-${u.eb}`, u.reason);
  }
  const before = {}, after = {};
  let n = 0;
  for (const e of g.edges) {
    if (e.kind !== 'observes') continue;
    const o = g.nodes[e.to], owner = g.nodes[e.from];
    if (o.kind !== 'method_call_observed' || !/function|method/.test(owner.kind)) continue;
    if (/(^|\/)tests?\/|test_|_test\.|\/benches\//.test(e.evidence.path) || owner.label.includes('test')) continue;
    if (!/^[A-Za-z_][\w.]*\.\w+\(/.test(o.label) || /^self\.\w+\(/.test(o.label)) continue;
    const k = `${e.evidence.path}@${e.evidence.start_byte}-${e.evidence.end_byte}`;
    const r = res.get(k); n++;
    before[r] = (before[r] ?? 0) + 1;
    const m = changed.get(k), r2 = m ? m.depois.resolution : r;
    after[r2] = (after[r2] ?? 0) + 1;
  }
  out.populacao[lang] = { total: n, antes: before, depois: after };
}
// Facts F1–F26: the asserted calls, by line and text, among the changed and instancia calls.
const fx = JSON.parse(fs.readFileSync(path.join(REPO, 'tools/precisao/fixtures/fatos.json'), 'utf8'));
for (const f of fx.fatos) for (const c of f.chamadas) {
  const m = sim.mudancas.find(x => x.line === c.linha && x.text.startsWith(c.texto));
  const i = sim.instancia.hc.find(x => x.text.startsWith(c.texto) && x.key && x.line === c.linha);
  if (m) out.fatos.push({ fato: f.id, linha: c.linha, texto: c.texto, verdade: c.verdade, antes: `${m.antes.resolution} (${m.antes.candidates})`, depois: `${m.depois.resolution} → ${m.depois.targets.join(' | ')}` });
}
console.log(JSON.stringify(out.amostra, null, 1));
console.log(out.mudam);
console.log(JSON.stringify(out.populacao, null, 1));
console.log(out.fatos);
fs.writeFileSync(simFile.replace('.json', '-eval.json'), JSON.stringify(out, null, 1));

// node eval.js <sim.json> : sample (by category, before/after), pool counts, facts of interest.
'use strict';
const fs = require('fs'), path = require('path');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const L = require(REPO + '/tools/precisao/lib.js');
const d = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const LINKED = new Set(['unique', 'receiver', 'type', 'trait_default', 'import', 'chosen']);
function classify(a, truth) {
  const right = truth.alvo && a.t.includes(truth.alvo);
  if (a.r === 'no_candidate_in_project') return 'sem_candidato';
  if (a.r === 'above_candidate_limit') return truth.alvo ? 'ambigua' : 'candidatos_falsos';
  if (a.r === 'homonym') return right ? 'ambigua' : 'candidatos_falsos';
  if (LINKED.has(a.r)) return a.t.length === 1 && right ? 'correta' : 'link_falso';
}
function factCls(a, truth) {
  if (!a || ['no_candidate_in_project', 'above_candidate_limit'].includes(a.r)) return 'nao_depende';
  const right = truth.alvo && a.t.includes(truth.alvo);
  if (a.r === 'homonym') return right ? 'suspeito' : 'errado';
  return a.t.length === 1 && right ? 'correto' : 'errado';
}
const KEYS = { hc: 'healthchecks-25597e41', rt: 'renderer-9b78f3a9' };
const out = { amostra: {}, mudancas: [], pop: {}, fatos: [] };
for (const [lang, key] of Object.entries(KEYS)) {
  const rows = d[key]; const byKey = new Map(rows.map(r => [r.key, r]));
  const fx = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/amostra-${lang}.json`, 'utf8'));
  const cnt = { antes: {}, depois: {} };
  for (const c of fx.chamadas) {
    const r = byKey.get(c.ocorrencia);
    if (!r) { console.log('nao achada', c.id); continue; }
    const a = classify(r.before, c.verdade), b = classify(r.after, c.verdade);
    cnt.antes[a] = (cnt.antes[a] ?? 0) + 1; cnt.depois[b] = (cnt.depois[b] ?? 0) + 1;
    if (a !== b || r.before.r !== r.after.r) out.mudancas.push(`${c.id} ${c.estrato} ${r.text.slice(0, 40)}: ${r.before.r}(${r.before.t.length}) ${a} -> ${r.after.r}(${r.after.t.length}) ${b}`);
  }
  out.amostra[lang] = cnt;
  // pool
  const graph = JSON.parse(fs.readFileSync(path.join(path.dirname(process.argv[2]), 'antes', `${key}.graph.json`), 'utf8'));
  const pool = new Set(L.pool({ g: graph }).map(p => p.occ.id));
  const P = { antes: {}, depois: {}, mf_antes: 0, mf_depois: 0 };
  for (const r of rows) {
    if (!pool.has(r.occ)) continue;
    P.antes[r.before.r] = (P.antes[r.before.r] ?? 0) + 1; P.depois[r.after.r] = (P.depois[r.after.r] ?? 0) + 1;
    const fn = s => s.r !== 'homonym' && s.t.length === 1 && /^[^ ]+ \(/.test(s.t[0]) && !s.t[0].includes('.') && !s.t[0].includes('::') && !s.t[0].startsWith('impl');
    if (fn(r.before)) P.mf_antes++; if (fn(r.after)) P.mf_depois++;
  }
  out.pop[lang] = P;
  if (lang === 'hc') {
    const ff = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/fatos.json`, 'utf8'));
    const rec = fs.readFileSync(`${REPO}/.studio/projects/${key}/facts.jsonl`, 'utf8').trim().split('\n').map(JSON.parse);
    for (const f of ff.fatos) {
      const fr = rec.find(x => x.id === f.id); if (!fr) continue;
      const calls = f.chamadas.map(c => {
        const found = rows.filter(r => r.at === `${fr.path}:${c.linha}` && (c.texto.endsWith('(') ? r.text.startsWith(c.texto) : r.text === c.texto));
        const W = ['nao_depende', 'correto', 'suspeito', 'errado'];
        const worst = k => found.map(r => factCls(r[k], c.verdade)).sort((x, y) => W.indexOf(y) - W.indexOf(x))[0] ?? 'nao achada';
        return `${c.texto}@${c.linha}: ${found.map(r => r.before.r + '(' + r.before.t.length + ')').join(',')} ${worst('before')} -> ${found.map(r => r.after.r + '(' + r.after.t.length + ')' + (r.after.r !== 'homonym' && r.after.t[0] ? ' ' + r.after.t[0] : '')).join(',')} ${worst('after')}`;
      });
      out.fatos.push(`${f.id}: ${calls.join(' | ')}`);
    }
  }
}
console.log(JSON.stringify(out, null, 1));

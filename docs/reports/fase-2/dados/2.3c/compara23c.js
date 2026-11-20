// Step 2.3c, Parada 2: /api/calls and /api/used-by of every symbol, before (2.3b binary) and after (2.3c
// binary), compared call by call with the prediction: the 8 Python calls listed below change from
// `homonym` to `type`; everything else stays the same, used_by included. The new `type_rule` of the
// links with `basis: type` is counted apart.
// Usage: node compara23c.js <dump-antes> <dump-depois> <out.json>
'use strict';
const fs = require('fs'), path = require('path');
const [antesDir, depoisDir, outFile] = process.argv.slice(2);
// Occurrence (path@start-end) -> the target the 2.3c gives it (Parada 1, recount; Q1 case b for HC-110).
const PREVISAO = {
  'hc/integrations/slack/views.py@3241-3254': 'Response.json',
  'hc/integrations/discord/views.py@2415-2428': 'Response.json',
  'hc/front/views.py@17740-17770': 'Project.num_checks_available',
  'hc/front/views.py@36857-36894': 'Project.num_checks_available',
  'hc/front/views.py@14049-14075': 'Profile.projects',
  'hc/integrations/telegram/views.py@4229-4255': 'Profile.projects',
  'hc/api/views.py@16323-16361': 'Project.num_checks_available',
  'hc/accounts/views.py@18237-18252': 'Member.can_accept',
};
const keys = ['healthchecks-25597e41', 'renderer-9b78f3a9'];
const out = {};
for (const key of keys) {
  const A = JSON.parse(fs.readFileSync(path.join(antesDir, `${key}.json`), 'utf8')).res;
  const D = JSON.parse(fs.readFileSync(path.join(depoisDir, `${key}.json`), 'utf8')).res;
  const g = JSON.parse(fs.readFileSync(path.join(depoisDir, `${key}.graph.json`), 'utf8'));
  const where = {};
  for (const e of g.edges) if (e.kind === 'observes') where[e.to] = `${e.evidence.path}@${e.evidence.start_byte}-${e.evidence.end_byte}`;
  const label = id => g.nodes[id]?.label ?? id;
  const s = { simbolos: Object.keys(A).length, simbolos_depois: Object.keys(D).length, type_rule: {}, used_by_iguais: 0, used_by_diferentes: [], previstas_cumpridas: [], fora_da_previsao: [], previsao_nao_cumprida: [] };
  const seen = new Set();
  for (const id of Object.keys(A)) {
    const a = A[id], d = D[id];
    if (!d) { s.fora_da_previsao.push({ simbolo: a.label, motivo: 'sumiu' }); continue; }
    if (JSON.stringify(a.used_by) === JSON.stringify(d.used_by)) s.used_by_iguais++; else s.used_by_diferentes.push(a.label);
    for (const l of d.links) if (l[8]) s.type_rule[l[8]] = (s.type_rule[l[8]] ?? 0) + 1;
    // Links without the new type_rule, grouped by occurrence.
    const group = (links, unresolved) => {
      const m = new Map();
      for (const l of links) { const k = l[0]; const v = m.get(k) ?? { res: l[3], rel: l[4], via: l[5], to: [] }; v.to.push(label(l[2])); m.set(k, v); }
      for (const u of unresolved) m.set(u[0], { res: u[2], via: u[3], to: [] });
      for (const v of m.values()) v.to.sort();
      return m;
    };
    const ga = group(a.links, a.unresolved), gd = group(d.links, d.unresolved);
    for (const occ of new Set([...ga.keys(), ...gd.keys()])) {
      const x = ga.get(occ), y = gd.get(occ), at = where[occ];
      const same = JSON.stringify(x) === JSON.stringify(y);
      const want = PREVISAO[at];
      if (want) {
        const ok = y && y.res === 'type' && y.to.length === 1 && y.to[0] === want && x && x.res === 'homonym';
        (ok ? s.previstas_cumpridas : s.previsao_nao_cumprida).push({ simbolo: a.label, at, antes: x, depois: y });
        seen.add(at);
      } else if (!same) {
        s.fora_da_previsao.push({ simbolo: a.label, at, antes: x, depois: y });
      }
    }
  }
  if (key.startsWith('healthchecks')) for (const at of Object.keys(PREVISAO)) if (!seen.has(at)) s.previsao_nao_cumprida.push({ at, motivo: 'não achada' });
  out[key] = s;
  console.log(key, `símbolos ${s.simbolos}/${s.simbolos_depois}; previstas cumpridas ${s.previstas_cumpridas.length}; não cumpridas ${s.previsao_nao_cumprida.length}; fora da previsão ${s.fora_da_previsao.length}; used_by iguais ${s.used_by_iguais}, diferentes ${s.used_by_diferentes.length}; type_rule`, s.type_rule);
}
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));

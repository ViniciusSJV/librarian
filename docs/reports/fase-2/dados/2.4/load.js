// Passo 2.4, Parada 1 (diagnóstico; não é código de produção). Carrega o grafo e o dump do
// /api/calls (dump23c.js) de um projeto e dá, por ocorrência de chamada, a resposta de hoje.
'use strict';
const fs = require('fs'), path = require('path');

function load(dir, key) {
  const g = JSON.parse(fs.readFileSync(path.join(dir, `${key}.graph.json`), 'utf8'));
  const d = JSON.parse(fs.readFileSync(path.join(dir, `${key}.json`), 'utf8'));
  const N = g.nodes;
  const declared = {}, parent = {}, children = {}, observes = {}, observer = {}, cands = {};
  for (const e of g.edges) {
    if (e.kind === 'declares') declared[e.to] = e.evidence;
    else if (e.kind === 'contains') { parent[e.to] = e.from; (children[e.from] ??= []).push(e.to); }
    else if (e.kind === 'observes') { (observes[e.from] ??= []).push(e.to); observer[e.to] ??= { from: e.from, ev: e.evidence }; }
    else if (e.kind === 'name_candidate') (cands[e.from] ??= []).push(e.to);
  }
  const where = id => declared[id] ? `${declared[id].path}:${declared[id].start_line}` : '?';
  const show = id => `${N[id].label} (${where(id)})`;
  const last = l => l.split(/::|\./).pop();
  // Calls: one entry per occurrence (the first answer that is not a field's `via` copy).
  const calls = new Map();
  for (const [owner, r] of Object.entries(d.res)) {
    for (const [occ, from, to, res, rel, via, runs, nc, rule] of r.links) {
      if (via) continue;
      const c = calls.get(occ) ?? { occ, owner: from, res, rel, runs, cands: nc, rule, targets: [] };
      if (c.owner !== from) continue;
      c.targets.push(to); calls.set(occ, c);
    }
    for (const [occ, from, reason, via, nc] of r.unresolved) {
      if (via || calls.has(occ)) continue;
      calls.set(occ, { occ, owner: from, res: reason, cands: nc, targets: [] });
    }
  }
  for (const c of calls.values()) {
    const o = observer[c.occ];
    c.text = N[c.occ].label; c.kind = N[c.occ].kind; c.lang = N[c.occ].language;
    c.path = o?.ev.path; c.line = o?.ev.start_line; c.sb = o?.ev.start_byte; c.eb = o?.ev.end_byte;
    c.amb = N[c.occ].ambiguous_candidates;
    c.test = /(^|\/)tests?\/|(^|\/)test_|_test\.|\/test\.py$/.test(c.path ?? '');
  }
  return { g, N, d, declared, parent, children, observes, observer, cands, where, show, last, calls };
}

const isTestPath = p => /(^|\/)tests?\/|(^|\/)test_|_test\.|\/test\.py$/.test(p ?? '');
module.exports = { load, isTestPath };

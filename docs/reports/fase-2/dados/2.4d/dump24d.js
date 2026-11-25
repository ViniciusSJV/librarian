// Passo 2.4d, Parada 2: o corpo cru do /api/calls (profundidade 1, e 3 numa amostra) e do /api/used-by de todos os símbolos.
//   node dump24d.js <saida> <bin> [calls|uses|tudo] [so-hash]
'use strict';
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const [out, bin, what = 'tudo'] = process.argv.slice(2);
const keys = ['healthchecks-25597e41', 'renderer-9b78f3a9'];
fs.mkdirSync(out, { recursive: true });
(async () => {
  const studio = path.join(L.REPO, '.studio'), before = keys.map(k => L.hashProject(studio, k).sha256);
  const root = L.copyStudio(studio, keys), s = await L.startStudio(bin, root), base = `http://127.0.0.1:${s.port}`;
  const raw = async (method, url, body) => { const r = await fetch(base + url, { method, headers: body ? { 'content-type': 'application/json' } : {}, body: body && JSON.stringify(body) }); const t = await r.text(); if (!r.ok) throw new Error(`${url}: ${t}`); return t; };
  try {
    for (const key of keys) {
      const { id: run } = await s.api('POST', '/api/generate', { key });
      const G = L.loadGraph(root, key, run), ids = Object.keys(G.g.nodes).filter(id => G.g.nodes[id].category === 'symbol').sort();
      fs.copyFileSync(path.join(root, '.studio', 'projects', key, 'runs', run, 'graph-v2.json'), path.join(out, `${key}.graph.json`));
      const hash = {}, calls = {}, uses = {}, all = crypto.createHash('sha256'); let i = 0, msU = [], t0 = Date.now();
      for (const id of ids) {
        if (what !== 'uses') {
          for (const depth of i % 25 === 0 ? [1, 3] : [1]) {
            const t = await raw('POST', '/api/calls', { key, run, node: id, depth, chosen: {} });
            hash[`${id}|${depth}`] = crypto.createHash('sha256').update(t).digest('hex'); all.update(t);
            if (depth === 1) { const c = JSON.parse(t); calls[id] = { links: c.links.map(l => [l.occurrence, l.from, l.to, l.resolution, l.relation, l.via ?? null, l.runs ?? null, l.basis, l.abstract_method ?? null]), unresolved: c.unresolved.map(u => [u.occurrence, u.from, u.reason, u.to ?? null, u.resolution ?? null]) }; }
          }
        }
        if (what !== 'calls') { const a = performance.now(); uses[id] = JSON.parse(await raw('GET', `/api/used-by?key=${key}&run=${run}&node=${encodeURIComponent(id)}&depth=1`)); msU.push(performance.now() - a); }
        i++;
      }
      const lines = [`{"run":"${run}","hash":${JSON.stringify(hash)},"total":"${all.digest('hex')}"}`];
      fs.writeFileSync(path.join(out, `${key}.hash.json`), lines[0]);
      fs.writeFileSync(path.join(out, `${key}.calls.json`), JSON.stringify(calls));
      const w = fs.openSync(path.join(out, `${key}.uses.jsonl`), 'w'); for (const id in uses) fs.writeSync(w, JSON.stringify([id, uses[id]]) + '\n'); fs.closeSync(w);
      const m = msU.slice(1).sort((a, b) => a - b);
      console.log(key, run, ids.length, 'símbolos;', ((Date.now() - t0) / 1000).toFixed(0), 's; used-by: primeiro', msU[0]?.toFixed(0), 'ms, mediana', m[m.length >> 1]?.toFixed(1), 'ms, máx', m.at(-1)?.toFixed(0));
    }
  } finally { s.stop(); }
  console.log('.studio original', keys.every((k, i) => L.hashProject(studio, k).sha256 === before[i]) ? 'inalterado' : 'MUDOU');
})().catch(e => { console.error(e); process.exit(2); });

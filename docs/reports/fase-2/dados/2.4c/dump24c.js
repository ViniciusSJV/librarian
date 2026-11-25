// O dump23c.js (2.3c: /api/calls e /api/used-by, profundidade 1, de todos os símbolos) com o tempo de cada /api/calls.
//   node dump24c.js <out> <bin>
'use strict';
const fs = require('fs'), path = require('path');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const [out, bin] = process.argv.slice(2);
const keys = ['healthchecks-25597e41', 'renderer-9b78f3a9'];
fs.mkdirSync(out, { recursive: true });
(async () => {
  await L.withStudio({ studio: path.join(L.REPO, '.studio'), bin, keys }, async ({ api, root }) => {
    for (const key of keys) {
      const { id: run } = await api('POST', '/api/generate', { key });
      const G = L.loadGraph(root, key, run);
      fs.copyFileSync(path.join(root, '.studio', 'projects', key, 'runs', run, 'graph-v2.json'), path.join(out, `${key}.graph.json`));
      const res = {}, ms = {}, t0 = performance.now();
      let calls = 0;
      const ids = Object.keys(G.g.nodes).filter(id => G.g.nodes[id].category === 'symbol').sort();
      for (const id of ids) {
        const a = performance.now();
        const c = await api('POST', '/api/calls', { key, run, node: id, depth: 1, chosen: {} });
        ms[id] = performance.now() - a;
        calls += ms[id];
        const u = await api('GET', `/api/used-by?key=${key}&run=${run}&node=${encodeURIComponent(id)}&depth=1`);
        res[id] = {
          label: G.g.nodes[id].label, kind: G.g.nodes[id].kind, at: G.where(id),
          links: c.links.map(l => [l.occurrence, l.from, l.to, l.resolution, l.relation, l.via ?? null, l.runs ?? null, l.candidates, l.type_rule ?? null]),
          unresolved: c.unresolved.map(x => [x.occurrence, x.from, x.reason, x.via ?? null, x.candidates]),
          used_by: u.links.map(l => [l.occurrence, l.from, l.to, l.candidates]),
        };
      }
      fs.writeFileSync(path.join(out, `${key}.json`), JSON.stringify({ run, res }));
      fs.writeFileSync(path.join(out, `${key}.ms.json`), JSON.stringify({ total_s: (performance.now() - t0) / 1000, calls_s: calls / 1000, ms }));
      console.log(key, run, ids.length, 'símbolos; total', ((performance.now() - t0) / 1000).toFixed(0), 's; /api/calls', (calls / 1000).toFixed(0), 's');
    }
  });
})().catch(e => { console.error(e); process.exit(2); });

// Step 2.3c, Parada 2 (dump23b.js com o type_rule de cada link): /api/calls (depth 1) and /api/used-by (depth 1) of every symbol of both
// projects, on a fresh run of the given binary over a copy of .studio. Usage: node dump23b.js <out> <bin>
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
      fs.copyFileSync(path.join(root, ".studio", "projects", key, "runs", run, "graph-v2.json"), path.join(out, `${key}.graph.json`));
      const res = {};
      const ids = Object.keys(G.g.nodes).filter(id => G.g.nodes[id].category === 'symbol').sort();
      for (const id of ids) {
        const c = await api('POST', '/api/calls', { key, run, node: id, depth: 1, chosen: {} });
        const u = await api('GET', `/api/used-by?key=${key}&run=${run}&node=${encodeURIComponent(id)}&depth=1`);
        res[id] = {
          label: G.g.nodes[id].label, kind: G.g.nodes[id].kind, at: G.where(id),
          links: c.links.map(l => [l.occurrence, l.from, l.to, l.resolution, l.relation, l.via ?? null, l.runs ?? null, l.candidates, l.type_rule ?? null]),
          unresolved: c.unresolved.map(x => [x.occurrence, x.from, x.reason, x.via ?? null, x.candidates]),
          used_by: u.links.map(l => [l.occurrence, l.from, l.to, l.candidates]),
        };
      }
      fs.writeFileSync(path.join(out, `${key}.json`), JSON.stringify({ run, res }));
      console.log(key, run, ids.length, 'símbolos');
    }
  });
})().catch(e => { console.error(e); process.exit(2); });

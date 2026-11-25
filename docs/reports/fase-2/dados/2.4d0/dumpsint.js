// Dump do /api/calls de um projeto sintetico (formato do dump23c.js).   node dumpsint.js <projeto> <out> <bin>
'use strict';
const fs = require('fs'), path = require('path'), os = require('os');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const [src, out, bin] = process.argv.slice(2);
(async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'sint24c-'));
  fs.mkdirSync(path.join(root, '.studio', 'projects'), { recursive: true });
  fs.mkdirSync(out, { recursive: true });
  const s = await L.startStudio(bin, root);
  try {
    const { key } = await s.api('POST', '/api/project', { root: src });
    const { id: run } = await s.api('POST', '/api/generate', { key });
    const G = L.loadGraph(root, key, run), res = {};
    fs.copyFileSync(path.join(root, '.studio', 'projects', key, 'runs', run, 'graph-v2.json'), path.join(out, 'sint.graph.json'));
    for (const id of Object.keys(G.g.nodes).filter(i => G.g.nodes[i].category === 'symbol').sort()) {
      const c = await s.api('POST', '/api/calls', { key, run, node: id, depth: 1, chosen: {} });
      res[id] = { links: c.links.map(l => [l.occurrence, l.from, l.to, l.resolution, l.relation, l.via ?? null, l.runs ?? null, l.candidates, l.type_rule ?? null]), unresolved: c.unresolved.map(x => [x.occurrence, x.from, x.reason, x.via ?? null, x.candidates]), used_by: [] };
    }
    fs.writeFileSync(path.join(out, 'sint.json'), JSON.stringify({ run, res }));
  } finally { s.stop(); }
})().catch(e => { console.error(e); process.exit(2); });

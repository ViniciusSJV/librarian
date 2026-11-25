// Passo 2.4c, Parada 1: (1) dump do sintético do 2.4c e do portão com o binário de hoje, no formato do dump23c.js;
// (2) tempo do /api/calls (profundidade 1) no Healthchecks, com e sem a leitura das subclasses.   node tempo.js <out>
'use strict';
const fs = require('fs'), path = require('path'), os = require('os');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const out = process.argv[2], bin = path.join(L.REPO, 'target/release');
const med = xs => xs.slice().sort((a, b) => a - b)[Math.floor(xs.length / 2)];
(async () => {
  for (const [nome, src] of [['sint', path.join(__dirname, 'sintetico')], ['portao', path.join(L.REPO, 'tools/precisao/sinteticos/py')]]) {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'sint24c-')), dir = path.join(out, nome);
    fs.mkdirSync(path.join(root, '.studio', 'projects'), { recursive: true });
    fs.mkdirSync(dir, { recursive: true });
    const s = await L.startStudio(bin, root);
    try {
      const { key } = await s.api('POST', '/api/project', { root: src });
      const { id: run } = await s.api('POST', '/api/generate', { key });
      const G = L.loadGraph(root, key, run), res = {};
      fs.copyFileSync(path.join(root, '.studio', 'projects', key, 'runs', run, 'graph-v2.json'), path.join(dir, 'sint.graph.json'));
      for (const id of Object.keys(G.g.nodes).filter(i => G.g.nodes[i].category === 'symbol').sort()) {
        const c = await s.api('POST', '/api/calls', { key, run, node: id, depth: 1, chosen: {} });
        res[id] = { links: c.links.map(l => [l.occurrence, l.from, l.to, l.resolution, l.relation, l.via ?? null, l.runs ?? null, l.candidates, l.type_rule ?? null]), unresolved: c.unresolved.map(x => [x.occurrence, x.from, x.reason, x.via ?? null, x.candidates]), used_by: [] };
      }
      fs.writeFileSync(path.join(dir, 'sint.json'), JSON.stringify({ run, res }));
    } finally { s.stop(); }
  }
  const key = 'healthchecks-25597e41';
  await L.withStudio({ studio: path.join(L.REPO, '.studio'), bin, keys: [key] }, async ({ api, root }) => {
    const { id: run } = await api('POST', '/api/generate', { key });
    const G = L.loadGraph(root, key, run);
    const pick = [['precisa hoje (abstrato)', 'Channel.notify'], ['não precisa hoje', 'HttpTransport._request'], ['não precisa hoje', 'HttpTransport.post'], ['não precisa hoje', 'Slackalike.payload'], ['não precisa hoje', 'Project.num_checks_available'], ['sem tipo', 'sortchecks'], ['precisa hoje (super)', 'WebhookForm.clean'], ['precisa hoje (super de teste)', 'AddPdTestCase.setUp']];
    for (const [c, label] of pick) {
      const id = Object.keys(G.g.nodes).find(i => G.g.nodes[i].category === 'symbol' && G.g.nodes[i].label === label);
      if (!id) { console.log(c, label, 'não achado'); continue; }
      const t = [];
      for (let i = 0; i < 9; i++) { const a = process.hrtime.bigint(); await api('POST', '/api/calls', { key, run, node: id, depth: 1, chosen: {} }); t.push(Number(process.hrtime.bigint() - a) / 1e6); }
      console.log(c.padEnd(30), label.padEnd(32), 'mediana', med(t).toFixed(0), 'ms  mín', Math.min(...t).toFixed(0), 'máx', Math.max(...t).toFixed(0));
    }
  });
})().catch(e => { console.error(e); process.exit(2); });

// Step 2.3b, Parada 1: what /api/calls and /api/used-by say today for the calls that run outside
// the body of their owner (decorators, defaults, field initializers in __init__), on a fresh run
// of the current binary over a copy of .studio. Usage: node q23b.js <topo.jsonl> <out-dir>
'use strict';
const fs = require('fs'), path = require('path');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const [topoFile, out] = process.argv.slice(2);
const rows = fs.readFileSync(topoFile, 'utf8').trim().split('\n').map(JSON.parse);
const key = 'healthchecks-25597e41';
fs.mkdirSync(out, { recursive: true });
(async () => {
  await L.withStudio({ studio: path.join(L.REPO, '.studio'), bin: path.join(L.REPO, 'target/release'), keys: [key] }, async ({ api, root }) => {
    const { id: run } = await api('POST', '/api/generate', { key });
    const G = L.loadGraph(root, key, run);
    fs.copyFileSync(path.join(root, '.studio', 'projects', key, 'runs', run, 'graph-v2.json'), path.join(out, 'graph.json'));
    const occ = new Map(); // path@start -> {id, owner}
    for (const e of G.g.edges) {
      if (e.kind !== 'observes') continue;
      const n = G.g.nodes[e.to];
      if (!/call_observed$/.test(n.kind)) continue;
      occ.set(`${e.evidence.path}@${e.evidence.start_byte}`, { id: e.to, owner: e.from, end: e.evidence.end_byte });
    }
    const cache = new Map();
    const calls = async node => {
      if (!cache.has(node)) cache.set(node, await api('POST', '/api/calls', { key, run, node, depth: 1, chosen: {} }));
      return cache.get(node);
    };
    const res = [];
    for (const r of rows) {
      if (r.place === 'funcao') continue;
      const o = occ.get(`${r.path}@${r.start}`);
      if (!o) { res.push({ ...r, occ: null }); continue; }
      const c = await calls(o.owner);
      const l = c.links.filter(x => x.occurrence === o.id);
      const u = c.unresolved.find(x => x.occurrence === o.id);
      res.push({ ...r, occ: o.id, graph_owner: G.g.nodes[o.owner].label, graph_owner_kind: G.g.nodes[o.owner].kind,
        resolution: l[0]?.resolution ?? u?.reason, relation: l[0]?.relation, runs: l[0]?.runs ? G.g.nodes[l[0].runs].label : undefined,
        targets: l.map(x => `${G.g.nodes[x.to].kind} ${G.g.nodes[x.to].label} (${G.where(x.to)})`) });
    }
    fs.writeFileSync(path.join(out, 'fora-do-corpo.json'), JSON.stringify({ run, res }, null, 1));
    // Real cases for question 2.
    const byLabel = (label, kind, file) => Object.values(G.g.nodes).find(n => n.label === label && n.kind === kind && G.where(n.id).startsWith(file));
    const show = {};
    const cases = [
      ['NullSink', 'class', 'hc/api/tests/test_smtpd.py'],
      ['TestServer.__init__', 'method', 'hc/api/tests/test_smtpd.py'],
      ['TestServer.stdout', 'field', 'hc/api/tests/test_smtpd.py'],
      ['row', 'function', 'hc/front/templatetags/asciitable.py'],
      ['cors', 'function', 'hc/api/decorators.py'],
      ['checks', 'function', 'hc/api/views.py'],
      ['require_setting', 'function', 'hc/front/decorators.py'],
      ['Command.__init__', 'method', 'hc/api/management/commands/sendalerts.py'],
      ['Command.executor', 'field', 'hc/api/management/commands/sendalerts.py'],
      ['CreateHelper.__init__', 'method', 'hc/lib/webauthn.py'],
      ['CreateHelper.server', 'field', 'hc/lib/webauthn.py'],
    ];
    for (const [label, kind, file] of cases) {
      const n = byLabel(label, kind, file);
      if (!n) { show[label] = 'não achado'; continue; }
      const c = await calls(n.id);
      const u = await api('GET', `/api/used-by?key=${key}&run=${run}&node=${encodeURIComponent(n.id)}&depth=1`);
      show[label] = {
        id: n.id, at: G.where(n.id), parent: G.parent[n.id] ? G.g.nodes[G.parent[n.id]].label : null,
        calls: c.links.map(x => `${x.text.split(/\r?\n/)[0]} -> ${G.g.nodes[x.to].label} [${x.resolution}, ${x.relation}${x.runs ? ', runs ' + G.g.nodes[x.runs].label : ''}] l.${x.evidence.start_line}`),
        unresolved: c.unresolved.map(x => `${(x.text || '').split(/\r?\n/)[0]} [${x.reason}] l.${x.evidence?.start_line}`),
        used_by: u,
      };
    }
    fs.writeFileSync(path.join(out, 'casos.json'), JSON.stringify(show, null, 1));
    console.log('run', run, 'linhas', res.length);
  });
})().catch(e => { console.error(e); process.exit(2); });

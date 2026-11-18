const root = process.argv[2];
const api = async (m, p, b) => { const r = await fetch('http://127.0.0.1:8773' + p, { method: m, headers: { 'content-type': 'application/json' }, body: b && JSON.stringify(b) }); const t = await r.text(); if (!r.ok) throw new Error(p + ' ' + t); return JSON.parse(t); };
(async () => {
  const { key } = await api('POST', '/api/project', { root });
  const { id: run } = await api('POST', '/api/generate', { key });
  const g = await api('GET', '/api/graph?key=' + key + '&run=' + run);
  const seed = Object.values(g.nodes).find(n => n.category === 'symbol' && n.label === 'seed').id;
  const c = await api('POST', '/api/calls', { key, run, node: seed, depth: 1, chosen: {} });
  const count = {}; for (const u of c.unresolved) count[u.reason] = (count[u.reason] ?? 0) + 1;
  console.log(JSON.stringify({ key, run, nodes: c.nodes.length, links: c.links.length, truncated: c.truncated, max_nodes: c.max_nodes, unresolved: count }));
  const lim = c.unresolved.filter(u => u.reason === 'node_limit');
  console.log('first and last node_limit:', JSON.stringify([lim[0], lim.at(-1)].map(u => ({ text: u.text, resolution: u.resolution, candidates: u.candidates, line: u.evidence.start_line, to: g.nodes[u.to].label }))));
  console.log('links + node_limit = 130 calls:', c.links.length + lim.length === 130);
  console.log('no_candidate:', JSON.stringify(c.unresolved.filter(u => u.reason === 'no_candidate_in_project').map(u => u.text)));
})().catch(e => { console.error(e.message); process.exit(1); });

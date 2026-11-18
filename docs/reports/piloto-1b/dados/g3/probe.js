const key = 'healthchecks-25597e41', run = '20261118-061046';
const post = async (port, body) => (await fetch(`http://127.0.0.1:${port}/api/calls`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })).json();
(async () => {
  const g = await (await fetch(`http://127.0.0.1:8773/api/graph?key=${key}&run=${run}`)).json();
  const seeds = Object.values(g.nodes).filter(n => n.category === 'symbol' && /function|method/.test(n.kind) && !/test/i.test(n.label));
  for (const s of seeds) {
    const now = await post(8773, { key, run, node: s.id, depth: 6, chosen: {} });
    if (!now.truncated) continue;
    const before = await post(8771, { key, run, node: s.id, depth: 6, chosen: {} });
    const { unresolved, ...rest } = now;
    const limited = unresolved.filter(u => u.reason === 'node_limit');
    const count = {}; for (const u of unresolved) count[u.reason] = (count[u.reason] ?? 0) + 1;
    console.log(JSON.stringify({ seed: s.label, id: s.id, nodes: now.nodes.length, links: now.links.length, truncated: now.truncated, compatible: JSON.stringify(rest) === JSON.stringify(before), unresolved: count }));
    const shown = new Set(now.nodes.map(n => n.id));
    console.log('node_limit targets not in nodes:', limited.every(u => !shown.has(u.to)), 'resolutions:', JSON.stringify(limited.reduce((a, u) => (a[u.resolution] = (a[u.resolution] ?? 0) + 1, a), {})));
    console.log(JSON.stringify(limited.slice(0, 3).map(u => ({ text: u.text, resolution: u.resolution, candidates: u.candidates, to: g.nodes[u.to]?.label, at: `${u.evidence.path}:${u.evidence.start_line}` })), null, 1));
    return;
  }
  console.log('no truncated walk found among', seeds.length);
})();

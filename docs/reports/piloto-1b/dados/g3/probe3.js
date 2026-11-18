const key = 'healthchecks-25597e41', run = '20261118-061046';
const post = async (port, body) => (await fetch(`http://127.0.0.1:${port}/api/calls`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })).json();
(async () => {
  const g = await (await fetch(`http://127.0.0.1:8773/api/graph?key=${key}&run=${run}`)).json();
  const seeds = Object.values(g.nodes).filter(n => n.category === 'symbol' && /function|method/.test(n.kind) && !/test/i.test(n.label));
  const sized = [];
  for (const s of seeds) sized.push([(await post(8773, { key, run, node: s.id, depth: 6, chosen: {} })).nodes.length, s]);
  sized.sort((a, b) => b[0] - a[0]);
  console.log('largest unchosen walks:', sized.slice(0, 5).map(([n, s]) => `${s.label}=${n}`).join(', '));
  for (const [, s] of sized.slice(0, 40)) {
    let chosen = {}, now;
    for (let round = 0; round < 20; round++) {
      now = await post(8773, { key, run, node: s.id, depth: 6, chosen });
      if (now.truncated) break;
      const open = now.links.filter(l => l.resolution === 'homonym' && !chosen[l.occurrence]);
      if (!open.length) break;
      // Like a person choosing, prefer a non-test candidate in another file.
      for (const l of open) if (!chosen[l.occurrence]) {
        const options = now.links.filter(x => x.occurrence === l.occurrence).map(x => x.to).filter(id => !/test/i.test(g.nodes[id].label));
        chosen[l.occurrence] = options.at(-1) ?? l.to;
      }
    }
    if (!now.truncated) continue;
    const before = await post(8771, { key, run, node: s.id, depth: 6, chosen });
    const { unresolved, ...rest } = now;
    const count = {}; for (const u of unresolved) count[u.reason] = (count[u.reason] ?? 0) + 1;
    console.log(JSON.stringify({ seed: s.label, id: s.id, chosen: Object.keys(chosen).length, nodes: now.nodes.length, links: now.links.length, truncated: now.truncated, compatible: JSON.stringify(rest) === JSON.stringify(before), unresolved: count }));
    const limited = unresolved.filter(u => u.reason === 'node_limit'), shown = new Set(now.nodes.map(n => n.id));
    console.log('targets outside the drawn nodes:', limited.every(u => !shown.has(u.to)), '| resolutions kept:', JSON.stringify(limited.reduce((a, u) => (a[u.resolution] = (a[u.resolution] ?? 0) + 1, a), {})));
    console.log(JSON.stringify(limited.slice(0, 4).map(u => ({ text: u.text.slice(0, 60), resolution: u.resolution, candidates: u.candidates, to: g.nodes[u.to]?.label, at: `${u.evidence.path}:${u.evidence.start_line}` })), null, 1));
    require('fs').writeFileSync('truncated.json', JSON.stringify({ seed: s.id, label: s.label, chosen }));
    return;
  }
  console.log('none truncated');
})();

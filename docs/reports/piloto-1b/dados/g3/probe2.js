const key = 'healthchecks-25597e41', run = '20261118-061046';
const post = async (port, body) => (await fetch(`http://127.0.0.1:${port}/api/calls`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })).json();
const seed = 'src:8ce3e55438349ca1cfd362c1daa62b1f586496f894fd70587817115292f75e4d:d03082adb2aacfd44f96bb0df8339a145b4979fee4277225c3a4b8612d1493e6:method:19528:22942'; // Check.ping
(async () => {
  const g = await (await fetch(`http://127.0.0.1:8773/api/graph?key=${key}&run=${run}`)).json();
  let chosen = {}, now;
  for (let round = 0; round < 15; round++) {
    now = await post(8773, { key, run, node: seed, depth: 6, chosen });
    if (now.truncated) break;
    const homonyms = now.links.filter(l => l.resolution === 'homonym' && !chosen[l.occurrence]);
    if (!homonyms.length) break;
    for (const l of homonyms) chosen[l.occurrence] ??= l.to;  // the first candidate, as a person clicking "follow this one"
  }
  const before = await post(8771, { key, run, node: seed, depth: 6, chosen });
  const { unresolved, ...rest } = now;
  const count = {}; for (const u of unresolved) count[u.reason] = (count[u.reason] ?? 0) + 1;
  console.log(JSON.stringify({ chosen: Object.keys(chosen).length, nodes: now.nodes.length, links: now.links.length, truncated: now.truncated, compatible: JSON.stringify(rest) === JSON.stringify(before), unresolved: count }));
  const limited = unresolved.filter(u => u.reason === 'node_limit'), shown = new Set(now.nodes.map(n => n.id));
  console.log('targets outside the drawn nodes:', limited.every(u => !shown.has(u.to)), '| resolutions kept:', JSON.stringify(limited.reduce((a, u) => (a[u.resolution] = (a[u.resolution] ?? 0) + 1, a), {})));
  console.log(JSON.stringify(limited.slice(0, 4).map(u => ({ text: u.text.slice(0, 60), resolution: u.resolution, candidates: u.candidates, to: g.nodes[u.to]?.label, at: `${u.evidence.path}:${u.evidence.start_line}` })), null, 1));
  require('fs').writeFileSync('chosen.json', JSON.stringify(chosen));
})();

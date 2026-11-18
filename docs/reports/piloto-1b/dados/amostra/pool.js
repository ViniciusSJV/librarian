// Part C: answer categories over every object.method() call of the pool, and how many unique
// links of a method call land on a module function (the error pattern of the sample).
const [port, key, run] = process.argv.slice(2);
const api = async (m, p, b) => (await fetch(`http://127.0.0.1:${port}${p}`, { method: m, headers: { 'content-type': 'application/json' }, body: b && JSON.stringify(b) })).json();
(async () => {
  const g = await api('GET', `/api/graph?key=${key}&run=${run}`);
  const owners = new Map();
  for (const e of g.edges) {
    if (e.kind !== 'observes') continue;
    const o = g.nodes[e.to], owner = g.nodes[e.from];
    if (o.kind !== 'method_call_observed' || !/function|method/.test(owner.kind)) continue;
    if (/(^|\/)tests?\/|test_|_test\.|\/benches\//.test(e.evidence.path) || owner.label.includes('test')) continue;
    if (!/^[A-Za-z_][\w.]*\.\w+\(/.test(o.label) || /^self\.\w+\(/.test(o.label)) continue;
    (owners.get(owner.id) ?? owners.set(owner.id, new Set()).get(owner.id)).add(o.id);
  }
  const count = {}, toKind = {}; let total = 0;
  for (const [owner, occ] of owners) {
    const c = await api('POST', '/api/calls', { key, run, node: owner, depth: 1, chosen: {} });
    for (const id of occ) {
      total++;
      const u = c.unresolved.find(x => x.occurrence === id), l = c.links.filter(x => x.occurrence === id);
      const cat = u ? u.reason : l[0].resolution;
      count[cat] = (count[cat] ?? 0) + 1;
      if (!u && l.length === 1) { const k = `${cat}->${g.nodes[l[0].to].kind}`; toKind[k] = (toKind[k] ?? 0) + 1; }
    }
  }
  console.log(JSON.stringify({ total, count, single_target_by_kind: toKind }));
})();

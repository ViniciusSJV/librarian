// 2.3a-0 forecast: `self.m()` above the limit, caller a method whose container declares exactly one
// method m (not a trait method), keyed by the label's last segment after `::` or `.`.
const fs = require('fs'), dir = process.argv[2];
const out = [];
for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  const g = JSON.parse(fs.readFileSync(`${dir}/${key}.graph.json`, 'utf8'));
  const d = JSON.parse(fs.readFileSync(`${dir}/${key}.calls.json`, 'utf8'));
  const parent = {}; for (const e of g.edges) if (e.kind === 'contains') parent[e.to] = e.from;
  const byName = {}, old = {}; let dots = 0;
  for (const [id, n] of Object.entries(g.nodes)) {
    if (n.category !== 'symbol' || !['method', 'trait_method'].includes(n.kind)) continue;
    if (n.language === 'rust' && n.label.includes('.')) dots++;
    (byName[n.label.split(/[:.]/).pop()] ??= []).push(id);
    (old[n.label.split('::').pop()] ??= []).push(id);
  }
  let changed = 0;
  for (const [owner, r] of Object.entries(d.res)) for (const u of r.unresolved) {
    if (u.reason !== 'above_candidate_limit') continue;
    const m = u.text.match(/^self\s*\.\s*(\w+)\s*\(/); if (!m) continue;
    const pick = ids => (ids ?? []).filter(x => parent[x] === parent[owner] && g.nodes[x].kind !== 'trait_method');
    const now = pick(byName[m[1]]), before = pick(old[m[1]]);
    if (now.length === 1 && before.length !== 1) { changed++; out.push([key, `${u.path}@${u.sb}-${u.eb}`, `${u.path}:${u.line}`, r.owner.label, u.text.slice(0, 60), g.nodes[now[0]].label].join('\t')); }
  }
  console.log(key, 'rótulos Rust com ponto:', dots, 'previstas:', changed);
}
fs.writeFileSync(process.argv[3], out.join('\n') + '\n');

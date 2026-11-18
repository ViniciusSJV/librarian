// Part C: a deterministic sample of object.method() calls, one per file, with what "what does it call?" answers.
// Usage: node sample.js <port> <key> <run> <n> <out.json>
const fs = require('fs'), crypto = require('crypto');
const [port, key, run, n, out] = process.argv.slice(2);
const api = async (m, p, b) => (await fetch(`http://127.0.0.1:${port}${p}`, { method: m, headers: { 'content-type': 'application/json' }, body: b && JSON.stringify(b) })).json();
(async () => {
  const project = await api('GET', `/api/project?key=${key}`);
  const root = project.root ?? project.project_root ?? project.config?.project_root;
  const g = await api('GET', `/api/graph?key=${key}&run=${run}`);
  const parent = {}; for (const e of g.edges) if (e.kind === 'contains') parent[e.to] = e.from;
  const pool = [];
  for (const e of g.edges) {
    if (e.kind !== 'observes') continue;
    const o = g.nodes[e.to], owner = g.nodes[e.from];
    if (o.kind !== 'method_call_observed' || !/function|method/.test(owner.kind)) continue;
    if (/(^|\/)tests?\/|test_|_test\.|\/benches\//.test(e.evidence.path) || owner.label.includes('test')) continue;
    // object.method(): a receiver that is a written name or attribute chain, not a literal.
    if (!/^[A-Za-z_][\w.]*\.\w+\(/.test(o.label) || /^self\.\w+\(/.test(o.label)) continue;
    pool.push({ e, o, owner, h: crypto.createHash('sha256').update(o.id).digest('hex') });
  }
  pool.sort((a, b) => a.h.localeCompare(b.h));
  const files = new Set(), picked = [];
  for (const p of pool) { if (picked.length >= +n) break; if (files.has(p.e.evidence.path)) continue; files.add(p.e.evidence.path); picked.push(p); }
  const rows = [];
  for (const p of picked) {
    const c = await api('POST', '/api/calls', { key, run, node: p.owner.id, depth: 1, chosen: {} });
    const links = c.links.filter(l => l.occurrence === p.o.id), un = c.unresolved.find(u => u.occurrence === p.o.id);
    const where = id => { const d = g.edges.find(x => x.kind === 'declares' && x.to === id); return `${g.nodes[id].label} (${d?.evidence.path}:${d?.evidence.start_line})`; };
    rows.push({ text: p.o.label.split('\n')[0].slice(0, 90), at: `${p.e.evidence.path}:${p.e.evidence.start_line}`, owner: p.owner.label,
      answer: un ? `${un.reason} (${un.candidates})` : `${links[0]?.resolution} (${links[0]?.candidates})`, targets: links.map(l => where(l.to)) });
  }
  fs.writeFileSync(out, JSON.stringify({ root, pool: pool.length, rows }, null, 1));
  console.log('root', root, 'pool', pool.length);
  rows.forEach((r, i) => console.log(`${i + 1}. ${r.at} [${r.owner}] ${r.text}\n   -> ${r.answer} ${r.targets.join(' | ').slice(0, 300)}`));
})().catch(e => { console.error(e); process.exit(1); });

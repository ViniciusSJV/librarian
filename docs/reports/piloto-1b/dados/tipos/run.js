// Part B: what the graph records for each case. Usage: node run.js <port> <root> <case,case,...>
const [port, root, list] = process.argv.slice(2);
const api = async (m, p, b) => { const r = await fetch(`http://127.0.0.1:${port}${p}`, { method: m, headers: { 'content-type': 'application/json' }, body: b && JSON.stringify(b) }); const t = await r.text(); if (!r.ok) throw new Error(p + ' ' + t); return JSON.parse(t); };
(async () => {
  const { key } = await api('POST', '/api/project', { root });
  const { id: run } = await api('POST', '/api/generate', { key });
  const g = await api('GET', `/api/graph?key=${key}&run=${run}`);
  console.log('profile', g.profile, 'nodes', Object.keys(g.nodes).length, 'edges', g.edges.length);
  const kinds = {}; for (const e of g.edges) kinds[`${e.kind}/${e.resolution}`] = (kinds[`${e.kind}/${e.resolution}`] ?? 0) + 1;
  const occKinds = {}; for (const n of Object.values(g.nodes)) if (n.category === 'occurrence') occKinds[n.kind] = (occKinds[n.kind] ?? 0) + 1;
  console.log('edge kinds', JSON.stringify(kinds)); console.log('occurrence kinds', JSON.stringify(occKinds));
  const parent = {}; for (const e of g.edges) if (e.kind === 'contains') parent[e.to] = e.from;
  const lab = id => { const n = g.nodes[id]; const p = parent[id] && g.nodes[parent[id]]; return `${p && n.label.indexOf('.') < 0 ? p.label + '::' : ''}${n.label}(${n.kind})`; };
  const by = {}; for (const e of g.edges) (by[e.from] ??= []).push(e);
  for (const name of list.split(',')) {
    const syms = Object.values(g.nodes).filter(n => n.category === 'symbol' && (n.label === name || n.label.endsWith('.' + name) || n.label.endsWith('::' + name)));
    for (const s of syms) {
      console.log(`\n== ${name} [${s.label} ${s.kind}]`);
      for (const e of (by[s.id] ?? []).filter(e => e.kind === 'observes')) {
        const o = g.nodes[e.to];
        const cands = (by[o.id] ?? []).filter(x => x.kind === 'name_candidate').map(x => lab(x.to));
        console.log(`  ${o.kind.padEnd(22)} ${JSON.stringify(o.label).slice(0, 40).padEnd(40)} L${e.evidence.start_line} -> ${o.ambiguous_candidates ? 'acima do teto: ' + o.ambiguous_candidates : cands.join(' | ') || '-'}`);
      }
      const c = await api('POST', '/api/calls', { key, run, node: s.id, depth: 1, chosen: {} });
      for (const l of c.links) console.log(`  CALL ${l.text} -> ${lab(l.to)} [${l.resolution}, ${l.candidates}]`);
      for (const u of c.unresolved) console.log(`  SEM LINK ${u.text} [${u.reason}, ${u.candidates}]`);
    }
  }
})().catch(e => { console.error(e.message); process.exit(1); });

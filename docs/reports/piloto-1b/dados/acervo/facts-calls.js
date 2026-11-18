// Part E: every call occurrence inside the lines each fact cites, with what "what does it call?" answers.
// Reads the facts of the original .studio (read only); queries the copy served at 8781.
const fs = require('fs');
const key = 'healthchecks-25597e41', run = '20261118-061046';
const facts = fs.readFileSync('C:/Users/beatl/Documents/Projetos/librarian/.studio/projects/healthchecks-25597e41/facts.jsonl', 'utf8').trim().split('\n').map(JSON.parse);
const api = async (m, p, b) => (await fetch('http://127.0.0.1:8781' + p, { method: m, headers: { 'content-type': 'application/json' }, body: b && JSON.stringify(b) })).json();
(async () => {
  const g = await api('GET', `/api/graph?key=${key}&run=${run}`);
  const declared = {}; for (const e of g.edges) if (e.kind === 'declares') declared[e.to] = e.evidence;
  const where = id => `${g.nodes[id].label} [${g.nodes[id].kind}] ${declared[id]?.path}:${declared[id]?.start_line}`;
  const out = [];
  for (const f of facts) {
    // The innermost declaration named symbol_name whose chunk holds the declaration line.
    const owner = Object.values(g.nodes).filter(n => n.category === 'symbol' && (n.label === f.symbol_name || n.label.endsWith('.' + f.symbol_name)))
      .filter(n => declared[n.id]?.path === f.path && declared[n.id].start_line <= f.declaration_line && declared[n.id].end_line >= f.declaration_line)
      .sort((a, b) => (declared[a.id].end_line - declared[a.id].start_line) - (declared[b.id].end_line - declared[b.id].start_line))[0];
    const ranges = f.citations.filter(c => c.path === f.path).map(c => [c.start_line, c.end_line]).filter(([s, e]) => !(s === e && s === f.declaration_line));
    const inside = ev => ranges.some(([s, e]) => ev.start_line >= s && ev.start_line <= e);
    const c = await api('POST', '/api/calls', { key, run, node: owner.id, depth: 1, chosen: {} });
    const calls = {};
    for (const l of c.links) if (inside(l.evidence)) (calls[l.occurrence] ??= { text: l.text.split('\n')[0], line: l.evidence.start_line, kind: l.occurrence_kind, resolution: l.resolution, candidates: l.candidates, targets: [] }).targets.push(where(l.to));
    for (const u of c.unresolved) if (inside(u.evidence)) calls[u.occurrence] = { text: u.text.split('\n')[0], line: u.evidence.start_line, kind: u.occurrence_kind, resolution: u.reason, candidates: u.candidates, targets: [] };
    out.push({ id: f.id, owner: owner.label, statement: f.statement, ranges, calls: Object.values(calls).sort((a, b) => a.line - b.line) });
  }
  fs.writeFileSync(process.argv[2], JSON.stringify(out, null, 1));
  for (const o of out) {
    console.log(`${o.id} [${o.owner}] linhas ${o.ranges.map(r => r.join('-')).join(',')}`);
    for (const k of o.calls) console.log(`   L${k.line} ${k.kind === 'method_call_observed' ? 'obj.m()' : 'f()'} ${k.text.slice(0, 70)} -> ${k.resolution} (${k.candidates}) ${k.targets.join(' | ').slice(0, 220)}`);
  }
})().catch(e => { console.error(e); process.exit(1); });

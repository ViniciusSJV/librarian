// Q6 repeated (Piloto 1b): new edition, the same question, the same 7 declarations, with_facts. Stops before sending.
const fs = require('fs');
const O = 'C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/q6';
const ORIG = O + '/root/.studio/projects/healthchecks-25597e41/runs/20261118-061046/turns/14';
const key = 'healthchecks-25597e41';
const api = async (m, p, b) => { const r = await fetch('http://127.0.0.1:8790' + p, { method: m, headers: { 'content-type': 'application/json' }, body: b && JSON.stringify(b) }); const t = await r.text(); if (!r.ok) throw new Error(`${p} ${r.status} ${t.slice(0, 400)}`); return JSON.parse(t); };
(async () => {
  const q = JSON.parse(fs.readFileSync(ORIG + '/question.json'));
  const project = await api('GET', `/api/project?key=${key}`);
  const before = project.runs.map(r => r.id);
  const gen = await api('POST', '/api/generate', { key });
  const run = gen.id;
  const runs = (await api('GET', `/api/project?key=${key}`)).runs;
  const old = runs.find(r => r.id === '20261118-061046'), now = runs.find(r => r.id === run);
  const g = await api('GET', `/api/graph?key=${key}&run=${run}`);
  const missing = q.gathered.nodes.filter(n => !g.nodes[n]);
  const prepared = await api('POST', '/api/gathered-question', { key, run, question: q.asked.original, nodes: q.gathered.nodes, with_facts: true });
  fs.writeFileSync(O + '/prepared-response.json', JSON.stringify(prepared, null, 1));
  console.log(JSON.stringify({ runs_before: before, new_run: run,
    edition: { old: { graph: old.graph_sha256, nodes: old.nodes, edges: old.edges, sources: old.sources, git: old.git }, new: { graph: now.graph_sha256, nodes: now.nodes, edges: now.edges, sources: now.sources, git: now.git, diagnostics: now.diagnostics } },
    nodes_missing_in_new_edition: missing, turn: prepared.turn ?? prepared.id, prepare_error: prepared.prepare_error,
    keys: Object.keys(prepared) }, null, 1));
})().catch(e => { console.error(e.message); process.exit(1); });

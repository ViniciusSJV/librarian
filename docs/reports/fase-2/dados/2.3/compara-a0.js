// 2.3a-0: /api/calls before and after, call by call, in every field; compared with the forecast.
const fs = require('fs'), [a, b, prev] = process.argv.slice(2);
const want = new Set(fs.readFileSync(prev, 'utf8').trim().split('\n').map(l => l.split('\t')[1]));
for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  const same = fs.readFileSync(`${a}/${key}.graph.json`).equals(fs.readFileSync(`${b}/${key}.graph.json`));
  const index = d => { const m = new Map(); for (const [o, r] of Object.entries(d.res)) {
    for (const l of r.links) { const k = `${l.path}@${l.sb}-${l.eb}`; const v = m.get(k) ?? []; v.push(JSON.stringify({ o, ...l, occ: undefined })); m.set(k, v); }
    for (const u of r.unresolved) { const k = `${u.path}@${u.sb}-${u.eb}`; const v = m.get(k) ?? []; v.push(JSON.stringify({ o, ...u, occ: undefined })); m.set(k, v); } } return m; };
  const A = index(JSON.parse(fs.readFileSync(`${a}/${key}.calls.json`, 'utf8'))), B = index(JSON.parse(fs.readFileSync(`${b}/${key}.calls.json`, 'utf8')));
  const keys = new Set([...A.keys(), ...B.keys()]);
  let diff = 0, expected = 0, unexpected = [], kinds = {};
  for (const k of keys) {
    const x = (A.get(k) ?? []).sort().join('\n'), y = (B.get(k) ?? []).sort().join('\n');
    if (x === y) continue;
    diff++;
    if (want.has(k)) { expected++; const p = JSON.parse(A.get(k)[0]), q = JSON.parse(B.get(k)[0]); const t = `${p.reason ?? p.resolution} (${p.candidates}) -> ${q.resolution} (${q.candidates}) basis=${q.basis} edge=${q.candidate_edge ?? q.edge ?? null}`; kinds[t] = (kinds[t] ?? 0) + 1; }
    else unexpected.push(k);
  }
  const missing = [...want].filter(k => k.startsWith('') && keys.has(k) && (A.get(k) ?? []).join() === (B.get(k) ?? []).join());
  console.log(key, { grafo_igual: same, chamadas: keys.size, mudaram: diff, previstas_que_mudaram: expected, nao_previstas: unexpected.length, previstas_que_nao_mudaram: key.startsWith('health') ? missing.length : 0 }, kinds, unexpected.slice(0, 5));
}

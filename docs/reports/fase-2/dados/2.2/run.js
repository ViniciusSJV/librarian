// node run.js <dump-dir> '<opts json>' [out.json]
// Without opts: checks that the simulator reproduces the dump. With opts: old vs new per call.
'use strict';
const fs = require('fs'), path = require('path');
const S = require('./sim.js');
const dir = process.argv[2], opts = JSON.parse(process.argv[3] || '{}'), outFile = process.argv[4];
const isTest = (p, owner) => /(^|\/)tests?\/|test_|_test\.|\/benches\//.test(p) || owner.includes('test');
const out = {};
for (const key of Object.keys(S.SRC)) {
  const graph = JSON.parse(fs.readFileSync(path.join(dir, `${key}.graph.json`), 'utf8'));
  const { res } = JSON.parse(fs.readFileSync(path.join(dir, `${key}.calls.json`), 'utf8'));
  const R = S.build(key, graph);
  if (opts.kind) opts.imports = S.importsFor(key, R);
  const lang = key.startsWith('health') ? 'python' : 'rust';
  const at = id => `${graph.nodes[id].label} (${R.declared[id] ? R.declared[id].path + ':' + R.declared[id].start_line : '?'})`;
  const rows = []; let mismatch = 0;
  for (const [owner, r] of Object.entries(res)) {
    const dumpBy = {};
    for (const l of r.links) (dumpBy[l.occ] ??= { r: l.resolution, t: [] }).t.push(l.to.label + ' (' + l.to.at + ')');
    for (const u of r.unresolved) if (u.reason !== 'node_limit') dumpBy[u.occ] = { r: u.reason, t: [] }; else (dumpBy[u.occ] ??= { r: u.resolution, t: [] }).t.push(u.to.label + ' (' + u.to.at + ')');
    for (const [occ, d] of Object.entries(dumpBy)) {
      const s0 = S.resolve(R, owner, occ, {}, lang);
      const show = s => { const multi = s.targets.length > 1 && s.r !== 'homonym'; if ((multi || s.r === 'trait') && !(R.cand[occ] ?? []).length) return { r: graph.nodes[occ].ambiguous_candidates != null ? 'above_candidate_limit' : 'no_candidate_in_project', t: [] }; return { r: multi ? 'homonym' : s.r === 'trait' ? 'homonym' : s.r, t: (multi || s.r === 'trait' ? (R.cand[occ] ?? []).map(e => e.to) : s.targets).map(at).sort() }; };
      const a = show(s0);
      const same = a.r === d.r && JSON.stringify(a.t) === JSON.stringify([...d.t].sort());
      if (!same) { mismatch++; if (mismatch <= 5) console.log('MISMATCH', key, graph.nodes[occ].label.slice(0, 60), JSON.stringify(a), JSON.stringify(d)); }
      if (Object.keys(opts).length) {
        const s1 = S.resolve(R, owner, occ, opts, lang); const b = show(s1);
        const ev = R.ev[occ];
        rows.push({ occ, owner: graph.nodes[owner].label, okind: graph.nodes[occ].kind, text: graph.nodes[occ].label.split(/\r?\n/)[0].slice(0, 90), at: `${ev.path}:${ev.start_line}`, key: `${ev.path}@${ev.start_byte}-${ev.end_byte}`,
          test: isTest(ev.path, graph.nodes[owner].label), cand: (R.cand[occ] ?? []).length, before: a, after: b, mod: s1.mod ?? null, dropped: s1.dropped ?? null, single: !!s1.single, above: !!s1.above, types: S.receiverTypes(R, owner, occ), wtype: S.writtenType(R, occ) });
      }
    }
  }
  console.log(key, 'simulador x dump: divergencias =', mismatch);
  out[key] = rows;
}
if (outFile) fs.writeFileSync(outFile, JSON.stringify(out));

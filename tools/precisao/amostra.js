// Picks the extended sample (step 2.0) and writes its skeleton, without the truth. Run once, over the
// baseline; the fixture it writes is what the gate measures afterwards, and the truth is filled by hand.
// Usage: node tools/precisao/amostra.js --studio <.studio> [--bin target/release] --saida <dir>
'use strict';
const fs = require('fs'), path = require('path');
const L = require('./lib');

// Calls per stratum. The Part C calls come first and count toward their stratum.
const QUOTAS = {
  hc: { no_candidate_in_project: 14, 'unique->outro': 10, 'unique->function': 12, homonym: 14, above_candidate_limit: 6, receiver: 4 },
  rt: { no_candidate_in_project: 7, 'unique->outro': 5, 'unique->function': 2, homonym: 7, above_candidate_limit: 4, receiver: 5 },
};
const PART_C = { hc: 20, rt: 10 };

const stratum = a => a.resolution === 'unique' ? `unique->${a.targets[0].kind === 'function' ? 'function' : 'outro'}` : a.resolution;

(async () => {
  const opt = L.parseArgs(process.argv.slice(2));
  if (!opt.studio || !opt.saida) throw new Error('uso: --studio <.studio> --saida <dir> [--bin <dir>]');
  const bin = opt.bin ?? path.join(L.REPO, 'target/release');
  const keys = [L.DEFAULTS.hc, L.DEFAULTS.rt].map(s => s.split('/')[0]);
  const out = await L.withStudio({ studio: opt.studio, bin, keys }, async ({ api, root }) => {
    const res = {};
    for (const lang of ['hc', 'rt']) {
      const [key, run] = L.DEFAULTS[lang].split('/');
      const G = L.loadGraph(root, key, run), P = L.pool(G);
      const owners = new Map();
      for (const p of P) (owners.get(p.owner.id) ?? owners.set(p.owner.id, []).get(p.owner.id)).push(p.occ.id);
      const A = await L.answers(api, key, run, G, owners);
      const sorted = [...P].sort((a, b) => a.h.localeCompare(b.h));
      // Part C: hash order, one call per file.
      const picked = [], files = new Set();
      for (const p of sorted) {
        if (picked.length >= PART_C[lang]) break;
        if (files.has(p.edge.evidence.path)) continue;
        files.add(p.edge.evidence.path); picked.push({ p, parte_c: true });
      }
      // Then each stratum up to its quota: hash order, files not yet used first.
      const have = s => picked.filter(x => stratum(A.get(x.p.occ.id)) === s).length;
      for (const [s, quota] of Object.entries(QUOTAS[lang])) {
        const free = sorted.filter(p => stratum(A.get(p.occ.id)) === s && !picked.some(x => x.p === p));
        const order = [...free.filter(p => !files.has(p.edge.evidence.path)), ...free.filter(p => files.has(p.edge.evidence.path))];
        for (const p of order) {
          if (have(s) >= quota) break;
          files.add(p.edge.evidence.path); picked.push({ p, parte_c: false });
        }
      }
      res[lang] = picked.map(({ p, parte_c }, i) => {
        const a = A.get(p.occ.id);
        return {
          id: `${lang.toUpperCase()}-${String(i + 1).padStart(2, '0')}`, parte_c, estrato: stratum(a),
          ocorrencia: p.key, local: `${p.edge.evidence.path}:${p.edge.evidence.start_line}`,
          texto: p.occ.label.split(/\r?\n/)[0].trim().slice(0, 100), dono: p.owner.label,
          linha_de_base: { resolution: a.resolution, candidates: a.candidates, alvos: a.targets.map(t => `${t.label} (${t.at})`) },
        };
      });
    }
    return res;
  });
  fs.mkdirSync(opt.saida, { recursive: true });
  for (const lang of ['hc', 'rt']) fs.writeFileSync(path.join(opt.saida, `amostra-${lang}.esqueleto.json`), JSON.stringify(out[lang], null, 1) + '\n');
  for (const lang of ['hc', 'rt']) {
    const by = {}; for (const r of out[lang]) by[r.estrato] = (by[r.estrato] ?? 0) + 1;
    console.log(lang, out[lang].length, JSON.stringify(by));
  }
})().catch(e => { console.error(e.message); process.exit(1); });

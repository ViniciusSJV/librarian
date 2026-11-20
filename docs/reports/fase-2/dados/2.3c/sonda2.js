// Sonda (JSON): para cada chamada com receptor nos arquivos dados dos projetos sintéticos, o que
// /api/calls responde, no formato dos níveis do portão.   node sonda2.js <bin> <out.jsonl>
'use strict';
const fs = require('fs'), path = require('path'), os = require('os');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const [bin, outFile] = process.argv.slice(2);
const SIN = 'C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/sinteticos';
(async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'sonda-'));
  fs.mkdirSync(path.join(root, '.studio', 'projects'), { recursive: true });
  const s = await L.startStudio(bin, root);
  const out = [];
  try {
    for (const [p, files] of [['py', ['pkg/retorno.py', 'pkg/casos.py']], ['rs', ['src/retorno.rs', 'src/casos.rs']]]) {
      const { key } = await s.api('POST', '/api/project', { root: path.join(SIN, p) });
      const { id: run } = await s.api('POST', '/api/generate', { key });
      const G = L.loadGraph(root, key, run);
      for (const [id, n] of Object.entries(G.g.nodes)) {
        if (n.category !== 'symbol' || !['function', 'method'].includes(n.kind) || !G.declared[id] || !files.includes(G.declared[id].path)) continue;
        const c = await s.api('POST', '/api/calls', { key, run, node: id, depth: 1, chosen: {} });
        const by = new Map();
        for (const l of c.links) {
          const a = by.get(l.occurrence) ?? { projeto: p, dono: n.label, linha: l.evidence.start_line, texto: l.text.split('\n')[0], resolution: l.resolution, candidates: l.candidates, alvos: [], regra: l.type_rule ?? null };
          a.alvos.push(`${G.g.nodes[l.to].label} (${G.where(l.to)})`);
          by.set(l.occurrence, a);
        }
        for (const u of c.unresolved) by.set(u.occurrence, { projeto: p, dono: n.label, linha: u.evidence.start_line, texto: (u.text || '').split('\n')[0], resolution: u.reason, candidates: u.candidates, alvos: [], regra: null });
        for (const a of by.values()) { a.alvos.sort(); out.push(a); }
      }
    }
  } finally { s.stop(); }
  fs.writeFileSync(outFile, out.map(x => JSON.stringify(x)).join('\n') + '\n');
  console.log(out.length, 'chamadas');
})().catch(e => { console.error(e); process.exit(2); });

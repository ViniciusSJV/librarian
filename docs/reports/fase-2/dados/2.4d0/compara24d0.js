// Passo 2.4d-0, Parada 1: /api/calls, used_by e grafo de todos os símbolos, antes (binário atual) e com o protótipo; cada
// mudança é separada em direta (receptor é classe do projeto, classe24c.json) ou indireta (o resto).
//   node compara24d0.js <dump-antes> <dump-depois> <out.json>
'use strict';
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const { load } = require('../2.4/load.js');
const [A0, D0, out] = process.argv.slice(2);
const direta = new Set(require('../2.4c/classe24c.json').map(r => r.occ));
const res = {};
for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  const gh = d => crypto.createHash('sha256').update(fs.readFileSync(path.join(d, `${key}.graph.json`))).digest('hex').slice(0, 12);
  const A = load(A0, key), D = load(D0, key), s = { simbolos: Object.keys(A.d.res).length, chamadas: A.calls.size, grafo: [gh(A0), gh(D0)], used_by_diferentes: 0, mudancas: [], resolucoes: {}, so_rotulo: 0, por_dono: 0 };
  for (const id of Object.keys(A.d.res)) {
    if (JSON.stringify(A.d.res[id].used_by) !== JSON.stringify(D.d.res[id].used_by)) s.used_by_diferentes++;
    if (JSON.stringify([A.d.res[id].links, A.d.res[id].unresolved]) !== JSON.stringify([D.d.res[id].links, D.d.res[id].unresolved])) s.por_dono++;
  }
  const f = (P, c) => c && { res: c.res, cands: c.cands, rule: c.rule ?? null, rel: c.rel ?? null, runs: c.runs ? P.show(c.runs) : null, alvos: c.targets.map(P.show).sort() };
  for (const occ of new Set([...A.calls.keys(), ...D.calls.keys()])) {
    const a = A.calls.get(occ), d = D.calls.get(occ), x = f(A, a), y = f(D, d), k = `${a.lang} ${x.res} -> ${y.res}`;
    s.resolucoes[k] = (s.resolucoes[k] ?? 0) + 1;
    if (JSON.stringify(x) === JSON.stringify(y)) continue;
    s.mudancas.push({ occ: `${a.path}@${a.sb}-${a.eb}`, local: `${a.path}:${a.line}`, test: a.test, texto: a.text.split('\n')[0].slice(0, 70), direta: direta.has(`${a.path}@${a.sb}-${a.eb}`), x, y });
  }
  res[key] = s;
  console.log(key, 'símbolos', s.simbolos, 'chamadas', s.chamadas, 'grafo', s.grafo.join(' '), '| used_by diferentes', s.used_by_diferentes, '| símbolos com resposta diferente', s.por_dono, '| mudanças', s.mudancas.length, 'diretas', s.mudancas.filter(m => m.direta).length, 'indiretas', s.mudancas.filter(m => !m.direta).length);
  for (const [k, n] of Object.entries(s.resolucoes)) if (k.split(' ')[1] !== k.split(' -> ')[1]) console.log('  ', k, n);
  for (const m of s.mudancas) console.log('  ', m.direta ? 'direta  ' : 'INDIRETA', m.test ? 'T' : 'P', m.local, m.texto, '|', m.x.res, m.x.cands, '->', m.y.res, m.y.rule ?? '', m.y.cands, m.y.alvos.join('; '));
  if (key.startsWith('health')) {
    const t = {};
    for (const c of D.calls.values()) if (direta.has(`${c.path}@${c.sb}-${c.eb}`)) { const a = A.calls.get(c.occ), k = `${a.res} -> ${c.res}${JSON.stringify(f(A, a)) === JSON.stringify(f(D, c)) ? ' (idêntica)' : ''}`; t[k] = (t[k] ?? 0) + 1; }
    console.log('   as 71:', t);
    console.log('   type_rule classe:', [...D.calls.values()].filter(c => c.rule === 'classe').length);
    // Receptores que passam a ter tipo pelo retorno das 18, com ou sem chamada de método sobre eles.
    const un = P => { const o = { total: 0, fora: 0 }; for (const c of P.calls.values()) if (c.lang === 'python' && c.targets.length === 1 && c.res !== 'homonym') { o.total++; if (!P.d.res[c.targets[0]].used_by.some(x => x[0] === c.occ)) o.fora++; } return o; };
    console.log('   links únicos (Python) e os que o used_by do alvo não mostra:', JSON.stringify(un(A)), '->', JSON.stringify(un(D)));
  }
}
fs.writeFileSync(out, JSON.stringify(res, null, 1) + '\n');

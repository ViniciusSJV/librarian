// Passo 2.4a, 24/11/2026: /api/calls e /api/used-by de todos os símbolos, com o binário do fd388cd (dump24a) e
// com o da correção de global e visibilidade (49fcfb2). Nada deve mudar.
//   node compara-correcao.js <dump24a> <dump-correcao> <out.json>
'use strict';
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const [a, b, outFile] = process.argv.slice(2);
const out = {};
for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  const rd = d => JSON.parse(fs.readFileSync(path.join(d, `${key}.json`), 'utf8')).res;
  const sha = d => crypto.createHash('sha256').update(fs.readFileSync(path.join(d, `${key}.graph.json`))).digest('hex');
  const A = rd(a), B = rd(b), dif = [];
  for (const id of new Set([...Object.keys(A), ...Object.keys(B)])) if (JSON.stringify(A[id]) !== JSON.stringify(B[id])) dif.push({ antes: A[id], depois: B[id] });
  out[key] = { simbolos: [Object.keys(A).length, Object.keys(B).length], diferentes: dif, grafo_igual: sha(a) === sha(b), grafo: sha(b) };
  console.log(key, out[key].simbolos, 'diferentes', dif.length, 'grafo igual', out[key].grafo_igual);
}
fs.writeFileSync(outFile, JSON.stringify(out, null, 1) + '\n');

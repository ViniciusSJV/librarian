// Regras 1 e 2: casos.   node lista12.js
'use strict';
const o = require('./py24b.json');
console.log('== metodos abstratos'); for (const a of o.abstratos) console.log(JSON.stringify(a));
console.log('== regra 1: chamadas'); for (const r of o.rows.filter(r => r.over)) console.log(r.at, r.text, '|', r.forma, r.classe, '|', r.mud, r.alvo, r.over.join('; '));
const x = o.rows.filter(r => r.r2);
console.log('== regra 2:', x.length, 'incertas', x.filter(r => r.r2.incerta).length);
for (const r of x) console.log(r.test ? 'T' : 'P', r.at, r.text.slice(0, 50), '|', r.forma, r.classe, r.mro.join('<'), '|', r.hoje, r.r2.antes, '->', r.r2.ficam.length, r.r2.incerta ? 'INCERTA' : '', r.r2.ficam.join('; '), '| alvos hoje:', r.hojeAlvos.join('; ').slice(0, 300));
const fs = require('fs');
for (const a of JSON.parse(fs.readFileSync('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/fixtures/amostra-hc.json', 'utf8')).chamadas) for (const r of x) if (r.occ === a.ocorrencia) console.log('AMOSTRA', a.id, r.text, JSON.stringify(a.verdade));

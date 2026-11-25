// Lista as mudanças previstas pelo py24b.js e cruza com a amostra.   node lista24b.js
'use strict';
const fs = require('fs');
const o = require('./py24b.json');
const f = r => console.log(r.test ? 'T' : 'P', r.at, r.dono, '|', r.text.slice(0, 50), '|', r.forma, r.classe, '|', r.hoje, r.nCand, '|', r.mud, '|', r.alvo ?? r.fora ?? '', r.via ?? '', (r.subclassesQueSobrescrevem ?? []).length ? 'subs:' + r.subclassesQueSobrescrevem.length : '', /unique|TROCA/.test(r.mud) ? r.hojeAlvos[0] : '');
const mud = o.rows.filter(r => /-> |TROCA/.test(r.mud));
console.log('== produção, mudanças'); mud.filter(r => !r.test).forEach(f);
console.log('== produção, outras de interesse'); o.rows.filter(r => !r.test && !mud.includes(r) && (/cls|prop/.test(r.forma) || /unique|atributo|protocol/.test(r.mud) || r.forma === 'super')).forEach(f);
console.log('== teste, sem ser super'); mud.filter(r => r.test && r.forma !== 'super').forEach(f);
const c = {}; o.rows.filter(r => r.test && r.forma === 'super').forEach(r => { const k = `${r.text} | ${r.alvo ?? r.fora} | ${r.mud}`; c[k] = (c[k] ?? 0) + 1; }); console.log('== teste, super', c);
console.log('sem tipo no simulador', o.check.semTipo);
console.log('init nenhum', [...new Set(o.inits.filter(x => /nenhum/.test(x.runs)).map(x => x.classe + ' ' + x.decos))]);
console.log('vários defs', o.rows.filter(r => r.multi).length, 'travas', o.rows.filter(r => /alcança|outra subclasse|sem achar/.test(r.mud)).length);
const am = JSON.parse(fs.readFileSync('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/fixtures/amostra-hc.json', 'utf8')).chamadas;
console.log('== amostra'); for (const a of am) for (const r of o.rows.filter(r => r.occ === a.ocorrencia)) console.log(a.id, a.estrato, r.text.slice(0, 40), r.forma, '|', r.mud, '|', r.alvo ?? r.fora ?? '', '| verdade', JSON.stringify(a.verdade).slice(0, 120));
console.log('descendentes', mud.filter(r => r.subclassesQueSobrescrevem?.length).map(r => r.at + ' ' + r.text.slice(0, 30) + ' ' + r.subclassesQueSobrescrevem.length));

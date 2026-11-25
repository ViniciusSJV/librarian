// Passo 2.4, Parada 1 (diagnóstico). Chamadas simples cujo nome é ligado localmente na função
// (atribuição, parâmetro, alvo de for/with): o NameScope.locals da regra. Lido do texto do dono.
'use strict';
const fs = require('fs'), path = require('path');
const o = require('./py.json');
const SRC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const { load } = require('./load.js');
const P = load(process.argv[2] ?? '../dump', 'healthchecks-25597e41');
const byAt = new Map();
for (const c of P.calls.values()) if (c.lang === 'python' && c.kind === 'call_observed') byAt.set(`${c.path}:${c.line}:${c.text}`, c);
const out = [];
for (const x of o.n0) {
  const c = byAt.get(`${x.at}:${x.text}`);
  if (!c) continue;
  let fn = c.owner;
  while (fn && !['function', 'method'].includes(P.N[fn]?.kind)) fn = P.parent[fn];
  if (!fn) continue;
  const ev = P.declared[fn];
  const body = Buffer.from(fs.readFileSync(path.join(SRC, c.path))).subarray(ev.start_byte, ev.end_byte).toString('utf8');
  const n = x.text.replace(/[^\w]/g, '');
  const head = body.slice(0, body.search(/\)\s*(->[^:]*)?:\s*\r?\n/) + 1);
  const assigned = new RegExp(`^\\s*${n}\\s*(:[^=\\n]*)?=[^=]`, 'm').test(body);
  const target = new RegExp(`\\bfor\\s+${n}\\b|\\bas\\s+${n}\\b`).test(body);
  const param = new RegExp(`[(,]\\s*\\*{0,2}${n}\\s*[:,)=]`).test(head);
  if (!assigned && !target && !param) continue;
  const m = body.match(new RegExp(`^\\s*${n}\\s*=\\s*(.+)$`, 'm'));
  out.push({ at: x.at, text: x.text, test: x.test, now: x.now.res, targets: x.now.targets, como: assigned ? 'atribuição' : target ? 'alvo' : 'parâmetro', valor: m ? m[1].trim().slice(0, 60) : null, binding: x.binding.how });
}
const c = {};
for (const x of out) { const k = `${x.test ? 'T' : 'P'} ${x.now} | ${x.text} | ${x.como} ${x.valor ?? ''}`; c[k] = (c[k] ?? 0) + 1; }
console.log(out.length);
console.log(c);
fs.writeFileSync('locais.json', JSON.stringify(out, null, 1));

// Passo 2.4c, antes dos commits: Classe.metodo(..) com Classe do projeto (Python) e T::f(..) (Rust), por resolução, e
// quantas fechariam se a classe fosse usada. Só conta. Com o dump das fontes finais (dump24c.js).
//   node classe24c.js <dump> <out.json>      (o simulador da hierarquia é o de py24c.js, por gera-classe)
'use strict';
const fs = require('fs'), path = require('path');
const [dump, out] = process.argv.slice(2);
// py24b.js até a regra, com as funções dele (named, lookup, info...) expostas.
let s = fs.readFileSync(path.join(__dirname, '../2.4b/py24b.js'), 'utf8').replace(/\r\n/g, '\n');
s = s.slice(0, s.indexOf('\nconst R = require'));
s = s.replace(/require\('\.\.\//g, `require('${path.join(__dirname, '..').replace(/\\/g, '/')}/`);
s = s.replace("const [dir, outFile] = process.argv.slice(2);", `const dir = ${JSON.stringify(dump)}, outFile = null;`);
const X = new Function('require', s.replace("'use strict';", '') + '\nreturn { P, N, info, lookup, named, parts, slice, show, declared, up, splitTop, balanced, paramsAt, descendants };')(require);
const { P, N, info, lookup, named, parts, slice, show, declared, up, splitTop, balanced, paramsAt, descendants } = X;
const cnt = (xs, f) => { const m = {}; xs.forEach(x => { const k = f(x); m[k] = (m[k] ?? 0) + 1; }); return Object.fromEntries(Object.entries(m).sort()); };
// O nome é ligado por uma função em volta (parâmetro, atribuição, for, with): não é a classe.
function local(c, name) {
  for (let fn = up(c.owner, ['function', 'method']); fn; fn = up(X.N[fn] && P.parent[fn], ['function', 'method'])) {
    const d = declared[fn], t = slice(c.path, d.start_byte, d.end_byte), o = paramsAt(t), e = balanced(t, o);
    if (splitTop(t.slice(o + 1, e)).some(p => p.replace(/^\*+/, '').split(/[:=]/)[0].trim() === name)) return true;
    if (new RegExp(`(^|\\n)\\s*${name}\\s*(:[^=\\n]+)?=(?!=)|\\bfor\\s+[^\\n]*\\b${name}\\b[^\\n]*\\bin\\b|\\bas\\s+${name}\\b`).test(t.slice(e))) return true;
  }
  return false;
}
const rows = [];
for (const c of P.calls.values()) {
  if (c.lang !== 'python' || !c.path || c.kind !== 'method_call_observed') continue;
  const text = slice(c.path, c.sb, c.eb), p = parts(text);
  if (!p || !/^[A-Za-z_][\w.]*$/.test(p.receiver) || /^(self|cls)\b/.test(p.receiver)) continue;
  const n = named(c.path, p.receiver);
  if (n.kind !== 'project' || !info[n.id] || local(c, p.receiver.split('.')[0])) continue;
  const l = lookup(n.id, p.name), k = info[n.id];
  const alvo = l.found ? show(l.found.at(-1)) : null;
  const situacao = l.found ? 'método achado' : l.external ? 'base de fora antes' : l.none ? 'ninguém declara' : l.undecided;
  const fecha = c.res === 'homonym' && l.found?.length === 1;
  rows.push({ at: `${c.path}:${c.line}`, occ: `${c.path}@${c.sb}-${c.eb}`, test: c.test, recv: p.receiver, nome: p.name, classe: N[n.id].label, onde: k.file === c.path ? 'mesmo módulo' : p.receiver.includes('.') ? 'por módulo' : 'importada', res: c.res, cands: c.cands, alvos: c.targets.map(show), situacao, alvo, fecha, certo: c.targets.length === 1 && c.res !== 'homonym' ? c.targets[0] === l.found?.at(-1) : null, sobrescritas: l.found ? descendants(n.id).filter(d => info[d].methods[p.name]).length : 0 });
}
console.log('Python: Classe.metodo(..) com Classe do projeto:', rows.length);
for (const t of [false, true]) console.log(t ? ' teste' : ' produção', cnt(rows.filter(r => r.test === t), r => `${r.res} | ${r.situacao} | ${r.onde}`));
console.log(' por resolução', cnt(rows, r => r.res));
console.log(' homonym que fechariam', rows.filter(r => r.fecha).length, 'de', rows.filter(r => r.res === 'homonym').length, cnt(rows.filter(r => r.fecha), r => `${r.classe}.${r.nome} [${r.cands}]`));
console.log(' dessas, com sobrescrita em subclasse:', rows.filter(r => r.fecha && r.sobrescritas).length);
rows.filter(r => r.fecha).forEach(r => console.log('   ', r.test ? 'T' : 'P', r.at, `${r.recv}.${r.nome}`, r.onde, r.res, r.cands, '->', r.alvo));
console.log(' links únicos: certos pela classe', rows.filter(r => r.certo === true).length, '; errados', rows.filter(r => r.certo === false).length);
rows.filter(r => r.certo === false).forEach(r => console.log('    DIFERE', r.at, `${r.recv}.${r.nome}`, r.res, r.alvos, r.situacao, r.alvo));
console.log(' links únicos cujo nome tem outra declaração de método no projeto:', rows.filter(r => r.certo && Object.keys(N).filter(i => N[i].category === 'symbol' && N[i].kind === 'method' && N[i].label.split('.').pop() === r.nome).length > 1).length);
console.log(' sem candidato', cnt(rows.filter(r => r.res === 'no_candidate_in_project'), r => r.nome));
fs.writeFileSync(out, JSON.stringify(rows, null, 1) + '\n');
// Rust
const { load } = require('../2.4/load.js');
const Q = load(dump, 'renderer-9b78f3a9');
const types = new Set(Object.values(Q.N).filter(n => n.category === 'symbol' && ['struct', 'enum', 'union', 'trait', 'type'].includes(n.kind)).map(n => n.label.split('::').pop().replace(/<.*$/, '')));
const rs = [];
for (const c of Q.calls.values()) {
  if (c.lang !== 'rust' || c.kind !== 'call_observed') continue;
  const m = c.text.match(/^((?:\w+::)*)(\w+)(?:::<[^(]*>)?::(\w+)\s*(?:\(|::<|$)/);
  if (!m || !/^[A-Z]/.test(m[2])) continue;
  const own = m[2] === 'Self' ? 'Self' : types.has(m[2]) ? 'tipo do projeto' : 'fora';
  rs.push({ at: `${c.path}:${c.line}`, text: c.text.slice(0, 60), tipo: m[2], f: m[3], own, res: c.res, rule: c.rule, n: c.targets.length, alvos: c.targets.map(Q.show) });
}
console.log('Rust: T::f(..):', rs.length, cnt(rs, r => `${r.own} | ${r.res} ${r.rule ?? ''}`));
const h = rs.filter(r => r.own !== 'fora' && ['homonym', 'above_candidate_limit'].includes(r.res));
console.log(' de tipo do projeto ou Self, sem fechar:', h.length, cnt(h, r => `${r.tipo}::${r.f} ${r.res} [${r.n}]`));
h.forEach(r => console.log('    ', r.at, r.text, r.res, r.alvos.map(a => a.slice(0, 70)).join('; ')));

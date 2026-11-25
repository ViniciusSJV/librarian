// Passo 2.4c, Parada 1: no Rust, links únicos para um método de trait e quem implementa.   node rust.js <dump>
const { load } = require('../2.4/load.js');
const P = load(process.argv[2], 'renderer-9b78f3a9'), { N, show, last } = P, c = {}, ex = [];
for (const x of P.calls.values()) { const k = N[x.targets[0]]?.kind ?? '-'; if (x.targets.length === 1 && x.res !== 'homonym' && k === 'trait_method') { const impls = Object.keys(N).filter(i => N[i].category === 'symbol' && N[i].kind === 'method' && last(N[i].label) === last(N[x.targets[0]].label) && / for /.test(N[i].label)).length; ex.push([x.path + ':' + x.line, x.text.slice(0, 50), x.res, show(x.targets[0]), 'impls de mesmo nome', impls].join(' | ')); } const r = x.res + ' ' + k; c[r] = (c[r] ?? 0) + 1; }
console.log('classes', Object.values(N).filter(n => n.kind === 'class').length, 'traits', Object.values(N).filter(n => n.kind === 'trait').length); console.log(c); ex.forEach(x => console.log(x));

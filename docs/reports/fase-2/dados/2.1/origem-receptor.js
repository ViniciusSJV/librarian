const fs = require('fs'); const R = 'C:/Users/beatl/Documents/Projetos/renderer/';
const t = require('./type-links.json').filter(x => !x.edge && x.kind === 'method_call_observed');
const escape = s => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const out = [];
for (const x of t) {
  const [f, rng] = x.key.split('@'); const [a] = rng.split('-').map(Number);
  const src = fs.readFileSync(R + f); const lines = src.slice(0, a).toString().split('\n');
  const paren = x.text.indexOf('(');
  const recv = x.text.slice(0, x.text.lastIndexOf('.', paren > 0 ? paren : undefined));
  let i = lines.length - 1; while (i > 0 && !/\bfn\s+\w+/.test(lines[i])) i--;
  const body = lines.slice(i).join('\n'); let origem = '?', ev = '';
  const v = recv.replace(/^&/, '').replace(/^\((.*)\)$/, '$1');
  const esc = escape(v);
  if (/^self\.\w+$/.test(v)) origem = 'campo (self.f)';
  else if (/\(/.test(v)) origem = /^[A-Z]/.test(v) ? 'construtor no receptor (T::f(..).m())' : 'retorno de função solta no receptor';
  else {
    const binds = [];
    const re = [[new RegExp('let\\s+(mut\\s+)?' + esc + '\\s*:\\s*([^=;]+)'), 'anotação (let x: T)'],
      [new RegExp('let\\s+(mut\\s+)?' + esc + '\\s*=\\s*([A-Z]\\w*)\\s*(::|\\{)'), 'construtor (let x = T::f(..))'],
      [new RegExp('let\\s+(mut\\s+)?' + esc + '\\s*=\\s*([a-z_]\\w*)\\s*\\('), 'retorno de função solta (let x = f(..))'],
      [new RegExp('for\\s+(\\(\\s*\\w+\\s*,\\s*)?' + esc + '\\)?\\s+in\\s+([^{]+)'), 'item de coleção (for)'],
      [new RegExp('\\(\\s*ref\\s+(mut\\s+)?' + esc + '\\s*\\)'), 'padrão de variante (T::V(ref x))'],
      [new RegExp('[(,]\\s*(mut\\s+)?' + esc + '\\s*:\\s*([^,)]+)'), 'parâmetro anotado']];
    for (const [r, k] of re) { const mm = body.match(r); if (mm) binds.push(k + ' <= ' + mm[0].trim().replace(/\s+/g, ' ').slice(0, 80)); }
    origem = binds.length ? binds[0].split(' <= ')[0] : '?'; ev = binds.join(' | ');
  }
  out.push({ key: x.key, at: x.at, pop: x.pool, text: x.text.slice(0, 70), to: x.to, n: x.n, origem, ev, owner: x.owner });
}
const c = {}; for (const o of out) c[o.origem] = (c[o.origem] || 0) + 1; console.log(c);
for (const o of out) console.log([o.pop ? 'POP' : '   ', o.at, o.key, o.text, '->', o.to, 'n=' + o.n, '|', o.owner, '|', o.ev || o.origem].join(' '));
fs.writeFileSync('origem.json', JSON.stringify(out, null, 1));

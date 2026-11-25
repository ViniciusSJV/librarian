// Passo 2.4, Parada 1 (diagnóstico; não é código de produção). Rust (ray tracer): a chamada simples
// f() e o caminho de módulo (super::f(), modulo::f()) resolvidos pelo escopo do módulo: o item do
// próprio módulo, depois o `use` explícito, depois o `use ..::*`; nada disso: de fora (prelude).
//   node rs24.js <dump-dir> <out.json>
'use strict';
const fs = require('fs'), path = require('path');
const { load } = require('./load.js');
const SRC = 'C:/Users/beatl/Documents/Projetos/renderer';
const KEY = 'renderer-9b78f3a9';
const CRATE = 'renderer';
const [dir, outFile] = process.argv.slice(2);
const P = load(dir, KEY);
const { N, declared, parent, children, cands, show } = P;
const LINKED = new Set(['unique', 'import', 'receiver', 'type', 'trait_default', 'chosen']);
const sym = id => N[id]?.category === 'symbol';
const srcs = {};
const src = f => srcs[f] ??= fs.readFileSync(path.join(SRC, f), 'utf8');

// Modules: file modules and inline `mod x { .. }` (graph `module` nodes).
const fileModPath = f => {
  if (f.startsWith('src/bin/')) return ['BIN:' + f];
  if (f === 'src/lib.rs' || f === 'src/main.rs') return [];
  return f.replace(/^src\//, '').replace(/\.rs$/, '').replace(/\/mod$/, '').split('/');
};
const mods = []; // { key, file, start, end, path: [..], node }
const files = new Set(Object.values(declared).map(e => e.path).filter(p => p.endsWith('.rs')));
for (const f of files) mods.push({ key: f, file: f, start: 0, end: Buffer.byteLength(src(f)), path: fileModPath(f), node: null });
for (const [id, n] of Object.entries(N)) {
  if (n.kind !== 'module' || !declared[id]) continue;
  const ev = declared[id];
  const text = Buffer.from(src(ev.path)).subarray(ev.start_byte, ev.end_byte).toString('utf8');
  if (!text.includes('{')) continue; // `mod x;` declares a file module
  mods.push({ key: id, file: ev.path, start: ev.start_byte, end: ev.end_byte, path: [...fileModPath(ev.path), n.label.split('::').pop()], node: id });
}
const modAt = (file, byte) => mods.filter(m => m.file === file && m.start <= byte && byte < m.end).sort((a, b) => (a.end - a.start) - (b.end - b.start))[0];
const modByPath = p => mods.find(m => m.path.join('::') === p.join('::'));

// Items of a module: symbols declared in its range and not inside a nested module or a function.
const itemsOf = {};
for (const [id, n] of Object.entries(N)) {
  if (!sym(id) || !declared[id] || n.language !== 'rust') continue;
  if (!['function', 'struct', 'enum', 'trait', 'type', 'const', 'static'].includes(n.kind)) continue;
  const p = parent[id];
  if (p && !['module', 'file'].includes(N[p]?.kind)) continue;
  const m = modAt(declared[id].path, declared[id].start_byte);
  (itemsOf[m.key] ??= {})[n.label.split('::').pop()] = id;
}
// `use` declarations of a module (not of a nested one): name -> path, and globs.
function usesOf(m) {
  const buf = Buffer.from(src(m.file));
  const nested = mods.filter(o => o !== m && o.file === m.file && o.start >= m.start && o.end <= m.end && o.start > m.start);
  const text = buf.subarray(m.start, m.end).toString('utf8');
  const out = { names: {}, globs: [] };
  const re = /\buse\s+([^;]+);/g;
  let r;
  while ((r = re.exec(text))) {
    const at = m.start + Buffer.byteLength(text.slice(0, r.index));
    if (nested.some(o => o.start <= at && at < o.end)) continue;
    expand(r[1].replace(/\s+/g, ''), [], out);
  }
  return out;
}
function expand(tree, prefix, out) {
  const brace = tree.indexOf('{');
  if (brace >= 0) {
    const head = tree.slice(0, brace).replace(/::$/, '').split('::').filter(Boolean);
    const inner = tree.slice(brace + 1, tree.lastIndexOf('}'));
    let depth = 0, cur = '';
    for (const ch of inner + ',') {
      if (ch === '{') depth++;
      if (ch === '}') depth--;
      if (ch === ',' && depth === 0) { if (cur) expand(cur, [...prefix, ...head], out); cur = ''; } else cur += ch;
    }
    return;
  }
  const [p, as] = tree.split(/as/).length === 2 && /::\w+as\w+$/.test(tree) ? [tree.slice(0, tree.lastIndexOf('as')), tree.slice(tree.lastIndexOf('as') + 2)] : [tree, null];
  const segs = [...prefix, ...p.split('::').filter(Boolean)];
  if (segs[segs.length - 1] === '*') out.globs.push(segs.slice(0, -1));
  else if (segs[segs.length - 1] === 'self') out.names[as ?? segs[segs.length - 2]] = segs.slice(0, -1);
  else out.names[as ?? segs[segs.length - 1]] = segs;
}
// A use path to a module path, from module m.
function absolute(m, segs) {
  const out = [...m.path];
  let i = 0;
  if (segs[0] === 'crate' || segs[0] === CRATE) { out.length = 0; i = 1; }
  else if (segs[0] === 'self') i = 1;
  else if (segs[0] === 'super') { while (segs[i] === 'super') { out.pop(); i++; } }
  else return null; // std, itertools, rayon...: outside
  if (out[0]?.startsWith('BIN:') && (segs[0] === 'crate')) return null;
  if (segs[0] === CRATE) return segs.slice(1);
  return [...out.filter(x => !x.startsWith('BIN:')), ...segs.slice(i)];
}
function lookupIn(modPath, name, depth = 0) {
  const m = modByPath(modPath);
  if (!m) return { kind: 'de-fora' };
  const own = itemsOf[m.key]?.[name];
  if (own) return { kind: 'item', id: own };
  if (depth > 2) return { kind: 'nada' };
  const u = usesOf(m);
  if (u.names[name]) {
    const abs = absolute(m, u.names[name]);
    if (!abs) return { kind: 'de-fora' };
    return lookupIn(abs.slice(0, -1), abs[abs.length - 1], depth + 1);
  }
  return { kind: 'nada' };
}
function bind(file, byte, name) {
  const m = modAt(file, byte);
  const own = itemsOf[m.key]?.[name];
  if (own) return { kind: 'item', how: 'mesmo módulo', id: own, mod: m.path.join('::') };
  const u = usesOf(m);
  if (u.names[name]) {
    const abs = absolute(m, u.names[name]);
    if (!abs) return { kind: 'de-fora', how: 'use de fora' };
    const r = lookupIn(abs.slice(0, -1), abs[abs.length - 1]);
    return { ...r, how: 'use', mod: m.path.join('::') };
  }
  const found = [];
  for (const g of u.globs) {
    const abs = absolute(m, g);
    if (!abs) continue;
    const r = lookupIn(abs, name);
    if (r.kind === 'item') found.push({ ...r, glob: g.join('::') });
  }
  if (found.length === 1) return { ...found[0], how: 'use glob', mod: m.path.join('::') };
  if (found.length > 1) return { kind: 'ambiguo-glob', how: 'use glob', found: found.map(f => show(f.id)) };
  return { kind: 'livre', how: 'nada (prelude ou de fora)', mod: m.path.join('::') };
}

const rows = [];
for (const c of P.calls.values()) {
  if (c.lang !== 'rust' || c.kind !== 'call_observed' || !c.path) continue;
  const segs = c.text.split('::');
  let b;
  if (segs.length === 1) b = bind(c.path, c.sb, c.text);
  else if (/^[a-z_]/.test(segs[segs.length - 2]) && !/^[A-Z]/.test(segs[0])) {
    // module path: super::f, modulo::f, crate::a::f
    const m = modAt(c.path, c.sb);
    let abs = absolute(m, segs.slice(0, -1));
    if (!abs) {
      const u = usesOf(m);
      const head = u.names[segs[0]];
      abs = head ? absolute(m, [...head, ...segs.slice(1, -1)]) : modByPath([...m.path.filter(x => !x.startsWith('BIN:')), ...segs.slice(0, -1)]) ? [...m.path, ...segs.slice(0, -1)] : null;
    }
    b = abs ? { ...lookupIn(abs, segs[segs.length - 1]), how: 'caminho de módulo' } : { kind: 'de-fora', how: 'caminho de fora' };
  } else continue;
  const target = b.kind === 'item' ? b.id : null;
  let mudanca;
  if (target) {
    if (LINKED.has(c.res) && c.targets.length === 1 && c.targets[0] === target) mudanca = 'confirma';
    else if (LINKED.has(c.res)) mudanca = 'troca-alvo';
    else mudanca = `${c.res}->escopo` + ((cands[c.occ] ?? []).includes(target) ? '' : ' (fora dos candidatos)');
  } else if (b.kind === 'de-fora' || b.kind === 'livre') mudanca = LINKED.has(c.res) ? 'link->? (escopo não acha no projeto)' : c.res === 'homonym' ? 'homonym->? (escopo não acha)' : 'igual';
  else mudanca = 'igual (' + b.kind + ')';
  rows.push({ at: `${c.path}:${c.line}`, text: c.text, test: c.test || /mod tests|#\[cfg\(test\)\]/.test('') , mod: modAt(c.path, c.sb).path.join('::'), now: { res: c.res, targets: c.targets.map(show), cands: (cands[c.occ] ?? []).map(show) }, binding: { kind: b.kind, how: b.how, to: target ? show(target) : b.found ?? null }, mudanca });
}
// test: inside a `mod tests` module
for (const r of rows) r.test = /(^|::)tests/.test(r.mod);
fs.writeFileSync(outFile, JSON.stringify(rows, null, 1));
const count = (xs, f) => xs.reduce((m, x) => { const k = f(x); m[k] = (m[k] ?? 0) + 1; return m; }, {});
console.log('Rust chamadas simples e de módulo:', rows.length);
console.log(' hoje:', count(rows, x => x.now.res));
console.log(' ligação:', count(rows, x => x.binding.kind + '/' + x.binding.how));
console.log(' mudança (produção):', count(rows.filter(x => !x.test), x => x.mudanca));
console.log(' mudança (teste):', count(rows.filter(x => x.test), x => x.mudanca));

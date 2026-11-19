// Step 2.3, Parada 1: Rust receivers bound by `let x = T::f(..)` or `let x = f(..)`, read from the text
// of the owner (diagnosis only). node rust23.js <graph-v2.json>
'use strict';
const fs = require('fs');
const SRC = 'C:/Users/beatl/Documents/Projetos/renderer/';
const g = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const decl = {}, rts = {}, observedBy = {}, cand = {};
for (const e of g.edges) {
  if (e.kind === 'declares') decl[e.to] = e.evidence;
  if (e.kind === 'observes') { (observedBy[e.from] ??= []).push(e.to); if (g.nodes[e.to].kind === 'receiver_type') (rts[`${e.evidence.path}@${e.evidence.start_byte}`] ??= []).push([e.evidence.end_byte, e.to]); }
  if (e.kind === 'name_candidate') (cand[e.from] ??= []).push(e.to);
}
const buf = {}; const src = p => buf[p] ??= fs.readFileSync(SRC + p);
const isTest = (p, owner) => /tests?_|\/tests?\//.test(owner) || /(^|\/)tests?\//.test(p);
const ret = id => (observedBy[id] ?? []).map(o => g.nodes[o]).filter(n => n.kind === 'declared_return').map(n => n.label)[0] ?? null;
const syms = Object.entries(g.nodes).filter(([id, n]) => n.category === 'symbol' && decl[id]);
const out = [];
for (const e of g.edges) {
  if (e.kind !== 'observes') continue;
  const o = g.nodes[e.to]; if (o.kind !== 'method_call_observed') continue;
  const m = o.label.match(/^([a-z_]\w*)\s*\.\s*(\w+)/); if (!m) continue;
  const x = m[1]; if (x === 'self') continue;
  const d = decl[e.from]; if (!d) continue;
  const text = src(d.path).slice(d.start_byte, e.evidence.start_byte).toString();
  const re = new RegExp(String.raw`let\s+(?:mut\s+)?${x}\b\s*(:[^=;]+)?=\s*([^;]*)`, 'g');
  let last = null, mm; while ((mm = re.exec(text))) last = mm;
  if (!last || last[1]) continue; // no binding, or annotated (the annotation decides)
  const rhs = last[2].trim();
  const rt = (rts[`${e.evidence.path}@${e.evidence.start_byte}`] ?? []).find(([end]) => end === e.evidence.start_byte + Buffer.byteLength(x));
  const recorded = rt ? (cand[rt[1]] ?? []).map(t => g.nodes[t].label) : null;
  let k = rhs.match(/^((?:\w+::)*)([A-Z]\w*)(?:<[^>]*>)?::(\w+)\s*\(/);
  if (k) {
    const T = k[2], f = k[3];
    const fs_ = syms.filter(([id, n]) => (n.kind === 'method' || n.kind === 'function') && n.label.endsWith('::' + f) && /^impl /.test(n.label) && new RegExp(String.raw`(^impl |for )${T}(<[^>]*>)?::${f}$`).test(n.label));
    out.push({ forma: 'T::f', at: `${e.evidence.path}:${e.evidence.start_line}`, teste: isTest(e.evidence.path, g.nodes[e.from].label), call: o.label.split('\n')[0].slice(0, 70), x, rhs: rhs.slice(0, 50), T, f, alvos: fs_.map(([id, n]) => `${n.label} -> ${ret(id)}`), tipo_gravado: recorded });
    continue;
  }
  k = rhs.match(/^((?:\w+::)*)([a-z_]\w*)\s*\(/);
  if (k) {
    const f = k[2];
    const fns = syms.filter(([id, n]) => n.kind === 'function' && n.label.split('::').pop() === f);
    const same = fns.filter(([id]) => decl[id].path === e.evidence.path);
    out.push({ forma: 'f()', at: `${e.evidence.path}:${e.evidence.start_line}`, teste: isTest(e.evidence.path, g.nodes[e.from].label), call: o.label.split('\n')[0].slice(0, 70), x, rhs: rhs.slice(0, 50), f, mesmo_arquivo: same.length > 0, alvos: fns.map(([id, n]) => `${n.label} (${decl[id].path}) -> ${ret(id)}`), tipo_gravado: recorded });
  }
}
fs.writeFileSync(process.argv[3], JSON.stringify(out, null, 1));
const c = {}; for (const r of out) { const k = `${r.forma} ${r.teste ? 'teste' : 'prod'} ${r.forma === 'f()' ? (r.alvos.length ? (r.mesmo_arquivo ? 'mesmo arquivo' : 'outro arquivo') : 'sem função no projeto') : (r.alvos.length ? 'f no projeto' : 'f fora')}`; c[k] = (c[k] ?? 0) + 1; }
console.log(c);

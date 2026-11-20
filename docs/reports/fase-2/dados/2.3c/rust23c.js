// Passo 2.3c, Parada 1 (diagnóstico). Ray tracer: cada chamada de método x.m() com x ligado por
// `let x = T::f(..)` ou `let x = f(..)` (sem anotação), a resolução de hoje da chamada que liga, o
// retorno escrito do alvo e o tipo que o 2.3c daria.   node rust23c.js <dump-dir> <out.json>
'use strict';
const fs = require('fs'), path = require('path');
const SRC = 'C:/Users/beatl/Documents/Projetos/renderer/';
const [dir, outFile] = process.argv.slice(2);
const KEY = 'renderer-9b78f3a9';
const g = JSON.parse(fs.readFileSync(path.join(dir, `${KEY}.graph.json`), 'utf8'));
const d = JSON.parse(fs.readFileSync(path.join(dir, `${KEY}.calls.json`), 'utf8'));
const decl = {}, parent = {}, observedBy = {}, rts = {};
for (const e of g.edges) {
  if (e.kind === 'declares') decl[e.to] = e.evidence;
  if (e.kind === 'contains') parent[e.to] = e.from;
  if (e.kind === 'observes') { (observedBy[e.from] ??= []).push(e.to); if (g.nodes[e.to].kind === 'receiver_type') (rts[`${e.evidence.path}@${e.evidence.start_byte}`] ??= []).push([e.evidence.end_byte, e.to]); }
}
const cand = {}; for (const e of g.edges) if (e.kind === 'name_candidate') (cand[e.from] ??= []).push(e.to);
const where = id => decl[id] ? `${decl[id].path}:${decl[id].start_line}` : '?';
const show = t => `${t.label} (${t.at})`;
const byShow = new Map(); for (const [id, n] of Object.entries(g.nodes)) if (n.category === 'symbol' && decl[id]) byShow.set(`${n.label} (${where(id)})`, id);
const A = new Map();
for (const [owner, r] of Object.entries(d.res)) {
  const by = new Map();
  for (const l of r.links) { const k = `${l.path}@${l.sb}`; const a = by.get(k) ?? { owner, ownerLabel: r.owner.label, text: l.text, line: l.line, path: l.path, sb: l.sb, resolution: l.resolution, targets: [] }; a.targets.push(l.to); by.set(k, a); }
  for (const u of r.unresolved) by.set(`${u.path}@${u.sb}`, { owner, ownerLabel: r.owner.label, text: u.text, line: u.line, path: u.path, sb: u.sb, resolution: u.reason, targets: [] });
  for (const [k, a] of by) if (!A.has(k)) A.set(k, a);
}
const projectTypes = new Set(Object.values(g.nodes).filter(n => n.category === 'symbol' && ['struct', 'enum', 'trait'].includes(n.kind)).map(n => n.label.split('::').pop()));
const LINKED = new Set(['unique', 'import', 'receiver', 'type', 'trait_default']);
const buf = {}; const src = p => buf[p] ??= fs.readFileSync(SRC + p);
const isTest = (p, owner) => /tests?_|\/tests?\//.test(owner) || /(^|\/)tests?\//.test(p);
// The type a declared return gives, by the proposed rule.
function retType(text, target) {
  let t = text.trim(), ref = false;
  while (t.startsWith('&')) { ref = true; t = t.replace(/^&('\w+\s+)?(mut\s+)?/, '').trim(); }
  const head = t.split('<')[0].split('::').pop().trim();
  if (['Option', 'Result', 'Box', 'Rc', 'Arc', 'Vec'].includes(head) || t.startsWith('(') || t.startsWith('[') || t.startsWith('impl ') || t.startsWith('dyn ')) return { invólucro: head || t };
  if (head === 'Self') {
    const c = parent[target]; if (!c) return { self_sem_impl: true };
    const lbl = g.nodes[c].label; if (g.nodes[c].kind !== 'impl') return { self_de_trait: lbl };
    const ty = (lbl.includes(' for ') ? lbl.split(' for ')[1] : lbl.replace(/^impl /, '')).split('<')[0].split('::').pop().trim();
    return { tipo: ty, ref, projeto: projectTypes.has(ty) };
  }
  return { tipo: head, ref, projeto: projectTypes.has(head) };
}
const out = [], notas = {};
const bump = k => notas[k] = (notas[k] ?? 0) + 1;
for (const e of g.edges) {
  if (e.kind !== 'observes') continue;
  const o = g.nodes[e.to]; if (o.kind !== 'method_call_observed') continue;
  const m = o.label.match(/^([a-z_]\w*)\s*\.\s*(\w+)/); if (!m) continue;
  const x = m[1]; if (x === 'self') continue;
  const dd = decl[e.from]; if (!dd) continue;
  const text = src(dd.path).slice(dd.start_byte, e.evidence.start_byte).toString('latin1');
  const re = new RegExp(String.raw`let\s+(?:mut\s+)?${x}\b\s*(:[^=;]+)?=\s*`, 'g');
  let last = null, mm; while ((mm = re.exec(text))) last = mm;
  if (!last || last[1]) continue;
  const rhsAt = dd.start_byte + last.index + last[0].length;
  const rhs = text.slice(last.index + last[0].length).split(';')[0].trim();
  let forma = null;
  if (/^((?:\w+::)*)([A-Z]\w*)(?:::<[^>]*>|<[^>]*>)?::(\w+)\s*\(/.test(rhs)) forma = 'T::f';
  else if (/^((?:\w+::)*)([a-z_]\w*)\s*\(/.test(rhs)) forma = 'f()';
  if (!forma) continue;
  const rt = (rts[`${e.evidence.path}@${e.evidence.start_byte}`] ?? []).find(([end]) => end === e.evidence.start_byte + Buffer.byteLength(x));
  const gravado = rt ? { regra: g.nodes[rt[1]].type_rule, candidatos: (cand[rt[1]] ?? []).map(t => g.nodes[t].label), nome: g.nodes[rt[1]].label } : null;
  const call = A.get(`${e.evidence.path}@${e.evidence.start_byte}`);
  const v = A.get(`${dd.path}@${rhsAt}`);
  const r = { forma, at: `${e.evidence.path}:${e.evidence.start_line}`, teste: isTest(e.evidence.path, g.nodes[e.from].label), call: o.label.split('\n')[0].slice(0, 60), rhs: rhs.slice(0, 50), gravado, hoje: call && { resolution: call.resolution, alvos: call.targets.map(show) }, liga: v ? { resolution: v.resolution, alvos: v.targets.map(show) } : null };
  if (!v) { r.fim = 'chamada que liga não achada'; }
  else if (!LINKED.has(v.resolution) || v.targets.length !== 1) { r.fim = `não fecha (${v.resolution}, ${v.targets.length})`; }
  else {
    const f = byShow.get(show(v.targets[0]));
    const ret = (observedBy[f] ?? []).map(o => g.nodes[o]).find(n => n.kind === 'declared_return');
    r.retorno = ret ? ret.label : null;
    r.tipo23c = ret ? retType(ret.label, f) : { sem_retorno: true };
    const antes = forma === 'T::f' ? rhs.match(/([A-Z]\w*)(?:::<[^>]*>|<[^>]*>)?::\w+\s*\(/)[1] : (gravado?.nome ?? null);
    r.fim = r.tipo23c.tipo ? (r.tipo23c.tipo === antes ? 'mesmo tipo de hoje' : `tipo muda: ${antes} -> ${r.tipo23c.tipo}`) : `sem tipo (${Object.keys(r.tipo23c)[0]})`;
  }
  bump(`${forma} ${r.teste ? 'teste' : 'prod'}: ${r.fim.replace(/: .*/, '')}`);
  out.push(r);
}
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));
console.log(Object.fromEntries(Object.entries(notas).sort()));

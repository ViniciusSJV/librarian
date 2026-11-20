// Passo 2.3c, Parada 2 (diagnóstico): conta os casos (a)-(d) da Q1 nos dois projetos, sobre o dump do
// binário do 2.3b. (a) a chamada que liga fecha com um alvo; (b) não fecha, mas todos os candidatos
// declaram o mesmo retorno; (c) sem candidato no projeto: vale o de hoje; (d) os candidatos discordam.
//   node casos23c.js <dump-dir> <tipos-hc.jsonl> <bases.json> <out.json>
'use strict';
const fs = require('fs'), path = require('path');
const I = require('C:/Users/beatl/Documents/Projetos/librarian/docs/reports/fase-2/dados/2.2/imports.js');
const SRC_HC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks', SRC_RT = 'C:/Users/beatl/Documents/Projetos/renderer/';
const [dir, tiposFile, basesFile, outFile] = process.argv.slice(2);
const LINKED = new Set(['unique', 'import', 'receiver', 'type', 'trait_default', 'chosen']);
const isTest = p => /(^|\/)tests?\/|test_|_test\./.test(p);

function load(key) {
  const g = JSON.parse(fs.readFileSync(path.join(dir, `${key}.graph.json`), 'utf8'));
  const d = JSON.parse(fs.readFileSync(path.join(dir, `${key}.calls.json`), 'utf8'));
  const P = { g, declared: {}, parent: {}, children: {}, observedBy: {}, rtAt: new Map(), A: new Map(), A2: new Map(), byShow: new Map() };
  for (const e of g.edges) {
    if (e.kind === 'declares') P.declared[e.to] = e.evidence;
    else if (e.kind === 'contains') { P.parent[e.to] = e.from; (P.children[e.from] ??= []).push(e.to); }
    else if (e.kind === 'observes') {
      (P.observedBy[e.from] ??= []).push(e.to);
      if (g.nodes[e.to].kind === 'receiver_type') (P.rtAt.set(`${e.evidence.path}@${e.evidence.start_byte}-${e.evidence.end_byte}`, e.to));
    }
  }
  P.where = id => P.declared[id] ? `${P.declared[id].path}:${P.declared[id].start_line}` : '?';
  for (const [id, n] of Object.entries(g.nodes)) if (n.category === 'symbol' && P.declared[id]) P.byShow.set(`${n.label} (${P.where(id)})`, id);
  for (const [owner, r] of Object.entries(d.res)) {
    const by = new Map();
    for (const l of r.links) { const k = `${l.path}@${l.sb}-${l.eb}`; const a = by.get(k) ?? { owner, text: l.text, line: l.line, path: l.path, sb: l.sb, resolution: l.resolution, candidates: l.candidates, targets: [] }; a.targets.push(l.to); by.set(k, a); }
    for (const u of r.unresolved) by.set(`${u.path}@${u.sb}-${u.eb}`, { owner, text: u.text, line: u.line, path: u.path, sb: u.sb, resolution: u.reason, candidates: u.candidates, targets: [] });
    for (const [k, a] of by) { if (!P.A.has(k)) P.A.set(k, a); if (!P.A2.has(`${a.path}@${a.sb}`)) P.A2.set(`${a.path}@${a.sb}`, a); }
  }
  P.id = t => P.byShow.get(`${t.label} (${t.at})`);
  P.ret = id => (P.observedBy[id] ?? []).map(o => g.nodes[o]).find(n => n.kind === 'declared_return')?.label ?? null;
  P.byName = {};
  for (const [id, n] of Object.entries(g.nodes)) if (n.category === 'symbol' && ['function', 'method', 'trait_method'].includes(n.kind)) (P.byName[n.label.split(/::|\./).pop()] ??= []).push(id);
  return P;
}
const show = t => `${t.label} (${t.at})`;

// ---------------- Python
const hc = load('healthchecks-25597e41');
const BASED = new Set(JSON.parse(fs.readFileSync(basesFile, 'utf8')));
const classes = {}, byFileName = {};
for (const [id, n] of Object.entries(hc.g.nodes)) {
  if (n.category !== 'symbol' || n.kind !== 'class' || !hc.declared[id]) continue;
  const name = n.label.split('.').pop(), methods = {};
  for (const c of hc.children[id] ?? []) if (hc.g.nodes[c].kind === 'method') (methods[hc.g.nodes[c].label.split('.').pop()] ??= []).push(c);
  const bases = (hc.observedBy[id] ?? []).filter(o => hc.g.nodes[o].kind === 'base_class' && hc.g.nodes[o].label !== 'object').map(o => hc.g.nodes[o].label);
  classes[id] = { id, label: n.label, methods, bases: bases.length ? bases : (BASED.has(name) ? ['(base)'] : []), at: hc.where(id) };
  if (!n.label.includes('.')) (byFileName[`${hc.declared[id].path}#${n.label}`] ??= []).push(id);
}
const tables = {};
const table = f => tables[f] ??= I.table(fs.readFileSync(path.join(SRC_HC, f), 'utf8'));
function classIn(file, name, hops = 0) {
  const local = byFileName[`${file}#${name}`];
  if (local?.length === 1) return local[0];
  const b = (table(file)[name] ?? []).find(v => !v.nested) ?? (table(file)[name] ?? [])[0];
  if (!b || b.kind !== 'from' || b.name !== name || hops > 3) return null;
  const mod = I.moduleFile(SRC_HC, file, b.module);
  return mod ? classIn(mod, name, hops + 1) : null;
}
function classOfAttr(file, head, name) {
  const b = (table(file)[head] ?? []).find(v => !v.nested) ?? (table(file)[head] ?? [])[0];
  if (!b) return null;
  const mod = b.kind === 'import' ? I.moduleFile(SRC_HC, file, b.module) : I.moduleFile(SRC_HC, file, (/^\.+$/.test(b.module) ? b.module : b.module + '.') + b.name);
  return mod ? classIn(mod, name, 1) : null;
}
function annName(text) {
  let t = text.trim(); if (/^["'].*["']$/.test(t)) t = t.slice(1, -1).trim();
  let m;
  if ((m = t.match(/^(?:typing\.)?Optional\[(.+)\]$/s))) return annName(m[1]);
  if ((m = t.match(/^(?:typing\.)?Union\[(.+)\]$/s))) { const p = m[1].split(',').map(s => s.trim()).filter(s => s !== 'None'); return p.length === 1 ? annName(p[0]) : null; }
  if (t.includes('|')) { const p = t.split('|').map(s => s.trim()).filter(s => s !== 'None'); return p.length === 1 ? annName(p[0]) : null; }
  return /^[A-Za-z_][\w.]*$/.test(t) && t !== 'None' ? t : null;
}
// The type a Python declaration gives as a return: a class id, or a string saying why not.
function pyRet(id) {
  const n = hc.g.nodes[id];
  if (n.kind === 'class') return id;
  const r = hc.ret(id);
  if (!r) return 'sem anotação';
  const name = annName(r);
  if (!name) return `não nomeia (${r})`;
  const file = hc.declared[id].path;
  if (name === 'Self') return hc.parent[id] && hc.g.nodes[hc.parent[id]].kind === 'class' ? hc.parent[id] : 'Self';
  const K = name.includes('.') ? classOfAttr(file, name.split('.')[0], name.split('.').pop()) : classIn(file, name);
  return K ?? `fora (${name})`;
}
const tipos = fs.readFileSync(tiposFile, 'utf8').trim().split('\n').map(JSON.parse).filter(r => r.method && r.via_call && r.type);
const out = { py: [], rs: [] }, notas = { py: {}, rs: {} };
const bump = (l, k) => notas[l][k] = (notas[l][k] ?? 0) + 1;
for (const r of tipos) {
  const a = hc.A.get(`${r.path}@${r.sb}-${r.eb}`), v = hc.A.get(`${r.path}@${r.via_call[0]}-${r.via_call[1]}`);
  if (!a || !v) { bump('py', 'não achada no dump'); continue; }
  let caso, alvos = [];
  if (LINKED.has(v.resolution) && v.targets.length === 1) {
    if (v.targets[0].kind === 'class') { bump('py', 'classe (2.3a)'); continue; }
    caso = 'a'; alvos = [hc.id(v.targets[0])];
  } else if (v.resolution === 'homonym' || (LINKED.has(v.resolution) && v.targets.length > 1)) { caso = 'bd'; alvos = v.targets.map(hc.id); }
  else if (v.resolution === 'above_candidate_limit') {
    const name = r.via_call[2] ? v.text.match(/\.\s*([\p{L}\p{N}_]+)\s*\(/u)?.[1] ?? null : v.text.match(/^([\p{L}\p{N}_]+)/u)?.[1];
    const m = (hc.byName[name] ?? []).filter(id => r.via_call[2] ? hc.g.nodes[id].kind === 'method' : hc.g.nodes[id].kind === 'function');
    caso = 'bd'; alvos = m;
  } else { caso = 'c'; }
  let tipo = null;
  if (caso !== 'c') {
    const rets = [...new Set(alvos.map(pyRet))];
    if (caso === 'bd') caso = rets.length === 1 ? 'b' : 'd';
    tipo = rets.length === 1 ? rets[0] : rets;
  }
  const cls = typeof tipo === 'string' && classes[tipo] ? classes[tipo] : null;
  // effect on the call
  let depois = null;
  if (cls && !cls.bases.includes('Protocol')) {
    const own = (cls.methods[r.name] ?? []).map(id => show({ label: hc.g.nodes[id].label, at: hc.where(id) }));
    if (['unique', 'import', 'receiver', 'chosen'].includes(a.resolution)) {
      if (a.resolution === 'unique' && a.targets[0].kind === 'method' && !own.includes(show(a.targets[0])) && !cls.bases.length && !cls.methods.__getattr__ && !own.length) depois = 'no_candidate_in_project';
    } else if (own.length) depois = `type ${own.join(';')}`;
    else if (!cls.bases.length && !cls.methods.__getattr__ && a.resolution !== 'no_candidate_in_project') depois = 'no_candidate_in_project';
  }
  const antes = `${a.resolution} ${a.targets.map(show).join(';')}`.trim();
  const muda = depois && depois !== antes;
  bump('py', `${caso} ${isTest(r.path) ? 'teste' : 'prod'}${cls ? ' classe' : ''}${muda ? ' MUDA' : ''}`);
  out.py.push({ caso, at: `${r.path}:${a.line}`, key: `${r.path}@${r.sb}-${r.eb}`, teste: isTest(r.path), text: a.text, liga: v.text, liga_res: v.resolution, alvos: alvos.length, tipo: cls ? cls.label : tipo, antes, depois: muda ? depois : null });
}

// ---------------- Rust
const rt = load('renderer-9b78f3a9');
const projectTypes = new Set(Object.values(rt.g.nodes).filter(n => n.category === 'symbol' && ['struct', 'enum', 'trait'].includes(n.kind)).map(n => n.label.split('::').pop()));
const implType = id => { const c = rt.parent[id]; if (!c || rt.g.nodes[c].kind !== 'impl') return null; const l = rt.g.nodes[c].label; return (l.includes(' for ') ? l.split(' for ')[1] : l.replace(/^impl /, '')).split('<')[0].split('::').pop().trim(); };
function rsRet(id) {
  const r = rt.ret(id); if (!r) return 'sem retorno';
  let t = r.trim(); while (t.startsWith('&')) t = t.replace(/^&('\w+\s+)?(mut\s+)?/, '').trim();
  const head = t.split('<')[0].split('::').pop().trim();
  if (['Option', 'Result', 'Box', 'Rc', 'Arc', 'Vec'].includes(head) || /^[(\[]|^impl |^dyn /.test(t)) return `invólucro ${head || t}`;
  if (head === 'Self') return implType(id) ?? 'Self de trait';
  return head;
}
const buf = {}; const src = p => buf[p] ??= fs.readFileSync(SRC_RT + p);
const cand = {}; for (const e of rt.g.edges) if (e.kind === 'name_candidate') (cand[e.from] ??= []).push(e.to);
for (const e of rt.g.edges) {
  if (e.kind !== 'observes') continue;
  const o = rt.g.nodes[e.to]; if (o.kind !== 'method_call_observed') continue;
  const m = o.label.match(/^([a-z_]\w*)\s*\.\s*(\w+)/); if (!m || m[1] === 'self') continue;
  const x = m[1], dd = rt.declared[e.from]; if (!dd) continue;
  const text = src(dd.path).slice(dd.start_byte, e.evidence.start_byte).toString('latin1');
  const re = new RegExp(String.raw`let\s+(?:mut\s+)?${x}\b\s*(:[^=;]+)?=\s*`, 'g');
  let last = null, mm; while ((mm = re.exec(text))) last = mm;
  if (!last || last[1]) continue;
  const rhs = text.slice(last.index + last[0].length).split(';')[0].trim();
  // only T::f(..) or f(..) as the whole value (not a method chain)
  let km = rhs.match(/^((?:\w+::)*?)(?:([A-Z]\w*)(?:::<[^>]*>|<[^>]*>)?::)?(\w+)\s*\(/);
  if (!km) continue;
  const close = (() => { let dpt = 0; for (let i = rhs.indexOf('('); i < rhs.length; i++) { if (rhs[i] === '(') dpt++; else if (rhs[i] === ')' && --dpt === 0) return i; } return -1; })();
  if (close < 0 || rhs.slice(close + 1).trim() !== '') continue;
  const T = km[2] ?? null, f = km[3];
  const forma = T ? 'T::f' : 'f()';
  const rhsAt = dd.start_byte + last.index + last[0].length;
  const v = rt.A2.get(`${dd.path}@${rhsAt}`), call = rt.A.get(`${e.evidence.path}@${e.evidence.start_byte}-${e.evidence.end_byte}`) ?? rt.A2.get(`${e.evidence.path}@${e.evidence.start_byte}`);
  const rtNode = [...rt.rtAt.entries()].length && rt.rtAt.get(`${e.evidence.path}@${e.evidence.start_byte}-${e.evidence.start_byte + Buffer.byteLength(x)}`);
  const hoje = rtNode ? `${rt.g.nodes[rtNode].type_rule}:${rt.g.nodes[rtNode].label}` : null;
  let caso, alvos = [];
  if (!v) { bump('rs', 'liga não achada'); continue; }
  if (LINKED.has(v.resolution) && v.targets.length === 1) { caso = 'a'; alvos = [rt.id(v.targets[0])]; }
  else {
    // the rule's targets: by the path's type, or the homonyms admitted, or all of the name
    if (T) alvos = (rt.byName[f] ?? []).filter(id => rt.g.nodes[id].kind !== 'function' && (implType(id) === T || (rt.g.nodes[rt.parent[id]]?.kind === 'trait' && false)));
    else alvos = v.resolution === 'homonym' ? v.targets.map(rt.id) : (rt.byName[f] ?? []).filter(id => rt.g.nodes[id].kind === 'function');
    caso = alvos.length ? 'bd' : 'c';
  }
  let tipo = null;
  if (caso !== 'c') { const rets = [...new Set(alvos.map(rsRet))]; if (caso === 'bd') caso = rets.length === 1 ? 'b' : 'd'; tipo = rets.length === 1 ? rets[0] : rets; }
  const antesTipo = T ?? (hoje ? hoje.split(':')[1] : null);
  bump('rs', `${forma} ${caso} ${isTest(e.evidence.path) || /tests?_/.test(rt.g.nodes[e.from].label) ? 'teste' : 'prod'}`);
  out.rs.push({ forma, caso, at: `${e.evidence.path}:${e.evidence.start_line}`, call: o.label.split('\n')[0].slice(0, 50), rhs: rhs.slice(0, 40), liga_res: v.resolution, alvos: alvos.length, tipo, gravado_hoje: hoje, hoje: call ? `${call.resolution} ${call.targets.map(show).join(';')}` : null });
}
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));
console.log('PY', Object.fromEntries(Object.entries(notas.py).sort()));
console.log('RS', Object.fromEntries(Object.entries(notas.rs).sort()));

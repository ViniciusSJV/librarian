// Passo 2.4c, Parada 1 (gerado por gera.js a partir do ../2.4b/py24b.js; diagnóstico, não é código de produção).
// para cada x.m(), o tipo estático do receptor (self, cls, super(), parâmetro, variável, construtor, retorno,
// campo, propriedade) e a busca de m na MRO (C3), com a base de fora como elemento opaco.
// Lê o dump de /api/calls (dump23c.js) e o texto dos arquivos.   node py24b.js <dump-dir> <out.json>
'use strict';
const fs = require('fs'), path = require('path');
const { load } = require('../2.4/load.js');
const I = require('../2.2/imports.js');
const SRC = process.env.SRC24B ?? 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const [dir, outFile] = process.argv.slice(2);
const P = load(dir, process.env.KEY24B ?? 'healthchecks-25597e41');
const { N, declared, parent, children, show } = P;
const LINKED = new Set(['unique', 'import', 'scope', 'receiver', 'type', 'trait_default', 'chosen']);
const cache = {}, tabs = {};
const buf = f => cache[f] ??= fs.readFileSync(path.join(SRC, f));
const slice = (f, a, b) => buf(f).subarray(a, b).toString('utf8');
const table = f => tabs[f] ??= I.table(buf(f).toString('utf8'));
const sym = id => N[id]?.category === 'symbol';
const last = l => l.split('.').pop();
const topLevel = {};
for (const [id, n] of Object.entries(N)) if (sym(id) && n.language === 'python' && declared[id] && ['function', 'class'].includes(n.kind) && !n.label.includes('.')) (topLevel[`${declared[id].path}#${n.label}`] ??= []).push(id);
for (const k of Object.keys(topLevel)) topLevel[k].sort((a, b) => declared[a].start_byte - declared[b].start_byte);
const classes = Object.keys(N).filter(id => sym(id) && N[id].kind === 'class' && N[id].language === 'python' && declared[id]);

function fromImport(file, module, name, hops = 0) {
  const mod = I.moduleFile(SRC, file, module);
  if (!mod) return { kind: 'external', key: `${module}.${name}` };
  const t = topLevel[`${mod}#${name}`];
  if (t?.length) return { kind: 'project', id: t[t.length - 1] };
  const re = (table(mod)[name] ?? []).filter(b => !b.nested && b.kind === 'from');
  if (re.length && hops < 3) return fromImport(mod, re.at(-1).module, re.at(-1).name, hops + 1);
  return { kind: 'external', key: `${module}.${name}` };
}
// A class name written in `file`. An annotation sees a TYPE_CHECKING (indented) import; a base does not.
function named(file, text, forBase) {
  text = text.split('[')[0].trim();
  if (text === 'object') return { kind: 'object' };
  if (!text.includes('.')) {
    const imps = (table(file)[text] ?? []).filter(b => !forBase || !b.nested);
    const own = topLevel[`${file}#${text}`];
    if (own?.length) return { kind: 'project', id: own.at(-1), risk: imps.length ? 'import-e-def' : undefined };
    const imp = imps.filter(b => !b.nested).at(-1) ?? imps.at(-1);
    if (imp?.kind === 'from') return fromImport(file, imp.module, imp.name);
    const onlyNested = forBase && (table(file)[text] ?? []).length > 0;
    return { kind: 'external', key: text, risk: onlyNested ? 'so-import-indentado' : undefined };
  }
  const parts = text.split('.'), cls = parts.pop();
  const imp = (table(file)[parts[0]] ?? [])[0];
  if (!imp) {
    const inner = classes.find(id => declared[id].path === file && N[id].label === text); // Outer.Inner
    return inner ? { kind: 'project', id: inner } : { kind: 'external', key: text };
  }
  let mod = imp.kind === 'import' ? (imp.dotted ?? imp.module) : (/^\.+$/.test(imp.module) ? imp.module : imp.module + '.') + imp.name;
  if (parts.length > 1 && imp.kind === 'from') mod += '.' + parts.slice(1).join('.');
  const f = I.moduleFile(SRC, file, mod);
  const id = f && topLevel[`${f}#${cls}`]?.at(-1);
  return id ? { kind: 'project', id } : { kind: 'external', key: `${mod}.${cls}` };
}
const Q = String.fromCharCode(39), D = String.fromCharCode(34);
// The end of a string literal that starts at s[i] (index after it).
function strEnd(s, i) {
  const q = s.startsWith(s[i].repeat(3), i) ? s[i].repeat(3) : s[i];
  i += q.length;
  while (i < s.length && !s.startsWith(q, i)) i += s[i] === '\\' ? 2 : 1;
  return i + q.length;
}
function balanced(s, i) {
  let d = 0;
  while (i < s.length) {
    const c = s[i];
    if (c === Q || c === D) { i = strEnd(s, i); continue; }
    if (c === '#') { while (i < s.length && s[i] !== '\n') i++; continue; }
    if ('([{'.includes(c)) d++;
    if (')]}'.includes(c)) { d--; if (!d) return i; }
    i++;
  }
  return -1;
}
function splitTop(s) {
  const out = []; let d = 0, cur = '', i = 0;
  while (i < s.length) {
    const c = s[i];
    if (c === Q || c === D) { const e = strEnd(s, i); cur += s.slice(i, e); i = e; continue; }
    if (c === ',' && !d) { out.push(cur); cur = ''; i++; continue; }
    if ('([{'.includes(c)) d++;
    if (')]}'.includes(c)) d--;
    cur += c; i++;
  }
  if (cur.trim()) out.push(cur);
  return out.map(x => x.trim()).filter(Boolean);
}
const decos = {};
function decorators(id) {
  if (decos[id]) return decos[id];
  const d = declared[id], pre = slice(d.path, 0, d.start_byte).split('\n');
  const out = [], own = slice(d.path, d.start_byte, d.end_byte);
  const head = own.slice(0, own.search(/\b(def|class)\s/));
  for (const m of head.matchAll(/@([\w.]+)/g)) out.push(m[1]);
  for (let i = pre.length - 2; i >= 0 && /^\s*(@|\)|#)/.test(pre[i]); i--) { const m = pre[i].match(/^\s*@([\w.]+)/); if (m) out.push(m[1]); }
  return decos[id] = out;
}
const paramsAt = s => s.indexOf('(', s.search(/\bdef\s/));
function retOf(id) {
  const d = declared[id], s = slice(d.path, d.start_byte, d.end_byte), c = balanced(s, paramsAt(s));
  const m = s.slice(c + 1).match(/^\s*->\s*([^:]+):/);
  return m ? m[1].trim() : null;
}
// annotation -> the written type name, as annotation() of python_types.rs
function ann(t) {
  if (!t) return null;
  t = t.trim();
  if ((t[0] === Q || t[0] === D) && t.at(-1) === t[0]) t = t.slice(1, -1).trim();
  const u = splitTop(t.replace(/\|/g, ','));
  if (t.includes('|') && !t.includes('[')) {
    if (u.length === 2 && u.includes('None')) return ann(u.find(x => x !== 'None'));
    return null;
  }
  const o = t.match(/^(?:typing\.)?Optional\[(.*)\]$/s); if (o) return ann(o[1]);
  const an = t.match(/^(?:typing\.)?Annotated\[(.*)\]$/s); if (an) return ann(splitTop(an[1])[0]);
  return /^[A-Za-z_][\w.]*$/.test(t) && t !== 'None' ? t : null;
}
const info = {};
for (const c of classes) {
  const ev = declared[c], body = slice(ev.path, ev.start_byte, ev.end_byte);
  const hm = body.match(/(?:^|\n)([ \t]*)class\s+\w+\s*(\(|:)/);
  let written = [];
  if (hm && hm[2] === '(') { const i = hm.index + hm[0].length - 1; written = splitTop(body.slice(i + 1, balanced(body, i))); }
  const bases = written.filter(b => !/^\w+\s*=[^=]/.test(b)).map(b => ({ text: b, ...(/^[\w.]+(\[.*\])?$/s.test(b) ? named(ev.path, b, true) : { kind: 'external', key: 'expr:' + b }) })).filter(b => b.kind !== 'object');
  const methods = {}, other = new Set(), fields = {};
  for (const k of children[c] ?? []) {
    if (N[k].kind === 'method') (methods[last(N[k].label)] ??= []).push(k);
    else if (N[k].kind === 'class') other.add(last(N[k].label));
  }
  const line = slice(ev.path, 0, ev.start_byte).split('\n').pop();
  const ind = (line.match(/^\s*/) ?? [''])[0].length;
  for (const m of body.matchAll(/^([ \t]+)(\w+)[ \t]*(?::[ \t]*([^=\n]+?))?[ \t]*(=(?!=)|$)/gm)) {
    if (m[1].length !== ind + 4 || (!m[4] && !m[3])) continue;
    if (m[4] === '=') other.add(m[2]);
    if (m[3] && !['else', 'try', 'finally'].includes(m[2])) (fields[m[2]] ??= []).push(ann(m[3]));
  }
  const init = methods.__init__?.at(-1), params = {};
  if (init) {
    const s = slice(ev.path, declared[init].start_byte, declared[init].end_byte), o = paramsAt(s);
    for (const p of splitTop(s.slice(o + 1, balanced(s, o)))) { const m = p.match(/^\*{0,2}(\w+)\s*:\s*([^=]+)/); if (m) params[m[1]] = ann(m[2]); }
  }
  const inInit = ix => { const b = ev.start_byte + Buffer.byteLength(body.slice(0, ix)); return init && declared[init].start_byte <= b && b < declared[init].end_byte; };
  for (const m of body.matchAll(/\bself\.(\w+)[ \t]*(?::[ \t]*([^=\n]+))?=(?!=)[ \t]*([^\n]*)/g)) {
    other.add(m[1]);
    const v = m[3].trim();
    if (m[2]) (fields[m[1]] ??= []).push(ann(m[2]));
    else if (v === 'None') { /* does not count */ }
    else if (!inInit(m.index)) (fields[m[1]] ??= []).push(null); // 2.3a: only __init__ gives a type
    else if (/^\w+$/.test(v) && params[v]) (fields[m[1]] ??= []).push(params[v]);
    else { const cm = v.match(/^([A-Za-z_][\w.]*)\(/); (fields[m[1]] ??= []).push(cm ? 'call:' + cm[1] : null); }
  }
  info[c] = { id: c, label: N[c].label, file: ev.path, bases, methods, other, fields, getattr: !!(methods.__getattr__ || methods.__getattribute__), protocol: bases.some(b => /Protocol\b/.test(b.text)), meta: written.filter(b => /^metaclass\s*=/.test(b)), decos: decorators(c), news: !!methods.__new__ };
}
function mro(c, seen = []) {
  if (seen.includes(c)) return null;
  const i = info[c], seqs = [];
  const el = b => b.kind === 'project' && info[b.id] ? b.id : `EXT:${b.key ?? b.text}`;
  for (const b of i.bases) {
    const e = el(b);
    if (e.startsWith('EXT:')) seqs.push([e]);
    else { const m = mro(e, [...seen, c]); if (!m) return null; seqs.push([...m]); }
  }
  seqs.push(i.bases.map(el));
  const out = [c];
  for (;;) {
    const live = seqs.filter(s => s.length);
    if (!live.length) return out;
    const h = live.map(s => s[0]).find(h => !live.some(t => t.indexOf(h) > 0));
    if (!h) return null;
    out.push(h);
    for (const s of live) if (s[0] === h) s.shift();
  }
}
// { found } | { external } (an outside base comes first) | { undecided } | { none }
function lookup(c, name, skip = 0) {
  const m = mro(c);
  if (!m) return { undecided: 'mro' };
  for (const k of m.slice(skip)) {
    if (k.startsWith('EXT:')) return { external: k.slice(4) };
    const i = info[k];
    if (i.protocol) return { undecided: 'protocol' };
    if (i.methods[name]) return { found: i.methods[name], at: k };
    if (i.other.has(name)) return { undecided: 'o nome é atributo, não método', at: k };
    if (i.getattr) return { undecided: 'getattr' };
  }
  return { none: true };
}
const descendants = c => classes.filter(d => d !== c && (mro(d) ?? []).includes(c));

const byStart = new Map();
for (const c of P.calls.values()) if (c.lang === 'python' && c.path) { const k = `${c.path}@${c.sb}`; if (!byStart.has(k)) byStart.set(k, []); byStart.get(k).push(c); }
const up = (s, ks) => { for (let x = s; x && sym(x); x = parent[x]) if (ks.includes(N[x].kind)) return x; return null; };
const classOfMethod = m => { const p = parent[m]; return p && N[p].kind === 'class' ? p : null; };
function firstParam(m) { const d = declared[m], s = slice(d.path, d.start_byte, d.end_byte), o = paramsAt(s); return (splitTop(s.slice(o + 1, balanced(s, o)))[0] ?? '').split(/[:=]/)[0].trim(); }
// what the return of a call (as the binary answers it) gives
function retType(call) {
  if (call.rel === 'instantiates' && call.targets.length === 1 && info[call.targets[0]]) return { cls: call.targets[0], rule: 'construtor' };
  const ts = call.targets.filter(t => ['function', 'method'].includes(N[t].kind));
  if (!ts.length) return null;
  const rs = ts.map(t => {
    const r = ann(retOf(t));
    if (!r) return null;
    if (r === 'Self') return classOfMethod(t);
    const n = named(declared[t].path, r);
    return n.kind === 'project' && info[n.id] ? n.id : null;
  });
  return rs.every(r => r && r === rs[0]) ? { cls: rs[0], rule: 'retorno' } : null;
}
const wholeEnd = (f, c, lim) => {
  if (c.kind === 'method_call_observed') return c.eb;
  const t = slice(f, c.eb, lim), o = t.indexOf('('), b = balanced(t, o);
  return c.eb + Buffer.byteLength(t.slice(0, b + 1));
};
let depth = 0;
function typeOf(f, owner, sb, eb) {
  if (depth > 5) return null;
  depth++;
  try { return typeOfExpr(f, owner, sb, eb); } finally { depth--; }
}
function typeOfExpr(f, owner, sb, eb) {
  const e = slice(f, sb, eb);
  if (/^super\(/.test(e) && balanced(e, 5) === e.length - 1) {
    const m = up(owner, ['method']), c = m && classOfMethod(m);
    return c ? { cls: c, rule: /^super\(\s*\)$/.test(e) ? 'super' : 'super(C, self)', skip: 1 } : null;
  }
  if (/^\w+$/.test(e)) {
    const meth = up(owner, ['method']);
    if (meth && classOfMethod(meth) && firstParam(meth) === e) {
      const ds = decorators(meth);
      if (ds.includes('staticmethod')) return null;
      return { cls: classOfMethod(meth), rule: ds.includes('classmethod') ? 'cls' : 'self' };
    }
    for (let fn = up(owner, ['function', 'method']); fn; fn = up(parent[fn], ['function', 'method'])) {
      const d = declared[fn], s = slice(f, d.start_byte, d.end_byte), o = paramsAt(s), cl = balanced(s, o);
      const par = splitTop(s.slice(o + 1, cl)).find(p => p.replace(/^\*+/, '').split(/[:=]/)[0].trim() === e);
      const body = s.slice(cl);
      const annv = body.match(new RegExp(`^[ \\t]*${e}[ \\t]*:[ \\t]*([^=\\n]+)`, 'm'));
      const w = par && !par.startsWith('*') && par.includes(':') ? par.split(':').slice(1).join(':').split('=')[0] : annv ? annv[1] : null;
      if (w) {
        const t = ann(w);
        if (!t) return null;
        const n = named(f, t);
        return n.kind === 'project' && info[n.id] ? { cls: n.id, rule: par && par.includes(':') ? 'parametro' : 'anotacao' } : { outside: n.key };
      }
      if (par) return null;
      if (new RegExp(`\\bfor\\s+[^\\n]*\\b${e}\\b[^\\n]*\\bin\\b|\\bas\\s+${e}\\b|\\b${e}\\s*:=|^\\s*[^=\\n#(]*,\\s*${e}\\b[^=\\n(]*=[^=]|^\\s*${e}\\s*,[^=\\n(]*=[^=]`, 'm').test(body)) return null;
      const asg = [...body.matchAll(new RegExp(`(^|\\n)([ \\t]*)${e}[ \\t]*=(?!=)[ \\t]*`, 'g'))];
      if (!asg.length) continue;
      let r;
      for (const a of asg) {
        const vs = a.index + a[0].length;
        if (/^None\s*(#.*)?(\n|$)/.test(body.slice(vs))) continue;
        const bs = d.start_byte + Buffer.byteLength(s.slice(0, cl)) + Buffer.byteLength(body.slice(0, vs));
        const hit = (byStart.get(`${f}@${bs}`) ?? []).map(c => ({ c, end: wholeEnd(f, c, d.end_byte) })).sort((x, y) => y.end - x.end)[0];
        if (!hit || !/^\s*(#.*)?$/.test(slice(f, hit.end, d.end_byte).split('\n')[0])) return null;
        const t = retType(hit.c);
        if (!t || (r && r.cls !== t.cls)) return null;
        r = t;
      }
      return r ?? null;
    }
    return null;
  }
  if (e.endsWith(')')) {
    for (const c of byStart.get(`${f}@${sb}`) ?? []) if (wholeEnd(f, c, eb) === eb) return retType(c);
    return null;
  }
  const am = e.match(/^(.*)\.(\w+)$/s);
  if (!am) return null;
  const b = typeOf(f, owner, sb, sb + Buffer.byteLength(am[1]));
  return b?.cls && !b.skip ? attribute(b.cls, am[2]) : null;
}
function attribute(cls, name) {
  const m = mro(cls);
  if (!m) return null;
  for (const [i, k] of m.entries()) {
    if (k.startsWith('EXT:')) return null;
    const x = info[k], meth = x.methods[name];
    if (meth) {
      const ds = meth.length === 1 ? decorators(meth[0]) : [];
      if (!ds.some(d => /^(property|cached_property|functools\.cached_property)$/.test(d))) return null;
      const t = ann(retOf(meth[0]));
      if (!t) return null;
      const n = named(x.file, t);
      return n.kind === 'project' && info[n.id] ? { cls: n.id, rule: ds.includes('property') ? 'propriedade' : 'cached_property', via: show(meth[0]) } : { outside: n.key };
    }
    const fs_ = x.fields[name];
    if (fs_) {
      if (!fs_.every(v => v && v === fs_[0])) return null;
      const n = named(x.file, fs_[0].replace('call:', ''));
      return n.kind === 'project' && info[n.id] ? { cls: n.id, rule: i ? 'campo-herdado' : 'campo' } : null;
    }
    if (x.other.has(name)) return null;
  }
  return null;
}
// The call's receiver and name: the argument list is the one that ends the text; scanned forward.
function parts(text) {
  let i = 0, open = -1;
  while (i < text.length) {
    const c = text[i];
    if (c === Q || c === D) { i = strEnd(text, i); continue; }
    if ('([{'.includes(c)) {
      const e = balanced(text, i);
      if (e < 0) return null;
      if (e === text.length - 1 && c === '(') { open = i; break; }
      i = e + 1; continue;
    }
    i++;
  }
  if (open < 0) return null;
  const m = text.slice(0, open).match(/^(.*?)\s*\.\s*(\w+)\s*$/s);
  return m ? { receiver: m[1], name: m[2] } : null;
}
// What the resolver reads (call_parts): backwards, without reading strings.
function back(text) {
  const t = text.trimEnd();
  let d = 0, o = -1;
  for (let i = t.length - 1; i >= 0; i--) {
    if (t[i] === ')') d++;
    else if (t[i] === '(') { d--; if (!d) { o = i; break; } if (d < 0) return null; }
  }
  return o < 0 ? null : (t.slice(0, o).match(/(\w+)\s*$/) ?? [])[1] ?? null;
}
const R = require('../2.4b/regras12.js')({ N, P, info, mro, lookup, descendants, slice, declared, show, decorators, balanced, paramsAt, retOf, classes });
const R4C = require('./regra24c.js')(() => ({ P, N, info, mro, lookup, descendants, classes, show, decorators, R, up, last, sym }));
const rows = [], misread = [], check = { igual: 0, dif: [], semTipo: [] };
for (const c of P.calls.values()) {
  if (c.lang !== 'python' || c.kind !== 'method_call_observed' || !c.path) continue;
  const text = slice(c.path, c.sb, c.eb), p = parts(text), at = `${c.path}:${c.line}`;
  if (!p) { misread.push({ at, text: text.slice(0, 60), simulador: true }); continue; }
  if (back(text) !== p.name) misread.push({ at, text: text.slice(0, 80), nome: p.name, lido: back(text), res: c.res });
  const t = typeOf(c.path, c.owner, c.sb, c.sb + Buffer.byteLength(p.receiver));
  if (['type', 'receiver'].includes(c.res)) {
    if (!t?.cls) check.semTipo.push({ at, text: text.slice(0, 60), hoje: c.targets.map(show), regra: c.rule });
    else {
      const r = lookup(t.cls, p.name, t.skip ?? 0);
      if (r.found?.includes(c.targets[0])) check.igual++;
      else check.dif.push({ at, text: text.slice(0, 60), hoje: c.targets.map(show), sim: `${t.rule} ${N[t.cls].label} ${JSON.stringify(r)}` });
    }
  }
  if (!t?.cls) continue;
  const r = lookup(t.cls, p.name, t.skip ?? 0);
  const own = !t.skip && r.at === t.cls;
  const desc = descendants(t.cls);
  // The proposed limits: super() decides only if every subclass of the project continues to the same
  // method; nothing is dropped through super(), nor when a subclass of the project reaches the name.
  let trava = null;
  const after = d => { const m = mro(d); if (!m) return 'mro'; for (const k of m.slice(m.indexOf(t.cls) + 1)) { if (k.startsWith('EXT:')) return k; if (info[k].methods[p.name]) return k; if (info[k].other.has(p.name) || info[k].getattr) return '?' + k; } return 'nenhum'; };
  if (t.skip && r.found && desc.some(d => after(d) !== r.at)) trava = 'super: outra subclasse continua em outro lugar';
  if (r.none && t.skip) trava = 'super sem achar';
  if (r.none && desc.some(d => { const x = lookup(d, p.name); return x.found || x.external || x.undecided; })) trava = 'uma subclasse alcança o nome';
  R4C(c, p, t, r, text, at);
  let mud;
  if (trava) mud = `igual (${trava}) [${c.res}]`;
  else if (r.found) mud = LINKED.has(c.res) ? (c.targets.length === 1 && r.found.includes(c.targets[0]) ? 'confirma' : 'TROCA-ALVO') : `${c.res} -> type`;
  else if (r.none) mud = c.res === 'no_candidate_in_project' ? 'igual (ninguém declara)' : `${c.res} -> sem (ninguém declara)`;
  else mud = `igual (${r.external ? 'base de fora antes' : r.undecided}) [${c.res}]`;
  // Rule 1: the method found is abstract and a project subclass overrides it (not through super(), nor a constructor).
  const abs = r.found && !trava ? R.abstract(r.found.at(-1)) : null;
  const over = abs && !t.skip && t.rule !== 'construtor' && !/fora da regra/.test(abs) ? R.overrides(t.cls, p.name) : [];
  if (over.length) mud = `${c.res} -> homonym (abstrato: ${over.length} sobrescritas)`;
  // Rule 2: candidate methods of a class the receiver's chain, and its subclasses', does not have.
  let r2 = null;
  if (!over.length && (!r.found || trava)) {
    const reach = R.reachable(t.cls, t.skip ?? 0);
    const edges = P.cands[c.occ] ?? [];
    const above = c.res === 'above_candidate_limit';
    const all = above ? Object.keys(N).filter(id => sym(id) && N[id].kind === 'method' && N[id].language === 'python' && last(N[id].label) === p.name) : c.res === 'no_candidate_in_project' ? [] : edges.filter(x => N[x].kind === 'method');
    if (reach && all.length) {
      const keep = all.filter(x => reach.has(parent[x]));
      if (keep.length < all.length) r2 = { antes: all.length, ficam: keep.map(show), fora: all.filter(x => !reach.has(parent[x])).map(show), incerta: R.opaqueUnknown(t.cls, t.skip ?? 0) };
    }
  }
  rows.push({
    r2, abs, over: over.length ? over.map(show) : undefined, baseDesconhecida: r.external,
    occ: `${c.path}@${c.sb}-${c.eb}`, at, text: text.replace(/\s+/g, ' ').slice(0, 90), nome: p.name, test: c.test, dono: N[c.owner].label, forma: t.rule, via: t.via,
    classe: N[t.cls].label, where: own ? 'própria' : r.found ? 'herdado' : '-', multi: r.found?.length > 1 || undefined, mro: (mro(t.cls) ?? ['?']).map(k => N[k] ? N[k].label : k),
    hoje: c.res, hojeAlvos: c.targets.map(show), nCand: c.cands ?? c.amb ?? 0, alvo: r.found ? show(r.found.at(-1)) : null, fora: r.external, mud,
    subclassesQueSobrescrevem: r.found ? desc.filter(d => info[d].methods[p.name]).map(d => N[d].label) : undefined,
    subclassesQueAlcancam: r.none ? desc.filter(d => { const x = lookup(d, p.name); return x.found || x.external; }).map(d => N[d].label) : undefined,
  });
}
require('./fim24c.js')(R4C, rows, check, outFile, { P, N, info, mro, classes, show, parts, slice, named, parent, declared, up, descendants, lookup, last, sym });

// Passo 2.4, Parada 1 (diagnóstico; não é código de produção). Python (Healthchecks):
//  N0-livre: a chamada simples f() resolvida pelo escopo (def do arquivo, def aninhada, from p import f);
//  N8: alias (from p import C as D, import a.b as c, alias de tipo, alias local);
//  N5: herança (método herdado por self.m(), x.m() de tipo escrito, super().m(), __init__ herdado).
// Lê o dump de /api/calls (dump23c.js) e o texto dos arquivos; simula a regra proposta.
//   node py24.js <dump-dir> <out.json>
'use strict';
const fs = require('fs'), path = require('path');
const { load } = require('./load.js');
const I = require('C:/Users/beatl/Documents/Projetos/librarian/docs/reports/fase-2/dados/2.2/imports.js');
const SRC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const KEY = 'healthchecks-25597e41';
const [dir, outFile] = process.argv.slice(2);
const P = load(dir, KEY);
const { N, declared, parent, children, observes, cands, where, show } = P;
const LINKED = new Set(['unique', 'import', 'receiver', 'type', 'trait_default', 'chosen']);

const srcCache = {};
const src = f => srcCache[f] ??= fs.readFileSync(path.join(SRC, f), 'utf8');
const tables = {};
const table = f => tables[f] ??= I.table(src(f));
const lineOf = (f, byte) => Buffer.from(src(f)).subarray(0, byte).toString('utf8').split('\n').length;

// Symbols
const sym = id => N[id]?.category === 'symbol';
const fileOf = id => declared[id]?.path;
const topLevel = {}; // file#name -> [ids] (functions and classes declared at module level)
for (const [id, n] of Object.entries(N)) {
  if (!sym(id) || n.language !== 'python' || !declared[id]) continue;
  if (!['function', 'class'].includes(n.kind)) continue;
  if (n.label.includes('.')) continue;
  (topLevel[`${fileOf(id)}#${n.label}`] ??= []).push(id);
}
// in source order: the last def of a name is the one bound when the module finishes loading
for (const k of Object.keys(topLevel)) topLevel[k].sort((a, b) => declared[a].start_byte - declared[b].start_byte);
const classes = Object.keys(N).filter(id => sym(id) && N[id].kind === 'class' && N[id].language === 'python');
const methodsOf = c => {
  const m = {};
  for (const k of children[c] ?? []) if (N[k].kind === 'method') (m[N[k].label.split('.').pop()] ??= []).push(k);
  return m;
};

// What a name written in `file` (at line `line`, inside the symbol `inside`) is bound to, by scope:
// { kind: 'def', id } | { kind: 'project', id } | { kind: 'external', module } | { kind: 'projeto-nao-achado' } | { kind: 'livre' }
// Order (Python): innermost function scope (a nested def, a nested import), then the module (the
// later of a def and an import of the same name), then nothing (builtin, star import).
function resolveFrom(file, module, name, hops = 0) {
  const mod = I.moduleFile(SRC, file, module);
  if (!mod) {
    // `from p import m` with m a module
    const asModule = I.moduleFile(SRC, file, (/^\.+$/.test(module) ? module : module + '.') + name);
    if (asModule) return { kind: 'modulo', file: asModule };
    const top = module.replace(/^\.+/, '').split('.')[0];
    if (!module.startsWith('.') && !fs.existsSync(path.join(SRC, top))) return { kind: 'external', module };
    return { kind: 'projeto-nao-achado', module };
  }
  const local = topLevel[`${mod}#${name}`];
  if (local?.length) return { kind: 'project', id: local[local.length - 1], file: mod, hops };
  if (hops >= 3) return { kind: 'projeto-nao-achado', module };
  const re = (table(mod)[name] ?? []).filter(b => !b.nested && b.kind === 'from');
  if (re.length) return resolveFrom(mod, re[re.length - 1].module, re[re.length - 1].name, hops + 1);
  return { kind: 'projeto-nao-achado', module };
}
function bindingOf(file, name, line, inside) {
  // enclosing function scopes, innermost first
  const chain = [];
  for (let s = inside; s && sym(s); s = parent[s]) chain.push(s);
  for (const s of chain) {
    if (!['function', 'method'].includes(N[s].kind)) continue;
    const ev = declared[s];
    const nestedDef = (children[s] ?? []).find(k => ['function', 'class'].includes(N[k].kind) && N[k].label.split('.').pop() === name);
    if (nestedDef) return { kind: 'def', id: nestedDef, how: 'aninhada' };
    const nestedImp = (table(file)[name] ?? []).filter(b => b.nested && b.line >= ev.start_line && b.line <= ev.end_line);
    if (nestedImp.length) {
      const b = nestedImp[nestedImp.length - 1];
      return b.kind === 'from' ? { ...resolveFrom(file, b.module, b.name), how: 'import-aninhado', alias: b.name !== name ? b.name : undefined } : { kind: 'modulo-chamado', how: 'import-aninhado' };
    }
  }
  const def = topLevel[`${file}#${name}`] ?? [];
  const imps = (table(file)[name] ?? []).filter(b => !b.nested);
  // a nested (if/try/TYPE_CHECKING) import at module level shows as nested in imports.js: count it as risk
  const indented = (table(file)[name] ?? []).filter(b => b.nested && !chain.some(s => declared[s] && b.line >= declared[s].start_line && b.line <= declared[s].end_line));
  const defLine = def.length ? declared[def[def.length - 1]].start_line : -1;
  const imp = imps[imps.length - 1];
  const risk = [];
  if (def.length > 1) risk.push('def-repetida');
  if (imps.length > 1) risk.push('import-repetido');
  if (def.length && imp) risk.push(defLine > imp.line ? 'def-depois-do-import' : 'import-depois-do-def');
  if (indented.length) risk.push('import-indentado-no-modulo');
  if (def.length && (!imp || defLine > imp.line)) return { kind: 'def', id: def[def.length - 1], how: 'modulo', risk };
  if (imp) {
    if (imp.kind === 'from') return { ...resolveFrom(file, imp.module, imp.name), how: 'import', alias: imp.name !== name ? imp.name : undefined, risk };
    return { kind: 'modulo-chamado', how: 'import', risk };
  }
  if (indented.length) {
    const b = indented[indented.length - 1];
    return b.kind === 'from' ? { ...resolveFrom(file, b.module, b.name), how: 'import-indentado', alias: b.name !== name ? b.name : undefined, risk } : { kind: 'modulo-chamado', how: 'import-indentado', risk };
  }
  if ((table(file)['*'] ?? []).length) return { kind: 'livre', how: 'import-estrela', risk };
  return { kind: 'livre', how: 'nada', risk };
}

// ---------- N0-livre: plain calls ----------
const plain = [...P.calls.values()].filter(c => c.lang === 'python' && c.kind === 'call_observed' && !c.text.includes('.') && c.path);
const n0 = [];
for (const c of plain) {
  const b = bindingOf(c.path, c.text, c.line, c.owner);
  const now = { res: c.res, targets: c.targets.map(show), rel: c.rel };
  let after, mudanca;
  const target = ['def', 'project'].includes(b.kind) ? b.id : null;
  if (target) {
    const kindT = N[target].kind;
    if (LINKED.has(c.res) && c.targets.length === 1 && c.targets[0] === target) { after = c.res; mudanca = 'confirma'; }
    else if (LINKED.has(c.res)) { after = 'escopo'; mudanca = 'troca-alvo'; }
    else if (c.res === 'homonym') { after = 'escopo'; mudanca = (cands[c.occ] ?? []).includes(target) ? 'homonym->escopo' : 'homonym->escopo(fora dos candidatos)'; }
    else if (c.res === 'above_candidate_limit') { after = 'escopo'; mudanca = 'teto->escopo'; }
    else { after = 'escopo'; mudanca = 'sem->escopo'; }
    after += kindT === 'class' ? ' (instancia)' : '';
  } else if (['external', 'modulo-chamado'].includes(b.kind)) {
    if (LINKED.has(c.res)) { after = 'no_candidate_in_project'; mudanca = 'link->sem (ligado a import de fora)'; }
    else if (c.res === 'homonym' || c.res === 'above_candidate_limit') { after = 'no_candidate_in_project'; mudanca = `${c.res}->sem (import de fora)`; }
    else { after = c.res; mudanca = 'igual'; }
  } else { after = c.res; mudanca = b.kind === 'livre' ? 'igual (nome livre)' : 'igual (' + b.kind + ')'; }
  n0.push({ at: `${c.path}:${c.line}`, text: c.text, test: c.test, owner: N[c.owner].label, now, binding: { kind: b.kind, how: b.how, to: target ? show(target) : (b.module ?? null), alias: b.alias, risk: b.risk?.length ? b.risk : undefined }, after, mudanca });
}

// ---------- classes, bases and MRO ----------
// A base written in the class's file: the class's own file's top-level class, a from-import, m.X.
function baseClass(file, text, line) {
  const name = text.split('[')[0].trim();
  if (name === 'object') return { kind: 'object' };
  if (!name.includes('.')) {
    const local = topLevel[`${file}#${name}`]?.filter(id => N[id].kind === 'class');
    const imps = (table(file)[name] ?? []);
    const imp = imps.filter(b => !b.nested)[0] ?? imps[0];
    if (local?.length && (!imp || declared[local[0]].start_line > imp.line)) return { kind: 'project', id: local[0] };
    if (imp?.kind === 'from') {
      const r = resolveFrom(file, imp.module, imp.name);
      if (r.kind === 'project' && N[r.id].kind === 'class') return { kind: 'project', id: r.id, alias: imp.name !== name ? imp.name : undefined };
      return { kind: r.kind === 'external' ? 'external' : r.kind, module: imp.module, name: imp.name };
    }
    if (local?.length) return { kind: 'project', id: local[0] };
    return { kind: 'external', module: '(builtin ou livre)', name };
  }
  const [head, ...rest] = name.split('.');
  const imp = (table(file)[head] ?? [])[0];
  if (!imp) return { kind: 'external', module: '?', name };
  let mod = imp.kind === 'import' ? (imp.dotted ?? imp.module) : (/^\.+$/.test(imp.module) ? imp.module : imp.module + '.') + imp.name;
  const cls = rest.pop();
  if (rest.length) mod += '.' + rest.join('.');
  const f = I.moduleFile(SRC, file, mod);
  if (!f) return { kind: 'external', module: mod, name: cls };
  const id = topLevel[`${f}#${cls}`]?.find(x => N[x].kind === 'class');
  return id ? { kind: 'project', id } : { kind: 'projeto-nao-achado', module: mod, name: cls };
}
const info = {};
for (const c of classes) {
  const file = fileOf(c);
  const s = src(file);
  const ev = declared[c];
  const body = Buffer.from(s).subarray(ev.start_byte, ev.end_byte).toString('utf8');
  // bases from the class header, as the extractor reads them (python_types.rs: Name, Attribute, Subscript)
  const hm = body.match(/class\s+\w+\s*\(/);
  const written = [];
  if (hm) {
    let i = hm.index + hm[0].length, depth = 0, cur = '';
    for (; i < body.length; i++) {
      const ch = body[i];
      if (depth === 0 && (ch === ',' || ch === ')')) { if (cur.trim()) written.push(cur.trim()); cur = ''; if (ch === ')') break; continue; }
      if ('([{'.includes(ch)) depth++; if (')]}'.includes(ch)) depth--;
      cur += ch;
    }
  }
  const bases = written.filter(b => !/^\w+\s*=/.test(b)).map(b => b.replace(/\s+/g, ' ')).map(b => ({ text: b, ...baseClass(file, b.split('[')[0], ev.start_line) }));
  info[c] = { id: c, label: N[c].label, file, bases: bases.filter(b => b.kind !== 'object'), methods: methodsOf(c), getattr: /def __getattr(ibute)?__\(/.test(body), protocol: bases.some(b => /Protocol/.test(b.text)) };
}
// C3 linearization over project classes; an external base is an opaque element ("EXT:name").
function mro(c, seen = new Set()) {
  if (seen.has(c)) return [c, 'CICLO'];
  seen.add(c);
  const i = info[c];
  if (!i) return [c];
  const seqs = i.bases.map(b => b.kind === 'project' ? mro(b.id, new Set(seen)) : [`EXT:${b.text}`]);
  seqs.push(i.bases.map(b => b.kind === 'project' ? b.id : `EXT:${b.text}`));
  const out = [c];
  const ss = seqs.map(s => [...s]).filter(s => s.length);
  while (ss.some(s => s.length)) {
    let head = null;
    for (const s of ss) {
      if (!s.length) continue;
      const h = s[0];
      if (!ss.some(t => t.indexOf(h) > 0)) { head = h; break; }
    }
    if (!head) return [...out, 'MRO-INCONSISTENTE'];
    out.push(head);
    for (const s of ss) if (s[0] === head) s.shift();
  }
  return out;
}
// Lookup of a method name along the MRO, starting after `skip` elements (super()).
// { found: id } | { external: 'EXT:..' } (an outside class comes first: may define it) | { none: true } | { getattr }
function lookup(c, name, skip = 0) {
  const m = mro(c).slice(skip);
  for (const k of m) {
    if (k === 'MRO-INCONSISTENTE' || k === 'CICLO') return { incerto: k };
    if (k.startsWith('EXT:')) return { external: k.slice(4) };
    const own = info[k]?.methods[name];
    if (own?.length) return { found: own[own.length - 1], at: k };
    if (info[k]?.getattr) return { getattr: k };
  }
  return { none: true };
}

// ---------- N5: inherited methods ----------
const n5 = [];
const rtByStart = {};
for (const [id, n] of Object.entries(N)) {
  if (n.kind !== 'receiver_type' || n.language !== 'python') continue;
  const ev = P.observer[id]?.ev;
  if (ev) (rtByStart[`${ev.path}@${ev.start_byte}`] ??= []).push({ id, end: ev.end_byte });
}
const methodCalls =[...P.calls.values()].filter(c => c.lang === 'python' && c.kind === 'method_call_observed' && c.path);
const classOfMethod = m => { const p = parent[m]; return p && N[p].kind === 'class' ? p : null; };
const enclosingMethod = s => { for (let x = s; x && sym(x); x = parent[x]) if (N[x].kind === 'method') return x; return null; };
function nameOf(text) {
  // the call's own argument list is the last balanced (...) at the end
  const t = text.trimEnd();
  if (!t.endsWith(')')) return null;
  let depth = 0, open = -1;
  for (let i = t.length - 1; i >= 0; i--) { if (t[i] === ')') depth++; else if (t[i] === '(') { depth--; if (depth === 0) { open = i; break; } } }
  if (open < 0) return null;
  const head = t.slice(0, open);
  const m = head.match(/^(.*)\.([A-Za-z_]\w*)$/s);
  return m ? { receiver: m[1], name: m[2] } : null;
}
const staticLike = m => /@(staticmethod|classmethod)/.test(Buffer.from(src(fileOf(m))).subarray(Math.max(0, declared[m].start_byte - 200), declared[m].start_byte + 40).toString('utf8').split('\n').slice(-4).join('\n'));
for (const c of methodCalls) {
  const parts = nameOf(c.text);
  if (!parts) continue;
  let cls = null, skip = 0, forma = null;
  const meth = enclosingMethod(c.owner);
  if (parts.receiver === 'self' && meth) { cls = classOfMethod(meth); forma = 'self.m()'; }
  else if (parts.receiver === 'super()' && meth) { cls = classOfMethod(meth); skip = 1; forma = 'super().m()'; }
  else if (/^super\(\s*[A-Za-z_]/.test(parts.receiver) && meth) { cls = classOfMethod(meth); skip = 1; forma = 'super(C, self).m()'; }
  else {
    // x.m() with a written receiver type (graph receiver_type: parametro, anotacao, construtor, campo).
    // Not simulated: the type by return or by another file's field (2.3c).
    const rts = (rtByStart[`${c.path}@${c.sb}`] ?? []).filter(r => r.end === c.sb + Buffer.byteLength(parts.receiver));
    const r = rts[0];
    if (!r) continue;
    const clsC = (cands[r.id] ?? []).filter(k => N[k].kind === 'class');
    // the written type, from the owner's text: `x: T` or `x = T(..)` (param, annotation, constructor)
    const own = declared[c.owner] ?? declared[parent[c.owner]];
    const body = own ? Buffer.from(src(c.path)).subarray(own.start_byte, own.end_byte).toString('utf8') : '';
    const recv = parts.receiver.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const tm = body.match(new RegExp('(?:^|[\\s(,])' + recv + '\\s*:\\s*"?([A-Za-z_][\\w.]*)')) ?? body.match(new RegExp('(?:^|\\s)' + recv + '\\s*=\\s*([A-Za-z_][\\w.]*)\\('));
    if (!tm) continue;
    const typeText = tm[1];
    let pick = [];
    if (typeText.includes('.')) {
      const [head, ...rest] = typeText.split('.');
      const b = bindingOf(c.path, head, c.line, c.owner);
      if (b.kind === 'modulo') { const id = topLevel[`${b.file}#${rest.join('.')}`]?.find(k => N[k].kind === 'class'); pick = id ? [id] : []; }
    } else {
      const b = bindingOf(c.path, typeText, c.line, c.owner);
      if (['def', 'project'].includes(b.kind) && N[b.id].kind === 'class') pick = [b.id];
    }
    if (pick.length !== 1) continue;
    cls = pick[0]; forma = 'x.m() (tipo escrito: ' + N[r.id].type_rule + ')';
    if (info[cls]?.methods[parts.name]?.length) continue; // own method: 2.3a already decides
  }
  if (!cls) continue;
  const own = info[cls]?.methods[parts.name];
  if (forma === 'self.m()' && own?.length) {
    // override: the static type (the enclosing class) decides; record if a subclass overrides it
    continue;
  }
  const r = lookup(cls, parts.name, skip);
  let after, mudanca;
  if (r.found) {
    after = 'type -> ' + show(r.found);
    if (LINKED.has(c.res) && c.targets.length === 1 && c.targets[0] === r.found) mudanca = 'confirma';
    else if (LINKED.has(c.res)) mudanca = 'troca-alvo';
    else mudanca = `${c.res}->type`;
  } else if (r.external) { after = c.res; mudanca = 'igual (base de fora antes)'; }
  else if (r.none) {
    if (info[cls].bases.length === 0 && forma === 'self.m()') { after = c.res; mudanca = 'igual (sem base: já é a regra do 2.3a)'; }
    else { after = 'no_candidate_in_project'; mudanca = LINKED.has(c.res) || c.res === 'homonym' || c.res === 'above_candidate_limit' ? `${c.res}->sem (nenhuma classe do MRO tem o método)` : 'igual'; }
  } else { after = c.res; mudanca = 'igual (' + Object.keys(r)[0] + ')'; }
  n5.push({ at: `${c.path}:${c.line}`, text: c.text.slice(0, 90), test: c.test, forma, classe: N[cls].label, mro: mro(cls).map(k => k.startsWith('EXT:') || !N[k] ? k : N[k].label), now: { res: c.res, targets: c.targets.map(show) }, after, mudanca });
}

// ---------- inherited __init__ on instantiation ----------
const inits = [];
for (const c of P.calls.values()) {
  if (c.lang !== 'python' || c.rel !== 'instantiates' || c.targets.length !== 1) continue;
  const cls = c.targets[0];
  if (info[cls]?.methods.__init__) continue;
  const r = lookup(cls, '__init__', 1);
  inits.push({ at: `${c.path}:${c.line}`, text: c.text.slice(0, 60), test: c.test, classe: show(cls), mro: mro(cls).map(k => k.startsWith('EXT:') || !N[k] ? k : N[k].label), runs: r.found ? show(r.found) : (r.external ? `fora (${r.external})` : r.none ? 'nenhum (object)' : JSON.stringify(r)) });
}

// ---------- N8: alias ----------
const aliases = [];
for (const f of new Set(Object.values(declared).map(e => e.path).filter(p => p.endsWith('.py')))) {
  for (const [bound, bs] of Object.entries(table(f))) for (const b of bs) {
    if (b.kind === 'from' && b.name !== bound && b.name !== '*') {
      const r = resolveFrom(f, b.module, b.name);
      aliases.push({ file: f, line: b.line, bound, name: b.name, module: b.module, kind: r.kind, to: r.id ? show(r.id) : r.file ?? r.module });
    }
    if (b.kind === 'import' && b.module.includes('.') && bound !== b.module.split('.')[0]) {
      const mf = I.moduleFile(SRC, f, b.module);
      aliases.push({ file: f, line: b.line, bound, name: b.module, module: b.module, kind: mf ? 'modulo' : 'external', to: mf ?? b.module });
    }
  }
}
// local / module aliases: NAME = Class (or = m.Class), and X: TypeAlias = ...
const localAliases = [];
for (const f of new Set(Object.values(declared).map(e => e.path).filter(p => p.endsWith('.py')))) {
  const lines = src(f).split('\n');
  lines.forEach((l, i) => {
    const m = l.match(/^(\s*)([A-Za-z_]\w*)\s*(:\s*TypeAlias\s*)?=\s*([A-Za-z_][\w.]*)\s*(\[.*\])?\s*$/);
    if (!m) return;
    const name = m[4].split('.').pop();
    if (!/^[A-Z]/.test(name) && !m[3]) return;
    const proj = Object.keys(topLevel).some(k => k.endsWith('#' + name) && topLevel[k].some(id => N[id].kind === 'class'));
    localAliases.push({ at: `${f}:${i + 1}`, alias: m[2], valor: m[4] + (m[5] ?? ''), indent: m[1].length, typeAlias: !!m[3], classeDoProjeto: proj });
  });
}

const out = { n0, n5, inits, aliases, localAliases, classes: Object.values(info).map(i => ({ label: i.label, file: i.file, bases: i.bases.map(b => b.kind === 'project' ? show(b.id) : `${b.kind}:${b.text}`), mro: mro(i.id).map(k => k.startsWith('EXT:') || !N[k] ? k : N[k].label) })) };
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));
const count = (xs, f) => xs.reduce((m, x) => { const k = f(x); m[k] = (m[k] ?? 0) + 1; return m; }, {});
console.log('N0 chamadas simples:', n0.length);
console.log(' por resposta de hoje:', count(n0, x => x.now.res));
console.log(' por ligação:', count(n0, x => x.binding.kind + '/' + x.binding.how));
console.log(' mudança (produção):', count(n0.filter(x => !x.test), x => x.mudanca));
console.log(' mudança (teste):', count(n0.filter(x => x.test), x => x.mudanca));
console.log('N5:', n5.length, count(n5, x => x.forma));
console.log(' mudança (produção):', count(n5.filter(x => !x.test), x => x.forma + ' ' + x.mudanca));
console.log(' mudança (teste):', count(n5.filter(x => x.test), x => x.forma + ' ' + x.mudanca));
console.log('__init__ herdado:', inits.length, count(inits, x => (x.test ? 'teste ' : 'prod ') + (x.runs.startsWith('fora') ? 'fora' : x.runs.startsWith('nenhum') ? 'nenhum' : 'projeto')));
console.log('alias de import:', aliases.length, count(aliases, x => x.kind));
console.log('alias local/tipo:', localAliases.length, count(localAliases, x => (x.classeDoProjeto ? 'classe do projeto' : 'outro') + (x.indent ? ' (indentado)' : ' (módulo)')));

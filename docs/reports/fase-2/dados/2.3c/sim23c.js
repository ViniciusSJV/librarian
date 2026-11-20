// Passo 2.3c, Parada 1 (diagnóstico; não é código de produção). Parte do dump de /api/calls do
// binário atual (2.3b) e muda só as chamadas de método cujo receptor está ligado ao resultado de uma
// chamada (x = f(..), f(..).m(), self.x = f(..)) que a regra liga hoje a uma função ou método único,
// com retorno anotado que nomeia uma classe do projeto.
//   node sim23c.js <dump-dir> <tipos-hc.jsonl> <bases.json> <out.json>
'use strict';
const fs = require('fs'), path = require('path');
const I = require('C:/Users/beatl/Documents/Projetos/librarian/docs/reports/fase-2/dados/2.2/imports.js');
const SRC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const [dir, tiposFile, basesFile, outFile] = process.argv.slice(2);
const KEY = 'healthchecks-25597e41';
const isTest = p => /(^|\/)tests?\/|test_|_test\./.test(p);
const LINKED = new Set(['unique', 'import', 'receiver', 'type', 'trait_default', 'chosen']);

const g = JSON.parse(fs.readFileSync(path.join(dir, `${KEY}.graph.json`), 'utf8'));
const d = JSON.parse(fs.readFileSync(path.join(dir, `${KEY}.calls.json`), 'utf8'));
const declared = {}, parent = {}, children = {}, observedBy = {}, rtAt = new Map();
for (const e of g.edges) {
  if (e.kind === 'declares') declared[e.to] = e.evidence;
  else if (e.kind === 'contains') { parent[e.to] = e.from; (children[e.from] ??= []).push(e.to); }
  else if (e.kind === 'observes') {
    (observedBy[e.from] ??= []).push(e.to);
    const n = g.nodes[e.to];
    if (n.kind === 'receiver_type') rtAt.set(`${e.evidence.path}@${e.evidence.start_byte}-${e.evidence.end_byte}`, { id: e.to, rule: n.type_rule, label: n.label });
  }
}
const where = id => declared[id] ? `${declared[id].path}:${declared[id].start_line}` : '?';
const show = t => `${t.label} (${t.at})`;
// id of a symbol by "label (path:line)"
const byShow = new Map();
for (const [id, n] of Object.entries(g.nodes)) if (n.category === 'symbol' && declared[id]) byShow.set(`${n.label} (${where(id)})`, id);

const A = new Map();
for (const [owner, r] of Object.entries(d.res)) {
  const by = new Map();
  for (const l of r.links) {
    const k = `${l.path}@${l.sb}-${l.eb}`;
    const a = by.get(k) ?? { key: k, owner, ownerKind: r.owner.kind, ownerLabel: r.owner.label, text: l.text, line: l.line, path: l.path, resolution: l.resolution, relation: l.relation, candidates: l.candidates, targets: [] };
    a.targets.push(l.to); by.set(k, a);
  }
  for (const u of r.unresolved) {
    const k = `${u.path}@${u.sb}-${u.eb}`;
    by.set(k, { key: k, owner, ownerKind: r.owner.kind, ownerLabel: r.owner.label, text: u.text, line: u.line, path: u.path, resolution: u.reason, candidates: u.candidates, targets: [] });
  }
  for (const [k, a] of by) if (!A.has(k)) A.set(k, a);
}

// Classes
const BASED = new Set(JSON.parse(fs.readFileSync(basesFile, 'utf8')));
const classes = {}, byFileName = {}, byName = {};
for (const [id, n] of Object.entries(g.nodes)) {
  if (n.category !== 'symbol' || n.kind !== 'class' || !declared[id]) continue;
  const file = declared[id].path, name = n.label.split('.').pop();
  const methods = {};
  for (const c of children[id] ?? []) if (g.nodes[c].kind === 'method') (methods[g.nodes[c].label.split('.').pop()] ??= []).push(c);
  const bases = (observedBy[id] ?? []).filter(o => g.nodes[o].kind === 'base_class' && g.nodes[o].label !== 'object').map(o => g.nodes[o].label);
  classes[id] = { id, file, label: n.label, name, methods, bases: bases.length ? bases : (BASED.has(name) ? ['(base escrita)'] : []), at: where(id) };
  if (!n.label.includes('.')) (byFileName[`${file}#${n.label}`] ??= []).push(id);
  (byName[name] ??= []).push(id);
}
const tables = {};
const table = f => tables[f] ??= I.table(fs.readFileSync(path.join(SRC, f), 'utf8'));
function classIn(file, name, hops = 0) {
  const local = byFileName[`${file}#${name}`];
  if (local?.length === 1) return { id: local[0] };
  const b = (table(file)[name] ?? []).find(v => !v.nested) ?? (table(file)[name] ?? [])[0];
  if (!b) return { naoachado: name };
  if (b.kind === 'from') {
    if (b.name !== name) return { alias: true };
    const mod = I.moduleFile(SRC, file, b.module);
    if (!mod) {
      const asModule = I.moduleFile(SRC, file, (/^\.+$/.test(b.module) ? b.module : b.module + '.') + b.name);
      if (asModule) return { module: true };
      return { external: b.module };
    }
    if (hops > 3) return { naoachado: name };
    return classIn(mod, b.name, hops + 1);
  }
  return { module: true };
}
function classOfAttr(file, headName, name) {
  const b = (table(file)[headName] ?? []).find(v => !v.nested) ?? (table(file)[headName] ?? [])[0];
  if (!b) return { naoachado: `${headName}.${name}` };
  const mod = b.kind === 'import' ? I.moduleFile(SRC, file, b.module) : I.moduleFile(SRC, file, (/^\.+$/.test(b.module) ? b.module : b.module + '.') + b.name);
  if (!mod) return { external: b.module };
  return classIn(mod, name, 1);
}
// The annotation a declared_return names, by the 2.3a rules (T, m.T, "T", T | None, Optional[T], Union[T, None]).
function annotation(text) {
  let t = text.trim();
  if (/^["'].*["']$/.test(t)) t = t.slice(1, -1).trim();
  let m;
  if ((m = t.match(/^(?:typing\.)?Optional\[(.+)\]$/s))) return annotation(m[1]);
  if ((m = t.match(/^(?:typing\.)?Annotated\[([^,\]]+),/s))) return annotation(m[1]);
  if ((m = t.match(/^(?:typing\.)?Union\[(.+)\]$/s))) { const p = m[1].split(',').map(s => s.trim()).filter(s => s !== 'None'); return p.length === 1 ? annotation(p[0]) : { uniao: t }; }
  if (t.includes('|')) { const p = t.split('|').map(s => s.trim()).filter(s => s !== 'None'); return p.length === 1 ? annotation(p[0]) : { uniao: t }; }
  if (t === 'None') return { none: true };
  if (t.includes('[')) return { generico: t };
  if (/^[A-Za-z_][\w.]*$/.test(t)) return { name: t };
  return { outro: t };
}

const tipos = fs.readFileSync(tiposFile, 'utf8').trim().split('\n').map(JSON.parse).filter(r => r.method);
const out = { mudancas: [], notas: {}, casos: [] };
const bump = k => out.notas[k] = (out.notas[k] ?? 0) + 1;
for (const r of tipos) {
  if (!r.via_call || !r.type) continue;
  const key = `${r.path}@${r.sb}-${r.eb}`;
  const a = A.get(key);
  if (!a) { bump('0 chamada sem dono no dump'); continue; }
  const prod = rtAt.get(`${r.path}@${r.recv[0]}-${r.recv[1]}`);
  const v = A.get(`${r.path}@${r.via_call[0]}-${r.via_call[1]}`);
  const caso = { key, at: `${r.path}:${a.line}`, teste: isTest(r.path), text: a.text, rule: r.rule, binding: v ? v.text : null, receiver_type_v9: prod ? `${prod.rule}:${prod.label}` : null };
  if (!v) { bump('1 chamada que liga não achada no dump'); continue; }
  if (!LINKED.has(v.resolution) || v.targets.length !== 1) { bump(`2 a regra não fecha a chamada que liga (${v.resolution})`); continue; }
  const f = byShow.get(show(v.targets[0]));
  const kind = v.targets[0].kind;
  if (kind === 'class') { bump('3 liga a uma classe (2.3a)'); continue; }
  if (!['function', 'method'].includes(kind)) { bump(`3 liga a ${kind}`); continue; }
  const ret = (observedBy[f] ?? []).map(o => g.nodes[o]).find(n => n.kind === 'declared_return');
  caso.f = show(v.targets[0]); caso.retorno = ret ? ret.label : null;
  if (!ret) { bump('4 f sem retorno anotado'); continue; }
  const ann = annotation(ret.label);
  if (!ann.name) { bump(`5 retorno ${Object.keys(ann)[0]}`); caso.ann = ann; out.casos.push({ ...caso, fim: 'retorno não nomeia um tipo' }); continue; }
  const file = declared[f].path;
  let K;
  if (ann.name === 'Self') K = { id: parent[f] && g.nodes[parent[f]].kind === 'class' ? parent[f] : null };
  else if (!ann.name.includes('.')) K = classIn(file, ann.name);
  else { const parts = ann.name.split('.'); K = classOfAttr(file, parts[0], parts.at(-1)); }
  if (!K.id) { const why = K.external ? 'de fora do projeto' : K.alias ? 'alias' : K.module ? 'módulo' : `não achado (${K.naoachado})`; bump(`6 retorno ${why}`); out.casos.push({ ...caso, fim: `retorno ${why}` }); continue; }
  const cls = classes[K.id];
  bump('7 retorno é classe do projeto');
  const own = cls.methods[r.name] ?? [];
  const prev = { resolution: a.resolution, candidates: a.candidates, targets: a.targets.map(show) };
  let next = null;
  if (['unique', 'import', 'receiver', 'chosen'].includes(a.resolution)) {
    if (a.resolution === 'unique' && a.targets[0].kind === 'method') {
      const fits = own.some(id => show({ label: g.nodes[id].label, at: where(id) }) === show(a.targets[0]));
      if (!fits && !cls.bases.length && !cls.methods.__getattr__ && !own.length) next = { resolution: 'no_candidate_in_project', targets: [], por: 'tipo derruba' };
    }
  } else if (cls.bases.includes('Protocol')) {
  } else if (own.length) {
    next = { resolution: 'type', targets: own.map(id => show({ label: g.nodes[id].label, at: where(id) })) };
  } else if (!cls.bases.length && !cls.methods.__getattr__ && a.resolution !== 'no_candidate_in_project') {
    next = { resolution: 'no_candidate_in_project', targets: [], por: 'tipo derruba' };
  }
  caso.tipo = `${cls.label} (${cls.at})`; caso.antes = prev; caso.depois = next ?? prev;
  out.casos.push({ ...caso, fim: next ? 'muda' : 'igual' });
  if (next && (next.resolution !== prev.resolution || JSON.stringify(next.targets) !== JSON.stringify(prev.targets))) out.mudancas.push(caso);
}
// Campos de classe de outro arquivo: o leitor acha o campo pelo nome da classe em todo o projeto;
// o V9 só no próprio arquivo. Os que o V9 não grava são o item 2.
for (const r of tipos) {
  if (!r.type || !r.rule?.startsWith('campo')) continue;
  const prod = rtAt.get(`${r.path}@${r.recv[0]}-${r.recv[1]}`);
  if (prod) continue;
  const key = `${r.path}@${r.sb}-${r.eb}`, a = A.get(key);
  out.casos.push({ key, at: `${r.path}:${a?.line}`, teste: isTest(r.path), text: a?.text, rule: r.rule, campo_outro_arquivo: true, tipo_lido: r.type, antes: a && { resolution: a.resolution, targets: a.targets.map(show) }, metodo: r.name, classes_do_nome: (byName[r.type] ?? []).map(id => `${classes[id].label} (${classes[id].at}) ${classes[id].methods[r.name] ? 'tem' : 'não tem'} ${r.name}${classes[id].bases.length ? ' base:' + classes[id].bases.join(',') : ''}`) });
  bump('8 campo de classe de outro arquivo');
}
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));
console.log(Object.fromEntries(Object.entries(out.notas).sort()));
const by = {};
for (const m of out.mudancas) { const k = `${m.antes.resolution} -> ${m.depois.resolution} (${m.teste ? 'teste' : 'prod'})`; by[k] = (by[k] ?? 0) + 1; }
console.log('mudancas', out.mudancas.length, by);

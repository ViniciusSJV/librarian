// Step 2.3, Parada 1: simulates subpasso 2.3a on top of the /api/calls dump of the current binary.
// The dump is the starting point (it is what the binary answers today); only calls whose target is
// a class/struct/variant (relation "instancia") or whose receiver gets a Python type change.
//   node sim23.js <dump-dir> <tipos-hc.jsonl> <out.json> [--alias] [--byname] [--exttype]
'use strict';
const fs = require('fs'), path = require('path');
const I = require('C:/Users/beatl/Documents/Projetos/librarian/docs/reports/fase-2/dados/2.2/imports.js');
const SRC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const [dir, tiposFile, outFile, ...flags] = process.argv.slice(2);
const opt = { alias: flags.includes('--alias'), byname: flags.includes('--byname'), exttype: flags.includes('--exttype') };
const KEYS = { hc: 'healthchecks-25597e41', rt: 'renderer-9b78f3a9' };
const isTest = p => /(^|\/)tests?\/|test_|_test\./.test(p);

function load(key) {
  const g = JSON.parse(fs.readFileSync(path.join(dir, `${key}.graph.json`), 'utf8'));
  const d = JSON.parse(fs.readFileSync(path.join(dir, `${key}.calls.json`), 'utf8'));
  const declared = {}, parent = {}, observedBy = {}, children = {};
  for (const e of g.edges) {
    if (e.kind === 'declares') declared[e.to] = e.evidence;
    else if (e.kind === 'contains') { parent[e.to] = e.from; (children[e.from] ??= []).push(e.to); }
    else if (e.kind === 'observes') (observedBy[e.from] ??= []).push(e.to);
  }
  const where = id => declared[id] ? `${declared[id].path}:${declared[id].start_line}` : '?';
  // every call answer by occurrence key path@sb-eb
  const A = new Map();
  for (const [owner, r] of Object.entries(d.res)) {
    const by = new Map();
    for (const l of r.links) {
      const k = `${l.path}@${l.sb}-${l.eb}`;
      const a = by.get(k) ?? { key: k, owner, ownerKind: r.owner.kind, ownerLabel: r.owner.label, text: l.text, line: l.line, path: l.path, resolution: l.resolution, basis: l.basis, candidates: l.candidates, targets: [] };
      a.targets.push(l.to); by.set(k, a);
    }
    for (const u of r.unresolved) {
      const k = `${u.path}@${u.sb}-${u.eb}`;
      if (u.reason === 'node_limit') { const a = by.get(k) ?? { key: k, owner, ownerKind: r.owner.kind, ownerLabel: r.owner.label, text: u.text, line: u.line, path: u.path, resolution: u.resolution, candidates: u.candidates, targets: [] }; a.targets.push(u.to); by.set(k, a); continue; }
      by.set(k, { key: k, owner, ownerKind: r.owner.kind, ownerLabel: r.owner.label, text: u.text, line: u.line, path: u.path, resolution: u.reason, candidates: u.candidates, targets: [] });
    }
    for (const [k, a] of by) A.set(k, a);
  }
  return { g, A, declared, parent, children, observedBy, where };
}

const LINKED = new Set(['unique', 'import', 'receiver', 'type', 'trait_default', 'chosen']);
const show = t => `${t.label} (${t.at})`;

function simulate() {
  const hc = load(KEYS.hc), rt = load(KEYS.rt);
  const out = { mudancas: [], instancia: { hc: [], rt: [] }, sem_classe: [], notas: {} };
  // --- instancia: a single linked target that is a class (Python) or a struct/variant (Rust).
  for (const [lang, P, kinds] of [['hc', hc, ['class']], ['rt', rt, ['struct', 'variant']]]) {
    for (const a of P.A.values()) {
      if (!LINKED.has(a.resolution) || a.targets.length !== 1) continue;
      if (kinds.includes(a.targets[0].kind)) out.instancia[lang].push({ key: a.key, text: a.text, owner: a.ownerLabel, resolution: a.resolution, alvo: show(a.targets[0]), teste: isTest(a.path) });
    }
  }
  // --- Python classes: id by (file, qualified name), methods, bases.
  const BASED = new Set(JSON.parse(fs.readFileSync(path.join(path.dirname(tiposFile), 'bases.json'), 'utf8')));
  const G = hc.g, classes = {}, byFileName = {}, byName = {};
  for (const [id, n] of Object.entries(G.nodes)) {
    if (n.category !== 'symbol' || n.kind !== 'class' || !hc.declared[id]) continue;
    const file = hc.declared[id].path;
    const methods = {};
    for (const c of hc.children[id] ?? []) if (G.nodes[c].kind === 'method') (methods[G.nodes[c].label.split('.').pop()] ??= []).push(c);
    const bases = (hc.observedBy[id] ?? []).filter(o => G.nodes[o].kind === 'base_class' && G.nodes[o].label !== 'object');
    // only the class's own bases (an occurrence observed by the class itself)
    classes[id] = { id, file, label: n.label, name: n.label.split('.').pop(), methods, bases: bases.length ? bases.map(o => G.nodes[o].label) : (BASED.has(n.label.split('.').pop()) ? ['(base escrita)'] : []), top: !n.label.includes('.'), at: hc.where(id) };
    if (classes[id].top) (byFileName[`${file}#${n.label}`] ??= []).push(id);
    (byName[classes[id].name] ??= []).push(id);
  }
  const tables = {};
  const table = f => tables[f] ??= I.table(fs.readFileSync(path.join(SRC, f), 'utf8'));
  // The class a written name means in a file: declared there, or imported from a project module.
  function classIn(file, name, hops = 0) {
    const local = byFileName[`${file}#${name}`];
    if (local?.length === 1) return { id: local[0], how: 'arquivo' };
    const b = (table(file)[name] ?? []).find(v => !v.nested) ?? (table(file)[name] ?? [])[0];
    if (!b) return null;
    if (b.kind === 'from') {
      if (!opt.alias && b.name !== name) return { alias: true };
      const mod = I.moduleFile(SRC, file, b.module);
      if (!mod) {
        const asModule = I.moduleFile(SRC, file, (/^\.+$/.test(b.module) ? b.module : b.module + '.') + b.name);
        if (asModule) return { module: true };
        // a project package whose module was not found, or a module from outside the project
        return { external: b.module };
      }
      if (hops > 3) return null;
      const r = classIn(mod, b.name, hops + 1);
      return r && r.id ? { ...r, how: 'import' } : r;
    }
    return { module: true };
  }
  function classOfAttr(file, headName, name) {
    const b = (table(file)[headName] ?? []).find(v => !v.nested) ?? (table(file)[headName] ?? [])[0];
    if (!b) return null;
    let mod = null;
    if (b.kind === 'import') mod = I.moduleFile(SRC, file, b.dotted && b.dotted !== b.module ? b.module : b.module);
    else mod = I.moduleFile(SRC, file, (/^\.+$/.test(b.module) ? b.module : b.module + '.') + b.name);
    if (!mod) return { external: b.module };
    const r = classIn(mod, name, 1);
    return r && r.id ? { ...r, how: 'import' } : r;
  }
  // innermost class of a file containing an offset (for `self`)
  function enclosing(file, sb, name) {
    const c = Object.values(classes).filter(c => c.file === file && c.name === name && hc.declared[c.id].start_byte <= sb && hc.declared[c.id].end_byte >= sb)
      .sort((a, b) => (hc.declared[a.id].end_byte - hc.declared[a.id].start_byte) - (hc.declared[b.id].end_byte - hc.declared[b.id].start_byte));
    return c[0] ? { id: c[0].id, how: 'self' } : null;
  }
  const tipos = fs.readFileSync(tiposFile, 'utf8').trim().split('\n').map(JSON.parse).filter(r => r.method && r.type);
  const notes = out.notas;
  const bump = k => notes[k] = (notes[k] ?? 0) + 1;
  for (const r of tipos) {
    const key = `${r.path}@${r.sb}-${r.eb}`;
    const a = hc.A.get(key);
    if (!a) { bump('chamada sem dono no grafo'); continue; }
    if (r.rule.startsWith('campo') && r.head !== 'self') { bump('campo de classe de outro arquivo (2.3c)'); continue; }
    // which class the written type means
    let K = null;
    if (r.rule === 'self') K = enclosing(r.path, r.sb, r.type);
    else if (r.rule.startsWith('campo')) { const ids = byName[r.type] ?? []; K = ids.length === 1 ? { id: ids[0], how: 'nome' } : ids.length > 1 ? { homonimas: ids.length } : null; }
    else if (opt.byname) { const ids = byName[r.type] ?? []; K = ids.length === 1 ? { id: ids[0], how: 'nome' } : ids.length > 1 ? { homonimas: ids.length } : null; }
    else if (r.type === r.type_head) K = classIn(r.path, r.type);
    else K = classOfAttr(r.path, r.type_head, r.type);
    a.tipo = { rule: r.rule, type: r.type, K };
    if (!K) { bump(`tipo sem classe (${r.rule})`); continue; }
    if (K.external) { bump(`tipo de fora do projeto (${r.rule})`); if (!opt.exttype) continue; }
    if (K.module || K.alias || K.homonimas) { bump(`tipo ${K.module ? 'é módulo' : K.alias ? 'alias (2.4)' : 'homônimo'} (${r.rule})`); continue; }
    if (!K.id) continue;
    const cls = classes[K.id];
    bump(`tipo do projeto (${r.rule})`);
    // construtor cross-check: what the binding call resolves to today
    if (r.via_call) {
      const v = hc.A.get(`${r.path}@${r.via_call[0]}-${r.via_call[1]}`);
      const ok = v && LINKED.has(v.resolution) && v.targets.length === 1 && v.targets[0].kind === 'class' && show(v.targets[0]) === `${cls.label} (${cls.at})`;
      bump(ok ? 'construtor: a chamada liga à mesma classe hoje' : `construtor: a chamada hoje ${v ? v.resolution : 'não achada'}`);
    }
    const name = (a.text.match(/\.([\p{L}\p{N}_]+)\s*\(/u) ? [...a.text.matchAll(/\.([\p{L}\p{N}_]+)\s*\(/gu)].pop()[1] : null);
    // name of the called method: the last `.name(` at the top level of the head. The tool's own text is enough.
    const m = r.text.match(/^(.*)\.([\p{L}\p{N}_]+)\s*\(/su);
    const method = r.name;
    const own = (cls.methods[method] ?? []);
    const prev = { resolution: a.resolution, candidates: a.candidates, targets: a.targets.map(show) };
    let next = null;
    if (['unique', 'import', 'receiver', 'chosen'].includes(a.resolution)) {
      // label kept when the type agrees; a unique ruled out when the type has no base and no such method
      if (a.resolution === 'unique' && a.targets[0].kind === 'method') {
        const fits = own.some(id => show({ label: G.nodes[id].label, at: hc.where(id) }) === show(a.targets[0]));
        if (!fits && !cls.bases.length && !own.length) next = { resolution: 'no_candidate_in_project', candidates: 0, targets: [], por: 'tipo derruba' };
        else if (!fits) bump('unique de outra classe, tipo com base: fica');
      }
    } else if (a.resolution === 'above_candidate_limit' && r.text.replace(/ /g, '').startsWith('self.' + r.name + '(') && a.ownerKind === 'method' && a.ownerLabel.startsWith(cls.label + '.') && !a.ownerLabel.slice(cls.label.length + 1).includes('.') && own.length === 1) {
      next = { resolution: 'receiver', candidates: a.candidates, targets: own.map(id => show({ label: G.nodes[id].label, at: hc.where(id) })), por: 'receiver acima do teto (chave do nome corrigida)' };
    } else if (own.length) {
      const sub = Object.values(classes).filter(c => c.bases.includes(cls.name) && c.methods[method]).map(c => c.label);
      next = { resolution: 'type', candidates: a.candidates, targets: own.map(id => show({ label: G.nodes[id].label, at: hc.where(id) })), por: 'tipo', sobrescrito_em: sub.length ? sub : undefined };
    } else if (!cls.bases.length && a.resolution !== 'no_candidate_in_project') {
      next = { resolution: 'no_candidate_in_project', candidates: 0, targets: [], por: 'tipo derruba' };
    }
    if (next && JSON.stringify(next.targets) !== JSON.stringify(prev.targets) || next && next.resolution !== prev.resolution) {
      out.mudancas.push({ key, path: r.path, line: a.line, text: a.text, owner: a.ownerLabel, ownerKind: a.ownerKind, teste: isTest(r.path) || /test/i.test(a.ownerLabel), rule: r.rule, tipo: `${cls.label} (${cls.at})`, base: cls.bases.join(',') || null, antes: prev, depois: next });
    }
  }
  return out;
}

const out = simulate();
fs.writeFileSync(outFile, JSON.stringify(out, null, 1));
console.log('notas', out.notas);
console.log('instancia hc', out.instancia.hc.length, 'teste', out.instancia.hc.filter(x => x.teste).length, '| rt', out.instancia.rt.length, 'teste', out.instancia.rt.filter(x => x.teste).length);
const by = {};
for (const m of out.mudancas) { const k = `${m.antes.resolution} -> ${m.depois.resolution} (${m.teste ? 'teste' : 'prod'})`; by[k] = (by[k] ?? 0) + 1; }
console.log('mudancas', out.mudancas.length, by);

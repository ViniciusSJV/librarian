// Simulates resolve.rs over a graph (current rule, then the proposed 2.2 rules) and compares with
// the /api/calls dump. node sim.js <dump-dir> [options as JSON]
'use strict';
const fs = require('fs'), path = require('path');
const I = require('./imports.js');
const SRC = { 'healthchecks-25597e41': 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks', 'renderer-9b78f3a9': 'C:/Users/beatl/Documents/Projetos/renderer' };
const RECV = { python: 'self', rust: 'self' };

const lastSegment = t => { t = t.split('<')[0]; const l = t.split('::').pop(); return l.split(/[\s&]/).pop(); };
const implParts = label => { if (!label.startsWith('impl ')) return [null, null]; const rest = label.slice(5); const i = rest.indexOf(' for '); return i >= 0 ? [lastSegment(rest.slice(i + 5)), lastSegment(rest.slice(0, i))] : [lastSegment(rest), null]; };
function matchingOpen(t, o, c) { let d = 0; for (let i = t.length - 1; i >= 0; i--) { if (t[i] === c) d++; else if (t[i] === o) { d--; if (d < 0) return null; if (d === 0) return i; } } return null; }
function callHead(text) { let h = text.trimEnd(); if (h.endsWith(')')) { const i = matchingOpen(h, '(', ')'); if (i === null) return null; h = h.slice(0, i); } if (h.endsWith('>')) { const i = matchingOpen(h, '<', '>'); if (i === null) return null; h = h.slice(0, i).replace(/(::)+$/, ''); } return h; }
const trailingIdent = t => { const m = t.match(/[\p{L}\p{N}_]+$/u); return m ? m[0] : null; };
function callParts(text) { const h = callHead(text); if (h === null) return null; const n = trailingIdent(h); if (!n) return null; const before = h.slice(0, h.length - n.length).trimEnd(); let r = ''; if (before.endsWith('.') || before.endsWith('::')) { const rest = before.replace(/(\.|::)$/, '').trimEnd(); r = trailingIdent(rest) ?? '()'; } return [r, n]; }
function writtenReceiver(text) { const h = callHead(text); if (h === null) return null; const n = trailingIdent(h); if (!n) return null; const b = h.slice(0, h.length - n.length).trimEnd(); if (!b.endsWith('.')) return null; const r = b.slice(0, -1).trimEnd(); return r || null; }

function build(key, graph) {
  const R = { graph, cand: {}, parent: {}, mbn: {}, rts: {}, ev: {}, implTraits: [], declared: {} };
  for (const e of graph.edges) {
    if (e.kind === 'name_candidate') (R.cand[e.from] ??= []).push(e);
    else if (e.kind === 'contains') R.parent[e.to] = e.from;
    else if (e.kind === 'declares') R.declared[e.to] = e.evidence;
    else if (e.kind === 'observes') { R.ev[e.to] = e.evidence; if (graph.nodes[e.to]?.kind === 'receiver_type') (R.rts[`${e.evidence.path}@${e.evidence.start_byte}`] ??= []).push([e.evidence.end_byte, e.to]); }
  }
  for (const [id, n] of Object.entries(graph.nodes)) {
    if (n.category !== 'symbol') continue;
    if (n.kind === 'method' || n.kind === 'trait_method') (R.mbn[n.label.split('::').pop()] ??= []).push(id);
    if (n.kind === 'impl') { const [t, tr] = implParts(n.label); if (t && tr) R.implTraits.push([t, tr]); }
  }
  return R;
}
const kindOf = (R, id) => R.graph.nodes[id].kind, labelOf = (R, id) => R.graph.nodes[id].label;
function ownerOf(R, m) { const o = R.parent[m]; if (!o) return null; if (kindOf(R, o) === 'impl') return implParts(labelOf(R, o)); if (kindOf(R, o) === 'trait') return [null, lastSegment(labelOf(R, o))]; return null; }
function receiverTypes(R, caller, occ) {
  const text = labelOf(R, occ);
  if (kindOf(R, occ) === 'call_observed') { const p = callParts(text); if (!p) return []; if (p[0] === 'Self') { const c = R.parent[caller]; return c && kindOf(R, c) === 'impl' && implParts(labelOf(R, c))[0] ? [implParts(labelOf(R, c))[0]] : []; } return /^\p{Lu}/u.test(p[0]) ? [p[0]] : []; }
  const ev = R.ev[occ]; if (!ev) return []; const wr = writtenReceiver(text); if (!wr) return [];
  const end = ev.start_byte + Buffer.byteLength(wr);
  const f = (R.rts[`${ev.path}@${ev.start_byte}`] ?? []).find(([e]) => e === end); if (!f) return [];
  return [...new Set((R.cand[f[1]] ?? []).map(t => lastSegment(labelOf(R, t.to))))].sort();
}
/** the written type label of the receiver (even with no project declaration). */
function writtenType(R, occ) {
  const ev = R.ev[occ]; const wr = writtenReceiver(labelOf(R, occ)); if (!ev || !wr) return null;
  const end = ev.start_byte + Buffer.byteLength(wr);
  const f = (R.rts[`${ev.path}@${ev.start_byte}`] ?? []).find(([e]) => e === end); return f ? labelOf(R, f[1]) : null;
}

/** opts: {kind: bool (method call -> no function), mirror: bool (bare call -> no method), promote: bool, typeDrop: bool, typeDropHomonym: bool, receiverAbove: bool, imports: fn(caller, occ)->{module:file|null}|null } */
function resolve(R, caller, occ, opts, lang) {
  const all = R.cand[occ] ?? [];
  let edges = all;
  const okind = kindOf(R, occ), text = labelOf(R, occ);
  const info = {};
  if (opts.kind && okind === 'method_call_observed') {
    const mod = lang === 'python' && opts.imports ? opts.imports(occ) : null;
    info.mod = mod;
    if (mod && opts.extImport === false && mod.external) edges = edges.filter(e => kindOf(R, e.to) !== 'function'); else if (mod) edges = edges.filter(e => kindOf(R, e.to) === 'function' && mod.file && R.declared[e.to]?.path === mod.file);
    else edges = edges.filter(e => kindOf(R, e.to) !== 'function');
  }
  if (opts.mirror && okind === 'call_observed') { const p = callParts(text); if (p && p[0] === '') edges = edges.filter(e => !['method', 'trait_method'].includes(kindOf(R, e.to))); }
  const filtered = edges.length !== all.length;
  const types = receiverTypes(R, caller, occ);
  if (opts.extTypeInherent && !types.length && edges.length === 1 && writtenType(R, occ) !== null && kindOf(R, edges[0].to) === 'method') { const o = R.parent[edges[0].to]; if (o && kindOf(R, o) === 'impl' && implParts(labelOf(R, o))[1] === null) return { r: 'no_candidate_in_project', targets: [], dropped: 'tipo-externo', filtered: true }; }
  if (opts.typeOnlyEmpty && types.length && edges.length && edges.every(e => ['method','trait_method'].includes(kindOf(R, e.to)) && !compatible(R, e.to, types))) return { r: 'no_candidate_in_project', targets: [], dropped: 'type', filtered: true };
  const bare = okind === 'call_observed' && (callParts(text) ?? [])[0] === '';
  if (edges.length === 1 && filtered && opts.importLabel && info.mod && info.mod.file) return { r: 'import', targets: [edges[0].to], filtered, mod: info.mod };
  if (edges.length === 1 && (!filtered || opts.promote || (opts.promoteBare && bare))) {
    const t = edges[0].to;
    if (opts.typeDrop && types.length && !compatible(R, t, types)) return { r: 'no_candidate_in_project', targets: [], dropped: 'type', filtered };
    return { r: 'unique', targets: [t], filtered, mod: info.mod };
  }
  const recvLang = RECV[lang];
  const own = recvLang && writtenReceiver(text) === recvLang;
  const same = edges.map(e => e.to).filter(t => R.parent[t] === R.parent[caller]);
  if (own && same.length === 1 && kindOf(R, same[0]) !== 'trait_method') return { r: 'receiver', targets: same, filtered };
  if (opts.receiverAbove && own && !all.length && R.graph.nodes[occ].ambiguous_candidates) {
    const name = callParts(text)?.[1];
    const inC = (R.mbn[name] ?? []).filter(m => R.parent[m] === R.parent[caller] && kindOf(R, m) !== 'trait_method');
    if (inC.length === 1) return { r: 'receiver', targets: inC, filtered, above: true };
  }
  const name = callParts(text)?.[1] ?? '';
  const methods = R.mbn[name] ?? [];
  if (types.length) {
    const ownT = methods.filter(m => { const o = ownerOf(R, m); return o && o[0] && types.includes(o[0]); });
    if (ownT.length) return { r: 'type', targets: ownT, filtered };
    const traits = new Set(R.implTraits.filter(([t]) => types.includes(t)).map(([, tr]) => tr));
    const defs = methods.filter(m => kindOf(R, m) === 'trait_method' && (o => o && o[1] && traits.has(o[1]))(ownerOf(R, m)));
    if (defs.length) return { r: 'trait_default', targets: defs, filtered };
    if (opts.typeDropHomonym && edges.length > 1) return { r: 'no_candidate_in_project', targets: [], dropped: 'type-homonym', filtered };
  }
  const trs = new Set(edges.map(e => { const o = ownerOf(R, e.to); return o?.[1] ?? null; }));
  if (edges.length > 1 && trs.size === 1 && [...trs][0] != null) return { r: 'trait', targets: edges.map(e => e.to), filtered };
  if (!edges.length) {
    if (!all.length) return R.graph.nodes[occ].ambiguous_candidates != null ? { r: 'above_candidate_limit', targets: [], filtered } : { r: 'no_candidate_in_project', targets: [], filtered };
    if (opts.promote === false && filtered) return { r: 'no_candidate_in_project', targets: [], filtered };
    return { r: 'no_candidate_in_project', targets: [], filtered };
  }
  if (edges.length === 1) return { r: 'homonym', targets: [edges[0].to], filtered, single: true };
  return { r: 'homonym', targets: edges.map(e => e.to), filtered };
}
function compatible(R, t, types) {
  if (kindOf(R, t) === 'function') return true;
  const o = ownerOf(R, t); if (!o) return true;
  if (o[0] && types.includes(o[0])) return true;
  if (kindOf(R, t) === 'trait_method' && o[1]) { if (types.includes(o[1])) return true; return R.implTraits.some(([ty, tr]) => types.includes(ty) && tr === o[1]); }
  return false;
}

function importsFor(key, R) {
  if (!key.startsWith('health')) return null;
  const tables = {};
  return occ => {
    const ev = R.ev[occ]; const wr = writtenReceiver(labelOf(R, occ)); if (!wr) return null;
    const head = wr.match(/^[A-Za-z_]\w*/)?.[0]; if (!head) return null;
    const t = tables[ev.path] ??= I.table(fs.readFileSync(path.join(SRC[key], ev.path), 'utf8'));
    const b = t[head]; if (!b) return null;
    // module-level binding first
    const x = b.find(v => !v.nested) ?? b[0];
    let file = null;
    if (x.kind === 'import') {
      if (x.dotted && x.dotted !== x.module) { if (wr === x.dotted || !wr.includes('.')) file = I.moduleFile(SRC[key], ev.path, wr === x.dotted ? x.dotted : x.module); else return { file: null, attr: true, x }; }
      else { file = I.moduleFile(SRC[key], ev.path, x.module); if (wr !== head) return file ? null : { file: null, external: true, attr: true, x }; }
      return { file, x };
    }
    file = I.moduleFile(SRC[key], ev.path, (/^\.+$/.test(x.module) ? x.module : x.module + '.') + x.name);
    if (!file) { if (I.moduleFile(SRC[key], ev.path, x.module) || /^\.+$/.test(x.module)) return null; return { file: null, external: true, x }; } // from project import Name: not a module
    if (wr !== head) return null;
    return { file, x };
  };
}

module.exports = { build, resolve, receiverTypes, writtenType, importsFor, callParts, writtenReceiver, kindOf, labelOf, SRC };

// Caça às lacunas (step 2.4e, DEC-48): matches the gabaritos (execution and types) with what the Librarian answers
// for every call occurrence (librarian.js), and writes the tables of the Parada 2 (Parada 1, Q7 to Q12).
// The gabarito is only a ruler: nothing here enters the graph, the gate or the acervo (DEC-48, item 6), and a
// disagreement is a candidate, never a verdict (DEC-22).
//
//   node tools/precisao/gabarito/medir.js --saida <dir>
//     [--hc <Healthchecks>] [--rt <ray tracer>]
//   Reads from <dir>: librarian-hc.json, librarian-rt.json, execucao-hc.jsonl, tipos-hc.jsonl, execucao-rt.jsonl,
//   renderer.scip. Writes medicao.json and tabelas.md.
'use strict';
const fs = require('fs'), path = require('path');
const { parse: parseScip } = require('./scip');
const L = require('../lib');

const opt = L.parseArgs(process.argv.slice(2));
const OUT = path.resolve(opt.saida);
// The project folders: --hc/--rt, else GABARITO_HC/GABARITO_RT, else the folders next to the repository (README.md).
const SRC = {
  hc: path.resolve(opt.hc ?? process.env.GABARITO_HC ?? path.join(L.REPO, '..', 'pilotos', 'healthchecks')),
  rt: path.resolve(opt.rt ?? process.env.GABARITO_RT ?? path.join(L.REPO, '..', 'renderer')),
};
const LINKED = new Set(['unique', 'receiver', 'type', 'trait_default', 'import', 'scope', 'chosen']); // portao.js
const DISCORD = new Set(['link_falso_candidato', 'discorda_polimorfica', 'ligacao_que_falta', 'possivel_sem_o_certo']);
const UNSEEN = new Set(['nao_viu', 'nao_viu_duble', 'casou_ambiguo']);
const FN_KINDS = new Set(['function', 'method', 'trait_method']);
const read = f => fs.readFileSync(path.join(OUT, f), 'utf8');
const jsonl = f => read(f).split('\n').filter(Boolean).map(l => JSON.parse(l));
const lastName = s => (String(s).match(/[\p{L}\p{N}_]+$/u) ?? [null])[0];
const isAbs = p => /^[A-Za-z]:[\\/]|^\//.test(p ?? '');
const PY_TEST = p => /(^|\/)tests?\/|(^|\/)test_[^/]*\.py$|_test\.py$/.test(p);

// ---------- workflows and paths (Parada 1, Q1 and Q2, approved with the e-mail ping in B) ----------
const WORKFLOWS = {
  hc: {
    A: { nome: 'Cadastro de Check (API)', entradas: [['hc/api/views.py', 481, 'checks'], ['hc/api/views.py', 456, 'create_check']],
      marcos: [['hc/api/decorators.py', 35, 'wrapper'], ['hc/api/views.py', 332, '_update'], ['hc/api/models.py', 339, 'going_down_after'], ['hc/api/models.py', 300, 'get_grace_start']] },
    B: { nome: 'Recebimento de ping', entradas: [['hc/api/views.py', 198, 'ping'], ['hc/api/views.py', 265, 'ping_by_slug'], ['hc/api/management/commands/smtpd.py', 49, '_process_message']],
      marcos: [['hc/api/models.py', 493, 'ping'], ['hc/api/models.py', 652, 'create_flip']] },
    C: { nome: 'Detecção de atraso', entradas: [['hc/api/management/commands/sendalerts.py', 184, 'handle'], ['hc/api/management/commands/sendalerts.py', 123, 'handle_going_down']],
      marcos: [['hc/api/models.py', 355, 'get_status'], ['hc/api/models.py', 339, 'going_down_after']] },
    D: { nome: 'Flip e notificações', entradas: [['hc/api/management/commands/sendalerts.py', 91, 'process_one_flip'], ['hc/api/management/commands/sendalerts.py', 24, 'notify']],
      marcos: [['hc/api/models.py', 1391, 'select_channels'], ['hc/api/models.py', 1152, 'notify'], ['hc/api/models.py', 1139, 'transport']] },
  },
  rt: {
    'CR-1': { nome: 'Render de uma cena', entradas: [['src/camera.rs', 95, 'render']],
      marcos: [['src/world.rs', 109, 'color_at'], ['src/world.rs', 28, 'intersect_world'], ['src/world.rs', 45, 'shade_hit'], ['src/world.rs', 92, 'is_shadowed'], ['src/world.rs', 65, 'reflected_color'], ['src/world.rs', 74, 'refracted_color']] },
    'CR-2': { nome: 'Malha OBJ com BVH', entradas: [['src/obj.rs', 165, 'load_bvh']], marcos: [['src/obj.rs', 13, 'parse_obj'], ['src/obj.rs', 123, 'build_bvh']] },
    'CR-3': { nome: 'Gravação da imagem', entradas: [['src/canvas.rs', 111, 'to_png'], ['src/canvas.rs', 84, 'to_ppm']],
      marcos: [['src/canvas.rs', 41, 'create_ppm_pixel_data'], ['src/canvas.rs', 94, 'to_rgba32']] },
  },
};

// ---------- the Librarian side ----------
function loadLib(lang) {
  const D = JSON.parse(read(`librarian-${lang}.json`));
  const byPath = new Map();
  for (const d of D.declaracoes) (byPath.get(d.path) ?? byPath.set(d.path, []).get(d.path)).push(d);
  const declById = new Map(D.declaracoes.map(d => [d.id, d]));
  const occById = new Map(D.ocorrencias.map(o => [o.id, o]));
  const byStart = new Map();
  for (const o of D.ocorrencias) { const k = `${o.path}@${o.start_byte}`; (byStart.get(k) ?? byStart.set(k, []).get(k)).push(o); }
  const byOwner = new Map();
  for (const o of D.ocorrencias) (byOwner.get(o.dono) ?? byOwner.set(o.dono, []).get(o.dono)).push(o);
  // Children by parent, for a class's __init__ (an instantiation runs it; DEC-46).
  const children = new Map();
  for (const d of D.declaracoes) if (d.parent) (children.get(d.parent) ?? children.set(d.parent, []).get(d.parent)).push(d);
  const byte = (p, line, col) => { const t = D.linhas[p]; return t && line >= 1 && line <= t.length ? t[line - 1] + col : null; };
  /** The innermost declaration of `path` holding `line` whose name is `name` (any name if null), of the given kinds. */
  const declAt = (p, line, name, kinds = null) => {
    let best = null;
    for (const d of byPath.get(p) ?? []) {
      if (line < d.start_line || line > d.end_line) continue;
      if (name && lastName(d.label) !== name) continue;
      if (kinds && !kinds.has(d.kind)) continue;
      if (!best || d.end_byte - d.start_byte < best.end_byte - best.start_byte) best = d;
    }
    return best;
  };
  const ownerAtByte = (p, b) => {
    let best = null;
    for (const d of byPath.get(p) ?? []) {
      if (!FN_KINDS.has(d.kind) || b < d.start_byte || b >= d.end_byte) continue;
      if (!best || d.end_byte - d.start_byte < best.end_byte - best.start_byte) best = d;
    }
    return best;
  };
  const initOf = id => (children.get(id) ?? []).find(c => lastName(c.label) === '__init__' && FN_KINDS.has(c.kind)) ?? null;
  /** The occurrence a gabarito's call span [s, e) is: same start; a method call ends where it ends; a simple call's
   *  span is the name only (Q7). */
  const matchSpan = (p, s, e) => {
    const xs = byStart.get(`${p}@${s}`) ?? [];
    return xs.find(o => o.kind === 'method_call_observed' && o.end_byte === e)
      ?? xs.find(o => o.kind === 'call_observed' && o.end_byte <= e)
      ?? xs.find(o => o.end_byte === e) ?? null;
  };
  return { D, byPath, declById, occById, byStart, byOwner, byte, declAt, ownerAtByte, initOf, matchSpan };
}

// ---------- gabarito targets (one string each): decl:<id> | ext:<name> | duble | semdecl:<where> ----------
function pyElement(LB, el) {
  if (el.duble) return 'duble';
  if (el.classe) {
    if (el.arquivo && !isAbs(el.arquivo)) {
      const name = lastName(el.classe);
      const d = (LB.byPath.get(el.arquivo) ?? []).filter(x => x.kind === 'class' && lastName(x.label) === name)
        .sort((a, b) => (a.end_byte - a.start_byte) - (b.end_byte - b.start_byte))[0];
      return d ? `decl:${d.id}` : `semdecl:${el.arquivo}:${el.classe}`;
    }
    return `ext:${el.classe}`;
  }
  if (el.externo !== undefined) return `ext:${el.externo}.${el.nome}`;
  if (el.arquivo === '<string>') return 'duble';
  if (el.arquivo && el.arquivo.startsWith('<')) return `ext:${el.arquivo}.${el.nome}`;
  if (el.arquivo && !isAbs(el.arquivo)) {
    const d = LB.declAt(el.arquivo, el.linha, lastName(el.nome));
    // No declaration in the edition: a lambda or other nameless code of the project.
    return d ? `decl:${d.id}` : `anon:${el.arquivo}:${el.linha}:${el.nome}`;
  }
  return `ext:${el.nome}`;
}
/** The execution's target: the innermost project function of the `__wrapped__` chain (the decorated one), else the
 *  called object. A project function of a test file called from production is a test double (Q3). */
function pyExecTarget(LB, chain, callerPath) {
  for (let i = chain.length - 1; i >= 0; i--) {
    const t = pyElement(LB, chain[i]);
    if (t.startsWith('decl:') || t.startsWith('anon:')) {
      const d = LB.declById.get(t.slice(5));
      if (d && PY_TEST(d.path) && !PY_TEST(callerPath)) return 'duble';
      return t;
    }
  }
  return pyElement(LB, chain[0]);
}

// ---------- classification of one occurrence against one gabarito (Q7, with the Parada 1 answers) ----------
function classify(LB, o, g) {
  if (!g || g.alvos.size === 0) return g?.duble ? 'nao_viu_duble' : 'nao_viu';
  if (g.ambiguo) return 'casou_ambiguo';
  for (const t of [...g.alvos]) {
    const d = t.startsWith('decl:') ? LB.declById.get(t.slice(5)) : null;
    if (d?.parent && g.alvos.has(`decl:${d.parent}`)) { g.alvos.delete(t); g.implicita = (g.implicita ?? 0) + 1; }
  }
  const r = o.resposta ?? { resolution: 'sem_resposta', alvos: [] };
  const T = [...g.alvos], proj = T.filter(t => t.startsWith('decl:')).map(t => t.slice(5));
  const lib = r.alvos ?? [];
  const base = g.base ?? null; // mypy or rust-analyzer gave a base (or trait) declaration with these overrides
  const byBase = base && lib.length > 0 && lib.some(x => x !== base.id) && lib.every(x => x === base.id || base.over.has(x));
  if (LINKED.has(r.resolution) && lib.length === 1) {
    const X = lib[0];
    if (T.length === 1 && T[0] === `decl:${X}`) return 'concorda';
    if (byBase) return 'concorda_pela_base';
    if (proj.includes(X)) return 'discorda_polimorfica';
    return 'link_falso_candidato';
  }
  if (lib.length > 0) {
    if (byBase) return 'concorda_pela_base';
    const inLib = proj.filter(x => lib.includes(x));
    if (inLib.length === proj.length && proj.length === T.length) return 'possivel_com_o_certo';
    if (inLib.length > 0) return 'possivel_em_parte';
    return 'possivel_sem_o_certo';
  }
  return proj.length === 0 ? 'concorda_externo' : 'ligacao_que_falta';
}

// ---------- provisional group (Q9; the classification that counts is the 2.4f's) ----------
function grupo(o) {
  const r = o.resposta ?? {};
  if (!LINKED.has(r.resolution)) return null;
  if (r.resolution === 'type' || r.resolution === 'trait_default' || (r.resolution === 'receiver' && r.type_rule)) return 'provavel_pelo_tipo';
  if (r.resolution === 'unique' && o.kind === 'method_call_observed') return 'provavel_pelo_nome';
  return 'outras';
}

// ---------- the Python gabaritos ----------
function pyGabaritos(LB) {
  const exec = new Map(), tipos = new Map(), soGab = { execucao: { total: 0, projeto: 0, exemplos: [] }, tipos: { total: 0, projeto: 0, exemplos: [] } };
  const edges = [];
  let foraEdition = 0, threads = 0;
  const add = (m, o) => m.get(o.id) ?? m.set(o.id, { alvos: new Set(), duble: false }).get(o.id);
  for (const r of jsonl('execucao-hc.jsonl')) {
    if (r.tipo !== 'chamada' || !r.pos) continue;
    if (r.outra_thread) threads++;
    const s = LB.byte(r.chamador, r.pos[0], r.pos[2]), e = LB.byte(r.chamador, r.pos[1], r.pos[3]);
    if (s === null) { foraEdition++; continue; }
    const t = pyExecTarget(LB, r.alvo, r.chamador);
    const caller = LB.ownerAtByte(r.chamador, s);
    if (caller && t.startsWith('decl:')) {
      const d = LB.declById.get(t.slice(5));
      edges.push([caller.id, d.id]);
      if (d.kind === 'class') { const i = LB.initOf(d.id); if (i) edges.push([caller.id, i.id]); }
      // Every project function of the decorator chain ran too (the wrappers): edges for the reach, not for the match.
      for (const el of r.alvo) {
        const te = pyElement(LB, el);
        if (te.startsWith('decl:') && te !== t) edges.push([caller.id, te.slice(5)]);
      }
    }
    const o = LB.matchSpan(r.chamador, s, e);
    if (!o) { soGab.execucao.total++; if (t.startsWith('decl:')) { soGab.execucao.projeto++; if (soGab.execucao.exemplos.length < 15) soGab.execucao.exemplos.push(`${r.chamador}:${r.pos[0]} → ${LB.declById.get(t.slice(5))?.label}`); } continue; }
    const g = add(exec, o);
    if (t === 'duble') g.duble = true; else g.alvos.add(t);
  }
  for (const r of jsonl('tipos-hc.jsonl')) {
    const [l1, c1, l2, c2] = r.chamada;
    const s = LB.byte(r.arquivo, l1, c1), e = LB.byte(r.arquivo, l2, c2);
    if (s === null) { foraEdition++; continue; }
    const o = LB.matchSpan(r.arquivo, s, e);
    // A mypy Var is a variable (a callable held in a name or field), not a declaration: no target (Parada 2).
    const ts = r.alvos.filter(a => a.tipo !== 'Var').map(a => {
      if (!a.arquivo || isAbs(a.arquivo)) return { t: `ext:${a.fullname}`, a };
      const d = a.tipo === 'TypeInfo'
        ? (LB.byPath.get(a.arquivo) ?? []).filter(x => x.kind === 'class' && lastName(x.label) === a.nome && x.start_line <= a.linha && a.linha <= x.end_line).sort((x, y) => (x.end_byte - x.start_byte) - (y.end_byte - y.start_byte))[0]
        : LB.declAt(a.arquivo, a.linha_def ?? a.linha, a.nome) ?? LB.declAt(a.arquivo, a.linha, a.nome);
      return { t: d ? `decl:${d.id}` : `gerado:${a.fullname}`, a, d };
    });
    if (!o) { soGab.tipos.total++; if (ts.some(x => x.d)) { soGab.tipos.projeto++; if (soGab.tipos.exemplos.length < 15) soGab.tipos.exemplos.push(`${r.arquivo}:${l1} ${r.texto} → ${ts.find(x => x.d).d.label}`); } continue; }
    const g = add(tipos, o);
    for (const x of ts) g.alvos.add(x.t);
    if (ts.length === 1 && ts[0].d && ts[0].a.sobrescritas?.length) {
      const over = new Set(ts[0].a.sobrescritas.map(s2 => LB.declAt(s2.arquivo, s2.linha, s2.nome)?.id).filter(Boolean));
      g.base = { id: ts[0].d.id, over };
    }
    if (!ts.length) g.motivo = r.alvos.length ? 'variavel' : r.motivo;
  }
  return { gab: { execucao: exec, tipos }, edges, soGab, foraEdition, threads };
}

// ---------- the Rust gabaritos ----------
/** The name token of a call: the last name before the argument list (and a turbofish), its byte offset in the text. */
function callName(text) {
  let head = text.replace(/\s+$/, '');
  const close = (open, closeCh) => {
    let depth = 0;
    for (let i = head.length - 1; i >= 0; i--) {
      if (head[i] === closeCh) depth++;
      else if (head[i] === open) { depth--; if (depth === 0) return i; }
    }
    return -1;
  };
  if (head.endsWith(')')) { const i = close('(', ')'); if (i < 0) return null; head = head.slice(0, i); }
  head = head.replace(/\s+$/, '');
  if (head.endsWith('>')) { const i = close('<', '>'); if (i >= 0) head = head.slice(0, i).replace(/::$/, ''); }
  const name = head.match(/[\p{L}\p{N}_]+$/u)?.[0];
  return name ? { name, offset: Buffer.byteLength(head) - Buffer.byteLength(name) } : null;
}

function rtGabaritos(LB) {
  const src = {};
  const text = (p, s, e) => {
    if (!src[p]) src[p] = fs.readFileSync(path.join(SRC.rt, p));
    return src[p].toString('utf8', s, e);
  };
  // Check the files on disk are the edition's bytes (the line tables must agree).
  const disco = { iguais: 0, diferentes: [] };
  for (const p of Object.keys(LB.D.linhas)) {
    const f = path.join(SRC.rt, p);
    if (!fs.existsSync(f)) { disco.diferentes.push(p); continue; }
    const b = fs.readFileSync(f), t = [0];
    for (let i = 0; i < b.length; i++) if (b[i] === 10) t.push(i + 1);
    if (t.length === LB.D.linhas[p].length && t.every((x, i) => x === LB.D.linhas[p][i])) disco.iguais++; else disco.diferentes.push(p);
  }
  // rust-analyzer (SCIP): references by path@byte; definitions of project symbols to declarations.
  const idx = parseScip(path.join(OUT, 'renderer.scip'));
  const refs = new Map(), defs = new Map();
  for (const d of idx.documents) {
    for (const o of d.occurrences) {
      if (!o.symbol || o.symbol.startsWith('local ')) continue;
      const b = LB.byte(d.path, o.line + 1, o.start);
      if (b === null) continue;
      if (o.roles & 1) { if (!defs.has(o.symbol)) defs.set(o.symbol, { path: d.path, line: o.line + 1, name: lastName(o.symbol.replace(/\(\)\.$|[#.]$/, '')) }); }
      else refs.set(`${d.path}@${b}`, o.symbol);
    }
  }
  const PROJECT = s => /^rust-analyzer cargo renderer /.test(s);
  const symTarget = s => {
    if (!PROJECT(s)) return { t: `ext:${s.replace(/^rust-analyzer cargo /, '')}` };
    const df = defs.get(s);
    if (!df) return { t: `semdecl:${s}` };
    const d = LB.declAt(df.path, df.line, df.name);
    return d ? { t: `decl:${d.id}`, d } : { t: `semdecl:${s}` };
  };
  const tipos = new Map(), used = new Set();
  for (const o of LB.D.ocorrencias) {
    const tx = text(o.path, o.start_byte, o.end_byte);
    const cn = callName(tx);
    if (!cn) continue;
    const k = `${o.path}@${o.start_byte + cn.offset}`;
    const s = refs.get(k);
    if (!s) continue;
    used.add(k);
    const x = symTarget(s);
    const g = { alvos: new Set([x.t]), nome: cn.name };
    // A trait's method (the "base"): the impls of that trait with the same name override it.
    if (x.d && x.d.kind === 'trait_method') {
      const trait = LB.declById.get(x.d.parent);
      const tn = trait ? lastName(trait.label) : null;
      if (tn) g.base = { id: x.d.id, over: new Set(LB.D.declaracoes.filter(d => FN_KINDS.has(d.kind) && lastName(d.label) === cn.name && new RegExp(`\\bimpl\\b[^:]*\\b${tn}\\b[^:]*\\bfor\\b`).test(d.label)).map(d => d.id)) };
    }
    tipos.set(o.id, g);
  }
  // SCIP references to project functions without a Librarian occurrence, by kind (from the source line).
  const OPS = /(\]|#)(add|sub|mul|div|rem|neg|not|index|index_mut|eq|ne|partial_cmp|cmp|lt|le|gt|ge|deref|deref_mut|drop|add_assign|sub_assign|mul_assign|div_assign)\(\)\.$/;
  const soGabTipos = { total: 0, por_tipo: {}, exemplos: {} };
  const lineOf = (p, b) => { const t = LB.D.linhas[p]; let i = 0, j = t.length - 1; while (i < j) { const m = (i + j + 1) >> 1; if (t[m] <= b) i = m; else j = m - 1; } return i; };
  for (const [k, sym] of refs) {
    if (used.has(k) || !/\(\)\.$/.test(sym) || !PROJECT(sym)) continue;
    const at = k.lastIndexOf('@'), p = k.slice(0, at), b = +k.slice(at + 1), li = lineOf(p, b);
    const t = LB.D.linhas[p], ls = t[li], le = li + 1 < t.length ? t[li + 1] : ls + 400;
    const raw = src[p] ?? (src[p] = fs.readFileSync(path.join(SRC.rt, p)));
    const before = raw.toString('utf8', ls, b), after = raw.toString('utf8', b, le).replace(/^[\p{L}\p{N}_]+/u, '').trimStart();
    const line = raw.toString('utf8', ls, le).replace(/\r?\n$/, '');
    // Inside a macro: an open `name!(`/`![` before it on the line, or a line of a multi-line vec![..] (an argument line).
    const openMacro = /\b[\p{L}_][\p{L}\p{N}_]*!\s*[(\[{]/u.test(before) || /^\s*[\p{Lu}][\p{L}\p{N}_:]*\(.*\)\s*,?\s*$/u.test(line) && !/[=;]/.test(line);
    const kind = /^\s*use /.test(line) ? 'use' : OPS.test(sym) ? 'operador do projeto' : openMacro ? 'dentro de macro'
      : !after.startsWith('(') && !after.startsWith('::<') ? 'referência como valor' : 'chamada sem ocorrência';
    const prodTest = /(^|\/)tests\//.test(p) || /#\[cfg\(test\)\]/.test(raw.toString('utf8', 0, b)) ? 'teste' : 'produção';
    const key = `${kind} (${prodTest})`;
    soGabTipos.total++;
    soGabTipos.por_tipo[key] = (soGabTipos.por_tipo[key] ?? 0) + 1;
    const ex = soGabTipos.exemplos[kind] ?? (soGabTipos.exemplos[kind] = []);
    if (ex.length < 5) ex.push(`${p}:${li + 1} ${line.trim().slice(0, 80)}`);
  }
  // Execution: pairs of functions from the probe.
  const exec = new Map(), edges = [], rayon = [];
  const parse = s => { const m = String(s).match(/^(.*):(\d+):([\p{L}\p{N}_]+)$/u); return m ? LB.declAt(m[1], +m[2], m[3]) : null; };
  const seenPairs = new Map(); // caller id -> Set of callee ids
  const executed = new Set();
  let semDecl = 0;
  for (const r of jsonl('execucao-rt.jsonl')) {
    const callee = parse(r.chamado);
    if (!callee) { semDecl++; continue; }
    executed.add(callee.id);
    if (r.chamador === '?') {
      // No caller: a root (the test harness, main) or the first call on a rayon thread (unnamed): an edge lost (Q4).
      if (r.fio === '') rayon.push(r);
      continue;
    }
    const caller = parse(r.chamador);
    if (!caller) { semDecl++; continue; }
    edges.push([caller.id, callee.id]);
    (seenPairs.get(caller.id) ?? seenPairs.set(caller.id, new Set()).get(caller.id)).add(callee.id);
  }
  for (const [owner, occs] of LB.byOwner) {
    const pairs = seenPairs.get(owner);
    if (!pairs) continue;
    const byName = new Map();
    for (const o of occs) {
      const tx = text(o.path, o.start_byte, o.end_byte), cn = callName(tx);
      if (!cn) continue;
      const q = tx.slice(0, cn.offset).match(/([\p{Lu}][\p{L}\p{N}_]*)\s*(?:<[^>]*>)?\s*::\s*$/u)?.[1] ?? '';
      const k = `${cn.name}|${q}`;
      (byName.get(k) ?? byName.set(k, []).get(k)).push(o);
    }
    for (const [k, os] of byName) {
      const [name, q] = k.split('|');
      const qual = d => !q || new RegExp(`\\b${q}\\b`).test(d.label);
      const ts = [...pairs].filter(id => lastName(LB.declById.get(id).label) === name && qual(LB.declById.get(id))).map(id => `decl:${id}`);
      if (!ts.length) continue;
      const answers = new Set(os.map(o => JSON.stringify(o.resposta?.alvos ?? [])));
      const amb = os.length > 1 && !(ts.length === 1 && answers.size === 1 && JSON.parse([...answers][0]).length === 1 && `decl:${JSON.parse([...answers][0])[0]}` === ts[0]);
      for (const o of os) exec.set(o.id, { alvos: new Set(ts), ambiguo: amb });
    }
  }
  return { gab: { execucao: exec, tipos }, edges, rayon, executed, semDecl, disco, soGab: { tipos: soGabTipos }, scipEncoding: [...new Set(idx.documents.map(d => d.encoding))] };
}

// ---------- reach of each workflow (DEC-48, item 2) ----------
function reach(LB, entries, edgeList) {
  const adj = new Map();
  for (const [a, b] of edgeList) (adj.get(a) ?? adj.set(a, new Set()).get(a)).add(b);
  const depth = new Map(entries.map(e => [e, 0]));
  let frontier = [...entries];
  while (frontier.length) {
    const next = [];
    for (const x of frontier) for (const y of adj.get(x) ?? []) if (!depth.has(y)) { depth.set(y, depth.get(x) + 1); next.push(y); }
    frontier = next;
  }
  return depth;
}
function graphEdges(LB) {
  const out = [];
  for (const o of LB.D.ocorrencias) {
    const r = o.resposta;
    if (!r || !LINKED.has(r.resolution) || r.relation === 'decorates') continue;
    for (const t of r.alvos) {
      out.push([o.dono, t]);
      const d = LB.declById.get(t);
      if (d?.kind === 'class') { const i = LB.initOf(t); if (i) out.push([o.dono, i.id]); }
    }
  }
  return out;
}

// ---------- families (Parada 1, Q10, with the two criteria) ----------
const FAMILIAS = [
  { id: 'FM-1', nome: 'Dublê de teste ligado por código de produção', lang: 'hc',
    medir: 'chamador fora de teste, alvo único do Librarian declarado em arquivo de teste; o gabarito de tipos dá outro alvo',
    librarian: 'existe: o Librarian vê sozinho que o alvo está em arquivo de teste e o chamador não',
    test: (LB, o, gs) => LINKED.has(o.resposta?.resolution) && o.resposta.alvos.length === 1 && PY_TEST(LB.declById.get(o.resposta.alvos[0])?.path ?? '') && !o.teste },
  { id: 'FM-2', nome: 'Método de biblioteca com o nome de um método do projeto', lang: '*',
    medir: '`unique` com receptor e sem tipo para um método do projeto, e um gabarito dá um alvo externo',
    librarian: 'parcial: só com uma lista de nomes de bibliotecas conhecidas (opção b do 2.4f); sem ela, não existe',
    test: (LB, o, gs) => o.resposta?.resolution === 'unique' && o.kind === 'method_call_observed' && !o.resposta.type_rule && Object.values(gs).some(g => g && g.alvos.size && [...g.alvos].every(t => t.startsWith('ext:'))) },
  { id: 'FM-3', nome: 'Tipo do elemento de array ou coleção', lang: 'rt',
    medir: 'receptor indexado (`a[i].m(..)`), e o gabarito dá um alvo diferente da resposta',
    librarian: 'existe: a forma `a[i].m(..)` é visível; o alvo não',
    test: (LB, o) => /\]\s*\.\s*[\p{L}_][\p{L}\p{N}_]*\s*\($/u.test((o.texto.match(/^(.*?\()/)?.[1]) ?? '') },
  { id: 'FM-4', nome: 'Despacho pelo método padrão de trait', lang: 'rt',
    medir: '`self.m(..)` dentro de um método padrão de trait, e o gabarito dá a implementação ou a declaração do trait',
    librarian: 'existe: o dono é um método padrão de trait',
    test: (LB, o) => o.dono_kind === 'trait_method' && /^self\./.test(o.texto) },
  { id: 'FM-5', nome: '`T::from` com vários `impl From<X>`', lang: 'rt',
    medir: '`T::from(..)` e o gabarito dá um alvo diferente da resposta',
    librarian: 'existe: o Librarian conta os `impl From` para `T`',
    // A Rust path call's span is the name only (`Patterns::from`), without the arguments.
    test: (LB, o) => /^[\p{Lu}][\p{L}\p{N}_]*(<[^>]*>)?::from\b/u.test(o.texto) },
];
function shape(o) {
  if (o.kind !== 'method_call_observed') return /::/.test(o.texto) ? 'caminho' : 'simples';
  if (/^self\.[\p{L}_][\p{L}\p{N}_]*\(/u.test(o.texto)) return 'self';
  if (/^[\p{L}_][\p{L}\p{N}_.]*\.[\p{L}_][\p{L}\p{N}_]*\s*\(/u.test(o.texto)) return 'nome';
  return 'expressão';
}
function gabKind(LB, gs) {
  const ts = new Set();
  for (const g of Object.values(gs)) for (const t of g?.alvos ?? []) {
    if (t.startsWith('ext:')) ts.add('externo');
    else if (t.startsWith('decl:')) ts.add(PY_TEST(LB.declById.get(t.slice(5))?.path ?? '') ? 'teste' : 'projeto');
    else ts.add('sem declaração');
  }
  return [...ts].sort().join('+') || 'nada';
}

// ---------- statistics ----------
function betaInv(p, a, b) { // bisection on the regularized incomplete beta
  let lo = 0, hi = 1;
  for (let i = 0; i < 100; i++) { const m = (lo + hi) / 2; if (ibeta(m, a, b) < p) lo = m; else hi = m; }
  return (lo + hi) / 2;
}
function lgamma(x) { const c = [76.18009172947146, -86.50532032941677, 24.01409824083091, -1.231739572450155, 0.1208650973866179e-2, -0.5395239384953e-5]; let y = x, t = x + 5.5; t -= (x + 0.5) * Math.log(t); let s = 1.000000000190015; for (const v of c) s += v / ++y; return -t + Math.log(2.5066282746310005 * s / x); }
function ibeta(x, a, b) {
  if (x <= 0) return 0; if (x >= 1) return 1;
  const bt = Math.exp(lgamma(a + b) - lgamma(a) - lgamma(b) + a * Math.log(x) + b * Math.log(1 - x));
  const cf = (x, a, b) => { let c = 1, d = 1 - (a + b) * x / (a + 1); d = 1 / d; let h = d; for (let m = 1; m < 300; m++) { const m2 = 2 * m; let aa = m * (b - m) * x / ((a + m2 - 1) * (a + m2)); d = 1 + aa * d; c = 1 + aa / c; d = 1 / d; h *= d * c; aa = -(a + m) * (a + b + m) * x / ((a + m2) * (a + m2 + 1)); d = 1 + aa * d; c = 1 + aa / c; d = 1 / d; h *= d * c; } return h; };
  return x < (a + 1) / (a + b + 2) ? bt * cf(x, a, b) / a : 1 - bt * cf(1 - x, b, a) / b;
}
const cp = (k, n) => n === 0 ? [null, null] : [k === 0 ? 0 : betaInv(0.025, k, n - k + 1), k === n ? 1 : betaInv(0.975, k + 1, n - k)];
const pct = x => x === null || x === undefined ? '—' : `${(100 * x).toFixed(1).replace('.', ',')}%`;
const frac = (a, b) => b ? `${a}/${b} (${pct(a / b)})` : '0/0';

// ---------- truth validated by the user (fixtures, read only): calibration of each gabarito ----------
function calibracao(lang, LB, gab) {
  const files = lang === 'hc' ? ['amostra-hc.json', 'estrato-unique.json'] : ['amostra-rt.json', 'estrato-unique.json'];
  const out = {};
  const truthTarget = s => {
    const m = String(s).match(/^(.*) \(([^()]+):(\d+)\)$/);
    if (!m) return null;
    const d = LB.declAt(m[2], +m[3], lastName(m[1])) ?? LB.declAt(m[2], +m[3], null);
    return d ? d.id : null;
  };
  const seen = new Set();
  for (const f of files) {
    const j = JSON.parse(fs.readFileSync(path.join(L.REPO, 'tools/precisao/fixtures', f), 'utf8'));
    for (const c of j.chamadas ?? j.amostra ?? j) {
      if (c.projeto && c.projeto !== lang) continue;
      const [p, se] = String(c.ocorrencia).split('@'); const [s, e] = se.split('-').map(Number);
      const o = (LB.byStart.get(`${p}@${s}`) ?? []).find(x => x.end_byte === e);
      if (!o || seen.has(o.id)) continue;
      seen.add(o.id);
      const v = c.verdade ?? {};
      for (const [gn, G] of Object.entries(gab)) {
        const g = G.get(o.id);
        const row = out[gn] ?? (out[gn] = { itens: 0, nao_viu: 0, concorda: 0, discorda: 0, discordancias: [] });
        row.itens++;
        if (!g || !g.alvos.size || g.ambiguo) { row.nao_viu++; continue; }
        const T = [...g.alvos];
        let ok;
        if (v.externo) ok = T.every(t => t.startsWith('ext:') || t.startsWith('gerado:'));
        else if (v.alvo) { const id = truthTarget(v.alvo); ok = id && T.length === 1 && T[0] === `decl:${id}`; }
        else if (v.ambigua) { const ids = new Set(v.ambigua.map(truthTarget).filter(Boolean)); ok = (T.every(t => t.startsWith('decl:') && ids.has(t.slice(5)))) || (g.base && [...ids].every(x => x === g.base.id || g.base.over.has(x))); if (!ok && v.ambigua.some(x => !/\(/.test(x))) ok = T.every(t => t.startsWith('ext:') || ids.has(t.slice(5))); }
        if (ok) row.concorda++; else { row.discorda++; row.discordancias.push(`${c.id} ${c.local}: verdade ${JSON.stringify(v.alvo ?? v.externo ?? v.ambigua)}; gabarito ${T.map(t => t.startsWith('decl:') ? `${LB.declById.get(t.slice(5)).label} (${LB.declById.get(t.slice(5)).path}:${LB.declById.get(t.slice(5)).start_line})` : t).join(', ')}`); }
      }
    }
  }
  return out;
}

// ---------- main ----------
const result = { projetos: {} };
const md = [];
for (const lang of ['hc', 'rt']) {
  const LB = loadLib(lang);
  const G = lang === 'hc' ? pyGabaritos(LB) : rtGabaritos(LB);
  const gab = G.gab;
  const gnames = Object.keys(gab);
  const occs = LB.D.ocorrencias;

  // Classification of every occurrence by every gabarito, and the combined verdict.
  const cls = new Map();
  for (const o of occs) {
    const c = {};
    for (const gn of gnames) c[gn] = classify(LB, o, gab[gn].get(o.id));
    const vals = Object.values(c);
    const seen = vals.filter(v => !UNSEEN.has(v));
    const disc = seen.filter(v => DISCORD.has(v));
    c.vista = seen.length > 0;
    c.combinado = !seen.length ? 'nao_viu' : disc.length && disc.length === seen.length ? disc[0] : disc.length ? 'gabaritos_divergem' : seen[0];
    cls.set(o.id, c);
  }

  // Reach (graph, execution, union) of each workflow.
  const gEdges = graphEdges(LB), eEdges = G.edges, uEdges = [...gEdges, ...eEdges];
  const wf = {};
  for (const [w, W] of Object.entries(WORKFLOWS[lang])) {
    const ent = W.entradas.map(([p, l, n]) => LB.declAt(p, l, n)).filter(Boolean);
    const mar = W.marcos.map(([p, l, n]) => LB.declAt(p, l, n)).filter(Boolean);
    const rg = reach(LB, ent.map(d => d.id), gEdges), re = reach(LB, ent.map(d => d.id), eEdges), ru = reach(LB, ent.map(d => d.id), uEdges);
    const both = [...ru.keys()].filter(x => rg.has(x) && re.has(x)).length;
    const soG = [...ru.keys()].filter(x => rg.has(x) && !re.has(x)), soE = [...ru.keys()].filter(x => !rg.has(x) && re.has(x));
    const inReach = occs.filter(o => ru.has(o.dono));
    const cov = gn => inReach.filter(o => !UNSEEN.has(cls.get(o.id)[gn])).length;
    wf[w] = { nome: W.nome, entradas: ent.map(d => `${d.label} (${d.path}:${d.start_line})`), entradas_nao_achadas: W.entradas.length - ent.length,
      simbolos: ru.size, pelos_dois: both, so_grafo: soG.length, so_execucao: soE.length, chamadas: inReach.length,
      cobertura: Object.fromEntries(gnames.map(gn => [gn, cov(gn)])), vistas_algum: inReach.filter(o => cls.get(o.id).vista).length,
      marcos: `${mar.filter(d => ru.has(d.id)).length}/${W.marcos.length}`, marcos_fora: mar.filter(d => !ru.has(d.id)).map(d => d.label),
      profundidade_uniao: Math.max(0, ...ru.values()), profundidade_grafo: Math.max(0, ...rg.values()), nos_grafo: rg.size,
      so_grafo_exemplos: soG.slice(0, 8).map(id => LB.declById.get(id)?.label), so_execucao_exemplos: soE.slice(0, 8).map(id => LB.declById.get(id)?.label),
      reach: ru };
    if (lang === 'rt') {
      const orphans = (G.rayon ?? []).map(r => { const m = String(r.chamado).match(/^(.*):(\d+):(\w+)$/); return m && LB.declAt(m[1], +m[2], m[3]); }).filter(d => d && ru.has(d.id));
      wf[w].rayon = [...new Set(orphans.map(d => `${d.label} (${d.path}:${d.start_line})`))];
      // Of what only the graph reaches, how much the execution reaches from the callees that lost their caller on a
      // rayon thread: below the cut, not an agreement (Parada 1, Q4).
      const below = reach(LB, [...new Set(orphans.map(d => d.id))], eEdges);
      wf[w].so_grafo_abaixo_do_rayon = soG.filter(x => below.has(x)).length;
    }
    // The rule of three (Q11) counts the "by name" links seen in the reach.
    const nomeIn = inReach.filter(o => !o.teste && grupo(o) === 'provavel_pelo_nome');
    wf[w].pelo_nome = { ligacoes: nomeIn.length, vistas: nomeIn.filter(o => cls.get(o.id).vista).length,
      discordancias: nomeIn.filter(o => ['link_falso_candidato', 'discorda_polimorfica'].includes(cls.get(o.id).combinado)).length };
    const tipoIn = inReach.filter(o => !o.teste && grupo(o) === 'provavel_pelo_tipo');
    wf[w].pelo_tipo = { ligacoes: tipoIn.length, vistas: tipoIn.filter(o => cls.get(o.id).vista).length,
      discordancias: tipoIn.filter(o => ['link_falso_candidato', 'discorda_polimorfica'].includes(cls.get(o.id).combinado)).length };
  }
  if (lang === 'hc') {
    const D = wf.D, sig = occs.filter(o => D.reach.has(o.dono) && /^hc\/integrations\/signal\//.test(o.path));
    D.perda_signal = { chamadas: sig.length, nao_vistas_execucao: sig.filter(o => UNSEEN.has(cls.get(o.id).execucao)).length };
  }
  const inWf = o => Object.entries(wf).filter(([, x]) => x.reach.has(o.dono)).map(([w]) => w);

  // Coverage of each family (Table 5): its population by the criterion the Librarian applies alone (Q10).
  const POP = {
    'FM-1': o => LINKED.has(o.resposta?.resolution) && o.resposta.alvos.length === 1 && PY_TEST(LB.declById.get(o.resposta.alvos[0])?.path ?? '') && !o.teste,
    'FM-2': o => !o.teste && grupo(o) === 'provavel_pelo_nome',
    'FM-3': o => FAMILIAS[2].test(LB, o),
    'FM-4': o => FAMILIAS[3].test(LB, o),
    'FM-5': o => FAMILIAS[4].test(LB, o),
  };
  const famCov = {};
  for (const F of FAMILIAS) {
    if (F.lang !== '*' && F.lang !== lang) continue;
    const xs = occs.filter(POP[F.id]);
    famCov[F.id] = { nome: F.nome, criterio_medir: F.medir, criterio_librarian: F.librarian, populacao: xs.length,
      ...Object.fromEntries(gnames.map(gn => [`vistas_${gn}`, xs.filter(o => !UNSEEN.has(cls.get(o.id)[gn])).length])),
      vistas_algum: xs.filter(o => cls.get(o.id).vista).length,
      discordancias: xs.filter(o => DISCORD.has(cls.get(o.id).combinado)).length };
  }

  // Families.
  const prod = occs.filter(o => !o.teste);
  // An occurrence enters a family only when the gabaritos that saw it agree on a disagreement; when one agrees with
  // the Librarian and another does not, it goes to the divergences, apart (a limit of one gabarito, not a gap).
  const fam = new Map(), divergem = {};
  const lbl = t => t.startsWith('decl:') ? `${LB.declById.get(t.slice(5))?.label} (:${LB.declById.get(t.slice(5))?.start_line})` : t;
  const ex = o => `${o.path}:${o.start_line} \`${o.texto.slice(0, 60)}\` → ${(o.resposta?.alvos ?? []).map(a => lbl(`decl:${a}`)).join(', ') || o.resposta?.resolution}; ${gnames.map(gn => `${gn} ${[...(gab[gn].get(o.id)?.alvos ?? [])].map(lbl).join('|') || '—'}`).join('; ')}`;
  for (const o of occs) {
    const c = cls.get(o.id);
    const discAny = gnames.filter(gn => DISCORD.has(c[gn]));
    if (!discAny.length) continue;
    if (c.combinado === 'gabaritos_divergem') {
      const k = gnames.map(gn => `${gn}: ${c[gn]}`).join('; ');
      const r = divergem[k] ?? (divergem[k] = { total: 0, producao: 0, exemplos: [] });
      r.total++; if (!o.teste) r.producao++;
      if (r.exemplos.length < 8) r.exemplos.push(ex(o));
      continue;
    }
    const gs = Object.fromEntries(gnames.map(gn => [gn, gab[gn].get(o.id)]));
    let f = FAMILIAS.find(F => (F.lang === '*' || F.lang === lang) && F.test(LB, o, gs));
    const key = f ? f.id : `novo|${o.resposta?.resolution}|${shape(o)}|${gabKind(LB, gs)}|${discAny.map(gn => c[gn]).sort()[0]}`;
    const row = fam.get(key) ?? fam.set(key, { chave: key, familia: f ?? null, ocorrencias: [], producao: 0, teste: 0, tipos: {}, workflows: new Set(), lf_em_workflow: 0, por_gabarito: {} }).get(key);
    row.ocorrencias.push(o);
    if (o.teste) row.teste++; else row.producao++;
    for (const gn of discAny) { row.tipos[c[gn]] = (row.tipos[c[gn]] ?? 0) + 1; row.por_gabarito[gn] = (row.por_gabarito[gn] ?? 0) + 1; }
    const ws = inWf(o);
    ws.forEach(w => row.workflows.add(w));
    if (ws.length && discAny.some(gn => c[gn] === 'link_falso_candidato' || c[gn] === 'discorda_polimorfica')) row.lf_em_workflow++;
  }
  const famRows = [...fam.values()].sort((a, b) => b.ocorrencias.length - a.ocorrencias.length);

  // Table 4: the provisional groups, outside tests.
  const t4 = {};
  for (const o of prod) {
    const gp = grupo(o);
    if (!gp) continue;
    const c = cls.get(o.id);
    const row = t4[gp] ?? (t4[gp] = { ligacoes: 0, vistas: 0, vistas_execucao: 0, so_tipos: 0, discordancias: 0, divergem: 0, exemplos: [] });
    row.ligacoes++;
    if (c.vista) row.vistas++;
    if (!UNSEEN.has(c.execucao)) row.vistas_execucao++; else if (!UNSEEN.has(c.tipos)) row.so_tipos++;
    const lf = ['link_falso_candidato', 'discorda_polimorfica'].includes(c.combinado) ? gnames.filter(gn => DISCORD.has(c[gn])) : [];
    if (lf.length) { row.discordancias++; if (row.exemplos.length < 40) row.exemplos.push(`${o.path}:${o.start_line} \`${o.texto.slice(0, 70)}\` → ${o.resposta.alvos.map(a => LB.declById.get(a)?.label).join(', ')} [${lf.map(gn => `${gn}: ${c[gn]}`).join('; ')}; ${gnames.map(gn => `${gn} ${[...(gab[gn].get(o.id)?.alvos ?? [])].map(t => t.startsWith('decl:') ? LB.declById.get(t.slice(5))?.label : t).join('|') || '—'}`).join('; ')}]`); }
    if (c.combinado === 'gabaritos_divergem') row.divergem++;
  }

  for (const r of Object.values(t4)) r.ic = cp(r.discordancias, r.vistas);

  // Populations (Q8): 655/245 (lista.js) and 233/94 (gate pool).
  const poolRe = /(^|\/)tests?\/|test_|_test\.|\/benches\//;
  const toMethod = o => o.resposta?.resolution === 'unique' && o.resposta.alvos.length === 1 && /^(method|trait_method)$/.test(LB.declById.get(o.resposta.alvos[0])?.kind ?? '');
  const lista = occs.filter(o => o.kind === 'method_call_observed' && o.resposta?.resolution === 'unique' && LB.declById.get(o.resposta.alvos[0])?.kind === 'method');
  const portao = occs.filter(o => o.kind === 'method_call_observed' && /function|method/.test(o.dono_kind) && !poolRe.test(o.path) && !o.dono_label.includes('test')
    && /^[A-Za-z_][\w.]*\.\w+\(/.test(o.texto) && !/^self\.\w+\(/.test(o.texto) && toMethod(o));
  const popRow = xs => { const v = xs.filter(o => cls.get(o.id).vista).length, d = xs.filter(o => ['link_falso_candidato', 'discorda_polimorfica'].includes(cls.get(o.id).combinado)).length, dv = xs.filter(o => cls.get(o.id).combinado === 'gabaritos_divergem').length; return { total: xs.length, vistas: v, discordancias: d, divergem: dv, ic: cp(d, v) }; };
  const porRes = {};
  for (const o of occs) { const k = `${o.resposta?.resolution ?? 'sem_resposta'}${o.teste ? ' (teste)' : ''}`; porRes[k] = (porRes[k] ?? 0) + 1; }
  const porCls = {};
  for (const gn of [...gnames, 'combinado']) { porCls[gn] = {}; for (const o of prod) { const k = cls.get(o.id)[gn]; porCls[gn][k] = (porCls[gn][k] ?? 0) + 1; } }

  result.projetos[lang] = {
    ocorrencias: occs.length, producao: prod.length, por_resolucao: porRes, por_classificacao_producao: porCls,
    populacao: { universo: popRow(occs), fora_de_testes: popRow(prod), lista_js: popRow(lista), portao: popRow(portao) },
    workflows: Object.fromEntries(Object.entries(wf).map(([k, v]) => { const { reach: _r, ...rest } = v; return [k, rest]; })),
    grupos: t4, familias: famRows.map(r => ({ chave: r.chave, id: r.familia?.id ?? null, nome: r.familia?.nome ?? null, total: r.ocorrencias.length, producao: r.producao, teste: r.teste, tipos: r.tipos, por_gabarito: r.por_gabarito, workflows: [...r.workflows], lf_em_workflow: r.lf_em_workflow,
      exemplos: r.ocorrencias.filter(o => !o.teste).concat(r.ocorrencias.filter(o => o.teste)).slice(0, 6).map(o => `${o.path}:${o.start_line} \`${o.texto.slice(0, 60)}\` → ${(o.resposta?.alvos ?? []).map(a => LB.declById.get(a)?.label).join(', ') || o.resposta?.resolution}; ${gnames.map(gn => `${gn} ${[...(gab[gn].get(o.id)?.alvos ?? [])].map(t => t.startsWith('decl:') ? LB.declById.get(t.slice(5))?.label : t.replace(/^ext:/, 'externo ')).join('|') || '—'}`).join('; ')}`) })),
    calibracao: calibracao(lang, LB, gab), divergem, cobertura_familias: famCov,
    so_do_gabarito: G.soGab, fora_da_edition: G.foraEdition ?? 0, extras: lang === 'rt' ? { disco: G.disco, sem_declaracao: G.semDecl, scip_encoding: G.scipEncoding, rayon: (G.rayon ?? []).map(r => `${r.chamado} [${r.fio || 'sem nome'}]`) } : { pares_em_outra_thread: G.threads },
  };
  // A small known list, measured (PLANO, 2.4d and the UQ): where each lands.
  result.projetos[lang].conhecidos = occs.filter(o => {
    const t = o.resposta?.alvos?.length === 1 ? LB.declById.get(o.resposta.alvos[0])?.label : '';
    return /FakeCurl\.|Ping\.get_body|impl Groups::len|impl Sorteio::next|impl Color::clamp/.test(t ?? '') && LINKED.has(o.resposta.resolution);
  }).map(o => ({ local: `${o.path}:${o.start_line}`, texto: o.texto.slice(0, 50), alvo: LB.declById.get(o.resposta.alvos[0]).label, teste: o.teste, ...Object.fromEntries(gnames.map(gn => [gn, cls.get(o.id)[gn]])) }));
}
fs.writeFileSync(path.join(OUT, 'medicao.json'), JSON.stringify(result, null, 1) + '\n');
console.log('medicao.json gravado');

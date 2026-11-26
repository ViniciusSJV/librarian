// Draws the stratum of `unique` links with a receiver (before step 2.4e): 15 calls of the Healthchecks and
// 15 of the ray tracer, and writes their skeleton, without the truth. Not a step of Phase 2: measure only,
// run once, like amostra.js; the gate does not run it. The truth is filled by hand and validated by the
// user (DEC-22).
//
// Population: the gate's pool (object.method() outside tests, without self.m(); lib.js, `pool`) on runs
// generated now by the binary under test, on a copy of .studio, whose answer is `unique` to a method
// (`method`, `trait_method`). A `unique` the receiver's type confirms keeps its label (SEARCH.md), so it
// stays here; each call records what the graph holds of the receiver's type, for the report only.
// The draw: sha256(SEED | path@start-end) in ascending order, the first 15 of each language. Nothing is
// excluded or forced; the known false links (PLANO, 2.4d) are only marked.
//
//   node tools/precisao/estrato-unique.js --studio .studio --saida <dir> [--bin target/release] [--fontes <sha256>]
'use strict';
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const L = require('./lib');

const SEED = 'estrato-unique-2026-11-25';
const PER_LANG = 15;
const METHODS = new Set(['method', 'trait_method']);
// The 18 known false links (PLANO, 2.4d), as `path:line` and the target's label.
const KNOWN = {
  hc: { 'Ping.get_body': ['hc/api/management/commands/smtpd.py:37', 'hc/api/management/commands/smtpd.py:41', 'hc/front/views.py:800', 'hc/front/views.py:805'] },
  rt: {
    'impl Groups::len': ['src/obj.rs:90', 'src/camera.rs:129', 'src/count.rs:133', 'src/count.rs:151'],
    'impl Sorteio::next': ['src/obj.rs:19', 'src/obj.rs:21', 'src/obj.rs:22', 'src/obj.rs:23', 'src/obj.rs:28', 'src/obj.rs:63', 'src/obj.rs:66', 'src/obj.rs:81', 'src/count.rs:133', 'src/count.rs:151'],
  },
};

// The call's whole written receiver, as `written_receiver` in resolve.rs: `a.b.m(x)` -> `a.b`, `a.f(x).m()` -> `a.f(x)`.
function matchingOpen(t, open, close) {
  let depth = 0;
  for (let i = t.length - 1; i >= 0; i--) {
    if (t[i] === close) depth++;
    else if (t[i] === open) { if (--depth < 0) return null; if (depth === 0) return i; }
  }
  return null;
}
function writtenReceiver(text) {
  let head = text.trimEnd();
  if (head.endsWith(')')) { const i = matchingOpen(head, '(', ')'); if (i === null) return null; head = head.slice(0, i); }
  if (head.endsWith('>')) { const i = matchingOpen(head, '<', '>'); if (i === null) return null; head = head.slice(0, i).replace(/(::)+$/, ''); }
  const name = head.match(/[\p{L}\p{N}_]+$/u)?.[0];
  if (!name) return null;
  const r = head.slice(0, head.length - name.length).trimEnd();
  return r.endsWith('.') ? (r.slice(0, -1).trimEnd() || null) : null;
}

/** What the graph holds of the receiver's type: the receiver_type that starts at the call and ends with the
 *  written receiver (as `receiver_type_node` in resolve.rs), its rule, and the declarations its name
 *  reaches. Python types settled at query time (return, field of another file; 2.3c) are not in the graph. */
function receiverType(G, byStart, p, target) {
  const recv = writtenReceiver(p.occ.label);
  const ev = p.edge.evidence;
  const end = recv === null ? null : ev.start_byte + Buffer.byteLength(recv);
  const id = (byStart.get(`${ev.path}@${ev.start_byte}`) ?? []).find(x => x.end === end)?.id;
  if (!id) return { receptor: recv, receiver_type: null, resumo: 'sem tipo escrito no grafo' };
  const named = G.g.edges.filter(e => e.kind === 'name_candidate' && e.from === id).map(e => G.g.nodes[e.to]).filter(n => /^(class|struct|enum|trait)$/.test(n.kind));
  const owner = G.g.nodes[G.parent[target]]?.label ?? '';
  const isOwner = named.some(n => owner === n.label || owner.startsWith(`impl ${n.label}`) || owner.includes(` for ${n.label}`));
  const regra = G.g.nodes[id].type_rule ?? null;
  return {
    receptor: recv, receiver_type: { regra, nomes: named.map(n => `${n.label} (${G.where(Object.keys(G.g.nodes).find(k => G.g.nodes[k] === n))})`) },
    resumo: named.length ? `tipo do projeto (${regra}), ${isOwner ? 'o dono do alvo' : 'outro que o dono do alvo'}` : `tipo escrito que não nomeia declaração do projeto (${regra})`,
  };
}

(async () => {
  const opt = L.parseArgs(process.argv.slice(2));
  if (!opt.studio || !opt.saida) throw new Error('uso: --studio <.studio> --saida <dir> [--bin <dir>] [--fontes <sha256>]');
  const bin = L.prepareBinaries(opt.bin ?? path.join(L.REPO, 'target/release'));
  if (opt.fontes && bin.fontes?.sha256 !== opt.fontes) { console.error(`fontes ${bin.fontes?.sha256 ?? 'desconhecidas'}, pedido ${opt.fontes}`); process.exit(2); }
  const studio = path.resolve(opt.studio);
  const keys = [L.DEFAULTS.hc, L.DEFAULTS.rt].map(s => s.split('/')[0]);
  const out = { semente: SEED, por_linguagem: PER_LANG, binarios: bin, projetos: {} };
  await L.withStudio({ studio, bin: bin.pasta, keys }, async ({ api, root, before }) => {
    out.studio_original_antes = before;
    for (const lang of ['hc', 'rt']) {
      const [key, run] = L.DEFAULTS[lang].split('/');
      const fresh = await api('POST', '/api/generate', { key });
      const old = JSON.parse(fs.readFileSync(path.join(root, '.studio', 'projects', key, 'runs', run, 'run.json'), 'utf8'));
      const G = L.loadGraph(root, key, fresh.id), P = L.pool(G);
      const owners = new Map();
      for (const p of P) (owners.get(p.owner.id) ?? owners.set(p.owner.id, []).get(p.owner.id)).push(p.occ.id);
      // Asked as the gate asks (lib.js, `answers`), keeping the target's id.
      const targetOf = new Map();
      for (const [owner, occs] of owners) {
        const c = await api('POST', '/api/calls', { key, run: fresh.id, node: owner, depth: 1, chosen: {} });
        for (const id of occs) {
          // A link cut by the node limit is still the answer (lib.js, `answers`).
          const ls = [...c.links, ...c.unresolved.filter(u => u.reason === 'node_limit')].filter(x => x.occurrence === id);
          if (ls.length === 1 && ls[0].resolution === 'unique') targetOf.set(id, ls[0]);
        }
      }
      const byStart = new Map();
      for (const [id, n] of Object.entries(G.g.nodes)) {
        if (n.kind !== 'receiver_type') continue;
        const m = id.match(/:(\d+):(\d+)$/), ev = G.g.edges.find(e => e.kind === 'observes' && e.to === id)?.evidence;
        if (!ev) continue;
        const k = `${ev.path}@${m[1]}`;
        (byStart.get(k) ?? byStart.set(k, []).get(k)).push({ id, end: +m[2] });
      }
      const known = new Map();
      for (const [label, at] of Object.entries(KNOWN[lang])) for (const a of at) known.set(`${a}|${label}`, true);
      const population = P.filter(p => targetOf.has(p.occ.id) && METHODS.has(G.g.nodes[targetOf.get(p.occ.id).to].kind)).map(p => {
        const l = targetOf.get(p.occ.id), target = G.g.nodes[l.to];
        const at = `${p.edge.evidence.path}:${p.edge.evidence.start_line}`;
        return {
          p, l, target, at, conhecido: known.has(`${at}|${target.label}`),
          ordem: crypto.createHash('sha256').update(`${SEED}|${p.key}`).digest('hex'),
        };
      });
      const byKind = {}, byType = {};
      for (const x of P) {
        const l = targetOf.get(x.occ.id);
        if (l) { const k = G.g.nodes[l.to].kind; byKind[k] = (byKind[k] ?? 0) + 1; }
      }
      const rows = population.map(x => ({ ...x, tipo: receiverType(G, byStart, x.p, x.l.to) }));
      for (const r of rows) { const k = r.tipo.resumo.replace(/ \(.*?\)/, ''); byType[k] = (byType[k] ?? 0) + 1; }
      const drawn = [...rows].sort((a, b) => a.ordem.localeCompare(b.ordem)).slice(0, PER_LANG);
      out.projetos[lang] = {
        run_novo: fresh.id, grafo_novo_igual_ao_de_referencia: fresh.graph_sha256 === old.graph_sha256,
        pool: P.length, unique_no_pool_por_tipo_de_alvo: byKind, populacao: rows.length, populacao_por_tipo_do_receptor: byType,
        conhecidos_na_populacao: rows.filter(r => r.conhecido).map(r => `${r.at} ${r.target.label}`),
        sorteadas: drawn.map((r, i) => ({
          id: `UQ-${lang === 'hc' ? i + 1 : PER_LANG + i + 1}`, ordem: r.ordem, ocorrencia: r.p.key, local: r.at,
          texto: r.p.occ.label.split(/\r?\n/)[0].trim().slice(0, 120), dono: r.p.owner.label,
          alvo: `${r.target.label} (${G.where(r.l.to)})`, resolution: r.l.resolution, candidates: r.l.candidates,
          receptor: r.tipo.receptor, receiver_type: r.tipo.receiver_type, tipo: r.tipo.resumo, conhecido: r.conhecido,
        })),
      };
    }
  });
  out.studio_original_depois = Object.fromEntries(keys.map(k => [k, L.hashProject(studio, k)]));
  out.studio_original_inalterado = keys.every(k => out.studio_original_depois[k].sha256 === out.studio_original_antes[k].sha256);
  fs.mkdirSync(opt.saida, { recursive: true });
  fs.writeFileSync(path.join(opt.saida, 'estrato-unique.esqueleto.json'), JSON.stringify(out, null, 1) + '\n');
  console.log(`semente ${SEED}; fontes ${bin.fontes?.sha256 ?? 'desconhecidas'}; .studio original inalterado: ${out.studio_original_inalterado}`);
  for (const [lang, r] of Object.entries(out.projetos)) {
    console.log(`${lang}: pool ${r.pool}, unique por alvo ${JSON.stringify(r.unique_no_pool_por_tipo_de_alvo)}, população ${r.populacao} ${JSON.stringify(r.populacao_por_tipo_do_receptor)}, conhecidos ${r.conhecidos_na_populacao.length}, grafo igual ao de referência ${r.grafo_novo_igual_ao_de_referencia}`);
    for (const s of r.sorteadas) console.log(`  ${s.id} ${s.local} ${s.texto.slice(0, 60)} -> ${s.alvo} | ${s.tipo}${s.conhecido ? ' | CONHECIDO' : ''}`);
  }
})().catch(e => { console.error(e.message); process.exit(1); });

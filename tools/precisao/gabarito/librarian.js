// What the Librarian answers for every call occurrence (step 2.4e, DEC-48): fresh runs of the Healthchecks and of the
// ray tracer, on a copy of .studio (lib.js, `withStudio`), and `/api/calls` at depth 1 asked once per owner of a call.
// Writes, per project, the occurrences with the answer, the declarations and the line table of each source snapshot
// of the edition (to turn a gabarito's line and column into the edition's bytes, CRLF included; Q7).
// Measure only: nothing here changes a rule, the gate, the original .studio or a project.
//
//   node tools/precisao/gabarito/librarian.js --studio .studio --saida <dir> [--bin target/release] [--fontes <sha256>]
'use strict';
const fs = require('fs'), path = require('path');
const L = require('../lib');

const CALL_KINDS = new Set(['call_observed', 'method_call_observed']);

/** Test code: a test file or folder (Python, Rust `tests/`) or a Rust `tests_*` module / `test` in the owner's label. */
function isTest(lang, p, ownerLabel) {
  if (lang === 'hc') return /(^|\/)tests?\/|(^|\/)test_[^/]*\.py$|_test\.py$/.test(p) || /(^|\.)test/i.test(ownerLabel ?? '') && /tests?/.test(p);
  return /(^|\/)tests\/|\/benches\//.test(p) || /(^|::)tests?(_\w+)?::/.test(ownerLabel ?? '');
}

function lineTable(buf) {
  const starts = [0];
  for (let i = 0; i < buf.length; i++) if (buf[i] === 10) starts.push(i + 1);
  return starts;
}

(async () => {
  const opt = L.parseArgs(process.argv.slice(2));
  if (!opt.studio || !opt.saida) throw new Error('uso: --studio <.studio> --saida <dir> [--bin <dir>] [--fontes <sha256>]');
  const bin = L.prepareBinaries(opt.bin ?? path.join(L.REPO, 'target/release'));
  if (opt.fontes && bin.fontes?.sha256 !== opt.fontes) { console.error(`fontes ${bin.fontes?.sha256 ?? 'desconhecidas'}, pedido ${opt.fontes}`); process.exit(2); }
  const studio = path.resolve(opt.studio);
  const keys = [L.DEFAULTS.hc, L.DEFAULTS.rt].map(s => s.split('/')[0]);
  fs.mkdirSync(opt.saida, { recursive: true });
  const resumo = { binarios: bin, projetos: {} };
  await L.withStudio({ studio, bin: bin.pasta, keys }, async ({ api, root, before }) => {
    resumo.studio_original_antes = before;
    for (const lang of ['hc', 'rt']) {
      const t0 = Date.now();
      const [key, run] = L.DEFAULTS[lang].split('/');
      const fresh = await api('POST', '/api/generate', { key });
      const old = JSON.parse(fs.readFileSync(path.join(root, '.studio', 'projects', key, 'runs', run, 'run.json'), 'utf8'));
      const runDir = path.join(root, '.studio', 'projects', key, 'runs', fresh.id);
      const G = L.loadGraph(root, key, fresh.id);
      const N = G.g.nodes;

      // Line tables of every source of the edition, from its snapshot (the bytes the graph's offsets count).
      const linhas = {};
      for (const l of fs.readFileSync(path.join(runDir, 'edition', 'sources.jsonl'), 'utf8').split('\n').filter(Boolean)) {
        const s = JSON.parse(l);
        if (s.language === 'auxiliary') continue;
        const snap = path.join(runDir, 'edition', 'snapshots', s.sha256);
        if (fs.existsSync(snap)) linhas[s.path] = lineTable(fs.readFileSync(snap));
      }

      const decls = [];
      for (const [id, ev] of Object.entries(G.declared)) {
        const n = N[id], m = id.match(/:(\d+):(\d+)$/);
        decls.push({ id, kind: n.kind, label: n.label, path: ev.path, start_line: ev.start_line, end_line: ev.end_line,
          start_byte: m ? +m[1] : ev.start_byte, end_byte: m ? +m[2] : ev.end_byte, parent: G.parent[id] ?? null });
      }

      const occs = new Map(), owners = new Map();
      for (const e of G.g.edges) {
        if (e.kind !== 'observes' || !CALL_KINDS.has(N[e.to].kind)) continue;
        const owner = N[e.from], ev = e.evidence;
        occs.set(e.to, {
          id: e.to, chave: L.occKey(ev), kind: N[e.to].kind, texto: N[e.to].label.split(/\r?\n/)[0].slice(0, 200),
          path: ev.path, start_byte: ev.start_byte, end_byte: ev.end_byte, start_line: ev.start_line, end_line: ev.end_line,
          dono: e.from, dono_kind: owner.kind, dono_label: owner.label, teste: isTest(lang, ev.path, owner.label), resposta: null,
        });
        (owners.get(e.from) ?? owners.set(e.from, []).get(e.from)).push(e.to);
      }

      const erros = {};
      let perguntas = 0;
      for (const [owner, ids] of owners) {
        let c;
        try { c = await api('POST', '/api/calls', { key, run: fresh.id, node: owner, depth: 1, chosen: {} }); perguntas++; }
        catch (err) { const k = `${N[owner].kind}: ${String(err.message).slice(0, 80)}`; erros[k] = (erros[k] ?? 0) + ids.length; continue; }
        const byOcc = new Map();
        for (const x of [...c.links, ...c.unresolved]) {
          if (x.from !== owner && !ids.includes(x.occurrence)) continue;
          (byOcc.get(x.occurrence) ?? byOcc.set(x.occurrence, []).get(x.occurrence)).push(x);
        }
        for (const id of ids) {
          const xs = byOcc.get(id) ?? [];
          const links = xs.filter(x => x.to && x.reason !== 'node_limit'), cut = xs.filter(x => x.reason === 'node_limit');
          const un = xs.find(x => x.reason && x.reason !== 'node_limit');
          const first = links[0] ?? cut[0];
          occs.get(id).resposta = un && !first
            ? { resolution: un.reason, candidates: un.candidates ?? 0, alvos: [] }
            : first
              ? { resolution: first.resolution, candidates: first.candidates, relation: first.relation ?? 'calls', type_rule: first.type_rule ?? null,
                  basis: first.basis ?? null, alvos: [...new Set([...links, ...cut].map(x => x.to))], node_limit: cut.length > 0,
                  via: [...new Set(xs.map(x => x.via).filter(Boolean))] }
              : { resolution: 'sem_resposta', candidates: 0, alvos: [] };
        }
      }
      const out = {
        projeto: lang, key, run_novo: fresh.id, grafo_novo_igual_ao_de_referencia: fresh.graph_sha256 === old.graph_sha256,
        graph_sha256: fresh.graph_sha256, git: fresh.git ?? null, linhas, declaracoes: decls, ocorrencias: [...occs.values()],
        donos: owners.size, perguntas, erros_por_tipo_de_dono: erros, segundos: (Date.now() - t0) / 1000,
      };
      fs.writeFileSync(path.join(opt.saida, `librarian-${lang}.json`), JSON.stringify(out));
      const porRes = {};
      for (const o of out.ocorrencias) { const r = o.resposta?.resolution ?? 'sem_resposta'; porRes[r] = (porRes[r] ?? 0) + 1; }
      resumo.projetos[lang] = { run_novo: fresh.id, grafo_igual: out.grafo_novo_igual_ao_de_referencia, ocorrencias: out.ocorrencias.length,
        donos: owners.size, perguntas, erros_por_tipo_de_dono: erros, por_resolucao: porRes, segundos: out.segundos };
      console.log(lang, JSON.stringify(resumo.projetos[lang]));
    }
  });
  resumo.studio_original_depois = Object.fromEntries(keys.map(k => [k, L.hashProject(studio, k)]));
  resumo.studio_original_inalterado = keys.every(k => resumo.studio_original_depois[k].sha256 === resumo.studio_original_antes[k].sha256);
  fs.writeFileSync(path.join(opt.saida, 'librarian-resumo.json'), JSON.stringify(resumo, null, 1) + '\n');
  console.log(`fontes ${bin.fontes?.sha256 ?? 'desconhecidas'}; .studio original inalterado: ${resumo.studio_original_inalterado}`);
})().catch(e => { console.error(e.stack || e.message); process.exit(1); });

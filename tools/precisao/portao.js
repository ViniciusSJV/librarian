// Precision gate of Phase 2 (step 2.0). One command measures, on a copy of .studio and with the
// binaries under test: the population of object.method() calls, the validated sample, the
// synthetic levels N0–N8 and the audit of facts F1–F26; with --base it applies the gate rule.
//
//   node tools/precisao/portao.js --studio .studio [--bin target/release] [--saida <dir>]
//                                 [--base tools/precisao/linha-de-base.json] [--gravar-base <arquivo>]
//   node tools/precisao/portao.js --so-sinteticos [--bin target/release]     (sem dados reais; CI)
//
// Exit code: 0 approved (or no --base), 1 rejected, 2 could not measure.
'use strict';
const fs = require('fs'), path = require('path'), os = require('os'), crypto = require('crypto');
const { execFileSync } = require('child_process');
const L = require('./lib');

const FIX = path.join(__dirname, 'fixtures');
const LANGS = { hc: 'Python (Healthchecks)', rt: 'Rust (ray tracer)' };
const CATS = ['correta', 'link_falso', 'candidatos_falsos', 'ambigua', 'sem_candidato'];
// `scope` (step 2.4a, DEC-42): a plain call settled by a definition in the caller's own module or
// block. A single-target link like the others.
const LINKED = new Set(['unique', 'receiver', 'type', 'trait_default', 'import', 'scope', 'chosen']);
/** Kinds a settled call builds instead of calling (DEC-39). */
const INSTANTIATED = new Set(['class', 'struct', 'variant']);

const readJson = f => JSON.parse(fs.readFileSync(f, 'utf8'));
const first = t => t.split(/\r?\n/)[0].trim();
const matches = (text, want) => want.endsWith('(') ? first(text).startsWith(want) : first(text) === want;
const stratum = a => a.resolution === 'unique' ? `unique->${a.targets[0]?.kind === 'function' ? 'function' : 'outro'}` : a.resolution;
const show = t => `${t.label} (${t.at})`;

/** One call of the sample against its truth. A truth `ambigua` (Q6 of 2.3, the rule of `dyn` in
 *  2.1) lists the bodies that may run: no single link is right, and a homonym is ambiguous when one
 *  of them is among its candidates. A link to a class, struct or variant must be `instantiates`
 *  (DEC-39). */
function classify(a, truth) {
  const shown = a.targets.map(show);
  if (truth.ambigua) {
    const any = truth.ambigua.some(t => shown.includes(t));
    if (a.resolution === 'no_candidate_in_project') return 'sem_candidato';
    if (a.resolution === 'above_candidate_limit') return 'ambigua';
    if (a.resolution === 'homonym') return any ? 'ambigua' : 'candidatos_falsos';
    return 'link_falso';
  }
  const right = truth.alvo && shown.includes(truth.alvo);
  if (a.resolution === 'no_candidate_in_project') return 'sem_candidato';
  if (a.resolution === 'above_candidate_limit') return truth.alvo ? 'ambigua' : 'candidatos_falsos';
  if (a.resolution === 'homonym') return right ? 'ambigua' : 'candidatos_falsos';
  if (LINKED.has(a.resolution)) {
    // DEC-40: a call written in a decorator is right only as `decorates`, and `decorates` is
    // right only for a truth that says so (`relacao: decora`).
    const decora = truth.relacao === 'decora';
    if (decora !== (a.relation === 'decorates')) return 'link_falso';
    const built = a.targets.length === 1 && INSTANTIATED.has(a.targets[0].kind);
    if (!decora && built && a.relation !== 'instantiates') return 'link_falso';
    return shown.length === 1 && right ? 'correta' : 'link_falso';
  }
  throw new Error(`resolução desconhecida: ${a.resolution}`);
}

/** The rule before DEC-40 (step 2.3a), frozen: the gate classifies every call by both and
 *  reports where they differ, which is the proof that DEC-40 keeps the baseline's verdict. */
function classifyBefore40(a, truth) {
  const shown = a.targets.map(show);
  if (truth.ambigua) {
    const any = truth.ambigua.some(t => shown.includes(t));
    if (a.resolution === 'no_candidate_in_project') return 'sem_candidato';
    if (a.resolution === 'above_candidate_limit') return 'ambigua';
    if (a.resolution === 'homonym') return any ? 'ambigua' : 'candidatos_falsos';
    return 'link_falso';
  }
  const right = truth.alvo && shown.includes(truth.alvo);
  if (a.resolution === 'no_candidate_in_project') return 'sem_candidato';
  if (a.resolution === 'above_candidate_limit') return truth.alvo ? 'ambigua' : 'candidatos_falsos';
  if (a.resolution === 'homonym') return right ? 'ambigua' : 'candidatos_falsos';
  if (LINKED.has(a.resolution)) {
    const built = a.targets.length === 1 && INSTANTIATED.has(a.targets[0].kind);
    if (built && a.relation !== 'instantiates') return 'link_falso';
    return shown.length === 1 && right ? 'correta' : 'link_falso';
  }
  throw new Error(`resolução desconhecida: ${a.resolution}`);
}

/** Part E: one asserted call. A truth `ambigua` (DEC-43, V3 of 2.4b) lists the bodies that may run: a
 *  homonym with one of them is suspect, as it is ambiguous in the sample; a single link is wrong. */
function classifyFactCall(a, truth) {
  if (!a || ['no_candidate_in_project', 'above_candidate_limit'].includes(a.resolution)) return 'nao_depende';
  const shown = a.targets.map(show), right = truth.ambigua ? truth.ambigua.some(t => shown.includes(t)) : truth.alvo && shown.includes(truth.alvo);
  if (truth.ambigua && a.resolution !== 'homonym') return 'errado';
  if (a.resolution === 'homonym') return right ? 'suspeito' : 'errado';
  return shown.length === 1 && right ? 'correto' : 'errado';
}
const WORST = ['nao_depende', 'correto', 'suspeito', 'errado'];

/** What /api/calls answers for each call of `owner`, with its text and line. */
async function callsOf(api, key, run, G, owner) {
  const c = await api('POST', '/api/calls', { key, run, node: owner, depth: 1, chosen: {} });
  const by = new Map();
  for (const l of c.links) {
    const o = by.get(l.occurrence) ?? { text: l.text, line: l.evidence.start_line, resolution: l.resolution, candidates: l.candidates, targets: [], relation: l.relation ?? 'calls' };
    o.targets.push({ label: G.g.nodes[l.to].label, kind: G.g.nodes[l.to].kind, at: G.where(l.to) }); by.set(l.occurrence, o);
  }
  for (const u of c.unresolved) {
    if (u.reason === 'node_limit') { const o = by.get(u.occurrence) ?? { text: u.text, line: u.evidence.start_line, resolution: u.resolution, candidates: u.candidates, targets: [] };
      o.targets.push({ label: G.g.nodes[u.to].label, kind: G.g.nodes[u.to].kind, at: G.where(u.to) }); by.set(u.occurrence, o); continue; }
    by.set(u.occurrence, { text: u.text, line: u.evidence.start_line, resolution: u.reason, candidates: u.candidates, targets: [] });
  }
  return [...by.values()];
}

/** The synthetic levels: each project generated afresh with the binary under test. */
async function levels(api, root) {
  const fx = readJson(path.join(FIX, 'niveis.json')), out = [];
  const runs = {};
  for (const p of ['py', 'rs']) {
    const { key } = await api('POST', '/api/project', { root: path.join(__dirname, 'sinteticos', p) });
    const { id: run } = await api('POST', '/api/generate', { key });
    runs[p] = { key, run, G: L.loadGraph(root, key, run) };
  }
  for (const c of fx.casos) {
    const { key, run, G } = runs[c.projeto];
    const owner = Object.keys(G.g.nodes).find(id => G.g.nodes[id].category === 'symbol' && G.g.nodes[id].label === c.dono && G.declared[id]);
    const found = owner && (await callsOf(api, key, run, G, owner)).find(x => x.line === c.linha && matches(x.text, c.texto));
    const now = found ? { resolution: found.resolution, candidates: found.candidates, alvos: found.targets.map(show).sort(), relacao: found.relation ?? null } : null;
    const same = now && now.resolution === c.atual.resolution && now.candidates === c.atual.candidates && JSON.stringify(now.alvos) === JSON.stringify([...c.atual.alvos].sort());
    let estado;
    if (!now) estado = 'ocorrência não encontrada';
    else if (c.alvo.relacao) {
      const linked = now.alvos.length === 1 && now.alvos[0] === c.alvo.alvo && now.resolution !== 'homonym';
      // Step 2.3a: an `instancia` target is reached by a link with `relation: instantiates`.
      if (linked && c.alvo.relacao === 'instancia' && now.relacao === 'instantiates') estado = 'no alvo';
      else estado = (same ? 'igual ao atual' : 'mudou') + (linked ? ' (sai como "chama", fora do alvo)' : '');
    }
    else if (now.alvos.length === 1 && now.alvos[0] === c.alvo.alvo && now.resolution === c.alvo.resolution) estado = c.alvo.passo === 'já correto' ? 'no alvo (já era)' : 'no alvo';
    else estado = same ? 'igual ao atual' : 'mudou';
    out.push({ id: c.id, atual_registrado: c.atual, agora: now, alvo: c.alvo, estado });
  }
  return out;
}

/** Population, sample and facts of one real project, on a run generated by the binary under test. */
async function project(api, root, lang, key, run) {
  const G = L.loadGraph(root, key, run), P = L.pool(G);
  const owners = new Map();
  for (const p of P) (owners.get(p.owner.id) ?? owners.set(p.owner.id, []).get(p.owner.id)).push(p.occ.id);
  const A = await L.answers(api, key, run, G, owners);
  const pop = { total: P.length, por_resolution: {}, por_estrato: {}, metodo_para_funcao: 0 };
  for (const p of P) {
    const a = A.get(p.occ.id);
    pop.por_resolution[a.resolution] = (pop.por_resolution[a.resolution] ?? 0) + 1;
    pop.por_estrato[stratum(a)] = (pop.por_estrato[stratum(a)] ?? 0) + 1;
    if (a.resolution !== 'homonym' && a.targets.length === 1 && a.targets[0].kind === 'function') pop.metodo_para_funcao++;
  }
  const byKey = new Map(P.map(p => [p.key, p]));
  const fx = readJson(path.join(FIX, `amostra-${lang}.json`));
  // Entries outside the population (calls by path, strata added in 2.1): found among every call of
  // the run, and asked for with their owners.
  const outside = fx.chamadas.filter(c => !byKey.has(c.ocorrencia)).map(c => c.ocorrencia);
  if (outside.length) {
    const extra = new Map();
    for (const e of G.g.edges) {
      if (e.kind !== 'observes' || !outside.includes(L.occKey(e.evidence)) || !/call_observed$/.test(G.g.nodes[e.to].kind)) continue;
      byKey.set(L.occKey(e.evidence), { occ: G.g.nodes[e.to], owner: G.g.nodes[e.from] });
      (extra.get(e.from) ?? extra.set(e.from, []).get(e.from)).push(e.to);
    }
    for (const [id, a] of await L.answers(api, key, run, G, extra)) A.set(id, a);
  }
  const sample = fx.chamadas.map(c => {
    const p = byKey.get(c.ocorrencia), a = p && A.get(p.occ.id);
    if (!a) return { id: c.id, estrato: c.estrato, categoria: 'ocorrência não encontrada', resposta: null };
    const tag = { instantiates: ' [instancia]', decorates: ' [decora]' }[a.relation] ?? '';
    return { id: c.id, estrato: c.estrato, parte_c: c.parte_c, categoria: classify(a, c.verdade), categoria_regra_anterior: classifyBefore40(a, c.verdade),
      resposta: `${a.resolution} (${a.candidates})${a.targets.length && a.resolution !== 'homonym' ? ' → ' + a.targets.map(show).join(' | ') : ''}${tag}`, validada: c.verdade_validada_por };
  });
  // DEC-40: the declaration that runs the call (`executor`), when the truth names one: whether its
  // own answer shows the occurrence. Recorded; never rejects.
  const executores = [];
  for (const c of fx.chamadas.filter(c => c.verdade.executor)) {
    const p = byKey.get(c.ocorrencia);
    const id = Object.keys(G.g.nodes).find(n => G.g.nodes[n].category === 'symbol' && G.declared[n] && show({ label: G.g.nodes[n].label, at: G.where(n) }) === c.verdade.executor);
    const answer = id && p ? await api('POST', '/api/calls', { key, run, node: id, depth: 1, chosen: {} }) : null;
    const shows = !!answer && [...answer.links, ...answer.unresolved].some(x => x.occurrence === p.occ.id);
    executores.push({ id: c.id, executor: c.verdade.executor, dono_no_grafo: p ? `${p.owner.label}` : null, mostra: shows });
  }
  return { pop, sample, executores, G, A };
}

async function facts(api, root, key, run) {
  const G = L.loadGraph(root, key, run);
  const fx = readJson(path.join(FIX, 'fatos.json'));
  const recorded = fs.readFileSync(path.join(root, '.studio', 'projects', key, 'facts.jsonl'), 'utf8').trim().split('\n').map(JSON.parse);
  const out = [];
  for (const f of fx.fatos) {
    const r = recorded.find(x => x.id === f.id);
    if (!r) { out.push({ id: f.id, classificacao: 'fato não encontrado', chamadas: [] }); continue; }
    // The innermost declaration named symbol_name whose span holds the declaration line.
    const owner = Object.keys(G.g.nodes).filter(id => G.g.nodes[id].category === 'symbol' && (G.g.nodes[id].label === r.symbol_name || G.g.nodes[id].label.endsWith('.' + r.symbol_name)))
      .filter(id => G.declared[id]?.path === r.path && G.declared[id].start_line <= r.declaration_line && G.declared[id].end_line >= r.declaration_line)
      .sort((a, b) => (G.declared[a].end_line - G.declared[a].start_line) - (G.declared[b].end_line - G.declared[b].start_line))[0];
    const all = f.chamadas.length && owner ? await callsOf(api, key, run, G, owner) : [];
    const calls = f.chamadas.map(c => {
      const found = all.filter(x => x.line === c.linha && matches(x.text, c.texto));
      const cls = found.length ? found.map(a => classifyFactCall(a, c.verdade)).sort((x, y) => WORST.indexOf(y) - WORST.indexOf(x))[0] : 'ocorrência não encontrada';
      return { linha: c.linha, texto: c.texto, classificacao: cls, resposta: found.map(a => `${a.resolution} (${a.candidates})`).join(', ') };
    });
    const worst = calls.some(c => c.classificacao === 'ocorrência não encontrada') ? 'ocorrência não encontrada'
      : calls.map(c => c.classificacao).sort((x, y) => WORST.indexOf(y) - WORST.indexOf(x))[0] ?? 'nao_depende';
    out.push({ id: f.id, classificacao: worst, chamadas: calls });
  }
  return out;
}

function hashRuns(root, key, runs) {
  return Object.fromEntries(runs.map(r => {
    const lines = [];
    const walk = d => { for (const e of fs.readdirSync(d, { withFileTypes: true })) { const f = path.join(d, e.name); if (e.isDirectory()) walk(f); else lines.push(path.relative(root, f) + ' ' + crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex')); } };
    walk(path.join(root, '.studio', 'projects', key, 'runs', r));
    return [r, crypto.createHash('sha256').update(lines.sort().join('\n')).digest('hex')];
  }));
}

const count = (rows, field) => { const c = {}; for (const r of rows) c[r[field]] = (c[r[field]] ?? 0) + 1; return c; };

/** Weighted estimate: each stratum's sample proportions times its size in the population. */
function estimate(sample, pop) {
  const est = Object.fromEntries(CATS.map(c => [c, 0]));
  for (const [s, n] of Object.entries(pop.por_estrato)) {
    const rows = sample.filter(r => r.estrato === s && CATS.includes(r.categoria));
    if (!rows.length) continue;
    for (const c of CATS) est[c] += n * rows.filter(r => r.categoria === c).length / rows.length;
  }
  return Object.fromEntries(Object.entries(est).map(([k, v]) => [k, Math.round(v)]));
}

/** The sample's counts over the entries the baseline also has: a stratum added later is not
 *  compared with a baseline that did not measure it. */
function comparable(now, base, lang) {
  const ids = new Set(base.projetos[lang].amostra.map(s => s.id));
  return count(now.projetos[lang].amostra.filter(s => ids.has(s.id)), 'categoria');
}

function gate(now, base) {
  const fail = [];
  for (const lang of Object.keys(LANGS)) {
    if (!now.projetos?.[lang] || !base.projetos?.[lang]) continue;
    const a = comparable(now, base, lang), b = base.projetos[lang].amostra_contagem;
    if ((a.link_falso ?? 0) > (b.link_falso ?? 0)) fail.push(`${LANGS[lang]}: link_falso subiu de ${b.link_falso ?? 0} para ${a.link_falso}`);
    if ((a.correta ?? 0) < (b.correta ?? 0)) fail.push(`${LANGS[lang]}: corretas caíram de ${b.correta ?? 0} para ${a.correta ?? 0}`);
    if (a['ocorrência não encontrada']) fail.push(`${LANGS[lang]}: ${a['ocorrência não encontrada']} chamada(s) da amostra não encontrada(s) no run novo`);
  }
  for (const e of now.editions ?? []) if (!e.ok) fail.push(`edition antiga alterada ou não verificada: ${e.projeto}/${e.run} (${e.detalhe})`);
  if (now.studio_original && !now.studio_original.inalterado) fail.push('o .studio original mudou');
  return fail;
}

function report(r, base, fail) {
  const L1 = [];
  const delta = (a, b) => b === undefined ? '' : a === b ? '' : ` (${a > b ? '+' : ''}${a - b})`;
  L1.push(`# Portão de precisão da Fase 2`, '', `Binário: ${typeof r.binario === 'string' ? r.binario : L.binaryLine(r.binario)}. Gerado em ${r.quando}.`, '');
  if (r.projetos) {
    L1.push('## Amostra (contagens reais)', '', '| | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato | Não encontradas |', '|---|---|---|---|---|---|---|');
    const added = {};
    for (const [lang, p] of Object.entries(r.projetos)) {
      const b = base?.projetos?.[lang]?.amostra_contagem, n = base?.projetos?.[lang] ? base.projetos[lang].amostra.length : p.amostra.length;
      const c = base?.projetos?.[lang] ? comparable(r, base, lang) : p.amostra_contagem;
      if (n < p.amostra.length) added[lang] = p;
      L1.push(`| ${LANGS[lang]} (${n}) | ${c.correta ?? 0}${delta(c.correta ?? 0, b && (b.correta ?? 0))} | ${c.link_falso ?? 0}${delta(c.link_falso ?? 0, b && (b.link_falso ?? 0))} | ${c.candidatos_falsos ?? 0}${delta(c.candidatos_falsos ?? 0, b && (b.candidatos_falsos ?? 0))} | ${c.ambigua ?? 0}${delta(c.ambigua ?? 0, b && (b.ambigua ?? 0))} | ${c.sem_candidato ?? 0}${delta(c.sem_candidato ?? 0, b && (b.sem_candidato ?? 0))} | ${c['ocorrência não encontrada'] ?? 0} |`);
    }
    if (Object.keys(added).length) {
      L1.push('', 'A tabela acima compara só as chamadas que a linha de base tem. A amostra inteira, com os estratos novos:', '', '| | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato | Não encontradas |', '|---|---|---|---|---|---|---|');
      for (const [lang, p] of Object.entries(added)) { const c = p.amostra_contagem; L1.push(`| ${LANGS[lang]} (${p.amostra.length}) | ${c.correta ?? 0} | ${c.link_falso ?? 0} | ${c.candidatos_falsos ?? 0} | ${c.ambigua ?? 0} | ${c.sem_candidato ?? 0} | ${c['ocorrência não encontrada'] ?? 0} |`); }
      for (const [lang, p] of Object.entries(added)) {
        const ids = new Set(base.projetos[lang].amostra.map(s => s.id));
        const byStratum = {};
        for (const s of p.amostra.filter(s => !ids.has(s.id))) (byStratum[s.estrato] ??= []).push(s.categoria);
        for (const [st, cats] of Object.entries(byStratum)) L1.push(`- ${LANGS[lang]}, estrato novo ${st} (${cats.length}): ${Object.entries(count(cats.map(c => ({ c })), 'c')).map(([k, v]) => `${k} ${v}`).join(', ')}`);
      }
    }
    L1.push('', 'Parte C (as 30 originais, dentro da amostra):', '');
    for (const [lang, p] of Object.entries(r.projetos)) L1.push(`- ${LANGS[lang]}: ${Object.entries(p.parte_c_contagem).map(([k, v]) => `${k} ${v}`).join(', ')}`);
    L1.push('', '## Estimativa para a população (ponderada pelo peso de cada estrato; é **estimativa**, não contagem)', '', '| | População | Corretas | link_falso | candidatos_falsos | Ambíguas | Sem candidato |', '|---|---|---|---|---|---|---|');
    for (const [lang, p] of Object.entries(r.projetos)) { const e = p.estimativa_populacao; L1.push(`| ${LANGS[lang]} | ${p.populacao.total} | ~${e.correta} | ~${e.link_falso} | ~${e.candidatos_falsos} | ~${e.ambigua} | ~${e.sem_candidato} |`); }
    L1.push('', '## População de chamadas obj.metodo() (contagem real)', '', '| | Total | Sem candidato | Única | Homônimo | Acima do teto | Receptor | Outras | Método → função solta |', '|---|---|---|---|---|---|---|---|---|');
    for (const [lang, p] of Object.entries(r.projetos)) {
      const q = p.populacao.por_resolution, known = ['no_candidate_in_project', 'unique', 'homonym', 'above_candidate_limit', 'receiver'];
      const other = Object.entries(q).filter(([k]) => !known.includes(k)).map(([k, v]) => `${k} ${v}`).join(', ') || '—';
      const b = base?.projetos?.[lang]?.populacao;
      L1.push(`| ${LANGS[lang]} | ${p.populacao.total} | ${q.no_candidate_in_project ?? 0} | ${q.unique ?? 0} | ${q.homonym ?? 0} | ${q.above_candidate_limit ?? 0} | ${q.receiver ?? 0} | ${other} | ${p.populacao.metodo_para_funcao}${delta(p.populacao.metodo_para_funcao, b?.metodo_para_funcao)} |`);
    }
    if (base?.projetos) {
      const changed = [];
      for (const [lang, p] of Object.entries(r.projetos)) for (const s of p.amostra) {
        const old = base.projetos[lang]?.amostra.find(x => x.id === s.id);
        if (old && old.categoria !== s.categoria) changed.push(`- ${s.id}: ${old.categoria} → ${s.categoria} (${s.resposta})`);
      }
      L1.push('', '## Chamadas da amostra que mudaram de categoria', '', ...(changed.length ? changed : ['Nenhuma.']));
    }
  }
  // DEC-40: every call of the sample by the rule before it, and the declaration that runs it.
  if (r.projetos) {
    const differ = [], base40 = [];
    let all = 0;
    for (const [lang, p] of Object.entries(r.projetos)) for (const s of p.amostra.filter(s => s.categoria_regra_anterior)) {
      all++;
      const inBase = !base?.projetos?.[lang] || base.projetos[lang].amostra.some(x => x.id === s.id);
      if (inBase) base40.push(s.id);
      if (s.categoria !== s.categoria_regra_anterior) differ.push(`- ${s.id}${inBase ? ' (na linha de base)' : ''}: ${s.categoria_regra_anterior} pela regra anterior → ${s.categoria} pela DEC-40 (${s.resposta})`);
    }
    L1.push('', '## Regra da DEC-40 contra a anterior', '', `Chamadas classificadas pelas duas regras: ${all}, das quais ${base40.length} estão na linha de base. Com veredito diferente:`, '', ...(differ.length ? differ : ['Nenhuma.']));
    const ex = Object.values(r.projetos).flatMap(p => p.executores ?? []);
    if (ex.length) {
      L1.push('', '## Executor (DEC-40; registrado, não reprova)', '', '| Id | Executor | Dono no grafo | O /api/calls do executor mostra a chamada |', '|---|---|---|---|');
      for (const e of ex) L1.push(`| ${e.id} | ${e.executor} | ${e.dono_no_grafo ?? '—'} | ${e.mostra ? 'sim' : 'não'} |`);
    }
  }
  L1.push('', '## Níveis N0–N8 (projetos sintéticos)', '', '| Caso | Atual registrado | Agora | Alvo da Fase 2 | Passo | Estado |', '|---|---|---|---|---|---|');
  for (const n of r.niveis) {
    const fmt = a => a ? `${a.resolution} (${a.candidates})` : '—';
    L1.push(`| ${n.id} | ${fmt(n.atual_registrado)} | ${fmt(n.agora)} | ${n.alvo.alvo}${n.alvo.relacao ? ` [${n.alvo.relacao}]` : ` [${n.alvo.resolution}]`} | ${n.alvo.passo} | ${n.estado} |`);
  }
  if (r.fatos) {
    const c = count(r.fatos, 'classificacao'), b = base?.fatos && count(base.fatos, 'classificacao');
    L1.push('', '## Auditoria dos fatos F1–F26 (Parte E)', '', '| Não depende | Correto | Suspeito | Errado | Não encontrado |', '|---|---|---|---|---|',
      `| ${c.nao_depende ?? 0}${delta(c.nao_depende ?? 0, b && (b.nao_depende ?? 0))} | ${c.correto ?? 0}${delta(c.correto ?? 0, b && (b.correto ?? 0))} | ${c.suspeito ?? 0}${delta(c.suspeito ?? 0, b && (b.suspeito ?? 0))} | ${c.errado ?? 0}${delta(c.errado ?? 0, b && (b.errado ?? 0))} | ${(c['fato não encontrado'] ?? 0) + (c['ocorrência não encontrada'] ?? 0)} |`, '');
    for (const k of ['correto', 'suspeito', 'errado']) L1.push(`- ${k}: ${r.fatos.filter(f => f.classificacao === k).map(f => f.id).join(', ') || '—'}`);
  }
  if (r.editions) {
    L1.push('', '## Editions antigas e .studio original', '');
    for (const e of r.editions) L1.push(`- ${e.projeto}/${e.run}: ${e.ok ? 'verify-graph ok, arquivos iguais' : 'FALHOU: ' + e.detalhe}; grafo do run novo ${e.grafo_novo_igual ? 'igual' : 'diferente'} ao antigo`);
    L1.push(`- .studio original: ${r.studio_original.inalterado ? 'inalterado' : 'MUDOU'} (${Object.entries(r.studio_original.depois).map(([k, v]) => `${k}: ${v.files} arquivos, ${v.sha256.slice(0, 12)}…`).join('; ')})`);
  }
  L1.push('', '## Regra do portão', '', 'Reprova se, em Python ou em Rust: link_falso aumentar; corretas diminuírem; uma chamada da amostra sumir do run novo; uma edition antiga falhar no verify-graph ou mudar; ou o .studio original mudar. candidatos_falsos, ambíguas, a população, os níveis e a auditoria dos fatos são registrados com a variação. Uma exceção só vale com justificativa caso a caso e aprovação do usuário.', '');
  if (base) L1.push(fail.length ? `**REPROVADO**\n\n${fail.map(f => `- ${f}`).join('\n')}` : '**APROVADO** contra a linha de base.');
  else L1.push('Sem `--base`: só medição, sem decisão.');
  return L1.join('\n') + '\n';
}

(async () => {
  const opt = L.parseArgs(process.argv.slice(2));
  const bin = path.resolve(opt.bin ?? path.join(L.REPO, 'target/release'));
  const exe = n => path.join(bin, process.platform === 'win32' ? n + '.exe' : n);
  // Built first when it is this repository's target/release; the hash of what is measured goes in the report.
  const result = { binario: L.prepareBinaries(bin), quando: new Date().toISOString() };
  // The lock (step 2.4, Q1): the measure must come from the same compiled sources as an earlier one
  // (the Parada 2's), whatever the binary's hash.
  if (typeof opt.fontes === 'string' && result.binario.fontes?.sha256 !== opt.fontes) {
    console.error(`recusado: as fontes compiladas (${result.binario.fontes?.sha256 ?? 'desconhecidas'}) não são as esperadas (${opt.fontes})`);
    process.exit(2);
  }
  if (opt['so-sinteticos']) {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'librarian-precisao-'));
    fs.mkdirSync(path.join(root, '.studio', 'projects'), { recursive: true });
    const s = await L.startStudio(bin, root);
    try { result.niveis = await levels(s.api, root); } finally { s.stop(); }
  } else {
    if (!opt.studio) throw new Error('uso: --studio <pasta .studio> [--bin <dir>] [--saida <dir>] [--base <json>]  (ou --so-sinteticos)');
    const studio = path.resolve(opt.studio);
    const refs = Object.fromEntries(Object.entries(L.DEFAULTS).map(([lang, s]) => [lang, s.split('/')]));
    // The run generated now must come from the same source as the reference run.
    for (const [lang, [key, run]] of Object.entries(refs)) {
      const rec = readJson(path.join(studio, 'projects', key, 'runs', run, 'run.json'));
      const head = L.gitHead(rec.root);
      if (head !== rec.git.head) throw new Error(`${key}: o projeto em ${rec.root} está em ${head}, e o run de referência em ${rec.git.head}`);
    }
    const keys = Object.values(refs).map(([k]) => k);
    await L.withStudio({ studio, bin, keys }, async ({ api, root, before }) => {
      const oldRuns = Object.fromEntries(keys.map(k => [k, fs.readdirSync(path.join(root, '.studio', 'projects', k, 'runs'))]));
      const hashesBefore = Object.fromEntries(keys.map(k => [k, hashRuns(root, k, oldRuns[k])]));
      result.projetos = {}; result.editions = [];
      for (const [lang, [key, run]] of Object.entries(refs)) {
        const fresh = await api('POST', '/api/generate', { key });
        const old = readJson(path.join(root, '.studio', 'projects', key, 'runs', run, 'run.json'));
        process.stderr.write(`${key}: run novo ${fresh.id}; medindo...\n`);
        const { pop, sample, executores } = await project(api, root, lang, key, fresh.id);
        result.projetos[lang] = { run_novo: fresh.id, grafo_novo_igual_ao_de_referencia: fresh.graph_sha256 === old.graph_sha256, populacao: pop,
          amostra_contagem: count(sample, 'categoria'), parte_c_contagem: count(sample.filter(s => s.parte_c), 'categoria'), estimativa_populacao: estimate(sample, pop), amostra: sample, executores };
        if (lang === 'hc') result.fatos = await facts(api, root, key, fresh.id);
        for (const r of oldRuns[key]) {
          const dir = path.join(root, '.studio', 'projects', key, 'runs', r);
          let ok = true, detalhe = '';
          try { execFileSync(exe('librarian_ingest'), ['verify-graph', path.join(dir, 'edition'), path.join(dir, 'graph-v2.json')], { encoding: 'utf8' }); }
          catch (e) { ok = false; detalhe = `verify-graph: ${(e.stdout || '') + (e.stderr || e.message)}`.trim(); }
          result.editions.push({ projeto: key, run: r, ok, detalhe, grafo_novo_igual: fresh.graph_sha256 === readJson(path.join(dir, 'run.json')).graph_sha256 });
        }
      }
      process.stderr.write('níveis N0–N8...\n');
      result.niveis = await levels(api, root);
      const hashesAfter = Object.fromEntries(keys.map(k => [k, hashRuns(root, k, oldRuns[k])]));
      for (const e of result.editions) if (hashesBefore[e.projeto][e.run] !== hashesAfter[e.projeto][e.run]) { e.ok = false; e.detalhe += ' arquivos do run antigo mudaram durante a medição'; }
      result.studio_original = { antes: before };
    });
    // withStudio already refused a changed original; record what it found.
    result.studio_original.depois = Object.fromEntries(keys.map(k => [k, L.hashProject(studio, k)]));
    result.studio_original.inalterado = keys.every(k => result.studio_original.depois[k].sha256 === result.studio_original.antes[k].sha256);
  }
  const base = opt.base ? readJson(path.resolve(opt.base)) : null;
  const fail = base ? gate(result, base) : [];
  const text = report(result, base, fail);
  const out = path.resolve(opt.saida ?? path.join(os.tmpdir(), 'librarian-precisao-relatorio'));
  fs.mkdirSync(out, { recursive: true });
  fs.writeFileSync(path.join(out, 'resultado.json'), JSON.stringify(result, null, 1) + '\n');
  fs.writeFileSync(path.join(out, 'relatorio.md'), text);
  if (opt['gravar-base']) {
    const { quando, binario, ...stable } = result;
    fs.writeFileSync(path.resolve(opt['gravar-base']), JSON.stringify(stable, null, 1) + '\n');
  }
  process.stdout.write(text);
  process.stderr.write(`resultado e relatório em ${out}\n`);
  process.exit(base && fail.length ? 1 : 0);
})().catch(e => { console.error(e.message); process.exit(2); });

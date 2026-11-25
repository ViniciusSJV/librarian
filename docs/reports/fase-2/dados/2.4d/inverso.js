// Passo 2.4d, Parada 1: o used_by pelo inverso da regra, medido (simulador; não é código de produção).
//   node inverso.js <f-depois (dump do 2.4d-0)> <pasta com inv-<projeto>-novo.json, do inverso.rs>
'use strict';
const fs = require('fs'), path = require('path');
const { load, isTestPath } = require('../2.4/load.js');
const [dump, sim] = process.argv.slice(2);
const KINDS = new Set(['call_observed', 'method_call_observed']);
const inc = (o, k, n = 1) => { o[k] = (o[k] ?? 0) + n; };
// O último nome antes do parêntese que fecha o texto da chamada.
function callName(t) {
  let k = 0;
  for (let i = t.length - 1; i >= 0; i--) {
    if (t[i] === ')') k++;
    else if (t[i] === '(' && --k === 0) return (/([A-Za-z_]\w*)\s*(?:::\s*<.*>)?\s*$/.exec(t.slice(0, i)) ?? [])[1] ?? t.slice(0, i);
  }
  return t;
}

for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  const P = load(dump, key), { N, d, g, show, last, declared } = P;
  const inv = JSON.parse(fs.readFileSync(path.join(sim, `inv-${key}-novo.json`), 'utf8'));
  const lang = key[0] === 'h' ? 'python' : 'rust';
  console.log(`\n######## ${key}`);
  const cat = {};
  for (const e of g.edges) if (e.kind === 'observes' && KINDS.has(N[e.to].kind)) inc(cat, `${N[e.from].category} ${N[e.from].kind}`);
  console.log('chamadas por quem observa', cat);

  // 0. O simulador contra o dump do /api/calls.
  const simL = new Set(), dumpL = new Set(), links = [];
  for (const c of inv) for (const t of c.to) simL.add([c.occ, c.from, t, c.res, c.rel, c.runs ?? ''].join('|'));
  for (const [id, r] of Object.entries(d.res)) for (const [occ, from, to, res, rel, v, runs] of r.links) {
    if (from !== id) continue;
    links.push({ occ, from, to, res, rel, via: v, runs });
    if (!v) dumpL.add([occ, from, to, res, rel, runs ?? ''].join('|'));
  }
  const owner = {}, uses = {}; // uses: o used_by de hoje, sem teto de nós
  for (const e of g.edges) if (e.kind === 'observes') (owner[e.to] ??= new Set()).add(e.from);
  for (const e of g.edges) if (e.kind === 'name_candidate') for (const o of owner[e.from] ?? []) (uses[e.to] ??= []).push({ occ: e.from, from: o, kind: N[e.from].kind });
  const soSim = [...simL].filter(x => !dumpL.has(x));
  console.log('0. simulador x dump: só no simulador', soSim.length, '; só no dump', [...dumpL].filter(x => !simL.has(x)).length, '; links do dump', links.length, 'dos quais via', links.filter(l => l.via).length);
  console.log('   ocorrências com mais de um observador', Object.values(owner).filter(s => s.size > 1).length);
  if (soSim.length) console.log('   exemplo só no simulador: observador', N[soSim[0].split('|')[1]].category);

  // 3. Invariante hoje.
  const occIn = {}, fromIn = {};
  for (const [b, u] of Object.entries(uses)) { occIn[b] = new Set(u.map(x => x.occ)); fromIn[b] = new Set(u.map(x => x.from)); }
  const capOcc = Object.fromEntries(Object.entries(d.res).map(([id, r]) => [id, new Set(r.used_by.map(x => x[0]))]));
  const I = {};
  const count = (k, to, l) => {
    const c = I[k] ??= { links: 0, faltaOcorrencia: 0, faltaNaAPI: 0, faltaA: 0 };
    c.links++;
    if (!occIn[to]?.has(l.occ)) c.faltaOcorrencia++;
    if (!capOcc[to]?.has(l.occ)) c.faltaNaAPI++;
    if (!fromIn[to]?.has(l.from)) c.faltaA++;
  };
  for (const l of links) {
    const k = l.res === 'homonym' ? 'possível' : `certo ${l.rel}`;
    count(l.via ? `${k} via` : k, l.to, l);
    if (l.runs) count('runs (__init__)', l.runs, l);
  }
  console.log('3. invariante hoje (faltaOcorrencia e faltaA: sem teto de nós; faltaNaAPI: com o teto de 120)');
  for (const [k, c] of Object.entries(I)) console.log('  ', k, JSON.stringify(c));
  const uni = links.filter(l => !l.via && l.res !== 'homonym'), fal = uni.filter(l => !occIn[l.to]?.has(l.occ)), falCap = uni.filter(l => !capOcc[l.to].has(l.occ));
  const por = {}, porCap = {};
  for (const l of fal) inc(por, l.res);
  for (const l of falCap) if (!fal.includes(l)) inc(porCap, l.res);
  console.log('   links únicos sem via', uni.length, '; faltam no grafo', fal.length, por, '; faltam só pelo teto de 120', falCap.length - fal.length, porCap, '; pela API', falCap.length);

  // 2. Os três tipos, por símbolo.
  const T = {};
  const at = b => T[b] ??= { certo: [], runs: [], tipo: [], homonimo: [], nome: [], descartado: [] };
  const byOcc = new Map(inv.map(c => [c.occ, c]));
  const viaOf = Object.fromEntries(links.filter(l => l.via).map(l => [l.occ, l.from]));
  for (const c of inv) {
    const single = c.to.length === 1 && c.res !== 'homonym';
    for (const t of c.to) (single ? at(t).certo : c.basis === 'type' ? at(t).tipo : at(t).homonimo).push(c);
    if (c.runs) at(c.runs).runs.push(c);
  }
  for (const [b, u] of Object.entries(uses)) for (const x of u) {
    const c = byOcc.get(x.occ);
    if (!c) at(b).nome.push(x);
    else if (!c.to.includes(b) && c.runs !== b) at(b).descartado.push(c);
  }
  const acima = {}, acimaOutras = {};
  for (const c of inv) if (c.res === 'above_candidate_limit') inc(acima, callName(N[c.occ].label));
  for (const id in N) if (N[id].category === 'occurrence' && N[id].ambiguous_candidates && !byOcc.has(id)) inc(acimaOutras, N[id].kind);
  const syms = Object.keys(N).filter(id => N[id].category === 'symbol');
  const cnt = {};
  for (const e of g.edges) if (e.kind === 'name_candidate') inc(cnt, e.from);
  const mud = { simbolos: syms.length, usoHoje: 0, usoDepois: 0, mudamOcorrencias: 0, ganhamCerto: 0, ganhamPossivel: 0, temDescartado: 0, deZeroAAlgum: 0, deAlgumAZero: 0 };
  const tot = { hoje: 0, certo: 0, certoNovo: 0, runs: 0, possivelTipo: 0, possivelHomonimo: 0, possivelNovo: 0, soPorNome: 0, descartado: 0, cheiaHoje: 0, cheiaDescartada: 0, cheiaPossivel: 0, tracejadaCerta: 0 };
  const rows = {};
  for (const b of syms) {
    const t = at(b), h = occIn[b] ?? new Set();
    const cert = new Set([...t.certo, ...t.runs].map(c => c.occ)), poss = new Set([...t.tipo, ...t.homonimo].map(c => c.occ));
    const nome = new Set(t.nome.map(x => x.occ)), desc = new Set(t.descartado.map(c => c.occ));
    const gc = [...cert].filter(o => !h.has(o)).length, gp = [...poss].filter(o => !h.has(o)).length;
    tot.hoje += h.size; tot.certo += t.certo.length; tot.runs += t.runs.length; tot.possivelTipo += t.tipo.length; tot.possivelHomonimo += t.homonimo.length;
    tot.soPorNome += nome.size; tot.descartado += desc.size; tot.certoNovo += gc; tot.possivelNovo += gp;
    for (const o of h) if (cnt[o] === 1) { tot.cheiaHoje++; if (desc.has(o)) tot.cheiaDescartada++; if (poss.has(o)) tot.cheiaPossivel++; }
    for (const o of cert) if (h.has(o) && cnt[o] > 1) tot.tracejadaCerta++;
    const depois = cert.size + poss.size + nome.size;
    if (h.size) mud.usoHoje++;
    if (depois) mud.usoDepois++;
    if (gc || gp || desc.size) mud.mudamOcorrencias++;
    if (gc) mud.ganhamCerto++;
    if (gp) mud.ganhamPossivel++;
    if (desc.size) mud.temDescartado++;
    if (!h.size && depois) mud.deZeroAAlgum++;
    if (h.size && !depois) mud.deAlgumAZero++;
    rows[b] = { nome: show(b), kind: N[b].kind, hoje: h.size, certo: t.certo.length, runs: t.runs.length, tipo: t.tipo.length, homonimo: t.homonimo.length, porNome: nome.size, descartado: desc.size, novoCerto: gc, novoPoss: gp };
  }
  console.log('5. símbolos', mud);
  console.log('5. ocorrências somadas por símbolo', tot);
  const rel = {}, ok = {}, dk = {}, pk = {};
  for (const c of inv) if (c.to.length === 1 && c.res !== 'homonym') inc(rel, `${c.rel} ${N[c.to[0]].kind}`);
  for (const b of syms) {
    for (const x of T[b].nome) inc(ok, x.kind);
    for (const c of T[b].descartado) inc(dk, c.to.length && c.res === 'homonym' ? 'ambígua, B não admitido' : c.to.length ? `fecha em outro (${c.res})` : c.res);
  }
  for (const c of inv) if (c.res === 'homonym') inc(pk, `${c.basis} ${c.to.length === 1 ? '1 candidato' : 'vários'}${c.rel === 'decorates' ? ' decorador' : ''}`);
  console.log('   links certos por relação e alvo', rel);
  console.log('   chamadas ambíguas', pk);
  console.log('   só por nome, por ocorrência', ok);
  console.log('   descartados, pela resposta da chamada', dk);
  console.log('   acima do teto, sem decisão: chamadas', Object.values(acima).reduce((a, b) => a + b, 0), Object.entries(acima).sort((a, b) => b[1] - a[1]).slice(0, 12), '; outras ocorrências', acimaOutras);
  const R = Object.values(rows);
  const line = r => `${r.nome}: hoje ${r.hoje} -> certo ${r.certo}${r.runs ? ` + instanciação ${r.runs}` : ''}, possível ${r.tipo}+${r.homonimo}, nome ${r.porNome}, descartado ${r.descartado} (certos novos ${r.novoCerto})`;
  const top = f => R.slice().sort((a, b) => f(b) - f(a)).slice(0, 8).filter(f).map(line).join('\n     ');
  console.log('   maiores ganhos certos\n     ' + top(r => r.novoCerto));
  console.log('   maiores descartes\n     ' + top(r => r.descartado));
  console.log('   maiores possíveis\n     ' + top(r => r.tipo + r.homonimo));
  console.log('   maior lista: certo', Math.max(...R.map(r => r.certo + r.runs)), '; possível', Math.max(...R.map(r => r.tipo + r.homonimo)), '; nome', Math.max(...R.map(r => r.porNome)));
  let over = 0;
  for (const b of syms) { const t = T[b]; if (new Set([...t.certo, ...t.runs, ...t.tipo, ...t.homonimo, ...t.nome].map(c => c.from)).size > 119) over++; }
  console.log('   símbolos com mais de 119 usuários diretos: hoje', syms.filter(b => (fromIn[b]?.size ?? 0) > 119).length, '; depois', over);

  // Percurso para trás. O possível, o só por nome e o decorador aparecem e não são seguidos.
  const up = (seed, depth, next) => {
    const seen = new Set([seed]); let fr = [seed];
    for (let k = 0; k < depth && fr.length; k++) { const nx = []; for (const b of fr) for (const [f, go] of next(b)) if (!seen.has(f)) { seen.add(f); if (go) nx.push(f); } fr = nx; }
    return seen.size;
  };
  const who = c => viaOf[c.occ] ?? c.from;
  const hoje = b => (uses[b] ?? []).map(x => [x.from, true]);
  const certo = b => T[b] ? [...T[b].certo, ...T[b].runs].map(c => [who(c), c.rel !== 'decorates']) : [];
  const tudo = b => T[b] ? [...certo(b), ...[...T[b].tipo, ...T[b].homonimo].map(c => [who(c), false]), ...T[b].nome.map(x => [x.from, false])] : [];
  const tudoSegueNome = b => T[b] ? [...certo(b), ...[...T[b].tipo, ...T[b].homonimo].map(c => [who(c), false]), ...T[b].nome.map(x => [x.from, true])] : [];
  for (const depth of [2, 3, 6]) for (const [nome, f] of [['hoje', hoje], ['os três, segue o certo', tudo], ['os três, segue o certo e o nome', tudoSegueNome], ['só certo', certo]]) {
    const v = syms.map(b => up(b, depth, f)).sort((a, b) => a - b);
    console.log(`   percurso para trás, profundidade ${depth}, ${nome}: média ${(v.reduce((a, b) => a + b) / v.length).toFixed(1)}, p99 ${v[Math.floor(v.length * .99)]}, máx ${v.at(-1)}, acima de 120 nós: ${v.filter(x => x > 120).length}`);
  }
  const want = lang === 'python'
    ? ['Webhook.is_noop (', 'Transport.is_noop (', 'HttpTransport.raise_for_response (', 'Discord.fix_asterisks (', 'Slackalike.fix_asterisks (', 'RocketChat.fix_asterisks (', 'TokenBucket.authorize (', 'authorize (hc/api/decorators', 'Telegram.send (', 'Check.ping (', 'Channel.notify (', 'Webhook.notify (', 'cors (', 'EmailThread.__init__', 'BaseTestCase.setUp (', 'Check.get_status (', 'FakeCurl', 'HttpTransport.post (', 'Ping.get_body ']
    : ['Matrix<4>::identity', 'impl Groups::len', 'Shape::Sphere', 'impl Tuple::dot', 'Patterns::from', 'Camera ('];
  const f = c => `${P.observer[c.occ].ev.path}:${P.observer[c.occ].ev.start_line} ${N[c.occ].label.replace(/\s+/g, ' ').slice(0, 60)} [${c.res ?? c.kind}${c.rel && c.rel !== 'calls' ? ' ' + c.rel : ''}${c.rule ? ' ' + c.rule : ''}${c.to && c.to.length > 1 ? ' entre ' + c.to.length : ''}] de ${N[c.from].label}`;
  for (const w of want) for (const b of syms) if (show(b).includes(w)) {
    const t = T[b];
    console.log('   EX ' + line(rows[b]));
    for (const [k, l] of [['certo', t.certo], ['instanciação', t.runs], ['possível (tipo)', t.tipo], ['possível (homônimo)', t.homonimo], ['nome', t.nome], ['descartado', t.descartado]]) l.slice(0, 4).forEach(c => console.log(`        ${k}: ${f(c)}`));
    const nm = last(N[b].label);
    if (acima[nm]) console.log(`        lacuna: ${acima[nm]} chamadas com este nome acima do teto, sem decisão`);
  }
  let tp = 0;
  for (const b of syms) if (isTestPath(declared[b]?.path)) for (const c of T[b].certo) if (!isTestPath(declared[c.from]?.path)) { tp++; if (tp < 6) console.log('   teste usado por produção:', show(b), '<-', f(c)); }
  console.log('   uso certo de B (teste) por A (produção):', tp);

}


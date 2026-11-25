// Passo 2.4d, Parada 2: (1) /api/calls antes x depois, byte a byte; (2) a invariante, nas duas direções, todos os símbolos;
// (3) a previsão do simulador; (4) os sorteios: 12 símbolos (UB-17 a UB-28) e 40 pares do possível por homônimo (PH-1 a PH-40).
//   node confere24d.js <d-antes> <d-depois> [sorteios.json]
'use strict';
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const [antes, depois, out] = process.argv.slice(2);
const J = f => JSON.parse(fs.readFileSync(f, 'utf8')), sha = s => crypto.createHash('sha256').update(s).digest('hex');
const isTest = p => /(^|\/)tests?\/|(^|\/)test_|_test\.|\/test\.py$/.test(p ?? '');
const sorteios = { UB: [], PH: [] };
for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  const a = J(path.join(antes, `${key}.hash.json`)), b = J(path.join(depois, `${key}.hash.json`));
  const ids = Object.keys(b.hash), dif = ids.filter(i => a.hash[i] !== b.hash[i]);
  console.log(`## ${key}`);
  console.log(`1. /api/calls: ${ids.filter(i => i.endsWith('|1')).length} respostas na profundidade 1 e ${ids.filter(i => i.endsWith('|3')).length} na 3; diferentes: ${dif.length}; só num lado: ${Object.keys(a.hash).length - ids.length}; sha256 de todas: ${a.total.slice(0, 16)} ${a.total === b.total ? '=' : '≠'} ${b.total.slice(0, 16)}`);
  const g = J(path.join(depois, `${key}.graph.json`)), N = g.nodes, calls = J(path.join(depois, `${key}.calls.json`));
  const uses = new Map(fs.readFileSync(path.join(depois, `${key}.uses.jsonl`), 'utf8').trim().split('\n').map(l => JSON.parse(l)));
  const decl = {}; for (const e of g.edges) if (e.kind === 'declares') decl[e.to] = e.evidence;
  const show = id => `${N[id].label} (${decl[id].path}:${decl[id].start_line})`;
  // 2. Invariante.
  const inUses = new Map([...uses].map(([s, u]) => [s, new Map(u.links.filter(l => l.to === s).map(l => [l.occurrence, l]))]));
  const ida = [], volta = [], fwd = new Map();
  let nl = 0, nu = 0;
  for (const [s, c] of Object.entries(calls)) {
    const mine = new Map(); fwd.set(s, mine);
    const all = [...c.links, ...c.unresolved.filter(u => u[2] === 'node_limit').map(u => [u[0], u[1], u[3], u[4], null, null, null, null, null])];
    for (const [occ, from, to, res, rel, via, runs, basis] of all) for (const t of [to, runs].filter(Boolean)) {
      nl++;
      const kind = res === 'homonym' ? 'possible' : 'linked', u = inUses.get(t)?.get(occ);
      mine.set(`${occ}|${t}`, kind);
      if (!u || u.use !== kind || (u.from !== s && u.via !== s) || (rel && u.relation !== rel) || (basis && u.basis !== basis) || u.resolution !== res) ida.push([show(s), t, occ, u?.use]);
    }
  }
  for (const [t, u] of uses) for (const l of u.links) {
    if (l.to !== t || l.use === 'name') continue;
    for (const s of [l.from, l.via].filter(Boolean)) { nu++; const m = fwd.get(s); if (m?.get(`${l.occurrence}|${t}`) !== l.use && !(l.instance_of && m?.get(`${l.occurrence}|${l.instance_of}`) === l.use)) volta.push([t, s, l.occurrence]); }
  }
  console.log(`2. invariante: ${nl} links na ida, ${nu} usos na volta; exceções ${ida.length} + ${volta.length}`, ida.slice(0, 3), volta.slice(0, 3));
  // Dois tipos para a mesma ocorrência, ou um uso também à parte.
  let mist = 0; for (const [t, u] of uses) { const seen = new Set(); for (const l of u.links.filter(l => l.to === t)) { if (seen.has(l.occurrence)) mist++; seen.add(l.occurrence); } for (const k of ['above_limit', 'abstract_calls', 'ruled_out']) for (const x of u[k]) if (seen.has(x.occurrence)) mist++; }
  console.log('   ocorrências em dois tipos, ou também à parte:', mist);
  // 3. Previsão.
  const tot = {}; for (const u of uses.values()) for (const k in u.counts) tot[k] = (tot[k] ?? 0) + u.counts[k];
  const runs = [...uses].reduce((n, [t, u]) => n + u.links.filter(l => l.to === t && l.instance_of).length, 0);
  const some = [...uses].filter(([, u]) => u.counts.linked + u.counts.possible_type + u.counts.possible_homonym + u.counts.name);
  console.log('3. totais por tipo (pares)', JSON.stringify(tot), '; roda ao instanciar', runs, '; símbolos com algum uso', some.length, '; com ligado ou possível', some.filter(([, u]) => u.counts.linked + u.counts.possible_type + u.counts.possible_homonym).length, '; cortados no desenho', [...uses.values()].filter(u => u.truncated).length);
  const ab = new Set(); for (const u of uses.values()) for (const x of u.above_limit) ab.add(x.occurrence); console.log('   chamadas acima do teto em alguma lacuna', ab.size);
  if (key[0] === 'r') { const t = [...uses].find(([i]) => N[i].label === 'impl From<Stripe> for Patterns::from')[1], c = {}; for (const x of t.above_limit) c[x.text] = (c[x.text] ?? 0) + 1; console.log('   Patterns::from, lacuna', t.above_limit.length, c); }
  // 4. Sorteios: ordem pelo sha256 de uma semente fixa com o id. Símbolos com algum uso, dos três tipos.
  const order = l => l.sort((x, y) => sha('2.4d|' + x) < sha('2.4d|' + y) ? -1 : 1);
  for (const id of order(some.map(([i]) => i)).slice(0, 6)) sorteios.UB.push({ key, simbolo: show(id), kind: N[id].kind, counts: uses.get(id).counts, usos: uses.get(id).links.filter(l => l.to === id).map(l => `${l.use === 'possible' ? l.basis : l.use}${l.relation && l.relation !== 'calls' ? ' ' + l.relation : ''} ${l.evidence.path}:${l.evidence.start_line} ${l.text.replace(/\s+/g, ' ').slice(0, 60)} [${l.occurrence_kind}] de ${N[l.from].label}`), lacuna: uses.get(id).above_limit.map(x => `${x.evidence.path}:${x.evidence.start_line}`) });
  const pairs = []; for (const [t, u] of uses) for (const l of u.links) if (l.to === t && l.use === 'possible' && l.basis === 'graph_edge') pairs.push(`${l.occurrence}|${t}`);
  // Por chamada, depois um dos corpos: um par sorteado direto mediria quase só self.client.get.
  const byCall = {}; for (const p of pairs) { const i = p.indexOf('|'); (byCall[p.slice(0, i)] ??= []).push(p.slice(i + 1)); }
  console.log('   pares de possível por homônimo', pairs.length, '; chamadas', Object.keys(byCall).length);
  for (const p of order(pairs.slice()).slice(0, 20)) { const i = p.indexOf('|'), o = p.slice(0, i), t = p.slice(i + 1), l = uses.get(t).links.find(l => l.to === t && l.occurrence === o); sorteios.PH.push({ key, chamada: `${l.evidence.path}:${l.evidence.start_line}`, texto: l.text.replace(/\s+/g, ' ').slice(0, 70), de: N[l.from].label, b: show(t), candidatos: l.candidates, teste: isTest(l.evidence.path) }); }
}
if (out) fs.writeFileSync(out, JSON.stringify(sorteios, null, 1) + '\n');

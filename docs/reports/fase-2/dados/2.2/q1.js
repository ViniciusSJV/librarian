// Q1 of step 2.2, on a graph-v2.json of a Python project: (a) calls without a receiver whose owner is in a
// class's scope (the class, a field of it or a method of it, decorators included) and that reach a method of
// that class; (b) functions nested in functions and methods, their kind, and the calls that reach them.
// Usage: node q1.js <graph-v2.json>
'use strict';
const fs = require('fs');
const g = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const obs = {}, par = {}, cand = {};
for (const e of g.edges) {
  if (e.kind === 'observes') obs[e.to] = e.from;
  if (e.kind === 'contains') par[e.to] = e.from;
  if (e.kind === 'name_candidate') (cand[e.from] ??= []).push(e.to);
}
let scope = 0, own = 0;
const nested = {}, nestedIds = new Set();
for (const [id, n] of Object.entries(g.nodes)) {
  if (n.category === 'symbol' && ['function', 'method'].includes(n.kind) && par[id] && ['function', 'method'].includes(g.nodes[par[id]].kind)) {
    nested[`${n.kind} em ${g.nodes[par[id]].kind}`] = (nested[`${n.kind} em ${g.nodes[par[id]].kind}`] ?? 0) + 1;
    nestedIds.add(id);
  }
}
let nestedCalls = 0;
for (const [id, n] of Object.entries(g.nodes)) {
  if (n.kind !== 'call_observed' || n.label.includes('.')) continue;
  const o = obs[id]; if (!o) continue;
  if ((cand[id] ?? []).some(t => nestedIds.has(t))) nestedCalls++;
  const cls = g.nodes[o].kind === 'class' ? o : g.nodes[par[o]]?.kind === 'class' ? par[o] : null;
  if (!cls) continue;
  scope++;
  if ((cand[id] ?? []).some(t => g.nodes[t].kind === 'method' && par[t] === cls)) { own++; console.log('(a)', n.label, '->', g.nodes[o].label); }
}
console.log(`(a) chamadas sem receptor no escopo de uma classe: ${scope}; que alcançam um método da própria classe: ${own}`);
console.log(`(b) funções aninhadas: ${JSON.stringify(nested)}; chamadas sem receptor que as têm como candidata: ${nestedCalls}`);

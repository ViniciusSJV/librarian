// Passo 2.4d-0, Parada 1: tempo do /api/calls e percurso de profundidade 6, antes e com o protótipo, um binário por vez, em
// rodadas alternadas.   node custo24d0.js <bin-antes> <bin-depois>
'use strict';
const path = require('path'), fs = require('fs');
const L = require('../../../../../tools/precisao/lib.js');
const bins = process.argv.slice(2), key = 'healthchecks-25597e41';
const med = xs => xs.slice().sort((a, b) => a - b)[Math.floor(xs.length / 2)];
const cmp = require('./compara24d0.json')[key].mudancas, ocs = new Set(cmp.map(m => m.occ));
(async () => {
  const T = [{}, {}], W = [{}, {}];
  for (let r = 0; r < 4; r++) for (const b of r % 2 ? [1, 0] : [0, 1]) {
    await L.withStudio({ studio: path.join(L.REPO, '.studio'), bin: bins[b], keys: [key] }, async ({ api, root }) => {
      const { id: run } = await api('POST', '/api/generate', { key }), G = L.loadGraph(root, key, run);
      const donos = new Set(G.g.edges.filter(e => e.kind === 'observes' && ocs.has(L.occKey(e.evidence))).map(e => e.from).filter(i => G.g.nodes[i].category === 'symbol'));
      const ids = Object.keys(G.g.nodes).filter(i => G.g.nodes[i].category === 'symbol').sort();
      const amostra = ids.filter((_, i) => i % 24 === 0);
      for (const id of new Set([...donos, ...amostra])) {
        const a = performance.now();
        await api('POST', '/api/calls', { key, run, node: id, depth: 1, chosen: {} });
        (T[b][id] ??= { dono: donos.has(id), label: G.g.nodes[id].label, t: [] }).t.push(performance.now() - a);
      }
      if (r === 0) for (const l of ['telegram_bot', 'verify_signal_number', 'TokenBucket.authorize_telegram', 'Telegram.notify', 'Channel.notify', 'Ping.get_body_bytes', 'Channel.pd', 'PagerDuty.notify', 'Email.notify']) {
        const id = ids.find(i => G.g.nodes[i].label === l);
        if (!id) { W[b][l] = 'não achado'; continue; }
        const c = await api('POST', '/api/calls', { key, run, node: id, depth: 6, max_nodes: 300, chosen: {} });
        W[b][l] = `${(c.nodes ?? []).length} nós, truncado ${c.truncated}`;
      }
    });
  }
  for (const [n, f] of [['donos das 18', x => x.dono], ['1 em cada 24 símbolos', x => !x.dono]]) {
    const s = T.map(t => Object.values(t).filter(f));
    console.log(n, s[0].length, 'símbolos | soma das medianas', s.map(x => x.reduce((a, y) => a + med(y.t), 0).toFixed(0) + ' ms').join(' -> '), '| mediana', s.map(x => med(x.map(y => med(y.t))).toFixed(0)).join(' -> '), '| máximo', s.map(x => Math.max(...x.map(y => med(y.t))).toFixed(0)).join(' -> '));
  }
  for (const l of Object.keys(W[0])) console.log('percurso 6', l, W[0][l], '->', W[1][l]);
})().catch(e => { console.error(e); process.exit(2); });

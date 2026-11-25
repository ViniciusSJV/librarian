// Passo 2.4d, Parada 2: o que não devia mudar, com os dois binários (inventário, envelope e níveis), e o custo do índice sem carga.
//   node outras24d.js <bin antes> <bin depois>
'use strict';
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const keys = ['healthchecks-25597e41', 'renderer-9b78f3a9'], sha = t => crypto.createHash('sha256').update(t).digest('hex').slice(0, 16);
(async () => {
  const out = {};
  for (const [nome, bin] of [['antes', process.argv[2]], ['depois', process.argv[3]]]) {
    const root = L.copyStudio(path.join(L.REPO, '.studio'), keys), s = await L.startStudio(bin, root), o = out[nome] = {};
    try {
      for (const key of keys) {
        const { id: run } = await s.api('POST', '/api/generate', { key });
        const ids = Object.entries(L.loadGraph(root, key, run).g.nodes).filter(([, n]) => n.category === 'symbol' && /function|method/.test(n.kind)).map(([i]) => i).sort();
        o[`${key} inventário`] = sha(JSON.stringify(await s.api('GET', `/api/inventory?key=${key}&run=${run}`)));
        const env = [];
        for (let i = 0; i < ids.length; i += Math.ceil(ids.length / 25)) {
          const e = await s.api('POST', '/api/envelope/suggest', { key, run, nodes: [ids[i]] }).catch(x => ({ erro: String(x).slice(0, 80) }));
          delete e.suggestion; env.push(JSON.stringify(e).replaceAll(run, 'RUN'));
        }
        o[`${key} envelope (${env.length})`] = sha(env.join('\n'));
        if (nome === 'depois') {
          const t = performance.now(); await s.api('GET', `/api/used-by?key=${key}&run=${run}&node=${encodeURIComponent(ids[0])}&depth=1`);
          const a = performance.now(); for (let i = 0; i < 100; i++) await s.api('GET', `/api/used-by?key=${key}&run=${run}&node=${encodeURIComponent(ids[i])}&depth=2`);
          console.log(key, 'primeiro used-by', (a - t).toFixed(0), 'ms (carrega a edition e o grafo e monta o índice); depois', ((performance.now() - a) / 100).toFixed(1), 'ms em média (profundidade 2)');
        }
      }
    } finally { s.stop(); }
  }
  for (const k in out.antes) console.log(k, out.antes[k] === out.depois[k] ? 'igual' : 'DIFERENTE', out.antes[k], out.depois[k]);
  const N = d => JSON.stringify(JSON.parse(fs.readFileSync(path.join(__dirname, d, 'resultado.json'), 'utf8')).niveis);
  console.log('níveis do portão antes x depois:', N('portao-antes') === N('portao-24d') ? 'iguais' : 'DIFERENTES');
  const F = d => JSON.stringify(JSON.parse(fs.readFileSync(path.join(__dirname, d, 'resultado.json'), 'utf8')).fatos);
  console.log('fatos antes x depois:', F('portao-antes') === F('portao-24d') ? 'iguais' : 'DIFERENTES');
})().catch(e => { console.error(e); process.exit(2); });

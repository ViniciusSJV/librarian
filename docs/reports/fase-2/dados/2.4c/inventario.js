// Passo 2.4c, Parada 2: o inventário antes e depois. (1) /api/inventory inteiro; (2) segue.rs, a regra do inventário
// sobre toda chamada do run.   node inventario.js <out> <bin-antes> <segue-antes> <bin-depois> <segue-depois>
'use strict';
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const [out, b0, s0, b1, s1] = process.argv.slice(2);
const keys = ['healthchecks-25597e41', 'renderer-9b78f3a9'];
fs.mkdirSync(out, { recursive: true });
(async () => {
  const inv = {}, seg = {};
  for (const [t, bin, segue] of [['antes', b0, s0], ['depois', b1, s1]]) {
    await L.withStudio({ studio: path.join(L.REPO, '.studio'), bin, keys }, async ({ api, root }) => {
      for (const key of keys) {
        const { id: run } = await api('POST', '/api/generate', { key });
        (inv[key] ??= {})[t] = await api('GET', `/api/inventory?key=${key}&run=${run}`);
        const dir = path.join(root, '.studio', 'projects', key, 'runs', run);
        const ed = fs.existsSync(path.join(dir, 'manifest.json')) ? dir : path.join(dir, fs.readdirSync(dir).find(d => fs.existsSync(path.join(dir, d, 'manifest.json'))));
        (seg[key] ??= {})[t] = execFileSync(segue, [ed], { maxBuffer: 1 << 28, encoding: 'utf8' }).split('\n').filter(Boolean);
      }
    });
  }
  // As diferenças do JSON, por caminho.
  const dif = (a, b, p, o) => {
    if (JSON.stringify(a) === JSON.stringify(b)) return;
    if (a && b && typeof a === 'object' && typeof b === 'object' && Array.isArray(a) === Array.isArray(b)) for (const k of new Set([...Object.keys(a), ...Object.keys(b)])) dif(a[k], b[k], `${p}.${k}`, o);
    else o.push(`${p}: ${JSON.stringify(a)?.slice(0, 200)} -> ${JSON.stringify(b)?.slice(0, 200)}`);
  };
  for (const key of keys) {
    const { antes, depois } = inv[key], o = [];
    dif(antes, depois, '', o);
    const x = depois.inventory ?? depois;
    console.log(`${key}: /api/inventory ${JSON.stringify(depois).length} bytes; operações ${x.operations?.length}, símbolos ${x.symbols?.length}, laços ${x.loops?.length}; diferenças: ${o.length}`);
    o.forEach(d => console.log('   ', d));
    const A = new Map(seg[key].antes.map(l => { const [a, b, t, r] = l.split('\t'); return [a + ' ' + b, [t, r]]; })), B = new Map(seg[key].depois.map(l => { const [a, b, t, r] = l.split('\t'); return [a + ' ' + b, [t, r]]; }));
    const ch = [...A].filter(([k, v]) => B.get(k)?.[1] !== v[1]), novas = [...B.keys()].filter(k => !A.has(k));
    const n = m => [...m.values()].filter(v => v[1].startsWith('segue')).length;
    console.log(`${key}: a regra do inventário sobre ${A.size} chamadas (${seg[key].antes.length} linhas): seguidas ${n(A)} -> ${n(B)}; mudam ${ch.length}; só depois ${novas.length}`);
    ch.forEach(([k, v]) => console.log('   ', k, '|', v[0], '|', v[1], '->', B.get(k)?.[1]));
  }
})().catch(e => { console.error(e); process.exit(2); });

// Passo 2.4c, Parada 2: as mudanças por extenso, as chamadas que não podem mudar, as contagens por resolution e o tempo.
//   node verifica24c.js <dump-antes> <dump-depois>      (dump24c.js)
'use strict';
const fs = require('fs'), path = require('path');
const { load } = require('../2.4/load.js');
const [d0, d1] = process.argv.slice(2);
for (const key of ['healthchecks-25597e41', 'renderer-9b78f3a9']) {
  const A = load(d0, key), B = load(d1, key);
  const ans = (P, c) => c ? `${c.res} ${c.rule ?? ''} [${c.targets.length}] ${c.targets.length < 3 ? c.targets.map(P.show).join(', ') : ''} candidates ${c.cands}` : 'ausente';
  console.log('==', key, 'chamadas', A.calls.size, B.calls.size);
  let n = 0;
  for (const o of new Set([...A.calls.keys(), ...B.calls.keys()])) {
    const x = ans(A, A.calls.get(o)), y = ans(B, B.calls.get(o));
    const full = (P, c) => JSON.stringify(c && [c.res, c.rule, c.rel, c.runs, c.cands, c.targets.map(P.show).sort()]);
    if (full(A, A.calls.get(o)) !== full(B, B.calls.get(o))) { n++; const c = B.calls.get(o); console.log('  MUDA', `${c.path}:${c.line}`, c.text.slice(0, 60), '|', x, '->', y); }
  }
  console.log('  mudanças:', n);
  const cnt = P => { const m = {}; for (const c of P.calls.values()) m[c.res] = (m[c.res] ?? 0) + 1; return m; };
  const x = cnt(A), y = cnt(B);
  console.log('  todas as chamadas:', Object.keys({ ...x, ...y }).map(k => `${k} ${x[k] ?? 0}->${y[k] ?? 0} (${(y[k] ?? 0) - (x[k] ?? 0)})`).join('; '));
  if (key.startsWith('health')) for (const [nome, where, re] of [['form.get_json', 'hc/integrations/call/views.py:26', /^form\.get_json/], ['HC-138', 'hc/api/transports.py:211', /^cls\.request/], ['HC-139', 'hc/integrations/telegram/transport.py:65', /^cls\.post/], ['HC-140', 'hc/api/transports.py:162', /^cls\.raise/], ['HC-142', 'hc/api/models.py:1153', /is_noop/], ['HC-141', 'hc/api/models.py:1173', /notify/]]) {
    const f = P => [...P.calls.values()].find(c => `${c.path}:${c.line}` === where && re.test(c.text));
    console.log('  ', nome, where, '|', ans(A, f(A)), '->', ans(B, f(B)));
  }
  const ma = JSON.parse(fs.readFileSync(path.join(d0, `${key}.ms.json`))), mb = JSON.parse(fs.readFileSync(path.join(d1, `${key}.ms.json`)));
  console.log(`  tempo: total ${ma.total_s.toFixed(0)} -> ${mb.total_s.toFixed(0)} s; só /api/calls ${ma.calls_s.toFixed(0)} -> ${mb.calls_s.toFixed(0)} s`);
  const med = m => { const v = Object.values(m.ms).sort((a, b) => a - b); return v[v.length >> 1].toFixed(0); };
  console.log(`  mediana por símbolo ${med(ma)} -> ${med(mb)} ms`);
  for (const [t, m, P] of [['antes', ma, A], ['depois', mb, B]]) {
    console.log('  os 10 mais lentos,', t);
    Object.entries(m.ms).sort((a, b) => b[1] - a[1]).slice(0, 10).forEach(([id, v]) => console.log('    ', v.toFixed(0).padStart(5), 'ms', P.show(id), '| antes', ma.ms[id]?.toFixed(0), 'depois', mb.ms[id]?.toFixed(0)));
  }
  if (key.startsWith('health')) {
    const lim = 380, la = Object.entries(ma.ms).filter(([, v]) => v > lim).length, lb = Object.entries(mb.ms).filter(([, v]) => v > lim).length;
    const passam = Object.keys(mb.ms).filter(id => mb.ms[id] > lim && ma.ms[id] <= lim).length, saem = Object.keys(mb.ms).filter(id => mb.ms[id] <= lim && ma.ms[id] > lim).length;
    console.log(`  símbolos acima de ${lim} ms (leem as subclasses): ${la} -> ${lb}; passam a pagar ${passam}; deixam ${saem}`);
    const h = m => { const b = {}; Object.values(m.ms).forEach(v => { const k = Math.floor(v / 50) * 50; b[k] = (b[k] ?? 0) + 1; }); return JSON.stringify(b); };
    console.log('  histograma (50 ms) antes', h(ma), '\n                     depois', h(mb));
  }
}

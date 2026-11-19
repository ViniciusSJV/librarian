// Questions 01 to 14 of the Healthchecks, as recorded, with H1 to H3 checked again by the binary under
// test, on a studio over a copy of .studio (the same check as docs/reports/piloto-1b/dados/p1/validate.js).
// Usage: node h1-h3.js [bin]   Nothing is written to the original .studio.
'use strict';
const path = require('path');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const bin = process.argv[2] || path.join(L.REPO, 'target/release');
const key = 'healthchecks-25597e41', run = '20261118-061046';
(async () => {
  let fails = 0;
  await L.withStudio({ studio: path.join(L.REPO, '.studio'), bin, keys: [key] }, async ({ api }) => {
    for (let n = 1; n <= 14; n++) {
      const t = String(n).padStart(2, '0');
      const e = await api('GET', `/api/evidence?key=${key}&run=${run}&turn=${t}`);
      const h = ['h1', 'h2', 'h3'].map(k => e.checks?.[k]?.status ?? e[k]?.status);
      const ok = h.every(s => s === 'passed' || s === 'ok' || s === 'approved');
      if (!ok) fails++;
      console.log(`${t}: H1 ${h[0]} · H2 ${h[1]} · H3 ${h[2]}${ok ? '' : '  <- FALHA'}`);
    }
  });
  console.log(fails ? `${fails} pergunta(s) com H1–H3 reprovado` : '14 de 14 com H1–H3 aprovados; .studio original inalterado');
  process.exitCode = fails ? 1 : 0;
})().catch(e => { console.error(e); process.exit(2); });

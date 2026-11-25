// G3 in the flowchart ("o que chama?"): the calls not shown, by reason. Fixed on 25/11/2026 (user's decision on the old
// batteries): it starts its own studio on the synthetic project, versioned here, in PT and EN. The part over the
// Healthchecks, whose counts were those of 18/11, is retired (crates/librarian-studio/tests/tela/APOSENTADAS.md).
'use strict';
const path = require('path'), fs = require('fs'), os = require('os');
const L = require('../../../../../tools/precisao/lib.js');
const { chromium } = require(L.playwright());
const OUT = process.env.TELA_OUT || path.join(os.tmpdir(), 'tela-g3');
fs.mkdirSync(OUT, { recursive: true });
const HOOK = "window.__t = { state, studio, t, STRINGS, selectNode, openPath }; if (location.hash === '#debug')";
let fails = 0, passes = 0;
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d !== '' ? '  -> ' + String(d).slice(0, 400) : ''}`); if (pass) passes++; else { fails++; process.exitCode = 1; } };

(async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'librarian-g3-')), syn = path.join(root, 'projeto');
  fs.cpSync(path.join(__dirname, 'synthetic'), syn, { recursive: true });
  fs.mkdirSync(path.join(root, '.studio/projects'), { recursive: true });
  fs.cpSync(path.join(L.REPO, 'modelfiles'), path.join(root, 'modelfiles'), { recursive: true });
  fs.mkdirSync(path.join(root, 'crates/librarian-studio/web'), { recursive: true });
  fs.copyFileSync(path.join(L.REPO, 'crates/librarian-studio/web/index.html'), path.join(root, 'crates/librarian-studio/web/index.html'));
  const bin = L.prepareBinaries(path.join(L.REPO, 'target/release'));
  console.log(`binário: ${L.binaryLine(bin)}`);
  const s = await L.startStudio(bin.pasta, root);
  const { key } = await s.api('POST', '/api/project', { root: syn });
  await s.api('POST', '/api/generate', { key });
  const browser = await chromium.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  try {
    for (const lang of ['pt', 'en']) {
      const T = (pt, en) => lang === 'pt' ? pt : en, tag = x => `${lang.toUpperCase()} ${x}`;
      const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale: T('pt-BR', 'en-US') });
      await context.addInitScript(l => { try { localStorage.setItem('librarian.lang', l); } catch { } }, lang);
      const p = await context.newPage(), errors = [];
      p.on('pageerror', e => errors.push(e.message));
      await p.route(u => new URL(u).pathname === '/', async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
      await p.goto(`http://127.0.0.1:${s.port}/`);
      await p.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
      await p.evaluate(r => { document.querySelector('#projectPath').value = r; document.querySelector('#openProject').click(); }, syn);
      await p.waitForFunction(() => __t.studio.run && __t.state.nodes.length && !document.querySelector('#openProject').disabled, null, { timeout: 180000 });
      await p.waitForTimeout(800);
      const answer = p.waitForResponse(r => new URL(r.url()).pathname === '/api/calls');
      await p.evaluate(() => { document.querySelector('#pathDepth').value = '1'; __t.openPath(__t.state.nodes.find(n => n.category === 'symbol' && n.label === 'seed').id, 'calls'); });
      const body = await (await answer).json();
      await p.waitForFunction(() => { const n = document.querySelector('#pathNote').textContent; return n && !/Seguindo|Following/.test(n); }, null, { timeout: 60000 });
      await p.waitForTimeout(500);
      const v = await p.evaluate(() => { const box = document.querySelector('#pathUnresolved'); box.open = true; return { note: document.querySelector('#pathNote').textContent, hidden: box.hidden, summary: box.querySelector('summary').textContent,
        groups: [...box.querySelectorAll('h4')].map(h => ({ title: h.textContent, why: h.nextElementSibling.textContent, items: [...h.nextElementSibling.nextElementSibling.querySelectorAll('li')].map(li => li.textContent) })) }; });
      await p.screenshot({ path: path.join(OUT, `${lang}-node-limit.png`) });
      const limited = v.groups.find(g => g.title.startsWith(T('Omitidas pelo limite', 'Left out by the point limit')));
      // The group titles of the page (path.unresolved.title.*), exactly.
      const none = v.groups.find(g => g.title.startsWith(T('Sem candidato no projeto (', 'No candidate in the project (')));
      const count = r => body.unresolved.filter(u => u.reason === r).length;
      ok(tag('a seção mostra a contagem da resposta'), !v.hidden && v.summary.startsWith(T(`Chamadas não apresentadas: ${body.unresolved.length} `, `Calls not shown: ${body.unresolved.length} `)), v.summary);
      ok(tag('node_limit em grupo próprio, 11 itens, como limite de apresentação'), count('node_limit') === 11 && limited?.items.length === 11 && T(/não ambiguidade/, /not ambiguity/).test(limited.why) && T(/120 pontos/, /120 points/).test(limited.why), JSON.stringify(limited).slice(0, 300));
      // Since step 2.2 (DEC-38, a34059c) the text of `unique` starts with "entre as declarações que a forma da chamada
      // alcança"; until then it was "seguida: o nome só combina". The end, "o nome só combina com esta", is the same.
      ok(tag('cada item diz o alvo e a resolução original'), limited?.items.every(i => T(/seria f\d+ · seguida: .*o nome só combina com esta/, /would be f\d+ · followed: .*matches only this one/).test(i)), limited?.items[0]);
      ok(tag('items[0] em sem candidato'), none?.items.some(i => i.startsWith('items[0]')), JSON.stringify(none)?.slice(0, 200));
      ok(tag('sem candidato não se diz externa'), T(/não diz que a chamada é externa/, /does not say the call is external/).test(none?.why ?? ''), none?.why);
      ok(tag('o aviso de truncado continua na nota'), T(/Parou no teto de 120 pontos/, /Stopped at the cap of 120 points/).test(v.note), v.note);
      ok(tag('nenhum erro de página'), !errors.length, errors.join(' | '));
      await context.close();
    }
  } finally {
    await browser.close(); s.stop();
    // The temporary root (a copy of the synthetic project and the studio's run); on Windows the studio may still hold a
    // file for a moment.
    for (let i = 0; i < 20; i++) { try { fs.rmSync(root, { recursive: true, force: true }); break; } catch { await new Promise(z => setTimeout(z, 250)); } }
    if (fs.existsSync(root)) console.log(`aviso: a pasta temporária ficou: ${root}`);
  }
  console.log(`${passes} passam, ${fails} falham. Capturas em ${OUT}`);
})().catch(e => { console.error(e); process.exit(1); });

// Revisão de nomes na tela real: uma função com duas linhas contadas (acrescentada só na resposta de
// /api/hotspots) e as demais com uma. Nada é enviado: /api/send, /api/ladder e as execuções são abortados.
const { chromium, RENDERER } = require('./ambiente');
const OUT = process.argv[2];
const B = 'http://127.0.0.1:8841', project = RENDERER, run = '20261104-020244';
const HOOK = "window.__t = { state, studio, t, showTab, loadRun, renderHot, optimize, openPath, callsText, runsText, countedLines, get hot() { return hot; }, get city() { return city; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + d : ''}`); if (!pass) process.exitCode = 1; };
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await (await browser.newContext({ viewport: { width: 1500, height: 1000 } })).newPage();
  const errors = []; let sent = 0;
  page.on('pageerror', e => errors.push(e.message));
  await page.route(/\/api\/(send|ladder|generate|instrument\/run|commands\/run|commands\/approve|paste|review)\b/, r => { sent++; return r.abort(); });
  const NAME = 'impl Groups::intersect';
  await page.route(/\/api\/hotspots\b/, async r => { const x = await r.fetch(); if (!x.ok()) return r.fulfill({ response: x }); const j = await x.json();
    const row = j.rows.find(q => q.name === NAME); if (!row) return r.fulfill({ response: x, json: j }); const line = Number(row.at.slice(row.path.length + 1));
    row.count_kind = 'several_lines'; j.rows.push({ ...row, at: `${row.path}:${line + 1}`, calls: 1000, cpu_ms: 1 }); r.fulfill({ response: x, json: j }); });
  await page.route(`${B}/`, async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  await page.fill('#projectPath', project);
  await page.click('#openProject');
  await page.waitForFunction(() => __t.studio.project, null, { timeout: 60000 });
  await page.evaluate(x => __t.loadRun(x), run);
  await page.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await page.waitForTimeout(500);
  await page.evaluate(() => __t.showTab('inventory'));
  const s = await page.evaluate(n => { const rows = __t.hot.rows.filter(r => r.name === n), one = __t.hot.rows.find(r => r.name === 'impl Tuple::dot');
    return { many: rows.map(r => __t.callsText(r)), runs: __t.runsText(rows[0]), one: __t.callsText(one), oneRuns: __t.runsText(one), lines: __t.countedLines(rows[0].symbol),
      options: [...document.querySelectorAll('#hotPick option')].map(o => o.textContent).filter(x => x.startsWith(n)),
      list: document.querySelector('#hotList').innerText, legend: __t.t('legend.height') }; }, NAME);
  ok('duas linhas: cada uma diz a sua', s.lines === 2 && s.many.every(x => /vezes na linha \d+/.test(x)), s.many.join(' | '));
  ok('Separar: a lista diz a linha', s.options.length === 2 && s.options.every(x => /vezes na linha \d+/.test(x)), s.options.join(' | '));
  ok('uma linha: texto como antes', s.one === '6.410.258 vezes' && s.oneRuns === 'roda 6.410.258 vezes', `${s.one} | ${s.oneRuns}`);
  ok('o aviso visível fala da maior contagem', /a maior contagem, que pode não ser quantas vezes ela foi chamada/.test(s.list));
  ok('legenda sem "quantas vezes rodou"', !/quantas vezes/.test(s.legend), s.legend);
  await page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), NAME);
  await page.waitForTimeout(800);
  const asked = await page.evaluate(() => ({ q: document.querySelector('#question').value, env: document.querySelector('#gathered').innerText + ' ' + (document.querySelector('#suggestion')?.innerText ?? '') }));
  ok('Otimizar: a pergunta ao modelo não chama a linha de chamadas da função', /passa [\d.]+ vezes pela linha \d+, uma de 2 linhas contadas \(pode não ser quantas vezes roda\)/.test(asked.q) && !/roda [\d.]+ vezes/.test(asked.q), asked.q.slice(0, 190));
  ok('sugestão: "maior contagem entre as linhas"', /maior contagem entre as linhas: [\d.]+/.test(asked.env));
  await page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), 'impl Tuple::dot');
  await page.waitForTimeout(800);
  const single = await page.evaluate(() => ({ q: document.querySelector('#question').value, env: document.querySelector('#gathered').innerText + ' ' + (document.querySelector('#suggestion')?.innerText ?? '') }));
  ok('Otimizar, uma linha: pergunta como antes', single.q.startsWith('A contagem do Librarian (Medir tudo) mostra: impl Tuple::dot roda 6.410.258 vezes'), single.q.slice(0, 110));
  const box = await page.evaluate(async n => { const r = __t.hot.rows.find(x => x.name === n); await __t.openPath(r.symbol); await new Promise(z => setTimeout(z, 1500)); return document.querySelector('#pathView')?.textContent ?? ''; }, NAME);
  ok('fluxograma: "≤ N ×" na função de duas linhas', /≤ [\d.]+ ×/.test(box), (box.match(/≤ [\d.]+ ×/) ?? [''])[0]);
  const building = await page.evaluate(n => { const b = __t.city?.list.find(x => x.name === n); return b ? { lines: __t.countedLines(b.id), tip: __t.t('city.tipLines', { calls: '1', ms: '2' }) } : null; }, NAME);
  ok('cidade: o prédio é reconhecido como de várias linhas', building?.lines === 2, building?.tip);
  ok('nenhum envio', sent === 0);
  ok('sem erro de página', errors.length === 0, errors.join(' | ').slice(0, 200));
  await page.screenshot({ path: `${OUT}/nomes-fluxograma.png` });
  await browser.close();
})().catch(e => { console.error(e); process.exit(2); });

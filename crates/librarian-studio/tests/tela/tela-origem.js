// Origem das contagens na tela real, com o registro real do renderer (r3). Só World::shade_hit recebe
// `unknown` e Groups::intersect ganha uma segunda linha, na resposta de /api/hotspots, porque esses casos
// não existem nesse registro. Nada é enviado: /api/send, /api/ladder e as execuções são abortados.
const { chromium, RENDERER } = require('./ambiente');
const OUT = process.argv[2];
const B = 'http://127.0.0.1:8841', project = RENDERER, run = '20261104-020244';
const HOOK = "window.__t = { state, studio, t, showTab, loadRun, optimize, openPath, callsText, runsText, camera, canvas, get hot() { return hot; }, get city() { return city; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + d : ''}`); if (!pass) process.exitCode = 1; };
const CASES = [
  { kind: 'entry_calls', name: 'impl Tuple::dot', rank: /^[\d.]+ vezes$/, runs: /^roda [\d.]+ vezes$/, tip: /^[\d.]+ chamadas · /, env: /[\d.]+ chamadas/, box: /[\d.]+ × · /, notBox: /≤|na linha|contagem:/ },
  { kind: 'own_counter', name: 'impl Matrix<4>::inverse', rank: /^[\d.]+ vezes na linha \d+$/, runs: /^passa [\d.]+ vezes pela linha \d+, de um contador do próprio projeto \(pode não ser quantas vezes roda\)$/, tip: /^[\d.]+ passagens pela linha contada · /, env: /[\d.]+ passagens pela linha contada/, box: /[\d.]+ × na linha \d+ · / },
  { kind: 'several_lines', name: 'impl Groups::intersect', rank: /^[\d.]+ vezes na linha \d+$/, runs: /^passa [\d.]+ vezes pela linha \d+, uma de 2 linhas contadas/, tip: /^maior contagem entre as linhas: [\d.]+ · /, env: /maior contagem entre as linhas: [\d.]+/, box: /≤ [\d.]+ × · / },
  { kind: 'unknown', name: 'impl World::shade_hit', rank: /^contagem observada: [\d.]+$/, runs: /^tem contagem observada de [\d.]+$/, tip: /^contagem observada: [\d.]+ · /, env: /contagem observada: [\d.]+/, box: /contagem: [\d.]+ · / },
];
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await (await browser.newContext({ viewport: { width: 1500, height: 1000 } })).newPage();
  const errors = []; let sent = 0;
  page.on('pageerror', e => errors.push(e.message));
  await page.route(/\/api\/(send|ladder|generate|instrument\/run|commands\/run|commands\/approve|paste|review)\b/, r => { sent++; return r.abort(); });
  await page.route(/\/api\/hotspots\b/, async r => { const x = await r.fetch(); if (!x.ok()) return r.fulfill({ response: x }); const j = await x.json();
    const g = j.rows.find(q => q.name === 'impl Groups::intersect'), w = j.rows.find(q => q.name === 'impl World::shade_hit');
    if (g && w) { const line = Number(g.at.slice(g.path.length + 1)); g.count_kind = 'several_lines'; j.rows.push({ ...g, at: `${g.path}:${line + 1}`, calls: 1000, cpu_ms: 1 }); w.count_kind = 'unknown'; }
    r.fulfill({ response: x, json: j }); });
  await page.route(`${B}/`, async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  await page.fill('#projectPath', project);
  await page.click('#openProject');
  await page.waitForFunction(() => __t.studio.project, null, { timeout: 60000 });
  await page.evaluate(x => __t.loadRun(x), run);
  await page.waitForFunction(() => __t.hot?.rows.some(r => r.count_kind === 'unknown'), null, { timeout: 120000 });
  await page.waitForTimeout(800);
  await page.evaluate(() => __t.showTab('inventory'));
  // Dica da cidade, antes de abrir outra vista: o mouse de verdade sobre o prédio. Um prédio pode estar
  // encoberto, então procura um ponto dele que responda.
  for (const c of CASES) {
    const points = await page.evaluate(n => { const b = __t.city.list.find(x => x.name === n); if (!b) return []; const rect = __t.canvas.getBoundingClientRect(), out = [];
      for (const k of [1, .95, .85, .7, .5, .3]) { const v = b.base.clone().lerp(b.roof, k).project(__t.camera);
        for (const dx of [0, -4, 4, -8, 8]) out.push({ x: rect.left + (v.x + 1) / 2 * rect.width + dx, y: rect.top + (1 - v.y) / 2 * rect.height }); }
      return out; }, c.name);
    let tip = null;
    for (const p of points) {
      await page.mouse.move(p.x - 1, p.y - 1); await page.mouse.move(p.x, p.y); await page.waitForTimeout(120);
      tip = await page.evaluate(() => { const t = document.querySelector('#tooltip'); return { shown: t.style.display === 'block', name: t.querySelector('strong')?.textContent, line: t.querySelector('div')?.textContent }; });
      if (tip.shown && tip.name === c.name) break;
    }
    ok(`${c.kind}: dica da cidade`, tip?.shown && tip.name === c.name && c.tip.test(tip.line), `${tip?.name}: ${tip?.line}`);
    await page.mouse.move(5, 5);
  }
  for (const c of CASES) {
    const s = await page.evaluate(n => { const r = __t.hot.rows.find(x => x.name === n); return { kind: r.count_kind, rank: __t.callsText(r), runs: __t.runsText(r),
      option: [...document.querySelectorAll('#hotPick option')].map(o => o.textContent).find(x => x.startsWith(n + ' · ')) }; }, c.name);
    ok(`${c.kind}: origem de ${c.name}`, s.kind === c.kind, s.kind);
    ok(`${c.kind}: ranking`, c.rank.test(s.rank), s.rank);
    ok(`${c.kind}: Separar`, s.option?.endsWith(s.rank), s.option);
    await page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), c.name);
    await page.waitForTimeout(700);
    const asked = await page.evaluate(() => ({ q: document.querySelector('#question').value, env: document.querySelector('#gathered').innerText + ' ' + (document.querySelector('#suggestion')?.innerText ?? '') }));
    ok(`${c.kind}: pergunta ao modelo`, asked.q.includes(`mostra: ${c.name} ${s.runs}`), asked.q.slice(0, 170));
    ok(`${c.kind}: sugestão de envelope`, c.env.test(asked.env), (asked.env.match(c.env) ?? [''])[0]);
    await page.evaluate(() => document.querySelector('#question') && (document.querySelector('#question').value = ''));
    const box = await page.evaluate(async n => { const r = __t.hot.rows.find(x => x.name === n); await __t.openPath(r.symbol); await new Promise(z => setTimeout(z, 1200));
      const groups = [...document.querySelectorAll('#pathView *')].filter(e => e.children.length === 0 && /×|contagem:/.test(e.textContent)).map(e => e.textContent); return groups; }, c.name);
    const shown = await page.evaluate(n => { const r = __t.hot.rows.find(x => x.name === n); return Math.round(r.calls).toLocaleString('pt-BR'); }, c.name);
    const match = box.find(t => t.includes(shown));
    ok(`${c.kind}: fluxograma`, match && c.box.test(match) && !(c.notBox?.test(match)), match);
  }
  const server = await page.evaluate(async () => { const key = __t.studio.project.key, run = __t.studio.run; const q = async question => (await (await fetch('/api/performance-question', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run, question }) })).json());
    return { own: await q('Otimize o Matrix<4>::inverse'), entry: await q('Otimize o Tuple::dot') }; });
  ok('servidor, contador próprio: texto ao modelo sem "chamadas"', /^Matrix<4>::inverse: [\d.]+ passagens observadas pela linha \d+, de um contador do próprio projeto/.test(server.own.changes[0].text), server.own.changes[0].text.slice(0, 130));
  ok('servidor, entrada: texto ao modelo com chamadas', /^Tuple::dot: [\d.]+ chamadas observadas nesta medição/.test(server.entry.changes[0].text), server.entry.changes[0].text.slice(0, 80));
  ok('nenhum envio', sent === 0);
  ok('sem erro de página', errors.length === 0, errors.join(' | ').slice(0, 200));
  await browser.close();
})().catch(e => { console.error(e); process.exit(2); });

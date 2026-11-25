// DEC-30 check of step 2.4b (from tela-24b.js): inherited, cls, super() and @property links (`type`), and the 30 overrides of
// an abstract method as one box whose detail has the whole list and the choice.   node tela-24b.js
const path = require('path');
const fs = require('fs');
const { chromium } = require('C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/pw/node_modules/playwright-core');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const OUT = process.env.TELA_OUT || path.join(require('os').tmpdir(), 'tela-24b');
fs.mkdirSync(OUT, { recursive: true });
const RT = 'C:/Users/beatl/Documents/Projetos/renderer', HC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const HOOK = "window.__t = { state, studio, t, STRINGS, selectNode, openPath }; if (location.hash === '#debug')";
let fails = 0, passes = 0;
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 1200) : ''}`); if (pass) passes++; else { fails++; process.exitCode = 1; } };

async function page(browser, base, { locale = 'pt-BR', width = 1440, height = 900, lang } = {}) {
  const context = await browser.newContext({ viewport: { width, height }, locale });
  if (lang) await context.addInitScript(l => { try { localStorage.setItem('librarian.lang', l); } catch { } }, lang);
  const p = await context.newPage(), errors = [], writes = [];
  p.on('pageerror', e => errors.push(e.message));
  p.on('request', r => { const u = new URL(r.url()); if (r.method() !== 'GET' && !['/api/calls', '/api/project'].includes(u.pathname)) writes.push(`${r.method()} ${u.pathname}`); });
  await p.route(u => new URL(u).pathname === '/', async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await p.goto(`${base}/`);
  await p.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  return { p, errors, writes, context };
}
async function open(p, root) {
  await p.evaluate(r => { document.querySelector('#projectPath').value = r; document.querySelector('#openProject').click(); }, root);
  await p.waitForFunction(() => __t.studio.run && __t.state.nodes.length && !document.querySelector('#openProject').disabled, null, { timeout: 180000 });
  await p.waitForTimeout(800);
}
// Opens "o que chama?" from the symbol, waits for the page's own /api/calls answer and clicks (a real mouse
// click) the drawn line whose call is `text` at `where`. Returns the detail text and the link the API gave.
async function edge(p, seed, text, where) {
  if (!(await p.evaluate(id => __t.state.index.has(id), seed))) return { error: `semente fora do grafo: ${seed}` };
  await p.evaluate(() => { document.querySelector('#pathDepth').value = '1'; });
  // A call written in test code is hidden by default ("sem testes"): by its file (Python) or by its
  // owner's `mod tests_*` (Rust, src/world.rs).
  const dono = Object.values(SEEDS).find(s => s.seed === seed)?.dono ?? '';
  await p.evaluate(test => { document.querySelector('#pathNoTests').checked = !test; }, /(^|\/)tests?\/|(^|\/)test_/.test(where) || /tests_|TestCase/.test(dono));
  const answer = p.waitForResponse(r => new URL(r.url()).pathname === '/api/calls' && r.request().method() === 'POST', { timeout: 60000 });
  await p.evaluate(id => __t.openPath(id, 'calls'), seed);
  const body = await (await answer).json();
  await p.waitForFunction(() => { const n = document.querySelector('#pathNote').textContent; return n && !/Seguindo|Following/.test(n); }, null, { timeout: 60000 });
  await p.waitForTimeout(600);
  const link = body.links.find(l => l.from === seed && l.text === text && `${l.evidence.path}:${l.evidence.start_line}` === where);
  const at = await p.evaluate(([text, where]) => {
    const line = [...document.querySelectorAll('#pathCanvas polyline.edge')].find(pl => pl.querySelector('title')?.textContent.split('\n').some(s => s.startsWith(text.slice(0, 80)) && s.endsWith(where)));
    if (!line) return null;
    line.scrollIntoView({ block: 'center', inline: 'center' });
    const pts = line.getAttribute('points').split(' ').map(s => s.split(',').map(Number));
    const m = line.getScreenCTM(), pt = line.ownerSVGElement.createSVGPoint();
    pt.x = (pts[0][0] + pts[1][0]) / 2; pt.y = (pts[0][1] + pts[1][1]) / 2;
    const s = pt.matrixTransform(m);
    return { x: s.x, y: s.y, dashed: line.classList.contains('dashed') || (line.getAttribute('stroke-dasharray') ?? '') !== '' };
  }, [text, where]);
  if (!at) return { error: 'linha não desenhada', link };
  for (const dx of [0, 2, 4, 6]) { await p.mouse.click(at.x + dx, at.y); if (await p.evaluate(() => !document.querySelector('#pathDetail').hidden)) break; }
  await p.waitForTimeout(300);
  const detail = await p.evaluate(() => { const b = document.querySelector('#pathDetail'); return b.hidden ? null : b.innerText; });
  return { link, detail, dashed: at.dashed };
}
const said = (p, key, n) => p.evaluate(([k, n]) => __t.t(k, { n }), [key, n]);
const styleOf = p => p.evaluate(() => { const li = document.querySelector('#pathDetail li .muted:last-child'), b = document.querySelector('#pathDetail');
  const cs = getComputedStyle(li ?? b), bs = getComputedStyle(b); return { color: cs.color, bg: bs.backgroundColor, theme: document.documentElement.dataset.theme ?? '(sistema)',
    overflow: document.documentElement.scrollWidth > innerWidth, detailRight: b.getBoundingClientRect().right, width: innerWidth }; });

const SEEDS = require('./seeds.json');
const CASES = [
  { name: 'is_noop (@property)', root: HC, where: 'hc/api/models.py:1153', res: 'type', to: /^Transport\.is_noop$/, basis: 'type', relation: 'calls', edgeNull: true, rule: 'propriedade' },
  { name: 'cls.post (herdado)', root: HC, where: 'hc/integrations/telegram/transport.py:65', res: 'type', to: /^HttpTransport\.post$/, basis: 'type', relation: 'calls', rule: 'cls' },
  { name: 'super().__init__', root: HC, where: 'hc/integrations/telegram/transport.py:15', res: 'type', to: /^TransportError\.__init__$/, basis: 'type', relation: 'calls', edgeNull: true, rule: 'super' },
  { name: 'self.payload (herdado)', root: HC, where: 'hc/integrations/mattermost/transport.py:15', res: 'type', to: /^Slackalike\.payload$/, basis: 'type', relation: 'calls', rule: 'self' },
];

(async () => {
  const studio = path.join(REPO, '.studio');
  const before = ['renderer-9b78f3a9', 'healthchecks-25597e41'].map(k => L.hashProject(studio, k).sha256);
  const root = L.copyStudio(studio, ['renderer-9b78f3a9', 'healthchecks-25597e41']);
  fs.cpSync(path.join(REPO, 'modelfiles'), path.join(root, 'modelfiles'), { recursive: true });
  fs.mkdirSync(path.join(root, 'crates/librarian-studio/web'), { recursive: true });
  fs.copyFileSync(path.join(REPO, 'crates/librarian-studio/web/index.html'), path.join(root, 'crates/librarian-studio/web/index.html'));
  const bin = L.prepareBinaries(path.join(REPO, 'target/release'));
  console.log(`binário: ${L.binaryLine(bin)}`);
  const s = await L.startStudio(bin.pasta, root);
  const base = `http://127.0.0.1:${s.port}`;
  for (const r of [RT, HC]) {
    const { key } = await s.api('POST', '/api/project', { root: r });
    const { id } = await s.api('POST', '/api/generate', { key });
    console.log(`run novo ${key}: ${id}`);
  }
  const browser = await chromium.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  const labelOf = (p, id) => p.evaluate(id => __t.state.nodes[__t.state.index.get(id)]?.label ?? '', id);
  try {
    for (const lang of ['pt', 'en']) {
      const { p, errors, writes, context } = await page(browser, base, { locale: lang === 'pt' ? 'pt-BR' : 'en-US', lang });
      let opened = null;
      for (const c of CASES) {
        if (opened !== c.root) { await open(p, c.root); opened = c.root; }
        const sd = SEEDS[c.where];
        const r = await edge(p, sd.seed, sd.text, c.where);
        const tag = `${lang.toUpperCase()} ${c.name}`;
        if (r.error) { ok(`${tag}: desenhado e clicável`, false, `${r.error}`); continue; }
        const followedText = await said(p, `path.resolution.${c.res}`, r.link?.candidates);
        ok(`${tag}: o detalhe diz "seguida"/"followed"`, /^(seguida|followed)/m.test(r.detail ?? ''), r.detail);
        ok(`${tag}: linha cheia (seguida), não tracejada`, !r.dashed, r.dashed);
        const toLabel = await labelOf(p, r.link?.to);
        ok(`${tag}: /api/calls dá ${c.res} → ${c.to}`, r.link?.resolution === c.res && c.to.test(toLabel), JSON.stringify(r.link && { resolution: r.link.resolution, to: toLabel, basis: r.link.basis, candidate_edge: r.link.candidate_edge, candidates: r.link.candidates }));
        ok(`${tag}: basis ${c.basis}, candidate_edge ${c.edgeNull ? 'null' : 'presente'}`, r.link?.basis === c.basis && (c.edgeNull ? r.link.candidate_edge === null : typeof r.link.candidate_edge === 'string'), `${r.link?.basis} ${r.link?.candidate_edge}`);
        ok(`${tag}: o detalhe da linha diz o texto da resolução`, r.detail?.includes(followedText), `${followedText} || ${r.detail}`);
        ok(`${tag}: type_rule ${c.rule}`, r.link?.type_rule === c.rule, r.link?.type_rule);
        ok(`${tag}: relation ${c.relation}`, (r.link?.relation ?? 'calls') === c.relation, r.link?.relation);
        if (c.runs) {
          const runsLabel = r.link?.runs ? await labelOf(p, r.link.runs) : null;
          ok(`${tag}: runs ${c.runs}`, c.runs.test(runsLabel ?? ''), r.link?.runs);
        }
        ok(`${tag}: nenhuma chave crua nem código sem texto`, !/path\.resolution|path\.relation|undefined|\{n\}|\{init\}/.test(r.detail ?? '') && !new RegExp(`^${c.res}$`, 'm').test(r.detail ?? ''), r.detail);
        const shot = `${lang}-${c.res}-${c.where.replace(/[/:.]/g, '_')}.png`;
        await p.screenshot({ path: path.join(OUT, shot) });
        console.log(`      [${shot}] ${r.detail?.replace(/\n/g, ' ⏎ ')}`);
      }
      { // the abstract method: one box, not 30
        const sd = SEEDS['hc/api/models.py:1173'], tag = lang.toUpperCase() + ' F23';
        const r = await edge(p, sd.seed, sd.text, 'hc/api/models.py:1173');
        const d = await p.evaluate(() => ({ boxes: [...document.querySelectorAll('#pathCanvas g.node')].map(n => n.dataset.id), texts: [...document.querySelectorAll('#pathCanvas g.node text')].map(x => x.textContent), items: [...document.querySelectorAll('#pathDetail .many-list li')].map(li => li.innerText), dash: document.querySelectorAll('#pathCanvas polyline.ambiguous').length }));
        ok(tag + ': uma caixa só para as 30', d.boxes.filter(b => b.startsWith('many:')).length === 1 && !d.boxes.some(b => b.includes('transport') && /notify/.test(b) && !b.startsWith('many:')), d.boxes.length);
        ok(tag + ': a caixa diz 30', d.texts.some(x => /^30 (candidatas|candidates)$/.test(x)), d.texts.join(' | '));
        ok(tag + ': uma linha tracejada', d.dash === 1, d.dash);
        ok(tag + ': o detalhe lista as 30, com arquivo e botão', d.items.length === 30 && d.items.every(x => /\.notify .*transport\.py:\d+/.test(x.replace(/\n/g, ' '))), d.items.slice(0, 3).join(' || '));
        ok(tag + ': o texto da resolução', r.detail?.includes(await said(p, 'path.resolution.overrides', 30)), r.detail?.slice(0, 300));
        ok(tag + ': nenhuma chave crua', !/path\.|undefined|\{n\}/.test(r.detail ?? ''), r.detail?.slice(0, 200));
        await p.screenshot({ path: path.join(OUT, lang + '-f23.png') });
        // The box itself, clicked, opens the same list.
        await p.evaluate(() => { document.querySelector('#pathDetail').hidden = true; });
        const box = p.locator('#pathCanvas g.node[data-id^="many:"]');
        await box.click();
        ok(tag + ': clicar na caixa abre a lista', await p.evaluate(() => !document.querySelector('#pathDetail').hidden && document.querySelectorAll('#pathDetail .many-list li').length === 30));
        // Choosing one follows it.
        const ans = p.waitForResponse(x => new URL(x.url()).pathname === '/api/calls');
        await p.locator('#pathDetail .many-list li', { hasText: 'Slackalike.notify' }).locator('button').click();
        const body = await (await ans).json();
        await p.waitForTimeout(900);
        const ch = body.links.filter(l => l.resolution === 'chosen');
        ok(tag + ': a escolha vira um link chosen para Slackalike.notify', ch.length === 1 && (await labelOf(p, ch[0].to)) === 'Slackalike.notify', JSON.stringify(ch.map(x => x.to)));
        ok(tag + ': a caixa some e a linha fica cheia', await p.evaluate(() => !document.querySelector('#pathCanvas g.node[data-id^="many:"]') && !document.querySelectorAll('#pathCanvas polyline.ambiguous').length));
        await p.click('#pathResetChoices');
        await p.waitForTimeout(900);
        ok(tag + ': limpar as escolhas volta à caixa', await p.evaluate(() => !!document.querySelector('#pathCanvas g.node[data-id^="many:"]')));
      }
      ok(`${lang.toUpperCase()}: nenhum erro de página`, !errors.length, errors.join(' | '));
      ok(`${lang.toUpperCase()}: nada além de abrir o projeto (POST /api/project = store.open) e /api/calls`, !writes.length, writes.join(', '));
      await context.close();
    }
    for (const v of [{ name: 'tema escuro', width: 1440, theme: 'dark' }, { name: 'tema claro', width: 1440, theme: 'light' }, { name: 'tela estreita (390 px)', width: 390, height: 844, theme: null }]) {
      const { p, errors, context } = await page(browser, base, { width: v.width, height: v.height ?? 900, lang: 'pt' });
      await open(p, HC);
      if (v.theme) {
        for (let i = 0; i < 2 && (await p.evaluate(() => document.documentElement.dataset.theme ?? null)) !== v.theme; i++) await p.click('#theme');
      }
      const w = 'hc/api/models.py:1173';
      const r = await edge(p, SEEDS[w].seed, SEEDS[w].text, w);
      const st = await styleOf(p);
      await p.screenshot({ path: path.join(OUT, `pt-f23-${v.theme ?? 'estreita'}.png`) });
      ok(`${v.name}: o detalhe aparece com o texto do abstrato`, r.detail?.includes('o método do tipo é abstrato'), r.detail ?? r.error);
      if (v.theme) ok(`${v.name}: tema aplicado (${st.theme}), texto ${st.color} sobre ${st.bg}`, st.theme === v.theme && st.color !== st.bg, JSON.stringify(st));
      else ok(`${v.name}: sem rolagem horizontal da página`, !st.overflow, JSON.stringify(st));
      ok(`${v.name}: nenhum erro de página`, !errors.length, errors.join(' | '));
      await context.close();
    }
  } finally {
    await browser.close();
    s.stop();
  }
  const after = ['renderer-9b78f3a9', 'healthchecks-25597e41'].map(k => L.hashProject(studio, k).sha256);
  ok('.studio original igual antes e depois', JSON.stringify(before) === JSON.stringify(after));
  console.log(`\n${passes} passam, ${fails} falham. Capturas em ${OUT}`);
})().catch(e => { console.error(e); process.exit(1); });

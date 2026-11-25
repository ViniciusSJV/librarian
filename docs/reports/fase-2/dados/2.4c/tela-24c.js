// DEC-30 check of step 2.4c (from tela-24c.js): the bodies that may run (2 lines, or one box above 8), the choice, the
// text that never says "abstract", and links that stay.   node tela-24c.js
const path = require('path');
const fs = require('fs');
const { chromium } = require('C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/pw/node_modules/playwright-core');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const OUT = process.env.TELA_OUT || path.join(require('os').tmpdir(), 'tela-24c');
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
    return { x: s.x, y: s.y, dashed: line.classList.contains('ambiguous') || line.classList.contains('dashed') || (line.getAttribute('stroke-dasharray') ?? '') !== '' };
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
  { name: 'form.get_json (construtor)', root: HC, where: 'hc/integrations/call/views.py:26', res: 'type', to: /^PhoneNumberForm\.get_json$/, basis: 'type', relation: 'calls', rule: 'construtor' },
  { name: 'cls.post (herdado)', root: HC, where: 'hc/integrations/telegram/transport.py:65', res: 'type', to: /^HttpTransport\.post$/, basis: 'type', relation: 'calls', rule: 'cls' },
  { name: 'self.payload (herdado)', root: HC, where: 'hc/integrations/mattermost/transport.py:15', res: 'type', to: /^Slackalike\.payload$/, basis: 'type', relation: 'calls', rule: 'self' },
];
// One box each: the count, the body to choose, and whether the static type's own is in the list.
const MANY = [
  { name: 'notify (abstrato)', where: 'hc/api/models.py:1173', n: 30, rule: 'propriedade', pick: 'Slackalike.notify', has: 'Webhook.notify', hasNot: 'Transport.notify' },
  { name: 'raise_for_response', where: 'hc/api/transports.py:162', n: 12, rule: 'cls', pick: 'Discord.raise_for_response', has: 'HttpTransport.raise_for_response' },
  { name: 'is_noop', where: 'hc/api/models.py:1153', n: 14, rule: 'propriedade', pick: 'Webhook.is_noop', has: 'Transport.is_noop' },
];
const NO_ABSTRACT = x => !/abstrat|abstract/i.test(x ?? '');

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
      for (const m of MANY) { // one box
        const sd = SEEDS[m.where], tag = lang.toUpperCase() + ' ' + m.name;
        const r = await edge(p, sd.seed, sd.text, m.where);
        if (r.error) { ok(tag, false, r.error); continue; }
        const word = new RegExp('^' + m.n + ' (candidatas|candidates)$');
        const d = await p.evaluate(() => ({ boxes: [...document.querySelectorAll('#pathCanvas g.node[data-id^="many:"]')].map(n => n.textContent), items: [...document.querySelectorAll('#pathDetail .many-list li')].map(li => li.innerText.replace(/\n/g, ' ')) }));
        const label = x => x.split(' ')[0];
        ok(tag + ': uma caixa com a contagem', d.boxes.filter(b => b.includes(m.n + ' ' + (lang === 'pt' ? 'candidatas' : 'candidates'))).length === 1, d.boxes.join(' | '));
        ok(tag + ': linha tracejada', r.dashed, r.dashed);
        ok(tag + ': o detalhe lista todas, com arquivo', d.items.length === m.n && d.items.every(x => /transports?\.py:\d+/.test(x)), d.items.slice(0, 3).join(' || '));
        ok(tag + ': traz ' + m.has, d.items.some(x => label(x) === m.has), d.items.map(label).join());
        if (m.hasNot) ok(tag + ': não traz ' + m.hasNot, !d.items.some(x => label(x) === m.hasNot));
        ok(tag + ': homonym, basis type, type_rule ' + m.rule + ', candidates ' + m.n, r.link?.resolution === 'homonym' && r.link.basis === 'type' && r.link.type_rule === m.rule && r.link.candidates === m.n && r.link.candidate_edge === null, JSON.stringify(r.link).slice(0, 300));
        const text = m.hasNot ? await p.evaluate(([n, method]) => __t.t('path.resolution.overridesAbstract', { n, method }), [m.n, m.hasNot]) : await said(p, 'path.resolution.overrides', m.n);
        ok(tag + (m.hasNot ? ': o texto diz que ' + m.hasNot + ' é abstrato, não roda e saiu da lista' : ': o texto da resolução, sem dizer abstrato'), r.detail?.includes(text) && (m.hasNot ? r.detail.includes(m.hasNot) && /saiu da lista|not in the list/.test(r.detail) : NO_ABSTRACT(r.detail)), r.detail?.slice(0, 400));
        ok(tag + ': abstract_method ' + (m.hasNot ?? 'ausente'), m.hasNot ? (await labelOf(p, r.link?.abstract_method)) === m.hasNot : r.link && !('abstract_method' in r.link), r.link?.abstract_method);
        ok(tag + ': nenhuma chave crua', !/path\.|undefined|\{n\}/.test(r.detail ?? ''), r.detail?.slice(0, 200));
        await p.screenshot({ path: path.join(OUT, lang + '-' + m.n + '.png') });
        await p.evaluate(() => { document.querySelector('#pathDetail').hidden = true; });
        await p.locator('#pathCanvas g.node[data-id^="many:"]', { hasText: word }).click().catch(async () => p.locator('#pathCanvas g.node[data-id^="many:"]', { hasText: m.n + ' ' }).first().click());
        ok(tag + ': clicar na caixa abre a lista', await p.evaluate(n => !document.querySelector('#pathDetail').hidden && document.querySelectorAll('#pathDetail .many-list li').length === n, m.n));
        const ans = p.waitForResponse(x => new URL(x.url()).pathname === '/api/calls');
        await p.locator('#pathDetail .many-list li').filter({ has: p.locator('code', { hasText: new RegExp('^' + m.pick.replace('.', '\\.') + '$') }) }).locator('button').click();
        const body = await (await ans).json();
        await p.waitForTimeout(900);
        const ch = body.links.filter(l => l.resolution === 'chosen');
        ok(tag + ': a escolha vira um link chosen para ' + m.pick, ch.length === 1 && (await labelOf(p, ch[0].to)) === m.pick, JSON.stringify(ch.map(x => x.to)));
        await p.click('#pathResetChoices');
        await p.waitForTimeout(900);
        ok(tag + ': limpar as escolhas volta à caixa', await p.evaluate(n => [...document.querySelectorAll('#pathCanvas g.node[data-id^="many:"]')].some(x => x.textContent.includes(n + ' ')), m.n));
      }
      { // two bodies: two dashed lines, each with the choice
        const w = 'hc/integrations/slack/transport.py:72', sd = SEEDS[w], tag = lang.toUpperCase() + ' fix_asterisks';
        const r = await edge(p, sd.seed, sd.text, w);
        const lines = await p.evaluate(w => [...document.querySelectorAll('#pathCanvas polyline.edge')].filter(x => x.querySelector('title')?.textContent.startsWith('self.fix_asterisks') && x.querySelector('title').textContent.includes(w)).map(x => x.classList.contains('ambiguous')), w);
        ok(tag + ': duas linhas tracejadas', lines.length === 2 && lines.every(Boolean), JSON.stringify(lines));
        ok(tag + ': homonym, basis type, self, 2, com aresta', r.link?.resolution === 'homonym' && r.link.basis === 'type' && r.link.type_rule === 'self' && r.link.candidates === 2 && typeof r.link.candidate_edge === 'string', JSON.stringify(r.link).slice(0, 300));
        ok(tag + ': o texto da resolução, sem dizer abstrato', r.detail?.includes(await said(p, 'path.resolution.overrides', 2)) && NO_ABSTRACT(r.detail) && !('abstract_method' in r.link), r.detail);
        await p.screenshot({ path: path.join(OUT, lang + '-fix.png') });
        const ans = p.waitForResponse(x => new URL(x.url()).pathname === '/api/calls');
        await p.locator('#pathDetail button', { hasText: await said(p, 'path.choose') }).first().click();
        const ch = (await (await ans).json()).links.filter(l => l.resolution === 'chosen');
        ok(tag + ': a escolha vira um link chosen', ch.length === 1 && /fix_asterisks$/.test(await labelOf(p, ch[0].to)), JSON.stringify(ch.map(x => x.to)));
        await p.waitForTimeout(900);
        await p.click('#pathResetChoices');
        await p.waitForTimeout(600);
      }
      ok(lang.toUpperCase() + ': só o texto do abstrato diz "abstrato"', await p.evaluate(() => JSON.stringify(Object.entries(__t.STRINGS).filter(([k, v]) => k.startsWith('path.') && /abstrat|abstract/i.test(JSON.stringify(v))).map(([k]) => k)) === '["path.resolution.overridesAbstract"]'));
      ok(`${lang.toUpperCase()}: nenhum erro de página`, !errors.length, errors.join(' | '));
      ok(`${lang.toUpperCase()}: nada além de abrir o projeto (POST /api/project = store.open) e /api/calls`, !writes.length, writes.join(', '));
      await context.close();
    }
    for (const v of [{ name: 'tema escuro', width: 1440, theme: 'dark' }, { name: 'tema claro', width: 1440, theme: 'light' }, { name: 'tela estreita (390 px)', width: 390, height: 844, theme: null }].flatMap(v => [{ ...v, w: 'hc/api/models.py:1153' }, { ...v, name: v.name + ', abstrato', w: 'hc/api/models.py:1173', abstract: true }])) {
      const { p, errors, context } = await page(browser, base, { width: v.width, height: v.height ?? 900, lang: 'pt' });
      await open(p, HC);
      if (v.theme) {
        for (let i = 0; i < 2 && (await p.evaluate(() => document.documentElement.dataset.theme ?? null)) !== v.theme; i++) await p.click('#theme');
      }
      const w = v.w;
      const r = await edge(p, SEEDS[w].seed, SEEDS[w].text, w);
      const st = await styleOf(p);
      await p.screenshot({ path: path.join(OUT, `pt-${v.abstract ? "abstrato" : "noop"}-${v.theme ?? 'estreita'}.png`) });
      ok(`${v.name}: o detalhe aparece com o texto certo`, r.detail?.includes('roda o corpo da classe do objeto') && (v.abstract ? /Transport\.notify, do tipo escrito, é abstrato/.test(r.detail) : NO_ABSTRACT(r.detail)), r.detail ?? r.error);
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

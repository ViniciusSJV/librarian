// DEC-30 check of step 2.1 in the flowchart ("o que chama?"): the texts of type, trait_default and receiver,
// on the real page, against a studio over a copy of .studio (isolated root), compared with the /api/calls answer
// the page itself received. PT and EN, light and dark theme, narrow screen. Nothing is written.
const path = require('path');
const fs = require('fs');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const { chromium } = require(L.playwright());
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const OUT = process.env.TELA_OUT || path.join(require('os').tmpdir(), 'tela-21');
fs.mkdirSync(OUT, { recursive: true });
const RT = 'C:/Users/beatl/Documents/Projetos/renderer', HC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const HOOK = "window.__t = { state, studio, t, STRINGS, selectNode, openPath }; if (location.hash === '#debug')";
let fails = 0, passes = 0;
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 400) : ''}`); if (pass) passes++; else { fails++; process.exitCode = 1; } };

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
async function edge(p, seed, text, where, shot) {
  if (!(await p.evaluate(id => __t.state.index.has(id), seed))) return { error: `semente fora do grafo: ${seed}` };
  await p.evaluate(() => { document.querySelector('#pathDepth').value = '1'; });
  const answer = p.waitForResponse(r => new URL(r.url()).pathname === '/api/calls' && r.request().method() === 'POST', { timeout: 60000 });
  await p.evaluate(id => __t.openPath(id, 'calls'), seed);
  const body = await (await answer).json();
  await p.waitForFunction(() => { const n = document.querySelector('#pathNote').textContent; return n && !/Seguindo|Following/.test(n); }, null, { timeout: 60000 });
  await p.waitForTimeout(600);
  const link = body.links.find(l => l.from === seed && l.text === text && `${l.evidence.path}:${l.evidence.start_line}` === where);
  // The middle of the polyline's first segment, in screen coordinates.
  const at = await p.evaluate(([text, where]) => {
    const line = [...document.querySelectorAll('#pathCanvas polyline.edge')].find(pl => pl.querySelector('title')?.textContent.split('\n').some(s => s.startsWith(text.slice(0, 80)) && s.endsWith(where)));
    if (!line) return null;
    line.scrollIntoView({ block: 'center', inline: 'center' });
    const pts = line.getAttribute('points').split(' ').map(s => s.split(',').map(Number));
    const m = line.getScreenCTM(), pt = line.ownerSVGElement.createSVGPoint();
    pt.x = (pts[0][0] + pts[1][0]) / 2; pt.y = (pts[0][1] + pts[1][1]) / 2;
    const s = pt.matrixTransform(m);
    return { x: s.x, y: s.y };
  }, [text, where]);
  if (!at) return { error: 'linha não desenhada', link };
  await p.mouse.click(at.x, at.y);
  await p.waitForTimeout(300);
  const detail = await p.evaluate(() => { const b = document.querySelector('#pathDetail'); return b.hidden ? null : b.innerText; });
  if (shot) await p.screenshot({ path: path.join(OUT, shot) });
  return { link, detail };
}
const said = (p, key, n) => p.evaluate(([k, n]) => __t.t(k, { n }), [key, n]);
const styleOf = p => p.evaluate(() => { const li = document.querySelector('#pathDetail li .muted:last-child'), b = document.querySelector('#pathDetail');
  const cs = getComputedStyle(li ?? b), bs = getComputedStyle(b); return { color: cs.color, bg: bs.backgroundColor, theme: document.documentElement.dataset.theme ?? '(sistema)',
    overflow: document.documentElement.scrollWidth > innerWidth, detailRight: b.getBoundingClientRect().right, width: innerWidth }; });

const SEEDS = require('./seeds.json');
const CASES = [
  { name: 'self.camera.validate()', root: RT, find: { label: 'impl SceneSpec::validate', path: 'src/scene.rs' }, text: 'self.camera.validate()', where: 'src/scene.rs:128',
    res: 'type', to: /CameraSpec::validate/, basis: 'type', edgeNull: false },
  { name: 'wall.set_transform (acima do teto)', root: RT, find: { label: 'main', path: 'src/bin/librarian_measure.rs' }, text: null, textStart: 'wall.set_transform(', where: 'src/bin/librarian_measure.rs:36',
    res: 'type', to: /Plane::set_transform/, basis: 'type', edgeNull: true },
  { name: 'child.intersect(local_ray)', root: RT, find: { label: 'impl Groups::intersect', path: 'src/groups.rs' }, text: 'child.intersect(local_ray)', where: 'src/groups.rs:95',
    res: 'trait_default', to: /Intersectable::intersect/, basis: 'type', edgeNull: false },
  { name: 'self.pause() (Healthchecks)', root: HC, find: null, text: 'self.pause()', where: 'hc/accounts/management/commands/senddeletionscheduled.py:93',
    res: 'receiver', to: /^Command\.pause$/, basis: 'graph_edge', edgeNull: false, sameFile: true },
];

(async () => {
  const studio = path.join(REPO, '.studio');
  const before = ['renderer-9b78f3a9', 'healthchecks-25597e41'].map(k => L.hashProject(studio, k).sha256);
  const root = L.copyStudio(studio, ['renderer-9b78f3a9', 'healthchecks-25597e41']);
  fs.cpSync(path.join(REPO, 'modelfiles'), path.join(root, 'modelfiles'), { recursive: true });
  // The page is read from the root: the working-tree page, the one under test.
  fs.mkdirSync(path.join(root, 'crates/librarian-studio/web'), { recursive: true });
  fs.copyFileSync(path.join(REPO, 'crates/librarian-studio/web/index.html'), path.join(root, 'crates/librarian-studio/web/index.html'));
  const s = await L.startStudio(path.join(REPO, 'target/release'), root);
  const base = `http://127.0.0.1:${s.port}`;
  const browser = await chromium.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  try {
    for (const lang of ['pt', 'en']) {
      const { p, errors, writes, context } = await page(browser, base, { locale: lang === 'pt' ? 'pt-BR' : 'en-US', lang });
      let opened = null;
      for (const c of CASES) {
        if (opened !== c.root) { await open(p, c.root); opened = c.root; }
        const r = await edge(p, SEEDS[c.where].seed, SEEDS[c.where].text, c.where, null);
        const tag = `${lang.toUpperCase()} ${c.name}`;
        if (r.error) { ok(`${tag}: desenhado e clicável`, false, `${r.error} ${r.body ?? ''}`); continue; }
        const expected = await said(p, `path.resolution.${c.res}`, r.link?.candidates);
        ok(`${tag}: /api/calls dá ${c.res} → ${c.to}`, r.link?.resolution === c.res && c.to.test(await p.evaluate(id => __t.state.nodes[__t.state.index.get(id)]?.label ?? '', r.link?.to)) , JSON.stringify(r.link && { resolution: r.link.resolution, to: r.link.to.slice(-40), basis: r.link.basis, candidate_edge: r.link.candidate_edge, candidates: r.link.candidates }));
        ok(`${tag}: basis ${c.basis}, candidate_edge ${c.edgeNull ? 'null' : 'presente'}`, r.link?.basis === c.basis && (c.edgeNull ? r.link.candidate_edge === null : typeof r.link.candidate_edge === 'string'), `${r.link?.basis} ${r.link?.candidate_edge}`);
        ok(`${tag}: o detalhe da linha diz o texto da resolução`, r.detail?.includes(expected), `${expected} || ${r.detail}`);
        ok(`${tag}: nenhuma chave crua nem código sem texto`, !/path\.resolution|undefined|\{n\}/.test(r.detail ?? '') && !new RegExp(`^${c.res}$`, 'm').test(r.detail ?? ''), r.detail);
        if (c.sameFile) ok(`${tag}: o alvo está no mesmo arquivo da chamada (a própria classe Command)`, r.link.to.split(':').slice(0, 3).join(':') === r.link.from.split(':').slice(0, 3).join(':'), r.link.to);
        const shot = `${lang}-${c.res}${c.edgeNull ? '-sem-aresta' : ''}.png`;
        await p.screenshot({ path: path.join(OUT, shot) });
        console.log(`      [${shot}] ${r.detail?.replace(/\n/g, ' ⏎ ')}`);
      }
      if (lang === 'pt') {
        ok('PT: textos novos em português', (await said(p, 'path.resolution.type')).startsWith('seguida: método do tipo escrito'));
      } else {
        ok('EN: textos novos em inglês', (await said(p, 'path.resolution.type')).startsWith('followed: a method of the receiver'));
        ok('EN: receiver cita self.m()', (await said(p, 'path.resolution.receiver')).includes('(self.m())'));
      }
      ok(`${lang.toUpperCase()}: nenhum erro de página`, !errors.length, errors.join(' | '));
      ok(`${lang.toUpperCase()}: nada além de abrir o projeto (POST /api/project = store.open) e /api/calls`, !writes.length, writes.join(', '));
      await context.close();
    }
    // Themes and narrow screen, PT, on the type case.
    for (const v of [{ name: 'tema escuro', width: 1440, theme: 'dark' }, { name: 'tema claro', width: 1440, theme: 'light' }, { name: 'tela estreita (390 px)', width: 390, height: 844, theme: null }]) {
      const { p, errors, context } = await page(browser, base, { width: v.width, height: v.height ?? 900, lang: 'pt' });
      await open(p, RT);
      if (v.theme) {
        // The real toggle, clicked until the page is in the theme asked for.
        for (let i = 0; i < 2 && (await p.evaluate(() => document.documentElement.dataset.theme ?? null)) !== v.theme; i++) await p.click('#theme');
      }
      const r = await edge(p, SEEDS[CASES[0].where].seed, CASES[0].text, CASES[0].where, null);
      const st = await styleOf(p);
      await p.screenshot({ path: path.join(OUT, `pt-type-${v.theme ?? 'estreita'}.png`) });
      ok(`${v.name}: o detalhe aparece com o texto de type`, r.detail?.includes(await said(p, 'path.resolution.type')), r.detail ?? r.error);
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

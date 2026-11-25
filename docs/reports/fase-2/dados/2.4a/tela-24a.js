// DEC-30 check of step 2.4a in the flowchart ("o que chama?"): a plain call settled by the scope (`scope`,
// Python and Rust, one of them an instantiation that runs its `__init__`), one settled by an import of the file
// (`import`, above the limit) and a `unique` the scope confirms, on the real page against a studio over a copy of
// .studio (isolated root), compared with the /api/calls answer the page itself received; the inventory's step
// text. PT and EN, light and dark theme, narrow screen. Nothing is written. (From tela-23b.js.)
//   node tela-24a.js   (needs seeds.json, written by seeds24a.js)
const path = require('path');
const fs = require('fs');
const { chromium } = require('C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/pw/node_modules/playwright-core');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const OUT = process.env.TELA_OUT || path.join(require('os').tmpdir(), 'tela-24a');
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
  await p.mouse.click(at.x, at.y);
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
  { name: 'Response (escopo: classe do próprio arquivo, instancia e roda o __init__)', root: HC, where: 'hc/lib/curl.py:206', res: 'scope', to: /^Response$/, basis: 'scope', relation: 'instantiates', runs: /^Response\.__init__$/ },
  { name: 'setup_mock (escopo: def do próprio arquivo, em teste)', root: HC, where: 'hc/integrations/signal/tests/test_signal_captcha.py:66', res: 'scope', to: /^setup_mock$/, basis: 'scope', relation: 'calls' },
  { name: 'notify (import do arquivo, acima do teto, em teste)', root: HC, where: 'hc/api/tests/test_sendalerts.py:116', res: 'import', to: /^notify$/, basis: 'import', relation: 'calls', edgeNull: true },
  { name: 'create_check → _lookup (unique confirmado pelo escopo; sem mudança)', root: HC, where: 'hc/api/views.py:463', res: 'unique', to: /^_lookup$/, basis: 'graph_edge', relation: 'calls' },
  { name: 'create_default_world (Rust: escopo, função do próprio mod tests)', root: RT, where: 'src/world.rs:192', res: 'scope', to: /^tests_world::create_default_world$/, basis: 'scope', relation: 'calls' },
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
      const step = await said(p, 'inventory.step.scope'), help = await said(p, 'inventory.step.scope.help');
      if (lang === 'pt') ok('PT: textos novos em português', (await said(p, 'path.resolution.scope')).startsWith('seguida pelo escopo') && step === 'pelo escopo' && help.startsWith('Seguida pelo escopo'), `${step} | ${help}`);
      else ok('EN: textos novos em inglês', (await said(p, 'path.resolution.scope')).startsWith('followed by the scope') && step === 'by scope' && help.startsWith('Followed by the scope'), `${step} | ${help}`);
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
      const c = CASES[0];
      const r = await edge(p, SEEDS[c.where].seed, SEEDS[c.where].text, c.where);
      const st = await styleOf(p);
      await p.screenshot({ path: path.join(OUT, `pt-escopo-${v.theme ?? 'estreita'}.png`) });
      ok(`${v.name}: o detalhe aparece com o texto do escopo`, r.detail?.includes('seguida pelo escopo'), r.detail ?? r.error);
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

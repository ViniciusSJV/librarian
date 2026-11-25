// DEC-30 check of step 2.3b in the flowchart ("o que chama?"): a decorator's call (`decorates`, not followed), a
// field's call answered by the method that declares it (`via`), and the 2.3a instantiation unchanged, on the real
// page against a studio over a copy of .studio (isolated root), compared with the /api/calls answer the page itself
// received; the envelope's new reason (`decorator`) through the page's own text. PT and EN, light and dark theme,
// narrow screen. Nothing is written.
const path = require('path');
const fs = require('fs');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const { chromium } = require(L.playwright());
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const OUT = process.env.TELA_OUT || path.join(require('os').tmpdir(), 'tela-23b');
fs.mkdirSync(OUT, { recursive: true });
const RT = 'C:/Users/beatl/Documents/Projetos/renderer', HC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const HOOK = "window.__t = { state, studio, t, STRINGS, selectNode, openPath, reasonText }; if (location.hash === '#debug')";
let fails = 0, passes = 0;
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 1200) : ''}`); if (pass) passes++; else { fails++; process.exitCode = 1; } };

async function page(browser, base, { locale = 'pt-BR', width = 1440, height = 900, lang } = {}) {
  const context = await browser.newContext({ viewport: { width, height }, locale });
  if (lang) await context.addInitScript(l => { try { localStorage.setItem('librarian.lang', l); } catch { } }, lang);
  const p = await context.newPage(), errors = [], writes = [];
  p.on('pageerror', e => errors.push(e.message));
  p.on('request', r => { const u = new URL(r.url()); if (r.method() !== 'GET' && !['/api/calls', '/api/project', '/api/envelope/suggest'].includes(u.pathname)) writes.push(`${r.method()} ${u.pathname}`); });
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
  // A call written in a test file is hidden by default ("sem testes"); the field case of 2.3b is one.
  await p.evaluate(w => { document.querySelector('#pathNoTests').checked = !/(^|\/)tests?\//.test(w); }, where);
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
  { name: 'cors (decorador de checks)', root: HC, where: 'hc/api/views.py:480', res: 'unique', to: /^cors$/, basis: 'graph_edge', edgeNull: false, n: 1, relation: 'decorates' },
  { name: 'NullSink (campo do __init__, via TestServer.stdout)', root: HC, where: 'hc/api/tests/test_smtpd.py:49', res: 'unique', to: /^NullSink$/, basis: 'graph_edge', edgeNull: false, n: 1, relation: 'instantiates', runs: null, via: /^TestServer\.stdout$/ },
  { name: 'forms.WebhookForm (instancia, sem __init__; 2.3a)', root: HC, where: 'hc/integrations/webhook/views.py:21', res: 'unique', to: /^WebhookForm$/, basis: 'graph_edge', edgeNull: false, n: 1, relation: 'instantiates', runs: null },
  { name: 'TransportError (instancia, roda o __init__; 2.3a sem mudança)', root: HC, where: 'hc/integrations/github/transport.py:53', res: 'unique', to: /^TransportError$/, basis: 'graph_edge', edgeNull: false, n: 1, relation: 'instantiates', runs: /^TransportError\.__init__$/ },
];
async function unresolvedOf(p, seed) {
  await p.evaluate(() => { document.querySelector('#pathDepth').value = '1'; });
  const answer = p.waitForResponse(r => new URL(r.url()).pathname === '/api/calls' && r.request().method() === 'POST', { timeout: 60000 });
  await p.evaluate(id => __t.openPath(id, 'calls'), seed);
  const body = await (await answer).json();
  await p.waitForFunction(() => { const n = document.querySelector('#pathNote').textContent; return n && !/Seguindo|Following/.test(n); }, null, { timeout: 60000 });
  await p.evaluate(() => { const d = document.querySelector('#pathUnresolved'); if (d) d.open = true; });
  await p.waitForTimeout(300);
  const text = await p.evaluate(() => document.querySelector('#pathUnresolved')?.innerText ?? '');
  return { body, text };
}

(async () => {
  const studio = path.join(REPO, '.studio');
  const before = ['renderer-9b78f3a9', 'healthchecks-25597e41'].map(k => L.hashProject(studio, k).sha256);
  const root = L.copyStudio(studio, ['renderer-9b78f3a9', 'healthchecks-25597e41']);
  fs.cpSync(path.join(REPO, 'modelfiles'), path.join(root, 'modelfiles'), { recursive: true });
  // The page is read from the root: the working-tree page, the one under test.
  fs.mkdirSync(path.join(root, 'crates/librarian-studio/web'), { recursive: true });
  fs.copyFileSync(path.join(REPO, 'crates/librarian-studio/web/index.html'), path.join(root, 'crates/librarian-studio/web/index.html'));
  // Built first, so a stale librarian_studio is never what the page talks to; its hash is recorded.
  const bin = L.prepareBinaries(path.join(REPO, 'target/release'));
  console.log(`binário: ${L.binaryLine(bin)}`);
  const s = await L.startStudio(bin.pasta, root);
  const base = `http://127.0.0.1:${s.port}`;
  // Step 2.3a: the types come from the edition, so the page must open a run of the profile under
  // test (v9). One new run of each project in the copy; the page opens the most recent.
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
        const r = await edge(p, SEEDS[c.where].seed, SEEDS[c.where].text, c.where, null);
        const tag = `${lang.toUpperCase()} ${c.name}`;
        if (r.error) { ok(`${tag}: desenhado e clicável`, false, `${r.error}`); continue; }
        // A link the walk does not go on from (decorates; instantiates without runs) is "decidida, sem
        // seguir", never "seguida" (asked before the validation of 2.3b).
        const notWalked = c.relation === 'decorates' || (c.relation === 'instantiates' && !c.runs);
        const followedText = await said(p, `path.resolution.${c.key ?? c.res}`, r.link?.candidates);
        const prefix = await said(p, 'path.resolution.notWalked');
        const expected = notWalked ? followedText.replace(/^(seguida|followed): /, `${prefix}: `) : followedText;
        if (notWalked) ok(`${tag}: o detalhe não diz "seguida"/"followed", diz "${prefix}"`, !/^(seguida|followed):/m.test(r.detail ?? '') && (r.detail ?? '').includes(`${prefix}: `), r.detail);
        else ok(`${tag}: o detalhe diz "seguida"/"followed" (o percurso segue)`, /^(seguida|followed):/m.test(r.detail ?? ''), r.detail);
        const toLabel = await labelOf(p, r.link?.to);
        ok(`${tag}: /api/calls dá ${c.res} (${c.n}) → ${c.to}`, r.link?.resolution === c.res && r.link?.candidates === c.n && c.to.test(toLabel), JSON.stringify(r.link && { resolution: r.link.resolution, to: toLabel, basis: r.link.basis, candidate_edge: r.link.candidate_edge, candidates: r.link.candidates }));
        if (c.toFile) ok(`${tag}: o alvo é a declaração de ${c.toFile}`, r.link.to === SEEDS[c.where].to, r.link.to);
        ok(`${tag}: basis ${c.basis}, candidate_edge ${c.edgeNull ? 'null' : 'presente'}`, r.link?.basis === c.basis && (c.edgeNull ? r.link.candidate_edge === null : typeof r.link.candidate_edge === 'string'), `${r.link?.basis} ${r.link?.candidate_edge}`);
        ok(`${tag}: o detalhe da linha diz o texto da resolução`, r.detail?.includes(expected), `${expected} || ${r.detail}`);
        ok(`${tag}: relation ${c.relation}`, (r.link?.relation ?? null) === c.relation, r.link?.relation);
        if (c.relation === 'instantiates') {
          const runsLabel = r.link?.runs ? await labelOf(p, r.link.runs) : null;
          ok(`${tag}: runs ${c.runs ?? 'ausente'}`, c.runs ? c.runs.test(runsLabel ?? '') : r.link?.runs === undefined, r.link?.runs);
          const text = c.runs ? await p.evaluate(([k, init]) => __t.t(k, { init }), ['path.relation.instantiatesRuns', runsLabel]) : await said(p, 'path.relation.instantiates');
          ok(`${tag}: o detalhe diz que instancia`, r.detail?.includes(text), `${text} || ${r.detail}`);
          const prefix = await said(p, 'use.instantiates');
          ok(`${tag}: a linha da chamada começa por "${prefix}:", não por "chama"`, (r.detail ?? '').split('\n').some(l => l.startsWith(`${prefix}: `)) && !/^(chama|calls): /m.test(r.detail ?? ''), r.detail);
        }
        if (c.relation === 'decorates') {
          ok(`${tag}: o detalhe diz que decora`, r.detail?.includes(await said(p, 'path.relation.decorates')), r.detail);
          const prefix = await said(p, 'use.decorates');
          ok(`${tag}: a linha da chamada começa por "${prefix}:", não por "chama"`, (r.detail ?? '').split('\n').some(l => l.startsWith(`${prefix}: `)) && !/^(chama|calls): /m.test(r.detail ?? ''), r.detail);
          ok(`${tag}: runs ausente (não seguido)`, r.link?.runs === undefined, r.link?.runs);
        }
        if (c.via) {
          const viaLabel = r.link?.via ? await labelOf(p, r.link.via) : '';
          ok(`${tag}: via ${c.via}`, c.via.test(viaLabel), r.link?.via);
          const text = await p.evaluate(([k, field]) => __t.t(k, { field }), ['path.via', viaLabel]);
          ok(`${tag}: o detalhe diz o campo`, r.detail?.includes(text), `${text} || ${r.detail}`);
        } else ok(`${tag}: sem via`, r.link?.via === undefined, r.link?.via);
        ok(`${tag}: nenhuma chave crua nem código sem texto`, !/path\.resolution|path\.relation|path\.via|undefined|\{n\}|\{init\}|\{field\}/.test(r.detail ?? '') && !new RegExp(`^${c.res}$`, 'm').test(r.detail ?? ''), r.detail);
        const shot = `${lang}-${c.res}${c.edgeNull ? '-sem-aresta' : ''}.png`;
        await p.screenshot({ path: path.join(OUT, shot) });
        console.log(`      [${shot}] ${r.detail?.replace(/\n/g, ' ⏎ ')}`);
      }
      // The envelope's new reason, through the page's own text for a reason (`reasonText`).
      const sug = await p.evaluate(async seed => (await fetch('/api/envelope/suggest', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key: __t.studio.project.key, run: __t.studio.run, nodes: [seed] }) })).json(), SEEDS['hc/api/views.py:480'].seed);
      const deco = (sug.items ?? []).flatMap(i => i.reasons.map(r => ({ name: i.name, r }))).filter(x => x.r.code === 'decorator');
      const shown = deco.length ? await p.evaluate(r => __t.reasonText(r), deco[0].r) : '';
      ok(`${lang.toUpperCase()}: envelope de checks traz cors com o motivo decorator, e nenhum called para cors`, deco.length === 1 && deco[0].name === 'cors' && !(sug.items ?? []).some(i => i.name === 'cors' && i.reasons.some(r => r.code === 'called')), JSON.stringify(sug.items?.map(i => [i.name, i.reasons.map(r => r.code)])));
      ok(`${lang.toUpperCase()}: texto do motivo sem chave crua ("${shown}")`, !!shown && !/why\.|\{t\}|\{lines\}/.test(shown) && shown.includes('checks'), shown);
      if (lang === 'pt') {
        ok('PT: textos novos em português', (await said(p, 'path.relation.decorates')).startsWith('decora: a chamada está num decorador') && (await said(p, 'use.decorates')) === 'decora');
      } else {
        ok('EN: textos novos em inglês', (await said(p, 'path.relation.decorates')).startsWith('decorates: the call is in a decorator') && (await said(p, 'use.decorates')) === 'decorates');
      }
      ok(`${lang.toUpperCase()}: nenhum erro de página`, !errors.length, errors.join(' | '));
      ok(`${lang.toUpperCase()}: nada além de abrir o projeto (POST /api/project = store.open), /api/calls e a sugestão do envelope`, !writes.length, writes.join(', '));
      await context.close();
    }
    // Themes and narrow screen, PT, on the import case.
    for (const v of [{ name: 'tema escuro', width: 1440, theme: 'dark' }, { name: 'tema claro', width: 1440, theme: 'light' }, { name: 'tela estreita (390 px)', width: 390, height: 844, theme: null }]) {
      const { p, errors, context } = await page(browser, base, { width: v.width, height: v.height ?? 900, lang: 'pt' });
      await open(p, HC);
      if (v.theme) {
        for (let i = 0; i < 2 && (await p.evaluate(() => document.documentElement.dataset.theme ?? null)) !== v.theme; i++) await p.click('#theme');
      }
      const c = CASES[0];
      const r = await edge(p, SEEDS[c.where].seed, SEEDS[c.where].text, c.where, null);
      const st = await styleOf(p);
      await p.screenshot({ path: path.join(OUT, `pt-decora-${v.theme ?? 'estreita'}.png`) });
      ok(`${v.name}: o detalhe aparece com o texto de decora`, r.detail?.includes('decora:'), r.detail ?? r.error);
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

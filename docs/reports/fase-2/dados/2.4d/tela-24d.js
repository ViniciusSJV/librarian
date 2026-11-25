// DEC-30, passo 2.4d: "Onde é usado?" pela regra, na página real, com cliques de mouse.   node tela-24d.js [bin]
'use strict';
const path = require('path'), fs = require('fs');
const { chromium } = require('C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/pw/node_modules/playwright-core');
const L = require('C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/lib.js');
const REPO = L.REPO, OUT = process.env.TELA_OUT || path.join(require('os').tmpdir(), 'tela-24d');
fs.mkdirSync(OUT, { recursive: true });
const RT = 'C:/Users/beatl/Documents/Projetos/renderer', HC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const HOOK = "window.__t = { state, studio, t, STRINGS, selectNode, openPath, get pathSeed() { return pathSeed; } }; if (location.hash === '#debug')";
let fails = 0, passes = 0;
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d !== '' ? '  -> ' + String(d).slice(0, 500) : ''}`); if (pass) passes++; else { fails++; process.exitCode = 1; } };

async function page(browser, base, lang, { width = 1440, height = 900, scheme = 'light' } = {}) {
  const context = await browser.newContext({ viewport: { width, height }, locale: lang === 'pt' ? 'pt-BR' : 'en-US', colorScheme: scheme });
  await context.addInitScript(l => { try { localStorage.setItem('librarian.lang', l); } catch { } }, lang);
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
// Abre "onde é usado?" e devolve a resposta da própria página e o que ela mostra.
async function used(p, name, { tests = true, depth = '1', solid = false } = {}) {
  const id = await p.evaluate(n => __t.state.nodes.find(x => x.category === 'symbol' && x.label === n[0] && (!n[1] || x.id.includes(n[1]) || true) && (!n[1] || JSON.stringify(x).includes(n[1])))?.id ?? __t.state.nodes.find(x => x.category === 'symbol' && x.label === n[0])?.id, name);
  if (!id) return { error: `sem símbolo ${name}` };
  await p.evaluate(([d, t, s]) => { document.querySelector('#pathDepth').value = d; document.querySelector('#pathNoTests').checked = !t; document.querySelector('#pathSolid').checked = s; }, [depth, tests, solid]);
  if (await p.evaluate(id => __t.pathSeed === id, id)) await p.evaluate(() => document.querySelector('#pathClose').click());
  const answer = p.waitForResponse(r => new URL(r.url()).pathname === '/api/used-by', { timeout: 60000 });
  await p.evaluate(id => __t.openPath(id, 'used'), id);
  const body = await (await answer).json();
  await p.waitForFunction(() => { const n = document.querySelector('#pathNote').textContent; return n && !/Procurando|Looking/.test(n); }, null, { timeout: 90000 });
  await p.waitForTimeout(500);
  const shown = await p.evaluate(() => ({ note: document.querySelector('#pathNote').textContent, uses: document.querySelector('#pathUses').hidden ? null : document.querySelector('#pathUses > summary').textContent,
    edges: [...document.querySelectorAll('#pathCanvas polyline.edge')].map(e => ({ cls: e.getAttribute('class'), title: e.querySelector('title')?.textContent })),
    boxes: [...document.querySelectorAll('#pathCanvas g.node[data-id^="many:use"]')].map(n => n.textContent),
    solidText: document.querySelector('#pathSolidText').textContent }));
  return { id, body, ...shown };
}
async function clickEdge(p, where) {
  const at = await p.evaluate(w => {
    const line = [...document.querySelectorAll('#pathCanvas polyline.edge')].find(x => !w || x.querySelector('title')?.textContent.includes(w));
    if (!line) return null;
    line.scrollIntoView({ block: 'center', inline: 'center' });
    const pts = line.getAttribute('points').split(' ').map(s => s.split(',').map(Number)), m = line.getScreenCTM(), pt = line.ownerSVGElement.createSVGPoint();
    pt.x = (pts[0][0] + pts[1][0]) / 2; pt.y = (pts[0][1] + pts[1][1]) / 2;
    const s = pt.matrixTransform(m); return { x: s.x, y: s.y };
  }, where);
  if (!at) return null;
  for (const dx of [0, 2, 4, 6]) { await p.mouse.click(at.x + dx, at.y); if (await p.evaluate(() => !document.querySelector('#pathDetail').hidden)) break; }
  await p.waitForTimeout(200);
  return p.evaluate(() => document.querySelector('#pathDetail').hidden ? null : document.querySelector('#pathDetail').innerText);
}
const NEVER = /\bcert[oa]s?\b|\bcertain\b|undefined|\{\w+\}|path\.use|path\.resolution/i;

(async () => {
  const studio = path.join(REPO, '.studio'), keys = ['renderer-9b78f3a9', 'healthchecks-25597e41'];
  const before = keys.map(k => L.hashProject(studio, k).sha256);
  const root = L.copyStudio(studio, keys);
  fs.cpSync(path.join(REPO, 'modelfiles'), path.join(root, 'modelfiles'), { recursive: true });
  fs.mkdirSync(path.join(root, 'crates/librarian-studio/web'), { recursive: true });
  fs.copyFileSync(path.join(REPO, 'crates/librarian-studio/web/index.html'), path.join(root, 'crates/librarian-studio/web/index.html'));
  const s = await L.startStudio(path.resolve(process.argv[2] ?? path.join(REPO, 'target/release')), root), base = `http://127.0.0.1:${s.port}`;
  for (const r of [RT, HC]) { const { key } = await s.api('POST', '/api/project', { root: r }); const { id } = await s.api('POST', '/api/generate', { key }); console.log('run novo', key, id); }
  const browser = await chromium.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  try {
    for (const lang of ['pt', 'en']) {
      const T = (pt, en) => lang === 'pt' ? pt : en, tag = x => `${lang.toUpperCase()} ${x}`;
      const { p, errors, writes, context } = await page(browser, base, lang);
      const say = (k, v = {}) => p.evaluate(([k, v]) => __t.t(k, v), [k, v]);
      await open(p, HC);
      let r = await used(p, ['Webhook.is_noop']);
      ok(tag('is_noop: resposta 0/1/0, lacuna 1'), r.body.counts.linked === 0 && r.body.counts.possible_type === 1 && r.body.counts.possible_homonym === 0 && r.body.counts.above_limit === 1, JSON.stringify(r.body.counts));
      ok(tag('is_noop: linha tracejada'), r.edges.length === 1 && / ambiguous$/.test(r.edges[0].cls), JSON.stringify(r.edges));
      ok(tag('is_noop: a lacuna diz que podem usar'), /podem usar este ponto|may use this point/.test(r.uses ?? ''), r.uses);
      ok(tag('is_noop: nunca um total'), !/\btotal\b/i.test(r.note) && /possíveis|possible/.test(r.note), r.note);
      let d = await clickEdge(p, 'models.py:1153');
      ok(tag('is_noop: o detalhe diz 1 de 14'), d?.includes(await say('path.use.possibleType', { n: 14 })), d);
      ok(tag('is_noop: sem "certo" nem chave crua'), !NEVER.test(r.note + r.uses + d), r.note);
      await p.screenshot({ path: path.join(OUT, `${lang}-is_noop.png`) });
      // Um possível se expande sob pedido, com duplo clique de mouse.
      const box = await p.evaluate(() => { document.querySelector('#pathDetail').hidden = true; document.querySelector('#pathCanvas').scrollLeft = 0; const n = [...document.querySelectorAll('#pathCanvas g.node')].find(x => !x.classList.contains('seed')); const b = n.getBoundingClientRect(); return { x: b.x + b.width / 2, y: b.y + b.height / 2, id: n.dataset.id }; });
      const next = p.waitForResponse(x => new URL(x.url()).pathname === '/api/used-by');
      await p.mouse.dblclick(box.x, box.y); await next; await p.waitForTimeout(500);
      ok(tag('duplo clique no possível continua a partir dele'), (await p.evaluate(() => __t.pathSeed)) === box.id);
      ok(tag('Channel.notify: 0 usos e a lacuna aparece'), /344|^.*\b\d+ (chamadas|calls) /.test(await p.evaluate(() => document.querySelector('#pathUses > summary').textContent)), await p.evaluate(() => document.querySelector('#pathUses > summary').textContent));
      r = await used(p, ['Transport.notify']);
      ok(tag('Transport.notify: 0 usos, abstrato à parte'), r.body.links.length === 0 && r.body.counts.abstract_calls === 1 && r.uses.includes(await say('path.use.aside.abstract_calls', { n: 1 })), r.uses);
      ok(tag('sem uso: o texto não afirma que ninguém usa'), r.note === await say('path.none'), r.note);
      r = await used(p, ['cors']); d = await clickEdge(p);
      ok(tag('cors: 13 ligados, decora'), r.body.counts.linked === 13 && r.edges.every(e => e.cls === 'edge') && d?.includes(await say('path.use.decorates')), d);
      ok(tag('cors: o filtro diz só usos ligados'), r.solidText === T('só usos ligados', 'only linked uses'), r.solidText);
      r = await used(p, ['EmailThread.__init__']); d = await clickEdge(p, 'hc/lib/emails.py:77');
      ok(tag('__init__: roda ao instanciar'), r.body.counts.linked === 4 && d?.includes(await say('path.use.instanceOf', { name: 'EmailThread' })), d);
      r = await used(p, ['EmailThread']); d = await clickEdge(p, 'hc/lib/emails.py:77');
      ok(tag('classe: instancia'), d?.includes(await say('path.use.instantiates')) && d.includes(T('ligado pela regra', 'linked by the rule')), d);
      r = await used(p, ['authorize'], { tests: false }); d = await clickEdge(p);
      ok(tag('authorize: só por nome, pontilhada, e os descartados contados'), r.body.counts.name === 8 && r.edges.every(e => / named$/.test(e.cls)) && r.uses.includes(await say('path.use.aside.ruled_out', { n: 13 })) && d?.includes(await say('path.use.name')), r.uses);
      await p.screenshot({ path: path.join(OUT, `${lang}-authorize.png`) });
      // A linha do ponto, agora abaixo do desenho: com o detalhe aberto, o botão continua ao alcance do mouse.
      const pt = await p.evaluate(() => { const n = [...document.querySelectorAll('#pathCanvas g.node')].find(x => !x.classList.contains('seed')); const b = n.getBoundingClientRect(); return { x: b.x + b.width / 2, y: b.y + b.height / 2 }; });
      await p.mouse.click(pt.x, pt.y); await p.waitForTimeout(400);
      const join = await p.evaluate(() => { const b = document.querySelector('#pathPointJoin'), r = b.getBoundingClientRect(), at = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return { hidden: document.querySelector('#pathPoint').hidden, detail: !document.querySelector('#pathDetail').hidden, hit: at === b, text: b.textContent, x: r.x + r.width / 2, y: r.y + r.height / 2, h: innerHeight }; });
      ok(tag('linha do ponto: visível e clicável com o detalhe aberto'), !join.hidden && join.detail && join.hit && join.y < join.h, JSON.stringify(join));
      await p.mouse.click(join.x, join.y); await p.waitForTimeout(300);
      ok(tag('linha do ponto: Juntar funciona'), (await p.evaluate(() => document.querySelector('#pathPointJoin').textContent)) !== join.text);
      await p.evaluate(() => document.querySelector('#pathPointJoin').click());
      r = await used(p, ['Ping.get_body']); d = await clickEdge(p);
      ok(tag('get_body: o motivo, sem prometer'), d?.includes(T('ligado pela regra', 'linked by the rule')) && !NEVER.test(d), d);
      r = await used(p, ['HttpTransport.post']);
      ok(tag('post: 461 homônimos contados, fora do desenho'), r.body.counts.possible_homonym === 461 && !r.boxes.length && r.note.includes('461') && r.edges.every(e => e.cls === 'edge') && !r.body.truncated, r.note);
      const hom = await p.evaluate(async () => { const u = document.querySelector('#pathUses'), was = u.open, d = u.querySelector('details[data-use="homonym"]'), closed = !d.open; u.open = true; d.open = true; await new Promise(z => setTimeout(z, 400)); return { was, closed, n: d.querySelectorAll('li').length }; });
      ok(tag('post: recolhidos por padrão, e a lista traz os 461 sob pedido'), !hom.was && hom.closed && hom.n === 461, JSON.stringify(hom));
      r = await used(p, ['UnsubscribeTestCase']); d = await clickEdge(p);
      ok(tag('tipo do receptor: texto próprio, sem dizer que o nome está escrito'), d?.includes(await say('path.use.receiver')) && !d.includes(await say('path.use.name')) && !/receiver_type/.test(d), d);
      ok(tag('tipo do receptor: o rótulo da linha'), d?.includes(await say('use.receiver_type')));
      await p.screenshot({ path: path.join(OUT, `${lang}-receptor.png`) });
      r = await used(p, ['ProfileAdmin.login']);
      ok(tag('só homônimos: diz que estão na lista'), r.note === await say('path.use.onlyList') && r.body.counts.possible_homonym === 711, r.note);
      r = await used(p, ['HttpTransport.post'], { tests: false });
      ok(tag('post: o filtro de testes vale nas contagens'), !r.note.includes('461') && r.uses.includes(await say('path.use.hiddenTests')), r.uses);
      r = await used(p, ['HttpTransport.post'], { solid: true });
      ok(tag('post: só ligados'), r.edges.length > 0 && r.edges.every(e => e.cls === 'edge') && !r.boxes.length, r.edges.length);
      r = await used(p, ['BaseTestCase.setUp']);
      ok(tag('setUp: o desenho corta, a lista dos 139 não'), r.body.truncated && r.body.counts.linked === 139 && r.note.includes(await say('path.use.truncated', { n: 120 })), r.note);
      await p.evaluate(() => { document.querySelector('#pathUses').open = true; document.querySelector('#pathUsesBody details').open = true; });
      await p.waitForTimeout(400);
      ok(tag('setUp: 139 na lista aberta'), (await p.evaluate(() => document.querySelectorAll('#pathUsesBody details[data-use="linked"] li').length)) === 139);
      await p.screenshot({ path: path.join(OUT, `${lang}-setUp.png`) });
      const why = await say('why.caller', { t: 'X', lines: 'L1' });
      ok(tag('envelope: pelo nome escrito'), /pelo nome escrito|by the written name/.test(why), why);
      const sum = await p.evaluate(() => Object.entries(__t.STRINGS).filter(([k]) => k.startsWith('path.use')).flatMap(([, v]) => v).join(' '));
      ok(tag('nenhum texto novo diz "certo"'), !/\bcert[oa]s?\b|\bcertain\b/i.test(sum));
      await open(p, RT);
      r = await used(p, ['impl World::color_at']);
      ok(tag('color_at: 7 ligados desenhados e 1 homônimo só contado'), r.body.counts.linked === 7 && r.body.counts.possible_homonym === 1 && r.edges.length > 0 && r.edges.every(e => e.cls === 'edge'), JSON.stringify(r.body.counts));
      r = await used(p, ['impl From<Stripe> for Patterns::from']);
      ok(tag('Patterns::from: lacuna 132'), r.body.counts.above_limit === 132 && r.body.links.length === 0, JSON.stringify(r.body.counts));
      // O outro sentido continua como era.
      const c = p.waitForResponse(x => new URL(x.url()).pathname === '/api/calls');
      await p.evaluate(() => __t.openPath(__t.state.nodes.find(x => x.label === 'impl World::color_at').id, 'calls')); await c; await p.waitForTimeout(900);
      ok(tag('"O que ele chama?" desenha e esconde a lista de usos'), await p.evaluate(() => document.querySelectorAll('#pathCanvas polyline.edge').length > 0 && document.querySelector('#pathUses').hidden));
      ok(tag('nenhum erro de página'), !errors.length, errors.join(' | '));
      ok(tag('nenhuma escrita'), !writes.length, writes.join(' | '));
      await context.close();
    }
    for (const v of [{ width: 420, height: 800, name: 'estreita' }, { scheme: 'dark', name: 'escura' }]) {
      const { p, errors, context } = await page(browser, base, 'pt', v);
      await open(p, HC);
      const r = await used(p, ['HttpTransport.post']);
      const over = await p.evaluate(() => document.documentElement.scrollWidth > innerWidth);
      ok(`${v.name}: desenha, sem rolagem horizontal da página`, r.edges.length > 0 && !over && !errors.length, errors.join());
      await p.screenshot({ path: path.join(OUT, `${v.name}.png`) });
      await context.close();
    }
  } finally { await browser.close(); s.stop(); }
  ok('.studio original inalterado', keys.every((k, i) => L.hashProject(studio, k).sha256 === before[i]));
  console.log(`${passes} passam, ${fails} falham; imagens em ${OUT}`);
})().catch(e => { console.error(e); process.exit(2); });

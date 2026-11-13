// F6: o rascunho do Builder guardado neste navegador (especificação 1.2, 3.9, 5.4 a 5.8, T14, T17; critérios
// 9.2.1 a 9.2.6 e 9.2.9), na página real, num Chrome sem janela, contra o servidor de rodar.js (8841).
// Casos: recarregar restaura o mesmo rascunho (envelope, seleção nas quatro listas, alvos, fatos, entrada,
// "chegou por", cobertura, texto, escada, modo necessário, "já preparado") e o corpo de /api/gathered-question
// sai igual byte a byte; restaurar não grava o perfil nem troca o modo; trocar de run guarda e restaura, sem
// levar o rascunho a outra run; outro projeto; edition trocada, fonte inexistente (recuperar só o texto) e
// formato desconhecido não se aplicam e não são sobrescritos; duas abas (só leitura, sem mesclar, "carregar a
// outra" e "manter a minha"); armazenamento bloqueado e cheio; a vista do Builder restaurada (T17); tela
// estreita, PT e EN, temas e teclado.
// Nada é enviado a modelo e nada é gravado no servidor: toda escrita é abortada e contada; PUT /api/project
// e POST /api/gathered-question são respondidos aqui, sem servidor.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium, RENDERER, CHAVE_RENDERER } = require('./ambiente');
const B = 'http://127.0.0.1:8841', RUN = '20261104-020244', OUTRA = '20261104-014710';
const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS;
const HOOK = "window.__t = { state, studio, t, optimize, toggleGathered, selectionOnRecord, loadRun,"
  + " get navigation() { return navigation; }, get gathered() { return gathered; }, get seeds() { return seeds; }, get selection() { return selection; },"
  + " get investigation() { return investigation; }, get gatherFacts() { return gatherFacts; }, get draft() { return draft; }, get builderOpen() { return builderOpen; },"
  + " get hot() { return hot; }, get mode() { return mode; }, get slot() { return draftSlot && draftSlot.status(); } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 300) : ''}`); if (!pass) process.exitCode = 1; };
const LEITURA = (method, pathname) => method === 'GET'
  || (method === 'POST' && ['/api/project', '/api/calls', '/api/envelope/suggest', '/api/inventory/investigate', '/api/performance-question'].includes(pathname));
const perfil = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'project.json');
const sha = f => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const NOME = run => `librarian.draft.v1:${CHAVE_RENDERER}:${run}`;
const RAIZ_B = 'C:/projeto-ficticio-da-bateria/b', CHAVE_B = 'ficticio-b-00000000';
const GATHERED = u => new URL(u).pathname === '/api/gathered-question';
const INV = 'impl Matrix<4>::inverse';
// Bloqueado: o acesso ao localStorage lança. Cheio sob demanda: com __cheio, gravar um rascunho lança.
const BLOQUEIA = () => Object.defineProperty(window, 'localStorage', { get() { throw new DOMException('O acesso ao armazenamento foi negado', 'SecurityError'); } });
const ENCHE = () => {
  const original = Storage.prototype.setItem;
  Storage.prototype.setItem = function (k, v) {
    if (window.__cheio && String(k).startsWith('librarian.draft.')) throw new DOMException('cota excedida', 'QuotaExceededError');
    return original.call(this, k, v);
  };
};

async function contexto(browser, { size = { width: 1440, height: 900 }, scheme = 'light', locale = 'pt-BR', bloqueia = false } = {}) {
  const context = await browser.newContext({ viewport: size, colorScheme: scheme, locale });
  await context.addInitScript(bloqueia ? BLOQUEIA : ENCHE);
  return context;
}
// Uma página do Studio no contexto, com as escritas abortadas e contadas, aberta no renderer, na run RUN.
async function pagina(context, { run = RUN } = {}) {
  const page = await context.newPage(), errors = [], writes = [], bodies = { gathered: [], put: 0 };
  page.on('pageerror', e => errors.push(e.message));
  page.on('dialog', d => d.accept());
  await page.route(u => new URL(u).pathname === '/', async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.route(u => new URL(u).pathname.startsWith('/api/'), async r => {
    const req = r.request(), { pathname } = new URL(req.url());
    if (LEITURA(req.method(), pathname)) return r.continue();
    // PUT /api/project: respondido com o perfil enviado, sem gravar (F7: só Alterar a política e Salvar o chamam).
    if (req.method() === 'PUT' && pathname === '/api/project') {
      bodies.put++;
      const res = await fetch(`${B}/api/project?key=${encodeURIComponent(CHAVE_RENDERER)}`);
      return r.fulfill({ json: { ...(await res.json()), settings: JSON.parse(req.postData()).settings } });
    }
    writes.push(`${req.method()} ${pathname}`);
    return r.abort();
  });
  await page.route(u => new URL(u).pathname.startsWith('/api/') && new URL(u).searchParams.get('key') === CHAVE_B, r => {
    if (r.request().method() !== 'GET') return r.fallback();
    const u = new URL(r.request().url()); u.searchParams.set('key', CHAVE_RENDERER);
    return r.fallback({ url: u.toString() });
  });
  await page.route(u => new URL(u).pathname === '/api/project', async r => {
    const req = r.request();
    if (req.method() !== 'POST' || JSON.parse(req.postData()).root !== RAIZ_B) return r.fallback();
    const res = await r.fetch({ postData: JSON.stringify({ root: RENDERER }) }), data = await res.json();
    return r.fulfill({ json: { ...data, key: CHAVE_B, settings: { ...data.settings, name: 'ficticio-b', root: RAIZ_B }, runs: [] } });
  });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  if (await page.evaluate(() => __t.studio.project?.key) !== CHAVE_RENDERER) await abrePor(page, RENDERER, CHAVE_RENDERER);
  await page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
  if (await page.evaluate(() => __t.studio.run) !== run) await trocaRun(page, run);
  await page.waitForTimeout(400);
  return { page, errors, writes, bodies };
}
async function abrePor(page, root, key) {
  await page.evaluate(r => { document.querySelector('#projectPath').value = r; document.querySelector('#openProject').click(); }, root);
  await page.waitForFunction(k => __t.studio.project?.key === k && !document.querySelector('#openProject').disabled, key, { timeout: 120000 });
  await page.waitForTimeout(600);
}
async function trocaRun(page, run) {
  const turns = page.waitForResponse(r => r.url().includes('/api/turns') && r.url().includes(`run=${run}`), { timeout: 120000 });
  await page.selectOption('#runs', run);
  await turns;
  await page.waitForFunction(r => __t.studio.run === r && __t.state.nodes.length && __t.state.graph, run, { timeout: 120000 });
  await page.waitForTimeout(500);
}
// Recarregar abre a run guardada do projeto (F9) ou, sem nenhuma, a mais recente: `run` volta à run do caso, se preciso.
async function recarrega(page, run = RUN) {
  await page.reload();
  await page.waitForFunction(() => window.__t && __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
  if (run && await page.evaluate(() => __t.studio.run) !== run) await trocaRun(page, run);
  await page.waitForTimeout(800);
}
async function responderPreparo(x) {
  await x.page.unroute(GATHERED).catch(() => {});
  const turns = await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`)).json();
  const base = turns.filter(t => t.gathered).at(-1) ?? turns.at(-1);
  await x.page.route(GATHERED, r => {
    const body = r.request().postData();
    x.bodies.gathered.push(body);
    return r.fulfill({ json: { ...base, id: 900000 + x.bodies.gathered.length, question: JSON.parse(body).question, sends: [] } });
  });
}
// O rascunho inteiro como a página o tem: o que vai ao servidor e o que só a tela sabe.
const rascunho = page => page.evaluate(() => JSON.stringify({ g: __t.gathered, s: __t.seeds, f: __t.gatherFacts, r: __t.selectionOnRecord(),
  i: __t.investigation && [...__t.investigation.ticked], sel: __t.selection && [...__t.selection.sources.keys()], dec: __t.selection && [...__t.selection.decisions],
  e: __t.draft.entry, m: __t.draft.required_mode, a: [...__t.draft.arrived], c: [...__t.draft.coverage].map(([k, v]) => [k, v.coverage_state]), p: __t.draft.prepared_as,
  q: document.querySelector('#question').value, l: document.querySelector('#ladderOn').checked }));
const tela = page => page.evaluate(() => {
  const vis = s => { const e = document.querySelector(s); return e.hidden ? '' : e.textContent; };
  return { draft: vis('#builderDraft') && document.querySelector('#builderDraftText').textContent, actions: [...document.querySelectorAll('#builderDraftActions button')].map(b => b.textContent),
    note: vis('#draftNote'), head: vis('#headDraft'), headTitle: document.querySelector('#headDraft').title, state: document.querySelector('#builderState').textContent,
    entry: vis('#builderEntry'), notice: vis('#builderNotice'), tab: !document.querySelector('#centerBuilderTab').hidden, view: __t.navigation.center,
    readOnly: document.querySelector('#question').readOnly, slot: __t.slot };
});
const guardado = (page, run = RUN) => page.evaluate(n => localStorage.getItem(n), NOME(run));
// Clica uma escolha do estado do rascunho; quando ela pede confirmação (o diálogo da página), confirma.
async function clica(page, texto) {
  await page.evaluate(t => [...document.querySelectorAll('#builderDraftActions button')].find(b => b.textContent === t).click(), texto);
  await page.waitForTimeout(250);
  if (await page.evaluate(() => document.querySelector('#askDialog').open)) await page.click('#askOk');
}
async function otimiza(page) {
  await page.waitForFunction(() => __t.hot?.rows?.length, null, { timeout: 120000 });
  await page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
  await page.waitForFunction(() => __t.gathered.length && !document.querySelector('#centerBuilderState').textContent, null, { timeout: 60000 });
  await page.waitForTimeout(300);
}
// F7 (T26): preparar um rascunho de Otimizar (modo necessário Melhorar) com a política em Localizar confirma a
// mudança da política antes; o PUT é respondido aqui, sem gravar.
async function confirmaT26(page) {
  await page.waitForFunction(() => document.querySelector('#askDialog').open, null, { timeout: 30000 });
  await page.click('#askOk');
}
const primeiroSimbolo = page => page.evaluate(() => __t.state.nodes.find(n => n.category === 'symbol').id);

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const perfilAntes = sha(perfil);
  const todasEscritas = [], todosErros = [];
  const fecha = async (ctx, ...xs) => { for (const x of xs) { todasEscritas.push(...x.writes); todosErros.push(...x.errors); } await ctx.close(); };
  try {
    // ================================================================ 1. guardar e recarregar (9.2.1, 9.2.9)
    let ctx = await contexto(browser);
    let x = await pagina(ctx);
    ok('1 sem rascunho: nada guardado para a run', await guardado(x.page) === null);
    await otimiza(x.page);
    let g = JSON.parse(await guardado(x.page) ?? 'null');
    ok('1 Otimizar guarda o rascunho em librarian.draft.v1:<key>:<run>, com esquema, key, run, catalog_sha256, saved_at e {tab, seq}',
      g && g.schema === 'librarian.draft.v1' && g.key === CHAVE_RENDERER && g.run === RUN && /^[0-9a-f]{64}$/.test(g.catalog_sha256) && !Number.isNaN(Date.parse(g.saved_at)) && g.tab && Number.isInteger(g.seq),
      JSON.stringify(g && { ...g, body: undefined }));
    ok('1 o catalog_sha256 guardado é o do grafo da run', g?.catalog_sha256 === await x.page.evaluate(() => __t.state.graph.catalog_sha256));
    ok('1 o corpo guardado tem as partes de 5.4 (Builder, achado, compositor, vínculo)', g && ['builder', 'finding', 'composer', 'prepared_as'].every(k => k in g.body)
      && ['included', 'seeds', 'with_facts', 'selection', 'entry', 'arrived', 'coverage'].every(k => k in g.body.builder) && ['text', 'ladder', 'required_mode'].every(k => k in g.body.composer), JSON.stringify(Object.keys(g?.body ?? {})));
    ok('1 nada da política no rascunho guardado (5.3)', !/allow_hypothesis|allow_patch|"mode"/.test(JSON.stringify(g)));
    let s = await tela(x.page);
    ok('1 o cabeçalho diz "guardado neste navegador"', /^Rascunho: \d+ fonte\(s\) · guardado neste navegador$/.test(s.head) && /não é registro/.test(s.headTitle), s.head);
    // Preparar (Localizar, sem escada, Nenhum), para conferir o corpo e o "já preparado" depois de recarregar.
    await responderPreparo(x);
    await x.page.evaluate(() => { const l = document.querySelector('#ladderOn'); l.checked = false; l.dispatchEvent(new Event('change')); const p = document.querySelector('#provider'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
    await x.page.fill('#question', 'F6 bateria: antes de recarregar');
    await x.page.click('#ask');
    await confirmaT26(x.page);
    await x.page.waitForFunction(() => /Já preparado/.test(document.querySelector('#builderState').textContent), null, { timeout: 30000 });
    const corpo1 = JSON.parse(x.bodies.gathered.at(-1));
    await x.page.fill('#question', 'F6 bateria: texto guardado');
    await x.page.evaluate(() => { const l = document.querySelector('#ladderOn'); l.checked = true; l.dispatchEvent(new Event('change')); });
    const antes = await rascunho(x.page);
    ok('1 o texto e a escada digitados ficam guardados', JSON.parse(await guardado(x.page)).body.composer.text === 'F6 bateria: texto guardado' && JSON.parse(await guardado(x.page)).body.composer.ladder === true);
    const modoAntes = await x.page.evaluate(() => localStorage.getItem('librarian.mode'));
    const docAntes = await guardado(x.page);
    const putAntes = x.bodies.put;
    const centroAntes = await x.page.evaluate(k => JSON.parse(localStorage.getItem(k) ?? '{}').center?.view ?? null, `librarian.nav.v1:${CHAVE_RENDERER}`);
    await recarrega(x.page);
    ok('1 9.2.1 recarregar restaura o mesmo rascunho (envelope, seleção, decisões, alvos, fatos, entrada, chegou por, cobertura, vínculo, texto, escada)', await rascunho(x.page) === antes,
      `${antes.slice(0, 150)} | ${(await rascunho(x.page)).slice(0, 150)}`);
    s = await tela(x.page);
    ok('1 3.9 "Rascunho restaurado deste navegador ({data})"', /^Rascunho restaurado deste navegador \(.+\)$/.test(s.draft), s.draft);
    ok('1 o "já preparado" sobrevive: mesmo registro, mesmo vínculo', /Já preparado como #9000\d+, sem mudanças desde então/.test(s.state), s.state);
    // F9: recarregar volta direto a esta run (a guardada), com o rascunho restaurado: a vista central guardada é refeita
    // (T17, como no caso 7). Antes do F9 recarregar abria a run mais recente, e o Builder só voltava à barra.
    ok('1 o Builder volta à barra com as fontes, e à frente só se era a vista central guardada (T17)', s.tab && s.view === (centroAntes === 'builder' ? 'builder' : 'city'), JSON.stringify([s.tab, s.view, centroAntes]));
    ok('1 9.2.9 restaurar não grava o perfil nem troca o modo', x.bodies.put === putAntes && await x.page.evaluate(() => __t.mode) === 'locate'
      && await x.page.evaluate(() => localStorage.getItem('librarian.mode')) === modoAntes, `${x.bodies.put} ${modoAntes}`);
    ok('1 5.5 passo 8: o modo necessário (Melhorar) aparece, diferente do modo da página, sem trocar nada', /Melhorar.*a política do projeto está em Localizar/.test(s.entry), s.entry);
    ok('1 restaurar não regrava o rascunho (o guardado fica igual em bytes)', await guardado(x.page) === docAntes);
    await x.page.evaluate(() => { const l = document.querySelector('#ladderOn'); l.checked = false; l.dispatchEvent(new Event('change')); const p = document.querySelector('#provider'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
    await responderPreparo(x);
    await x.page.fill('#question', 'F6 bateria: antes de recarregar');
    await x.page.click('#ask');
    await confirmaT26(x.page);
    const n1 = x.bodies.gathered.length;
    for (let i = 0; i < 100 && x.bodies.gathered.length === n1; i++) await x.page.waitForTimeout(100);
    const corpo2 = JSON.parse(x.bodies.gathered.at(-1));
    ok('1 9.5 depois de recarregar, o corpo de /api/gathered-question é byte a byte o de antes', JSON.stringify(corpo2) === JSON.stringify(corpo1), `${JSON.stringify(corpo1).length} ${JSON.stringify(corpo2).length}`);
    s = await tela(x.page);
    ok('1 depois de mudar, o aviso de restaurado sai', !s.draft, s.draft);

    // ================================================================ 2. trocar de run (9.2.2, 9.2.3, T14)
    await x.page.fill('#question', 'F6 bateria: texto da run nova');
    const naRun = await rascunho(x.page);
    await trocaRun(x.page, OUTRA);
    s = await tela(x.page);
    ok('2 T14 trocar de run: aviso "O rascunho da run X ficou guardado neste navegador"', s.note === `O rascunho da run ${RUN} ficou guardado neste navegador.`, s.note);
    ok('2 9.2.3 o rascunho não aparece na outra run: Builder vazio e compositor vazio', await x.page.evaluate(() => !__t.gathered.length && !__t.investigation && !document.querySelector('#question').value && !__t.draft.entry), await rascunho(x.page));
    ok('2 nada guardado para a outra run; o da run deixada continua', await guardado(x.page, OUTRA) === null && JSON.parse(await guardado(x.page)).body.composer.text === 'F6 bateria: texto da run nova');
    const id2 = await primeiroSimbolo(x.page);
    await x.page.evaluate(i => __t.toggleGathered(i), id2);
    await x.page.fill('#question', 'F6 bateria: rascunho da outra run');
    const naOutra = await rascunho(x.page);
    ok('2 a outra run guarda o próprio rascunho, na própria chave', JSON.parse(await guardado(x.page, OUTRA) ?? 'null')?.body.builder.included.join() === id2 && JSON.parse(await guardado(x.page, OUTRA)).run === OUTRA);
    await trocaRun(x.page, RUN);
    ok('2 9.2.2 voltar à run restaura o rascunho dela', await rascunho(x.page) === naRun);
    ok('2 o aviso agora fala da outra run', (await tela(x.page)).note === `O rascunho da run ${OUTRA} ficou guardado neste navegador.`, (await tela(x.page)).note);
    await trocaRun(x.page, OUTRA);
    ok('2 e a outra run, o dela', await rascunho(x.page) === naOutra);
    await trocaRun(x.page, RUN);

    // ================================================================ 3. outro projeto (T25: rascunhos de cada projeto e run)
    const chavesAntes = await x.page.evaluate(() => Object.keys(localStorage).filter(k => k.startsWith('librarian.draft')).sort().join());
    await abrePor(x.page, RAIZ_B, CHAVE_B);
    s = await tela(x.page);
    ok('3 abrir B (sem runs): nada do rascunho de A na página', await x.page.evaluate(() => !__t.gathered.length && !document.querySelector('#question').value) && !s.head, JSON.stringify(s.head));
    ok('3 o aviso diz que o rascunho de A ficou guardado', s.note === `O rascunho da run ${RUN} ficou guardado neste navegador.`, s.note);
    ok('3 os rascunhos guardados ficam iguais', await x.page.evaluate(() => Object.keys(localStorage).filter(k => k.startsWith('librarian.draft')).sort().join()) === chavesAntes);
    await abrePor(x.page, RENDERER, CHAVE_RENDERER);
    await x.page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
    if (await x.page.evaluate(() => __t.studio.run) !== RUN) await trocaRun(x.page, RUN);
    ok('3 reabrir A restaura o rascunho da run', await rascunho(x.page) === naRun);
    // Recomeçar não apaga rascunhos guardados (2.1).
    await x.page.click('#resetStudio');
    await x.page.waitForTimeout(300);
    ok('3 Recomeçar limpa a página e não apaga os rascunhos guardados', await x.page.evaluate(() => !__t.gathered.length) && await x.page.evaluate(() => Object.keys(localStorage).filter(k => k.startsWith('librarian.draft')).sort().join()) === chavesAntes);
    await fecha(ctx, x);

    // ================================================================ 4. incompatíveis (5.5 passos 1 a 4; 9.2.4)
    ctx = await contexto(browser);
    x = await pagina(ctx);
    await otimiza(x.page);
    const bom = await guardado(x.page);
    // Passo 3: outra edition.
    await x.page.evaluate(([n, v]) => { const d = JSON.parse(v); d.catalog_sha256 = 'f'.repeat(64); localStorage.setItem(n, JSON.stringify(d)); }, [NOME(RUN), bom]);
    let alterado = await guardado(x.page);
    await recarrega(x.page);
    s = await tela(x.page);
    ok('4 9.2.4 edition diferente: não aplicado, com os nomes, e só Descartar', /^Rascunho de outra edition: não aplicado\. Fontes: .*Matrix<4>::inverse/.test(s.draft) && s.actions.join() === 'Descartar' && await x.page.evaluate(() => !__t.gathered.length), s.draft);
    ok('4 o cabeçalho e o compositor dizem que há um rascunho guardado não aplicado', s.head === 'Rascunho guardado não aplicado' && /não foi aplicado/.test(s.note) && s.tab, JSON.stringify([s.head, s.note]));
    await x.page.evaluate(i => __t.toggleGathered(i), await primeiroSimbolo(x.page));
    s = await tela(x.page);
    ok('4 o rascunho novo desta página não sobrescreve o guardado: fica só nesta página', await guardado(x.page) === alterado && /só nesta página/.test(s.head) && /só nesta página até você descartar/.test(s.draft), s.head);
    await clica(x.page, 'Descartar');
    await x.page.waitForTimeout(400);
    g = JSON.parse(await guardado(x.page) ?? 'null');
    ok('4 Descartar (com confirmação) apaga o guardado e guarda o desta página no lugar', g && g.catalog_sha256 === await x.page.evaluate(() => __t.state.graph.catalog_sha256) && g.body.builder.included.length === 1 && !(await tela(x.page)).draft, JSON.stringify(g?.body.builder.included));
    // Passo 4: fonte inexistente.
    await x.page.evaluate(([n, v]) => { const d = JSON.parse(v); d.body.builder.included.push('nao-existe::F6'); d.body.builder.selection.sources.push({ id: 'nao-existe::F6', name: 'fonte_fantasma', provenance: 'manual' }); d.body.composer.text = 'F6 texto a recuperar'; localStorage.setItem(n, JSON.stringify(d)); }, [NOME(RUN), bom]);
    alterado = await guardado(x.page);
    await recarrega(x.page);
    s = await tela(x.page);
    ok('4 fonte inexistente: "1 fonte(s) não existem nesta edition: fonte_fantasma", não aplicado', /^1 fonte\(s\) não existem nesta edition: fonte_fantasma\. O rascunho não foi aplicado\./.test(s.draft) && await x.page.evaluate(() => !__t.gathered.length), s.draft);
    ok('4 oferece Descartar e recuperar só o texto', s.actions.join('|') === 'Descartar|Recuperar só o texto do compositor', s.actions.join('|'));
    ok('4 nada sobrescrito antes da escolha', await guardado(x.page) === alterado);
    await clica(x.page, 'Recuperar só o texto do compositor');
    await x.page.waitForTimeout(400);
    g = JSON.parse(await guardado(x.page) ?? 'null');
    ok('4 recuperar o texto (com confirmação): o texto volta ao compositor, sem fontes, e é guardado', await x.page.evaluate(() => document.querySelector('#question').value) === 'F6 texto a recuperar'
      && await x.page.evaluate(() => !__t.gathered.length) && g?.body.composer.text === 'F6 texto a recuperar' && !g.body.builder.included.length, JSON.stringify(g?.body.composer));
    // Passo 1: formato desconhecido (esquema e corpo).
    for (const [rotulo, valor] of [['esquema', JSON.stringify({ schema: 'librarian.draft.v0', key: CHAVE_RENDERER, run: RUN, body: {} })],
      ['corpo', JSON.stringify({ ...JSON.parse(bom), body: { v: 99 } })]]) {
      await x.page.evaluate(([n, v]) => localStorage.setItem(n, v), [NOME(RUN), valor]);
      await recarrega(x.page);
      s = await tela(x.page);
      ok(`4 formato desconhecido (${rotulo}): não aplicado; Guardar ou Descartar`, /^Rascunho em formato desconhecido: não aplicado/.test(s.draft) && s.actions.join('|') === 'Guardar|Descartar' && await guardado(x.page) === valor, s.draft);
      await clica(x.page, 'Guardar');
      s = await tela(x.page);
      ok(`4 Guardar (${rotulo}): continua guardado, sem aplicar`, /continua guardado neste navegador, sem ser aplicado/.test(s.draft) && s.actions.join() === 'Descartar' && await guardado(x.page) === valor, s.draft);
    }
    await clica(x.page, 'Descartar');
    await x.page.waitForTimeout(400);
    ok('4 Descartar apaga o de formato desconhecido', await guardado(x.page) === null && !(await tela(x.page)).draft);
    await fecha(ctx, x);

    // ================================================================ 5. duas abas (5.7; 9.2.5)
    ctx = await contexto(browser);
    const a1 = await pagina(ctx);
    await otimiza(a1.page);
    const a2 = await pagina(ctx);
    ok('5 a segunda aba restaura o mesmo rascunho', await rascunho(a2.page) === await rascunho(a1.page));
    await a2.page.fill('#question', 'F6 texto da aba 2');
    await a1.page.waitForFunction(() => __t.slot?.state === 'conflict', null, { timeout: 10000 }).catch(() => {});
    s = await tela(a1.page);
    ok('5 9.2.5 a outra aba gravou: "Rascunho alterado em outra aba", só leitura, sem mesclar', /^Rascunho alterado em outra aba: só leitura até você escolher\. Nada é mesclado\.$/.test(s.draft) && s.readOnly
      && s.actions.join('|') === 'Carregar a versão da outra aba|Manter a minha (sobrescrever)' && s.head.includes('Rascunho alterado em outra aba'), JSON.stringify(s));
    const guardadoA2 = await guardado(a1.page), r1 = await rascunho(a1.page);
    await a1.page.evaluate(i => __t.toggleGathered(i), await primeiroSimbolo(a1.page));
    s = await tela(a1.page);
    ok('5 mudar em conflito é recusado, com o motivo escrito, e nada é gravado', await rascunho(a1.page) === r1 && await guardado(a1.page) === guardadoA2 && /escolha no Builder qual versão fica/.test(s.notice), s.notice);
    ok('5 os controles do Builder ficam desabilitados', await a1.page.evaluate(() => [...document.querySelectorAll('#builderRecord button, #builderClear')].every(b => b.disabled)));
    await a1.page.evaluate(() => { document.querySelector('#question').readOnly = false; document.querySelector('#question').value = 'x'; document.querySelector('#ask').disabled = false; document.querySelector('#ask').click(); document.querySelector('#question').readOnly = true; });
    ok('5 preparar em conflito é recusado, com o motivo', /escolha no Builder qual versão fica/.test(await a1.page.evaluate(() => document.querySelector('#llmNote').textContent)) && await guardado(a1.page) === guardadoA2);
    await a1.page.evaluate(() => { document.querySelector('#question').value = 'F6 bateria: texto guardado'; });
    await clica(a1.page, 'Carregar a versão da outra aba');
    await a1.page.waitForTimeout(300);
    s = await tela(a1.page);
    ok('5 "Carregar a versão da outra aba": a página fica com a versão guardada, sem conflito', await a1.page.evaluate(() => document.querySelector('#question').value) === 'F6 texto da aba 2' && !s.readOnly && s.slot.state !== 'conflict' && await guardado(a1.page) === guardadoA2, JSON.stringify(s.slot?.state));
    await a2.page.fill('#question', 'F6 texto da aba 2, de novo');
    await a1.page.waitForFunction(() => __t.slot?.state === 'conflict', null, { timeout: 10000 }).catch(() => {});
    await clica(a1.page, 'Manter a minha (sobrescrever)');
    await a1.page.waitForTimeout(300);
    g = JSON.parse(await guardado(a1.page));
    ok('5 "Manter a minha": a versão desta aba sobrescreve a outra', g.body.composer.text === 'F6 texto da aba 2' && g.tab === await a1.page.evaluate(() => localState.tab), g.body.composer.text);
    await a2.page.waitForFunction(() => __t.slot?.state === 'conflict', null, { timeout: 10000 }).catch(() => {});
    ok('5 e agora é a outra aba que entra em "alterado em outra aba"', (await tela(a2.page)).slot?.state === 'conflict');
    await fecha(ctx, a1, a2);

    // ================================================================ 6. armazenamento bloqueado e cheio (5.8; 9.2.6)
    ctx = await contexto(browser, { bloqueia: true });
    x = await pagina(ctx);
    await otimiza(x.page);
    s = await tela(x.page);
    ok('6 9.2.6 bloqueado: a página funciona e diz "só nesta página"', /· só nesta página$/.test(s.head) && /^Rascunho só nesta página: o armazenamento do navegador não está disponível/.test(s.draft), JSON.stringify([s.head, s.draft]));
    const bloqueado = await rascunho(x.page);
    await trocaRun(x.page, OUTRA);
    ok('6 trocar de run: "ficou só nesta página"', (await tela(x.page)).note === `O rascunho da run ${RUN} ficou só nesta página.`, (await tela(x.page)).note);
    await trocaRun(x.page, RUN);
    ok('6 voltar: o rascunho desta página volta', await rascunho(x.page) === bloqueado);
    await x.page.mouse.click(5, 5);
    let perguntou = false;
    x.page.removeAllListeners('dialog');
    x.page.on('dialog', d => { if (d.type() === 'beforeunload') perguntou = true; d.accept(); });
    await x.page.close({ runBeforeUnload: true });
    await new Promise(z => setTimeout(z, 1500));
    ok('6 9.2.6 sair com o rascunho só nesta página: o navegador pergunta antes', perguntou);
    await fecha(ctx, x);
    ctx = await contexto(browser);
    x = await pagina(ctx);
    await otimiza(x.page);
    const ultimo = await guardado(x.page);
    await x.page.evaluate(() => { window.__cheio = true; });
    await x.page.fill('#question', 'F6 texto que não cabe');
    s = await tela(x.page);
    ok('6 cota excedida: "Não foi possível guardar o rascunho"; a última versão guardada fica', /^Não foi possível guardar o rascunho: o armazenamento do navegador está cheio/.test(s.draft) && /só nesta página$/.test(s.head) && await guardado(x.page) === ultimo, s.draft);
    await x.page.evaluate(() => { window.__cheio = false; });
    await x.page.fill('#question', 'F6 texto que cabe');
    s = await tela(x.page);
    ok('6 com espaço de novo, a mudança seguinte é guardada', JSON.parse(await guardado(x.page)).body.composer.text === 'F6 texto que cabe' && /guardado neste navegador$/.test(s.head) && !s.draft, s.head);

    await fecha(ctx, x);

    // ================================================================ 7. a run aberta pelo recarregamento: restauração direta e a vista do Builder (T17)
    ctx = await contexto(browser);
    const RECENTE = (await (await fetch(`${B}/api/project`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ root: RENDERER }) })).json()).runs[0].id;
    x = await pagina(ctx, { run: RECENTE });
    await x.page.evaluate(i => __t.toggleGathered(i), await primeiroSimbolo(x.page));
    await x.page.fill('#question', 'F6 texto que cabe');
    const naRecente = await rascunho(x.page);
    await x.page.click('#draftOpen');
    ok('7 abrir o Builder grava center.view = builder', JSON.parse(await x.page.evaluate(k => localStorage.getItem(k), `librarian.nav.v1:${CHAVE_RENDERER}`)).center?.view === 'builder');
    await recarrega(x.page, null);
    s = await tela(x.page);
    ok(`7 9.2.1 recarregar abre a run mais recente (${RECENTE}) e restaura o rascunho dela direto`, await x.page.evaluate(() => __t.studio.run) === RECENTE && await rascunho(x.page) === naRecente && /^Rascunho restaurado deste navegador/.test(s.draft), s.draft);
    ok('7 T17 recarregar com o rascunho restaurado traz o Builder de volta à frente', s.view === 'builder' && s.tab, JSON.stringify([s.view, s.tab]));
    await x.page.evaluate(() => document.querySelector('#centerCity').click());
    ok('7 ir à Cidade não muda o rascunho guardado', JSON.parse(await guardado(x.page, RECENTE)).body.composer.text === 'F6 texto que cabe');
    await fecha(ctx, x);

    // ================================================================ 8. tela estreita, EN, temas, teclado
    for (const [scheme, locale] of [['dark', 'en-US'], ['light', 'pt-BR']]) {
      ctx = await contexto(browser, { size: { width: 390, height: 844 }, scheme, locale });
      x = await pagina(ctx);
      if (locale === 'en-US') await x.page.evaluate(() => document.querySelector('[data-lang="en"]')?.click());
      await x.page.evaluate(([n, v]) => { const d = JSON.parse(v); d.catalog_sha256 = 'e'.repeat(64); localStorage.setItem(n, JSON.stringify(d)); }, [NOME(RUN), bom]);
      await recarrega(x.page);
      await x.page.click('#colSwitch [data-col="center"]');
      await x.page.click('#centerBuilder');
      s = await tela(x.page);
      const en = locale === 'en-US' && await x.page.evaluate(() => document.documentElement.lang === 'en' || LANG === 'en').catch(() => true);
      ok(`8 ${scheme}/${locale} 390 px: o estado incompatível aparece no Builder`, en ? /^Draft of another edition: not applied\. Sources: /.test(s.draft) && s.actions.join() === 'Discard' : /^Rascunho de outra edition/.test(s.draft), s.draft);
      ok(`8 ${scheme}/${locale} sem rolagem horizontal`, await x.page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
      const c = await x.page.evaluate(() => { const st = e => getComputedStyle(document.querySelector(e)); return [st('#builderDraftText').color, st('#builderView').backgroundColor]; });
      const L = k => { const m = k.match(/[\d.]+/g).slice(0, 3).map(Number).map(v => v / 255).map(v => v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4); return 0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]; };
      const [hi, lo] = [L(c[0]), L(c[1])].sort((p, q) => q - p);
      ok(`8 ${scheme}/${locale} contraste do texto do estado ≥ 4,5`, (hi + 0.05) / (lo + 0.05) >= 4.5, `${c.join(' / ')} = ${((hi + 0.05) / (lo + 0.05)).toFixed(2)}`);
      // Teclado: o botão Descartar alcançável por foco e acionável por Enter (abre a confirmação).
      x.page.removeAllListeners('dialog');
      await x.page.evaluate(() => document.querySelector('#builderDraftActions button').focus());
      await x.page.keyboard.press('Enter');
      await x.page.waitForTimeout(300);
      ok(`8 ${scheme}/${locale} teclado: Enter em Descartar abre a confirmação`, await x.page.evaluate(() => document.querySelector('#askDialog').open));
      await x.page.keyboard.press('Escape');
      await x.page.waitForTimeout(200);
      ok(`8 ${scheme}/${locale} cancelar não apaga o guardado`, JSON.parse(await guardado(x.page)).catalog_sha256 === 'e'.repeat(64));
      await x.page.evaluate(n => localStorage.removeItem(n), NOME(RUN));
      await fecha(ctx, x);
    }
    const chavesEN = await (async () => { const src = fs.readFileSync(path.join(__dirname, '../../web/index.html'), 'utf8');
      return [...src.matchAll(/^\s*'((?:draft|head)\.[A-Za-z]+)': \['((?:[^'\\]|\\.)*)', '((?:[^'\\]|\\.)*)'\]/gm)]; })();
    ok('8 as chaves draft.* e head.* têm PT e EN, com as mesmas variáveis', chavesEN.length > 30 && chavesEN.every(m => m[2] && m[3] && JSON.stringify((m[2].match(/\{\w+\}/g) ?? []).sort()) === JSON.stringify((m[3].match(/\{\w+\}/g) ?? []).sort())), chavesEN.length);
  } catch (e) { console.error(e); ok('a bateria terminou sem exceção', false, e.message); }
  finally {
    ok('nenhum erro na página', todosErros.length === 0, todosErros.join(' | '));
    ok('nada gravado no servidor (escritas abortadas)', todasEscritas.length === 0, todasEscritas.join(','));
    ok('9.2.9 o perfil (project.json) fica igual em bytes', sha(perfil) === perfilAntes);
    await browser.close();
  }
})();

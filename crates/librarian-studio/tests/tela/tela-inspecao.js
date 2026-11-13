// F3: exploração e inspeção (especificação 2.2, T1 a T3, T18, 1.1; critérios 9.1.2 e 9.1.3), na página real,
// num Chrome sem janela, contra o servidor de rodar.js (8841).
// Casos: A, clique num prédio dentro de Explorar (Estrutura, Onde pesa, Inventário): a inspeção abre sobre a
// lista da vista, e "← voltar à lista" devolve a lista como estava (consulta, resultados, filtros, dobras,
// rolagem); B, clique fora de Explorar (Projeto, Régua, Acervo, Léxico): a área não muda e aparece
// "Selecionado: X [Inspecionar]"; Inspecionar leva a Explorar na vista guardada, e Voltar à área de origem;
// C, Inspecionar a partir de cada vista guardada, sem forçar Estrutura; T3, a pilha de relações; D, tela
// estreita; E, PT/EN, teclado, foco, semântica e temas; modo arquivo. Os cliques na cidade são cliques de
// mouse de verdade, num ponto onde o raycast da própria página acha o prédio.
// Nada é enviado a modelo e nada é gravado no servidor: toda escrita é abortada e contada.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium, RENDERER, CHAVE_RENDERER } = require('./ambiente');
const OUT = process.argv[2];
const B = 'http://127.0.0.1:8841', RUN = '20261104-020244';
const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS;
// O gancho: o estado e um ponto da tela onde o raycast da página acha um prédio (o primeiro que não está em `skip`).
const HOOK = "window.__t = { state, studio, t, selectNode, camera, get navigation() { return navigation; },"
  + " pixel(skip = []) { const r = canvas.getBoundingClientRect();"
  + "   for (const b of city.list) { if (skip.includes(b.id) || state.index.get(b.id) == null) continue;"
  + "     const v = b.roof.clone().setY(b.roof.y - 0.4).project(camera); if (Math.abs(v.x) > .6 || Math.abs(v.y) > .6 || v.z > 1) continue;"
  + "     pointer.set(v.x, v.y); if (pick() !== b) continue;"
  + "     const x = r.left + (v.x + 1) / 2 * r.width, y = r.top + (1 - v.y) / 2 * r.height;"
  + "     if (document.elementFromPoint(x, y) !== canvas) continue;"
  + "     return { x, y, id: b.id, label: state.nodes[state.index.get(b.id)].label }; } return null; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 300) : ''}`); if (!pass) process.exitCode = 1; };
const LEITURA = (method, pathname) => method === 'GET' || (method === 'POST' && ['/api/project', '/api/calls'].includes(pathname));
const perfil = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'project.json');
const sha = f => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const NAV_KEYS = 'active_turn,area,center,explore_view,run';

async function abre(browser, { size = { width: 1440, height: 900 }, scheme = 'light' } = {}) {
  const context = await browser.newContext({ viewport: size, colorScheme: scheme, locale: 'pt-BR' });
  const page = await context.newPage(), errors = [], writes = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.route(u => new URL(u).pathname === '/', async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.route(u => new URL(u).pathname.startsWith('/api/'), r => {
    const req = r.request(), { pathname } = new URL(req.url());
    if (LEITURA(req.method(), pathname)) return r.continue();
    writes.push(`${req.method()} ${pathname}`);
    return r.abort();
  });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  const turns = page.waitForResponse(r => r.url().includes('/api/turns'), { timeout: 120000 });
  await page.fill('#projectPath', RENDERER);
  await page.click('#openProject');
  await turns;
  await page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
  // A run com medição gravada (a de tela-4b), escolhida pelo seletor do cabeçalho.
  if (await page.evaluate(() => __t.studio.run) !== RUN) {
    await page.selectOption('#runs', RUN);
    await page.waitForFunction(r => __t.studio.run === r && __t.state.nodes.length && document.querySelector('#hotList').children.length, RUN, { timeout: 120000 });
  }
  await page.waitForTimeout(800); // a câmera termina o enquadramento
  return { page, errors, writes, context, close: () => context.close() };
}
const onde = page => page.evaluate(() => ({
  area: document.querySelector('#areas [aria-current="page"]')?.dataset.area,
  view: document.querySelector('.subnav [aria-current]')?.dataset.view,
  panels: [...document.querySelectorAll('#side .panel')].filter(p => p.offsetParent !== null).map(p => p.dataset.panel),
  views: [...document.querySelectorAll('[data-view-panel]')].filter(p => p.offsetParent !== null).map(p => p.dataset.viewPanel),
  // Lido com tolerância: numa página sem a inspeção do F3, cada verificação falha por si, sem abortar.
  inspecting: (document.querySelector('#inspection')?.offsetParent ?? null) !== null,
  inspector: document.querySelector('#inspector').textContent,
  depth: __t.navigation.inspect?.length ?? 0,
  bar: (document.querySelector('#selBar')?.offsetParent ?? null) !== null, barText: document.querySelector('#selText')?.textContent ?? '',
  back: !document.querySelector('#navBack').disabled, backTitle: document.querySelector('#navBack').title,
  toList: document.querySelector('#inspToList')?.textContent ?? '',
  col: document.querySelector('#app').dataset.col,
  selected: __t.state.selectedNode == null ? null : __t.state.nodes[__t.state.selectedNode].id,
}));
const vai = (page, area) => page.click(`#areas [data-area="${area}"]`);
const vista = (page, view) => page.click(`.subnav [data-view="${view}"]`);
const visivel = (page, sel) => page.evaluate(s => { const e = document.querySelector(s); return Boolean(e && e.offsetParent !== null && e.getClientRects().length); }, sel);
const hash = s => crypto.createHash('sha256').update(s).digest('hex').slice(0, 16);
/// Um clique de mouse num prédio da cidade (não no que está em `skip`).
async function clicaPredio(page, skip = []) {
  const p = await page.evaluate(s => __t.pixel(s), skip);
  if (!p) throw new Error('nenhum prédio clicável no centro da cidade');
  await page.mouse.move(p.x, p.y); await page.waitForTimeout(80);
  await page.mouse.click(p.x, p.y); await page.waitForTimeout(150);
  return p;
}
const navGuardada = async page => { const v = await page.evaluate(k => localStorage.getItem(k), `librarian.nav.v1:${CHAVE_RENDERER}`); return v ? JSON.parse(v) : null; };

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const perfilAntes = sha(perfil);
  const turnosAntes = (await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=20261104-020244`)).json()).length;
  const todasEscritas = [], todosErros = [], idsInspecionados = new Set();
  const fecha = async x => { todasEscritas.push(...x.writes); todosErros.push(...x.errors); await x.close(); };
  try {
    // ================================================================ Caso A: clique dentro de Explorar
    let x = await abre(browser);
    let w;
    // ---- Estrutura: consulta, resultados e rolagem da lista de Encontrar
    await vai(x.page, 'explore'); await vista(x.page, 'structure');
    await x.page.fill('#query', 'Matrix');
    await x.page.waitForTimeout(200);
    const estruturaAntes = await x.page.evaluate(() => { const r = document.querySelector('#results'); r.scrollTop = 120; return { q: document.querySelector('#query').value, html: r.innerHTML, n: r.children.length, top: r.scrollTop, first: r.firstElementChild }; });
    await x.page.evaluate(() => { window.__primeiro = document.querySelector('#results').firstElementChild; });
    let p = await clicaPredio(x.page);
    idsInspecionados.add(p.id);
    w = await onde(x.page);
    ok('A Estrutura: clique num prédio abre a inspeção do prédio (T1)', w.inspecting && w.depth === 1 && w.selected === p.id && w.inspector.includes(p.label.slice(0, 40)), JSON.stringify({ i: w.inspecting, d: w.depth, s: w.selected, p: p.id }));
    ok('A Estrutura: a área e a vista continuam Explorar › Estrutura', w.area === 'explore' && w.view === 'structure' && w.panels.join() === 'explore', JSON.stringify(w));
    ok('A Estrutura: a inspeção fica sobre a lista (a lista sai da tela, mas não é substituída: mesmos elementos)', !w.views.length && await x.page.evaluate(() => document.querySelector('#results').firstElementChild === window.__primeiro && document.querySelector('[data-view-panel="structure"]').contains(document.querySelector('#query'))));
    ok('A Estrutura: o ponto vai para o Voltar (Explorar › Estrutura)', w.back && w.backTitle === 'Voltar a: Explorar › Estrutura', w.backTitle);
    ok('A Estrutura: com a inspeção na tela, a barra "Selecionado" não aparece', !w.bar);
    ok('A Estrutura: "← voltar à lista" diz qual lista', w.toList === '← Voltar à lista (Estrutura)', w.toList);
    await x.page.click('#inspToList');
    w = await onde(x.page);
    const estruturaDepois = await x.page.evaluate(() => { const r = document.querySelector('#results'); return { q: document.querySelector('#query').value, html: r.innerHTML, n: r.children.length, top: r.scrollTop }; });
    ok('A Estrutura: "← voltar à lista" fecha a inspeção e mostra a lista', !w.inspecting && w.views.join() === 'structure' && w.view === 'structure', JSON.stringify(w));
    ok('A Estrutura: consulta, resultados e rolagem da lista ficam como estavam', estruturaDepois.q === estruturaAntes.q && estruturaDepois.html === estruturaAntes.html && estruturaDepois.n === estruturaAntes.n && estruturaDepois.n > 5 && Math.abs(estruturaDepois.top - estruturaAntes.top) <= 1,
      JSON.stringify({ antes: [estruturaAntes.q, estruturaAntes.n, estruturaAntes.top], depois: [estruturaDepois.q, estruturaDepois.n, estruturaDepois.top] }));
    ok('A Estrutura: a seleção continua, e a barra oferece Inspecionar de novo', w.selected === p.id && w.bar && w.barText === `Selecionado: ${p.label.length > 80 ? p.label.slice(0, 80) + '…' : p.label}`, w.barText);
    ok('A Estrutura: "← voltar à lista" não deixa ponto no Voltar: Voltar leva ao ponto anterior (Projeto)', w.backTitle === 'Voltar a: Projeto', w.backTitle);
    // Voltar com a inspeção aberta volta à lista (o ponto do T1).
    p = await clicaPredio(x.page, [p.id]);
    await x.page.click('#navBack');
    w = await onde(x.page);
    ok('A Voltar com a inspeção aberta volta à lista de Estrutura, com a consulta', !w.inspecting && w.views.join() === 'structure' && await x.page.inputValue('#query') === 'Matrix', JSON.stringify(w));
    // Pelo teclado: Enter num resultado abre a inspeção e leva o foco a ela; voltar à lista devolve o foco ao resultado.
    await x.page.focus('#results li:nth-child(3)');
    const escolhido = await x.page.evaluate(() => document.activeElement.textContent);
    const rotulo = await x.page.evaluate(() => document.activeElement.lastChild.textContent);
    await x.page.keyboard.press('Enter');
    w = await onde(x.page);
    const focoNaInspecao = await x.page.evaluate(() => document.activeElement?.id);
    ok('A teclado: Enter num resultado de Encontrar abre a inspeção sobre a lista (T1)', w.inspecting && w.view === 'structure' && w.inspector.includes(rotulo.slice(0, 40)), JSON.stringify({ rotulo, insp: w.inspector.slice(0, 60) }));
    ok('A teclado: o foco vai para o título da inspeção', focoNaInspecao === 'inspTitle', focoNaInspecao);
    await x.page.keyboard.press('Tab');
    const tab1 = await x.page.evaluate(() => ({ id: document.activeElement?.closest('button')?.id ?? document.activeElement?.id, outline: getComputedStyle(document.activeElement).outlineStyle }));
    await x.page.focus('#inspToList');
    const contorno = await x.page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);
    await x.page.keyboard.press('Enter');
    const volta = await x.page.evaluate(() => ({ text: document.activeElement?.textContent, li: document.activeElement?.tagName, shown: document.querySelector('#inspection').offsetParent !== null }));
    ok('A teclado: "← voltar à lista" pelo Enter devolve o foco ao resultado que abriu a inspeção', !volta.shown && volta.li === 'LI' && volta.text === escolhido, JSON.stringify(volta));
    ok('A teclado: os botões da inspeção têm contorno de foco visível', contorno !== 'none' && tab1.outline !== 'none', JSON.stringify({ contorno, tab1 }));

    // ---- Onde pesa: dobras abertas, escolha de Separar, rolagem
    await vista(x.page, 'hot');
    await x.page.waitForFunction(() => document.querySelectorAll('#hotList details.finding').length > 3, null, { timeout: 60000 });
    const quente = await x.page.evaluate(() => {
      const d = [...document.querySelectorAll('#hotList details.finding')];
      d[1].open = true; d[3].open = true;
      const s = document.querySelector('#hotList-counted_no_passes'); if (s) s.open = true;
      const pick = document.querySelector('#hotPick'); pick.value = pick.options[2].value; pick.dispatchEvent(new Event('change'));
      return true;
    });
    await x.page.waitForTimeout(200);
    await x.page.evaluate(() => { const d = [...document.querySelectorAll('#hotList details.finding')]; d[1].open = true; d[3].open = true; document.querySelector('#side').scrollTop = 400; });
    const estadoQuente = () => x.page.evaluate(() => ({ html: document.querySelector('#hotList').innerHTML.length, open: [...document.querySelectorAll('#hotList details')].map(d => d.open).join(''),
      pick: document.querySelector('#hotPick').value, side: document.querySelector('#side').scrollTop, first: document.querySelector('#hotList details.finding') }));
    const quenteAntes = await estadoQuente();
    await x.page.evaluate(() => { window.__primeiroQuente = document.querySelector('#hotList details.finding'); });
    p = await clicaPredio(x.page);
    w = await onde(x.page);
    ok('A Onde pesa: clique num prédio abre a inspeção sobre Onde pesa (a vista continua Onde pesa)', quente && w.inspecting && w.area === 'explore' && w.view === 'hot' && w.selected === p.id && w.toList === '← Voltar à lista (Onde pesa)', JSON.stringify(w));
    ok('A Onde pesa: a lista não é substituída (mesmos elementos, escondidos)', !w.views.length && await x.page.evaluate(() => document.querySelector('#hotList details.finding') === window.__primeiroQuente));
    ok('A Onde pesa: o ponto no Voltar é Explorar › Onde pesa', w.backTitle === 'Voltar a: Explorar › Onde pesa', w.backTitle);
    await x.page.click('#inspToList');
    const quenteDepois = await estadoQuente();
    ok('A Onde pesa: voltar à lista devolve dobras, escolha de Separar, ranking e rolagem como estavam',
      quenteDepois.html === quenteAntes.html && quenteDepois.open === quenteAntes.open && quenteDepois.pick === quenteAntes.pick && quenteAntes.pick && Math.abs(quenteDepois.side - quenteAntes.side) <= 1 && quenteAntes.side > 100,
      JSON.stringify({ html: quenteDepois.html === quenteAntes.html, open: quenteDepois.open === quenteAntes.open, pick: [quenteAntes.pick, quenteDepois.pick], side: [quenteAntes.side, quenteDepois.side] }));
    // "Inspecionar (no lugar)": o nome de uma linha do ranking abre a inspeção sobre Onde pesa.
    const nome = await x.page.evaluate(() => { const a = document.querySelector('#hotList ol a'); a.focus(); return a.textContent; });
    await x.page.keyboard.press('Enter');
    w = await onde(x.page);
    ok('A Onde pesa: o nome de uma linha do ranking inspeciona no lugar (Explorar › Onde pesa)', w.inspecting && w.view === 'hot' && w.inspector.includes(nome.slice(0, 20)), JSON.stringify({ nome, w: w.inspector.slice(0, 60) }));
    await x.page.click('#inspToList');
    ok('A Onde pesa: de volta à lista, o foco volta ao nome que abriu a inspeção', await x.page.evaluate(n => document.activeElement?.textContent === n, nome));

    // ---- Inventário: montado, com filtro, busca e uma dobra aberta
    await vista(x.page, 'inventory');
    await x.page.click('#inventoryBuild');
    await x.page.waitForFunction(() => !document.querySelector('#inventoryFilter').hidden && document.querySelectorAll('#inventoryList details.finding').length, null, { timeout: 120000 });
    const filtro = await x.page.evaluate(() => { const s = document.querySelector('#inventoryFilter'); s.value = [...s.options].find(o => o.value)?.value ?? ''; s.dispatchEvent(new Event('change')); return s.value; });
    await x.page.fill('#inventorySearch', 'color');
    await x.page.waitForTimeout(300);
    await x.page.evaluate(() => { const d = document.querySelector('#inventoryList details.finding'); if (d) d.open = true; document.querySelector('#side').scrollTop = 250; });
    const estadoInv = () => x.page.evaluate(() => ({ html: hashless(document.querySelector('#inventoryList').innerHTML), filter: document.querySelector('#inventoryFilter').value, search: document.querySelector('#inventorySearch').value,
      open: [...document.querySelectorAll('#inventoryList details')].map(d => d.open).join(''), side: document.querySelector('#side').scrollTop, terms: document.querySelector('#inventoryTerms').textContent }));
    await x.page.evaluate(() => { window.hashless = s => s.length + ':' + s.slice(0, 200); });
    const invAntes = await estadoInv();
    p = await clicaPredio(x.page);
    w = await onde(x.page);
    ok('A Inventário: clique num prédio abre a inspeção sobre o Inventário', w.inspecting && w.view === 'inventory' && w.selected === p.id && w.toList === '← Voltar à lista (Inventário)', JSON.stringify(w));
    ok('A Inventário: a vista não foi trocada por Estrutura nem por outra', w.view === 'inventory' && w.area === 'explore');
    await x.page.click('#inspToList');
    const invDepois = await estadoInv();
    ok('A Inventário: voltar à lista devolve filtro, busca, termos, achados, dobras e rolagem',
      JSON.stringify({ ...invDepois, side: 0 }) === JSON.stringify({ ...invAntes, side: 0 }) && Math.abs(invDepois.side - invAntes.side) <= 1 && invAntes.search === 'color' && invAntes.filter === filtro,
      JSON.stringify(Object.fromEntries(Object.keys(invAntes).map(k => [k, k === 'side' ? [invAntes.side, invDepois.side] : invAntes[k] === invDepois[k]]))) + ` filtro ${invAntes.filter} busca ${invAntes.search}`);

    // ---- T3: a pilha de relações e "←"
    p = await clicaPredio(x.page);
    const rel = await x.page.evaluate(() => { const li = document.querySelector('#inspector ul.list li'); return li ? li.textContent : null; });
    if (rel) {
      await x.page.click('#inspector ul.list li');
      w = await onde(x.page);
      const sub = await x.page.evaluate(() => ({ up: document.querySelector('#inspUp').offsetParent !== null, upText: document.querySelector('#inspUp').textContent, edge: __t.state.selectedEdge != null }));
      ok('T3 inspecionar uma relação empilha a inspeção da aresta, com "←" um nível', w.depth === 2 && sub.edge && sub.up && sub.upText.startsWith('← Um nível acima: '), JSON.stringify(sub));
      ok('T3 empilhar não deixa ponto no Voltar', w.backTitle.startsWith('Voltar a: Explorar › Inventário') && !w.backTitle.includes('Inspeção:'), w.backTitle);
      await x.page.click('#inspector dl a');
      w = await onde(x.page);
      ok('T3 uma ponta da aresta empilha mais um nível', w.depth === 3 && w.selected != null);
      await x.page.click('#inspUp');
      w = await onde(x.page);
      ok('T3 "←" volta um nível, e a seleção acompanha a inspeção mostrada (a aresta)', w.depth === 2 && await x.page.evaluate(() => __t.state.selectedEdge != null));
      await x.page.click('#inspUp');
      w = await onde(x.page);
      ok('T3 "←" de novo volta ao nó, e o botão some no primeiro nível', w.depth === 1 && w.selected === p.id && !(await visivel(x.page, '#inspUp')));
      await x.page.click('#inspToList');
    } else ok('T3 o nó clicado tem relações para inspecionar', false, p.label);
    ok('A sem erro na página', x.errors.length === 0, x.errors.join(' | '));
    if (OUT) await x.page.screenshot({ path: `${OUT}/inspecao-lista.png` });
    await fecha(x);

    // ================================================================ Caso B e C: clique fora de Explorar
    x = await abre(browser);
    // A vista guardada de Explorar passa a ser Onde pesa.
    await vai(x.page, 'explore'); await vista(x.page, 'hot');
    await x.page.waitForFunction(() => document.querySelectorAll('#hotList details.finding').length > 3, null, { timeout: 60000 });
    await x.page.evaluate(() => { document.querySelectorAll('#hotList details.finding')[2].open = true; });
    const quenteB = await x.page.evaluate(() => [...document.querySelectorAll('#hotList details')].map(d => d.open).join(''));
    let skip = [];
    for (const area of ['project', 'ruler', 'facts', 'lexicon', 'settings']) {
      await vai(x.page, area);
      if (area === 'lexicon') await x.page.evaluate(() => { document.querySelector('#lexicon').value += '\n'; });
      const antes = await onde(x.page), navAntes = await navGuardada(x.page), lexAntes = await x.page.inputValue('#lexicon');
      p = await clicaPredio(x.page, skip); skip.push(p.id);
      w = await onde(x.page);
      ok(`B ${area}: o clique seleciona o prédio, a área não muda e a inspeção não abre (T2)`, w.selected === p.id && w.area === area && w.panels.join() === area && !w.inspecting && !(await visivel(x.page, '#inspector')), JSON.stringify(w));
      ok(`B ${area}: aparece "Selecionado: X [Inspecionar]"`, w.bar && w.barText === `Selecionado: ${p.label.length > 80 ? p.label.slice(0, 80) + '…' : p.label}` && await visivel(x.page, '#selInspect') && await x.page.textContent('#selInspect') === 'Inspecionar', w.barText);
      ok(`B ${area}: nenhum ponto novo no Voltar e nenhuma navegação gravada`, w.backTitle === antes.backTitle && JSON.stringify(await navGuardada(x.page)) === JSON.stringify(navAntes), `${antes.backTitle} -> ${w.backTitle}`);
      if (area === 'lexicon') ok('B lexicon: o texto editado (não salvo) do Léxico continua', await x.page.inputValue('#lexicon') === lexAntes);
    }
    // Inspecionar, a partir de Régua, com Onde pesa guardada.
    await vai(x.page, 'ruler');
    p = await clicaPredio(x.page, skip); skip.push(p.id);
    await x.page.click('#selInspect');
    w = await onde(x.page);
    ok('B Inspecionar: leva a Explorar na vista guardada (Onde pesa), não a Estrutura', w.area === 'explore' && w.view === 'hot' && (await navGuardada(x.page))?.explore_view === 'hot', JSON.stringify(w));
    ok('B Inspecionar: a inspeção do selecionado aparece sobre a lista de Onde pesa', w.inspecting && w.depth === 1 && w.selected === p.id && w.inspector.includes(p.label.slice(0, 40)) && w.toList === '← Voltar à lista (Onde pesa)' && !w.bar, JSON.stringify(w));
    ok('B Inspecionar: grava o ponto de origem no Voltar (Régua)', w.backTitle === 'Voltar a: Régua', w.backTitle);
    ok('B Inspecionar: o foco vai para a inspeção', await x.page.evaluate(() => document.activeElement?.id) === 'inspTitle');
    ok('B Inspecionar: a navegação guardada tem só área e vista (nada da inspeção)', Object.keys(await navGuardada(x.page)).sort().join() === NAV_KEYS && (await navGuardada(x.page)).area === 'explore');
    await x.page.click('#inspToList');
    w = await onde(x.page);
    ok('B voltar à lista: mostra Onde pesa como estava (dobras)', !w.inspecting && w.views.join() === 'hot' && await x.page.evaluate(() => [...document.querySelectorAll('#hotList details')].map(d => d.open).join('')) === quenteB);
    await x.page.click('#navBack');
    w = await onde(x.page);
    ok('B Voltar: depois de voltar à lista, Voltar recupera a área de origem (Régua)', w.area === 'ruler' && w.panels.join() === 'ruler', JSON.stringify(w));
    // De novo, agora usando Voltar com a inspeção aberta.
    p = await clicaPredio(x.page, skip); skip.push(p.id);
    await x.page.click('#selInspect');
    await x.page.click('#navBack');
    w = await onde(x.page);
    ok('B Voltar com a inspeção aberta volta direto à área de origem (Régua), e a seleção fica', w.area === 'ruler' && w.selected === p.id && w.bar, JSON.stringify(w));
    // Caso C: Inspecionar a partir de cada vista guardada.
    for (const view of ['inventory', 'structure', 'hot']) {
      await vai(x.page, 'explore'); await vista(x.page, view);
      if (view === 'inventory') await x.page.click('#inventoryBuild');
      await vai(x.page, 'facts');
      p = await clicaPredio(x.page, skip); skip.push(p.id);
      await x.page.click('#selInspect');
      w = await onde(x.page);
      ok(`C vista guardada ${view}: Inspecionar abre Explorar › ${view} com a inspeção sobre ela`, w.area === 'explore' && w.view === view && w.inspecting && w.selected === p.id && w.backTitle === 'Voltar a: Acervo', JSON.stringify(w));
      await x.page.click('#inspToList');
      w = await onde(x.page);
      ok(`C vista guardada ${view}: voltar à lista mostra a lista dessa vista`, !w.inspecting && w.views.join() === view);
    }
    // Explorar guarda a inspeção como estado da área: sair e voltar a Explorar a mantém.
    p = await clicaPredio(x.page, skip); skip.push(p.id);
    await vai(x.page, 'project');
    await vai(x.page, 'explore');
    w = await onde(x.page);
    ok('a inspeção fica em Explorar ao sair e voltar à área (pilha de inspeção preservada)', w.inspecting && w.selected === p.id && w.view === 'hot', JSON.stringify(w));
    // Trocar de vista dentro de Explorar mostra a lista dela (a inspeção fica no ponto do Voltar).
    await vista(x.page, 'structure');
    w = await onde(x.page);
    ok('trocar de vista em Explorar mostra a lista da vista nova', !w.inspecting && w.views.join() === 'structure');
    await x.page.click('#navBack');
    w = await onde(x.page);
    ok('Voltar depois de trocar de vista devolve a inspeção sobre Onde pesa', w.inspecting && w.view === 'hot' && w.selected === p.id, JSON.stringify(w));
    // Escape limpa a seleção: a inspeção, que era dela, fecha, sem ponto novo no Voltar.
    const antesEsc = await x.page.evaluate(() => __t.navigation.back.length);
    await x.page.focus('#inspTitle'); await x.page.keyboard.press('Escape');
    w = await onde(x.page);
    ok('Escape limpa a seleção e fecha a inspeção (sem ponto novo no Voltar)', !w.inspecting && w.selected === null && !w.bar && w.views.join() === 'hot' && await x.page.evaluate(() => __t.navigation.back.length) === antesEsc, JSON.stringify(w));
    // Trocar de run fecha a inspeção do grafo anterior.
    p = await clicaPredio(x.page, skip);
    const outra = await x.page.evaluate(r => [...document.querySelectorAll('#runs option')].map(o => o.value).find(v => v !== r), RUN);
    if (outra) {
      await x.page.selectOption('#runs', outra);
      await x.page.waitForFunction(r => __t.studio.run === r && __t.state.nodes.length, outra, { timeout: 120000 });
      await x.page.waitForTimeout(500);
      w = await onde(x.page);
      ok('trocar de run fecha a inspeção do grafo anterior e limpa a barra; a área e a vista ficam', !w.inspecting && w.depth === 0 && !w.bar && w.area === 'explore' && w.view === 'hot', JSON.stringify(w));
    }
    ok('B sem erro na página', x.errors.length === 0, x.errors.join(' | '));
    await fecha(x);

    // ================================================================ Caso D: tela estreita
    x = await abre(browser, { size: { width: 390, height: 844 }, scheme: 'dark' });
    const coluna = () => x.page.evaluate(() => ({ col: document.querySelector('#app').dataset.col, announce: document.querySelector('#announce').textContent,
      labels: [...document.querySelectorAll('#colSwitch [data-col]')].map(b => b.textContent), scroll: document.documentElement.scrollWidth - document.documentElement.clientWidth }));
    await x.page.click('#colSwitch [data-col="center"]');
    await x.page.waitForTimeout(800);
    let c = await coluna();
    p = await clicaPredio(x.page);
    w = await onde(x.page);
    let c2 = await coluna();
    ok('D estreita, fora de Explorar: o clique não troca de coluna nem de área e não anuncia nada', c2.col === 'center' && w.area === 'project' && c2.announce === c.announce && !w.inspecting, JSON.stringify({ c, c2, area: w.area }));
    ok('D estreita: a barra "Selecionado: X [Inspecionar]" fica à vista no cabeçalho, com o Centro na frente', w.bar && await visivel(x.page, '#selInspect'), w.barText);
    await x.page.click('#selInspect');
    w = await onde(x.page); c2 = await coluna();
    ok('D estreita: Inspecionar leva à Navegação, em Explorar › Estrutura (vista guardada), com a inspeção', c2.col === 'nav' && w.area === 'explore' && w.view === 'structure' && w.inspecting && w.selected === p.id, JSON.stringify({ c2, w }));
    ok('D estreita: a troca de coluna é anunciada', c2.announce === 'Coluna mostrada: Navegação: Explorar › Estrutura › Inspeção' && c2.labels[0] === 'Navegação: Explorar › Estrutura › Inspeção', JSON.stringify(c2));
    ok('D estreita: o foco vai para a inspeção', await x.page.evaluate(() => document.activeElement?.id) === 'inspTitle');
    await x.page.click('#inspToList');
    w = await onde(x.page); c2 = await coluna();
    ok('D estreita: voltar à lista mostra a lista e o alternador acompanha', !w.inspecting && w.views.join() === 'structure' && c2.col === 'nav' && c2.labels[0] === 'Navegação: Explorar › Estrutura', JSON.stringify(c2));
    // Em Explorar, com a cidade na frente: a inspeção abre por trás, a coluna fica, a barra leva até ela.
    await x.page.click('#colSwitch [data-col="center"]');
    await x.page.waitForTimeout(400);
    c = await coluna();
    p = await clicaPredio(x.page, [p.id]);
    w = await onde(x.page); c2 = await coluna();
    ok('D estreita, em Explorar: o clique abre a inspeção sobre a lista, sem trocar de coluna nem anunciar', c2.col === 'center' && w.depth === 1 && w.selected === p.id && c2.announce === c.announce, JSON.stringify({ c2, d: w.depth }));
    ok('D estreita, em Explorar: a barra aparece porque a inspeção não está à vista', w.bar);
    const pilha = await x.page.evaluate(() => __t.navigation.back.length);
    await x.page.click('#selInspect');
    w = await onde(x.page); c2 = await coluna();
    ok('D estreita: Inspecionar mostra a inspeção já aberta, sem ponto novo no Voltar (o do clique leva à lista)', c2.col === 'nav' && w.inspecting && !w.bar
      && await x.page.evaluate(() => __t.navigation.back.length) === pilha && w.backTitle === 'Voltar a: Explorar › Estrutura', w.backTitle);
    ok('D estreita: sem rolagem horizontal', c2.scroll <= 1, String(c2.scroll));
    if (OUT) await x.page.screenshot({ path: `${OUT}/inspecao-estreita.png` });
    ok('D sem erro na página', x.errors.length === 0, x.errors.join(' | '));
    await fecha(x);

    // ================================================================ Caso E: idiomas, semântica e temas
    for (const scheme of ['light', 'dark']) {
      x = await abre(browser, { scheme });
      await vai(x.page, 'ruler');
      p = await clicaPredio(x.page);
      const cores = await x.page.evaluate(() => {
        const lum = c => { const [r, g, b] = c.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number).map(v => { v /= 255; return v <= .03928 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4; }); return .2126 * r + .7152 * g + .0722 * b; };
        const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + .05) / (y + .05); };
        const head = getComputedStyle(document.querySelector('#head')).backgroundColor;
        const btn = document.querySelector('#selInspect');
        return { text: ratio(getComputedStyle(document.querySelector('#selText')).color, head), button: ratio(getComputedStyle(btn).color, getComputedStyle(btn).backgroundColor), head: lum(head) };
      });
      ok(`E tema ${scheme === 'light' ? 'claro' : 'escuro'}: barra de seleção legível (contraste ≥ 4,5) sobre o fundo do tema`, cores.text >= 4.5 && cores.button >= 4.5 && (scheme === 'light' ? cores.head > .7 : cores.head < .1), JSON.stringify(cores));
      await x.page.click('#selInspect');
      const corInsp = await x.page.evaluate(() => getComputedStyle(document.querySelector('#inspToList')).color !== getComputedStyle(document.querySelector('#inspToList')).backgroundColor);
      ok(`E tema ${scheme === 'light' ? 'claro' : 'escuro'}: "← voltar à lista" aparece com cor própria`, corInsp);
      if (OUT) await x.page.screenshot({ path: `${OUT}/inspecao-${scheme}.png` });
      if (scheme === 'light') {
        // Semântica: grupo nomeado, região viva, botões com nome sem a seta.
        const sem = await x.page.evaluate(() => ({ role: document.querySelector('#selBar').getAttribute('role'), aria: document.querySelector('#selBar').getAttribute('aria-label'),
          live: document.querySelector('#selText').getAttribute('aria-live'), tags: ['#selInspect', '#inspToList', '#inspUp'].map(s => document.querySelector(s).tagName).join(),
          arrow: document.querySelector('#inspToList span')?.getAttribute('aria-hidden') }));
        ok('E semântica: barra em grupo nomeado ("Seleção"), texto em região viva, ações em <button>, seta escondida do leitor', sem.role === 'group' && sem.aria === 'Seleção' && sem.live === 'polite' && sem.tags === 'BUTTON,BUTTON,BUTTON' && sem.arrow === 'true', JSON.stringify(sem));
        await x.page.click('#inspToList');
        const arvore = JSON.stringify(await x.page.accessibility.snapshot({ interestingOnly: true }));
        ok('E acessibilidade: "Inspecionar" e o texto "Selecionado: …" estão na árvore de acessibilidade', arvore.includes('"Inspecionar"') && arvore.includes('Selecionado: '));
        await x.page.click('#selInspect');
        const arvore2 = JSON.stringify(await x.page.accessibility.snapshot({ interestingOnly: true }));
        ok('E acessibilidade: o botão "Voltar à lista (Estrutura)" tem o nome sem a seta', arvore2.includes('"Voltar à lista (Estrutura)"'), arvore2.match(/Voltar à lista[^"]*/)?.[0]);
        // Teclado: Tab chega a Inspecionar, Enter aciona, o contorno de foco aparece.
        await x.page.click('#inspToList'); await vai(x.page, 'ruler');
        await x.page.evaluate(() => document.querySelector('#headDirty').hidden ? document.querySelector('#runs').focus() : document.querySelector('#headDirty').focus());
        let chegou = false;
        for (let k = 0; k < 6 && !chegou; k++) { await x.page.keyboard.press('Tab'); chegou = await x.page.evaluate(() => document.activeElement?.id === 'selInspect'); }
        const foco = await x.page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);
        await x.page.keyboard.press('Enter');
        w = await onde(x.page);
        ok('E teclado: Tab chega a Inspecionar, com contorno de foco, e Enter abre a inspeção', chegou && foco !== 'none' && w.inspecting && w.area === 'explore', JSON.stringify({ chegou, foco, area: w.area }));
        // EN
        await x.page.click('#inspector ul.list li');
        await x.page.click('[data-lang="en"]');
        const en = await x.page.evaluate(() => ({ toList: document.querySelector('#inspToList').textContent, up: document.querySelector('#inspUp').textContent, title: document.querySelector('#inspTitle').textContent,
          back: document.querySelector('#navBack').title, col: document.querySelector('#colSwitch [data-col="nav"]').textContent }));
        ok('E EN: "← Back to the list (Structure)", "← One level up: …", título "Inspection"', en.toList === '← Back to the list (Structure)' && en.up.startsWith('← One level up: ') && en.title.startsWith('Inspection'), JSON.stringify(en));
        await x.page.click('#inspToList'); await vai(x.page, 'facts');
        const enBar = await x.page.evaluate(() => ({ text: document.querySelector('#selText').textContent, btn: document.querySelector('#selInspect').textContent, aria: document.querySelector('#selBar').getAttribute('aria-label') }));
        ok('E EN: "Selected: …" e "Inspect", grupo "Selection"', /^Selected: /.test(enBar.text) && enBar.btn === 'Inspect' && enBar.aria === 'Selection', JSON.stringify(enBar));
        await x.page.click('#selInspect');
        ok('E EN: Voltar diz para onde volta, em inglês', (await onde(x.page)).backTitle === 'Back to: Facts', (await onde(x.page)).backTitle);
        await x.page.click('[data-lang="pt"]');
        ok('E PT de novo: a barra e a inspeção voltam ao português', (await onde(x.page)).toList === '← Voltar à lista (Estrutura)' && (await onde(x.page)).backTitle === 'Voltar a: Acervo');
      }
      ok(`E tema ${scheme}: sem erro na página`, x.errors.length === 0, x.errors.join(' | '));
      await fecha(x);
    }

    // ================================================================ modo arquivo (sem servidor): a inspeção de sempre
    const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const fp = await ctx.newPage(), ferros = [];
    fp.on('pageerror', e => ferros.push(e.message));
    await fp.goto('file:///' + path.join(RAIZ, 'crates/librarian-studio/web/index.html').replaceAll('\\', '/') + '#debug');
    await fp.waitForFunction(() => window.viewerDebug, null, { timeout: 60000 });
    await fp.setInputFiles('#files', [path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'runs', '20261104-020244', 'graph-v2.json')]);
    await fp.waitForFunction(() => window.viewerDebug.state.nodes.length > 0, null, { timeout: 60000 });
    await fp.fill('#query', 'Matrix'); await fp.waitForTimeout(200);
    await fp.click('#results li:first-child');
    const arq = await fp.evaluate(() => ({ insp: document.querySelector('#inspector').textContent, nav: document.querySelector('#inspNav').offsetParent !== null, bar: document.querySelector('#selBar').offsetParent !== null,
      order: ['#query', '#inspector', '#filesCard'].map(s => Math.round(document.querySelector(s).getBoundingClientRect().top)), results: document.querySelectorAll('#results li').length }));
    ok('modo arquivo: clicar num resultado mostra a inspeção, sem "voltar à lista" nem barra, e a lista continua', arq.insp.length > 20 && !arq.nav && !arq.bar && arq.results > 1, JSON.stringify(arq));
    ok('modo arquivo: a ordem de antes, Encontrar, inspeção e Abrir arquivos', arq.order[0] < arq.order[1] && arq.order[1] < arq.order[2], arq.order.join());
    ok('modo arquivo: sem erro', ferros.length === 0, ferros.join(' | '));
    await ctx.close();
  } finally {
    ok('sem erro de página', todosErros.length === 0, todosErros.join(' | ').slice(0, 400));
    ok('nenhuma escrita no servidor ao selecionar e inspecionar (nada enviado a modelo)', todasEscritas.length === 0, todasEscritas.join(','));
    ok('o perfil (project.json) fica igual em bytes', sha(perfil) === perfilAntes);
    ok('as perguntas da run ficam as mesmas', (await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=20261104-020244`)).json()).length === turnosAntes);
    await browser.close();
  }
})().catch(e => { console.error(e); process.exit(1); });

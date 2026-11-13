// F2: estrutura de navegação do novo frontend (especificação 1.1 a 1.3, 1.6, 5.3, T18), na página real,
// num Chrome sem janela, contra o servidor de rodar.js (8841 e 8842).
// Cobre os critérios do F2: 9.1.1 (quatro áreas e quatro ferramentas com aria-current; Léxico a um clique),
// 9.1.6 (Voltar não altera rascunho nem registros), 9.1.7 (sem estado guardado, abre em Projeto, com o
// estado "sem projeto" e a cidade), e mais: as vistas de Explorar, a seleção fora de Explorar sem mudar a
// área (F3; os casos completos estão em tela-inspecao.js), a navegação guardada por projeto, a conversão
// única de librarian.tab (tabela 1.3), o cabeçalho, a tela estreita com o alternador, PT/EN, temas,
// teclado e o modo arquivo.
// Nada é enviado a modelo e nada é gravado no servidor: toda escrita é abortada e contada.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium, RENDERER, PYTHON, CHAVE_RENDERER } = require('./ambiente');
const OUT = process.argv[2];
const B = 'http://127.0.0.1:8841', P = 'http://127.0.0.1:8842';
const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS;
const HOOK = "window.__t = { state, studio, t, selectNode, openPath, camera, cityGroup, get city() { return city; }, get gathered() { return gathered; }, get pathSeed() { return pathSeed; }, get navigation() { return navigation; }, get userMoved() { return userMoved; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + d : ''}`); if (!pass) process.exitCode = 1; };
const AREAS = ['project', 'explore', 'ruler', 'facts'], TOOLS = ['lexicon', 'settings', 'modelfiles', 'files'];
// Leituras e as duas rotas que só leem com POST: abrir um projeto existente e o fluxograma.
const LEITURA = (method, pathname) => method === 'GET' || (method === 'POST' && ['/api/project', '/api/calls'].includes(pathname));
const perfil = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'project.json');
const sha = f => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');

/// Uma página num contexto novo. `storage` é semeado uma vez, antes da página; `init` roda antes da página
/// em toda carga. Toda requisição de escrita é abortada e anotada em `writes`.
async function abre(browser, base, { size = { width: 1440, height: 900 }, scheme = 'light', storage = null, init = null, project = RENDERER, open = true } = {}) {
  const context = await browser.newContext({ viewport: size, colorScheme: scheme, locale: 'pt-BR' });
  if (storage) await context.addInitScript(s => { if (!sessionStorage.getItem('f2.semeado')) { for (const [k, v] of Object.entries(s)) localStorage.setItem(k, v); sessionStorage.setItem('f2.semeado', '1'); } }, storage);
  if (init) await context.addInitScript(init);
  const page = await context.newPage(), errors = [], writes = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.route(u => new URL(u).pathname === '/', async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.route(u => new URL(u).pathname.startsWith('/api/'), r => {
    const req = r.request(), { pathname } = new URL(req.url());
    if (LEITURA(req.method(), pathname)) return r.continue();
    writes.push(`${req.method()} ${pathname}`);
    return r.abort();
  });
  await page.goto(`${base}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  if (open && project) await abreProjeto(page, project);
  return { page, errors, writes, context, close: () => context.close() };
}
async function abreProjeto(page, project) {
  const turns = page.waitForResponse(r => r.url().includes('/api/turns'), { timeout: 120000 });
  await page.fill('#projectPath', project);
  await page.click('#openProject');
  await turns;
  await page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
  await page.waitForTimeout(300);
}
// O que a navegação mostra agora.
const onde = page => page.evaluate(() => ({
  current: [...document.querySelectorAll('#areas [data-area]')].filter(b => b.getAttribute('aria-current') === 'page').map(b => b.dataset.area),
  currentViews: [...document.querySelectorAll('.subnav [data-view]')].filter(b => b.hasAttribute('aria-current')).map(b => b.dataset.view),
  panels: [...document.querySelectorAll('#side .panel')].filter(p => p.offsetParent !== null).map(p => p.dataset.panel),
  views: [...document.querySelectorAll('[data-view-panel]')].filter(p => p.offsetParent !== null).map(p => p.dataset.viewPanel),
  back: !document.querySelector('#navBack').disabled, backTitle: document.querySelector('#navBack').title,
}));
const guardado = (page, key = null) => page.evaluate(k => {
  const out = {};
  for (const name of Object.keys(localStorage)) if (!k || name === k) out[name] = localStorage.getItem(name);
  return out;
}, key);
const navGuardada = async page => { const v = (await guardado(page, `librarian.nav.v1:${CHAVE_RENDERER}`))[`librarian.nav.v1:${CHAVE_RENDERER}`]; return v ? JSON.parse(v) : null; };
const vai = (page, area) => page.click(`#areas [data-area="${area}"]`);
const visivel = (page, sel) => page.evaluate(s => { const e = document.querySelector(s); return Boolean(e && e.offsetParent !== null && e.getClientRects().length); }, sel);

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const perfilAntes = sha(perfil);
  const turnosAntes = (await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=20261104-020244`)).json()).length;
  const todasEscritas = [], todosErros = [];
  const fecha = async x => { todasEscritas.push(...x.writes); todosErros.push(...x.errors); await x.close(); };
  try {
    // ---------------------------------------------------------------- 9.1.7 ponto de partida
    let x = await abre(browser, B, { open: false });
    let w = await onde(x.page);
    ok('9.1.7 sem estado guardado: abre na área Projeto (aria-current="page")', w.current.join() === 'project' && w.panels.join() === 'project', JSON.stringify(w));
    ok('9.1.7 o estado "sem projeto" aparece, no cabeçalho e em Projeto', await visivel(x.page, '#projectEmpty') && await visivel(x.page, '#headNoProject')
      && /Sem projeto/.test(await x.page.textContent('#projectEmpty')), await x.page.textContent('#projectEmpty'));
    ok('9.1.7 a cidade fica no centro', await visivel(x.page, '#scene') && await visivel(x.page, '#viewport'));
    ok('T18 sem ponto anterior, Voltar fica desabilitado', !w.back);
    ok('a página não grava librarian.tab ao iniciar', !('librarian.tab' in await guardado(x.page)), Object.keys(await guardado(x.page)).join(','));
    await fecha(x);

    // ---------------------------------------------------------------- 9.1.1 áreas e ferramentas
    x = await abre(browser, B);
    w = await onde(x.page);
    ok('abrir o projeto, sem navegação guardada, deixa a área onde estava (Projeto)', w.current.join() === 'project', JSON.stringify(w));
    const rotulos = await x.page.evaluate(() => ({
      areas: [...document.querySelectorAll('#areas [role="group"]')[0].querySelectorAll('[data-area]')].map(b => b.dataset.area),
      tools: [...document.querySelectorAll('#areas [role="group"]')[1].querySelectorAll('[data-area]')].map(b => b.dataset.area),
      grupos: [...document.querySelectorAll('#areas [role="group"]')].map(g => document.getElementById(g.getAttribute('aria-labelledby'))?.textContent),
    }));
    ok('9.1.1 quatro áreas: Projeto, Explorar, Régua, Acervo', rotulos.areas.join() === AREAS.join(), rotulos.areas.join());
    ok('9.1.1 quatro ferramentas: Léxico, Configurações, Modelfiles, Abrir arquivos', rotulos.tools.join() === TOOLS.join(), rotulos.tools.join());
    ok('áreas e ferramentas em grupos separados e nomeados', rotulos.grupos.join() === 'Áreas,Ferramentas', rotulos.grupos.join());
    for (const area of [...AREAS, ...TOOLS]) {
      await vai(x.page, area);
      w = await onde(x.page);
      ok(`9.1.1 ${area}: só ele com aria-current="page" e só o painel dele aparece`, w.current.join() === area && w.panels.join() === area, JSON.stringify(w));
    }
    // O Léxico a um clique de qualquer lugar: de cada área e ferramenta, ele está visível e um clique basta.
    let lexico = true;
    for (const from of [...AREAS, 'settings', 'modelfiles', 'files']) {
      await vai(x.page, from);
      const v = await visivel(x.page, '#areas [data-area="lexicon"]');
      await vai(x.page, 'lexicon');
      lexico &&= v && (await onde(x.page)).current.join() === 'lexicon' && await visivel(x.page, '#lexicon');
    }
    ok('9.1.1 o Léxico se alcança com um clique de qualquer área e ferramenta', lexico);
    // As funcionalidades antigas continuam alcançáveis no lugar novo.
    await vai(x.page, 'settings');
    ok('Configurações: o conteúdo de #settingsBox (Salvar, Exportar, instruções)', await visivel(x.page, '#saveSettings') && await visivel(x.page, '#exportProfile') && await visivel(x.page, '#instructions'));
    await vai(x.page, 'project');
    ok('Projeto: caminho, Abrir, Limpar, Entradas, Gerar e a nota da run', await visivel(x.page, '#openProject') && await visivel(x.page, '#resetStudio') && await visivel(x.page, '#inputs')
      && await visivel(x.page, '#generate') && /fontes/.test(await x.page.textContent('#runNote')));
    await vai(x.page, 'modelfiles');
    await x.page.click('#toolModelfiles');
    ok('Modelfiles: a ferramenta abre o diálogo de Modelfiles (o mesmo do compositor)', await x.page.evaluate(() => document.querySelector('#modelfileDialog').open));
    await x.page.click('#modelfileDialog [data-close]');
    await vai(x.page, 'files');
    ok('Abrir arquivos: no Studio começa fechado, como antes (F0)', await visivel(x.page, '#filesCard') && !(await x.page.evaluate(() => document.querySelector('#filesCard').open)));
    await x.page.click('#filesCard > summary');
    ok('Abrir arquivos: aberto, mostra pasta e arquivos, como antes', await x.page.evaluate(() => document.querySelector('#filesCard').open) && await visivel(x.page, '#provenance'));
    await vai(x.page, 'ruler');
    ok('Régua: comandos e execuções, com a contagem no item', await visivel(x.page, '#commandsList') && /\(\d+\)/.test(await x.page.textContent('#areas [data-area="ruler"]')), await x.page.textContent('#areas [data-area="ruler"]'));
    await vai(x.page, 'facts');
    ok('Acervo: a lista de fatos, com a contagem no item', await visivel(x.page, '#factsList') && /\(\d+\)/.test(await x.page.textContent('#areas [data-area="facts"]')));
    ok('o compositor e a conversa continuam à direita', await visivel(x.page, '#question') && await visivel(x.page, '#turns'));

    // ---------------------------------------------------------------- Explorar e suas vistas
    await vai(x.page, 'explore');
    const vistas = {};
    // A inspeção não é parte de Estrutura desde o F3: é um estado de Explorar sobre a lista (tela-inspecao.js).
    for (const [view, sel] of [['structure', '#query'], ['hot', '#hotCard'], ['inventory', '#inventoryBuild']]) {
      await x.page.click(`.subnav [data-view="${view}"]`);
      w = await onde(x.page);
      vistas[view] = w.currentViews.join() === view && w.views.join() === view && w.current.join() === 'explore' && await visivel(x.page, sel);
    }
    ok('Explorar: Estrutura (Encontrar), Onde pesa e Inventário, uma vista por vez, com aria-current', Object.values(vistas).every(Boolean), JSON.stringify(vistas));
    ok('a vista de Explorar fica guardada para o projeto', (await navGuardada(x.page))?.explore_view === 'inventory', JSON.stringify(await navGuardada(x.page)));

    // ---------------------------------------------------------------- 9.1.6 Voltar
    // Um rascunho guardado e as perguntas da run: Voltar não pode mexer em nenhum dos dois.
    const rascunho = await x.page.evaluate(() => {
      const s = localState.openDraft({ key: __t.studio.project.key, run: __t.studio.run, catalog: __t.state.graph.catalog_sha256 });
      s.update({ f2: 'rascunho da bateria' });
      return { name: s.name, value: localStorage.getItem(s.name) };
    });
    const turnosPagina = await x.page.evaluate(() => JSON.stringify(__t.studio.turns.map(t => t.id)));
    await vai(x.page, 'project');
    await vai(x.page, 'explore'); await x.page.click('.subnav [data-view="hot"]');
    await vai(x.page, 'ruler');
    await vai(x.page, 'lexicon');
    const passos = [];
    for (let i = 0; i < 4; i++) {
      const title = (await onde(x.page)).backTitle;
      await x.page.click('#navBack');
      const o = await onde(x.page);
      passos.push(`${o.current.join()}${o.current.join() === 'explore' ? '/' + o.currentViews.join() : ''} (${title})`);
    }
    ok('T18 Voltar refaz o caminho: Régua, Explorar › Onde pesa, Explorar › Inventário, Projeto', passos.map(p => p.split(' ')[0]).join() === 'ruler,explore/hot,explore/inventory,project', passos.join(' | '));
    ok('Voltar diz para onde volta', /Voltar a: Régua/.test(passos[0]), passos[0]);
    // Os pontos de antes (as idas e vindas de 9.1.1) também estão na pilha: Voltar até o começo.
    let cliques = 0;
    while (!(await x.page.evaluate(() => document.querySelector('#navBack').disabled)) && cliques < 60) { await x.page.click('#navBack'); cliques++; }
    const nada = await x.page.evaluate(() => [document.querySelector('#navBack').disabled, document.querySelector('#navBack').title]);
    ok('T18 Voltar fica desabilitado quando não há mais ponto anterior', nada[0] && cliques < 60 && (await onde(x.page)).current.join() === 'project', `${cliques} cliques; ${nada[1]}`);
    ok('9.1.6 Voltar não altera o rascunho guardado', (await guardado(x.page, rascunho.name))[rascunho.name] === rascunho.value);
    ok('9.1.6 Voltar não altera as perguntas da página', (await x.page.evaluate(() => JSON.stringify(__t.studio.turns.map(t => t.id)))) === turnosPagina);
    await x.page.evaluate(n => localStorage.removeItem(n), rascunho.name);

    // ---------------------------------------------------------------- seleção (F3: regra da 2.2; casos completos em tela-inspecao.js)
    await vai(x.page, 'ruler');
    const voltarAntes = (await onde(x.page)).backTitle;
    const no = await x.page.evaluate(() => { const i = __t.state.nodes.findIndex(n => n.category === 'symbol'); __t.selectNode(i); return { i, label: __t.state.nodes[i].label }; });
    w = await onde(x.page);
    ok('9.1.2 seleção fora de Explorar não muda a área nem grava ponto no Voltar', w.current.join() === 'ruler' && w.panels.join() === 'ruler' && !(await visivel(x.page, '#inspector')) && w.backTitle === voltarAntes, JSON.stringify(w));
    ok('9.1.2 a barra mostra "Selecionado: X [Inspecionar]"', await visivel(x.page, '#selBar') && (await x.page.textContent('#selText')).includes(no.label.slice(0, 40)) && await x.page.textContent('#selInspect') === 'Inspecionar');
    await x.page.click('#selInspect');
    w = await onde(x.page);
    ok('9.1.2 "Inspecionar" leva a Explorar, mostra a inspeção do nó e grava a Régua no Voltar', w.current.join() === 'explore' && await visivel(x.page, '#inspector') && (await x.page.textContent('#inspector')).includes(no.label.slice(0, 40)) && /Régua/.test(w.backTitle), JSON.stringify(w));
    await x.page.click('#navBack');
    ok('Voltar depois de Inspecionar volta à Régua e mantém o nó selecionado', (await onde(x.page)).current.join() === 'ruler' && await x.page.evaluate(() => __t.state.selectedNode) === no.i);

    // ---------------------------------------------------------------- cabeçalho
    const proj = await (await fetch(`${B}/api/project`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ root: RENDERER }) })).json();
    const head = await x.page.evaluate(() => ({ text: document.querySelector('#headProject').textContent, title: document.querySelector('#headProject').title,
      options: [...document.querySelectorAll('#runs option')].map(o => [o.value, o.textContent]), run: document.querySelector('#runs').value, dirty: document.querySelector('#headDirty').offsetParent !== null }));
    ok('cabeçalho: o nome do projeto, com o caminho completo no título', head.text === `Projeto: ${proj.settings.name}` && head.title === proj.settings.root, `${head.text} | ${head.title}`);
    ok('cabeçalho: a run em seletor com id e git curto, igual a /api/project', head.options.length === proj.runs.length
      && head.options.every(([id, txt], k) => id === proj.runs[k].id && txt === `${id} · git ${proj.runs[k].git?.head?.slice(0, 7) ?? 'sem git'}${proj.runs[k].git?.dirty ? ' · com alterações' : ''}`), JSON.stringify(head.options.slice(0, 2)));
    ok('cabeçalho: run limpa, sem o aviso de alterações', !head.dirty && !proj.runs.find(r => r.id === head.run)?.git?.dirty);
    await x.page.click('#headProject');
    ok('cabeçalho: o projeto leva à área Projeto', (await onde(x.page)).current.join() === 'project');
    await fecha(x);

    // Uma run com alterações (resposta de /api/project alterada só nesta página): o aviso e o destino.
    x = await abre(browser, B, { open: false });
    await x.page.route(u => new URL(u).pathname === '/api/project', async r => {
      if (r.request().method() !== 'POST') return r.continue();
      const res = await r.fetch(), data = await res.json();
      data.runs[0].git = { ...(data.runs[0].git ?? {}), dirty: true };
      return r.fulfill({ response: res, json: data });
    });
    await abreProjeto(x.page, RENDERER);
    const suja = await x.page.evaluate(() => ({ shown: document.querySelector('#headDirty').offsetParent !== null, text: document.querySelector('#headDirty').textContent, option: document.querySelector('#runs option').textContent }));
    ok('cabeçalho: run com alterações mostra o aviso escrito e "com alterações" no seletor', suja.shown && suja.text === 'Run com alterações não commitadas: comandos da Régua não rodam nela' && suja.option.endsWith(' · com alterações'), JSON.stringify(suja));
    await x.page.click('#headDirty');
    ok('cabeçalho: o aviso leva à Régua', (await onde(x.page)).current.join() === 'ruler');
    await fecha(x);

    // ---------------------------------------------------------------- navegação guardada por projeto
    x = await abre(browser, B);
    await vai(x.page, 'explore'); await x.page.click('.subnav [data-view="hot"]');
    const nav = await navGuardada(x.page);
    ok('5.3 a navegação fica em librarian.nav.v1:<key>, só com área e vista de Explorar', nav?.area === 'explore' && nav?.explore_view === 'hot' && nav.run === null && nav.center === null
      && Object.keys(nav).sort().join() === 'active_turn,area,center,explore_view,run', JSON.stringify(nav));
    ok('5.3 nada da política do modo vai para a navegação', !/mode|allow_/.test(JSON.stringify(await guardado(x.page, `librarian.nav.v1:${CHAVE_RENDERER}`))));
    const reload = x.page.waitForResponse(r => r.url().includes('/api/turns'), { timeout: 120000 });
    await x.page.reload();
    await x.page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
    await reload; await x.page.waitForTimeout(500);
    w = await onde(x.page);
    ok('T17 recarregar restaura a área e a vista de Explorar do projeto', w.current.join() === 'explore' && w.currentViews.join() === 'hot', JSON.stringify(w));
    ok('T18 depois de restaurar não há ponto anterior: Voltar desabilitado', !w.back);
    ok('sem run escolhida pela pessoa (nenhuma guardada), recarregar abre a mais recente (F9: 2.1)', await x.page.$eval('#runs', s => s.value) === proj.runs[0].id);
    ok('a página não grava librarian.tab ao navegar', !('librarian.tab' in await guardado(x.page)));
    await fecha(x);

    // ---------------------------------------------------------------- isolamento entre projetos: A → B sem navegação → A
    // B é um projeto fictício na mesma origem (o localStorage é por origem): a resposta de POST /api/project
    // para o caminho dele é montada aqui, sem runs, e nada é criado na raiz.
    const RAIZ_B = 'C:/projeto-ficticio-da-bateria/b', CHAVE_B = 'ficticio-b-00000000';
    const navDe = async (page, k) => { const v = (await guardado(page, `librarian.nav.v1:${k}`))[`librarian.nav.v1:${k}`]; return v ?? null; };
    const comB = async page => {
      // As leituras seguintes de B (comandos, fatos…) são respondidas pelas do projeto real: só GET.
      await page.route(u => new URL(u).pathname.startsWith('/api/') && new URL(u).searchParams.get('key') === CHAVE_B, r => {
        if (r.request().method() !== 'GET') return r.fallback();
        const u = new URL(r.request().url()); u.searchParams.set('key', CHAVE_RENDERER);
        return r.fallback({ url: u.toString() });
      });
      await comBProjeto(page);
    };
    const comBProjeto = async page => page.route(u => new URL(u).pathname === '/api/project', async r => {
      const req = r.request();
      if (req.method() !== 'POST' || JSON.parse(req.postData()).root !== RAIZ_B) return r.fallback();
      const res = await r.fetch({ postData: JSON.stringify({ root: RENDERER }) }), data = await res.json();
      return r.fulfill({ json: { ...data, key: CHAVE_B, settings: { ...data.settings, name: 'ficticio-b', root: RAIZ_B }, runs: [] } });
    });
    // Abre um projeto sem passar pela área Projeto, para que o ponto em memória no momento da troca
    // seja o de A (Explorar › Onde pesa): o pior caso para o isolamento.
    const abrePor = async (page, root, key) => {
      await page.evaluate(r => { document.querySelector('#projectPath').value = r; document.querySelector('#openProject').click(); }, root);
      await page.waitForFunction(k => __t.studio.project?.key === k, key, { timeout: 120000 });
      await page.waitForTimeout(800);
    };
    const abreB = page => abrePor(page, RAIZ_B, CHAVE_B);
    // Voltar entre projetos (decisão aprovada depois do F3, especificação 5.3): Voltar percorre só os pontos do
    // projeto aberto; abrir outro projeto começa uma pilha vazia; voltar ao outro só abrindo-o de novo.
    const pilha = page => page.evaluate(() => __t.navigation.back.length);
    // Pedidos de abertura de projeto feitos pela página, com o caminho pedido.
    const aberturas = page => { const lista = []; page.on('request', r => { if (new URL(r.url()).pathname === '/api/project' && r.method() === 'POST') lista.push(JSON.parse(r.postData()).root); }); return lista; };
    for (const bloqueado of [false, true]) {
      const nome = bloqueado ? 'isolamento (armazenamento bloqueado)' : 'isolamento';
      x = await abre(browser, B, bloqueado ? { init: () => Object.defineProperty(window, 'localStorage', { get() { throw new DOMException('negado', 'SecurityError'); } }) } : {});
      const abertos = aberturas(x.page);
      await comB(x.page);
      // Um rascunho guardado de A: nenhuma troca de projeto pode mexer nele.
      const rascunhoA = bloqueado ? null : await x.page.evaluate(() => {
        const s = localState.openDraft({ key: __t.studio.project.key, run: __t.studio.run, catalog: __t.state.graph.catalog_sha256 });
        s.update({ voltar: 'rascunho de A' });
        return { name: s.name, value: localStorage.getItem(s.name) };
      });
      // ---- Cenário A: A em Explorar › Onde pesa, com uma inspeção aberta e pontos no Voltar
      await vai(x.page, 'ruler');
      await vai(x.page, 'explore'); await x.page.click('.subnav [data-view="hot"]');
      await x.page.evaluate(() => __t.selectNode(__t.state.nodes.findIndex(n => n.category === 'symbol')));
      const pilhaA = await pilha(x.page);
      const navA = bloqueado ? null : await navDe(x.page, CHAVE_RENDERER);
      await abreB(x.page);
      w = await onde(x.page);
      ok(`${nome} A: B sem navegação guardada começa em Projeto, não em Explorar › Onde pesa de A`, w.current.join() === 'project' && w.panels.join() === 'project', JSON.stringify(w));
      ok(`${nome} A: em B o Voltar começa desabilitado, com a pilha vazia (A tinha ${pilhaA} pontos)`, pilhaA > 0 && !w.back && await pilha(x.page) === 0 && w.backTitle === 'Nenhum ponto anterior nesta página', w.backTitle);
      ok(`${nome} A: a inspeção de A não passa para B`, await x.page.evaluate(() => __t.navigation.inspect.length === 0 && document.querySelector('#inspection').hidden));
      if (!bloqueado) {
        ok(`${nome} A: abrir B não grava nenhuma navegação para B`, await navDe(x.page, CHAVE_B) === null);
        ok(`${nome} A: a navegação de A fica intacta`, await navDe(x.page, CHAVE_RENDERER) === navA, navA);
      }
      // ---- Cenário B: navegação em B; Voltar percorre só pontos de B e nunca reabre A
      const abertosAntes = abertos.length;
      await vai(x.page, 'ruler'); await vai(x.page, 'facts'); await vai(x.page, 'explore'); await x.page.click('.subnav [data-view="inventory"]');
      const caminho = [];
      while (!(await x.page.evaluate(() => document.querySelector('#navBack').disabled)) && caminho.length < 10) {
        await x.page.click('#navBack');
        const o = await onde(x.page);
        caminho.push(`${o.current.join()}${o.current.join() === 'explore' ? '/' + o.currentViews.join() : ''}`);
      }
      ok(`${nome} B: Voltar em B percorre só os pontos de B e para em Projeto`, caminho.join() === 'explore/structure,facts,ruler,project', caminho.join());
      ok(`${nome} B: o projeto aberto continua B o tempo todo, e Voltar não pede a abertura de nenhum projeto`,
        await x.page.evaluate(k => __t.studio.project.key === k, CHAVE_B) && abertos.length === abertosAntes, `${abertos.length - abertosAntes} aberturas`);
      if (!bloqueado) {
        const navB = JSON.parse(await navDe(x.page, CHAVE_B));
        ok(`${nome} B: a navegação de B é de B (sem a vista Onde pesa de A)`, navB?.area === 'project' && navB.explore_view === 'structure' && await navDe(x.page, CHAVE_RENDERER) === navA, JSON.stringify(navB));
      }
      await vai(x.page, 'ruler');
      // ---- Cenário C: abrir A de novo, pelo caminho normal
      await abrePor(x.page, RENDERER, CHAVE_RENDERER);
      await x.page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
      w = await onde(x.page);
      if (!bloqueado) ok(`${nome} C: abrir A de novo restaura Explorar › Onde pesa de A`, w.current.join() === 'explore' && w.currentViews.join() === 'hot', JSON.stringify(w));
      else ok(`${nome} C: sem armazenamento, A recomeça em Projeto, sem herdar a Régua de B`, w.current.join() === 'project', JSON.stringify(w));
      ok(`${nome} C: em A, o Voltar começa desabilitado (nenhum ponto de B nem os antigos de A)`, !w.back && await pilha(x.page) === 0, w.backTitle);
      await vai(x.page, 'facts');
      await x.page.click('#navBack');
      ok(`${nome} C: depois de um movimento em A, Voltar volta ao ponto de A`, (await onde(x.page)).current.join() === (bloqueado ? 'project' : 'explore') && !(await onde(x.page)).back);
      // ---- Cenário D: preservação
      if (!bloqueado) {
        ok(`${nome} D: a navegação de B continua a de B`, JSON.parse(await navDe(x.page, CHAVE_B))?.area === 'ruler');
        ok(`${nome} D: a navegação guardada de A só mudou pelos movimentos feitos em A`, JSON.parse(await navDe(x.page, CHAVE_RENDERER))?.area === 'explore' && JSON.parse(await navDe(x.page, CHAVE_RENDERER))?.explore_view === 'hot');
        ok(`${nome} D: o rascunho de A fica igual`, (await guardado(x.page, rascunhoA.name))[rascunhoA.name] === rascunhoA.value);
        ok(`${nome} D: nenhum rascunho aparece para B`, !Object.keys(await guardado(x.page)).some(k => k.startsWith(`librarian.draft.v1:${CHAVE_B}`)));
        await x.page.evaluate(n => localStorage.removeItem(n), rascunhoA.name);
        await abreB(x.page);
        ok(`${nome} D: abrir B de novo restaura a Régua de B, com o Voltar vazio`, (await onde(x.page)).current.join() === 'ruler' && !(await onde(x.page)).back);
      }
      ok(`${nome} D: o perfil fica igual em bytes`, sha(perfil) === perfilAntes);
      ok(`${nome}: sem erro na página`, x.errors.length === 0, x.errors.join(' | '));
      await fecha(x);
    }

    // ---------------------------------------------------------------- grafo isolado entre projetos (correção final do F3)
    // B: fictício, sem runs (o mesmo de cima). C: fictício, com runs: responde com o renderer, outra chave e
    // só uma run antiga, de outro catalog_sha256, para provar que o grafo foi trocado. Nada é criado na raiz.
    {
      const RAIZ_C = 'C:/projeto-ficticio-da-bateria/c', CHAVE_C = 'ficticio-c-00000000';
      const projeto = await (await fetch(`${B}/api/project`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ root: RENDERER }) })).json();
      const RUN_C = projeto.runs.at(-1).id, RUN_A = projeto.runs[0].id;
      const catalogo = async run => (await (await fetch(`${B}/api/graph?key=${CHAVE_RENDERER}&run=${run}`)).json()).catalog_sha256;
      const [CAT_A, CAT_C] = [await catalogo(RUN_A), await catalogo(RUN_C)];
      const comC = page => Promise.all([
        page.route(u => new URL(u).pathname.startsWith('/api/') && new URL(u).searchParams.get('key') === CHAVE_C, r => {
          if (r.request().method() !== 'GET') return r.fallback();
          const u = new URL(r.request().url()); u.searchParams.set('key', CHAVE_RENDERER);
          return r.fallback({ url: u.toString() });
        }),
        page.route(u => new URL(u).pathname === '/api/project', async r => {
          const req = r.request();
          if (req.method() !== 'POST' || JSON.parse(req.postData()).root !== RAIZ_C) return r.fallback();
          const res = await r.fetch({ postData: JSON.stringify({ root: RENDERER }) }), data = await res.json();
          return r.fulfill({ json: { ...data, key: CHAVE_C, settings: { ...data.settings, name: 'ficticio-c', root: RAIZ_C }, runs: data.runs.filter(x => x.id === RUN_C) } });
        }),
      ]);
      // O que da página ainda vem de um grafo: dados, cena, camadas, busca, seleção, inspeção, fluxograma, envelope.
      const grafo = page => page.evaluate(() => ({
        nodes: __t.state.nodes.length, graph: Boolean(__t.state.graph), catalog: __t.state.graph?.catalog_sha256 ?? null,
        scene: __t.cityGroup.children.length, buildings: __t.city?.list.length ?? 0,
        labels: document.querySelectorAll('#cityLabels > *, #labels > *').length,
        selected: __t.state.selectedNode ?? __t.state.selectedEdge, matches: __t.state.matches.size,
        query: document.querySelector('#query').value, queryOff: document.querySelector('#query').disabled, results: document.querySelectorAll('#results li').length,
        provenance: document.querySelectorAll('#provenance dd').length, overlay: !document.querySelector('#overlayCard').hidden,
        bar: !document.querySelector('#selBar').hidden, inspect: __t.navigation.inspect.length, inspection: !document.querySelector('#inspection').hidden,
        path: !document.querySelector('#pathView').hidden, seed: __t.pathSeed, pathNodes: document.querySelectorAll('#pathCanvas .node').length,
        gathered: __t.gathered.length, run: __t.studio.run, active: __t.studio.activeTurn, turns: document.querySelectorAll('#turns .turn').length,
        status: document.querySelector('#status').textContent,
      }));
      // A com tudo que vem do grafo: busca, seleção com inspeção, fluxograma aberto, uma declaração juntada.
      const prepara = async page => {
        await vai(page, 'explore'); await page.click('.subnav [data-view="structure"]');
        await page.fill('#query', 'Matrix'); await page.waitForTimeout(200);
        const no = await page.evaluate(() => { const i = __t.state.nodes.findIndex(n => n.category === 'symbol' && n.kind === 'method' && n.label.includes('Matrix')); __t.selectNode(i); return __t.state.nodes[i]; });
        await page.click('#inspector button:has-text("Juntar")');
        await page.click('#inspector button:has-text("Onde é usado?")');
        await page.waitForFunction(() => document.querySelectorAll('#pathCanvas .node').length > 0, null, { timeout: 60000 });
        await vai(page, 'explore');
        return no;
      };
      x = await abre(browser, B);
      await comB(x.page); await comC(x.page);
      const rascunho = await x.page.evaluate(() => {
        const s = localState.openDraft({ key: __t.studio.project.key, run: __t.studio.run, catalog: __t.state.graph.catalog_sha256 });
        s.update({ grafo: 'rascunho de A' });
        return { name: s.name, value: localStorage.getItem(s.name) };
      });
      // ---- Cenário A: projeto com grafo → projeto sem runs
      const noA = await prepara(x.page);
      let g = await grafo(x.page);
      ok('grafo A: em A, busca, seleção, inspeção, fluxograma e envelope funcionam antes da troca', g.nodes > 0 && g.scene > 0 && g.results > 5 && g.selected != null && g.inspection && g.path && g.pathNodes > 0 && g.gathered === 1 && g.catalog === CAT_A, JSON.stringify(g));
      const navA = await navDe(x.page, CHAVE_RENDERER);
      await x.page.evaluate(k => localStorage.setItem(`librarian.nav.v1:${k}`, JSON.stringify({ area: 'ruler' })), CHAVE_B);
      const navB = await navDe(x.page, CHAVE_B);
      await abreB(x.page);
      g = await grafo(x.page);
      ok('grafo A: em B (sem runs) o grafo de A sai: nenhum nó, nenhuma aresta, nenhum grafo', g.nodes === 0 && !g.graph && g.catalog === null && g.run === null, JSON.stringify(g));
      ok('grafo A: a cidade fica vazia: nenhum objeto na cena, nenhum prédio, nenhum rótulo', g.scene === 0 && g.buildings === 0 && g.labels === 0, JSON.stringify(g));
      ok('grafo A: nenhuma seleção, inspeção, barra, busca ou procedência de A', g.selected == null && g.inspect === 0 && !g.inspection && !g.bar && g.matches === 0 && g.query === '' && g.queryOff && g.results === 0 && g.provenance === 0 && !g.overlay, JSON.stringify(g));
      ok('grafo A: o fluxograma de A fecha, com a semente e os pontos descartados', !g.path && g.seed === null && g.pathNodes === 0, JSON.stringify(g));
      ok('grafo A: nem o envelope, nem a pergunta ativa, nem as perguntas de A', g.gathered === 0 && g.active === null && g.turns === 0, JSON.stringify(g));
      ok('grafo A: a cidade diz que o projeto ainda não tem grafo', g.status === 'Nenhum grafo gerado ainda para este projeto.', g.status);
      const texto = await x.page.evaluate(() => document.body.innerText);
      ok('grafo A: o nome do nó de A não aparece em nenhum lugar visível de B', !texto.includes(noA.label.slice(0, 40)) && !texto.includes('renderer-librarian'), noA.label);
      if (OUT) await x.page.screenshot({ path: `${OUT}/grafo-projeto-sem-run.png` });
      // ---- Cenário B: a navegação de B, sem interferência do estado vazio
      w = await onde(x.page);
      ok('grafo B: B recupera a própria navegação (Régua), e o Voltar começa desabilitado', w.current.join() === 'ruler' && !w.back, JSON.stringify(w));
      await vai(x.page, 'explore'); await vai(x.page, 'facts');
      await x.page.click('#navBack');
      ok('grafo B: navegar em B funciona com a cidade vazia (Voltar volta a Explorar de B)', (await onde(x.page)).current.join() === 'explore' && x.errors.length === 0, x.errors.join(' | '));
      await vai(x.page, 'ruler');
      ok('grafo B: a navegação guardada de B não ganhou nada de A', JSON.parse(await navDe(x.page, CHAVE_B)).area === 'ruler' && JSON.parse(await navDe(x.page, CHAVE_B)).explore_view !== 'hot' && JSON.parse(navB).area === 'ruler');
      // PT/EN: a orientação da cidade acompanha o idioma.
      await x.page.click('[data-lang="en"]');
      const en = await x.page.textContent('#status');
      await x.page.click('[data-lang="pt"]');
      ok('grafo B: a orientação da cidade em inglês e de volta em português', en === 'No graph generated yet for this project.' && await x.page.textContent('#status') === 'Nenhum grafo gerado ainda para este projeto.', en);
      // ---- Cenário C: A de novo
      await abrePor(x.page, RENDERER, CHAVE_RENDERER);
      await x.page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
      await x.page.waitForTimeout(500);
      g = await grafo(x.page); w = await onde(x.page);
      ok('grafo C: A recupera a própria navegação (Explorar › Estrutura) com o Voltar vazio', w.current.join() === 'explore' && w.currentViews.join() === 'structure' && !w.back && await navDe(x.page, CHAVE_RENDERER) === navA, JSON.stringify(w));
      ok('grafo C: o grafo de A volta inteiro, com a cidade montada', g.catalog === CAT_A && g.nodes > 0 && g.scene > 0 && g.buildings > 0 && g.run === RUN_A, JSON.stringify(g));
      ok('grafo C: o rascunho de A fica igual', (await guardado(x.page, rascunho.name))[rascunho.name] === rascunho.value);
      ok('grafo C: o perfil fica igual em bytes', sha(perfil) === perfilAntes);
      // ---- Cenário D: A com grafo → C com runs
      await prepara(x.page);
      await abrePor(x.page, RAIZ_C, CHAVE_C);
      await x.page.waitForFunction(r => __t.studio.run === r && __t.state.nodes.length, RUN_C, { timeout: 120000 });
      await x.page.waitForTimeout(500);
      g = await grafo(x.page);
      ok(`grafo D: o grafo de A é trocado pelo de C (catalog_sha256 da run ${RUN_C})`, CAT_A !== CAT_C && g.catalog === CAT_C && g.nodes > 0 && g.scene > 0 && g.buildings > 0, JSON.stringify({ CAT_A, CAT_C, g: g.catalog }));
      ok('grafo D: em C não há seleção, inspeção, barra, fluxograma nem envelope de A', g.selected == null && g.inspect === 0 && !g.inspection && !g.bar && !g.path && g.seed === null && g.gathered === 0, JSON.stringify(g));
      // As funções normais em C: Encontrar, selecionar com inspeção, o fluxograma.
      await vai(x.page, 'explore');
      await x.page.fill('#query', 'Tuple'); await x.page.waitForTimeout(200);
      await x.page.click('#results li:first-child');
      g = await grafo(x.page);
      ok('grafo D: em C, Encontrar e a inspeção funcionam', g.results > 0 && g.selected != null && g.inspection, JSON.stringify(g));
      ok('grafo D: sem erro na página', x.errors.length === 0, x.errors.join(' | '));
      await x.page.evaluate(n => localStorage.removeItem(n), rascunho.name);
      await fecha(x);
    }

    // ---------------------------------------------------------------- conversão única de librarian.tab (1.3)
    const TABELA = { project: ['project', null], lexicon: ['lexicon', null], inventory: ['explore', 'hot'], ruler: ['ruler', null], facts: ['facts', null], map: ['explore', 'structure'] };
    for (const [antigo, [area, view]] of Object.entries(TABELA)) {
      x = await abre(browser, B, { storage: { 'librarian.tab': antigo, 'librarian.project': RENDERER }, open: false });
      await x.page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
      await x.page.waitForTimeout(300);
      w = await onde(x.page);
      const n = await navGuardada(x.page), resto = await guardado(x.page);
      ok(`1.3 tab "${antigo}" vira ${area}${view ? ' › ' + view : ''}, gravado na navegação do projeto, e a chave antiga sai`,
        w.current.join() === area && (!view || w.currentViews.join() === view) && n?.area === area && (!view || n.explore_view === view) && !('librarian.tab' in resto),
        JSON.stringify({ w: w.current, v: w.currentViews, n, tab: resto['librarian.tab'] }));
      await fecha(x);
    }
    // Navegação já guardada prevalece; a chave antiga fica para o primeiro projeto sem navegação.
    x = await abre(browser, B, { storage: { 'librarian.tab': 'facts', 'librarian.project': RENDERER, [`librarian.nav.v1:${CHAVE_RENDERER}`]: JSON.stringify({ area: 'ruler', explore_view: 'inventory' }) }, open: false });
    await x.page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
    await x.page.waitForTimeout(300);
    w = await onde(x.page);
    ok('1.3 com navegação guardada, ela prevalece e não é sobrescrita pela chave antiga', w.current.join() === 'ruler' && (await navGuardada(x.page))?.area === 'ruler', JSON.stringify(w));
    ok('1.3 nesse caso a chave antiga não é convertida nem apagada', (await guardado(x.page))['librarian.tab'] === 'facts');
    await fecha(x);
    // Valor sem destino na tabela: não é convertido nem apagado.
    x = await abre(browser, B, { storage: { 'librarian.tab': 'cidade', 'librarian.project': RENDERER }, open: false });
    await x.page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
    await x.page.waitForTimeout(300);
    ok('1.3 valor sem destino na tabela: a área fica em Projeto e a chave antiga fica como estava', (await onde(x.page)).current.join() === 'project' && (await guardado(x.page))['librarian.tab'] === 'cidade');
    await fecha(x);
    // Gravação da navegação recusada: a chave antiga é preservada, e a página funciona.
    x = await abre(browser, B, { storage: { 'librarian.tab': 'ruler', 'librarian.project': RENDERER }, open: false,
      init: () => { const o = Storage.prototype.setItem; Storage.prototype.setItem = function (k, v) { if (String(k).startsWith('librarian.nav.')) throw new DOMException('cota excedida', 'QuotaExceededError'); return o.call(this, k, v); }; } });
    await x.page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
    await x.page.waitForTimeout(300);
    const falhou = await guardado(x.page);
    ok('1.3 se gravar a navegação falha, librarian.tab fica', falhou['librarian.tab'] === 'ruler' && !Object.keys(falhou).some(k => k.startsWith('librarian.nav.')), Object.keys(falhou).join(','));
    await vai(x.page, 'facts');
    ok('com a navegação sem gravar, a página navega normalmente', (await onde(x.page)).current.join() === 'facts' && x.errors.length === 0, x.errors.join(' | '));
    await fecha(x);

    // ---------------------------------------------------------------- PT/EN
    x = await abre(browser, B);
    await x.page.click('[data-lang="en"]');
    const en = await x.page.evaluate(() => ({ items: [...document.querySelectorAll('#areas [data-area]')].map(b => b.textContent.replace(/\s*\(.*\)$/, '')),
      groups: [...document.querySelectorAll('#areas h2')].map(h => h.textContent), views: [...document.querySelectorAll('.subnav [data-view]')].map(b => b.textContent.replace(/\s*\(.*\)$/, '')),
      back: document.querySelector('#navBack').textContent, head: document.querySelector('#headProject').textContent, aria: document.querySelector('#areas').getAttribute('aria-label') }));
    ok('EN: áreas e ferramentas', en.items.join() === 'Project,Explore,Ruler,Facts,Lexicon,Settings,Modelfiles,Open files', en.items.join());
    ok('EN: grupos, vistas, Voltar, cabeçalho e rótulo da navegação', en.groups.join() === 'Areas,Tools' && en.views.join() === 'Structure,Where the time goes,Inventory'
      && en.back.trim() === '← Back' && en.head.startsWith('Project: ') && en.aria === 'Studio navigation', JSON.stringify(en));
    await x.page.click('[data-lang="pt"]');
    ok('PT de novo: os mesmos itens em português', (await x.page.evaluate(() => [...document.querySelectorAll('#areas [data-area]')].map(b => b.textContent.replace(/\s*\(.*\)$/, '')).join())) === 'Projeto,Explorar,Régua,Acervo,Léxico,Configurações,Modelfiles,Abrir arquivos');

    // ---------------------------------------------------------------- teclado e foco
    await x.page.focus('#areas [data-area="ruler"]');
    await x.page.keyboard.press('Enter');
    const k1 = (await onde(x.page)).current.join();
    await x.page.keyboard.press('Tab');
    const foco = await x.page.evaluate(() => ({ area: document.activeElement?.dataset.area, outline: getComputedStyle(document.activeElement).outlineStyle }));
    await x.page.keyboard.press('Space');
    ok('teclado: Enter e Espaço acionam as áreas, Tab passa ao próximo item', k1 === 'ruler' && foco.area === 'facts' && (await onde(x.page)).current.join() === 'facts', JSON.stringify(foco));
    ok('teclado: o item com foco tem contorno visível', foco.outline !== 'none', foco.outline);
    await x.page.focus('#navBack'); await x.page.keyboard.press('Enter');
    ok('teclado: Voltar pelo teclado', (await onde(x.page)).current.join() === 'ruler');
    await fecha(x);

    // ---------------------------------------------------------------- temas
    for (const scheme of ['light', 'dark']) {
      x = await abre(browser, B, { scheme });
      const cores = await x.page.evaluate(() => {
        const lum = c => { const [r, g, b] = c.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number).map(v => { v /= 255; return v <= .03928 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4; }); return .2126 * r + .7152 * g + .0722 * b; };
        const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + .05) / (y + .05); };
        const cur = document.querySelector('#areas [aria-current="page"]'), other = document.querySelector('#areas [data-area]:not([aria-current])');
        const head = getComputedStyle(document.querySelector('#head')).backgroundColor;
        return { side: lum(getComputedStyle(document.querySelector('#side')).backgroundColor), cur: ratio(getComputedStyle(cur).color, getComputedStyle(cur).backgroundColor),
          other: ratio(getComputedStyle(other).color, getComputedStyle(other.parentElement).backgroundColor), head: lum(head) };
      });
      ok(`tema ${scheme === 'light' ? 'claro' : 'escuro'}: fundo da navegação e do cabeçalho do tema, itens legíveis (contraste ≥ 4,5)`,
        (scheme === 'light' ? cores.side > .7 && cores.head > .7 : cores.side < .1 && cores.head < .1) && cores.cur >= 4.5 && cores.other >= 4.5, JSON.stringify(cores));
      if (scheme === 'dark') {
        await x.page.click('#theme');
        const claro = await x.page.evaluate(() => [document.documentElement.dataset.theme, getComputedStyle(document.querySelector('#head')).backgroundColor]);
        ok('tema: a troca manual muda a navegação também', claro[0] === 'light' && claro[1] === 'rgb(252, 252, 251)', claro.join(' '));
      }
      if (OUT) await x.page.screenshot({ path: `${OUT}/navegacao-${scheme}.png` });
      await fecha(x);
    }

    // ---------------------------------------------------------------- tela estreita (1.1)
    x = await abre(browser, B, { size: { width: 390, height: 844 }, scheme: 'dark' });
    const coluna = () => x.page.evaluate(() => ({ col: document.querySelector('#app').dataset.col,
      shown: ['#areas', '#viewport', '#chat'].filter(s => document.querySelector(s).offsetParent !== null || (s === '#viewport' && document.querySelector(s).getClientRects().length)),
      head: document.querySelector('#head').getBoundingClientRect().height > 0 && document.querySelector('#runs').offsetParent !== null,
      buttons: [...document.querySelectorAll('#colSwitch [data-col]')].map(b => [b.dataset.col, b.getAttribute('aria-pressed'), b.textContent]),
      scroll: document.documentElement.scrollWidth - document.documentElement.clientWidth }));
    let c = await coluna();
    ok('estreita: alternador de três posições, com resumo de cada uma', c.buttons.length === 3 && c.buttons[0][2] === 'Navegação: Projeto' && c.buttons[1][2] === 'Centro: Cidade' && /^Pergunta: /.test(c.buttons[2][2]), JSON.stringify(c.buttons));
    ok('estreita: começa na Navegação, uma coluna por vez', c.col === 'nav' && c.shown.join() === '#areas' && c.buttons[0][1] === 'true', JSON.stringify(c));
    const largura = [c.scroll];
    await x.page.click('#colSwitch [data-col="question"]');
    await x.page.fill('#question', 'texto da bateria');
    await x.page.click('#colSwitch [data-col="center"]');
    await x.page.waitForTimeout(800);
    c = await coluna(); largura.push(c.scroll);
    const cam = await x.page.evaluate(() => [__t.camera.position.x, __t.camera.position.y, __t.camera.position.z, __t.camera.aspect]);
    ok('estreita: Centro mostra só a cidade, com o cabeçalho', c.col === 'center' && c.shown.join() === '#viewport' && c.head, JSON.stringify(c));
    ok('estreita: a cidade montada escondida aparece com a câmera válida', cam.every(Number.isFinite) && cam[3] > 0, cam.join(','));
    await x.page.click('#colSwitch [data-col="question"]');
    c = await coluna(); largura.push(c.scroll);
    ok('estreita: Pergunta mostra só a conversa e o compositor', c.col === 'question' && c.shown.join() === '#chat' && await visivel(x.page, '#question'), JSON.stringify(c));
    ok('estreita: trocar de coluna não descarta nada (texto do compositor)', await x.page.inputValue('#question') === 'texto da bateria');
    ok('estreita: sem rolagem horizontal em nenhuma das três', largura.every(v => v <= 1), largura.join(','));
    // Uma ação que leva a outra coluna: o fluxograma vai para o Centro, e a troca é anunciada.
    await x.page.click('#colSwitch [data-col="nav"]');
    const anuncioAntes = await x.page.textContent('#announce');
    await x.page.evaluate(() => { const i = __t.state.nodes.findIndex(n => n.category === 'symbol' && n.kind === 'function'); __t.openPath(__t.state.nodes[i].id, 'used'); });
    await x.page.waitForTimeout(300);
    c = await coluna();
    // A região de anúncio continua na árvore de acessibilidade com o Centro na frente: não pode
    // estar em display:none nem visibility:hidden, e fica fora da tela (sr-only).
    const anuncio = await x.page.evaluate(() => {
      const a = document.querySelector('#announce'), s = getComputedStyle(a), r = a.getBoundingClientRect();
      return { display: s.display, visibility: s.visibility, live: a.getAttribute('aria-live'), text: a.textContent, rects: a.getClientRects().length, w: r.width, h: r.height, clip: s.clip };
    });
    const arvore = JSON.stringify(await x.page.accessibility.snapshot({ interestingOnly: false }));
    ok('estreita: com o Centro na frente, #announce não está em display:none nem escondido', c.col === 'center' && anuncio.display !== 'none' && anuncio.visibility !== 'hidden' && anuncio.rects > 0, JSON.stringify(anuncio));
    ok('estreita: #announce tem aria-live="polite" e não aparece na tela (1 px, recortado)', anuncio.live === 'polite' && anuncio.w <= 1 && anuncio.h <= 1, JSON.stringify(anuncio));
    ok('estreita: o texto de #announce mudou ao abrir o fluxograma e está na árvore de acessibilidade', anuncio.text !== anuncioAntes && arvore.includes('Coluna mostrada: Centro: Fluxograma'), `${JSON.stringify(anuncioAntes)} -> ${JSON.stringify(anuncio.text)}`);
    ok('estreita: abrir o fluxograma leva ao Centro, mostra "Centro: Fluxograma" e anuncia', c.col === 'center' && c.buttons[1][2] === 'Centro: Fluxograma'
      && await x.page.textContent('#announce') === 'Coluna mostrada: Centro: Fluxograma' && await x.page.getAttribute('#announce', 'aria-live') === 'polite', JSON.stringify(c.buttons));
    await x.page.click('#pathClose');
    // Selecionar na cidade não tira a pessoa da cidade, e fora de Explorar a área não muda (F3).
    await x.page.evaluate(() => __t.selectNode(__t.state.nodes.findIndex(n => n.category === 'symbol')));
    c = await coluna();
    ok('estreita: selecionar um nó com a cidade na frente não troca de coluna nem de área', c.col === 'center' && c.buttons[0][2] === 'Navegação: Projeto' && await visivel(x.page, '#selInspect'), JSON.stringify(c.buttons));
    await x.page.click('#colSwitch [data-col="nav"]');
    await x.page.click('#areas [data-area="explore"]'); await x.page.click('.subnav [data-view="inventory"]');
    c = await coluna(); largura.push(c.scroll);
    ok('estreita: Explorar › Inventário cabe na largura', c.scroll <= 1, String(c.scroll));
    if (OUT) await x.page.screenshot({ path: `${OUT}/navegacao-estreita.png` });
    await fecha(x);

    // ---------------------------------------------------------------- Python (8842), sem medição
    x = await abre(browser, P, { project: PYTHON });
    await vai(x.page, 'explore'); await x.page.click('.subnav [data-view="hot"]');
    await x.page.waitForFunction(() => !document.querySelector('#hotState').hidden, null, { timeout: 60000 });
    ok('Python: Explorar › Onde pesa mostra o estado de Medir tudo', await visivel(x.page, '#hotState') && (await x.page.textContent('#hotState')).length > 10, await x.page.textContent('#hotState'));
    await fecha(x);

    // ---------------------------------------------------------------- modo arquivo (sem servidor)
    const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const fp = await ctx.newPage(), ferros = [];
    fp.on('pageerror', e => ferros.push(e.message));
    await fp.goto('file:///' + path.join(RAIZ, 'crates/librarian-studio/web/index.html').replaceAll('\\', '/') + '#debug');
    await fp.waitForFunction(() => window.viewerDebug, null, { timeout: 60000 });
    const arq = () => fp.evaluate(() => ({ studio: document.body.classList.contains('studio'), nav: document.querySelector('#areas').offsetParent !== null, head: document.querySelector('#navBack').offsetParent !== null,
      shown: ['#query', '#inspector', '#filesCard', '#hotCard', '#inventoryBuild', '#chat', '.subnav', '#colSwitch'].filter(s => document.querySelector(s).offsetParent !== null), open: document.querySelector('#filesCard').open }));
    const a0 = await arq();
    ok('modo arquivo: sem navegação nem cabeçalho do Studio; Encontrar, inspeção e Abrir arquivos (aberto), como antes', !a0.studio && !a0.nav && !a0.head && a0.shown.join() === '#query,#inspector,#filesCard' && a0.open, JSON.stringify(a0));
    const runDir = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'runs', '20261104-020244');
    const turnDir = fs.readdirSync(path.join(runDir, 'turns')).map(d => path.join(runDir, 'turns', d)).find(d => fs.existsSync(path.join(d, 'prepared/search.json')) && fs.existsSync(path.join(d, 'prepared/request.json')));
    await fp.setInputFiles('#files', [path.join(runDir, 'graph-v2.json'), ...(turnDir ? [path.join(turnDir, 'prepared/search.json'), path.join(turnDir, 'prepared/request.json')] : [])]);
    await fp.waitForFunction(() => window.viewerDebug.state.nodes.length > 0, null, { timeout: 60000 });
    await fp.waitForTimeout(500);
    const a1 = await fp.evaluate(() => ({ nodes: window.viewerDebug.state.nodes.length, overlay: !document.querySelector('#overlayCard').hidden && document.querySelector('#overlayCard').offsetParent !== null, hits: document.querySelectorAll('#overlayHits li').length }));
    ok('modo arquivo: abre o grafo e mostra o #overlayCard com a busca', a1.nodes > 0 && (!turnDir || (a1.overlay && a1.hits > 0)), JSON.stringify(a1));
    ok('modo arquivo: sem erro', ferros.length === 0, ferros.join(' | '));
    ok('modo arquivo: nenhuma chave de navegação gravada', !(await fp.evaluate(() => Object.keys(localStorage))).some(k => k.startsWith('librarian.nav.') || k === 'librarian.tab'));
    await ctx.close();
  } finally {
    ok('sem erro de página', todosErros.length === 0, todosErros.join(' | ').slice(0, 400));
    ok('nenhuma escrita no servidor ao navegar (nada enviado a modelo)', todasEscritas.length === 0, todasEscritas.join(','));
    ok('o perfil (project.json) fica igual em bytes', sha(perfil) === perfilAntes);
    ok('as perguntas da run ficam as mesmas', (await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=20261104-020244`)).json()).length === turnosAntes);
    await browser.close();
  }
})().catch(e => { console.error(e); process.exit(1); });

// F4: vistas do espaço central (especificação 1.1, 1.4, 2.3, 5.3, T5, T14, T17, T18; critério 9.1.4), na
// página real, num Chrome sem janela, contra o servidor de rodar.js (8841).
// Casos: a barra de vistas (Cidade sempre, o Fluxograma enquanto aberto, com título, estado e fechar); a
// cidade como base e estado inicial; abrir o fluxograma pela inspeção; ir à Cidade e voltar preservando
// câmera, seleção, inspeção, semente, modo, profundidade, opções, escolhas e o próprio desenho; abrir de novo
// o mesmo conteúdo; substituir e fechar com confirmação quando há escolhas de homônimos (cancelar não muda
// nada); a resposta antiga ou de outra run ignorada (sameRun) e a troca de run fechando o fluxograma (T14);
// as perguntas por ponto (askPath) que não entram na conversa de outra run; a vista central guardada em
// librarian.nav.v1:<key> (center.view) e restaurada só quando dá para refazer (T17); o isolamento entre
// projetos (com e sem runs); tela estreita; PT e EN; temas; teclado, foco e semântica; o modo arquivo.
// Nada é enviado a modelo e nada é gravado no servidor: toda escrita é abortada e contada, e as duas que o
// caso de askPath precisa (PUT /api/project e POST /api/path-questions) são respondidas aqui, sem servidor.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium, RENDERER, CHAVE_RENDERER } = require('./ambiente');
const OUT = process.argv[2];
const B = 'http://127.0.0.1:8841', RUN = '20261104-020244', OUTRA = '20261104-014710';
const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS;
const HOOK = "window.__t = { state, studio, t, STRINGS, selectNode, openPath, camera, get navigation() { return navigation; }, get pathSeed() { return pathSeed; },"
  + " get pathMode() { return pathMode; }, get pathChosen() { return pathChosen; }, get hot() { return hot; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 300) : ''}`); if (!pass) process.exitCode = 1; };
const LEITURA = (method, pathname) => method === 'GET' || (method === 'POST' && ['/api/project', '/api/calls'].includes(pathname));
const perfil = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'project.json');
const sha = f => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const NAV = `librarian.nav.v1:${CHAVE_RENDERER}`;
// Predicados nomeados: o Playwright só remove (unroute) uma rota pela mesma referência.
const rota = p => { const f = u => new URL(u).pathname === p; return f; };
const USED_BY = rota('/api/used-by'), PATH_Q = rota('/api/path-questions'), SEND = rota('/api/send'), PROJECT = rota('/api/project');
// B: projeto fictício sem runs, na mesma origem; a resposta de POST /api/project é montada aqui.
const RAIZ_B = 'C:/projeto-ficticio-da-bateria/b', CHAVE_B = 'ficticio-b-00000000', NAV_B = `librarian.nav.v1:${CHAVE_B}`;

/// Uma página num contexto novo, com o renderer aberto na run com medição. `storage` é semeado uma vez.
async function abre(browser, { size = { width: 1440, height: 900 }, scheme = 'light', storage = null, run = RUN } = {}) {
  const context = await browser.newContext({ viewport: size, colorScheme: scheme, locale: 'pt-BR' });
  if (storage) await context.addInitScript(s => { if (!sessionStorage.getItem('f4.semeado')) { for (const [k, v] of Object.entries(s)) localStorage.setItem(k, v); sessionStorage.setItem('f4.semeado', '1'); } }, storage);
  const page = await context.newPage(), errors = [], writes = [], reads = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('request', r => { const p = new URL(r.url()).pathname; if (p === '/api/calls' || p === '/api/used-by') reads.push(p); });
  await page.route(u => new URL(u).pathname === '/', async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.route(u => new URL(u).pathname.startsWith('/api/'), r => {
    const req = r.request(), { pathname } = new URL(req.url());
    if (LEITURA(req.method(), pathname)) return r.continue();
    writes.push(`${req.method()} ${pathname}`);
    return r.abort();
  });
  // B: as leituras dele são respondidas pelas do projeto real (só GET), e o POST /api/project, sem runs.
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
  await abrePor(page, RENDERER, CHAVE_RENDERER);
  await page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
  if (run && await page.evaluate(() => __t.studio.run) !== run) await trocaRun(page, run);
  await page.waitForTimeout(800); // a câmera termina o enquadramento
  return { page, errors, writes, reads, context, close: () => context.close() };
}
async function abrePor(page, root, key) {
  await page.evaluate(r => { document.querySelector('#projectPath').value = r; document.querySelector('#openProject').click(); }, root);
  await page.waitForFunction(k => __t.studio.project?.key === k && !document.querySelector('#openProject').disabled, key, { timeout: 120000 });
  await page.waitForTimeout(800);
}
async function trocaRun(page, run) {
  // A troca termina quando as perguntas da run nova chegam (studio.run muda antes do grafo e da conversa).
  const turns = page.waitForResponse(r => r.url().includes('/api/turns') && r.url().includes(`run=${run}`), { timeout: 120000 });
  await page.selectOption('#runs', run);
  await turns;
  await page.waitForFunction(r => __t.studio.run === r && __t.state.nodes.length && __t.state.graph, run, { timeout: 120000 });
  await page.waitForTimeout(600);
}
// O que a barra e o centro mostram agora.
const centro = page => page.evaluate(() => {
  const bar = document.querySelector('#centerBar'), r = e => e.getBoundingClientRect();
  const shown = e => Boolean(e && e.offsetParent !== null && e.getClientRects().length);
  // Lido com tolerância: numa página sem a barra (antes do F4), cada verificação falha por si.
  if (!bar) return { bar: false, buttons: [], tab: false, path: shown(document.querySelector('#pathView')), view: null, seed: __t.pathSeed, chosen: Object.keys(__t.pathChosen).length,
    nodes: document.querySelectorAll('#pathCanvas .node').length, col: document.querySelector('#colSwitch [data-col="center"]').textContent };
  return {
    bar: shown(bar), aria: bar.getAttribute('aria-label'), tag: bar.tagName,
    buttons: [...bar.querySelectorAll('button')].filter(shown).map(b => ({ id: b.id, text: b.textContent, current: b.getAttribute('aria-current'), label: b.getAttribute('aria-label') })),
    tab: shown(document.querySelector('#centerPathTab')), title: document.querySelector('#centerPathTitle').textContent, state: document.querySelector('#centerPathState').textContent,
    path: shown(document.querySelector('#pathView')), pathTop: r(document.querySelector('#pathView')).top, barBottom: r(bar).bottom,
    statusTop: r(document.querySelector('#status')).top, view: __t.navigation.center, seed: __t.pathSeed, mode: __t.pathMode, chosen: Object.keys(__t.pathChosen).length,
    nodes: document.querySelectorAll('#pathCanvas .node').length, note: document.querySelector('#pathNote').textContent,
    col: document.querySelector('#colSwitch [data-col="center"]').textContent,
  };
});
const nav = async (page, k = NAV) => { const v = await page.evaluate(k => localStorage.getItem(k), k); return v; };
const navObj = async (page, k = NAV) => { const v = await nav(page, k); return v ? JSON.parse(v) : null; };
const vai = (page, area) => page.click(`#areas [data-area="${area}"]`);
const desenhado = page => page.waitForFunction(() => __t.pathSeed != null && !/carregando|loading/.test(document.querySelector('#centerPathState').textContent)
  && document.querySelector('#centerPathState').textContent !== '', null, { timeout: 60000 });
// Seleciona em Explorar › Estrutura (a inspeção abre, F3) e usa o botão da inspeção (T5).
async function abreFluxo(page, id, mode = 'used') {
  if (await page.evaluate(() => __t.navigation.area) !== 'explore') await vai(page, 'explore');
  await page.evaluate(id => __t.selectNode(__t.state.index.get(id)), id);
  await page.click(`#inspector button:has-text("${mode === 'calls' ? 'O que ele chama?' : 'Onde é usado?'}")`);
  await desenhado(page);
}
const camera = page => page.evaluate(() => [...__t.camera.position.toArray(), ...__t.camera.quaternion.toArray()]);
const igual = (a, b) => a.length === b.length && a.every((v, i) => Math.abs(v - b[i]) < 1e-6);
// A câmera lida só depois de parar (o enquadramento e o amortecimento dos controles terminam).
async function cameraParada(page) {
  let a = await camera(page);
  for (let i = 0; i < 40; i++) { await page.waitForTimeout(250); const b = await camera(page); if (igual(a, b)) return b; a = b; }
  return a;
}
const dialogo = page => page.evaluate(() => ({ open: document.querySelector('#askDialog').open, title: document.querySelector('#askTitle').textContent,
  message: document.querySelector('#askMessage').textContent, ok: document.querySelector('#askOk').textContent }));
const cancela = page => page.click('#askDialog .row:last-child [data-close]');
const confirma = page => page.click('#askOk');
// Contraste WCAG entre duas cores computadas (rgb/rgba ou color(srgb …)).
function cor(s) {
  let m = s.match(/rgba?\(([^)]+)\)/);
  if (m) { const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number); return p.slice(0, 3).map(v => v / 255); }
  m = s.match(/color\(srgb ([^)]+)\)/);
  if (m) return m[1].split(/[ /]+/).filter(Boolean).slice(0, 3).map(Number);
  return null;
}
function contraste(a, b) {
  const L = c => { const [r, g, bl] = c.map(v => v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4); return 0.2126 * r + 0.7152 * g + 0.0722 * bl; };
  const [x, y] = [L(cor(a)), L(cor(b))].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const perfilAntes = sha(perfil);
  const turnosAntes = (await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`)).json()).length;
  const todasEscritas = [], todosErros = [];
  const fecha = async x => { todasEscritas.push(...x.writes); todosErros.push(...x.errors); await x.close(); };
  try {
    // ================================================================ 1. estado inicial e barra
    let x = await abre(browser);
    let c = await centro(x.page);
    ok('1 a barra de vistas está presente no Studio, como navegação nomeada "Vistas do centro"', c.bar && c.tag === 'NAV' && c.aria === 'Vistas do centro', JSON.stringify(c));
    ok('1 estado inicial: só "Cidade", marcada como a vista atual; nenhuma outra vista aberta', c.buttons.length === 1 && c.buttons[0].text === 'Cidade' && c.buttons[0].current === 'true' && !c.tab && !c.path && c.view === 'city' && c.seed === null, JSON.stringify(c.buttons));
    ok('1 a cidade continua a base: o canvas ocupa o centro inteiro, sob a barra', await x.page.evaluate(() => { const v = document.querySelector('#viewport').getBoundingClientRect(), s = document.querySelector('#scene').getBoundingClientRect(); return Math.abs(v.width - s.width) < 1 && Math.abs(v.height - s.height) < 1; }));
    ok('1 a barra não cobre o estado da cidade (#status abaixo dela)', c.statusTop >= c.barBottom, `${c.statusTop} ${c.barBottom}`);
    // Fechamento do F9: dois comportamentos, cada um conferido à parte e sem alternativa. Abrir não grava nada; escolher a run pelo
    // seletor (F9) grava só `run`. A bateria abre o projeto e escolhe RUN pelo seletor porque a run mais recente não é RUN.
    const recente = await x.page.evaluate(() => __t.studio.project.runs[0].id);
    ok('1 a bateria escolheu RUN pelo seletor (a mais recente é outra), e isso grava só a run: nem área, nem vista de Explorar, nem a vista central', recente !== RUN
      && JSON.stringify(JSON.parse(await nav(x.page) ?? 'null')) === JSON.stringify({ run: RUN, area: null, explore_view: null, center: null, active_turn: {} }), `${recente} ${await nav(x.page)}`);
    {
      const y = await abre(browser, { run: null });
      ok('1 abrir o projeto sem escolher run não grava a navegação (nem a vista central)', await nav(y.page) === null && await y.page.evaluate(() => __t.studio.run) === recente, await nav(y.page));
      await fecha(y);
    }
    // F5 acrescentou o Builder à barra (escondido sem rascunho); o F10, a Revisão (escondida sem Revisão aberta). A Comparação continua fora (F12).
    ok('1 sem botão de Comparação (F12); o Builder (F5) só aparece com rascunho e a Revisão (F10) só com uma Revisão aberta', await x.page.evaluate(() => document.querySelectorAll('#centerBar [data-center]').length === 4 && ![...document.querySelectorAll('#centerBar [data-center]')].some(b => /compare/.test(b.dataset.center)) && document.querySelector('#centerBuilderTab').hidden && document.querySelector('#centerReviewTab').hidden));

    // ================================================================ 2. abrir o fluxograma pela inspeção (T5)
    const sementes = await x.page.evaluate(() => [...new Set(__t.hot.rows.map(r => r.symbol).filter(s => s && __t.state.index.get(s) != null))]);
    const S1 = sementes[0], S2 = sementes[1];
    await vai(x.page, 'explore');
    await x.page.evaluate(id => __t.selectNode(__t.state.index.get(id)), S1);
    const camAntes = await cameraParada(x.page);
    await x.page.click('#inspector button:has-text("Onde é usado?")');
    await desenhado(x.page);
    c = await centro(x.page);
    const nome1 = await x.page.evaluate(id => __t.state.nodes[__t.state.index.get(id)].label, S1);
    ok('2 "Onde é usado?" abre o Fluxograma na frente, com a aba marcada como atual e a Cidade não', c.path && c.tab && c.view === 'path' && c.buttons.find(b => b.id === 'centerPath')?.current === 'true' && c.buttons.find(b => b.id === 'centerCity')?.current === null, JSON.stringify(c.buttons));
    ok('2 a aba tem título com o nome da semente', c.title.startsWith('Fluxograma: ') && nome1.endsWith(c.title.slice('Fluxograma: '.length)), c.title);
    ok('2 a aba tem o estado do desenho, igual ao que está desenhado', (c.nodes > 0 && c.state === `· ${c.nodes} pontos`) || (c.nodes === 0 && c.state === '· vazio'), `${c.state} ${c.nodes}`);
    ok('2 a aba tem o botão de fechar com nome acessível', c.buttons.some(b => b.id === 'centerPathClose' && b.label === 'Fechar o fluxograma'));
    ok('2 o fluxograma abre abaixo da barra, sem cobri-la', Math.abs(c.pathTop - c.barBottom) <= 1, `${c.pathTop} ${c.barBottom}`);
    ok('2 o movimento grava a vista central na navegação do projeto (center.view = path)', (await navObj(x.page))?.center?.view === 'path', await nav(x.page));
    const cam2 = await camera(x.page);
    ok('2 a câmera não muda ao abrir o fluxograma', igual(cam2, camAntes), `${camAntes} -> ${cam2}`);
    ok('2 a inspeção (F3) continua aberta em Explorar', await x.page.evaluate(() => __t.navigation.area === 'explore' && __t.navigation.inspect.length === 1));

    // ================================================================ 3. ir à Cidade e voltar (9.1.4)
    await x.page.selectOption('#pathDepth', '3'); await desenhado(x.page);
    await x.page.uncheck('#pathNoTests'); await desenhado(x.page);
    await x.page.evaluate(() => { const c = document.querySelector('#pathCanvas'); c.scrollTop = 40; c.scrollLeft = 30; window.__svg = c.firstElementChild; });
    const rolagem = await x.page.evaluate(() => [document.querySelector('#pathCanvas').scrollTop, document.querySelector('#pathCanvas').scrollLeft]);
    const lidas = x.reads.length, selecao = await x.page.evaluate(() => __t.state.selectedNode);
    await x.page.click('#centerCity');
    c = await centro(x.page);
    ok('3 "Cidade" esconde o fluxograma, que continua aberto na barra', !c.path && c.tab && c.view === 'city' && c.seed === S1 && c.buttons.find(b => b.id === 'centerCity')?.current === 'true', JSON.stringify(c));
    const cam3 = await camera(x.page);
    ok('3 na cidade, a câmera é a de antes do fluxograma (9.1.4)', igual(cam3, camAntes), `${camAntes} -> ${cam3}`);
    ok('3 a seleção continua a mesma na cidade (seleção única, 1.4)', await x.page.evaluate(() => __t.state.selectedNode) === selecao && selecao != null);
    ok('3 a navegação guarda center.view = city', (await navObj(x.page))?.center?.view === 'city');
    await x.page.click('#centerPath');
    c = await centro(x.page);
    const volta = await x.page.evaluate(() => ({ same: document.querySelector('#pathCanvas').firstElementChild === window.__svg, depth: document.querySelector('#pathDepth').value,
      noTests: document.querySelector('#pathNoTests').checked, scroll: [document.querySelector('#pathCanvas').scrollTop, document.querySelector('#pathCanvas').scrollLeft] }));
    ok('3 voltar ao Fluxograma pela barra mostra o mesmo desenho, sem pedir de novo ao servidor', c.path && volta.same && x.reads.length === lidas, JSON.stringify({ volta, pedidos: x.reads.length - lidas }));
    ok('3 semente, modo, profundidade e opções preservados', c.seed === S1 && c.mode === 'used' && volta.depth === '3' && volta.noTests === false, JSON.stringify(volta));
    ok('3 a rolagem do fluxograma preservada', volta.scroll[0] === rolagem[0] && volta.scroll[1] === rolagem[1], JSON.stringify({ volta: volta.scroll, rolagem }));
    ok('3 trocar de vista não grava ponto no Voltar nem muda a área ou a inspeção', await x.page.evaluate(() => __t.navigation.area === 'explore' && __t.navigation.inspect.length === 1) && (await x.page.getAttribute('#navBack', 'title')) !== null);
    const cam4 = await camera(x.page);
    ok('3 a câmera não mudou com as idas e vindas', igual(cam4, camAntes), `${camAntes} -> ${cam4}`);

    // ---- abrir de novo o mesmo conteúdo (mesma semente e modo) com a cidade na frente: traz o aberto, como estava
    await x.page.click('#centerCity');
    await x.page.click('#inspector button:has-text("Onde é usado?")');
    c = await centro(x.page);
    ok('3 "Onde é usado?" da mesma semente traz o fluxograma aberto, sem refazer nem perder a profundidade', c.path && x.reads.length === lidas && await x.page.inputValue('#pathDepth') === '3'
      && await x.page.evaluate(() => document.querySelector('#pathCanvas').firstElementChild === window.__svg), `pedidos ${x.reads.length - lidas}`);

    // ---- resposta antiga descartada: outra semente pedida antes da primeira chegar
    let solta;
    const segura = new Promise(z => { solta = z; });
    let atrasou = false;
    await x.page.route(USED_BY, async r => { if (!atrasou) { atrasou = true; await segura; } return r.fallback(); });
    await x.page.evaluate(id => { __t.openPath(id, 'used'); }, S2);
    await x.page.waitForTimeout(200);
    c = await centro(x.page);
    ok('3 enquanto desenha, a aba diz "carregando…"', c.state === '· carregando…', c.state);
    await x.page.evaluate(id => { __t.openPath(id, 'calls'); }, S2);
    await desenhado(x.page);
    const depoisDaSegunda = await centro(x.page);
    solta(); await x.page.waitForTimeout(1500);
    c = await centro(x.page);
    ok('3 a resposta que chega depois de outra ser pedida é ignorada (o desenho é o da última)', c.mode === 'calls' && c.seed === S2 && c.nodes === depoisDaSegunda.nodes && c.state === depoisDaSegunda.state && /^O que ele chama/.test(await x.page.textContent('#pathTitle')), JSON.stringify({ c, d: depoisDaSegunda.state }));
    await x.page.unroute(USED_BY);

    // ================================================================ 4. escolhas de homônimos: substituir e fechar
    // Uma semente cujo fluxograma de chamadas tem uma chamada entre homônimos ainda sem escolha.
    let comHomonimo = null;
    await x.page.uncheck('#pathRan').catch(() => {});
    for (const s of sementes.slice(0, 25)) {
      await x.page.evaluate(id => __t.openPath(id, 'calls'), s); await desenhado(x.page);
      if (await x.page.evaluate(() => document.querySelectorAll('#pathCanvas polyline.edge.ambiguous').length)) { comHomonimo = s; break; }
    }
    ok('4 há uma semente com chamada entre homônimos nesta run', comHomonimo != null);
    if (comHomonimo) {
      await x.page.evaluate(() => document.querySelector('#pathCanvas polyline.edge.ambiguous').dispatchEvent(new MouseEvent('click', { bubbles: true })));
      await x.page.click('#pathDetail button:has-text("Seguir por este")');
      await desenhado(x.page);
      c = await centro(x.page);
      ok('4 uma escolha de homônimo feita na tela', c.chosen === 1 && c.seed === comHomonimo, JSON.stringify(c));
      // Ir à cidade e voltar preserva a escolha.
      await x.page.click('#centerCity'); await x.page.click('#centerPath');
      ok('4 ir à Cidade e voltar preserva a escolha', (await centro(x.page)).chosen === 1);
      // Substituir: outra semente pede confirmação; cancelar não muda nada.
      const outra = sementes.find(s => s !== comHomonimo);
      const antesReplace = await centro(x.page), lidasR = x.reads.length;
      await x.page.evaluate(id => { __t.openPath(id, 'used'); }, outra);
      await x.page.waitForFunction(() => document.querySelector('#askDialog').open);
      let d = await dialogo(x.page);
      ok('4 abrir outro conteúdo com escolhas pede confirmação, com o nome e o número de escolhas', d.title === 'Substituir o fluxograma?' && /tem escolhas de homônimos \(1\)/.test(d.message) && d.ok === 'Substituir', JSON.stringify(d));
      await cancela(x.page); await x.page.waitForTimeout(300);
      c = await centro(x.page);
      ok('4 cancelar a substituição não muda nada (semente, modo, escolhas, desenho, nenhum pedido)', c.seed === antesReplace.seed && c.mode === antesReplace.mode && c.chosen === 1 && c.nodes === antesReplace.nodes && c.path && x.reads.length === lidasR, JSON.stringify(c));
      // Fechar com escolhas: confirmação; cancelar não fecha; o foco volta ao botão que abriu o diálogo.
      await x.page.focus('#centerPathClose'); await x.page.keyboard.press('Enter');
      await x.page.waitForFunction(() => document.querySelector('#askDialog').open);
      d = await dialogo(x.page);
      ok('4 fechar com escolhas pede confirmação', d.title === 'Fechar o fluxograma?' && /\(1\)/.test(d.message) && d.ok === 'Fechar e descartar', JSON.stringify(d));
      ok('4 o diálogo prende o foco (o foco está dentro dele)', await x.page.evaluate(() => document.querySelector('#askDialog').contains(document.activeElement)));
      await cancela(x.page); await x.page.waitForTimeout(300);
      c = await centro(x.page);
      ok('4 cancelar o fechamento deixa o fluxograma aberto, com a escolha', c.path && c.tab && c.chosen === 1);
      ok('4 o foco volta ao botão de fechar depois de cancelar', await x.page.evaluate(() => document.activeElement?.id) === 'centerPathClose', await x.page.evaluate(() => document.activeElement?.id));
      // Confirmar a substituição.
      await x.page.evaluate(id => { __t.openPath(id, 'used'); }, outra);
      await x.page.waitForFunction(() => document.querySelector('#askDialog').open);
      await confirma(x.page); await desenhado(x.page);
      c = await centro(x.page);
      ok('4 confirmar substitui: outra semente, escolhas descartadas', c.seed === outra && c.mode === 'used' && c.chosen === 0 && c.path, JSON.stringify(c));
      // De novo com escolha, e fechar confirmando.
      await x.page.evaluate(id => __t.openPath(id, 'calls'), comHomonimo); await desenhado(x.page);
      await x.page.evaluate(() => document.querySelector('#pathCanvas polyline.edge.ambiguous').dispatchEvent(new MouseEvent('click', { bubbles: true })));
      await x.page.click('#pathDetail button:has-text("Seguir por este")'); await desenhado(x.page);
      await x.page.click('#pathClose');
      await x.page.waitForFunction(() => document.querySelector('#askDialog').open);
      ok('4 "Fechar o fluxograma" da própria barra do fluxograma também pede confirmação', (await dialogo(x.page)).title === 'Fechar o fluxograma?');
      await confirma(x.page); await x.page.waitForTimeout(300);
      c = await centro(x.page);
      ok('4 confirmar fecha: sem aba, sem semente, sem escolhas, sem desenho, Cidade na frente', !c.path && !c.tab && c.seed === null && c.chosen === 0 && c.nodes === 0 && c.view === 'city' && c.buttons.length === 1, JSON.stringify(c));
      ok('4 o foco vai para "Cidade" quando a aba some', await x.page.evaluate(() => document.activeElement?.id) === 'centerCity');
      ok('4 fechar grava center.view = city', (await navObj(x.page))?.center?.view === 'city');
    }
    // Fechar sem escolhas: sem diálogo.
    await abreFluxo(x.page, S1, 'used');
    await x.page.click('#centerPathClose'); await x.page.waitForTimeout(300);
    c = await centro(x.page);
    ok('4 fechar sem escolhas não pede confirmação', !(await x.page.evaluate(() => document.querySelector('#askDialog').open)) && !c.tab && c.seed === null && c.view === 'city', JSON.stringify(c));

    // ================================================================ 5. teclado e semântica
    await abreFluxo(x.page, S1, 'used');
    await x.page.focus('#centerCity');
    await x.page.keyboard.press('Enter');
    ok('5 Enter em "Cidade" mostra a cidade', (await centro(x.page)).view === 'city');
    await x.page.keyboard.press('Tab');
    ok('5 Tab leva à aba do Fluxograma', await x.page.evaluate(() => document.activeElement?.id) === 'centerPath');
    const contorno = await x.page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle);
    ok('5 o foco pelo teclado é visível na barra (contorno)', contorno !== 'none', contorno);
    await x.page.keyboard.press('Enter');
    ok('5 Enter na aba mostra o Fluxograma', (await centro(x.page)).view === 'path');
    await x.page.keyboard.press('Tab');
    ok('5 Tab leva ao botão de fechar, com nome e título', await x.page.evaluate(() => document.activeElement?.id === 'centerPathClose' && document.activeElement.title === 'Fechar o fluxograma'));
    await x.page.keyboard.press('Enter'); await x.page.waitForTimeout(200);
    ok('5 Enter no fechar fecha (sem escolhas) e o foco vai para "Cidade"', !(await centro(x.page)).tab && await x.page.evaluate(() => document.activeElement?.id) === 'centerCity');
    const arvore = JSON.stringify(await x.page.accessibility.snapshot({ interestingOnly: false }));
    ok('5 a barra está na árvore de acessibilidade como navegação "Vistas do centro"', /"role":"navigation","name":"Vistas do centro"/.test(arvore));

    // ================================================================ 6. seleção no fluxograma (F3) e inspeção
    await vai(x.page, 'project');
    await abreFluxo(x.page, comHomonimo ?? S1, 'calls');
    await vai(x.page, 'ruler');
    const clicavel = await x.page.evaluate(() => [...document.querySelectorAll('#pathCanvas .node')].map(n => n.dataset.id).find(id => id !== __t.pathSeed && __t.state.index.get(id) != null));
    if (clicavel) {
      await x.page.evaluate(id => document.querySelector(`#pathCanvas .node[data-id="${CSS.escape(id)}"]`).dispatchEvent(new MouseEvent('click', { bubbles: true })), clicavel);
      ok('6 clique num ponto do fluxograma fora de Explorar: área não muda, barra "Selecionado" (F3)', await x.page.evaluate(id => __t.navigation.area === 'ruler' && __t.state.nodes[__t.state.selectedNode]?.id === id && !document.querySelector('#selBar').hidden, clicavel));
      await x.page.click('#centerCity');
      ok('6 a seleção feita no fluxograma vale na cidade', await x.page.evaluate(id => __t.state.nodes[__t.state.selectedNode]?.id === id, clicavel));
      await x.page.click('#selInspect');
      ok('6 Inspecionar leva a Explorar com a inspeção, e a vista central não muda', await x.page.evaluate(id => __t.navigation.area === 'explore' && __t.navigation.inspect.at(-1)?.node === id && __t.navigation.center === 'city', clicavel));
      await x.page.click('#centerPath');
      await x.page.click('#inspToList');
      ok('6 "← voltar à lista" não mexe na vista central', (await centro(x.page)).view === 'path');
    } else ok('6 há ponto clicável no fluxograma', false);

    // ================================================================ 7. troca de run (T14) e sameRun
    await x.page.click('#centerCity');
    const navAntesRun = await nav(x.page);
    await trocaRun(x.page, OUTRA);
    c = await centro(x.page);
    ok('7 trocar de run fecha o fluxograma da run anterior (T14), e a Cidade fica na frente', !c.tab && !c.path && c.seed === null && c.view === 'city', JSON.stringify(c));
    // F9: a troca de run pela pessoa guarda só a run nova; o fechamento do fluxograma não grava a vista central.
    ok('7 o fechamento pela troca de run não grava a navegação (só a run escolhida muda, F9)', JSON.stringify({ ...JSON.parse(navAntesRun), run: OUTRA }) === await nav(x.page), `${navAntesRun} -> ${await nav(x.page)}`);
    // Resposta do fluxograma que chega depois de trocar de run: ignorada.
    const SR = await x.page.evaluate(() => { const h = __t.state.nodes.find(n => n.category === 'symbol' && /function|method/.test(n.kind)); return h.id; });
    let soltaRun; const seguraRun = new Promise(z => { soltaRun = z; });
    await x.page.route(USED_BY, async r => { await seguraRun; return r.fallback(); });
    await x.page.evaluate(() => __t.navigation.area !== 'explore' && document.querySelector('#areas [data-area="explore"]').click());
    await x.page.evaluate(id => { __t.openPath(id, 'used'); }, SR);
    await x.page.waitForTimeout(200);
    await trocaRun(x.page, RUN);
    soltaRun(); await x.page.waitForTimeout(1500);
    c = await centro(x.page);
    ok('7 a resposta pedida na run anterior é ignorada: nada desenhado, fluxograma fechado (sameRun)', !c.tab && !c.path && c.seed === null && c.nodes === 0 && c.note === '', JSON.stringify(c));
    await x.page.unroute(USED_BY);

    // ---- askPath: as perguntas criadas na run anterior não entram na conversa da nova, e nada é enviado
    if (comHomonimo) {
      await abreFluxo(x.page, comHomonimo, 'calls');
      const turnosRun = await x.page.evaluate(() => __t.studio.turns.length);
      const projeto = await x.page.evaluate(() => __t.studio.project);
      let soltaPq; const seguraPq = new Promise(z => { soltaPq = z; });
      const enviados = [];
      await x.page.route(PROJECT, r => r.request().method() === 'PUT' ? r.fulfill({ json: projeto }) : r.fallback());
      await x.page.route(PATH_Q, async r => {
        const nodes = JSON.parse(r.request().postData()).nodes;
        await seguraPq;
        return r.fulfill({ json: nodes.map((n, k) => ({ id: `f4-falso-${k}`, query: 'q', path: { node: n }, sends: [] })) });
      });
      await x.page.route(SEND, r => { enviados.push(1); return r.abort(); });
      const visivelAsk = await x.page.evaluate(() => !document.querySelector('#pathAsk').hidden);
      if (visivelAsk) {
        await x.page.click('#pathAsk');
        await x.page.waitForFunction(() => document.querySelector('#askDialog').open);
        await confirma(x.page);
        await x.page.waitForTimeout(300);
        await trocaRun(x.page, OUTRA);
        // Fechamento do F9: espera a página tratar a resposta segurada (a nota de erro), em vez de um tempo fixo: as leituras da run
        // nova agora correm juntas, e numa máquina carregada 1 s não bastava. A expectativa abaixo é a mesma.
        soltaPq(); await x.page.waitForFunction(() => document.querySelector("#chatNote").classList.contains("error"), null, { timeout: 30000 }).catch(() => {}); await x.page.waitForTimeout(300);
        const r = await x.page.evaluate(() => ({ falsos: __t.studio.turns.filter(t => String(t.id).startsWith('f4-falso')).length, run: __t.studio.run, note: document.querySelector('#chatNote').textContent }));
        ok('7 askPath: perguntas que chegam depois da troca de run não entram na conversa da nova run', r.falsos === 0 && r.run === OUTRA, JSON.stringify(r));
        ok('7 askPath: nada é enviado daqui, e a tela diz que ficaram preparadas na run anterior', enviados.length === 0 && r.note.includes(`run ${RUN}`) && /não foram enviadas/.test(r.note), r.note);
        await trocaRun(x.page, RUN);
        const deVolta = await x.page.evaluate(() => ({ n: __t.studio.turns.length, falsos: __t.studio.turns.filter(t => String(t.id).startsWith('f4-falso')).length }));
        ok('7 askPath: a conversa da run de origem não ganhou perguntas falsas da página', deVolta.n === turnosRun && deVolta.falsos === 0, JSON.stringify({ deVolta, turnosRun }));
      } else ok('7 askPath: o botão de perguntas por ponto aparece no fluxograma de chamadas', false);
      await x.page.unroute(PATH_Q);
      await x.page.unroute(SEND);
      await x.page.unroute(PROJECT);
    }

    // ================================================================ 8. restauração (T17) e o mesmo projeto aberto de novo
    await abreFluxo(x.page, S1, 'used');
    // F9 (2.1): abrir de novo abre a run guardada do projeto (a escolhida pela pessoa), ou a mais recente sem nenhuma.
    const runs0 = await x.page.evaluate(() => __t.studio.project.runs[0].id), guardada = JSON.parse(await nav(x.page) ?? '{}').run ?? runs0;
    await abrePor(x.page, RENDERER, CHAVE_RENDERER);
    c = await centro(x.page);
    if (guardada === RUN) ok('8 abrir de novo o mesmo projeto volta à run guardada, a mesma (F9): o fluxograma continua aberto e na frente', c.path && c.seed === S1 && await x.page.evaluate(() => __t.studio.run) === RUN, JSON.stringify(c));
    else ok('8 abrir de novo o mesmo projeto vai para outra run: o fluxograma da outra run fecha (T14)', !c.tab && c.seed === null && c.view === 'city', JSON.stringify(c));
    if (await x.page.evaluate(() => __t.studio.run) !== RUN) await trocaRun(x.page, RUN);
    await abreFluxo(x.page, S1, 'used');
    const navPath = await nav(x.page);
    ok('8 com o fluxograma na frente, a navegação guarda center.view = path', JSON.parse(navPath).center?.view === 'path', navPath);
    await x.page.reload();
    await x.page.waitForFunction(() => window.__t && __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
    await x.page.waitForTimeout(800);
    c = await centro(x.page);
    ok('8 recarregar: o fluxograma não é refeito (2.3) e a Cidade fica na frente (T17)', !c.path && !c.tab && c.view === 'city' && c.seed === null, JSON.stringify(c));
    ok('8 recarregar não grava a navegação (o center.view guardado fica como estava)', await nav(x.page) === navPath, `${navPath} -> ${await nav(x.page)}`);
    ok('8 área e vista de Explorar restauradas como antes (F2)', await x.page.evaluate(n => __t.navigation.area === n.area && __t.navigation.explore === (n.explore_view ?? 'structure'), JSON.parse(navPath)));
    await fecha(x);

    // ================================================================ 9. isolamento entre projetos
    x = await abre(browser, { storage: { [NAV_B]: JSON.stringify({ run: null, area: 'facts', explore_view: null, center: { view: 'path' }, active_turn: {} }) } });
    await abreFluxo(x.page, S1, 'used');
    const navA = await nav(x.page), navB = await nav(x.page, NAV_B);
    await abrePor(x.page, RAIZ_B, CHAVE_B);
    c = await centro(x.page);
    ok('9 abrir B (sem runs) com o fluxograma de A aberto: fecha, sem aba, Cidade na frente', !c.tab && !c.path && c.seed === null && c.view === 'city' && c.nodes === 0, JSON.stringify(c));
    ok('9 B tinha center.view = path guardado: não é refeito, e abrir B não grava nada', await nav(x.page, NAV_B) === navB, await nav(x.page, NAV_B));
    ok('9 B recupera a própria área guardada (Acervo), sem herdar a de A', await x.page.evaluate(() => __t.navigation.area) === 'facts');
    ok('9 a navegação de A não muda com a troca', await nav(x.page) === navA, `${navA} -> ${await nav(x.page)}`);
    ok('9 em B, nada do grafo de A: cidade vazia e sem seleção', await x.page.evaluate(() => __t.state.nodes.length === 0 && __t.state.selectedNode == null));
    await x.page.click('#centerCity');
    ok('9 em B, clicar em "Cidade" (já na frente) não grava nada', await nav(x.page, NAV_B) === navB);
    await abrePor(x.page, RENDERER, CHAVE_RENDERER);
    await x.page.waitForFunction(() => __t.state.nodes.length, null, { timeout: 120000 });
    c = await centro(x.page);
    ok('9 de volta a A: o fluxograma de antes foi descartado; Cidade na frente', !c.tab && c.seed === null && c.view === 'city', JSON.stringify(c));
    ok('9 de volta a A: abrir não grava a navegação de A', await nav(x.page) === navA);
    await fecha(x);

    // ================================================================ 10. tela estreita (1.1)
    x = await abre(browser, { size: { width: 390, height: 844 }, scheme: 'dark' });
    const col = () => x.page.evaluate(() => ({ col: document.querySelector('#app').dataset.col, center: document.querySelector('#colSwitch [data-col="center"]').textContent,
      announce: document.querySelector('#announce').textContent, scroll: document.documentElement.scrollWidth - document.documentElement.clientWidth }));
    await x.page.click('#colSwitch [data-col="nav"]');
    await abreFluxo(x.page, S1, 'used');
    let k = await col();
    ok('10 estreita: abrir o fluxograma leva ao Centro e anuncia "Centro: Fluxograma"', k.col === 'center' && k.center === 'Centro: Fluxograma' && k.announce === 'Coluna mostrada: Centro: Fluxograma', JSON.stringify(k));
    c = await centro(x.page);
    ok('10 estreita: a barra de vistas aparece no Centro, com as duas vistas', c.bar && c.buttons.length === 3 && c.path, JSON.stringify(c.buttons));
    await x.page.click('#centerCity');
    k = await col(); c = await centro(x.page);
    ok('10 estreita: "Cidade" na barra muda o resumo para "Centro: Cidade", e o fluxograma continua aberto', k.center === 'Centro: Cidade' && c.tab && !c.path, JSON.stringify(k));
    const cam = await x.page.evaluate(() => [__t.camera.aspect, ...__t.camera.position.toArray()]);
    ok('10 estreita: a cidade aparece com a câmera válida', cam.every(Number.isFinite) && cam[0] > 0, cam.join(','));
    await x.page.click('#centerPath');
    await x.page.click('#colSwitch [data-col="question"]');
    await x.page.click('#colSwitch [data-col="center"]');
    c = await centro(x.page); k = await col();
    ok('10 estreita: trocar de coluna e voltar preserva a vista central', c.path && c.view === 'path' && k.center === 'Centro: Fluxograma', JSON.stringify(k));
    ok('10 estreita: sem rolagem horizontal da página com a barra e o fluxograma', k.scroll <= 1, String(k.scroll));
    ok('10 estreita: a barra cabe na largura (rola dentro de si se precisar)', await x.page.evaluate(() => document.querySelector('#centerBar').getBoundingClientRect().right <= innerWidth + 1));
    if (OUT) await x.page.screenshot({ path: `${OUT}/centro-estreita.png` });
    // Temas: contraste do texto da barra (escuro aqui, claro depois).
    const contr = await x.page.evaluate(() => { const b = document.querySelector('#centerPath'), s = getComputedStyle(b), bar = getComputedStyle(document.querySelector('#centerBar')), c2 = getComputedStyle(document.querySelector('#centerCity'));
      return { text: s.color, bg: s.backgroundColor, other: c2.color, barBg: getComputedStyle(document.body).backgroundColor, surface: getComputedStyle(document.querySelector('#side')).backgroundColor }; });
    ok('10 tema escuro: contraste da aba atual ≥ 4,5', contraste(contr.text, contr.bg) >= 4.5, JSON.stringify(contr));
    ok('10 tema escuro: contraste da vista não atual ≥ 4,5 sobre o fundo da barra', contraste(contr.other, contr.surface) >= 4.5, JSON.stringify(contr));
    await fecha(x);

    // ================================================================ 11. EN, tema claro
    x = await abre(browser, { scheme: 'light' });
    await x.page.click('[data-lang="en"]');
    await abreFluxo(x.page, S1, 'used').catch(async () => {
      await x.page.evaluate(id => __t.selectNode(__t.state.index.get(id)), S1);
      await x.page.click('#inspector button:has-text("Where is it used?")'); await desenhado(x.page);
    });
    c = await centro(x.page);
    ok('11 EN: "City", "Flowchart: …", estado em inglês, fechar "Close the flowchart", navegação "Center views"',
      c.buttons[0].text === 'City' && c.title.startsWith('Flowchart: ') && /^· (\d+ points|empty)/.test(c.state) && c.buttons.some(b => b.label === 'Close the flowchart') && c.aria === 'Center views', JSON.stringify(c));
    await x.page.click('#centerCity');
    ok('11 EN: resumo da coluna "Center: City"', (await centro(x.page)).col === 'Center: City');
    await x.page.click('[data-lang="pt"]');
    c = await centro(x.page);
    ok('11 voltar a PT atualiza a barra (com o fluxograma atrás da cidade)', c.buttons[0].text === 'Cidade' && c.title.startsWith('Fluxograma: ') && c.aria === 'Vistas do centro', JSON.stringify(c));
    const chaves = await x.page.evaluate(() => Object.entries(__t.STRINGS).filter(([k]) => k.startsWith('center.') || k === 'path.runChanged' || k === 'path.close').map(([k, v]) => [k, Array.isArray(v) && v.length === 2 && v.every(s => typeof s === 'string' && s.trim())]));
    ok('11 todas as chaves novas existem em PT e EN', chaves.length >= 17 && chaves.every(([, b]) => b), JSON.stringify(chaves.filter(([, b]) => !b)));
    const contrL = await x.page.evaluate(() => { const b = document.querySelector('#centerCity'), s = getComputedStyle(b), p = getComputedStyle(document.querySelector('#centerPath'));
      return { text: s.color, bg: s.backgroundColor, other: p.color, surface: getComputedStyle(document.querySelector('#side')).backgroundColor }; });
    ok('11 tema claro: contraste da vista atual ≥ 4,5', contraste(contrL.text, contrL.bg) >= 4.5, JSON.stringify(contrL));
    ok('11 tema claro: contraste da vista não atual ≥ 4,5 sobre o fundo da barra', contraste(contrL.other, contrL.surface) >= 4.5, JSON.stringify(contrL));
    await x.page.evaluate(() => { document.documentElement.dataset.theme = 'dark'; });
    const contrD = await x.page.evaluate(() => { const s = getComputedStyle(document.querySelector('#centerCity')); return { text: s.color, bg: s.backgroundColor }; });
    ok('11 troca manual de tema: a barra muda de cor e mantém contraste', contrD.text !== contrL.text && contraste(contrD.text, contrD.bg) >= 4.5, JSON.stringify(contrD));
    if (OUT) await x.page.screenshot({ path: `${OUT}/centro-largo.png` });
    await fecha(x);

    // ================================================================ 12. modo arquivo (sem servidor)
    const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const fp = await ctx.newPage(), ferros = [];
    fp.on('pageerror', e => ferros.push(e.message));
    await fp.goto('file:///' + path.join(RAIZ, 'crates/librarian-studio/web/index.html').replaceAll('\\', '/') + '#debug');
    await fp.waitForFunction(() => window.viewerDebug, null, { timeout: 60000 });
    ok('12 modo arquivo: sem barra de vistas, e o #status no lugar de antes', await fp.evaluate(() => getComputedStyle(document.querySelector('#centerBar')).display === 'none' && document.querySelector('#status').getBoundingClientRect().top === 12));
    ok('12 modo arquivo: sem erro', ferros.length === 0, ferros.join(' | '));
    await ctx.close();
  } finally {
    ok('sem erro de página', todosErros.length === 0, todosErros.join(' | ').slice(0, 400));
    ok('nenhuma escrita no servidor (nada enviado a modelo)', todasEscritas.length === 0, todasEscritas.join(','));
    ok('o perfil (project.json) fica igual em bytes', sha(perfil) === perfilAntes);
    ok('as perguntas da run ficam as mesmas', (await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`)).json()).length === turnosAntes);
    await browser.close();
  }
})().catch(e => { console.error(e); process.exit(1); });

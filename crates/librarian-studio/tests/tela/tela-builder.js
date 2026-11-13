// F5: Evidence Builder (especificação 1.2, 1.4, 1.5, 2.3, 2.4, 3.1 a 3.10, 5.4, 7.1, 7.3, T4, T6, T8, T21;
// critérios 9.1.5, 9.2.7, 9.2.8 e a parte de 9.5 que se confere na página), na página real, num Chrome sem
// janela, contra o servidor de rodar.js (8841).
// Casos: o Builder como vista central (aba, fechar só esconde, voltar pela barra, pelo compositor e pelo
// cabeçalho); a lista fora do compositor (uma linha só); Juntar pela inspeção e pelo fluxograma (soma, sem
// trocar de vista); Sugerir, Juntar as sugeridas, Remover e Incluir nas quatro listas do formato 2, iguais ao
// registro que vai ao servidor; proveniência registrada separada do "chegou por" local; a confirmação ao
// substituir um rascunho não vazio (Otimizar, Investigar, Pedir esta mudança, Limpar), com Cancelar sem
// mudança nenhuma; a sequência dos pedidos (resposta antiga ou de outra run descartada, com aviso) e Juntar
// desabilitado durante uma entrada que recomeça; o achado em modo achado, nunca convertido em seleção;
// "Pedir esta mudança" pelo Builder, com o corpo da escada montado do rascunho; o modo necessário;
// "já preparado" e "alterado depois"; a recusa do servidor como veio; troca de run e de projeto; tela
// estreita; PT e EN; temas; teclado e foco.
// Nada é enviado a modelo e nada é gravado no servidor: toda escrita é abortada e contada; PUT /api/project
// e POST /api/gathered-question são respondidos aqui, sem servidor, e os corpos são guardados para conferir.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium, RENDERER, CHAVE_RENDERER } = require('./ambiente');
const OUT = process.argv[2];
const B = 'http://127.0.0.1:8841', RUN = '20261104-020244', OUTRA = '20261104-014710';
const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS;
const HOOK = "window.__t = { state, studio, t, STRINGS, selectNode, openPath, loadRun, goTo, optimize, investigate, toggleGathered, selectionOnRecord,"
  + " get navigation() { return navigation; }, get gathered() { return gathered; }, get seeds() { return seeds; }, get selection() { return selection; },"
  + " get suggestion() { return suggestion; }, get investigation() { return investigation; }, get gatherFacts() { return gatherFacts; }, get draft() { return draft; },"
  + " get builderOpen() { return builderOpen; }, get builderLoading() { return builderLoading; }, get hot() { return hot; }, get mode() { return mode; },"
  + " get perfEntries() { return perfEntries; }, get pathSeed() { return pathSeed; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 300) : ''}`); if (!pass) process.exitCode = 1; };
const LEITURA = (method, pathname) => method === 'GET'
  || (method === 'POST' && ['/api/project', '/api/calls', '/api/envelope/suggest', '/api/inventory/investigate', '/api/performance-question'].includes(pathname));
const perfil = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'project.json');
const sha = f => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const NAV = `librarian.nav.v1:${CHAVE_RENDERER}`;
const RAIZ_B = 'C:/projeto-ficticio-da-bateria/b', CHAVE_B = 'ficticio-b-00000000';
const rota = p => u => new URL(u).pathname === p;
const SUGGEST = rota('/api/envelope/suggest'), INVESTIGATE = rota('/api/inventory/investigate'), GATHERED = rota('/api/gathered-question');

async function abre(browser, { size = { width: 1440, height: 900 }, scheme = 'light', locale = 'pt-BR', storage = null } = {}) {
  const context = await browser.newContext({ viewport: size, colorScheme: scheme, locale });
  if (storage) await context.addInitScript(s => { if (!sessionStorage.getItem('f5.semeado')) { for (const [k, v] of Object.entries(s)) localStorage.setItem(k, v); sessionStorage.setItem('f5.semeado', '1'); } }, storage);
  const page = await context.newPage(), errors = [], writes = [], bodies = { gathered: [], ladder: [], put: 0 }, reads = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('request', r => { const p = new URL(r.url()).pathname; if (['/api/envelope/suggest', '/api/inventory/investigate'].includes(p)) reads.push(p); });
  await page.route(u => new URL(u).pathname === '/', async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.route(u => new URL(u).pathname.startsWith('/api/'), async r => {
    const req = r.request(), { pathname } = new URL(req.url());
    if (LEITURA(req.method(), pathname)) return r.continue();
    // PUT /api/project: respondido com o perfil enviado, sem gravar (F7: só Alterar a política e Salvar o chamam).
    if (req.method() === 'PUT' && pathname === '/api/project') {
      bodies.put++;
      const sent = JSON.parse(req.postData()), key = sent.key;
      const res = await fetch(`${B}/api/project?key=${encodeURIComponent(key === CHAVE_B ? CHAVE_RENDERER : key)}`);
      return r.fulfill({ json: { ...(await res.json()), key, settings: sent.settings } });
    }
    if (req.method() === 'POST' && pathname === '/api/ladder') { bodies.ladder.push(req.postData()); writes.push('POST /api/ladder'); return r.abort(); }
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
  await abrePor(page, RENDERER, CHAVE_RENDERER);
  await page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
  if (await page.evaluate(() => __t.studio.run) !== RUN) await trocaRun(page, RUN);
  await page.waitForFunction(() => __t.hot?.rows?.length, null, { timeout: 120000 });
  await page.waitForTimeout(500);
  return { page, errors, writes, bodies, reads, context, close: () => context.close() };
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
// /api/gathered-question respondido aqui com uma pergunta da própria run (só o id e o texto trocados),
// para a conversa desenhar uma pergunta real sem nada gravado. `recusa`: devolve o erro do servidor.
async function responderPreparo(x, { recusa = null } = {}) {
  await x.page.unroute(GATHERED).catch(() => {});
  const turns = await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`)).json();
  const base = turns.filter(t => t.gathered).at(-1) ?? turns.at(-1);
  await x.page.route(GATHERED, r => {
    const body = r.request().postData();
    x.bodies.gathered.push(body);
    if (recusa) return r.fulfill({ status: 400, json: { error: recusa } });
    return r.fulfill({ json: { ...base, id: 900000 + x.bodies.gathered.length, question: JSON.parse(body).question, sends: [] } });
  });
}
const b = page => page.evaluate(() => {
  const shown = e => Boolean(e && e.offsetParent !== null && e.getClientRects().length);
  const parts = Object.fromEntries([...document.querySelectorAll('#builderRecord .part')].filter(s => s.querySelector('h3')).map(s => [s.querySelector('h3').textContent, s.querySelectorAll('.src').length]));
  return {
    view: __t.navigation.center, shown: shown(document.querySelector('#builderView')), tab: shown(document.querySelector('#centerBuilderTab')),
    tabText: document.querySelector('#centerBuilder').textContent, current: document.querySelector('#centerBuilder').getAttribute('aria-current'),
    state: document.querySelector('#builderState').textContent, entry: document.querySelector('#builderEntry').hidden ? '' : document.querySelector('#builderEntry').textContent,
    notice: document.querySelector('#builderNotice').hidden ? '' : document.querySelector('#builderNotice').textContent,
    record: document.querySelector('#builderRecord').innerText, parts,
    line: document.querySelector('#draftText').textContent, head: document.querySelector('#headDraft').hidden ? null : document.querySelector('#headDraft').textContent,
    gathered: __t.gathered.length, area: __t.navigation.area, col: document.querySelector('#colSwitch [data-col="center"]').textContent,
  };
});
const dialogo = page => page.evaluate(() => ({ open: document.querySelector('#askDialog').open, title: document.querySelector('#askTitle').textContent,
  message: document.querySelector('#askMessage').textContent, ok: document.querySelector('#askOk').textContent }));
const cancela = page => page.click('#askDialog .row:last-child [data-close]');
const confirma = page => page.click('#askOk');
const esperaDialogo = page => page.waitForFunction(() => document.querySelector('#askDialog').open, null, { timeout: 30000 });
// O rascunho inteiro, como a página o tem: o que vai ao servidor e o que a tela sabe dele.
const rascunho = page => page.evaluate(() => JSON.stringify({ g: __t.gathered, s: __t.seeds, f: __t.gatherFacts, r: __t.selectionOnRecord(), i: __t.investigation && [...__t.investigation.ticked],
  sel: __t.selection && [...__t.selection.sources.keys()], e: __t.draft.entry, m: __t.draft.required_mode, a: [...__t.draft.arrived], q: document.querySelector('#question').value }));
const linha = async (page, n) => { await page.waitForFunction(() => __t.hot?.rows, null, { timeout: 120000 }); return page.evaluate(n => __t.hot.rows.find(r => r.name === n), n); };
const INV = 'impl Matrix<4>::inverse', OUTRO = 'impl World::color_at';
// Juntar pela inspeção (T4): seleciona em Explorar e usa o botão da inspeção.
async function juntaPelaInspecao(page, name) {
  if (await page.evaluate(() => __t.navigation.area) !== 'explore') await page.click('#areas [data-area="explore"]');
  // Fechamento do F9: a medição da run é lida junto com as outras leituras; espera ela chegar antes de procurar a linha.
  await page.waitForFunction(() => __t.hot?.rows, null, { timeout: 120000 });
  const id = await page.evaluate(n => { const r = __t.hot.rows.find(x => x.name === n); __t.selectNode(__t.state.index.get(r.symbol)); return r.symbol; }, name);
  await page.waitForTimeout(150);
  await page.click(`#inspector button:has-text("${await page.evaluate(() => __t.t('gather.add'))}")`);
  return id;
}
function cor(s) {
  let m = s.match(/rgba?\(([^)]+)\)/);
  if (m) { const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number); return p.slice(0, 3).map(v => v / 255); }
  m = s.match(/color\(srgb ([^)]+)\)/);
  if (m) return m[1].split(/[ /]+/).filter(Boolean).slice(0, 3).map(Number);
  return null;
}
function contraste(a, c) {
  const L = k => { const [r, g, bl] = k.map(v => v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4); return 0.2126 * r + 0.7152 * g + 0.0722 * bl; };
  const [x, y] = [L(cor(a)), L(cor(c))].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}
const cores = page => page.evaluate(() => {
  const bg = getComputedStyle(document.querySelector('#builderView')).backgroundColor;
  const c = s => getComputedStyle(document.querySelector(s)).color;
  return { bg, state: c('#builderState'), src: c('#builderRecord .src'), says: c('#builderRecord .says'), notice: c('#builderNotice') };
});

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const perfilAntes = sha(perfil);
  const turnosAntes = (await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`)).json()).length;
  const todasEscritas = [], todosErros = [];
  const fecha = async x => { todasEscritas.push(...x.writes); todosErros.push(...x.errors); await x.close(); };
  try {
    // ================================================================ 1. estrutura: vista central, compositor e cabeçalho
    let x = await abre(browser);
    let s = await b(x.page);
    ok('1 sem rascunho: Builder fora da barra e fora da frente; a cidade é a vista', !s.tab && !s.shown && s.view === 'city', JSON.stringify(s));
    ok('1 a lista de fontes saiu do compositor: #gathered fica no Builder, não no compositor', await x.page.evaluate(() => !document.querySelector('#composer #gathered') && Boolean(document.querySelector('#builderView #gathered'))));
    ok('1 o compositor mostra uma linha sobre o Builder', s.line === 'Builder: sem fontes' && await x.page.evaluate(() => document.querySelector('#draftOpen').textContent === 'Abrir no Builder'), s.line);
    ok('1 o cabeçalho não mostra rascunho vazio', s.head === null);
    await x.page.click('#draftOpen');
    s = await b(x.page);
    ok('1 "Abrir no Builder" põe o Builder na frente, vazio, com o estado escrito', s.shown && s.tab && s.view === 'builder' && s.current === 'true' && s.state === 'Junte uma fonte pela inspeção ou use Otimizar', JSON.stringify(s));
    ok('1 o foco vai ao título do Builder', await x.page.evaluate(() => document.activeElement?.id === 'builderTitle'));
    ok('1 abrir o Builder é movimento da pessoa: a vista central fica na navegação do projeto', JSON.parse(await x.page.evaluate(k => localStorage.getItem(k), NAV) ?? '{}').center?.view === 'builder');
    ok('1 rascunho vazio: nada guardado no navegador', await x.page.evaluate(() => !Object.keys(localStorage).some(k => k.startsWith('librarian.draft'))));
    await x.page.click('#centerCity');

    // ================================================================ 2. Juntar pela inspeção (T4)
    const inv = await juntaPelaInspecao(x.page, INV);
    s = await b(x.page);
    ok('2 Juntar soma uma fonte e fica onde estava: área Explorar, centro na cidade', s.gathered === 1 && s.area === 'explore' && s.view === 'city', JSON.stringify(s));
    ok('2 o contador sobe na barra, no compositor e no cabeçalho', s.tab && s.tabText === 'Builder (1)' && /^Rascunho: 1 fonte\(s\) · alvo /.test(s.line) && s.head === 'Rascunho: 1 fonte(s) · guardado neste navegador', JSON.stringify([s.tabText, s.line, s.head]));
    ok('2 o cabeçalho diz que o rascunho fica guardado neste navegador e não é registro (F6)', /guardado neste navegador.*não é registro/.test(await x.page.evaluate(() => document.querySelector('#headDraft').title)));
    ok('2 "chegou por" é só da tela: inspeção', await x.page.evaluate(i => __t.draft.arrived.get(i), inv) === 'inspection');
    await x.page.click('#headDraft');
    s = await b(x.page);
    ok('2 o cabeçalho abre o Builder', s.view === 'builder' && s.shown);
    ok('2 sem sugestão não há registro: o Builder diz isso e mostra só Incluídas e Removidas', /Sem sugestão, não há registro de seleção/.test(s.record) && s.parts['Incluídas (1) · vão ao modelo'] === 1 && 'Removidas (0) · tiradas por você; não vão' in s.parts && !Object.keys(s.parts).some(k => /Deixadas|Não juntadas/.test(k)), JSON.stringify(s.parts));
    ok('2 sem sugestão: nenhuma proveniência "no registro", só o "chegou por" local', !/no registro:/.test(s.record) && /chegou por: inspeção · só local/.test(s.record), s.record.slice(0, 200));
    ok('2 o estado escreve a fonte e o alvo', /^1 fonte\(s\) incluída\(s\) · alvo: Matrix<4>::inverse/.test(s.state), s.state);

    // ================================================================ 3. fechar só esconde (9.1.5)
    let antes = await rascunho(x.page);
    await x.page.click('#centerBuilderClose');
    s = await b(x.page);
    ok('3 9.1.5 fechar o Builder: sai da barra e da frente, e a cidade volta', !s.tab && !s.shown && s.view === 'city', JSON.stringify(s));
    ok('3 9.1.5 fechar o Builder não altera o rascunho', await rascunho(x.page) === antes);
    ok('3 depois de fechar, o foco vai para "Cidade"', await x.page.evaluate(() => document.activeElement?.id === 'centerCity'));
    await x.page.click('#draftOpen');
    ok('3 reabrir mostra o mesmo rascunho', await rascunho(x.page) === antes && (await b(x.page)).shown);
    await x.page.click('#centerCity');
    ok('3 ir à Cidade esconde o Builder, que continua na barra, sem mudar o rascunho', (await b(x.page)).tab && !(await b(x.page)).shown && await rascunho(x.page) === antes);
    await x.page.click('#centerBuilder');

    // ================================================================ 4. Sugerir e as quatro listas (3.1, 3.5)
    await x.page.click(`#gathered button:has-text("Sugerir envelope")`);
    await x.page.waitForFunction(() => __t.suggestion && __t.selectionOnRecord(), null, { timeout: 60000 });
    const prefixo = (await x.page.evaluate(() => __t.t('envelope.use', { n: 'X' }))).split('(')[0];
    await x.page.click(`#suggestion button:has-text("${prefixo.trim()}")`);
    await x.page.waitForTimeout(200);
    const confere = async rotulo => {
      const r = await x.page.evaluate(() => { const rec = __t.selectionOnRecord(); return { n: Object.fromEntries(['included', 'removed', 'left_out', 'not_joined'].map(k => [k, rec[k].length])),
        ids: ['included', 'removed', 'left_out', 'not_joined'].flatMap(k => rec[k].map(e => e.id)), sources: [...__t.selection.sources.keys()], nodes: JSON.stringify(__t.gathered) === JSON.stringify(rec.included.map(e => e.id)) }; });
      const v = await b(x.page);
      const tela = { included: v.parts[`Incluídas (${r.n.included}) · vão ao modelo`], removed: v.parts[`Removidas (${r.n.removed}) · tiradas por você; não vão`],
        left_out: v.parts[`Deixadas de fora (${r.n.left_out}) · pela sugestão, com o motivo; não vão`], not_joined: v.parts[`Não juntadas (${r.n.not_joined}) · sugeridas e não trazidas; não vão`] };
      ok(`4 ${rotulo}: as quatro listas da tela são as do registro (formato 2), com as mesmas contagens`, ['included', 'removed', 'left_out', 'not_joined'].every(k => tela[k] === r.n[k]), JSON.stringify({ tela, rec: r.n }));
      ok(`4 ${rotulo}: cada fonte conhecida em exatamente uma lista; incluídas = nodes`, r.ids.length === new Set(r.ids).size && r.sources.every(i => r.ids.includes(i)) && r.nodes, JSON.stringify(r.n));
      return r;
    };
    let r = await confere('depois de juntar as sugeridas');
    s = await b(x.page);
    ok('4 com sugestão, o Builder diz que as listas vão ao servidor como registro', /registro da seleção \(formato 2\)/.test(s.record) && /Só as incluídas vão ao modelo/.test(s.record));
    ok('4 orçamento: a tela mostra a conta e diz que não decide', /Conta da tela: [\d.]+ KB nas incluídas.*de \d+ KB do orçamento da sugestão.*quem confere é o servidor/.test(s.record), s.record.match(/Conta da tela[^\n]*/)?.[0]);
    ok('4 proveniência registrada separada do "chegou por" local', /no registro: sugerida/.test(s.record) && /no registro: à mão/.test(s.record) && /chegou por: sugestão · só local/.test(s.record));
    ok('4 o "chegou por" é marcado como local (título e estilo próprios)', await x.page.evaluate(() => { const t = [...document.querySelectorAll('#builderRecord .tag.local')]; return t.length > 0 && t.every(e => /não vai ao servidor/.test(e.title)); }));
    ok('4 estado da informação por texto: Estrutural e, com medição, Medido', /Estrutural/.test(s.record) && /Medido/.test(s.record));
    ok('4 o registro não ganhou campo novo (nada da tela vai ao servidor)', await x.page.evaluate(() => { const rec = __t.selectionOnRecord(); const k = new Set(rec.included.flatMap(e => Object.keys(e))); return !k.has('arrived') && !k.has('coverage_state') && !k.has('via'); }));
    // Remover uma incluída que não é alvo, pelo botão da lista.
    const tirada = await x.page.evaluate(() => __t.gathered.find(g => !__t.seeds.includes(g)));
    await x.page.evaluate(id => { const row = [...document.querySelectorAll('#builderRecord .part')].find(p => /^Incluídas/.test(p.querySelector('h3').textContent));
      [...row.querySelectorAll('.src button')].find(b => b.getAttribute('aria-label').endsWith(': ' + (__t.selection.sources.get(id).name.replace(/^impl (.+ for )?/, '')))).click(); }, tirada);
    r = await confere('depois de Remover');
    ok('4 Remover leva a fonte a removed, com decisão single', await x.page.evaluate(id => { const e = __t.selectionOnRecord().removed.find(x => x.id === id); return e?.decision === 'removed' && e.decision_scope === 'single'; }, tirada));
    // Incluir uma deixada de fora, pelo botão da lista.
    const deFora = await x.page.evaluate(() => __t.selectionOnRecord().left_out[0]?.id ?? null);
    if (deFora) {
      await x.page.evaluate(id => { const row = [...document.querySelectorAll('#builderRecord .part')].find(p => /^Deixadas/.test(p.querySelector('h3').textContent));
        row.querySelectorAll('.src')[__t.selectionOnRecord().left_out.findIndex(e => e.id === id)].querySelector('button').click(); }, deFora);
      r = await confere('depois de Incluir uma deixada de fora');
      ok('4 Incluir uma deixada de fora: included, single, proveniência left_out preservada', await x.page.evaluate(id => { const e = __t.selectionOnRecord().included.find(x => x.id === id); return e?.decision === 'included' && e.decision_scope === 'single' && e.provenance === 'left_out'; }, deFora));
    } else ok('4 (sem deixada de fora nesta sugestão)', true);
    await x.page.evaluate(id => { const row = [...document.querySelectorAll('#builderRecord .part')].find(p => /^Removidas/.test(p.querySelector('h3').textContent)); row.querySelector('.src button').click(); }, tirada);
    await confere('depois de Incluir a removida');
    ok('4 Incluir uma removida a devolve às incluídas', await x.page.evaluate(id => __t.gathered.includes(id), tirada));

    // ================================================================ 5. preparar: corpo, "já preparado", alterado, recusa
    await responderPreparo(x);
    await x.page.evaluate(() => { document.querySelector('#ladderOn').checked = false; const p = document.querySelector('#provider'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
    const esperado = await x.page.evaluate(() => ({ nodes: __t.gathered.slice(), with_facts: __t.gatherFacts, selection: __t.selectionOnRecord() }));
    await x.page.fill('#question', 'F5 bateria: o que muda?');
    await x.page.click('#ask');
    await x.page.waitForFunction(() => /Já preparado/.test(document.querySelector('#builderState').textContent), null, { timeout: 30000 });
    const corpo = JSON.parse(x.bodies.gathered.at(-1));
    ok('5 9.5 o corpo leva nodes = incluídas e a seleção = o registro mostrado', JSON.stringify(corpo.nodes) === JSON.stringify(esperado.nodes) && corpo.with_facts === esperado.with_facts
      && JSON.stringify(corpo.origin.selection) === JSON.stringify(esperado.selection) && corpo.origin.kind === 'selection', JSON.stringify(Object.keys(corpo)));
    ok('5 9.5 as chaves do corpo são as de antes, na mesma ordem', JSON.stringify(Object.keys(corpo)) === '["key","run","question","nodes","with_facts","origin"]' && JSON.stringify(Object.keys(corpo.origin)) === '["kind","summary","selection"]', JSON.stringify(Object.keys(corpo)));
    s = await b(x.page);
    ok('5 depois de preparar: "Já preparado como #N, sem mudanças desde então"', /Já preparado como #9000\d+, sem mudanças desde então/.test(s.state), s.state);
    await x.page.evaluate(id => __t.toggleGathered(id), tirada);
    ok('5 mudar depois de preparar: "Alterado depois de preparar #N"', /Alterado depois de preparar #9000\d+/.test((await b(x.page)).state), (await b(x.page)).state);
    await x.page.evaluate(id => __t.toggleGathered(id), tirada);
    await responderPreparo(x, { recusa: 'Seleção recusada (teste da bateria); nenhuma pergunta foi criada' });
    antes = await rascunho(x.page);
    await x.page.fill('#question', 'F5 bateria: recusa');
    await x.page.click('#ask');
    await x.page.waitForFunction(() => !document.querySelector('#builderNotice').hidden, null, { timeout: 30000 });
    s = await b(x.page);
    ok('5 recusa do servidor: aparece no Builder como veio', s.notice === 'Seleção recusada (teste da bateria); nenhuma pergunta foi criada', s.notice);
    ok('5 recusa: o rascunho fica como estava', (await rascunho(x.page)).replace(/"q":"[^"]*"/, '') === antes.replace(/"q":"[^"]*"/, ''));
    await x.page.unroute(GATHERED);
    await fecha(x);

    // ================================================================ 6. substituir um rascunho não vazio (9.2.7, 3.7, T6, T8, T21)
    x = await abre(browser);
    await juntaPelaInspecao(x.page, OUTRO);
    await x.page.fill('#question', 'texto que a pessoa escreveu');
    antes = await rascunho(x.page);
    const sugestoesAntes = x.reads.length;
    const optimizar = x.page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
    await esperaDialogo(x.page);
    let d = await dialogo(x.page);
    ok('6 9.2.7 Otimizar sobre rascunho não vazio pede confirmação, com fontes, alvo e texto', d.title === 'Substituir o rascunho atual?' && /^1 fonte\(s\), alvo World::color_at, texto “texto que a pessoa escreveu”$/.test(d.message) && d.ok === 'Substituir', JSON.stringify(d));
    ok('6 o diálogo prende o foco', await x.page.evaluate(() => document.querySelector('#askDialog').contains(document.activeElement)));
    await cancela(x.page); await optimizar;
    ok('6 9.2.7 Cancelar não muda nada: rascunho, texto e nenhum pedido ao servidor', await rascunho(x.page) === antes && x.reads.length === sugestoesAntes, `${x.reads.length - sugestoesAntes} pedidos`);
    const optimizar2 = x.page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
    await esperaDialogo(x.page); await confirma(x.page); await optimizar2;
    await x.page.waitForFunction(() => !__t.builderLoading, null, { timeout: 60000 });
    s = await b(x.page);
    const linhaInv = await linha(x.page, INV);
    ok('6 T6 Substituir: rascunho novo de Otimizar no Builder, na frente', s.view === 'builder' && s.shown && await x.page.evaluate(id => JSON.stringify(__t.seeds) === JSON.stringify([id]) && __t.gatherFacts === true && document.querySelector('#ladderOn').checked, linhaInv.symbol), JSON.stringify(s));
    ok('6 a entrada e o modo necessário ficam escritos no Builder', s.entry === `Entrada: Otimizar · ${INV} · Modo necessário: Melhorar (registrado neste rascunho); a política do projeto está em Localizar`, s.entry);
    ok('6 o modo necessário fica no rascunho (5.4)', await x.page.evaluate(() => __t.draft.required_mode === 'improve' && __t.draft.entry.kind === 'optimize'));
    ok('6 as fontes de Otimizar chegam por "Otimizar" (só na tela)', /chegou por: Otimizar · só local/.test(s.record));
    // F7: Otimizar não troca mais o modo; a política do projeto (Localizar) diferente do necessário aparece, sem gravar.
    ok('6 a política do projeto diferente do modo necessário aparece no Builder, sem gravar nada', /; a política do projeto está em Localizar$/.test((await b(x.page)).entry) && x.bodies.put === 0 && await x.page.evaluate(() => __t.mode) === 'locate', (await b(x.page)).entry);
    // Limpar (T21).
    antes = await rascunho(x.page);
    await x.page.click('#builderClear');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('6 T21 Limpar pede confirmação e diz que o texto fica', d.title === 'Limpar o rascunho?' && /O texto da pergunta no compositor fica\.$/.test(d.message), d.message);
    await cancela(x.page);
    ok('6 T21 Cancelar não muda nada', await rascunho(x.page) === antes);
    ok('6 o foco volta ao botão que abriu o diálogo', await x.page.evaluate(() => document.activeElement?.id === 'builderClear'));
    await x.page.click('#builderClear'); await esperaDialogo(x.page); await confirma(x.page);
    s = await b(x.page);
    ok('6 T21 Limpar esvazia o Builder e mantém o texto', s.gathered === 0 && s.state === 'Junte uma fonte pela inspeção ou use Otimizar' && await x.page.evaluate(() => document.querySelector('#question').value.length > 0 && __t.draft.required_mode === null && __t.draft.entry === null), JSON.stringify(s));
    // Rascunho vazio: sem confirmação.
    await x.page.fill('#question', '');
    const vazio = x.page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
    await vazio;
    ok('6 rascunho vazio: Otimizar não pede confirmação', !(await dialogo(x.page)).open && await x.page.evaluate(() => __t.gathered.length > 0));
    // Investigar um achado sobre rascunho não vazio (T8).
    const inventario = (await (await fetch(`${B}/api/inventory?key=${CHAVE_RENDERER}&run=${RUN}`)).json()).inventory;
    const laco = inventario.loops.find(l => l.through_calls?.length) ?? inventario.loops[0];
    antes = await rascunho(x.page);
    let inv2 = x.page.evaluate(o => __t.investigate(o, document.createElement('button')), laco.occurrence);
    await esperaDialogo(x.page);
    ok('6 9.2.7 Investigar sobre rascunho não vazio pede confirmação', (await dialogo(x.page)).title === 'Substituir o rascunho atual?');
    await cancela(x.page); await inv2;
    ok('6 9.2.7 Cancelar a investigação não muda nada', await rascunho(x.page) === antes);
    inv2 = x.page.evaluate(o => __t.investigate(o, document.createElement('button')), laco.occurrence);
    await esperaDialogo(x.page); await confirma(x.page); await inv2;
    s = await b(x.page);
    ok('6 T8 achado no Builder, em modo achado, com o rótulo da especificação', s.view === 'builder' && /Achado do inventário: itens marcados, sem registro de seleção no servidor/.test(await x.page.evaluate(() => document.querySelector('#gathered').innerText)), s.state);
    ok('6 o achado nunca vira seleção: sem listas do formato 2 e sem registro', s.record.trim() === '' && await x.page.evaluate(() => __t.selection === null && __t.selectionOnRecord() === null && __t.gathered.length === 0));
    ok('6 o achado no compositor e no cabeçalho: uma linha, sem lista', /^Achado do inventário: \d+ item\(ns\) marcado\(s\), sem registro de seleção$/.test(s.line) && s.head === 'Rascunho: achado do inventário · guardado neste navegador' && s.tabText === 'Builder: achado', JSON.stringify([s.line, s.head, s.tabText]));
    ok('6 o achado registra o modo necessário Localizar', await x.page.evaluate(() => __t.draft.required_mode === 'locate') && /Modo necessário: Localizar/.test(s.entry), s.entry);
    // O corpo do achado é o de hoje.
    await responderPreparo(x);
    await x.page.evaluate(() => { const p = document.querySelector('#provider'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
    const marcados = await x.page.evaluate(() => __t.investigation.items.map(i => i.id).filter(id => __t.investigation.ticked.has(id)));
    const nAntes = x.bodies.gathered.length;
    await x.page.click('#ask');
    for (let i = 0; i < 40 && x.bodies.gathered.length === nAntes; i++) await x.page.waitForTimeout(250);
    const cAchado = JSON.parse(x.bodies.gathered.at(-1));
    ok('6 9.5 achado: corpo como antes ({inventory, kind, summary}, sem seleção, só os marcados)', JSON.stringify(cAchado.nodes) === JSON.stringify(marcados) && cAchado.origin.inventory === laco.occurrence && !('selection' in cAchado.origin) && cAchado.with_facts === false
      && JSON.stringify(Object.keys(cAchado.origin)) === '["inventory","kind","summary"]', JSON.stringify(cAchado.origin).slice(0, 120));
    await x.page.unroute(GATHERED);
    await fecha(x);

    // ================================================================ 7. sequência e Juntar durante o carregamento (3.6, 9.2.8)
    x = await abre(browser);
    await x.page.route(SUGGEST, async r => { await new Promise(z => setTimeout(z, 2500)); return r.continue(); });
    await x.page.click('#areas [data-area="explore"]');
    const outro = await x.page.evaluate(n => { const r = __t.hot.rows.find(x => x.name === n); __t.selectNode(__t.state.index.get(r.symbol)); return r.symbol; }, OUTRO);
    const lento = x.page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
    await x.page.waitForFunction(() => __t.builderLoading, null, { timeout: 10000 });
    await x.page.waitForTimeout(100);
    const durante = await x.page.evaluate(() => { const btn = [...document.querySelectorAll('#inspector button')].find(k => k.textContent === __t.t('gather.add'));
      return { disabled: btn?.disabled, title: btn?.title, visible: document.querySelector('#inspector').innerText, line: document.querySelector('#draftText').textContent, tab: document.querySelector('#centerBuilderState').textContent }; });
    ok('7 9.2.8 durante uma entrada que recomeça, Juntar fica desabilitado com o motivo escrito', durante.disabled === true && /Aguarde: Otimizar · .* está carregando um rascunho novo/.test(durante.title) && /Aguarde: Otimizar/.test(durante.visible), JSON.stringify(durante));
    ok('7 o carregamento aparece no compositor e na barra', /^Rascunho: carregando Otimizar/.test(durante.line) && /carregando/.test(durante.tab), JSON.stringify(durante));
    await x.page.evaluate(id => __t.toggleGathered(id, 'inspection'), outro);
    s = await b(x.page);
    ok('7 9.2.8 Juntar durante o carregamento não se perde em silêncio: nada juntado, com aviso', s.gathered === 0 && /^Nada foi juntado: Juntar fica desabilitado enquanto Otimizar/.test(s.notice), s.notice);
    await lento;
    ok('7 depois do carregamento, Juntar volta a funcionar', await x.page.evaluate(() => !__t.builderLoading && __t.gathered.length > 0 && ![...document.querySelectorAll('#inspector button')].find(k => k.textContent === __t.t('gather.add'))?.disabled));
    // Resposta antiga descartada: Otimizar A (lento) e, antes dele chegar, Otimizar B.
    await x.page.unroute(SUGGEST);
    let primeira = true;
    await x.page.route(SUGGEST, async r => { const lentoA = primeira; primeira = false; if (lentoA) await new Promise(z => setTimeout(z, 3000)); return r.continue(); });
    await x.page.click('#builderClear'); await esperaDialogo(x.page); await confirma(x.page);
    await x.page.fill('#question', '');
    const a1 = x.page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
    await x.page.waitForFunction(() => __t.builderLoading, null, { timeout: 10000 });
    const a2 = x.page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), OUTRO);
    await Promise.all([a1, a2]);
    s = await b(x.page);
    const outroSym = (await linha(x.page, OUTRO)).symbol;
    ok('7 3.6 a resposta antiga é ignorada: fica o rascunho do pedido mais novo', await x.page.evaluate(id => JSON.stringify(__t.seeds) === JSON.stringify([id]), outroSym) && /World::color_at/.test(s.entry), s.entry);
    ok('7 3.6 com o aviso escrito', s.notice === 'Uma sugestão anterior foi descartada: outra foi pedida depois.', s.notice);
    // Resposta de outra run descartada.
    await x.page.unroute(SUGGEST);
    await x.page.route(SUGGEST, async r => { await new Promise(z => setTimeout(z, 2500)); return r.continue(); });
    await x.page.click('#builderClear'); await esperaDialogo(x.page); await confirma(x.page);
    await x.page.fill('#question', '');
    const a3 = x.page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
    await x.page.waitForFunction(() => __t.builderLoading, null, { timeout: 10000 });
    await trocaRun(x.page, OUTRA);
    await a3;
    s = await b(x.page);
    ok('7 3.6 resposta de outra run: nada entra na run nova, com o aviso', s.gathered === 0 && s.notice === `Uma resposta pedida na run ${RUN} foi descartada: a run mudou.`, JSON.stringify([s.gathered, s.notice]));
    await x.page.unroute(SUGGEST);
    await fecha(x);

    // ================================================================ 8. "Pedir esta mudança" pelo Builder (2.4.1)
    x = await abre(browser);
    await x.page.evaluate(() => { const p = document.querySelector('#provider'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
    const nomes = await x.page.evaluate(() => __t.hot.rows.filter(r => r.symbol && r.calls > 0).slice(0, 12).map(r => r.name));
    let cartao = null;
    for (const nm of nomes) {
      const curto = nm.replace(/^impl (.+ for )?/, '').replace(/<[^>]*>/g, '');
      await x.page.fill('#question', `Otimize o ${curto}`); await x.page.click('#ask');
      await x.page.waitForTimeout(1500);
      cartao = await x.page.evaluate(() => __t.perfEntries.at(-1)?.changes?.length ? __t.perfEntries.at(-1).changes[0] : null);
      if (cartao) break;
    }
    if (!cartao) ok('8 nenhuma pergunta reconhecida com mudança nesta run', false);
    else {
      await x.page.evaluate(() => [...document.querySelectorAll('.perf-card button')].find(k => k.textContent === __t.t('perf.toLadder')).click());
      await x.page.waitForFunction(() => __t.gathered.length && !__t.builderLoading, null, { timeout: 60000 });
      await x.page.waitForTimeout(200);
      s = await b(x.page);
      ok('8 o cartão não envia mais: nenhum pedido à escada', x.bodies.ladder.length === 0 && x.writes.length === 0, JSON.stringify(x.writes));
      ok('8 rascunho novo no Builder: alvo, sugestão, texto, escada marcada, modo necessário Melhorar', s.view === 'builder' && await x.page.evaluate(c => JSON.stringify(__t.seeds) === JSON.stringify(c.nodes)
        && document.querySelector('#question').value === c.text && document.querySelector('#ladderOn').checked && __t.gatherFacts === true && __t.draft.required_mode === 'improve' && __t.suggestion != null, cartao)
        && /^Entrada: Pedir esta mudança · /.test(s.entry), s.entry);
      const esperadoL = await x.page.evaluate(() => ({ nodes: __t.gathered.slice(), selection: __t.selectionOnRecord() }));
      await x.page.click('#ask');
      // F7 (T26): a escada usa Melhorar; com a política em Localizar, confirma a mudança da política antes.
      await esperaDialogo(x.page); await confirma(x.page);
      for (let i = 0; i < 40 && !x.bodies.ladder.length; i++) await x.page.waitForTimeout(250);
      const l = x.bodies.ladder.length ? JSON.parse(x.bodies.ladder[0]) : null;
      ok('8 9.5 "Rodar a escada" sem edição: o corpo leva a pergunta, as incluídas e a seleção do rascunho', l && l.question === cartao.text && JSON.stringify(l.nodes) === JSON.stringify(esperadoL.nodes) && l.with_facts === true
        && l.origin.kind === 'ladder' && l.origin.summary === cartao.text.slice(0, 80) && JSON.stringify(l.origin.selection) === JSON.stringify(esperadoL.selection) && l.retries === 0 && l.lacuna_rounds === 0, l ? JSON.stringify(Object.keys(l)) : 'sem corpo');
      ok('8 9.5 as chaves do corpo da escada são as de antes, na mesma ordem', l && JSON.stringify(Object.keys(l)) === '["key","run","question","nodes","with_facts","gate","models","retries","lacuna_rounds","origin"]', l ? JSON.stringify(Object.keys(l)) : '');
    }
    await fecha(x);

    // ================================================================ 9. fluxograma: Juntar num ponto, sem trocar de vista (2.3, Q5)
    x = await abre(browser);
    await x.page.click('#areas [data-area="ruler"]');
    const semente = (await linha(x.page, OUTRO)).symbol;
    await x.page.evaluate(id => __t.openPath(id, 'calls'), semente);
    await x.page.waitForFunction(() => document.querySelectorAll('#pathCanvas .node').length > 1, null, { timeout: 60000 });
    const ponto = await x.page.evaluate(() => { const n = [...document.querySelectorAll('#pathCanvas .node')].find(g => !g.classList.contains('seed')); n.dispatchEvent(new MouseEvent('click', { bubbles: true })); return n.dataset.id; });
    await x.page.waitForTimeout(150);
    ok('9 clicar num ponto mostra "Juntar ao rascunho" no fluxograma', await x.page.evaluate(() => !document.querySelector('#pathPoint').hidden && document.querySelector('#pathPointJoin').textContent === 'Juntar ao rascunho'));
    await x.page.click('#pathPointJoin');
    s = await b(x.page);
    ok('9 T4 o ponto soma uma fonte e o centro continua no fluxograma; a área não muda', s.gathered === 1 && s.view === 'path' && s.area === 'ruler' && await x.page.evaluate(id => __t.gathered[0] === id && __t.draft.arrived.get(id) === 'path', ponto), JSON.stringify(s));
    ok('9 o botão vira "Tirar do rascunho"', await x.page.evaluate(() => document.querySelector('#pathPointJoin').textContent === 'Tirar do rascunho'));
    // Troca de run: o rascunho da run deixada não é levado (F6 guarda por run).
    await trocaRun(x.page, OUTRA);
    s = await b(x.page);
    ok('9 trocar de run: o rascunho não passa para a outra run; a barra continua com o Builder', s.gathered === 0 && s.tab && await x.page.evaluate(() => __t.draft.arrived.size === 0), JSON.stringify(s));
    // Troca de projeto: o Builder do projeto deixado sai da barra.
    await trocaRun(x.page, RUN);
    await juntaPelaInspecao(x.page, INV);
    await x.page.click('#draftOpen');
    await abrePor(x.page, RAIZ_B, CHAVE_B);
    s = await b(x.page);
    ok('9 abrir outro projeto: nada do rascunho nem do Builder do anterior', s.gathered === 0 && !s.tab && !s.shown && s.view === 'city' && s.head === null, JSON.stringify(s));
    await fecha(x);

    // ================================================================ 10. tela estreita
    x = await abre(browser, { size: { width: 390, height: 844 } });
    await x.page.click('#colSwitch [data-col="nav"]');
    await juntaPelaInspecao(x.page, OUTRO);
    ok('10 estreita: Juntar não troca de coluna', await x.page.evaluate(() => __t.navigation.column === 'nav'));
    await x.page.click('#colSwitch [data-col="question"]');
    await x.page.click('#draftOpen');
    s = await b(x.page);
    ok('10 estreita: "Abrir no Builder" leva ao Centro e anuncia', s.view === 'builder' && await x.page.evaluate(() => __t.navigation.column === 'center' && document.querySelector('#announce').textContent === 'Coluna mostrada: Centro: Builder (1)'), await x.page.evaluate(() => document.querySelector('#announce').textContent));
    ok('10 estreita: o resumo da coluna diz "Centro: Builder (N)"', s.col === 'Centro: Builder (1)', s.col);
    ok('10 estreita: sem rolagem horizontal da página com o Builder', await x.page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
    await x.page.click('#colSwitch [data-col="nav"]');
    await x.page.click('#areas [data-area="explore"]');
    await x.page.click('#colSwitch [data-col="question"]');
    const ot = x.page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
    await esperaDialogo(x.page); await confirma(x.page); await ot;
    await x.page.waitForFunction(() => !__t.builderLoading, null, { timeout: 60000 });
    ok('10 estreita: Otimizar leva ao Centro (Builder) e anuncia', await x.page.evaluate(() => __t.navigation.column === 'center' && /^Coluna mostrada: Centro: Builder \(\d+\)$/.test(document.querySelector('#announce').textContent)), await x.page.evaluate(() => document.querySelector('#announce').textContent));
    ok('10 estreita: trocar de coluna não descarta o rascunho', await (async () => { const a = await rascunho(x.page); await x.page.click('#colSwitch [data-col="nav"]'); await x.page.click('#colSwitch [data-col="center"]'); return a === await rascunho(x.page) && (await b(x.page)).shown; })());
    ok('10 estreita: o Builder cabe na largura (sem rolagem horizontal dentro dele)', await x.page.evaluate(() => { const v = document.querySelector('#builderView'); return v.scrollWidth <= v.clientWidth + 1; }));
    await fecha(x);

    // ================================================================ 11. inglês, temas, teclado
    x = await abre(browser, { scheme: 'dark', locale: 'en-US' });
    await x.page.click('[data-lang="en"]');
    await juntaPelaInspecao(x.page, INV);
    await x.page.click('#draftOpen');
    await x.page.click(`#gathered button:has-text("Suggest envelope")`);
    await x.page.waitForFunction(() => __t.suggestion, null, { timeout: 60000 });
    s = await b(x.page);
    const textoEN = await x.page.evaluate(() => document.querySelector('#builderView').innerText + ' ' + document.querySelector('#draftLine').innerText + ' ' + document.querySelector('#headDraft').textContent + ' ' + document.querySelector('#centerBar').innerText);
    ok('11 EN: o Builder em inglês', /^1 source\(s\) included · target: /.test(s.state) && /Included \(1\) · go to the model/.test(s.record) && /Not joined \(\d+\) · suggested and not brought in/.test(s.record) && /arrived by: inspection · local only/.test(s.record) && /^Draft: 1 source\(s\) · target /.test(s.line) && s.head === 'Draft: 1 source(s) · kept in this browser', JSON.stringify([s.state, s.line, s.head]));
    ok('11 EN: nenhuma chave crua nem frase em português no Builder', !/builder\.|draft\.|envelope\.|Incluídas|Rascunho|chegou por|só nesta página/.test(textoEN), textoEN.match(/builder\.\S+|Incluídas|Rascunho|chegou por/)?.[0]);
    let c2 = await cores(x.page);
    ok('11 tema escuro: contraste do estado e das fontes ≥ 4,5', contraste(c2.state, c2.bg) >= 4.5 && contraste(c2.src, c2.bg) >= 4.5 && contraste(c2.says, c2.bg) >= 4.5, JSON.stringify(c2));
    await x.page.click('#theme');
    c2 = await cores(x.page);
    ok('11 troca manual para claro: contraste ≥ 4,5', contraste(c2.state, c2.bg) >= 4.5 && contraste(c2.src, c2.bg) >= 4.5 && contraste(c2.says, c2.bg) >= 4.5, JSON.stringify(c2));
    await x.page.click('[data-lang="pt"]');
    ok('11 voltar a PT redesenha o Builder', /^1 fonte\(s\) incluída\(s\)/.test((await b(x.page)).state));
    // Teclado: Cidade → Builder → × pela barra.
    await x.page.focus('#centerCity');
    await x.page.keyboard.press('Tab');
    const foco1 = await x.page.evaluate(() => document.activeElement?.id);
    await x.page.keyboard.press('Tab');
    const foco2 = await x.page.evaluate(() => document.activeElement?.id);
    ok('11 teclado: Tab percorre Cidade → Builder → ×', foco1 === 'centerBuilder' && foco2 === 'centerBuilderClose', `${foco1} ${foco2}`);
    await x.page.focus('#centerCity'); await x.page.keyboard.press('Enter');
    await x.page.focus('#centerBuilder'); await x.page.keyboard.press('Enter');
    ok('11 teclado: Enter na aba traz o Builder', (await b(x.page)).view === 'builder');
    ok('11 contorno visível no foco pelo teclado', await x.page.evaluate(() => getComputedStyle(document.activeElement).outlineStyle !== 'none'));
    await x.page.keyboard.press('Tab'); await x.page.keyboard.press('Enter');
    ok('11 teclado: Enter no × fecha o Builder sem perder o rascunho', !(await b(x.page)).tab && (await b(x.page)).gathered === 1);
    ok('11 semântica: a vista do Builder é uma região nomeada pelo título; os botões de lista têm nome com a fonte', await x.page.evaluate(() => { document.querySelector('#draftOpen').click(); const v = document.querySelector('#builderView');
      return v.tagName === 'SECTION' && v.getAttribute('aria-labelledby') === 'builderTitle' && [...v.querySelectorAll('.src button')].every(k => /: \S/.test(k.getAttribute('aria-label') ?? '')); }));
    const chaves = await x.page.evaluate(() => Object.keys(__t.STRINGS).filter(k => /^(builder|draft|col\.builder|head\.draft|path\.point)/.test(k)).filter(k => !(Array.isArray(__t.STRINGS[k]) && __t.STRINGS[k].length === 2 && __t.STRINGS[k].every(v => typeof v === 'string' && v))));
    ok('11 as chaves novas existem em PT e EN', chaves.length === 0, chaves.join(', '));
    await fecha(x);
  } finally {
    await browser.close();
  }
  // ================================================================ 12. nada gravado
  ok('12 nenhuma escrita chegou ao servidor (as tentativas foram abortadas ou respondidas aqui)', todasEscritas.every(w => w === 'POST /api/ladder'), JSON.stringify([...new Set(todasEscritas)]));
  ok('12 o perfil do projeto ficou igual em bytes', sha(perfil) === perfilAntes);
  ok('12 nenhuma pergunta nova na run', (await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`)).json()).length === turnosAntes);
  ok('12 nenhum erro de página', todosErros.length === 0, todosErros.join(' | '));
  if (OUT) fs.writeFileSync(path.join(OUT, 'tela-builder.json'), JSON.stringify({ escritas: todasEscritas, erros: todosErros }, null, 1));
})().catch(e => { console.error(e); process.exit(1); });

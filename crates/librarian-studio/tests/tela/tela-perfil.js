// F7: o compositor e a gravação do perfil (especificação 2.10, 3.10 a 3.13, 5.4, 5.5 passos 7 e 8, 7.3, T20, T22,
// T24, T25 (modo), T26; critérios 9.3 e 9.4 e a tabela 3.11), na página real, num Chrome sem janela, contra o
// servidor de rodar.js (8841).
// Casos: abrir projetos sincroniza o modo com a política de cada um, com qualquer valor em librarian.mode, sem
// gravar; política personalizada; Alterar a política grava só allow_hypothesis e allow_patch (inclusive em bytes,
// no project.json de uma cópia), muda o seletor só depois do sucesso e deixa as edições não salvas como estão;
// falha simulada; perfil alterado fora; Salvar mostra a lista e grava o perfil relido mais as edições; nenhum
// outro fluxo grava (Gerar, Exportar, Sugerir e Aplicar léxico, Preparar, Perguntar, escada, cadeia, lotes,
// caminho); T26 em cada ação que exige um modo; Otimizar e o cartão não trocam o modo; critérios do rascunho
// (editar, bloquear, Salvar como padrão, Desfazer, guardar e restaurar, o padrão que mudou, por run); o texto
// fica depois de preparar; o caminho previsto; o indicador de não salvo; tela estreita, PT e EN, temas e teclado.
// Nada é enviado a modelo. O perfil é respondido por esta bateria (um perfil em memória por projeto, com a mesma
// validação do servidor); só o caso 2b deixa PUT /api/project chegar ao servidor e restaura o project.json depois.
// As criações de pergunta são respondidas aqui, sem gravar; as outras escritas são abortadas e contadas.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium, RENDERER, CHAVE_RENDERER } = require('./ambiente');
const B = 'http://127.0.0.1:8841', RUN = '20261104-020244', OUTRA = '20261104-014710';
const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS;
const HOOK = "window.__t = { state, studio, t, STRINGS, optimize, investigate, toggleGathered, selectionOnRecord, loadRun, continueTurn, goTo, selectNode,"
  + " get mode() { return mode; }, get gathered() { return gathered; }, get seeds() { return seeds; }, get investigation() { return investigation; },"
  + " get draft() { return draft; }, get hot() { return hot; }, get perfEntries() { return perfEntries; }, get draftCriteria() { return draftCriteria; },"
  + " get pathTargets() { return pathTargets; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 300) : ''}`); if (!pass) process.exitCode = 1; };
const perfil = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'project.json');
const sha = f => crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const RAIZ_B = 'C:/projeto-ficticio-da-bateria/b', CHAVE_B = 'ficticio-b-00000000';
const RAIZ_C = 'C:/projeto-ficticio-da-bateria/c', CHAVE_C = 'ficticio-c-00000000';
const NOME = run => `librarian.draft.v1:${CHAVE_RENDERER}:${run}`;
const INV = 'impl Matrix<4>::inverse';
const LEITURA = (method, pathname) => method === 'GET' && pathname !== '/api/project'
  || (method === 'POST' && ['/api/calls', '/api/envelope/suggest', '/api/inventory/investigate', '/api/performance-question'].includes(pathname));
const CRIA = ['/api/gathered-question', '/api/ask', '/api/path-questions', '/api/ladder', '/api/continue'];
const diferentes = (a, b) => [...new Set([...Object.keys(a), ...Object.keys(b)])].filter(k => JSON.stringify(a[k]) !== JSON.stringify(b[k])).sort();

// O perfil de cada projeto, em memória: o do renderer começa igual ao gravado no servidor; B (Melhorar) e C
// (política personalizada) são fictícios, sem runs. A validação é a de store.rs (save_settings).
let real = null, base = null;
async function perfis() {
  if (!real) {
    real = await (await fetch(`${B}/api/project?key=${CHAVE_RENDERER}`)).json();
    const turns = await (await fetch(`${B}/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`)).json();
    base = turns.filter(t => t.gathered).at(-1) ?? turns.at(-1);
  }
  return {
    [CHAVE_RENDERER]: structuredClone(real.settings),
    [CHAVE_B]: { ...structuredClone(real.settings), name: 'ficticio-b', root: RAIZ_B, allow_hypothesis: true, allow_patch: true, criteria: 'Critério de B' },
    [CHAVE_C]: { ...structuredClone(real.settings), name: 'ficticio-c', root: RAIZ_C, allow_hypothesis: true, allow_patch: false },
  };
}
const projeto = (x, key) => ({ key, settings: structuredClone(x.store[key]), runs: key === CHAVE_RENDERER ? real.runs : [] });

async function contexto(browser, { size = { width: 1440, height: 900 }, scheme = 'light', locale = 'pt-BR', storage = null } = {}) {
  const context = await browser.newContext({ viewport: size, colorScheme: scheme, locale });
  if (storage) await context.addInitScript(s => { if (!sessionStorage.getItem('f7.semeado')) { for (const [k, v] of Object.entries(s)) localStorage.setItem(k, v); sessionStorage.setItem('f7.semeado', '1'); } }, storage);
  return context;
}
// Uma página do Studio aberta no renderer, na run RUN. `x.store`: os perfis; `x.puts`: os corpos de PUT
// /api/project; `x.creates`: os corpos das criações de pergunta; `x.writes`: as outras escritas (abortadas).
async function pagina(context, { run = RUN, store = null } = {}) {
  const page = await context.newPage();
  const x = { page, context, errors: [], writes: [], puts: [], creates: [], gets: 0, store: store ?? await perfis(), falhaPut: null, realPut: false, lexiconAnswer: null };
  page.on('pageerror', e => x.errors.push(e.message));
  await page.route(u => new URL(u).pathname === '/', async r => { const y = await r.fetch(); r.fulfill({ response: y, body: (await y.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.route(u => new URL(u).pathname.startsWith('/api/'), async r => {
    const req = r.request(), url = new URL(req.url()), { pathname } = url, method = req.method();
    const body = req.postData() ? JSON.parse(req.postData()) : null;
    if (pathname === '/api/project') {
      if (method === 'POST') {
        const key = body.root === RAIZ_B ? CHAVE_B : body.root === RAIZ_C ? CHAVE_C : CHAVE_RENDERER;
        if (key === CHAVE_RENDERER && x.realPut) return r.continue();
        return r.fulfill({ json: projeto(x, key) });
      }
      if (method === 'GET') { x.gets++; if (x.realPut) return r.continue(); return r.fulfill({ json: projeto(x, url.searchParams.get('key')) }); }
      if (method === 'PUT') {
        x.puts.push(body);
        if (x.realPut) return r.continue();
        if (x.falhaPut) return r.fulfill({ status: 400, json: { error: x.falhaPut } });
        const s = body.settings, atual = x.store[body.key];
        if (s.root !== atual.root) return r.fulfill({ status: 400, json: { error: 'A pasta do projeto não muda; abra outra pasta como outro projeto' } });
        if (!s.inputs.length || !s.hits || s.hits > s.top) return r.fulfill({ status: 400, json: { error: 'Informe entradas e 1 <= hits <= top' } });
        x.store[body.key] = structuredClone(s);
        return r.fulfill({ json: projeto(x, body.key) });
      }
    }
    // Outros projetos fictícios leem o grafo do renderer (não têm runs, mas as leituras gerais existem).
    if ([CHAVE_B, CHAVE_C].includes(url.searchParams.get('key')) && method === 'GET') {
      url.searchParams.set('key', CHAVE_RENDERER);
      return r.continue({ url: url.toString() });
    }
    if (LEITURA(method, pathname)) return r.continue();
    if (method === 'POST' && CRIA.includes(pathname)) {
      x.creates.push({ path: pathname, raw: req.postData(), body });
      if (pathname === '/api/path-questions') return r.fulfill({ json: [] });
      if (pathname === '/api/ladder') return r.fulfill({ json: { passed: false, turn: null, nodes: [], steps: [] } });
      return r.fulfill({ json: { ...base, id: 900000 + x.creates.length, question: body.question, query: null, prepare_error: null, sends: [] } });
    }
    if (method === 'POST' && pathname === '/api/lexicon/suggest' && x.lexiconAnswer) { x.writes.push('POST /api/lexicon/suggest'); return r.fulfill({ json: x.lexiconAnswer }); }
    x.writes.push(`${method} ${pathname}`);
    return r.abort();
  });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  if (await page.evaluate(() => __t.studio.project?.key) !== CHAVE_RENDERER) await abrePor(page, RENDERER, CHAVE_RENDERER);
  await page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
  if (run && await page.evaluate(() => __t.studio.run) !== run) await trocaRun(page, run);
  await page.waitForFunction(() => __t.studio.commands, null, { timeout: 60000 });
  await page.waitForTimeout(400);
  return x;
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
async function recarrega(page, run = RUN) {
  await page.reload();
  await page.waitForFunction(() => window.__t && __t.studio.project && (__t.studio.run ? __t.state.nodes.length : true), null, { timeout: 120000 });
  if (run && await page.evaluate(() => __t.studio.run) !== run) await trocaRun(page, run);
  await page.waitForTimeout(800);
}
const fecha = x => x.context.close();
const esperaDialogo = page => page.waitForFunction(() => document.querySelector('#askDialog').open, null, { timeout: 30000 });
const dialogo = page => page.evaluate(() => ({ title: document.querySelector('#askTitle').textContent, message: document.querySelector('#askMessage').textContent,
  ok: document.querySelector('#askOk').hidden ? null : document.querySelector('#askOk').textContent,
  choices: [...document.querySelectorAll('#askChoices button')].map(b => b.textContent), focusIn: document.querySelector('#askDialog').contains(document.activeElement) }));
const confirma = async page => { await page.click('#askOk'); await page.waitForTimeout(400); };
const cancela = async page => { await page.click('#askDialog [data-close]'); await page.waitForTimeout(300); };
const escolhe = async (page, texto) => { await page.evaluate(t => [...document.querySelectorAll('#askChoices button')].find(b => b.textContent === t).click(), texto); await page.waitForTimeout(400); };
const seletor = page => page.evaluate(() => [...document.querySelectorAll('#modeSeg [data-mode]')].filter(b => b.getAttribute('aria-pressed') === 'true').map(b => b.dataset.mode).join() || 'nenhum');
const vis = (page, s) => page.evaluate(q => { const e = document.querySelector(q); return e && !e.hidden && e.offsetParent !== null ? e.textContent : ''; }, s);
// Muda um campo da tela como a pessoa faria (dispara input e change).
const edita = (page, sel, valor) => page.evaluate(([s, v]) => { const e = document.querySelector(s); if (e.type === 'checkbox') e.checked = v; else e.value = v; e.dispatchEvent(new Event('input', { bubbles: true })); e.dispatchEvent(new Event('change', { bubbles: true })); }, [sel, valor]);
const manual = page => page.evaluate(() => { const p = document.querySelector('#provider'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
async function otimiza(page) {
  await page.waitForFunction(() => __t.hot?.rows?.length, null, { timeout: 120000 });
  await page.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
  await page.waitForTimeout(300);
  if (await page.evaluate(() => document.querySelector('#askDialog').open)) await confirma(page);
  await page.waitForFunction(() => __t.gathered.length && !document.querySelector('#centerBuilderState').textContent, null, { timeout: 60000 });
  await page.waitForTimeout(300);
}
async function juntaUma(page) {
  await page.waitForFunction(() => __t.hot?.rows?.length, null, { timeout: 120000 });
  const id = await page.evaluate(n => __t.hot.rows.find(r => r.name === n).symbol, INV);
  await page.evaluate(i => __t.toggleGathered(i, 'inspect'), id);
  await page.waitForTimeout(200);
  return id;
}
// Separar a primeira função com linhas: a criação passa pelo lote (askHotFacts).
async function separa(page) {
  await page.evaluate(() => __t.goTo('explore', 'hot'));
  await page.waitForFunction(() => document.querySelector('#hotPick')?.options.length > 1, null, { timeout: 60000 });
  return page.evaluate(() => { const sel = document.querySelector('#hotPick'); for (const o of [...sel.options].slice(1)) { sel.value = o.value; sel.dispatchEvent(new Event('change')); const b = document.querySelector('#hotList .hot-sep button.primary'); if (b && !b.disabled) { b.click(); return o.value; } } return null; });
}
// Um cartão de desempenho com "Pedir esta mudança" (como a captura do F0): perguntas "Otimize o …" sem modelo.
async function cartaoComMudanca(page) {
  // A medição da run carrega depois da página (como nas linhas acima, que já esperam por ela).
  await page.waitForFunction(() => __t.hot?.rows?.length, null, { timeout: 120000 });
  const names = await page.evaluate(() => __t.hot.rows.filter(x => x.symbol && x.calls > 0).slice(0, 12).map(x => x.name));
  await manual(page);
  for (const nm of names) {
    const short = nm.replace(/^impl (.+ for )?/, '').replace(/<[^>]*>/g, '');
    await page.fill('#question', `Otimize o ${short}`); await page.click('#ask'); await page.waitForTimeout(1500);
    const card = await page.evaluate(() => __t.perfEntries.at(-1));
    if (card?.changes?.length) return card;
  }
  return null;
}
const contraste = cores => {
  const L = k => { const m = k.match(/[\d.]+/g).slice(0, 3).map(Number).map(v => v / 255).map(v => v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4); return 0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]; };
  const [hi, lo] = [L(cores[0]), L(cores[1])].sort((p, q) => q - p);
  return (hi + 0.05) / (lo + 0.05);
};

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const perfilAntes = sha(perfil);
  const todosErros = [], todasEscritas = [];
  let x, s, d;
  const junta = y => { todosErros.push(...y.errors); todasEscritas.push(...y.writes.filter(w => !y.esperadas?.includes(w))); };
  try {
    // ================================================================ 1. abrir projetos: o modo é a política de cada um (9.4.3, 9.4.6, T25, 3.13.4)
    for (const chave of ['improve', 'locate', null]) {
      const ctx = await contexto(browser, { storage: chave ? { 'librarian.mode': chave } : {} });
      x = await pagina(ctx);
      ok(`1 librarian.mode=${chave}: o renderer (perfil em Localizar) abre em Localizar`, await seletor(x.page) === 'locate', await seletor(x.page));
      await abrePor(x.page, RAIZ_B, CHAVE_B);
      ok(`1 librarian.mode=${chave}: B (perfil em Melhorar) abre em Melhorar`, await seletor(x.page) === 'improve', await seletor(x.page));
      await abrePor(x.page, RENDERER, CHAVE_RENDERER);
      await x.page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
      ok(`1 librarian.mode=${chave}: voltar ao renderer mostra Localizar`, await seletor(x.page) === 'locate', await seletor(x.page));
      await abrePor(x.page, RAIZ_B, CHAVE_B);
      await recarrega(x.page, null);
      ok(`1 librarian.mode=${chave}: recarregar com B aberto mostra Melhorar`, await seletor(x.page) === 'improve' && await x.page.evaluate(() => __t.studio.project.key) === CHAVE_B, await seletor(x.page));
      ok(`1 librarian.mode=${chave}: nenhuma abertura grava o perfil`, x.puts.length === 0, x.puts.length);
      ok(`1 3.13.4 librarian.mode não é lida, gravada nem apagada`, await x.page.evaluate(() => localStorage.getItem('librarian.mode')) === chave, await x.page.evaluate(() => localStorage.getItem('librarian.mode')));
      junta(x); await fecha(x);
    }
    // Política personalizada (9.4.6).
    x = await pagina(await contexto(browser));
    await abrePor(x.page, RAIZ_C, CHAVE_C);
    ok('1 9.4.6 política personalizada: nenhum modo marcado', await seletor(x.page) === 'nenhum', await seletor(x.page));
    ok('1 9.4.6 o texto "Política personalizada: hipótese sim, patch não"', await vis(x.page, '#modeState') === 'Política personalizada: hipótese sim, patch não', await vis(x.page, '#modeState'));
    ok('1 9.4.6 abrir não altera o perfil', x.puts.length === 0 && x.store[CHAVE_C].allow_hypothesis === true && x.store[CHAVE_C].allow_patch === false);
    // Escolher um modo é a operação explícita, que iguala os dois campos.
    await x.page.click('#modeSeg [data-mode="locate"]');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('1 escolher um modo na política personalizada pede confirmação, com o que o modo permite', d.title === 'Alterar a política do modo' && /Localizar: só fatos; hipóteses e mudanças não permitidas\. Grava só hipótese e patch/.test(d.message), JSON.stringify(d));
    await confirma(x.page);
    ok('1 depois de confirmar: os dois campos iguais (false/false) e o seletor em Localizar', x.store[CHAVE_C].allow_hypothesis === false && x.store[CHAVE_C].allow_patch === false && await seletor(x.page) === 'locate' && await vis(x.page, '#modeState') === '');
    junta(x); await fecha(x);

    // ================================================================ 2. Alterar a política (9.4.2, T20, 3.13.2)
    x = await pagina(await contexto(browser));
    await otimiza(x.page);
    // Edições não salvas em Configurações, Léxico e Entradas, e critérios do rascunho editados.
    await x.page.evaluate(() => __t.goTo('settings'));
    await edita(x.page, '#top', '7');
    await x.page.evaluate(() => __t.goTo('lexicon'));
    await edita(x.page, '#lexicon', '{ "groups": [["matriz", "matrix"]], "stop_words": [] }');
    await x.page.evaluate(() => __t.goTo('project'));
    await edita(x.page, '#inputs', 'src, tests');
    await edita(x.page, '#criteria', 'Critério do rascunho do F7');
    s = await vis(x.page, '#headUnsaved');
    // Fechamento do F9: os critérios próprios do rascunho não são alteração não salva do perfil; o cabeçalho os diz na linha do rascunho.
    ok('2 1.2 o cabeçalho diz onde há alterações não salvas do perfil; os critérios do rascunho, na linha do rascunho', s === 'Alterações não salvas: Entradas (Projeto), Léxico, Configurações'
      && (await vis(x.page, '#headDraft')).endsWith(' · critérios próprios, não salvos como padrão'), `${s} | ${await vis(x.page, '#headDraft')}`);
    ok('2 7.4 cada lugar marca as suas', await x.page.evaluate(() => [document.querySelector('#settingsUnsaved').textContent, document.querySelector('#lexiconUnsaved').textContent, document.querySelector('#inputsUnsaved').textContent].join(' | '))
      === 'Alterações não salvas: top | Alterações não salvas: Léxico | Alterações não salvas: Entradas');
    ok('2 editar não grava nada', x.puts.length === 0);
    const antes = structuredClone(x.store[CHAVE_RENDERER]);
    await x.page.click('#modeSeg [data-mode="improve"]');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('2 o seletor pede confirmação: o modo novo e o que ele permite', d.title === 'Alterar a política do modo' && d.message.startsWith('Alterar a política do projeto para Melhorar?') && /Melhorar: hipóteses e mudanças permitidas\. Grava só hipótese e patch no perfil do projeto; outras alterações não salvas continuam não salvas\./.test(d.message) && d.ok === 'Alterar a política', JSON.stringify(d));
    ok('2 releu o perfil antes de perguntar (3.13.3)', x.gets >= 1, x.gets);
    ok('2 antes de confirmar, o seletor continua em Localizar', await seletor(x.page) === 'locate');
    await cancela(x.page);
    ok('2 Cancelar: nada gravado, seletor em Localizar', x.puts.length === 0 && await seletor(x.page) === 'locate');
    await x.page.click('#modeSeg [data-mode="improve"]');
    await esperaDialogo(x.page); await confirma(x.page);
    await x.page.waitForFunction(() => __t.mode === 'improve', null, { timeout: 30000 });
    const put = x.puts.at(-1);
    ok('2 9.4.2 um PUT, que difere do perfil relido só em allow_hypothesis e allow_patch', x.puts.length === 1 && JSON.stringify(diferentes(antes, put.settings)) === '["allow_hypothesis","allow_patch"]' && put.settings.allow_hypothesis && put.settings.allow_patch, JSON.stringify(diferentes(antes, put.settings)));
    ok('2 9.4.2 o PUT não leva top, léxico, entradas nem critérios editados', put.settings.top === antes.top && JSON.stringify(put.settings.lexicon) === JSON.stringify(antes.lexicon) && JSON.stringify(put.settings.inputs) === JSON.stringify(antes.inputs) && put.settings.criteria === antes.criteria);
    ok('2 o seletor muda depois do sucesso, com "Política do projeto alterada: Melhorar (hipóteses e mudanças permitidas)"', await seletor(x.page) === 'improve' && await vis(x.page, '#modeNote') === 'Política do projeto alterada: Melhorar (hipóteses e mudanças permitidas)', await vis(x.page, '#modeNote'));
    ok('2 7.1 a mensagem não diz "salvo"', !/salv/i.test(await vis(x.page, '#modeNote')));
    ok('2 9.4.2 as edições continuam na tela e marcadas não salvas', await x.page.evaluate(() => document.querySelector('#top').value === '7' && /matriz/.test(document.querySelector('#lexicon').value) && document.querySelector('#inputs').value === 'src, tests' && __t.draftCriteria === 'Critério do rascunho do F7')
      && await vis(x.page, '#headUnsaved') === s, await vis(x.page, '#headUnsaved'));
    ok('2 o rascunho e o modo necessário ficam como estavam', await x.page.evaluate(() => __t.gathered.length > 0 && __t.draft.required_mode === 'improve'));
    // Pelo teclado: Tab chega ao seletor, Enter abre a confirmação, Escape cancela e o foco volta.
    await x.page.focus('#modeSeg [data-mode="locate"]');
    await x.page.keyboard.press('Enter');
    await esperaDialogo(x.page);
    ok('2 9.12.6 teclado: Enter no seletor abre a confirmação, com o foco preso no diálogo', (await dialogo(x.page)).focusIn);
    await x.page.keyboard.press('Escape'); await x.page.waitForTimeout(300);
    ok('2 Escape cancela: nada gravado e o foco volta ao seletor', x.puts.length === 1 && await x.page.evaluate(() => document.activeElement === document.querySelector('#modeSeg [data-mode="locate"]')) && await seletor(x.page) === 'improve');
    junta(x); await fecha(x);

    // 2b. Em bytes, no servidor de verdade (cópia da raiz): o project.json depois só difere em dois campos.
    {
      const bytes = fs.readFileSync(perfil);
      x = await pagina(await contexto(browser));
      x.realPut = true;
      await x.page.evaluate(() => __t.goTo('settings'));
      await edita(x.page, '#hits', '2');
      await x.page.click('#modeSeg [data-mode="improve"]');
      await esperaDialogo(x.page); await confirma(x.page);
      await x.page.waitForFunction(() => __t.mode === 'improve', null, { timeout: 30000 });
      const depois = JSON.parse(fs.readFileSync(perfil, 'utf8')), antesJ = JSON.parse(bytes.toString('utf8'));
      ok('2b 9.4.2 project.json real: só allow_hypothesis e allow_patch mudaram', JSON.stringify(diferentes(antesJ, depois)) === '["allow_hypothesis","allow_patch"]', JSON.stringify(diferentes(antesJ, depois)));
      ok('2b a edição de hits continua não salva', await x.page.evaluate(() => document.querySelector('#hits').value) === '2' && /hits/.test(await vis(x.page, '#headUnsaved') + await x.page.evaluate(() => document.querySelector('#settingsUnsaved').textContent)));
      junta(x); await fecha(x);
      fs.writeFileSync(perfil, bytes);
      ok('2b o project.json da cópia volta aos bytes de antes', sha(perfil) === perfilAntes);
    }

    // ================================================================ 3. falha simulada (9.4.5) e perfil alterado fora (9.4.8)
    x = await pagina(await contexto(browser));
    x.store[CHAVE_RENDERER].inputs = [];
    const comEntradasVazias = JSON.stringify(x.store[CHAVE_RENDERER]);
    await x.page.click('#modeSeg [data-mode="improve"]');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('3 9.4.8 o perfil mudou fora (entradas): a confirmação diz antes de continuar', d.message.startsWith('O perfil foi alterado fora desta página: Entradas'), d.message);
    await confirma(x.page);
    await x.page.waitForTimeout(500);
    ok('3 9.4.5 o servidor recusa: o seletor continua em Localizar', await seletor(x.page) === 'locate' && await x.page.evaluate(() => __t.mode) === 'locate');
    ok('3 9.4.5 "A política não foi alterada: {mensagem}. O modo continua Localizar"', await vis(x.page, '#modeNote') === 'A política não foi alterada: Informe entradas e 1 <= hits <= top. O modo continua Localizar', await vis(x.page, '#modeNote'));
    ok('3 9.4.5 o perfil fica igual', JSON.stringify(x.store[CHAVE_RENDERER]) === comEntradasVazias);
    // Rede interrompida: o mesmo resultado.
    x.store[CHAVE_RENDERER].inputs = real.settings.inputs;
    const PROJ = u => new URL(u).pathname === '/api/project';
    await x.page.route(PROJ, r => r.request().method() === 'PUT' ? r.abort() : r.fallback());
    await x.page.click('#modeSeg [data-mode="improve"]');
    await esperaDialogo(x.page); await confirma(x.page); await x.page.waitForTimeout(500);
    ok('3 9.4.5 rede interrompida: seletor em Localizar e "A política não foi alterada"', await seletor(x.page) === 'locate' && /^A política não foi alterada: .*O modo continua Localizar$/.test(await vis(x.page, '#modeNote')), await vis(x.page, '#modeNote'));
    await x.page.unroute(PROJ);
    // Outra aba gravou o perfil depois da leitura: Salvar mostra a diferença e não a desfaz.
    await x.page.evaluate(() => __t.goTo('settings'));
    await edita(x.page, '#hits', '2');
    x.store[CHAVE_RENDERER].top = 9;
    x.store[CHAVE_RENDERER].instructions = ['Instrução gravada por outra aba'];
    await x.page.click('#saveSettings');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('3 9.4.8 Salvar: "O perfil foi alterado fora desta página: Instruções, top" antes da lista', d.message.startsWith('O perfil foi alterado fora desta página: Instruções, top') && /Salvar estas alterações no perfil do projeto\? \n• hits: 1 → 2$/.test(d.message), d.message);
    await confirma(x.page);
    await x.page.waitForTimeout(300);
    const salvo = x.puts.at(-1).settings;
    ok('3 Salvar grava o perfil relido mais a edição: hits 2, e top 9 e as instruções da outra aba ficam', salvo.hits === 2 && salvo.top === 9 && salvo.instructions[0] === 'Instrução gravada por outra aba');
    ok('3 os campos não editados seguem o perfil relido na tela', await x.page.evaluate(() => document.querySelector('#top').value === '9' && document.querySelector('#instructions').value === 'Instrução gravada por outra aba'));
    junta(x); await fecha(x);

    // ================================================================ 4. Salvar (T24, 9.4.1) e Aplicar léxico (2.10)
    x = await pagina(await contexto(browser));
    await x.page.evaluate(() => __t.goTo('settings'));
    await x.page.click('#saveSettings');
    await x.page.waitForTimeout(300);
    ok('4 Salvar sem alterações: "Nada a salvar", sem PUT', !await x.page.evaluate(() => document.querySelector('#askDialog').open) && x.puts.length === 0 && /^Nada a salvar/.test(await vis(x.page, '#settingsNote')), await vis(x.page, '#settingsNote'));
    await edita(x.page, '#hits', '3');
    await edita(x.page, '#top', '6');
    await edita(x.page, '#excludeTests', true);
    await x.page.click('#saveSettings');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('4 7.3 "Salvar estas alterações no perfil do projeto?" com cada campo, antes e depois', d.title === 'Salvar no perfil do projeto' && d.message === 'Salvar estas alterações no perfil do projeto? \n• top: 5 → 6\n• hits: 1 → 3\n• Deixar os testes de fora: não → sim' && d.ok === 'Salvar', JSON.stringify(d));
    await cancela(x.page);
    ok('4 Cancelar: nada gravado e as edições ficam', x.puts.length === 0 && await x.page.evaluate(() => document.querySelector('#hits').value) === '3' && /hits/.test(await x.page.evaluate(() => document.querySelector('#settingsUnsaved').textContent)));
    await x.page.click('#saveSettings');
    await esperaDialogo(x.page); await confirma(x.page);
    await x.page.waitForTimeout(300);
    ok('4 confirmar: um PUT, com só os três campos diferentes do perfil relido', x.puts.length === 1 && JSON.stringify(diferentes(real.settings, x.puts[0].settings)) === '["exclude_tests","hits","top"]', JSON.stringify(diferentes(real.settings, x.puts[0].settings)));
    ok('4 7.3 "Configurações salvas: top, hits, Deixar os testes de fora" e nada mais não salvo', await vis(x.page, '#settingsNote') === 'Configurações salvas: top, hits, Deixar os testes de fora' && await vis(x.page, '#headUnsaved') === '' && await x.page.evaluate(() => document.querySelector('#settingsUnsaved').hidden), await vis(x.page, '#settingsNote'));
    ok('4 7.1 Salvar não diz "política alterada"', !/política/i.test(await vis(x.page, '#settingsNote')));
    // Léxico inválido: Salvar recusa na página, sem PUT.
    await x.page.evaluate(() => __t.goTo('lexicon'));
    await edita(x.page, '#lexicon', '{ isto não é JSON');
    await x.page.click('#saveLexicon');
    await x.page.waitForTimeout(300);
    ok('4 léxico inválido: Salvar léxico recusa com a mensagem, sem PUT', x.puts.length === 1 && /^Léxico: JSON inválido/.test(await vis(x.page, '#lexiconNote')), await vis(x.page, '#lexiconNote'));
    // F0, pendência 6: o léxico mal digitado não bloqueia mais preparar (nada mais salva por baixo).
    await juntaUma(x.page); await manual(x.page);
    await x.page.fill('#question', 'F7 com léxico inválido na tela');
    await x.page.click('#ask');
    await x.page.waitForTimeout(800);
    ok('4 léxico inválido na tela não bloqueia Preparar, e Preparar não grava', x.creates.length === 1 && x.puts.length === 1, `${x.creates.length} ${x.puts.length}`);
    // Aplicar uma proposta: leva ao texto e abre Salvar; Cancelar deixa não salvo.
    await edita(x.page, '#lexicon', JSON.stringify(real.settings.lexicon, null, 2));
    x.lexiconAnswer = { id: 'f7', query: 'pedido', send: { state: 'completed', modelfile: null }, proposal: { accepted: 1, rejected: 0, truncated: false,
      report: [{ status: 'accepted', words: ['matriz', 'matrix'], code_terms: ['matrix'], notes: [] }], lexicon: { groups: [['matriz', 'matrix']], stop_words: [] }, stop_report: [], stop_accepted: 0, stop_rejected: 0 } };
    x.esperadas = ['POST /api/lexicon/suggest'];
    await x.page.click('#suggestLexicon');
    await x.page.waitForFunction(() => !document.querySelector('#lexiconProposal').hidden, null, { timeout: 30000 });
    ok('4 2.10 Sugerir léxico não grava o perfil', x.puts.length === 1);
    await x.page.evaluate(() => document.querySelector('#lexiconProposal button.primary').click());
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('4 2.10 Aplicar abre Salvar com a lista: "Léxico: 0 grupo(s)… → 1 grupo(s)…"', /^Salvar estas alterações no perfil do projeto\? \n• Léxico: \d+ grupo\(s\), \d+ stop word\(s\) → 1 grupo\(s\), 0 stop word\(s\)$/.test(d.message), d.message);
    await cancela(x.page);
    ok('4 2.10 Cancelar: a proposta fica no texto, como não salva', x.puts.length === 1 && /matriz/.test(await x.page.evaluate(() => document.querySelector('#lexicon').value)) && await vis(x.page, '#lexiconUnsaved') === 'Alterações não salvas: Léxico'
      && /sem salvar/.test(await vis(x.page, '#suggestNote')), await vis(x.page, '#suggestNote'));
    await x.page.evaluate(() => document.querySelector('#lexiconProposal button.primary').click());
    await esperaDialogo(x.page); await confirma(x.page); await x.page.waitForTimeout(300);
    ok('4 2.10 confirmar: o PUT leva o léxico novo e nada além', x.puts.length === 2 && JSON.stringify(diferentes(x.puts[0].settings, x.puts[1].settings)) === '["lexicon"]' && /aplicados e salvos/.test(await vis(x.page, '#suggestNote')), JSON.stringify(diferentes(x.puts[0].settings, x.puts[1].settings)));
    junta(x); await fecha(x);

    // ================================================================ 5. nenhum outro fluxo grava o perfil (9.4.1), com edições não salvas na tela
    x = await pagina(await contexto(browser));
    x.esperadas = ['POST /api/generate', 'POST /api/project/export', 'POST /api/lexicon/suggest'];
    await x.page.evaluate(() => __t.goTo('settings'));
    await edita(x.page, '#hits', '2');
    // Gerar (T22): três escolhas.
    await x.page.evaluate(() => __t.goTo('project'));
    await x.page.click('#generate');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('5 T22 Gerar com edições não salvas: "Gerar com as configurações gravadas", "Salvar e gerar", Cancelar', d.title === 'Gerar com alterações não salvas?' && d.message === 'Há alterações não salvas: hits. Esta ação usa o perfil gravado.' && JSON.stringify(d.choices) === '["Gerar com as configurações gravadas","Salvar e gerar"]' && d.ok === null, JSON.stringify(d));
    await cancela(x.page);
    ok('5 T22 Cancelar: nem gera nem grava', !x.writes.includes('POST /api/generate') && x.puts.length === 0);
    await x.page.click('#generate');
    await esperaDialogo(x.page); await escolhe(x.page, 'Gerar com as configurações gravadas');
    ok('5 9.4.1 "Gerar com as gravadas": pede a geração e não grava o perfil', x.writes.filter(w => w === 'POST /api/generate').length === 1 && x.puts.length === 0);
    await x.page.click('#generate');
    await esperaDialogo(x.page); await escolhe(x.page, 'Salvar e gerar');
    await esperaDialogo(x.page);
    ok('5 T22 "Salvar e gerar" abre Salvar, com a lista', /• hits: 1 → 2/.test((await dialogo(x.page)).message));
    await confirma(x.page);
    ok('5 T22 depois de Salvar: um PUT e a geração', x.puts.length === 1 && x.writes.filter(w => w === 'POST /api/generate').length === 2);
    await edita(x.page, '#hits', '3');
    // Exportar (2.10).
    await x.page.evaluate(() => __t.goTo('settings'));
    await x.page.click('#exportProfile');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('5 2.10 Exportar com edições: "Exportar o gravado", "Salvar e exportar", Cancelar', JSON.stringify(d.choices) === '["Exportar o gravado","Salvar e exportar"]', JSON.stringify(d));
    await escolhe(x.page, 'Exportar o gravado');
    ok('5 9.4.1 "Exportar o gravado" não grava o perfil', x.writes.includes('POST /api/project/export') && x.puts.length === 1);
    // Sugerir léxico, com o léxico editado: avisa que usa o gravado.
    await x.page.evaluate(() => __t.goTo('lexicon'));
    await edita(x.page, '#lexicon', '{ "groups": [["a", "b"]], "stop_words": [] }');
    x.lexiconAnswer = { id: 'f7', query: 'pedido', send: { state: 'completed', modelfile: null }, proposal: null };
    await x.page.click('#suggestLexicon');
    await x.page.waitForTimeout(800);
    ok('5 9.4.1 Sugerir léxico não grava e avisa "A sugestão usa o léxico gravado"', x.puts.length === 1 && x.writes.includes('POST /api/lexicon/suggest') && /A sugestão usa o léxico gravado, não as edições não salvas\./.test(await x.page.evaluate(() => document.querySelector('#suggestNote').textContent)), await x.page.evaluate(() => document.querySelector('#suggestNote').textContent));
    // Perguntar (pergunta livre) e Preparar (Builder): sem PUT.
    await manual(x.page);
    await x.page.fill('#question', 'F7 pergunta livre sem intenção de desempenho');
    await x.page.click('#ask');
    await x.page.waitForTimeout(1500);
    ok('5 9.4.1 Perguntar (/api/ask) não grava', x.creates.some(c => c.path === '/api/ask') && x.puts.length === 1);
    await juntaUma(x.page);
    await x.page.fill('#question', 'F7 Builder');
    await x.page.click('#ask');
    await x.page.waitForTimeout(800);
    ok('5 9.4.1 Preparar (Builder) não grava', x.creates.filter(c => c.path === '/api/gathered-question').length === 1 && x.puts.length === 1);
    ok('5 3.10 o texto do compositor fica depois de preparar', await x.page.inputValue('#question') === 'F7 Builder');
    // Lote (Separar) e caminho, já em Localizar: sem confirmação e sem PUT.
    const antesLote = x.creates.length;
    ok('5 9.4.1 Separar em Localizar: cria sem perguntar nada e sem PUT', await separa(x.page) !== null && (await x.page.waitForTimeout(3000), x.creates.length > antesLote) && x.puts.length === 1 && !await x.page.evaluate(() => document.querySelector('#askDialog').open), `${x.creates.length - antesLote}`);
    // Cadeia em Localizar, escolhendo Localizar no diálogo: sem confirmação e sem PUT.
    const cadeia = x.page.evaluate(() => __t.continueTurn({ id: 1 }, document.createElement('button')));
    await esperaDialogo(x.page);
    await x.page.fill('#askInput', 'F7 cadeia');
    await x.page.evaluate(() => [...document.querySelectorAll('#askExtra .seg button')][0].click());
    await confirma(x.page); await cadeia;
    ok('5 9.4.1 cadeia no modo da política: sem confirmação e sem PUT', x.creates.some(c => c.path === '/api/continue') && x.puts.length === 1);
    ok('5 o perfil em memória só mudou pelo Salvar explícito', JSON.stringify(diferentes(real.settings, x.store[CHAVE_RENDERER])) === '["hits"]');
    junta(x); await fecha(x);

    // ================================================================ 6. T26 em cada ação que exige um modo; Otimizar e o cartão não trocam o modo
    x = await pagina(await contexto(browser));
    // Otimizar (política em Localizar): nada muda no modo nem no perfil; o caminho avisa.
    await otimiza(x.page);
    ok('6 pendência 25: Otimizar não troca o modo nem grava', await seletor(x.page) === 'locate' && x.puts.length === 0);
    s = await x.page.evaluate(() => document.querySelector('#route').textContent);
    ok('6 3.11 o caminho: escada com o portão, sem revisão (C1), e o modo diferente da política', /^Escada: .*haiku.*; portão: .+\. Esta rota envia sem revisão \(depende do C1\)\nEsta pergunta usa o modo Melhorar; a política do projeto está em Localizar$/.test(s), s);
    ok('6 3.11 botão por rota: "Rodar a escada"', /^Rodar a escada \(\d+\) · /.test(await x.page.textContent('#ask')), await x.page.textContent('#ask'));
    ok('6 5.5 passo 8: o Builder diz "a política do projeto está em Localizar"', /Modo necessário: Melhorar \(registrado neste rascunho\); a política do projeto está em Localizar$/.test(await vis(x.page, '#builderEntry')), await vis(x.page, '#builderEntry'));
    const esperado = await x.page.evaluate(() => ({ nodes: __t.gathered.slice(), selection: __t.selectionOnRecord() }));
    await x.page.click('#ask');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('6 T26 Rodar a escada: "Esta ação usa o modo Melhorar; a política do projeto está em Localizar. Alterar a política para Melhorar e continuar?"', d.title === 'Esta ação usa outro modo' && d.message.startsWith('Esta ação usa o modo Melhorar; a política do projeto está em Localizar. Alterar a política para Melhorar e continuar?') && d.ok === 'Alterar e continuar', JSON.stringify(d));
    await cancela(x.page);
    ok('6 T26 Cancelar: nem escada nem PUT', x.creates.length === 0 && x.puts.length === 0 && await seletor(x.page) === 'locate');
    await x.page.click('#ask');
    await esperaDialogo(x.page); await confirma(x.page);
    await x.page.waitForFunction(() => document.querySelector('#ask').disabled === false, null, { timeout: 30000 });
    await x.page.waitForTimeout(500);
    const escada = x.creates.find(c => c.path === '/api/ladder');
    ok('6 T26 confirmar: um PUT só com a política, depois a escada', x.puts.length === 1 && JSON.stringify(diferentes(real.settings, x.puts[0].settings)) === '["allow_hypothesis","allow_patch"]' && Boolean(escada), `${x.puts.length} ${Boolean(escada)}`);
    ok('6 9.5 o corpo da escada: as mesmas chaves, nodes e seleção, retries e lacuna_rounds 0', escada && JSON.stringify(Object.keys(escada.body)) === '["key","run","question","nodes","with_facts","gate","models","retries","lacuna_rounds","origin"]'
      && JSON.stringify(escada.body.nodes) === JSON.stringify(esperado.nodes) && JSON.stringify(escada.body.origin.selection) === JSON.stringify(esperado.selection) && escada.body.retries === 0 && escada.body.lacuna_rounds === 0, escada && Object.keys(escada.body).join());
    ok('6 3.10 o texto fica depois da escada', /Matrix<4>::inverse|inverse/.test(await x.page.inputValue('#question')));
    // Agora em Melhorar: Separar pede T26 para Localizar (9.4.7).
    const criadas = x.creates.length;
    await separa(x.page);
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('6 9.4.7 Separar com a política em Melhorar pede T26', d.message.startsWith('Esta ação usa o modo Localizar; a política do projeto está em Melhorar.'), d.message);
    await cancela(x.page); await x.page.waitForTimeout(500);
    ok('6 9.4.7 Cancelar: nenhuma pergunta criada, nada gravado', x.creates.length === criadas && x.puts.length === 1);
    await separa(x.page);
    await esperaDialogo(x.page); await confirma(x.page); await x.page.waitForTimeout(3000);
    ok('6 9.4.7 confirmar: grava só os dois campos (false/false) e cria as perguntas', x.puts.length === 2 && JSON.stringify(diferentes(x.puts[0].settings, x.puts[1].settings)) === '["allow_hypothesis","allow_patch"]' && !x.puts[1].settings.allow_patch && x.creates.length > criadas, `${x.creates.length - criadas}`);
    // Cadeia escolhendo Melhorar com a política em Localizar.
    const cadeia2 = x.page.evaluate(() => __t.continueTurn({ id: 1 }, document.createElement('button')));
    await esperaDialogo(x.page);
    await x.page.fill('#askInput', 'F7 cadeia em Melhorar');
    await confirma(x.page);
    await esperaDialogo(x.page);
    ok('6 T26 cadeia: o modo escolhido no diálogo (Melhorar) pede a confirmação', (await dialogo(x.page)).message.startsWith('Esta ação usa o modo Melhorar; a política do projeto está em Localizar.'));
    await cancela(x.page); await cadeia2;
    ok('6 T26 cadeia cancelada: nada criado nem gravado', !x.creates.some(c => c.path === '/api/continue') && x.puts.length === 2);
    junta(x); await fecha(x);

    // Achado do inventário com a política em Melhorar; caminho (askPath) com a política em Melhorar.
    {
      const store = await perfis();
      store[CHAVE_RENDERER].allow_hypothesis = store[CHAVE_RENDERER].allow_patch = true;
      x = await pagina(await contexto(browser), { store });
      ok('6 com o perfil em Melhorar, a página abre em Melhorar', await seletor(x.page) === 'improve');
      const inv = await (await fetch(`${B}/api/inventory?key=${CHAVE_RENDERER}&run=${RUN}`)).json();
      const loop = inv.inventory.loops.find(l => l.through_calls?.length) ?? inv.inventory.loops[0];
      await x.page.evaluate(o => __t.investigate(o, document.createElement('button')), loop.occurrence);
      await x.page.waitForFunction(() => __t.investigation, null, { timeout: 60000 });
      ok('6 o achado não troca o modo (registra Localizar como necessário)', await seletor(x.page) === 'improve' && x.puts.length === 0 && await x.page.evaluate(() => __t.draft.required_mode) === 'locate');
      ok('6 3.11 o caminho do achado', /^Achado do inventário: \d+ item\(ns\) marcado\(s\), sem registro de seleção\nPreparar abre a Revisão \(parcial\); o envio é um passo à parte, na Revisão\nEsta pergunta usa o modo Localizar; a política do projeto está em Melhorar$/.test(await x.page.evaluate(() => document.querySelector('#route').textContent)), await x.page.evaluate(() => document.querySelector('#route').textContent));
      await manual(x.page);
      await x.page.click('#ask');
      await esperaDialogo(x.page);
      ok('6 T26 Preparar o achado em Melhorar pede Localizar', (await dialogo(x.page)).message.startsWith('Esta ação usa o modo Localizar; a política do projeto está em Melhorar.'));
      await confirma(x.page); await x.page.waitForTimeout(800);
      const achado = x.creates.find(c => c.path === '/api/gathered-question');
      ok('6 depois de confirmar: um PUT (só a política) e o corpo do achado sem seleção, como antes', x.puts.length === 1 && achado && JSON.stringify(Object.keys(achado.body.origin)) === '["inventory","kind","summary"]', achado && JSON.stringify(Object.keys(achado.body.origin)));
      // askPath, de volta a Melhorar pelo seletor.
      await x.page.click('#modeSeg [data-mode="improve"]'); await esperaDialogo(x.page); await confirma(x.page);
      await x.page.waitForFunction(() => __t.mode === 'improve', null, { timeout: 30000 });
      await x.page.evaluate(n => { const r = __t.hot.rows.find(y => y.name === n); __t.selectNode(__t.state.index.get(r.symbol)); __t.goTo('explore', 'structure'); }, INV);
      await x.page.waitForTimeout(300);
      await x.page.evaluate(() => [...document.querySelectorAll('#inspector button')].find(b => b.textContent === __t.t('inspector.calls')).click());
      await x.page.waitForFunction(() => !document.querySelector('#pathAsk').hidden || document.querySelector('#pathCanvas svg'), null, { timeout: 90000 });
      if (await x.page.evaluate(() => !document.querySelector('#pathAsk').hidden)) {
        await x.page.click('#pathAsk'); await esperaDialogo(x.page);
        await x.page.evaluate(() => { const p = document.querySelector('#askExtra select'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
        await confirma(x.page);
        await esperaDialogo(x.page);
        ok('6 T26 perguntas por caminho em Melhorar pedem Localizar', (await dialogo(x.page)).message.startsWith('Esta ação usa o modo Localizar; a política do projeto está em Melhorar.'));
        await cancela(x.page); await x.page.waitForTimeout(400);
        ok('6 T26 caminho cancelado: nada criado nem gravado', !x.creates.some(c => c.path === '/api/path-questions') && x.puts.length === 2);
      } else ok('6 (sem pontos no caminho)', true);
      junta(x); await fecha(x);
    }

    // "Pedir esta mudança" (pendência 26): o clique não grava; a escada pede T26 uma vez e grava só a política.
    x = await pagina(await contexto(browser));
    const card = await cartaoComMudanca(x.page);
    if (card) {
      await x.page.evaluate(() => [...document.querySelectorAll('.perf-card button')].find(b => b.textContent === __t.t('perf.toLadder')).click());
      await x.page.waitForTimeout(300);
      if (await x.page.evaluate(() => document.querySelector('#askDialog').open)) await confirma(x.page);
      await x.page.waitForFunction(() => __t.gathered.length && __t.draft.entry?.kind === 'change' && !document.querySelector('#centerBuilderState').textContent, null, { timeout: 60000 });
      ok('6 cartão "Pedir esta mudança": monta o rascunho sem trocar o modo e sem gravar', await seletor(x.page) === 'locate' && x.puts.length === 0 && await x.page.evaluate(() => __t.draft.required_mode) === 'improve');
      await x.page.click('#ask');
      await esperaDialogo(x.page); await confirma(x.page);
      await x.page.waitForFunction(() => document.querySelector('#ask').disabled === false, null, { timeout: 30000 });
      await x.page.waitForTimeout(500);
      ok('6 pendência 26: do cartão à escada, um único PUT (só a política, confirmado), não dois implícitos', x.puts.length === 1 && JSON.stringify(diferentes(real.settings, x.puts[0].settings)) === '["allow_hypothesis","allow_patch"]' && x.creates.some(c => c.path === '/api/ladder'));
    } else ok('6 (nenhum cartão com mudança nesta run)', true);
    junta(x); await fecha(x);

    // ================================================================ 7. critérios do rascunho (3.12, 5.4, 5.5 passo 7, 9.3)
    let ctx = await contexto(browser);
    x = await pagina(ctx);
    x.esperadas = ['POST /api/generate'];
    await otimiza(x.page);
    ok('7 em Melhorar necessário, os critérios aparecem com o padrão do projeto', await x.page.evaluate(() => !document.querySelector('#criteriaBox').hidden && document.querySelector('#criteria').value === __t.studio.project.settings.criteria));
    await edita(x.page, '#criteria', 'Critério F7: só esta pergunta');
    s = await x.page.evaluate(() => document.querySelector('#criteriaCount').textContent);
    // Fechamento do F9: "não salvos" diz "como padrão" e onde ficam (no rascunho), para não parecer uma edição do perfil.
    ok('7 3.12 editar: "Diferem do padrão do projeto (não salvos como padrão; ficam no rascunho)"', /· Diferem do padrão do projeto \(não salvos como padrão; ficam no rascunho\)$/.test(s), s);
    ok('7 3.11 o caminho avisa o D10', /Os critérios editados ainda não podem valer só para esta pergunta \(depende do D10\)$/.test(await x.page.evaluate(() => document.querySelector('#route').textContent)));
    ok('7 9.3.1 editar não grava o perfil', x.puts.length === 0 && x.store[CHAVE_RENDERER].criteria === real.settings.criteria);
    let g = JSON.parse(await x.page.evaluate(n => localStorage.getItem(n), NOME(RUN)));
    ok('7 5.4 o rascunho guarda os critérios e o padrão de quando foram escritos', g.body.composer.criteria?.text === 'Critério F7: só esta pergunta' && g.body.composer.criteria?.base === real.settings.criteria);
    ok('7 5.3 o rascunho nunca guarda a política', !/allow_|"mode"/.test(JSON.stringify(g)));
    await x.page.click('#ask');
    await x.page.waitForTimeout(300);
    ok('7 9.3.2 Preparar bloqueado, com a mensagem e as três opções', await vis(x.page, '#criteriaBlockText') === 'Os critérios editados ainda não podem valer só para esta pergunta (depende do D10). Escolha:'
      && JSON.stringify(await x.page.evaluate(() => [...document.querySelectorAll('#criteriaBlock button')].map(b => b.textContent))) === '["Salvar como padrão do projeto","Desfazer a edição","Cancelar"]');
    ok('7 bloqueado: nem T26, nem escada, nem PUT', !await x.page.evaluate(() => document.querySelector('#askDialog').open) && x.creates.length === 0 && x.puts.length === 0);
    ok('7 o foco vai para a primeira opção', await x.page.evaluate(() => document.activeElement === document.querySelector('#criteriaSave')));
    await x.page.click('#criteriaCancel');
    ok('7 Cancelar fecha o bloqueio e não muda nada', await x.page.evaluate(() => document.querySelector('#criteriaBlock').hidden && __t.draftCriteria === 'Critério F7: só esta pergunta'));
    // Gerar e alterar a política com critérios editados: nenhum leva os critérios (9.3.1).
    await x.page.evaluate(() => __t.goTo('project'));
    await x.page.click('#generate'); await x.page.waitForTimeout(400);
    await x.page.click('#modeSeg [data-mode="improve"]'); await esperaDialogo(x.page); await confirma(x.page);
    await x.page.waitForFunction(() => __t.mode === 'improve', null, { timeout: 30000 });
    ok('7 9.3.1 Gerar e Alterar a política não levam os critérios editados', x.puts.length === 1 && x.puts[0].settings.criteria === real.settings.criteria && x.store[CHAVE_RENDERER].criteria === real.settings.criteria);
    ok('7 os critérios do rascunho continuam editados', await x.page.evaluate(() => __t.draftCriteria === 'Critério F7: só esta pergunta'));
    // Recarregar: restaura os critérios do rascunho, sem gravar (9.2.9).
    await recarrega(x.page);
    ok('7 5.5 recarregar restaura os critérios do rascunho, ainda diferentes do padrão', await x.page.evaluate(() => document.querySelector('#criteria').value === 'Critério F7: só esta pergunta' && __t.draftCriteria === 'Critério F7: só esta pergunta')
      && /Diferem do padrão/.test(await x.page.evaluate(() => document.querySelector('#criteriaCount').textContent)));
    ok('7 9.2.9 restaurar não grava o perfil', x.puts.length === 1);
    // Outro run: os critérios não passam; de volta, voltam.
    await trocaRun(x.page, OUTRA);
    ok('7 5.6 noutra run, o campo mostra o padrão (o rascunho não passa)', await x.page.evaluate(() => __t.draftCriteria === null && document.querySelector('#criteria').value === __t.studio.project.settings.criteria));
    await trocaRun(x.page, RUN);
    ok('7 5.6 de volta à run, os critérios do rascunho voltam', await x.page.evaluate(() => __t.draftCriteria === 'Critério F7: só esta pergunta'));
    // Outro projeto (B): o padrão de B, nunca o rascunho de A.
    await abrePor(x.page, RAIZ_B, CHAVE_B);
    ok('7 outro projeto: o campo mostra o padrão de B', await x.page.evaluate(() => __t.draftCriteria === null && document.querySelector('#criteria').value === 'Critério de B'));
    await abrePor(x.page, RENDERER, CHAVE_RENDERER);
    await x.page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
    if (await x.page.evaluate(() => __t.studio.run) !== RUN) await trocaRun(x.page, RUN);
    // 5.5 passo 7: o padrão mudou (fora desta página) desde o rascunho.
    x.store[CHAVE_RENDERER].criteria = 'Padrão novo, gravado depois do rascunho';
    await recarrega(x.page);
    s = await vis(x.page, '#criteriaNote');
    ok('7 5.5 passo 7: "O padrão do projeto mudou desde este rascunho", mostrando os dois', s.startsWith('O padrão do projeto mudou desde este rascunho. Padrão quando o rascunho foi escrito: “# Criterios previos') && /Padrão atual: “Padrão novo, gravado depois do rascunho”\.$/.test(s), s);
    ok('7 passo 7: os critérios do rascunho continuam os do rascunho, e nada é gravado', await x.page.evaluate(() => __t.draftCriteria === 'Critério F7: só esta pergunta') && x.puts.length === 1);
    // Desfazer.
    await x.page.click('#ask'); await x.page.waitForTimeout(200);
    await x.page.click('#criteriaUndo');
    ok('7 Desfazer: o campo volta ao padrão atual, sem aviso de D10 e sem gravar', await x.page.evaluate(() => __t.draftCriteria === null && document.querySelector('#criteria').value === 'Padrão novo, gravado depois do rascunho'
      && !/D10/.test(document.querySelector('#route').textContent) && document.querySelector('#criteriaBlock').hidden) && x.puts.length === 1);
    g = JSON.parse(await x.page.evaluate(n => localStorage.getItem(n), NOME(RUN)));
    ok('7 Desfazer: o rascunho guardado deixa de ter critérios', g.body.composer.criteria === null);
    // Salvar como padrão do projeto, com outra edição não salva em Configurações (que fica não salva).
    await x.page.evaluate(() => __t.goTo('settings'));
    await edita(x.page, '#hits', '2');
    await edita(x.page, '#criteria', 'Critério F7 que vira padrão');
    await x.page.click('#ask'); await x.page.waitForTimeout(200);
    await x.page.click('#criteriaSave');
    await esperaDialogo(x.page);
    d = await dialogo(x.page);
    ok('7 9.3.3 "Salvar como padrão" mostra o que muda (só os critérios)', d.message === 'Salvar estas alterações no perfil do projeto? \n• Critérios padrão: Padrão novo, gravado depois do rascunho → Critério F7 que vira padrão', d.message);
    await confirma(x.page); await x.page.waitForTimeout(300);
    ok('7 9.3.3 o PUT muda só criteria; hits continua não salvo', x.puts.length === 2 && JSON.stringify(diferentes(x.puts[0].settings, x.puts[1].settings)) === '["criteria"]' && x.puts[1].settings.criteria === 'Critério F7 que vira padrão'
      && await x.page.evaluate(() => document.querySelector('#hits').value === '2' && /hits/.test(document.querySelector('#settingsUnsaved').textContent)), JSON.stringify(diferentes(x.puts[0].settings, x.puts[1].settings)));
    ok('7 depois de salvar: os critérios deixam de diferir, "Configurações salvas: Critérios padrão"', await x.page.evaluate(() => __t.draftCriteria === null && document.querySelector('#criteriaBlock').hidden) && await vis(x.page, '#criteriaSaid') === 'Configurações salvas: Critérios padrão', await vis(x.page, '#criteriaSaid'));
    junta(x); await fecha(x);

    // Um rascunho guardado antes do F7 (sem critérios) continua válido e segue o padrão.
    ctx = await contexto(browser);
    x = await pagina(ctx);
    await otimiza(x.page);
    await x.page.evaluate(n => { const d = JSON.parse(localStorage.getItem(n)); delete d.body.composer.criteria; localStorage.setItem(n, JSON.stringify(d)); }, NOME(RUN));
    await recarrega(x.page);
    ok('7 rascunho do F6 (sem o campo de critérios): restaurado, seguindo o padrão', await x.page.evaluate(() => __t.gathered.length > 0 && __t.draftCriteria === null && document.querySelector('#criteria').value === __t.studio.project.settings.criteria));
    junta(x); await fecha(x);

    // ================================================================ 8. o caminho previsto (3.11) e o botão por rota
    x = await pagina(await contexto(browser));
    s = await x.page.evaluate(() => document.querySelector('#route').textContent);
    ok('8 3.11 sem fontes: "Primeiro as regras de desempenho…"', s === 'Primeiro as regras de desempenho. Se não reconhecerem, pergunta livre com busca', s);
    await juntaUma(x.page);
    ok('8 3.11 com fontes: "Builder: 1 fonte(s)" e o botão habilitado', await x.page.evaluate(() => document.querySelector('#route').textContent === 'Builder: 1 fonte(s)\nPreparar abre a Revisão (parcial); o envio é um passo à parte, na Revisão' &&!document.querySelector('#ask').disabled));
    junta(x); await fecha(x);
    // Sem comando aprovado: a escada fica desabilitada, com o motivo.
    x = await pagina(await contexto(browser));
    const CMDS = u => new URL(u).pathname === '/api/commands';
    await x.page.route(CMDS, async r => { const y = await r.fetch(); const j = await y.json(); j.commands = j.commands.map(c => ({ ...c, approved: null })); r.fulfill({ json: j }); });
    await recarrega(x.page);
    await otimiza(x.page);
    ok('8 3.11 escada sem comando aprovado: "Sem comando aprovado no portão (→ Régua)" e o botão desabilitado', await x.page.evaluate(() => document.querySelector('#route').textContent.startsWith('Sem comando aprovado no portão (→ Régua)') && document.querySelector('#ask').disabled));
    await x.page.unroute(CMDS);
    // Sem alvo: desabilitado, com "Escolha um alvo no Builder".
    await x.page.evaluate(() => { for (const s of [...__t.seeds]) __t.toggleGathered(s); });
    await x.page.waitForTimeout(200);
    ok('8 3.11 fontes sem alvo: "Escolha um alvo no Builder" e o botão desabilitado', await x.page.evaluate(() => document.querySelector('#route').textContent.startsWith('Escolha um alvo no Builder') && document.querySelector('#ask').disabled), await x.page.evaluate(() => document.querySelector('#route').textContent));
    junta(x); await fecha(x);

    // ================================================================ 9. tela estreita, EN, temas, teclado, chaves
    for (const [scheme, locale] of [['dark', 'en-US'], ['light', 'pt-BR']]) {
      ctx = await contexto(browser, { size: { width: 390, height: 844 }, scheme });
      x = await pagina(ctx);
      if (locale === 'en-US') { await x.page.evaluate(() => document.querySelector('[data-lang="en"]').click()); await x.page.waitForTimeout(300); }
      await x.page.evaluate(() => __t.goTo('settings'));
      await edita(x.page, '#hits', '2');
      s = await vis(x.page, '#headUnsaved');
      ok(`9 ${scheme}/${locale} 390 px: o indicador de não salvo no cabeçalho`, locale === 'en-US' ? s === 'Unsaved changes: Settings' : s === 'Alterações não salvas: Configurações', s);
      await x.page.click('#colSwitch [data-col="question"]');
      await x.page.click('#modeSeg [data-mode="improve"]');
      await esperaDialogo(x.page);
      d = await dialogo(x.page);
      ok(`9 ${scheme}/${locale} a confirmação da política no idioma`, locale === 'en-US' ? d.title === 'Change the mode policy' && /Improve: hypotheses and changes allowed\. Writes only hypothesis and patch/.test(d.message) : d.title === 'Alterar a política do modo', JSON.stringify(d));
      await confirma(x.page);
      await x.page.waitForFunction(() => __t.mode === 'improve', null, { timeout: 30000 });
      s = await vis(x.page, '#modeNote');
      ok(`9 ${scheme}/${locale} a mensagem da política alterada`, locale === 'en-US' ? s === 'Project policy changed: Improve (hypotheses and changes allowed)' : s === 'Política do projeto alterada: Melhorar (hipóteses e mudanças permitidas)', s);
      ok(`9 ${scheme}/${locale} sem rolagem horizontal`, await x.page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1));
      const cores = await x.page.evaluate(() => { const st = e => getComputedStyle(document.querySelector(e)); return [[st('#modeNote').color, st('#composer').backgroundColor], [st('#route').color, st('#composer').backgroundColor], [st('#headUnsaved').color, st('#head').backgroundColor]]; });
      for (const [i, c] of cores.entries()) ok(`9 ${scheme}/${locale} contraste ≥ 4,5 (${['nota do modo', 'caminho previsto', 'não salvo'][i]})`, contraste(c) >= 4.5, `${c.join(' / ')} = ${contraste(c).toFixed(2)}`);
      // O botão do cabeçalho leva à ferramenta.
      await x.page.click('#headUnsaved');
      ok(`9 ${scheme}/${locale} o indicador leva a Configurações`, await x.page.evaluate(() => document.querySelector('#areas [data-area="settings"]').getAttribute('aria-current') === 'page'));
      junta(x); await fecha(x);
    }
    const src = fs.readFileSync(path.join(__dirname, '../../web/index.html'), 'utf8');
    const chaves = [...src.matchAll(/^\s*'((?:mode|field|profile|save|unsaved|generate|export|criteria|route)\.[A-Za-z.]+)': \['((?:[^'\\]|\\.)*)',\s*'((?:[^'\\]|\\.)*)'\]/gm)];
    ok('9 9.12.2 as chaves do F7 têm PT e EN, com as mesmas variáveis', chaves.length > 60 && chaves.every(m => m[2] && m[3] && JSON.stringify((m[2].match(/\{\w+\}/g) ?? []).sort()) === JSON.stringify((m[3].match(/\{\w+\}/g) ?? []).sort())), chaves.length);
    ok('9 8.4.3 a página não chama mais saveSettings, collectSettings nem setMode', !/\b(saveSettings|collectSettings|setMode)\(/.test(src));
    ok('9 3.13.4 a página não grava librarian.mode', !/saved\.set\('mode'/.test(src));
    ok('9 PUT /api/project só nas duas operações (saveProfile e changePolicy)', (src.match(/api\('PUT', '\/api\/project'/g) ?? []).length === 2);
  } catch (e) { console.error(e); ok('a bateria terminou sem exceção', false, e.message); }
  finally {
    ok('nenhum erro na página', todosErros.length === 0, todosErros.join(' | '));
    ok('nenhuma escrita inesperada (as outras são abortadas)', todasEscritas.length === 0, todasEscritas.join(','));
    ok('o perfil (project.json) da raiz fica igual em bytes', sha(perfil) === perfilAntes);
    await browser.close();
  }
})();

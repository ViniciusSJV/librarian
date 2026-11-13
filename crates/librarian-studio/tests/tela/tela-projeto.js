// F9: Projeto, Régua e Acervo (especificação 2.1, 2.5, 2.6, 5.3, T9, T13, T17, T25; critério 9.10), na página real,
// num Chrome sem janela, contra os servidores de rodar.js (8841 Rust, 8842 Python).
// Casos: o resumo de estado do projeto conferido contra /api/commands, /api/hotspots, /api/facts, /api/turns e o
// perfil, cada item com o próprio estado (carregando, falha de leitura, sucesso) e uma falha que não afeta os outros,
// e o destino de cada item com o ponto no Voltar; a run guardada por projeto (restaurar ao recarregar e ao abrir de
// novo, a pergunta ativa, os rascunhos separados por run e por projeto, run guardada que não existe mais, armazenamento
// bloqueado, resposta atrasada da run restaurada, nenhuma gravação ao abrir); T25 (sem edições, cancelar, abrir sem
// salvar, salvar e abrir, Salvar cancelado, Salvar recusado, pelos caminhos Abrir, Enter, Procurar e Limpar); o aviso do
// navegador ao sair e ao recarregar (T17) com e sem edições, com rascunho guardado, com armazenamento bloqueado, num
// conflito entre abas e depois de um Salvar recusado; a Régua (versão, aprovação nesta versão, linha de base em vigor,
// Rodar desabilitado antes do clique numa run com alterações, "Abrir a pergunta #N", "→ Onde pesa"); o Acervo (filtro
// por estado, Ir à origem na mesma run e em outra com T13, Revogar… no controle de decisão, Juntar a declaração, os
// estados); Rust com medição e Python sem instrument (9.10); tela estreita, PT e EN, temas, contraste e teclado.
// Nada é enviado a modelo e nada é gravado no servidor: as escritas que a bateria precisa ver são respondidas aqui;
// as outras são abortadas e contadas.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium, RENDERER, PYTHON, CHAVE_RENDERER, CHAVE_PYTHON } = require('./ambiente');
const B = 'http://127.0.0.1:8841', P = 'http://127.0.0.1:8842', RUN = '20261104-020244', OUTRA = '20261104-014710';
const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS;
const HOOK = "window.__t = { state, studio, t, STRINGS, loadRun, activate, renderTurns, setLang, localState, navigation, goTo, goBack, toggleGathered, unsavedFields,"
  + " factDeclaration, investigate, get investigation() { return investigation; }, get draftCriteria() { return draftCriteria; },"
  + " get gathered() { return gathered; }, get draft() { return draft; }, get hotStatus() { return hotStatus; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 300) : ''}`); if (!pass) process.exitCode = 1; };
const perfil = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'project.json');
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const NAV = k => `librarian.nav.v1:${k}`, DRAFT = (k, r) => `librarian.draft.v1:${k}:${r}`;
const get = async (base, p) => (await fetch(base + p)).json();
const segura = () => { let solta; const p = new Promise(r => { solta = r; }); return { p, solta }; };
const LEITURA = (method, pathname) => method === 'GET'
  || (method === 'POST' && ['/api/project', '/api/calls', '/api/envelope/suggest', '/api/inventory/investigate', '/api/performance-question'].includes(pathname));
const contraste = cores => {
  const L = k => { const m = k.match(/[\d.]+/g).slice(0, 3).map(Number).map(v => v / 255).map(v => v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4); return 0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]; };
  const [hi, lo] = [L(cores[0]), L(cores[1])].sort((p, q) => q - p);
  return (hi + 0.05) / (lo + 0.05);
};
// O projeto fictício B (T25 e o isolamento por projeto): responde com o renderer, outra chave e outra raiz.
const RAIZ_B = 'C:/projeto-ficticio-da-bateria/f9b', CHAVE_B = 'ficticio-f9b-00000000';
// O armazenamento bloqueado: toda leitura e escrita do localStorage lança, como num navegador que o recusa.
const BLOQUEIA = () => { const fail = () => { throw new DOMException('bloqueado pela bateria', 'SecurityError'); };
  Object.defineProperty(window, 'localStorage', { configurable: true, get: () => ({ getItem: fail, setItem: fail, removeItem: fail, key: fail, clear: fail, get length() { return fail(); } }) }); };

async function pagina(browser, { base = B, project = RENDERER, key = CHAVE_RENDERER, size = { width: 1440, height: 900 }, scheme = 'light', locale = 'pt-BR',
  context = null, storage = null, bloqueado = false, antes = null, b = null, espera = true } = {}) {
  const ctx = context ?? await browser.newContext({ viewport: size, colorScheme: scheme, locale });
  if (!context && storage) await ctx.addInitScript(s => { if (!sessionStorage.getItem('f9.semeado')) { for (const [k, v] of Object.entries(s)) localStorage.setItem(k, v); sessionStorage.setItem('f9.semeado', '1'); } }, storage);
  if (!context && bloqueado) await ctx.addInitScript(BLOQUEIA);
  const page = await ctx.newPage();
  const x = { page, context: ctx, errors: [], writes: [], reqs: [], fake: {}, base };
  page.on('pageerror', e => x.errors.push(e.message));
  await page.route(u => new URL(u).pathname === '/', async r => { const y = await r.fetch(); r.fulfill({ response: y, body: (await y.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.route(u => new URL(u).pathname.startsWith('/api/'), async r => {
    const req = r.request(), url = new URL(req.url()), { pathname } = url, method = req.method(), chave = `${method} ${pathname}`;
    const raw = req.postData(), body = raw ? JSON.parse(raw) : null;
    x.reqs.push({ chave, url: req.url(), raw, body, run: url.searchParams.get('run'), key: url.searchParams.get('key') ?? body?.key ?? null });
    const f = x.fake[chave];
    if (f) { const feito = await f(r, body, url); if (feito !== false) return; }
    // B: as leituras dele são as do renderer; abrir B responde o renderer com a chave e a raiz de B.
    if (method === 'GET' && url.searchParams.get('key') === CHAVE_B) { url.searchParams.set('key', CHAVE_RENDERER); return r.continue({ url: url.toString() }); }
    if (chave === 'POST /api/project' && body?.root === RAIZ_B) {
      const res = await r.fetch({ postData: JSON.stringify({ root: RENDERER }) }), data = await res.json();
      return r.fulfill({ json: { ...data, key: CHAVE_B, settings: { ...data.settings, name: 'ficticio-f9b', root: RAIZ_B }, runs: b?.runs ? data.runs.filter(z => b.runs.includes(z.id)) : [] } });
    }
    if (LEITURA(method, pathname)) return r.continue();
    x.writes.push(chave);
    return r.abort();
  });
  if (antes) await antes(x);
  await page.goto(`${base}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  if (project && await page.evaluate(() => __t.studio.project?.key) !== key) await abre(page, project, key);
  if (espera && project) await pronta(page);
  return x;
}
const pronta = page => page.waitForFunction(() => !__t.studio.run || (__t.studio.turnsState !== 'loading' && (__t.studio.facts || __t.studio.factsError) && __t.studio.commandsState !== 'loading'),
  null, { timeout: 120000 }).then(() => page.waitForTimeout(400));
async function abre(page, root, key, { pelo = 'botao' } = {}) {
  await page.fill('#projectPath', root);
  if (pelo === 'enter') await page.press('#projectPath', 'Enter'); else await page.click('#openProject');
  await page.waitForFunction(k => __t.studio.project?.key === k && !document.querySelector('#openProject').disabled, key, { timeout: 120000 });
  await page.waitForTimeout(600);
}
async function trocaRun(page, run) {
  await page.selectOption('#runs', run);
  await page.waitForFunction(r => __t.studio.run === r && __t.state.graph && __t.studio.turnsState !== 'loading', run, { timeout: 120000 });
  await pronta(page);
}
async function recarrega(page) {
  await page.reload();
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  await page.waitForFunction(() => __t.studio.project && (__t.studio.run ? __t.state.graph : true), null, { timeout: 120000 });
  await pronta(page);
}
const fecha = x => x.context.close();
const dialogo = page => page.waitForFunction(() => document.querySelector('#askDialog').open, null, { timeout: 30000 });
const aberto = page => page.evaluate(() => document.querySelector('#askDialog').open);
const titulo = page => page.evaluate(() => document.querySelector('#askTitle').textContent);
const mensagem = page => page.evaluate(() => document.querySelector('#askMessage').textContent);
const escolhe = async (page, valor) => { await page.click(`#askChoices button[value="choice:${valor}"]`); await page.waitForTimeout(300); };
const cancela = async page => { await page.click('#askDialog button[data-close][data-i18n="common.cancel"]'); await page.waitForTimeout(300); };
const confirma = async page => { await page.click('#askOk'); await page.waitForTimeout(300); };
const T = (page, k, v = {}) => page.evaluate(([k, v]) => __t.t(k, v), [k, v]);
const resumo = page => page.evaluate(() => Object.fromEntries([...document.querySelectorAll('#projectSummary li')].map(li => [li.dataset.summary,
  { text: li.querySelector('.summary-value').textContent, link: li.querySelector('button.summary-value') != null, error: li.querySelector('.summary-value').classList.contains('error') }])));
const navGuardada = (page, k = CHAVE_RENDERER) => page.evaluate(k => { try { return localStorage.getItem(k); } catch { return 'bloqueado'; } }, NAV(k));
const nota = (page, sel) => page.evaluate(s => document.querySelector(s).textContent, sel);
const vai = async (page, area) => { await page.click(`#areas [data-area="${area}"]`); await page.waitForTimeout(150); };
// Tenta fechar a página com beforeunload: devolve se o navegador perguntou antes de sair.
async function perguntaAoSair(page) {
  let perguntou = false;
  page.on('dialog', d => { if (d.type() === 'beforeunload') perguntou = true; d.accept().catch(() => {}); });
  await page.close({ runBeforeUnload: true });
  await new Promise(r => setTimeout(r, 800));
  return perguntou;
}
// O que a página decide no beforeunload (recarregar, navegar para fora ou fechar disparam o mesmo evento).
const pedeAviso = page => page.evaluate(() => { const e = new Event('beforeunload', { cancelable: true }); window.dispatchEvent(e); return e.defaultPrevented; });
// Uma edição de verdade (com ativação do usuário), no campo Entradas da área Projeto.
async function edita(page, extra = 'f9-edicao') {
  await vai(page, 'project');
  const antes = await page.inputValue('#inputs');
  await page.click('#inputs'); await page.press('#inputs', 'End'); await page.keyboard.type(`, ${extra}`);
  await page.waitForTimeout(200);
  return antes;
}
// O perfil gravado do renderer, como a página o lê, para responder um PUT sem gravar.
let PROJETO = null;
const respondePut = x => { x.fake['PUT /api/project'] = async (r, body) => { x.puts = (x.puts ?? 0) + 1; x.putBody = body; return r.fulfill({ json: { ...PROJETO, settings: body.settings } }); }; };

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const perfilAntes = sha(fs.readFileSync(perfil));
  PROJETO = await get(B, `/api/project?key=${CHAVE_RENDERER}`);
  const abertoRenderer = await (await fetch(`${B}/api/project`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ root: RENDERER }) })).json();
  const RECENTE = abertoRenderer.runs[0].id;
  const commands = await get(B, `/api/commands?key=${CHAVE_RENDERER}&run=${RUN}`), facts = (await get(B, `/api/facts?key=${CHAVE_RENDERER}&run=${RUN}`)).facts;
  const turns = await get(B, `/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`), execs = await get(B, `/api/executions?key=${CHAVE_RENDERER}&run=${RUN}`);
  const hotRes = await fetch(`${B}/api/hotspots?key=${CHAVE_RENDERER}&run=${RUN}`), hot = hotRes.ok ? await hotRes.json() : null;
  const grafo = await get(B, `/api/graph?key=${CHAVE_RENDERER}&run=${RUN}`);
  const num = id => (/^\d+$/.test(id) ? Number(id) : Infinity);
  const ordem = [...turns].sort((a, b) => num(a.id) - num(b.id));
  let x;

  try {
    // ================================================================ 1. resumo de estado (2.1)
    x = await pagina(browser);
    await trocaRun(x.page, RUN);
    await vai(x.page, 'project');
    let r = await resumo(x.page);
    ok('1 resumo: os oito itens, na ordem da especificação', JSON.stringify(Object.keys(r)) === JSON.stringify(['measure', 'commands', 'manual', 'lexicon', 'policy', 'facts', 'questions', 'run']), Object.keys(r).join());
    const aprovados = commands.commands.filter(c => c.approved).length;
    ok('1 Comandos: "N de M aprovados na versão atual", do /api/commands', r.commands.text === await T(x.page, 'summary.commands', { n: aprovados, m: commands.commands.length }) && r.commands.link, r.commands.text);
    const manual = commands.manual_counts;
    ok('1 Contagens manuais: do manual_counts', r.manual.text === (manual.length ? await T(x.page, 'summary.manual.some', { n: manual.length, lines: manual.reduce((a, c) => a + c.lines, 0) }) : await T(x.page, 'summary.manual.none')), r.manual.text);
    ok('1 Medição: disponível, com a data e o comando da medição (/api/hotspots)', hot && r.measure.text === await T(x.page, 'summary.measure.ready', { when: hot.started, command: hot.command }), r.measure.text);
    const cont = st => facts.filter(f => f.status === st).length;
    ok('1 Fatos: vigentes, desatualizados, revogados, aplicados, do /api/facts', r.facts.text.startsWith(await T(x.page, 'summary.facts', { current: cont('current'), stale: cont('stale'), revoked: cont('revoked'), applied: cont('applied') })), r.facts.text);
    const pendentes = ordem.filter(t => (t.sends ?? []).some(s => (s.check?.claims ?? []).some(k => !s.reviews?.[k.id])));
    const preparadas = turns.filter(t => !t.prepare_error && !(t.sends ?? []).length).length;
    ok('1 Perguntas: preparadas e não enviadas, com decisões pendentes e a primeira, do /api/turns', r.questions.text === await T(x.page, 'summary.questions', { prepared: preparadas, pending: pendentes.length })
      + (pendentes.length ? await T(x.page, 'summary.questionsFirst', { id: pendentes[0].id }) : ''), r.questions.text);
    const lex = PROJETO.settings.lexicon;
    ok('1 Léxico: grupos e stop words do perfil gravado', r.lexicon.text === (lex.groups?.length || lex.stop_words?.length ? await T(x.page, 'summary.lexicon.some', { groups: lex.groups?.length ?? 0, stops: lex.stop_words?.length ?? 0 }) : await T(x.page, 'summary.lexicon.empty')), r.lexicon.text);
    const modo = PROJETO.settings.allow_hypothesis && PROJETO.settings.allow_patch ? 'improve' : !PROJETO.settings.allow_hypothesis && !PROJETO.settings.allow_patch ? 'locate' : null;
    ok('1 Política: o modo do projeto, da política gravada', modo && r.policy.text === await T(x.page, 'summary.policy', { mode: await T(x.page, `mode.${modo}`) }), r.policy.text);
    const run = abertoRenderer.runs.find(z => z.id === RUN);
    ok('1 Run: id, fontes, símbolos, diagnósticos e git', r.run.text.startsWith(`${RUN} · ${run.sources} fontes · ${run.symbols} símbolos`) && r.run.text.includes(run.git?.head?.slice(0, 7) ?? 'sem git'), r.run.text);
    // Destinos: cada item leva ao lugar dele e o ponto de partida vai para o Voltar.
    await x.page.click('#projectSummary [data-summary="commands"] button');
    ok('1 Comandos leva à Régua, com o ponto de partida no Voltar', await x.page.evaluate(() => __t.navigation.area === 'ruler' && !document.querySelector('#navBack').disabled));
    await x.page.click('#navBack'); await x.page.waitForTimeout(200);
    ok('1 Voltar devolve ao Projeto', await x.page.evaluate(() => __t.navigation.area) === 'project');
    await x.page.click('#projectSummary [data-summary="measure"] button');
    ok('1 Medição leva a Explorar › Onde pesa', await x.page.evaluate(() => __t.navigation.area === 'explore' && __t.navigation.explore === 'hot'));
    await vai(x.page, 'project');
    await x.page.click('#projectSummary [data-summary="facts"] button');
    ok('1 Fatos leva ao Acervo', await x.page.evaluate(() => __t.navigation.area) === 'facts');
    await vai(x.page, 'project');
    await x.page.click('#projectSummary [data-summary="lexicon"] button');
    ok('1 Léxico leva à ferramenta Léxico', await x.page.evaluate(() => __t.navigation.area) === 'lexicon');
    await vai(x.page, 'project');
    await x.page.click('#projectSummary [data-summary="policy"] button');
    ok('1 Política leva ao seletor do modo, no compositor', await x.page.evaluate(() => document.activeElement?.matches('#modeSeg [data-mode]') === true));
    if (pendentes.length) {
      const navAntes = await navGuardada(x.page);
      await x.page.click('#projectSummary [data-summary="questions"] button'); await x.page.waitForTimeout(300);
      const depois = JSON.parse(await navGuardada(x.page) ?? '{}');
      ok('1 Perguntas leva à primeira com decisões pendentes, ativada e guardada como escolha da pessoa', await x.page.evaluate(() => __t.studio.activeTurn) === pendentes[0].id
        && depois.active_turn?.[RUN] === pendentes[0].id, `${navAntes} → ${JSON.stringify(depois)}`);
    }
    ok('1 nenhum PUT, nenhuma escrita', !x.writes.length && !x.reqs.some(q => q.chave === 'PUT /api/project'), x.writes.join());
    await fecha(x);

    // Uma falha num item não afeta os outros; carregando é dito por item.
    x = await pagina(browser, { antes: async y => { y.fake['GET /api/facts'] = r => r.fulfill({ status: 500, json: { error: 'fatos ilegíveis (bateria)' } }); } });
    await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('1 falha nos fatos: o item diz que não foi possível ler, com a mensagem do servidor', r.facts.error && r.facts.text === await T(x.page, 'summary.unread', { error: 'fatos ilegíveis (bateria)' }), r.facts.text);
    ok('1 falha nos fatos: Comandos, Perguntas e Medição continuam com o que leram', /aprovado/.test(r.commands.text) && !r.questions.error && !/Carregando/.test(r.questions.text) && !r.measure.error, JSON.stringify(r));
    await vai(x.page, 'facts');
    ok('1 falha nos fatos: o Acervo diz que não foi possível ler', /Não foi possível ler os fatos: fatos ilegíveis/.test(await nota(x.page, '#factsList')));
    await fecha(x);
    x = await pagina(browser, { antes: async y => { y.fake['GET /api/commands'] = r => r.fulfill({ status: 500, json: { error: 'comandos ilegíveis (bateria)' } }); } });
    await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('1 falha nos comandos: Comandos e Contagens manuais dizem a falha; Fatos e Perguntas não', r.commands.error && r.manual.error && /comandos ilegíveis/.test(r.commands.text) && !r.facts.error && !/Carregando/.test(r.facts.text) && !r.questions.error, JSON.stringify(r));
    await fecha(x);
    const segurados = segura();
    x = await pagina(browser, { espera: false, project: null, storage: { 'librarian.project': RENDERER }, antes: async y => { y.fake['GET /api/commands'] = async r => { await segurados.p; return r.continue(); }; } });
    await x.page.waitForFunction(() => __t.studio.run && __t.studio.facts && __t.studio.turnsState === 'ready', null, { timeout: 120000 });
    await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('1 carregando: com os comandos ainda a caminho, Comandos e Contagens dizem "Carregando…", e Fatos e Perguntas já mostram o que leram', r.commands.text === 'Carregando…' && r.manual.text === 'Carregando…' && !/Carregando/.test(r.facts.text) && !/Carregando/.test(r.questions.text), JSON.stringify(r));
    segurados.solta();
    await pronta(x.page);
    ok('1 depois de chegar: Comandos com os números', /aprovado/.test((await resumo(x.page)).commands.text));
    await fecha(x);
    x = await pagina(browser, { antes: async y => { y.fake['GET /api/turns'] = r => r.fulfill({ status: 500, json: { error: 'perguntas ilegíveis (bateria)' } }); } });
    await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('1 falha nas perguntas: o item diz a falha; Fatos e Comandos não', r.questions.error && /perguntas ilegíveis/.test(r.questions.text) && !r.facts.error && !r.commands.error, JSON.stringify(r));
    await fecha(x);
    // Projeto sem runs (B fictício): "Sem run: gere a primeira" nos itens que dependem da run.
    x = await pagina(browser);
    await abre(x.page, RAIZ_B, CHAVE_B); await pronta(x.page);
    r = await resumo(x.page);
    ok('1 projeto sem runs: Medição, Fatos, Perguntas e Run dizem "Sem run: gere a primeira"', ['measure', 'facts', 'questions', 'run'].every(k => r[k].text === 'Sem run: gere a primeira') && /aprovado/.test(r.commands.text), JSON.stringify(r));
    await fecha(x);

    // ================================================================ 2. run guardada por projeto (2.1, T17, T25)
    x = await pagina(browser, { b: { runs: [RUN, OUTRA, RECENTE] } });
    ok('2 sem run guardada, abre a mais recente', await x.page.evaluate(() => __t.studio.run) === RECENTE);
    ok('2 abrir não grava navegação', await navGuardada(x.page) === null, await navGuardada(x.page));
    await trocaRun(x.page, RUN);
    let nav = JSON.parse(await navGuardada(x.page) ?? '{}');
    ok('2 trocar de run pelo seletor guarda a run do projeto (5.3)', nav.run === RUN && !/mode|allow_/.test(JSON.stringify(nav)), JSON.stringify(nav));
    // A pergunta ativa da run, escolhida pela pessoa, e um rascunho em cada run.
    const ativa = ordem[0].id;
    await x.page.click(`#turns .turn[data-turn="${ativa}"] .bubble.user`); await x.page.waitForTimeout(300);
    const declara = Object.values(grafo.nodes).filter(n => n.category === 'symbol' && n.kind === 'method').slice(0, 2).map(n => n.id);
    await x.page.evaluate(id => __t.toggleGathered(id, 'inspection'), declara[0]); await x.page.waitForTimeout(300);
    const rascunhoRun = await x.page.evaluate(k => localStorage.getItem(k), DRAFT(CHAVE_RENDERER, RUN));
    await trocaRun(x.page, OUTRA);
    const declaraOutra = await x.page.evaluate(() => __t.state.nodes.find(n => n.category === 'symbol' && n.kind === 'function')?.id);
    await x.page.evaluate(id => __t.toggleGathered(id, 'inspection'), declaraOutra); await x.page.waitForTimeout(300);
    await trocaRun(x.page, RUN);
    const navAntes = await navGuardada(x.page);
    const reqsAntes = x.reqs.length;
    await recarrega(x.page);
    ok('2 recarregar volta à última run escolhida (não à mais recente)', await x.page.evaluate(() => __t.studio.run) === RUN);
    ok('2 recarregar restaura a pergunta ativa dela', await x.page.evaluate(() => __t.studio.activeTurn) === ativa);
    ok('2 recarregar restaura o rascunho dela, e só o dela', JSON.stringify(await x.page.evaluate(() => __t.gathered)) === JSON.stringify([declara[0]]));
    ok('2 recarregar não grava a navegação nem o rascunho', await navGuardada(x.page) === navAntes && await x.page.evaluate(k => localStorage.getItem(k), DRAFT(CHAVE_RENDERER, RUN)) === rascunhoRun);
    ok('2 recarregar não grava o perfil nem a política (nenhum PUT)', !x.reqs.slice(reqsAntes).some(q => q.chave === 'PUT /api/project'));
    ok('2 o modo continua o da política gravada', await x.page.evaluate(() => [...document.querySelectorAll('#modeSeg [aria-pressed="true"]')].map(b => b.dataset.mode).join()) === modo);
    await trocaRun(x.page, OUTRA);
    ok('2 a outra run tem o próprio rascunho, sem mistura', JSON.stringify(await x.page.evaluate(() => __t.gathered)) === JSON.stringify([declaraOutra]));
    const turnsOutra = await get(B, `/api/turns?key=${CHAVE_RENDERER}&run=${OUTRA}`);
    ok('2 a conversa é a da outra run: nenhuma pergunta de outra run aparece', await x.page.evaluate(() => __t.studio.turns.length) === turnsOutra.length);
    await recarrega(x.page);
    ok('2 recarregar de novo volta à outra run, com o rascunho dela', await x.page.evaluate(() => __t.studio.run) === OUTRA && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === JSON.stringify([declaraOutra]));
    // Outro projeto: a run guardada de B é de B; abrir A de novo volta à run de A.
    await abre(x.page, RAIZ_B, CHAVE_B); await pronta(x.page);
    ok('2 projeto B sem run guardada: a mais recente dele, não a run guardada de A', await x.page.evaluate(() => __t.studio.run) === RECENTE);
    await trocaRun(x.page, RUN);
    ok('2 a run de B fica na navegação de B', JSON.parse(await navGuardada(x.page, CHAVE_B) ?? '{}').run === RUN && JSON.parse(await navGuardada(x.page) ?? '{}').run === OUTRA);
    ok('2 o rascunho de A não aparece em B (mesmo id de run)', JSON.stringify(await x.page.evaluate(() => __t.gathered)) === '[]');
    await abre(x.page, RENDERER, CHAVE_RENDERER); await pronta(x.page);
    ok('2 abrir A de novo volta à run guardada de A, com o rascunho dela', await x.page.evaluate(() => __t.studio.run) === OUTRA && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === JSON.stringify([declaraOutra]));
    await abre(x.page, RAIZ_B, CHAVE_B); await pronta(x.page);
    ok('2 abrir B de novo volta à run guardada de B', await x.page.evaluate(() => __t.studio.run) === RUN && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === '[]');
    ok('2 nenhum PUT, nenhuma escrita', !x.writes.length && !x.reqs.some(q => q.chave === 'PUT /api/project'), x.writes.join());
    await fecha(x);
    // A run guardada não existe mais: abre a mais recente, diz por quê e não regrava nada.
    x = await pagina(browser, { storage: { [NAV(CHAVE_RENDERER)]: JSON.stringify({ run: '20990101-000000', area: 'project' }) } });
    ok('2 run guardada que não existe mais: abre a mais recente e diz', await x.page.evaluate(() => __t.studio.run) === RECENTE && /não existe mais: aberta a mais recente/.test(await nota(x.page, '#runNote')), await nota(x.page, '#runNote'));
    ok('2 … sem regravar a navegação', JSON.parse(await navGuardada(x.page)).run === '20990101-000000');
    await fecha(x);
    // Armazenamento bloqueado: a página funciona; abrir o mesmo projeto de novo mantém a run da memória.
    x = await pagina(browser, { bloqueado: true });
    await trocaRun(x.page, RUN);
    await abre(x.page, RENDERER, CHAVE_RENDERER); await pronta(x.page);
    ok('2 armazenamento bloqueado: abrir de novo o mesmo projeto fica na run escolhida (memória)', await x.page.evaluate(() => __t.studio.run) === RUN);
    ok('2 armazenamento bloqueado: sem erro de página', !x.errors.length, x.errors.join(' | '));
    await fecha(x);
    // Resposta atrasada da run restaurada: a pessoa troca de run antes de ela chegar; nada dela aparece.
    {
      const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      await ctx.addInitScript(([k, v]) => { if (!sessionStorage.getItem('f9.semeado')) { localStorage.setItem(k, v); localStorage.setItem('librarian.project', 'C:/Users/beatl/Documents/Projetos/renderer-librarian'); sessionStorage.setItem('f9.semeado', '1'); } },
        [NAV(CHAVE_RENDERER), JSON.stringify({ run: RUN, area: 'project' })]);
      const hold = segura();
      x = await pagina(browser, { context: ctx, project: null, antes: async y => { y.fake['GET /api/turns'] = async (r, b, url) => { if (url.searchParams.get('run') !== RUN) return false; await hold.p; return r.continue(); }; } });
      await x.page.waitForFunction(() => __t.studio.run === '20261104-020244' && __t.state.graph, null, { timeout: 120000 });
      await x.page.selectOption('#runs', OUTRA);
      await x.page.waitForFunction(r => __t.studio.run === r && __t.studio.turnsState === 'ready', OUTRA, { timeout: 120000 });
      hold.solta(); await x.page.waitForTimeout(1500);
      ok('2 resposta atrasada da run restaurada não entra na conversa da run escolhida depois', await x.page.evaluate(() => __t.studio.turns.length) === turnsOutra.length && await x.page.evaluate(() => __t.studio.run) === OUTRA);
      ok('2 … e a run guardada passa a ser a escolhida', JSON.parse(await navGuardada(x.page)).run === OUTRA);
      await fecha(x);
    }

    // ================================================================ 3. T25: trocar de projeto com alterações não salvas
    x = await pagina(browser);
    respondePut(x);
    // Sem alterações: troca direto.
    await abre(x.page, RAIZ_B, CHAVE_B);
    ok('3 sem alterações não salvas: troca sem perguntar', !(await aberto(x.page)) && await x.page.evaluate(() => __t.studio.project.key) === CHAVE_B);
    await abre(x.page, RENDERER, CHAVE_RENDERER); await pronta(x.page);
    // Com alterações: pergunta antes de qualquer pedido de troca.
    let original = await edita(x.page);
    let postsAntes = x.reqs.filter(q => q.chave === 'POST /api/project').length;
    await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject');
    await dialogo(x.page);
    ok('3 com alterações: o diálogo diz que há alterações não salvas, em qual projeto e quais', await titulo(x.page) === 'Trocar de projeto com alterações não salvas?'
      && (await mensagem(x.page)).includes('Entradas') && (await mensagem(x.page)).includes(PROJETO.settings.name), await mensagem(x.page));
    ok('3 as escolhas: Abrir sem salvar, Salvar e abrir, Cancelar', await x.page.evaluate(() => [...document.querySelectorAll('#askChoices button')].map(b => b.textContent).join('|')) === 'Abrir sem salvar|Salvar e abrir'
      && await x.page.evaluate(() => !document.querySelector('#askDialog button[data-i18n="common.cancel"]').hidden));
    ok('3 a troca não começa antes da decisão (nenhum POST /api/project)', x.reqs.filter(q => q.chave === 'POST /api/project').length === postsAntes);
    await cancela(x.page);
    ok('3 Cancelar: o projeto continua o mesmo, com a edição e o indicador', await x.page.evaluate(() => __t.studio.project.key) === CHAVE_RENDERER
      && (await x.page.inputValue('#inputs')).endsWith('f9-edicao') && await x.page.evaluate(() => !document.querySelector('#headUnsaved').hidden), await nota(x.page, '#headUnsaved'));
    ok('3 Cancelar: diz que nada mudou', /Nada mudou/.test(await nota(x.page, '#projectNote')), await nota(x.page, '#projectNote'));
    // Fechamento do F9: Cancelar devolve o campo ao caminho do projeto aberto; cada nova tentativa escreve de novo o caminho de B.
    ok('3 Cancelar: nenhum pedido de troca nem gravação', x.reqs.filter(q => q.chave === 'POST /api/project').length === postsAntes && !x.puts);
    // Salvar e abrir, com o Salvar cancelado: nada muda.
    await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject'); await dialogo(x.page);
    await escolhe(x.page, 'save'); await dialogo(x.page);
    ok('3 Salvar e abrir: mostra o Salvar explícito, com a lista', /Salvar estas alterações no perfil do projeto\?/.test(await mensagem(x.page)) && (await mensagem(x.page)).includes('Entradas'), await mensagem(x.page));
    await cancela(x.page);
    ok('3 Salvar cancelado: fica no projeto, com a edição, sem gravar nem trocar', await x.page.evaluate(() => __t.studio.project.key) === CHAVE_RENDERER
      && (await x.page.inputValue('#inputs')).endsWith('f9-edicao') && !x.puts && x.reqs.filter(q => q.chave === 'POST /api/project').length === postsAntes);
    // Salvar recusado pelo servidor: não troca e a edição continua não salva.
    x.fake['PUT /api/project'] = r => { x.puts = (x.puts ?? 0) + 1; return r.fulfill({ status: 400, json: { error: 'recusado pela bateria' } }); };
    await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject'); await dialogo(x.page);
    await escolhe(x.page, 'save'); await dialogo(x.page); await confirma(x.page);
    await x.page.waitForFunction(() => !document.querySelector('#openProject').disabled, null, { timeout: 30000 });
    ok('3 Salvar recusado: não troca de projeto', await x.page.evaluate(() => __t.studio.project.key) === CHAVE_RENDERER && x.reqs.filter(q => q.chave === 'POST /api/project').length === postsAntes);
    ok('3 Salvar recusado: diz que o projeto não foi trocado, com a mensagem do servidor', /O projeto não foi trocado: o Salvar falhou \(recusado pela bateria\)/.test(await nota(x.page, '#projectNote')), await nota(x.page, '#projectNote'));
    ok('3 Salvar recusado: a edição continua, marcada não salva', (await x.page.inputValue('#inputs')).endsWith('f9-edicao') && await x.page.evaluate(() => __t.unsavedFields().join()) === 'inputs'
      && await x.page.evaluate(() => !document.querySelector('#headUnsaved').hidden));
    ok('3 T17: depois do Salvar recusado, sair ainda avisa', await pedeAviso(x.page));
    // Salvar e abrir: grava só pelo Salvar explícito, depois troca.
    respondePut(x); x.puts = 0;
    await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject'); await dialogo(x.page);
    await escolhe(x.page, 'save'); await dialogo(x.page); await confirma(x.page);
    await x.page.waitForFunction(k => __t.studio.project?.key === k, CHAVE_B, { timeout: 60000 }); await pronta(x.page);
    const iPut = x.reqs.findIndex(q => q.chave === 'PUT /api/project'), iPost = x.reqs.findLastIndex(q => q.chave === 'POST /api/project');
    ok('3 Salvar e abrir: um PUT com a edição, antes do pedido de troca', x.puts === 1 && x.putBody?.settings?.inputs?.includes('f9-edicao') && iPut >= 0 && iPut < iPost, JSON.stringify(x.putBody?.settings?.inputs));
    ok('3 Salvar e abrir: o PUT leva o perfil relido mais só a edição (política igual)', x.putBody.settings.allow_hypothesis === PROJETO.settings.allow_hypothesis && x.putBody.settings.allow_patch === PROJETO.settings.allow_patch
      && x.putBody.settings.criteria === PROJETO.settings.criteria);
    ok('3 Salvar e abrir: abriu B, sem alterações não salvas e sem o indicador', await x.page.evaluate(() => __t.studio.project.key) === CHAVE_B && await x.page.evaluate(() => __t.unsavedFields().length) === 0
      && await x.page.evaluate(() => document.querySelector('#headUnsaved').hidden));
    // Abrir sem salvar: descarta e diz o que foi descartado.
    original = await edita(x.page);
    postsAntes = x.reqs.filter(q => q.chave === 'POST /api/project').length; x.puts = 0;
    await x.page.fill('#projectPath', RENDERER); await x.page.click('#openProject'); await dialogo(x.page);
    await escolhe(x.page, 'discard');
    await x.page.waitForFunction(k => __t.studio.project?.key === k && !document.querySelector('#openProject').disabled, CHAVE_RENDERER, { timeout: 120000 }); await pronta(x.page);
    ok('3 Abrir sem salvar: troca, sem gravar, e diz o que descartou', !x.puts && /Alterações não salvas descartadas de ficticio-f9b: Entradas/.test(await nota(x.page, '#projectNote')), await nota(x.page, '#projectNote'));
    ok('3 Abrir sem salvar: o projeto aberto mostra o perfil dele, sem alterações não salvas', await x.page.evaluate(() => __t.unsavedFields().length) === 0 && await x.page.evaluate(() => document.querySelector('#headUnsaved').hidden)
      && await x.page.inputValue('#inputs') === (PROJETO.settings.inputs ?? []).join(', '));
    // Os outros caminhos: Enter no campo, Procurar › Escolher e Limpar.
    await edita(x.page);
    postsAntes = x.reqs.filter(q => q.chave === 'POST /api/project').length;
    await x.page.fill('#projectPath', RAIZ_B); await x.page.press('#projectPath', 'Enter'); await dialogo(x.page);
    ok('3 Enter no campo do projeto: a mesma confirmação, antes de trocar', await titulo(x.page) === 'Trocar de projeto com alterações não salvas?' && x.reqs.filter(q => q.chave === 'POST /api/project').length === postsAntes, `${await titulo(x.page)} ${x.reqs.filter(q => q.chave === 'POST /api/project').length} ${postsAntes}`);
    await cancela(x.page);
    x.fake['GET /api/browse'] = r => r.fulfill({ json: { path: RAIZ_B, parent: null, marker: null, dirs: [] } });
    await x.page.click('#browse'); await x.page.waitForFunction(() => document.querySelector('#browseDialog').open && !document.querySelector('#browsePick').disabled);
    await x.page.click('#browsePick'); await dialogo(x.page);
    ok('3 Procurar › Escolher: a mesma confirmação, antes de trocar', await titulo(x.page) === 'Trocar de projeto com alterações não salvas?' && x.reqs.filter(q => q.chave === 'POST /api/project').length === postsAntes);
    await cancela(x.page);
    ok('3 … e Cancelar mantém o projeto e a edição', await x.page.evaluate(() => __t.studio.project.key) === CHAVE_RENDERER && (await x.page.inputValue('#inputs')).endsWith('f9-edicao'));
    await x.page.click('#resetStudio'); await dialogo(x.page);
    ok('3 Limpar: a confirmação própria', await titulo(x.page) === 'Limpar com alterações não salvas?' && await x.page.evaluate(() => [...document.querySelectorAll('#askChoices button')].map(b => b.textContent).join('|')) === 'Limpar sem salvar|Salvar e limpar');
    await cancela(x.page);
    ok('3 Limpar cancelado: o projeto continua aberto', await x.page.evaluate(() => __t.studio.project?.key) === CHAVE_RENDERER && (await x.page.inputValue('#inputs')).endsWith('f9-edicao'));
    await x.page.click('#resetStudio'); await dialogo(x.page); await escolhe(x.page, 'discard'); await x.page.waitForTimeout(400);
    ok('3 Limpar sem salvar: limpa a página, sem gravar', await x.page.evaluate(() => __t.studio.project) === null && !x.puts);
    ok('3 sem erros de página; nenhuma escrita além dos PUT respondidos aqui', !x.errors.length && !x.writes.length, `${x.errors.join(' | ')} ${x.writes.join()}`);
    await fecha(x);

    // ================================================================ 4. T17: aviso ao recarregar ou sair
    x = await pagina(browser);
    await vai(x.page, 'project'); await x.page.click('#areas [data-area="ruler"]');
    ok('4 sem alterações não salvas: nenhum aviso', !(await pedeAviso(x.page)));
    await x.page.evaluate(() => __t.toggleGathered(__t.state.nodes.find(n => n.category === 'symbol' && n.kind === 'method').id, 'inspection'));
    await x.page.waitForTimeout(300);
    ok('4 rascunho guardado neste navegador, sem alterações: nenhum aviso', await x.page.evaluate(() => __t.gathered.length) === 1 && await x.page.evaluate(() => /guardado neste navegador/.test(document.querySelector('#headDraft').textContent)) && !(await pedeAviso(x.page)));
    ok('4 sair de verdade (fechar) sem alterações: o navegador não pergunta', !(await perguntaAoSair(x.page)));
    await x.context.close();
    x = await pagina(browser);
    await edita(x.page);
    ok('4 com alterações não salvas: recarregar ou sair pede o aviso do navegador', await pedeAviso(x.page));
    ok('4 sair de verdade (fechar) com alterações: o navegador pergunta', await perguntaAoSair(x.page));
    await x.context.close();
    // Recarregar de verdade: o navegador pergunta; aceito, a edição se perde e o perfil fica como estava.
    x = await pagina(browser);
    await edita(x.page);
    let perguntou = false;
    x.page.once('dialog', d => { perguntou = d.type() === 'beforeunload'; d.accept(); });
    await x.page.reload().catch(() => {});
    await x.page.waitForFunction(() => window.__t && __t.studio.project, null, { timeout: 120000 }); await pronta(x.page);
    ok('4 recarregar de verdade com alterações: o navegador pergunta antes', perguntou);
    ok('4 … aceito, a página volta sem a edição (o aviso não guarda nada)', !(await x.page.inputValue('#inputs')).includes('f9-edicao') && !(await pedeAviso(x.page)));
    // Depois de Salvar com sucesso, nada a avisar.
    respondePut(x);
    await edita(x.page);
    await vai(x.page, 'settings'); await x.page.click('#saveSettings'); await dialogo(x.page); await confirma(x.page); await x.page.waitForTimeout(400);
    ok('4 depois de Salvar com sucesso: nenhum aviso', !(await pedeAviso(x.page)) && x.puts === 1);
    await fecha(x);
    // Armazenamento bloqueado: com edição avisa; sem edição e sem rascunho, não.
    x = await pagina(browser, { bloqueado: true });
    ok('4 armazenamento bloqueado, sem edição nem rascunho: nenhum aviso', !(await pedeAviso(x.page)));
    await edita(x.page);
    ok('4 armazenamento bloqueado, com edição: avisa', await pedeAviso(x.page));
    await fecha(x);
    // Conflito entre abas: o rascunho guardado é o da outra aba; só uma edição do perfil pede aviso.
    {
      const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const a = await pagina(browser, { context: ctx }); await trocaRun(a.page, RUN);
      const no = await a.page.evaluate(() => __t.state.nodes.find(n => n.category === 'symbol' && n.kind === 'method').id);
      await a.page.evaluate(id => __t.toggleGathered(id, 'inspection'), no); await a.page.waitForTimeout(300);
      const b = await pagina(browser, { context: ctx }); await trocaRun(b.page, RUN);
      await b.page.evaluate(() => __t.toggleGathered(__t.gathered[0], 'inspection')); await b.page.waitForTimeout(500);
      ok('4 conflito entre abas: esta aba entra em "alterado em outra aba"', await a.page.evaluate(() => /outra aba/.test(document.querySelector('#headDraft').textContent)), await a.page.evaluate(() => document.querySelector('#headDraft').textContent));
      ok('4 conflito, sem edição do perfil: nenhum aviso (o guardado é o da outra aba)', !(await pedeAviso(a.page)));
      await edita(a.page);
      ok('4 conflito, com edição do perfil: avisa', await pedeAviso(a.page));
      await ctx.close();
    }

    // ================================================================ 5. Régua (2.5)
    x = await pagina(browser);
    await trocaRun(x.page, RUN);
    await vai(x.page, 'ruler');
    const regua = await x.page.evaluate(() => [...document.querySelectorAll('#commandsList li[data-command]')].map(li => ({ name: li.dataset.command, text: li.textContent,
      base: li.querySelector('[data-baseline]')?.dataset.baseline ?? null, run: li.querySelector('button.primary')?.disabled })));
    // Fechamento do F9: o critério da escada (main.rs, portão de /api/ladder): qualquer versão do comando.
    const baseOf = c => execs.filter(z => z.name === c.name && !z.change && !z.instrumentation && z.summary.succeeded > 0).reduce((a, z) => (!a || z.started > a.started ? z : a), null);
    ok('5 cada comando com a versão (hash) e "aprovado nesta versão" ou "não aprovado nesta versão"', regua.length === commands.commands.length && commands.commands.every((c, k) => regua[k].text.includes(`versão ${c.sha256.slice(0, 12)}`)
      && regua[k].text.includes(c.approved ? `aprovado nesta versão em ${c.approved}` : 'não aprovado nesta versão')), regua.map(z => z.text.slice(0, 80)).join(' | '));
    ok('5 linha de base em vigor: a última com sucesso, sem mudança e sem instrumentação, de qualquer versão (critério da escada)', commands.commands.every((c, k) => (baseOf(c)?.id ?? '') === regua[k].base), JSON.stringify(regua.map(z => z.base)));
    ok('5 Rodar habilitado nos aprovados de uma run sem alterações', commands.commands.every((c, k) => regua[k].run === !c.approved));
    const hist = await x.page.evaluate(() => [...document.querySelectorAll('#executionsList li[data-execution]')].map(li => ({ id: li.dataset.execution, kind: li.dataset.kind, btn: li.querySelector(':scope > button')?.textContent ?? null })));
    const tipo = z => (z.change ? 'change' : z.instrumentation ? 'measure' : 'baseline');
    ok('5 histórico: mudança, medição e linha de base como o registro diz', hist.length === execs.length && execs.every(z => hist.find(h => h.id === z.id)?.kind === tipo(z)));
    ok('5 histórico: cada mudança com "Abrir a pergunta #N"; cada medição com "→ Onde pesa"; linha de base sem link', execs.every(z => { const h = hist.find(y => y.id === z.id);
      return z.change ? h.btn === `Abrir a pergunta #${z.change.turn}` : z.instrumentation ? h.btn === '→ Onde pesa' : h.btn === null; }));
    const mud = execs.find(z => z.change && turns.some(t => t.id === z.change.turn));
    await x.page.click(`#executionsList li[data-execution="${mud.id}"] > button`); await x.page.waitForTimeout(400);
    ok('5 "Abrir a pergunta #N" ativa a pergunta (T9), guardada como escolha da pessoa, e a mostra', await x.page.evaluate(() => __t.studio.activeTurn) === mud.change.turn
      && JSON.parse(await navGuardada(x.page)).active_turn?.[RUN] === mud.change.turn && await x.page.evaluate(id => Boolean(document.querySelector(`#turns .turn.active[data-turn="${id}"]`)), mud.change.turn));
    ok('5 … sem tocar o rascunho nem trocar de área', await x.page.evaluate(() => __t.navigation.area) === 'ruler' && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === '[]');
    const med = execs.find(z => z.instrumentation);
    if (med) {
      await x.page.click(`#executionsList li[data-execution="${med.id}"] > button`); await x.page.waitForTimeout(300);
      ok('5 "→ Onde pesa" leva a Explorar › Onde pesa, com o ponto no Voltar', await x.page.evaluate(() => __t.navigation.area === 'explore' && __t.navigation.explore === 'hot' && !document.querySelector('#navBack').disabled));
    }
    ok('5 nenhuma execução pedida', !x.reqs.some(q => q.chave === 'POST /api/commands/run'));
    await fecha(x);
    // Run com alterações: Rodar desabilitado antes do clique, com o motivo.
    x = await pagina(browser, { antes: async y => { y.fake['POST /api/project'] = async (r, body) => { if (body?.root !== RENDERER) return false; const res = await r.fetch(), data = await res.json();
      data.runs = data.runs.map(z => (z.id === RUN ? { ...z, git: { ...(z.git ?? {}), dirty: true } } : z)); return r.fulfill({ json: data }); }; } });
    await trocaRun(x.page, RUN); await vai(x.page, 'ruler');
    const sujo = await x.page.evaluate(() => [...document.querySelectorAll('#commandsList li[data-command]')].map(li => ({ disabled: li.querySelector('button.primary').disabled, title: li.querySelector('button.primary').title, text: li.textContent })));
    ok('5 run com alterações: Rodar desabilitado em todos os comandos, antes do clique', sujo.every(z => z.disabled));
    ok('5 … com o motivo escrito (título e texto nos aprovados)', sujo.every(z => /alterações não commitadas/.test(z.title)) && sujo.some(z => /Rodar desabilitado: run com alterações não commitadas/.test(z.text)));
    ok('5 resumo: a Run diz "com alterações" e leva à Régua', (await resumo(x.page)).run.text.includes('com alterações') && (await resumo(x.page)).run.link);
    await fecha(x);

    // ================================================================ 6. Acervo (2.6, T13)
    x = await pagina(browser);
    await trocaRun(x.page, RUN); await vai(x.page, 'facts');
    const lista = () => x.page.evaluate(() => [...document.querySelectorAll('#factsList li[data-fact]')].map(li => ({ id: li.dataset.fact, status: li.dataset.status,
      actions: [...li.querySelectorAll('[data-action]')].map(b => b.dataset.action).join() })));
    let fl = await lista();
    ok('6 lista: os fatos de /api/facts, com o estado de cada um', fl.length === facts.length && facts.every(f => fl.find(z => z.id === f.id)?.status === f.status));
    ok('6 cada fato vigente com Ir à origem, Revogar… e Juntar a declaração', fl.filter(z => z.status === 'current').every(z => z.actions === 'origin,revoke,join'));
    const opcoes = await x.page.evaluate(() => [...document.querySelectorAll('#factsFilter option')].map(o => `${o.value}:${o.textContent}`));
    ok('6 filtro por estado: todos e os estados presentes, com as contagens', opcoes[0] === `:todos (${facts.length})` && opcoes.includes(`current:vigente (${cont('current')})`) && (!cont('stale') || opcoes.includes(`stale:desatualizado (${cont('stale')})`)), opcoes.join(' | '));
    if (cont('stale')) {
      await x.page.selectOption('#factsFilter', 'stale'); await x.page.waitForTimeout(200);
      fl = await lista();
      ok('6 filtro "desatualizado": só os desatualizados', fl.length === cont('stale') && fl.every(z => z.status === 'stale'));
      await x.page.selectOption('#factsFilter', ''); await x.page.waitForTimeout(200);
    }
    // Ir à origem na mesma run: ativa a pergunta, mostra o envio e o item.
    const daRun = facts.find(f => f.run === RUN && f.kind === 'FACT');
    await x.page.click(`#factsList li[data-fact="${daRun.id}"] [data-action="origin"]`); await x.page.waitForTimeout(500);
    ok('6 Ir à origem (mesma run): sem perguntar, ativa a pergunta de origem', !(await aberto(x.page)) && await x.page.evaluate(() => __t.studio.activeTurn) === daRun.turn);
    ok('6 … mostra o envio e o item de origem (camada de decisão)', await x.page.evaluate(([t, s, c]) => Boolean(document.querySelector(`#turns .turn[data-turn="${t}"] [data-send="${s}"] .layer-decide [data-claim="${c}"].origin-shown`)), [daRun.turn, daRun.send, daRun.claim_id]));
    ok('6 … e diz qual é a origem', (await nota(x.page, '#factsNote')).startsWith(`Origem do fato ${daRun.id}: pergunta #${daRun.turn}`), await nota(x.page, '#factsNote'));
    // Revogar…: leva ao controle de decisão do item, com o foco em Revogar; nada é gravado.
    await x.page.click(`#factsList li[data-fact="${daRun.id}"] [data-action="revoke"]`); await x.page.waitForTimeout(500);
    ok('6 Revogar…: o foco vai ao "Revogar" do item de origem, sem gravar nada', await x.page.evaluate(() => document.activeElement?.dataset.decide) === 'revoke'
      && await x.page.evaluate(() => document.activeElement.closest('[data-claim]')?.classList.contains('origin-shown')) && !x.reqs.some(q => q.chave === 'POST /api/review'));
    // Juntar a declaração: a declaração exata (aresta declares na linha do fato), chegou por Acervo.
    const esperado = grafo.edges.filter(e => e.kind === 'declares' && e.evidence?.path === daRun.path && e.evidence?.start_line === daRun.declaration_line)
      .map(e => e.to).filter(id => grafo.nodes[id]?.category === 'symbol' && (grafo.nodes[id].label === daRun.symbol_name || grafo.nodes[id].label.endsWith(`::${daRun.symbol_name}`)));
    await x.page.click(`#factsList li[data-fact="${daRun.id}"] [data-action="join"]`); await x.page.waitForTimeout(400);
    ok('6 Juntar a declaração: a fonte é a declaração do fato, no grafo da run', esperado.length === 1 && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === JSON.stringify(esperado), JSON.stringify(esperado));
    ok('6 … chegou por "Acervo" (só local) e proveniência manual', await x.page.evaluate(id => __t.draft.arrived.get(id), esperado[0]) === 'facts'
      && /Declaração intersect_world juntada ao rascunho/.test(await nota(x.page, '#factsNote')));
    ok('6 … o rascunho é guardado na run aberta, sem gravar no servidor', Boolean(await x.page.evaluate(k => localStorage.getItem(k), DRAFT(CHAVE_RENDERER, RUN))) && !x.writes.length);
    await x.page.click(`#factsList li[data-fact="${daRun.id}"] [data-action="join"]`); await x.page.waitForTimeout(200);
    ok('6 juntar de novo não duplica: "já está no rascunho"', (await x.page.evaluate(() => __t.gathered.length)) === 1 && /já está no rascunho/.test(await nota(x.page, '#factsNote')));
    // Ir à origem em outra run: T13.
    const deOutra = facts.find(f => f.run === OUTRA && f.kind === 'FACT');
    const rascunhoAntes = await x.page.evaluate(k => localStorage.getItem(k), DRAFT(CHAVE_RENDERER, RUN));
    await x.page.click(`#factsList li[data-fact="${deOutra.id}"] [data-action="origin"]`); await dialogo(x.page);
    ok('6 T13: "A origem está na run X. Trocar?"', (await mensagem(x.page)).startsWith(`A origem está na run ${OUTRA}. Trocar?`), await mensagem(x.page));
    await cancela(x.page);
    ok('6 T13 cancelado: a run e a navegação não mudam', await x.page.evaluate(() => __t.studio.run) === RUN && JSON.parse(await navGuardada(x.page)).run === RUN);
    await x.page.click(`#factsList li[data-fact="${deOutra.id}"] [data-action="origin"]`); await dialogo(x.page); await confirma(x.page);
    await x.page.waitForFunction(r => __t.studio.run === r && __t.studio.turnsState === 'ready' && __t.studio.activeTurn != null, OUTRA, { timeout: 120000 }); await x.page.waitForTimeout(600);
    ok('6 T13 confirmado: troca de run e ativa a pergunta de origem', await x.page.evaluate(() => __t.studio.activeTurn) === deOutra.turn);
    ok('6 T13: a run e a pergunta ficam guardadas como escolha da pessoa', JSON.parse(await navGuardada(x.page)).run === OUTRA && JSON.parse(await navGuardada(x.page)).active_turn?.[OUTRA] === deOutra.turn);
    ok('6 T13: mostra o envio e o item de origem', await x.page.evaluate(([t, s, c]) => Boolean(document.querySelector(`#turns .turn[data-turn="${t}"] [data-send="${s}"] .layer-decide [data-claim="${c}"].origin-shown`)), [deOutra.turn, deOutra.send, deOutra.claim_id]));
    ok('6 T13: o rascunho da run de saída fica guardado, e não vem para esta run', await x.page.evaluate(k => localStorage.getItem(k), DRAFT(CHAVE_RENDERER, RUN)) === rascunhoAntes
      && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === '[]');
    ok('6 nenhum PUT, nenhuma revisão, nenhuma escrita', !x.writes.length && !x.reqs.some(q => ['PUT /api/project', 'POST /api/review'].includes(q.chave)), x.writes.join());
    await fecha(x);
    // Estados: revogado sem Revogar nem Juntar; declaração fora do grafo; vazio; carregando.
    x = await pagina(browser, { antes: async y => { y.fake['GET /api/facts'] = async r => { const res = await r.fetch(), data = await res.json();
      data.facts = data.facts.map((f, k) => (k === 0 ? { ...f, status: 'revoked', status_reason: 'Revogado em teste: bateria' } : k === 1 ? { ...f, status: 'current', path: 'src/nao-existe.rs', declaration_line: 1, citations: [{ path: 'src/nao-existe.rs', start_line: 1, end_line: 1, quote: 'fn x() {}' }] } : f));
      return r.fulfill({ json: data }); }; } });
    await trocaRun(x.page, RUN); await vai(x.page, 'facts');
    fl = await lista();
    ok('6 fato revogado: só Ir à origem (sem Revogar nem Juntar)', fl[0].status === 'revoked' && fl[0].actions === 'origin');
    await x.page.click(`#factsList li[data-fact="${fl[1].id}"] [data-action="join"]`); await x.page.waitForTimeout(200);
    ok('6 declaração fora do grafo da run: nada é juntado, e diz por quê', JSON.stringify(await x.page.evaluate(() => __t.gathered)) === '[]' && /não está no grafo desta run: nada foi juntado/.test(await nota(x.page, '#factsNote')), await nota(x.page, '#factsNote'));
    await fecha(x);
    x = await pagina(browser, { antes: async y => { y.fake['GET /api/facts'] = async r => { const res = await r.fetch(), data = await res.json(); return r.fulfill({ json: { ...data, facts: [] } }); }; } });
    await vai(x.page, 'facts');
    ok('6 Acervo vazio: "Nenhum fato aprovado neste projeto" (2.6) no Acervo e no resumo', (await nota(x.page, '#factsList')).startsWith('Nenhum fato aprovado neste projeto.') && (await resumo(x.page)).facts.text === 'Nenhum fato aprovado neste projeto');
    await fecha(x);
    {
      const h = segura();
      x = await pagina(browser, { espera: false, project: null, storage: { 'librarian.project': RENDERER }, antes: async y => { y.fake['GET /api/facts'] = async r => { await h.p; return r.continue(); }; } });
      await x.page.waitForFunction(() => __t.studio.run && __t.studio.turnsState === 'ready', null, { timeout: 120000 });
      await vai(x.page, 'facts');
      ok('6 Acervo carregando: "Carregando os fatos…"', /Carregando os fatos/.test(await nota(x.page, '#factsList')));
      h.solta(); await pronta(x.page);
      await fecha(x);
    }

    // ================================================================ 7. 9.10: Rust com medição, Python sem instrument
    x = await pagina(browser, { base: P, project: PYTHON, key: CHAVE_PYTHON });
    const cmdPy = await get(P, `/api/commands?key=${CHAVE_PYTHON}&run=${await x.page.evaluate(() => __t.studio.run)}`);
    await vai(x.page, 'project');
    r = await resumo(x.page);
    const motivo = cmdPy.measure_all?.available === false ? await T(x.page, `hot.unavailable.${cmdPy.measure_all.reason}`, cmdPy.measure_all) : null;
    ok('7 Python: Medição "Não disponível" com o motivo, no resumo', motivo && r.measure.text === motivo, `${r.measure.text} / ${JSON.stringify(cmdPy.measure_all)}`);
    await x.page.click('#projectSummary [data-summary="measure"] button');
    ok('7 Python: o item leva a Explorar › Onde pesa, que diz o mesmo motivo', await x.page.evaluate(() => __t.navigation.explore) === 'hot' && (await nota(x.page, '#hotState')) === motivo);
    await x.page.click('.subnav [data-view="inventory"]');
    ok('7 Python: o Inventário é o caminho (Montar habilitado)', await x.page.evaluate(() => !document.querySelector('#inventoryBuild').disabled));
    ok('7 Python: sem erros de página', !x.errors.length, x.errors.join(' | '));
    await fecha(x);
    x = await pagina(browser);
    await trocaRun(x.page, RUN);
    await x.page.click('#projectSummary [data-summary="measure"] button');
    await x.page.waitForFunction(() => document.querySelectorAll('#hotList select option').length > 1, null, { timeout: 60000 }).catch(() => {});
    ok('7 Rust: Medição disponível leva a Onde pesa, com o ranking', await x.page.evaluate(() => __t.navigation.explore === 'hot' && document.querySelectorAll('#hotList select option').length > 1));
    await fecha(x);

    // ================================================================ 8. tela estreita, idiomas, temas, teclado, chaves
    x = await pagina(browser, { size: { width: 390, height: 844 }, scheme: 'dark' });
    await x.page.click('#colSwitch [data-col="nav"]'); await vai(x.page, 'project');
    const larg = await x.page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    ok('8 tela estreita: o resumo cabe na largura', larg <= 1, String(larg));
    const cores = await x.page.evaluate(() => { const b = document.querySelector('#projectSummary .summary-label'); const bg = getComputedStyle(document.body).backgroundColor; return [getComputedStyle(b).color, bg]; });
    ok('8 tema escuro: contraste do rótulo do resumo ≥ 4,5', contraste(cores) >= 4.5, contraste(cores).toFixed(2));
    await x.page.click('#projectSummary [data-summary="commands"] button');
    ok('8 tela estreita: o item leva à Régua na coluna de navegação', await x.page.evaluate(() => __t.navigation.area === 'ruler' && __t.navigation.column === 'nav'));
    await fecha(x);
    x = await pagina(browser, { scheme: 'light' });
    const coresClaro = await x.page.evaluate(() => [...document.querySelectorAll('#projectSummary .summary-label, #projectSummary .summary-value')].map(e => [getComputedStyle(e).color, getComputedStyle(document.body).backgroundColor]));
    ok('8 tema claro: contraste do resumo ≥ 4,5', coresClaro.every(c => contraste(c) >= 4.5), coresClaro.map(c => contraste(c).toFixed(2)).join());
    await vai(x.page, 'project');
    await x.page.focus('#projectSummary [data-summary="facts"] button'); await x.page.keyboard.press('Enter'); await x.page.waitForTimeout(200);
    ok('8 teclado: Enter num item do resumo leva ao destino', await x.page.evaluate(() => __t.navigation.area) === 'facts');
    await x.page.click('[data-lang="en"]'); await x.page.waitForTimeout(800); await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('8 EN: o resumo em inglês', (await x.page.textContent('#summaryTitle')) === 'Project summary' && /approved in the current version/.test(r.commands.text), r.commands.text);
    await vai(x.page, 'facts');
    ok('8 EN: o Acervo em inglês', /Go to origin/.test(await nota(x.page, '#factsList')));
    await edita(x.page);
    await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject'); await dialogo(x.page);
    ok('8 EN: a confirmação do T25 em inglês', await titulo(x.page) === 'Switch project with unsaved changes?');
    await cancela(x.page);
    await x.page.click('[data-lang="pt"]'); await x.page.waitForTimeout(400);
    const chaves = await x.page.evaluate(() => Object.entries(__t.STRINGS).filter(([k]) => /^(summary|leave)\./.test(k) || ['project.keptRunGone', 'commands.version', 'commands.approvedHere', 'commands.unapprovedHere', 'commands.baseline', 'commands.noBaseline', 'commands.dirty',
      'executions.openTurn', 'executions.measureShort', 'executions.toHot', 'executions.turnGone', 'builder.via.facts'].includes(k) || /^facts\.(filter|loading|unread|noRun|toOrigin|revokeAt|join|switch|runGone|turnGone|origin)/.test(k)));
    ok('8 as chaves do F9 existem em PT e EN, não vazias, com as mesmas variáveis', chaves.length > 60 && chaves.every(([, v]) => Array.isArray(v) && v.length === 2 && v[0] && v[1]
      && JSON.stringify((v[0].match(/\{\w+\}/g) ?? []).sort()) === JSON.stringify((v[1].match(/\{\w+\}/g) ?? []).sort())), String(chaves.length));
    ok('8 sem erros de página', !x.errors.length, x.errors.join(' | '));
    await fecha(x);

    // ================================================================ 9. fechamento do F9 (13/11): M1, M2, carregamento independente, T25 e decisões
    const junta = async (id, page = x.page) => { await page.click(`#factsList li[data-fact="${id}"] [data-action="join"]`); await page.waitForTimeout(250); return { note: await nota(page, '#factsNote'), gathered: await page.evaluate(() => __t.gathered.slice()) }; };
    let j;
    // Cada bloco da seção 9 roda à parte: uma exceção vira a FALHA do bloco e o seguinte continua (contraprova verificação a verificação).
    const fechaBloco = async (nome, e) => { ok(`9 bloco ${nome} terminou sem exceção`, false, e.stack); await x?.context?.close().catch(() => {}); };
    const SUMARIO = ['measure', 'commands', 'manual', 'facts', 'questions'];
    const terminais = r => SUMARIO.every(k => r[k] && !/Carregando/.test(r[k].text));
    const esperaTerminais = page => page.waitForFunction(ks => ks.every(k => { const v = document.querySelector(`#projectSummary [data-summary="${k}"] .summary-value`); return v && !/Carregando/.test(v.textContent); }), SUMARIO, { timeout: 120000 });
    const rascunhos = page => page.evaluate(() => Object.fromEntries(Object.keys(localStorage).filter(k => k.startsWith('librarian.draft.v1:')).map(k => [k, localStorage.getItem(k)])));
    const campos = page => page.evaluate(() => ['#inputs', '#lexicon', '#instructions', '#top', '#depth', '#hits', '#radius', '#numbered', '#acceptDiag', '#excludeTests', '#answerFormat', '#allowInference'].map(s => document.querySelector(s).disabled));
    const GRAFO_FALHA = 'grafo ilegível (bateria)';
    const [cmdOutra, factsOutra, turnsOutra2] = [await get(B, `/api/commands?key=${CHAVE_RENDERER}&run=${OUTRA}`), (await get(B, `/api/facts?key=${CHAVE_RENDERER}&run=${OUTRA}`)).facts, await get(B, `/api/turns?key=${CHAVE_RENDERER}&run=${OUTRA}`)];

    // ---- 9a M1: a leitura do grafo falha ao abrir uma run (a run anterior tinha grafo e rascunho).
    try {
    x = await pagina(browser, { antes: async y => { y.fake['GET /api/graph'] = (r, b, url) => (url.searchParams.get('run') === OUTRA ? r.fulfill({ status: 500, json: { error: GRAFO_FALHA } }) : false); } });
    await trocaRun(x.page, RUN);
    const fonteM1 = await x.page.evaluate(() => __t.state.nodes.find(n => n.category === 'symbol' && n.kind === 'method').id);
    await x.page.evaluate(id => __t.toggleGathered(id, 'inspection'), fonteM1); await x.page.waitForTimeout(300);
    const rascAntes = await rascunhos(x.page);
    await x.page.selectOption('#runs', OUTRA);
    await x.page.waitForFunction(() => __t.studio.graphState === 'failed', null, { timeout: 60000 });
    await esperaTerminais(x.page).catch(() => {});
    await x.page.waitForFunction(() => __t.studio.turnsState !== 'loading', null, { timeout: 60000 }).catch(() => {});
    await x.page.waitForTimeout(400);
    await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('9 M1 grafo ilegível: Medição, Comandos, Contagens, Fatos e Perguntas chegam a um estado terminal (nenhum "Carregando…")', terminais(r), JSON.stringify(r));
    const contO = st => factsOutra.filter(f => f.status === st).length, pendO = turnsOutra2.filter(t => (t.sends ?? []).some(s => (s.check?.claims ?? []).some(k => !s.reviews?.[k.id])));
    ok('9 M1 … cada um com o que leu desta run (/api/commands, /api/facts e /api/turns da run aberta)', r.commands.text === await T(x.page, 'summary.commands', { n: cmdOutra.commands.filter(c => c.approved).length, m: cmdOutra.commands.length })
      && r.facts.text.startsWith(await T(x.page, 'summary.facts', { current: contO('current'), stale: contO('stale'), revoked: contO('revoked'), applied: contO('applied') }))
      && r.questions.text.startsWith(await T(x.page, 'summary.questions', { prepared: turnsOutra2.filter(t => !t.prepare_error && !(t.sends ?? []).length).length, pending: pendO.length })), JSON.stringify(r));
    ok('9 M1 a Run diz, como erro, que o grafo não pôde ser lido (com a mensagem do servidor) e leva à Régua', r.run.error && r.run.link && r.run.text.endsWith(`não foi possível ler o grafo: ${GRAFO_FALHA}`), r.run.text);
    const falhaTexto = `Não foi possível ler o grafo da run ${OUTRA}: ${GRAFO_FALHA}`;
    ok('9 M1 no lugar do grafo: a cidade e a nota da run dizem a falha', await nota(x.page, '#status') === falhaTexto && await nota(x.page, '#runNote') === falhaTexto
      && await x.page.evaluate(() => document.querySelector('#runNote').classList.contains('error')), `${await nota(x.page, '#status')} | ${await nota(x.page, '#runNote')}`);
    ok('9 M1 a nota distante do projeto não é o lugar da falha', !(await nota(x.page, '#projectNote')).includes(GRAFO_FALHA), await nota(x.page, '#projectNote'));
    ok('9 M1 nada do grafo da run anterior fica na tela como se fosse desta (cidade, seleção, proveniência)', await x.page.evaluate(() => __t.state.graph === null && __t.state.nodes.length === 0
      && __t.state.selectedNode === null && document.querySelector('#provenance').children.length === 0));
    ok('9 M1 a conversa chega às perguntas desta run (não fica em "Carregando as perguntas…")', await x.page.evaluate(() => __t.studio.turnsState) === 'ready' && await x.page.evaluate(() => __t.studio.turns.length) === turnsOutra2.length
      && !/Carregando as perguntas/.test(await nota(x.page, '#turns')));
    ok('9 M1 o rascunho guardado de cada run fica como estava; nenhum aparece nesta run', JSON.stringify(await rascunhos(x.page)) === JSON.stringify(rascAntes) && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === '[]');
    await vai(x.page, 'ruler');
    ok('9 M1 a Régua lista os comandos desta run', await x.page.evaluate(() => document.querySelectorAll('#commandsList li[data-command]').length) === cmdOutra.commands.length);
    await vai(x.page, 'facts');
    const vigO = factsOutra.find(f => f.kind === 'FACT' && f.status === 'current');
    await x.page.click(`#factsList li[data-fact="${vigO.id}"] [data-action="join"]`); await x.page.waitForTimeout(200);
    ok('9 M1 o Acervo lista os fatos; Juntar diz que o grafo não pôde ser lido, e nada é juntado', await x.page.evaluate(() => document.querySelectorAll('#factsList li[data-fact]').length) === factsOutra.length
      && await nota(x.page, '#factsNote') === falhaTexto && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === '[]', await nota(x.page, '#factsNote'));
    await x.page.click('[data-lang="en"]'); await x.page.waitForTimeout(600);
    ok('9 M1 EN: a falha continua dita no lugar dela, em inglês', await nota(x.page, '#status') === `Could not read the graph of run ${OUTRA}: ${GRAFO_FALHA}` && await nota(x.page, '#runNote') === `Could not read the graph of run ${OUTRA}: ${GRAFO_FALHA}`);
    await x.page.click('[data-lang="pt"]'); await x.page.waitForTimeout(400);
    await trocaRun(x.page, RUN);
    ok('9 M1 de volta a uma run com grafo legível: o grafo, o rascunho dela e a Run sem erro', await x.page.evaluate(() => __t.studio.graphState === 'ready' && Boolean(__t.state.graph))
      && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === JSON.stringify([fonteM1]) && !(await resumo(x.page)).run.error);
    ok('9 M1 sem erros de página e sem escritas', !x.errors.length && !x.writes.length, `${x.errors.join(' | ')} ${x.writes.join()}`);
    await fecha(x);
    // A primeira run aberta já com o grafo ilegível (recarregar o Studio): nada fica carregando.
    x = await pagina(browser, { espera: false, project: null, storage: { 'librarian.project': RENDERER }, antes: async y => { y.fake['GET /api/graph'] = r => r.fulfill({ status: 500, json: { error: GRAFO_FALHA } }); } });
    await x.page.waitForFunction(() => __t.studio.graphState === 'failed', null, { timeout: 120000 });
    await esperaTerminais(x.page).catch(() => {});
    await x.page.waitForFunction(() => __t.studio.turnsState !== 'loading', null, { timeout: 60000 }).catch(() => {});
    await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('9 M1 ao abrir o projeto com o grafo ilegível: todos os itens terminam; a falha está na Run, na cidade e na nota da run', terminais(r) && r.run.error && /não foi possível ler o grafo/.test(r.run.text)
      && /^Não foi possível ler o grafo da run/.test(await nota(x.page, '#status')) && /^Não foi possível ler o grafo da run/.test(await nota(x.page, '#runNote')) && await x.page.evaluate(() => __t.studio.turnsState) === 'ready', JSON.stringify(r));
    ok('9 M1 … sem erros de página', !x.errors.length, x.errors.join(' | '));
    await fecha(x);

    } catch (e) { await fechaBloco('9a', e); }
    // ---- 9b carregamento independente (2.1): atraso e falha de cada leitura.
    try {
    {
      const h = segura();
      // A run guardada é a que tem perguntas (a mais recente não tem nenhuma).
      x = await pagina(browser, { espera: false, project: null, storage: { 'librarian.project': RENDERER, [NAV(CHAVE_RENDERER)]: JSON.stringify({ run: RUN }) }, antes: async y => { y.fake['GET /api/graph'] = async r => { await h.p; return r.continue(); }; } });
      await x.page.waitForFunction(() => __t.studio.run && __t.studio.turnsState === 'ready' && __t.studio.facts && __t.studio.commandsState === 'ready', null, { timeout: 120000 });
      await esperaTerminais(x.page).catch(() => {});
      await vai(x.page, 'project');
      r = await resumo(x.page);
      ok('9 atraso do grafo: Medição, Comandos, Contagens, Fatos e Perguntas chegam antes dele', terminais(r), JSON.stringify(r));
      ok('9 atraso do grafo: a conversa mostra as perguntas; a cidade diz "Carregando grafo…" e não mostra grafo nenhum', await x.page.evaluate(() => __t.studio.turns.length > 0 && __t.state.graph === null && __t.studio.graphState === 'loading')
        && await nota(x.page, '#status') === 'Carregando grafo…');
      h.solta();
      await x.page.waitForFunction(() => __t.studio.graphState === 'ready' && __t.state.graph, null, { timeout: 120000 });
      ok('9 atraso do grafo: quando chega, a cidade aparece, com a pergunta ativa da run', await x.page.evaluate(() => __t.state.nodes.length > 0 && __t.studio.activeTurn != null));
      ok('9 atraso do grafo: sem erros de página', !x.errors.length, x.errors.join(' | '));
      await fecha(x);
    }
    {
      const h = segura();
      x = await pagina(browser, { espera: false, project: null, storage: { 'librarian.project': RENDERER }, antes: async y => { y.fake['GET /api/facts'] = async r => { await h.p; return r.continue(); }; } });
      await x.page.waitForFunction(() => __t.studio.run && __t.state.graph && __t.studio.turnsState === 'ready' && __t.studio.commandsState === 'ready', null, { timeout: 120000 });
      await x.page.waitForTimeout(300);
      await vai(x.page, 'project');
      r = await resumo(x.page);
      ok('9 atraso dos fatos: Comandos e Contagens chegam sem esperar os fatos (antes do fechamento, os comandos só eram lidos depois deles)', /aprovado/.test(r.commands.text) && !/Carregando/.test(r.manual.text) && r.facts.text === 'Carregando…', JSON.stringify(r));
      ok('9 atraso dos fatos: Perguntas e Medição também', !/Carregando/.test(r.questions.text) && !/Carregando/.test(r.measure.text), JSON.stringify(r));
      h.solta(); await pronta(x.page);
      ok('9 atraso dos fatos: depois, os fatos', !/Carregando/.test((await resumo(x.page)).facts.text));
      await fecha(x);
    }
    {
      const h = segura();
      x = await pagina(browser, { espera: false, project: null, storage: { 'librarian.project': RENDERER }, antes: async y => { y.fake['GET /api/turns'] = async r => { await h.p; return r.continue(); }; } });
      await x.page.waitForFunction(() => __t.studio.run && __t.state.graph && __t.studio.facts && __t.studio.commandsState === 'ready', null, { timeout: 120000 });
      await x.page.waitForTimeout(300);
      await vai(x.page, 'project');
      r = await resumo(x.page);
      ok('9 atraso das perguntas: Perguntas diz "Carregando…"; Fatos, Comandos, Contagens e Medição já mostram o que leram, e a cidade também', r.questions.text === 'Carregando…' && ['measure', 'commands', 'manual', 'facts'].every(k => !/Carregando/.test(r[k].text))
        && await x.page.evaluate(() => __t.state.nodes.length > 0), JSON.stringify(r));
      h.solta(); await pronta(x.page);
      ok('9 atraso das perguntas: depois, as perguntas', !/Carregando/.test((await resumo(x.page)).questions.text));
      await fecha(x);
    }
    x = await pagina(browser, { antes: async y => { y.fake['GET /api/hotspots'] = r => r.fulfill({ status: 500, json: { error: 'medição ilegível (bateria)' } }); } });
    await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('9 falha da medição: Medição diz que não foi possível ler; os outros itens não', r.measure.error && /medição ilegível/.test(r.measure.text) && ['commands', 'manual', 'facts', 'questions'].every(k => !r[k].error && !/Carregando/.test(r[k].text)), JSON.stringify(r));
    await fecha(x);

    } catch (e) { await fechaBloco('9b', e); }
    // ---- 9c M2: Juntar a declaração de funções Python decoradas (dados reais) e os casos sem correspondência e ambíguos.
    try {
    const projPy = await (await fetch(`${P}/api/project`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ root: PYTHON }) })).json(), RUN_PY = projPy.runs[0].id;
    const grafoPy = await get(P, `/api/graph?key=${CHAVE_PYTHON}&run=${RUN_PY}`), factsPy = (await get(P, `/api/facts?key=${CHAVE_PYTHON}&run=${RUN_PY}`)).facts;
    const fonte = rel => fs.readFileSync(path.join(PYTHON, rel), 'utf8').split(/\r?\n/);
    const declaradas = (g, p) => g.edges.filter(e => e.kind === 'declares' && e.evidence?.path === p && g.nodes[e.to]?.category === 'symbol').map(e => ({ id: e.to, label: g.nodes[e.to].label, s: e.evidence.start_line, e: e.evidence.end_line }));
    const F7 = factsPy.find(f => f.id === 'F7');
    const decF7 = declaradas(grafoPy, F7.path).filter(d => d.label === 'plan_route'), linhasF7 = fonte(F7.path);
    ok('9 M2 dado real: F7 (Python) guarda a linha 13 (o def); no grafo, a declaração de plan_route vai de 12 (o decorador) a 24', F7.status === 'current' && F7.declaration_line === 13 && decF7.length === 1 && decF7[0].s === 12 && decF7[0].e === 24
      && /^\s*@/.test(linhasF7[11]) && /def plan_route\(/.test(linhasF7[12]), JSON.stringify(decF7));
    // Fatos sintéticos sobre declarações reais do grafo (nenhuma aresta inventada): dois decoradores, nome inexistente,
    // linha fora da declaração e um fato que não cita a própria linha de declaração.
    const cita = (p, l, q) => [{ path: p, start_line: l, end_line: l, quote: q }];
    const sint = (id, p, nome, l, extra = {}) => ({ ...F7, id, path: p, symbol_name: nome, declaration_line: l, line: l, end_line: l, citations: cita(p, l, fonte(p)[l - 1] ?? ''), statement: `Fato sintético da bateria (${id})`, ...extra });
    const CFG = 'app/core/config.py', linhasCfg = fonte(CFG);
    const sinteticos = [sint('FD1', CFG, '_use_psycopg_driver', 37), sint('FD2', CFG, 'emails_enabled', 63), sint('FN1', F7.path, 'nao_existe', 13), sint('FN2', F7.path, 'plan_route', 30),
      sint('FU1', F7.path, 'plan_route', 13, { citations: cita(F7.path, 22, linhasF7[21]) })];
    x = await pagina(browser, { base: P, project: PYTHON, key: CHAVE_PYTHON, antes: async y => { y.fake['GET /api/facts'] = async r => { const res = await r.fetch(), data = await res.json(); return r.fulfill({ json: { ...data, facts: [...data.facts, ...sinteticos] } }); }; } });
    await vai(x.page, 'facts');
    j = await junta('F7');
    ok('9 M2 Juntar F7: junta a declaração decorada de plan_route (12–24), "chegou por Acervo"', JSON.stringify(j.gathered) === JSON.stringify([decF7[0].id]) && j.note === 'Declaração plan_route juntada ao rascunho (manual).'
      && await x.page.evaluate(id => __t.draft.arrived.get(id), decF7[0].id) === 'facts', `${j.note} ${JSON.stringify(j.gathered)}`);
    // Cada fato vigente real acha exatamente uma declaração: a que tem a linha do fato no cabeçalho, conferido no texto da fonte
    // (as linhas antes dela, desde o início da declaração, são decoradores, comentários ou vazias).
    const achadas = await x.page.evaluate(() => __t.studio.facts.filter(f => f.kind === 'FACT' && /^F\d+$/.test(f.id)).map(f => ({ id: f.id, found: __t.factDeclaration(f).map(d => ({ id: d.node.id, label: d.node.label, s: d.start, e: d.end })) })));
    const cabecalho = (f, d) => { const ls = fonte(f.path); return ls.slice(d.s - 1, f.declaration_line - 1).every(l => /^\s*(@|#|$)/.test(l)) && new RegExp(`\\b${f.symbol_name}\\b`).test(ls[f.declaration_line - 1]); };
    ok('9 M2 os 8 fatos vigentes reais do projeto Python acham, cada um, exatamente a declaração cujo cabeçalho contém a linha do fato', achadas.length === factsPy.filter(f => f.kind === 'FACT').length
      && achadas.every(a => { const f = factsPy.find(z => z.id === a.id); return f.status === 'current' && a.found.length === 1 && cabecalho(f, a.found[0]); }), JSON.stringify(achadas.map(a => [a.id, a.found.map(d => `${d.label}@${d.s}`)])));
    const decCfg = declaradas(grafoPy, CFG);
    const psy = decCfg.find(d => d.label === 'Settings._use_psycopg_driver'), emails = decCfg.find(d => d.label === 'Settings.emails_enabled');
    j = await junta('FD1');
    ok('9 M2 dois decoradores (@field_validator, @classmethod; def na 37): junta Settings._use_psycopg_driver (35–42)', psy && psy.s === 35 && psy.e === 42 && /^\s*@field_validator/.test(linhasCfg[34]) && /^\s*@classmethod/.test(linhasCfg[35])
      && /def _use_psycopg_driver\(/.test(linhasCfg[36]) && j.gathered.at(-1) === psy.id, `${j.note} ${JSON.stringify(psy)}`);
    j = await junta('FD2');
    ok('9 M2 dois decoradores, um com comentário (@computed_field  # …, @property; def na 63): junta Settings.emails_enabled (61–64)', emails && emails.s === 61 && /^\s*@computed_field\s+#/.test(linhasCfg[60]) && /^\s*@property/.test(linhasCfg[61])
      && j.gathered.at(-1) === emails.id && j.gathered.length === 3, `${j.note} ${JSON.stringify(emails)}`);
    j = await junta('FN1');
    ok('9 M2 sem correspondência (nome que o arquivo não declara): nada é juntado, e diz que não está no grafo', j.gathered.length === 3 && j.note === `A declaração nao_existe (${F7.path}:13) não está no grafo desta run: nada foi juntado.`, j.note);
    j = await junta('FN2');
    ok('9 M2 sem correspondência (linha fora de qualquer declaração de plan_route): nada é juntado', j.gathered.length === 3 && j.note === `A declaração plan_route (${F7.path}:30) não está no grafo desta run: nada foi juntado.`, j.note);
    j = await junta('FU1');
    ok('9 M2 fato que não cita a própria linha de declaração: nada é juntado, e diz por quê', j.gathered.length === 3 && j.note === 'O fato FU1 não cita a própria linha de declaração (13): ela não foi reconferida nesta run. Nada foi juntado.', j.note);
    ok('9 M2 Python: nenhuma escrita, sem erros de página', !x.writes.length && !x.errors.length, `${x.writes.join()} ${x.errors.join(' | ')}`);
    await fecha(x);
    // Ambiguidade: não há caso real (0 em 30 grafos, auditoria). Caso sintético declarado: uma segunda declaração de plan_route com
    // o mesmo intervalo, acrescentada à resposta de /api/graph nesta página (o servidor não muda).
    x = await pagina(browser, { base: P, project: PYTHON, key: CHAVE_PYTHON, antes: async y => { y.fake['GET /api/graph'] = async r => { const res = await r.fetch(), g = await res.json();
      const e = g.edges.find(z => z.to === decF7[0].id && z.kind === 'declares'), dup = `${decF7[0].id}:duplicada-pela-bateria`;
      g.nodes[dup] = { ...g.nodes[decF7[0].id], id: dup }; g.edges.push({ ...e, id: `${e.id}-duplicada`, to: dup }); return r.fulfill({ json: g }); }; } });
    await vai(x.page, 'facts');
    j = await junta('F7');
    ok('9 M2 ambíguo (sintético): duas declarações de plan_route com a linha 13 no cabeçalho; nada é juntado e a mensagem diz quais', !j.gathered.length
      && j.note.startsWith(`Ambíguo: 2 declarações de plan_route em ${F7.path} contêm a linha 13 no cabeçalho nesta run (plan_route (12–24); plan_route (12–24)); nada foi juntado`), j.note);
    await fecha(x);
    // Rust: os fatos vigentes reais acham a declaração que começa na linha deles; o F8 desatualizado não junta por aproximação.
    x = await pagina(browser);
    await trocaRun(x.page, RUN); await vai(x.page, 'facts');
    const achadasRs = await x.page.evaluate(() => __t.studio.facts.filter(f => f.kind === 'FACT').map(f => ({ id: f.id, status: f.status, line: f.declaration_line, found: __t.factDeclaration(f).map(d => d.start) })));
    ok('9 M2 Rust: cada fato vigente real acha exatamente a declaração que começa na linha dele', achadasRs.filter(a => a.status === 'current').length >= 8
      && achadasRs.filter(a => a.status === 'current').every(a => a.found.length === 1 && a.found[0] === a.line), JSON.stringify(achadasRs));
    const F8 = facts.find(f => f.id === 'F8'), dec86 = declaradas(grafo, F8?.path ?? '').find(d => d.label.endsWith('::intersect') && d.s <= (F8?.declaration_line ?? 0) && (F8?.declaration_line ?? 0) <= d.e);
    j = await junta('F8');
    ok('9 M2 Rust F8 desatualizado (a linha 88 é hoje corpo de intersect, 86–101): recusa com o motivo do servidor, nada é juntado (uma regra só por "contém a linha" juntaria por aproximação)', F8?.status === 'stale' && dec86?.s === 86
      && !j.gathered.length && j.note === `O fato F8 não está vigente nesta run (${F8.status_reason}): não dá para confirmar que a linha de declaração ainda é a dele. Nada foi juntado.`, j.note);
    await fecha(x);

    } catch (e) { await fechaBloco('9c', e); }
    // ---- 9d Juntar a declaração durante uma investigação e num conflito entre abas.
    try {
    x = await pagina(browser);
    await trocaRun(x.page, RUN);
    const inv = await get(B, `/api/inventory?key=${CHAVE_RENDERER}&run=${RUN}`), laco = inv.inventory.loops.find(l => l.through_calls?.length) ?? inv.inventory.loops[0];
    await x.page.evaluate(o => __t.investigate(o, document.createElement('button')), laco.occurrence);
    await x.page.waitForFunction(() => __t.investigation, null, { timeout: 60000 });
    await vai(x.page, 'facts');
    const vigente = facts.find(f => f.kind === 'FACT' && f.status === 'current' && f.run === RUN) ?? facts.find(f => f.kind === 'FACT' && f.status === 'current');
    const antesInv = await x.page.evaluate(() => __t.gathered.slice()), rascInv = await rascunhos(x.page);
    j = await junta(vigente.id);
    ok('9 Juntar durante uma investigação: recusa com o motivo, nada juntado, a investigação continua e nada é gravado', JSON.stringify(j.gathered) === JSON.stringify(antesInv) && j.note === await T(x.page, 'gather.duringInvestigation')
      && await x.page.evaluate(() => Boolean(__t.investigation)) && JSON.stringify(await rascunhos(x.page)) === JSON.stringify(rascInv) && !x.writes.length, j.note);
    await fecha(x);
    {
      const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
      const a = await pagina(browser, { context: ctx }); await trocaRun(a.page, RUN);
      const b2 = await pagina(browser, { context: ctx }); await trocaRun(b2.page, RUN);
      const no = await b2.page.evaluate(() => __t.state.nodes.find(n => n.category === 'symbol' && n.kind === 'method').id);
      await b2.page.evaluate(id => __t.toggleGathered(id, 'inspection'), no); await b2.page.waitForTimeout(600);
      ok('9 conflito entre abas: a primeira aba entra em "alterado em outra aba"', await a.page.evaluate(() => /outra aba/.test(document.querySelector('#headDraft').textContent)));
      const guardadoB = await a.page.evaluate(k => localStorage.getItem(k), DRAFT(CHAVE_RENDERER, RUN));
      await vai(a.page, 'facts');
      j = await junta(vigente.id, a.page);
      ok('9 Juntar num conflito entre abas: recusa com o motivo, nada juntado, o rascunho guardado continua o da outra aba', j.note === await T(a.page, 'draft.locked')
        && JSON.stringify(j.gathered) === '[]' && await a.page.evaluate(k => localStorage.getItem(k), DRAFT(CHAVE_RENDERER, RUN)) === guardadoB && !a.writes.length, j.note);
      await ctx.close();
    }

    } catch (e) { await fechaBloco('9d', e); }
    // ---- 9e T25: edição durante a abertura do projeto (depois da confirmação).
    try {
    {
      x = await pagina(browser);
      const h = segura();
      x.fake['POST /api/project'] = async (r, body) => { if (body?.root !== RAIZ_B) return false; await h.p; return false; };
      await edita(x.page);
      await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject'); await dialogo(x.page);
      await escolhe(x.page, 'discard');
      await x.page.waitForFunction(() => document.querySelector('#projectNote').textContent === 'Abrindo…', null, { timeout: 30000 });
      ok('9 T25 durante a abertura (depois de "Abrir sem salvar"): os campos do perfil ficam fechados a edições', (await campos(x.page)).every(Boolean), JSON.stringify(await campos(x.page)));
      const valor = await x.page.inputValue('#inputs');
      const digitou = await x.page.fill('#inputs', 'edição durante a abertura', { timeout: 1500 }).then(() => true, () => false);
      ok('9 … uma edição nesse intervalo não acontece (nada a perder em silêncio)', !digitou && await x.page.inputValue('#inputs') === valor);
      h.solta();
      await x.page.waitForFunction(k => __t.studio.project?.key === k && !document.querySelector('#openProject').disabled, CHAVE_B, { timeout: 120000 }); await pronta(x.page);
      ok('9 … depois de abrir, os campos voltam a aceitar edição, com o perfil do projeto aberto', (await campos(x.page)).every(v => !v) && await x.page.evaluate(() => __t.unsavedFields().length) === 0);
      await fecha(x);
      x = await pagina(browser);
      x.fake['POST /api/project'] = (r, body) => (body?.root === RAIZ_B ? r.fulfill({ status: 500, json: { error: 'abertura recusada (bateria)' } }) : false);
      await edita(x.page);
      await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject'); await dialogo(x.page);
      await escolhe(x.page, 'discard');
      await x.page.waitForFunction(() => !document.querySelector('#openProject').disabled, null, { timeout: 30000 });
      ok('9 … abertura recusada pelo servidor: o projeto não muda, a edição continua e os campos voltam a aceitar edição', await x.page.evaluate(() => __t.studio.project.key) === CHAVE_RENDERER
        && (await x.page.inputValue('#inputs')).endsWith('f9-edicao') && (await campos(x.page)).every(v => !v) && /abertura recusada \(bateria\)/.test(await nota(x.page, '#projectNote')), await nota(x.page, '#projectNote'));
      await fecha(x);
    }

    } catch (e) { await fechaBloco('9e', e); }
    // ---- 9f T25 cancelado: o campo do caminho volta ao projeto aberto.
    try {
    x = await pagina(browser);
    respondePut(x);
    await edita(x.page);
    await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject'); await dialogo(x.page); await cancela(x.page);
    ok('9 T25 Cancelar (Abrir): o campo do caminho volta ao do projeto aberto', await x.page.inputValue('#projectPath') === RENDERER && await x.page.evaluate(() => __t.studio.project.key) === CHAVE_RENDERER, await x.page.inputValue('#projectPath'));
    await x.page.fill('#projectPath', RAIZ_B); await x.page.press('#projectPath', 'Enter'); await dialogo(x.page); await cancela(x.page);
    ok('9 T25 Cancelar (Enter): idem', await x.page.inputValue('#projectPath') === RENDERER, await x.page.inputValue('#projectPath'));
    x.fake['GET /api/browse'] = r => r.fulfill({ json: { path: RAIZ_B, parent: null, marker: null, dirs: [] } });
    await x.page.click('#browse'); await x.page.waitForFunction(() => document.querySelector('#browseDialog').open && !document.querySelector('#browsePick').disabled);
    await x.page.click('#browsePick'); await dialogo(x.page); await cancela(x.page);
    ok('9 T25 Cancelar (Procurar › Escolher): idem', await x.page.inputValue('#projectPath') === RENDERER, await x.page.inputValue('#projectPath'));
    x.fake['PUT /api/project'] = r => r.fulfill({ status: 400, json: { error: 'recusado pela bateria' } });
    await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject'); await dialogo(x.page); await escolhe(x.page, 'save'); await dialogo(x.page); await confirma(x.page);
    await x.page.waitForFunction(() => !document.querySelector('#openProject').disabled, null, { timeout: 30000 });
    ok('9 T25 Salvar recusado: o projeto não muda e o campo volta ao caminho dele', await x.page.inputValue('#projectPath') === RENDERER && await x.page.evaluate(() => __t.studio.project.key) === CHAVE_RENDERER, await x.page.inputValue('#projectPath'));
    await fecha(x);

    } catch (e) { await fechaBloco('9f', e); }
    // ---- 9g Reabrir o mesmo projeto: nada é descartado, então nada é perguntado.
    try {
    x = await pagina(browser);
    await trocaRun(x.page, RUN);
    await x.page.click(`#turns .turn[data-turn="${ordem[0].id}"] .bubble.user`); await x.page.waitForTimeout(300);
    const fonteG = await x.page.evaluate(() => __t.state.nodes.find(n => n.category === 'symbol' && n.kind === 'method').id);
    await x.page.evaluate(id => __t.toggleGathered(id, 'inspection'), fonteG); await x.page.waitForTimeout(300);
    await edita(x.page);
    const posts = x.reqs.filter(q => q.chave === 'POST /api/project').length;
    const reabre = async caminho => { await x.page.fill('#projectPath', caminho); await x.page.click('#openProject'); await x.page.waitForTimeout(300);
      const p = await aberto(x.page); await x.page.waitForFunction(() => !document.querySelector('#openProject').disabled, null, { timeout: 60000 }); await x.page.waitForTimeout(300); return p; };
    let perguntouG = await reabre(RENDERER);
    ok('9 reabrir o mesmo projeto com alterações não salvas: sem T25 (o pedido de abertura é feito)', !perguntouG && x.reqs.filter(q => q.chave === 'POST /api/project').length === posts + 1);
    ok('9 … a edição continua (não salva), com a run, a pergunta ativa e o rascunho', (await x.page.inputValue('#inputs')).endsWith('f9-edicao') && await x.page.evaluate(() => __t.unsavedFields().join()) === 'inputs'
      && await x.page.evaluate(() => __t.studio.run) === RUN && await x.page.evaluate(() => __t.studio.activeTurn) === ordem[0].id && JSON.stringify(await x.page.evaluate(() => __t.gathered)) === JSON.stringify([fonteG]));
    ok('9 … e diz que nada foi descartado', /já estava aberto/.test(await nota(x.page, '#projectNote')) && /nada da página foi descartado/.test(await nota(x.page, '#projectNote')), await nota(x.page, '#projectNote'));
    perguntouG = await reabre(RENDERER.replace(/\//g, '\\') + '\\');
    ok('9 o mesmo caminho escrito com barras invertidas e barra final: o mesmo projeto, sem T25 e sem perda', !perguntouG && (await x.page.inputValue('#inputs')).endsWith('f9-edicao') && await x.page.evaluate(() => __t.studio.run) === RUN);
    // A run aberta sumiu do registro: a troca de run é obrigatória, mas nada se perde (a edição fica, o rascunho da run fica guardado).
    const rascG = await x.page.evaluate(k => localStorage.getItem(k), DRAFT(CHAVE_RENDERER, RUN));
    x.fake['POST /api/project'] = async (r, body) => { if (body?.root !== RENDERER) return false; const res = await r.fetch(), data = await res.json(); return r.fulfill({ json: { ...data, runs: data.runs.filter(z => z.id !== RUN) } }); };
    perguntouG = await reabre(RENDERER); await pronta(x.page);
    ok('9 reabrir com a run aberta fora do registro: sem T25; abre a mais recente, a edição continua e o rascunho da run fica guardado', !perguntouG && await x.page.evaluate(() => __t.studio.run) === RECENTE
      && (await x.page.inputValue('#inputs')).endsWith('f9-edicao') && await x.page.evaluate(k => localStorage.getItem(k), DRAFT(CHAVE_RENDERER, RUN)) === rascG);
    ok('9 reabrir: nenhum PUT, nenhuma escrita, sem erros de página', !x.reqs.some(q => q.chave === 'PUT /api/project') && !x.writes.length && !x.errors.length, `${x.writes.join()} ${x.errors.join(' | ')}`);
    await fecha(x);

    } catch (e) { await fechaBloco('9g', e); }
    // ---- 9h Linha de base em vigor quando a versão do comando muda (critério da escada, 2.5).
    try {
    {
      const comBase = commands.commands.find(c => execs.some(z => z.name === c.name && !z.change && !z.instrumentation && z.summary.succeeded > 0));
      const NOVA = 'f'.repeat(64);
      x = await pagina(browser, { antes: async y => { y.fake['GET /api/commands'] = async r => { const res = await r.fetch(), data = await res.json();
        data.commands = data.commands.map(c => (c.name === comBase.name ? { ...c, sha256: NOVA, approved: null } : c)); return r.fulfill({ json: data }); }; } });
      await trocaRun(x.page, RUN); await vai(x.page, 'ruler');
      const li = await x.page.evaluate(n => { const e = document.querySelector(`#commandsList li[data-command="${CSS.escape(n)}"]`); const b = e.querySelector('[data-baseline]');
        return { text: e.textContent, base: b.dataset.baseline, versao: b.dataset.baselineVersion, run: e.querySelector('button.primary').disabled }; }, comBase.name);
      const esperada = baseOf(comBase);
      ok('9 versão do comando trocada: a linha de base em vigor continua a última sem mudança, de outra versão (a escada a aceita)', li.base === esperada.id && li.versao === 'other'
        && li.text.includes(`de outra versão do comando (${esperada.command_sha256.slice(0, 12)})`), JSON.stringify(li));
      ok('9 … a versão nova é dita "não aprovado nesta versão" (não afirma que nunca houve aprovação) e Rodar fica desabilitado', li.text.includes(`versão ${NOVA.slice(0, 12)}`) && li.text.includes('não aprovado nesta versão') && li.run);
      const outros = await x.page.evaluate(() => [...document.querySelectorAll('#commandsList [data-baseline]')].map(b => b.dataset.baselineVersion));
      ok('9 … os comandos de versão inalterada com linha de base dizem que ela é desta versão', outros.filter(v => v === 'this').length === commands.commands.filter(c => c.name !== comBase.name && baseOf(c) && baseOf(c).command_sha256 === c.sha256).length, JSON.stringify(outros));
      await fecha(x);
    }

    } catch (e) { await fechaBloco('9h', e); }
    // ---- 9i Destinos do resumo: Run à Régua (sem alterações); Contagens manuais ao Inventário em todos os estados.
    try {
    x = await pagina(browser);
    await trocaRun(x.page, RUN); await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('9 Run sem alterações tem link', !r.run.error && r.run.link, JSON.stringify(r.run));
    await x.page.click('#projectSummary [data-summary="run"] button');
    ok('9 Run leva à Régua, com o ponto no Voltar', await x.page.evaluate(() => __t.navigation.area === 'ruler' && !document.querySelector('#navBack').disabled));
    await vai(x.page, 'project');
    await x.page.click('#projectSummary [data-summary="manual"] button');
    ok('9 Contagens manuais leva a Explorar › Inventário (onde as contagens aparecem), com o ponto no Voltar', await x.page.evaluate(() => __t.navigation.area === 'explore' && __t.navigation.explore === 'inventory' && !document.querySelector('#navBack').disabled));
    await fecha(x);
    for (const [caso, mc, texto] of [['nenhuma', [], 'Nenhuma'], ['ilegíveis', null, 'Falha de leitura (não quer dizer "nenhuma")']]) {
      x = await pagina(browser, { antes: async y => { y.fake['GET /api/commands'] = async r => { const res = await r.fetch(), data = await res.json(); return r.fulfill({ json: { ...data, manual_counts: mc } }); }; } });
      await vai(x.page, 'project');
      r = await resumo(x.page);
      await x.page.click('#projectSummary [data-summary="manual"] button');
      ok(`9 Contagens manuais ${caso}: "${texto}", e o item leva ao Inventário`, r.manual.text === texto && r.manual.link && await x.page.evaluate(() => __t.navigation.explore) === 'inventory', JSON.stringify(r.manual));
      await fecha(x);
    }

    } catch (e) { await fechaBloco('9i', e); }
    // ---- 9j Critérios guardados no rascunho × alteração do perfil não salva.
    try {
    x = await pagina(browser);
    respondePut(x);
    const criterio = 'Critério do fechamento do F9: só esta pergunta';
    await x.page.evaluate(v => { const c = document.querySelector('#criteria'); c.value = v; c.dispatchEvent(new Event('input')); }, criterio); await x.page.waitForTimeout(300);
    ok('9 critérios próprios no rascunho: não são "Alterações não salvas" do perfil', await x.page.evaluate(() => document.querySelector('#headUnsaved').hidden) && await x.page.evaluate(() => __t.unsavedFields().length) === 0);
    ok('9 … o cabeçalho os diz na linha do rascunho: guardados neste navegador, não salvos como padrão', await nota(x.page, '#headDraft') === 'Rascunho: 0 fonte(s) · guardado neste navegador · critérios próprios, não salvos como padrão', await nota(x.page, '#headDraft'));
    ok('9 … o compositor diz que não foram salvos como padrão e ficam no rascunho', (await nota(x.page, '#criteriaCount')).endsWith('· Diferem do padrão do projeto (não salvos como padrão; ficam no rascunho)'), await nota(x.page, '#criteriaCount'));
    ok('9 … sair não pede aviso (estão guardados com o rascunho)', !(await pedeAviso(x.page)));
    await x.page.click('#headDraft'); await x.page.waitForTimeout(200);
    ok('9 … o cabeçalho leva aos critérios, no compositor', await x.page.evaluate(() => document.activeElement?.id) === 'criteria');
    await abre(x.page, RAIZ_B, CHAVE_B);
    ok('9 … trocar de projeto com eles não pergunta nada (sem T25)', !(await aberto(x.page)) && await x.page.evaluate(() => __t.studio.project.key) === CHAVE_B);
    await abre(x.page, RENDERER, CHAVE_RENDERER); await pronta(x.page);
    ok('9 … de volta ao projeto, os critérios voltam com o rascunho da run', await x.page.evaluate(() => __t.draftCriteria) === criterio);
    await edita(x.page);
    ok('9 com uma edição do perfil também: "Alterações não salvas" diz só a do perfil', await nota(x.page, '#headUnsaved') === 'Alterações não salvas: Entradas (Projeto)', await nota(x.page, '#headUnsaved'));
    await x.page.fill('#projectPath', RAIZ_B); await x.page.click('#openProject'); await dialogo(x.page);
    ok('9 … o T25 lista só a edição do perfil, não os critérios do rascunho', (await mensagem(x.page)).includes('Entradas') && !/crit[ée]rio/i.test(await mensagem(x.page)), await mensagem(x.page));
    await cancela(x.page);
    ok('9 critérios do rascunho: nenhum PUT', !x.puts);
    await fecha(x);

    } catch (e) { await fechaBloco('9j', e); }
    // ---- 9k Gerar guarda a run nova; abrir e recarregar não gravam escolha nenhuma.
    try {
    x = await pagina(browser);
    ok('9 abrir sem escolha: nada gravado na navegação', await navGuardada(x.page) === null && await x.page.evaluate(() => __t.studio.run) === RECENTE);
    await recarrega(x.page);
    ok('9 recarregar sem escolha: nada gravado', await navGuardada(x.page) === null);
    // /api/generate não roda na bateria: a resposta simulada aponta para uma run existente como se fosse a nova.
    x.fake['POST /api/generate'] = r => r.fulfill({ json: { id: RUN } });
    await vai(x.page, 'project'); await x.page.click('#generate');
    await x.page.waitForFunction(r => __t.studio.run === r && __t.studio.turnsState === 'ready', RUN, { timeout: 120000 }); await pronta(x.page);
    ok('9 Gerar: a run nova vira a ativa e fica guardada como escolha (nav.run)', JSON.parse(await navGuardada(x.page) ?? '{}').run === RUN && x.reqs.some(q => q.chave === 'POST /api/generate'));
    const navG = await navGuardada(x.page);
    await recarrega(x.page);
    ok('9 … recarregar volta a ela, sem regravar a navegação', await x.page.evaluate(() => __t.studio.run) === RUN && await navGuardada(x.page) === navG);
    ok('9 Gerar: nenhum PUT (não aciona Salvar)', !x.reqs.some(q => q.chave === 'PUT /api/project'));
    await fecha(x);

    } catch (e) { await fechaBloco('9k', e); }
    // ---- 9l Itens vazios.
    try {
    x = await pagina(browser, { antes: async y => {
      y.fake['GET /api/turns'] = r => r.fulfill({ json: [] });
      y.fake['GET /api/facts'] = async r => { const res = await r.fetch(), data = await res.json(); return r.fulfill({ json: { ...data, facts: [] } }); };
      y.fake['GET /api/commands'] = async r => { const res = await r.fetch(), data = await res.json(); return r.fulfill({ json: { ...data, commands: [], manual_counts: [] } }); };
      y.fake['POST /api/project'] = async (r, body) => { if (body?.root !== RENDERER) return false; const res = await r.fetch(), data = await res.json(); return r.fulfill({ json: { ...data, settings: { ...data.settings, lexicon: { groups: [], stop_words: [] } } } }); }; } });
    await vai(x.page, 'project');
    r = await resumo(x.page);
    ok('9 vazios no resumo: Perguntas, Fatos, Comandos, Contagens e Léxico', r.questions.text === 'Nenhuma pergunta nesta run' && r.facts.text === 'Nenhum fato aprovado neste projeto' && r.commands.text === 'Nenhum comando no perfil'
      && r.manual.text === 'Nenhuma' && r.lexicon.text === 'Vazio (→ Léxico)', JSON.stringify(r));
    ok('9 vazios nos lugares: a conversa, o Acervo (2.6) e a Régua', await nota(x.page, '#turns') === 'Nenhuma pergunta nesta run ainda.' && (await nota(x.page, '#factsList')).startsWith('Nenhum fato aprovado neste projeto.')
      && (await nota(x.page, '#commandsList')).startsWith('O librarian.json do projeto não declara commands.'), `${await nota(x.page, '#turns')} | ${await nota(x.page, '#factsList')} | ${await nota(x.page, '#commandsList')}`);
    await abre(x.page, RAIZ_B, CHAVE_B); await pronta(x.page);
    ok('9 projeto sem runs: a nota da run diz "Sem run: gere a primeira" (2.1)', await nota(x.page, '#runNote') === 'Sem run: gere a primeira', await nota(x.page, '#runNote'));
    ok('9 vazios: sem erros de página nem escritas', !x.errors.length && !x.writes.length, `${x.errors.join(' | ')} ${x.writes.join()}`);
    await fecha(x);
    } catch (e) { await fechaBloco('9l', e); }
  } catch (e) {
    ok('a bateria terminou sem exceção', false, e.stack);
  }
  ok('o perfil do renderer está igual em bytes ao do início', sha(fs.readFileSync(perfil)) === perfilAntes);
  await browser.close();
})();

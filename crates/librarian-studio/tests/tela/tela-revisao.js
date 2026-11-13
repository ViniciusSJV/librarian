// F10: a Revisão, o lote e "Aprovar todos" (especificação 4.1 a 4.5, 7.3, T7, T10 a T12, T14, T17, T19; critérios
// 9.6 e 9.7), na página real, num Chrome sem janela, contra o servidor de rodar.js (8841).
// Três operações diferentes, testadas em separado:
//   A. Revisão individual (4.1 a 4.3): Preparar não envia; "Ver revisão" mostra a pergunta preparada; só "Enviar",
//      explícito, envia; os avisos da especificação (H1 a H3 recusadas, fato revogado, "Não conferido nesta revisão").
//   B. Lote (4.5): "Enviar as marcadas" envia cada pergunta marcada sozinha, uma por vez; cada uma guarda o próprio
//      contexto e resultado; uma falha ou resposta atrasada não contamina outra; as desmarcadas não são enviadas.
//      Nada é aprovado no lote.
//   C. "Aprovar todos" (depois das respostas): a prévia separa os FACTs aprováveis dos não aprováveis, com o motivo;
//      só a confirmação explícita aprova; o resultado sai por item; nenhuma regra da aprovação uma a uma é contornada.
//      Nada é enviado por ele.
// Também: guardas de run e projeto, respostas atrasadas, persistência (T17), estados carregando, vazio e falha, PT e
// EN, temas, tela estreita e teclado. Nada é enviado a modelo e nada é gravado no servidor: preparar, enviar e
// decidir são respondidos aqui (alguns segurados); qualquer outra escrita é abortada e contada.
// Casos simulados (os dados reais não os têm, e cada um está declarado onde é usado): H1 a H3 recusadas, fato
// revogado num pacote, perguntas criadas pelo lote (cópias de perguntas reais com outro id), envios e respostas.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium, RENDERER, CHAVE_RENDERER } = require('./ambiente');
const B = 'http://127.0.0.1:8841', RUN = '20261104-020244', OUTRA = '20261104-014710';
const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS;
const HOOK = "window.__t = { state, studio, t, STRINGS, loadRun, activate, renderTurns, selectNode, setLang, localState, askFacts, askHotFacts, approveHotFacts,"
  // Os nomes do F10 lidos com typeof: a mesma bateria roda contra a página anterior (contraprova) sem quebrar o gancho.
  + " openReview: typeof openReview === 'function' ? openReview : () => {}, closeReview: typeof closeReview === 'function' ? closeReview : () => {}, renderHot,"
  + " get review() { return typeof review === 'undefined' ? null : review; }, get hotBatch() { return typeof hotBatch === 'undefined' ? null : hotBatch; }, get navigation() { return navigation; },"
  + " get hot() { return hot; }, get gathered() { return gathered; }, get draft() { return draft; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 300) : ''}`); if (!pass) process.exitCode = 1; };
const perfil = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'project.json');
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const NAV = `librarian.nav.v1:${CHAVE_RENDERER}`;
const get = async p => (await fetch(B + p)).json();
const segura = () => { let solta; const p = new Promise(r => { solta = r; }); return { p, solta }; };
const LEITURA = (method, pathname) => method === 'GET'
  || (method === 'POST' && ['/api/project', '/api/calls', '/api/envelope/suggest', '/api/inventory/investigate', '/api/performance-question'].includes(pathname));
const contraste = cores => {
  const L = k => { const m = k.match(/[\d.]+/g).slice(0, 3).map(Number).map(v => v / 255).map(v => v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4); return 0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]; };
  const [hi, lo] = [L(cores[0]), L(cores[1])].sort((p, q) => q - p);
  return (hi + 0.05) / (lo + 0.05);
};
// 7.1 e 9.6.4: frases que nenhuma tela pode dizer (afirmações; "não é garantida", exigida pela 4.1, não é uma delas).
const PROIBIDAS = [/o pacote revisado é o enviado/i, /the reviewed package is the one sent/i, /bloquead[oa] pelo servidor/i, /blocked by the server/i,
  /\bvalidad[oa]\b/i, /\bvalidated\b/i, /revisão garantida/i, /(?<!not )guaranteed review/i, /(?<!não é )garantid[oa] (pela|por esta) revisão/i,
  /(?<!is not )guaranteed by this review/i, /\bvencedora\b/i, /\bwinner\b/i];

async function pagina(browser, { size = { width: 1440, height: 900 }, scheme = 'light', locale = 'pt-BR', storage = null, run = RUN, antes = null } = {}) {
  const context = await browser.newContext({ viewport: size, colorScheme: scheme, locale });
  if (storage) await context.addInitScript(s => { if (!sessionStorage.getItem('f10.semeado')) { for (const [k, v] of Object.entries(s)) localStorage.setItem(k, v); sessionStorage.setItem('f10.semeado', '1'); } }, storage);
  const page = await context.newPage();
  const x = { page, context, errors: [], writes: [], reqs: [], fake: {} };
  page.on('pageerror', e => x.errors.push(e.message));
  await page.route(u => new URL(u).pathname === '/', async r => { const y = await r.fetch(); r.fulfill({ response: y, body: (await y.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.route(u => new URL(u).pathname.startsWith('/api/'), async r => {
    const req = r.request(), url = new URL(req.url()), { pathname } = url, method = req.method(), chave = `${method} ${pathname}`;
    const raw = req.postData(), body = raw ? JSON.parse(raw) : null;
    x.reqs.push({ chave, url: req.url(), raw, body, run: url.searchParams.get('run'), turn: url.searchParams.get('turn') ?? body?.turn ?? null, at: Date.now() });
    const f = x.fake[chave];
    if (f) { const feito = await f(r, body, url); if (feito !== false) return; }
    if (LEITURA(method, pathname)) return r.continue();
    x.writes.push(chave);
    return r.abort();
  });
  if (antes) await antes(x);
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  if (await page.evaluate(() => __t.studio.project?.key) !== CHAVE_RENDERER) await abre(page, RENDERER, CHAVE_RENDERER);
  await page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
  if (run && await page.evaluate(() => __t.studio.run) !== run) await trocaRun(page, run);
  await pronta(page);
  return x;
}
const pronta = page => page.waitForFunction(() => __t.studio.turnsState !== 'loading' && (__t.studio.facts || __t.studio.factsError) && __t.hot?.rows, null, { timeout: 120000 }).then(() => page.waitForTimeout(400));
async function abre(page, root, key) {
  await page.evaluate(r => { document.querySelector('#projectPath').value = r; document.querySelector('#openProject').click(); }, root);
  await page.waitForFunction(k => __t.studio.project?.key === k && !document.querySelector('#openProject').disabled, key, { timeout: 120000 });
  await page.waitForTimeout(600);
}
async function trocaRun(page, run, { espera = true } = {}) {
  await page.selectOption('#runs', run);
  if (!espera) return;
  await page.waitForFunction(r => __t.studio.run === r && __t.state.nodes.length && __t.state.graph && __t.studio.turnsState !== 'loading', run, { timeout: 120000 });
  await pronta(page);
}
const fecha = x => x.context.close();
const dialogo = page => page.waitForFunction(() => document.querySelector('#askDialog').open, null, { timeout: 30000 });
const fechado = page => page.waitForFunction(() => !document.querySelector('#askDialog').open, null, { timeout: 30000 });
const T = (page, k, v = {}) => page.evaluate(([k, v]) => __t.t(k, v), [k, v]);
const conta = (x, chave, turn) => x.reqs.filter(r => r.chave === chave && (turn == null || String(r.turn) === String(turn))).length;
// A Revisão como a tela a mostra.
const rv = page => page.evaluate(() => {
  const shown = e => Boolean(e && e.offsetParent !== null && e.getClientRects().length);
  const q = s => document.querySelector(s);
  const blocks = Object.fromEntries([...document.querySelectorAll('#reviewBody [data-block]')].map(b => [b.dataset.block, b.innerText]));
  return { view: __t.navigation.center, shown: shown(q('#reviewView')), tab: shown(q('#centerReviewTab')), tabText: q('#centerReview').textContent,
    current: q('#centerReview').getAttribute('aria-current'), title: q('#reviewTitle').textContent, note: q('#reviewNote').textContent, blocks,
    text: q('#reviewBody').innerText, query: q('#reviewBody .rv-query')?.textContent ?? null,
    send: q('#reviewSend') && { text: q('#reviewSend').textContent, disabled: q('#reviewSend').disabled },
    lotSend: q('#lotSend') && { text: q('#lotSend').textContent, disabled: q('#lotSend').disabled },
    why: q('#reviewWhy')?.textContent ?? '', revoked: q('#reviewRevoked')?.innerText ?? '', sent: q('#reviewSent')?.innerText ?? '',
    review: __t.review && { kind: __t.review.kind, id: __t.review.id ?? null, n: __t.review.items?.length ?? null } };
});
const itens = page => page.evaluate(() => [...document.querySelectorAll('#reviewBody .rv-item')].map(d => ({ item: d.dataset.item, turn: d.dataset.turn, marked: d.dataset.marked,
  result: d.dataset.result, box: { checked: d.querySelector('input').checked, disabled: d.querySelector('input').disabled }, text: d.innerText,
  query: d.querySelector('details pre')?.textContent ?? null })));
const esperaRevisao = (page, pred = '1') => page.waitForFunction(new Function(`return Boolean(window.__t && __t.review) && (${pred});`), null, { timeout: 60000 });
// Uma pergunta criada aqui: cópia de uma real, com outro id e sem envios.
const falsa = (base, id, extra = {}) => ({ ...structuredClone(base), id: String(id), sends: [], comparison: null, ...extra });
// Um envio respondido aqui: cópia de um envio real, com outro id, sem decisões; o vínculo aponta o hash dado.
const envioFalso = (base, id, evidence) => { const s = structuredClone(base); s.id = id; s.reviews = {}; if (s.link) s.link.evidence_sha256 = evidence; return s; };

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const perfilAntes = sha(fs.readFileSync(perfil));
  const turns = await get(`/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`);
  const T135 = turns.find(t => t.id === '135'), T139 = turns.find(t => t.id === '139'), T01 = turns.find(t => t.id === '01'), LADDER = turns.find(t => t.gathered?.origin?.kind === 'ladder');
  const ev135 = await get(`/api/evidence?key=${CHAVE_RENDERER}&run=${RUN}&turn=135`), ev139 = await get(`/api/evidence?key=${CHAVE_RENDERER}&run=${RUN}&turn=139`);
  const facts = (await get(`/api/facts?key=${CHAVE_RENDERER}&run=${RUN}`)).facts;
  const ENVIO = T01.sends[0];
  const MARCA = ev135.mark.sha256;
  ok('0 dados: 135 e 139 são perguntas do Builder preparadas e não enviadas, com H1 a H3 aprovadas e pacote de formato 2',
    [T135, T139].every(t => t && t.gathered?.origin?.kind === 'selection' && !(t.sends ?? []).length && t.query) && ['h1', 'h2', 'h3'].every(h => ev135.checks[h].status === 'passed') && ev135.package.format_version === 2);
  const todosErros = [], todasEscritas = [];
  const junta = x => { todosErros.push(...x.errors); todasEscritas.push(...x.writes); };
  let x, r;
  // Cada parte num bloco: uma exceção vira a FALHA do bloco, a página é fechada e o bloco seguinte continua.
  let nBloco = 0;
  const bloco = async fn => { const k = ++nBloco; x = null; try { await fn(); } catch (e) { ok(`bloco ${k} terminou sem exceção`, false, String(e.message).split(String.fromCharCode(10))[0]); } finally { if (x) { junta(x); await fecha(x).catch(() => {}); } } };

  // ================================================================ A1. Preparar não envia; a Revisão abre (T10, 4.1)
  await bloco(async () => {
  x = await pagina(browser);
  {
    const page = x.page;
    let criadas = 0;
    x.fake['POST /api/gathered-question'] = rr => { criadas++; return rr.fulfill({ json: falsa(T135, 900100 + criadas, { question: JSON.parse(rr.request().postData()).question }) }); };
    x.fake['GET /api/evidence'] = (rr, _, url) => (url.searchParams.get('turn').startsWith('9001') ? rr.fulfill({ json: { ...structuredClone(ev135), turn: url.searchParams.get('turn') } }) : false);
    // Junta uma fonte pela inspeção (ela vira o alvo) e escolhe um LLM de verdade no compositor: antes do F10, Preparar
    // também enviava a esse LLM.
    await page.click('#areas [data-area="explore"]');
    const alvo = await page.evaluate(() => { const r = __t.hot.rows.find(x => x.name === 'impl Matrix<4>::inverse'); __t.selectNode(__t.state.index.get(r.symbol)); return r.symbol; });
    await page.waitForTimeout(200);
    await page.click(`#inspector button:has-text("${await T(page, 'gather.add')}")`);
    await page.waitForFunction(() => __t.gathered.length === 1);
    const provedor = await page.evaluate(() => { const p = document.querySelector('#provider'); const v = [...p.options].map(o => o.value).find(v => v === 'claude-cli') ?? [...p.options].map(o => o.value).find(v => v !== 'manual'); p.value = v; p.dispatchEvent(new Event('change')); return v; });
    await page.fill('#question', 'F10 bateria: o que este método faz?');
    const botao = await page.evaluate(() => ({ text: document.querySelector('#ask').textContent, route: document.querySelector('#route').textContent }));
    ok('A1 3.11 rota 4: o botão diz "Preparar", não "Perguntar com …"', botao.text === await T(page, 'composer.prepareBuilder', { n: 1 }), botao.text);
    ok('A1 3.11 coluna "Revisão na v1": o caminho previsto diz que Preparar abre a Revisão e que o envio é à parte', botao.route.includes(await T(page, 'route.review')), botao.route);
    await page.click('#ask');
    await esperaRevisao(page, "__t.review.kind === 'turn' && __t.review.evidence");
    r = await rv(page);
    ok('A1 T10 Preparar cria a pergunta (um POST /api/gathered-question)', criadas === 1);
    ok('A1 Preparar NÃO envia: nenhum POST /api/send', conta(x, 'POST /api/send') === 0, conta(x, 'POST /api/send'));
    ok('A1 a Revisão abre no centro, na frente, com a aba "Revisão #N" atual', r.view === 'review' && r.shown && r.tab && r.current === 'true' && r.tabText.startsWith(await T(page, 'review.tabTurn', { id: '900101' })), JSON.stringify([r.view, r.shown, r.tab, r.tabText]));
    ok('A1 título: "Revisão da pergunta #900101"', r.title === await T(page, 'review.titleTurn', { id: '900101' }), r.title);
    const turno = await page.evaluate(() => __t.studio.turns.find(t => t.id === '900101'));
    ok('A1 9.6.1 "A pergunta preparada" é igual ao turn.query, byte a byte', r.query === turno.query && r.query === T135.query);
    ok('A1 a pergunta entra na linha do tempo como "Preparada, não enviada"', await page.evaluate(() => [...document.querySelectorAll('#turns .turn[data-turn="900101"] .state-tag')].map(s => s.dataset.state).join()) === 'prepared');
    ok('A1 3.10 o texto do compositor continua depois de preparar', await page.evaluate(() => document.querySelector('#question').value) === 'F10 bateria: o que este método faz?');
    ok('A1 o rascunho fica "já preparado como #900101"', await page.evaluate(() => __t.draft.prepared_as?.turn) === '900101');
    // Os sete blocos da 4.1, com os rótulos obrigatórios.
    const rotulos = { package: 'review.label.package', checks: 'review.label.checks', chosen: 'review.label.chosen', only: 'review.label.only' };
    for (const [b, k] of Object.entries(rotulos)) ok(`A1 4.1 bloco "${b}" com o rótulo obrigatório`, (r.blocks[b] ?? '').includes(await T(page, k)), (r.blocks[b] ?? '').slice(0, 120));
    ok('A1 4.1 "Política e critérios usados": gravados com a pergunta, podem diferir do perfil atual', (r.blocks.used ?? '').includes(await T(page, 'used.label')));
    ok('A1 4.1 "Pacote gravado": marca, formato e fichas com declared_by', /declared_by/.test(r.blocks.package) && r.blocks.package.includes(String(ev135.package.fichas.length)));
    ok('A1 4.1 "Conferências nesta consulta": H1, H2 e H3 aprovadas e H4 informativo', ['h1', 'h2', 'h3'].every(h => r.blocks.checks.includes(`${'H' + h[1]}`)) && r.blocks.checks.includes('H4'));
    ok('A1 4.1 "Escolhido para o envio": o provedor do compositor', r.blocks.chosen.includes(await page.evaluate(v => __t.t('provider.short.' + v) !== 'provider.short.' + v ? __t.t('provider.short.' + v) : ({ ollama: 'Ollama', 'claude-cli': 'Claude CLI' })[v], provedor)), r.blocks.chosen.slice(0, 160));
    ok('A1 4.1 "Só você vê": lacunas e o que ficou de fora vêm do registro da seleção, "não vai ao modelo"', r.blocks.only.includes(await T(page, 'review.label.only')));
    ok('A1 9.6.3 / 7.3 "Não conferido nesta revisão" sempre presente, com o texto da especificação', (r.blocks.unchecked ?? '').includes(await T(page, 'review.unchecked'))
      && (await T(page, 'review.unchecked')) === 'Não conferido nesta revisão: identidade com o envio (D3), SYSTEM e skills (D4), recusa por fato revogado (D7)');
    ok('A1 Enviar habilitado (H1 a H3 aprovadas) e diz a quem envia', r.send && !r.send.disabled && r.send.text.startsWith('Enviar · '), JSON.stringify(r.send));
    ok('A1 9.6.4 nenhuma frase proibida na Revisão (PT)', !PROIBIDAS.some(p => p.test(r.text)), PROIBIDAS.find(p => p.test(r.text)));
    ok('A1 a Revisão não decide: nenhum botão Aprovar ou Recusar nela', !(await page.evaluate(([a, b]) => [...document.querySelectorAll('#reviewBody button')].some(x => [a, b].includes(x.textContent)), [await T(page, 'review.approve'), await T(page, 'review.reject')])));

    // A2. Enviar só com a ação explícita; o hash do vínculo ao lado do visto (4.2, T12).
    const envio = segura();
    x.fake['POST /api/send'] = async (rr, body) => { await envio.p; return rr.fulfill({ json: envioFalso(ENVIO, `${body.turn}-teste`, MARCA) }); };
    await page.waitForTimeout(500);
    ok('A2 enquanto ninguém clica Enviar, nada é enviado', conta(x, 'POST /api/send') === 0);
    await page.click('#reviewSend');
    await page.waitForFunction(() => document.querySelector('#reviewSend')?.disabled, null, { timeout: 10000 });
    r = await rv(page);
    ok('A2 durante o envio: Enviar desabilitado, "Enviando…"', r.send.disabled && r.why === await T(page, 'review.whySending'), r.why);
    envio.solta();
    await page.waitForFunction(() => document.querySelector('#reviewSent'), null, { timeout: 30000 });
    r = await rv(page);
    const corpoEnvio = x.reqs.filter(q => q.chave === 'POST /api/send').at(-1).body;
    ok('A2 um único POST /api/send, da pergunta #900101, na run dela, com o LLM do compositor', conta(x, 'POST /api/send') === 1 && corpoEnvio.turn === '900101' && corpoEnvio.run === RUN && corpoEnvio.provider === provedor, JSON.stringify(corpoEnvio));
    ok('A2 o corpo do envio tem as chaves de sempre', JSON.stringify(Object.keys(corpoEnvio)) === '["key","run","turn","provider","modelfile","claude_model","skills"]', JSON.stringify(Object.keys(corpoEnvio)));
    ok('A2 4.2 depois do envio: evidence_sha256 do vínculo ao lado do visto, "iguais"', r.sent.includes(MARCA.slice(0, 12)) && r.sent.includes(await T(page, 'review.hashSame')), r.sent);
    ok('A2 4.2 rótulo "comparação feita pela página, depois do envio"', r.sent.includes(await T(page, 'review.hashLabel')));
    ok('A2 as decisões ficam na conversa (camada 3), não na Revisão', r.sent.includes(await T(page, 'review.decideThere')));
    ok('A2 a resposta aparece na própria pergunta, na conversa', await page.evaluate(() => Boolean(document.querySelector('#turns .turn[data-turn="900101"] .layer-decide'))));
    // Hash diferente: a página diz o que viu, sem dizer que o servidor bloqueou.
    x.fake['POST /api/send'] = (rr, body) => rr.fulfill({ json: envioFalso(ENVIO, `${body.turn}-outro`, 'f'.repeat(64)) });
    await page.click('#reviewSend');
    await page.waitForFunction(t => document.querySelector('#reviewSent')?.innerText.includes(t), await T(page, 'review.hashDiffers'), { timeout: 30000 });
    ok('A2 vínculo com outro hash: "diferentes", e o envio aparece como de novo ("Enviar de novo")', (await rv(page)).send.text.startsWith('Enviar de novo'));
    // Falha do envio: na Revisão e na própria pergunta; a pergunta continua preparada.
    x.fake['POST /api/send'] = rr => rr.fulfill({ status: 502, json: { error: 'provedor indisponível (teste)' } });
    await page.click('#reviewSend');
    await page.waitForFunction(() => /falhou/.test(document.querySelector('#reviewSent')?.innerText ?? ''), null, { timeout: 30000 });
    r = await rv(page);
    ok('A2 falha do envio: a mensagem do servidor como veio, na Revisão', r.sent === await T(page, 'review.sendFailed', { error: 'provedor indisponível (teste)' }), r.sent);
    ok('A2 falha do envio: a mesma mensagem na própria pergunta', (await page.evaluate(() => document.querySelector('#turns .turn[data-turn="900101"]').innerText)).includes('provedor indisponível (teste)'));

    // A3. Voltar ao rascunho (T19) e Abrir na conversa.
    await page.click('#reviewBody [data-action="draft"]');
    ok('A3 T19 "Voltar ao rascunho" abre o Builder; a Revisão continua na barra', await page.evaluate(() => __t.navigation.center) === 'builder' && (await rv(page)).tab);
    ok('A3 a pergunta continua no registro (nada apagado)', conta(x, 'DELETE /api/turn') === 0);
    await page.click('#centerReview');
    await page.click('#reviewBody [data-action="conversation"]');
    ok('A3 "Abrir na conversa" ativa a pergunta', await page.evaluate(() => __t.studio.activeTurn) === '900101');
    ok('A3 os corpos de /api/gathered-question têm as chaves de sempre (9.5)', JSON.stringify(Object.keys(x.reqs.find(q => q.chave === 'POST /api/gathered-question').body)) === '["key","run","question","nodes","with_facts","origin"]');
    ok('A3 o nó enviado é o alvo juntado', JSON.stringify(x.reqs.find(q => q.chave === 'POST /api/gathered-question').body.nodes) === JSON.stringify([alvo]));
    // Fechar só fecha.
    await page.click('#centerReviewClose');
    r = await rv(page);
    ok('A3 1.4 fechar a Revisão só fecha: Cidade na frente, a aba some, nada é gravado no servidor', r.view === 'city' && !r.tab && !r.review && !x.writes.length, JSON.stringify(x.writes));
  }
  });

  // ================================================================ A4. "Ver revisão" na conversa; avisos (9.6.2, 4.3); atrasos
  await bloco(async () => {
  x = await pagina(browser);
  {
    const page = x.page;
    await page.evaluate(() => __t.activate('135', false));
    const temBotao = id => page.evaluate(id => { const d = document.querySelector(`#turns .turn[data-turn="${id}"]`); return d ? { btn: Boolean(d.querySelector('[data-action="review"]')), none: d.querySelector('.review-none')?.textContent ?? null } : null; }, id);
    ok('A4 4.7 "Ver revisão" na pergunta do Builder preparada (#135)', (await temBotao('135'))?.btn === true);
    await page.evaluate(id => { const t = __t.studio.turns.find(x => x.id === id); t.expanded = true; __t.renderTurns(); }, LADDER.id);
    const esc = await temBotao(LADDER.id);
    ok('A4 4.4 pergunta da escada: sem "Ver revisão", com "Sem revisão nesta rota"', esc && !esc.btn && esc.none === await T(page, 'review.noRoute'), JSON.stringify(esc));
    await page.click('#turns .turn[data-turn="135"] [data-action="review"]');
    await esperaRevisao(page, "__t.review.id === '135' && __t.review.evidence");
    r = await rv(page);
    ok('A4 "Ver revisão" abre a Revisão da #135 com a query dela', r.title === await T(page, 'review.titleTurn', { id: '135' }) && r.query === T135.query);
    ok('A4 abrir não grava nada no servidor (só leituras de /api/evidence e /api/facts)', !x.writes.length && conta(x, 'GET /api/evidence', '135') >= 1);
    // O que vai ser escolhido segue o compositor.
    const antesEscolha = r.blocks.chosen;
    await page.evaluate(() => { const p = document.querySelector('#provider'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
    r = await rv(page);
    ok('A4 trocar o LLM no compositor muda "Escolhido para o envio" na Revisão', r.blocks.chosen !== antesEscolha && r.blocks.chosen.includes(await T(page, 'review.chosenManual')) && r.send.text === await T(page, 'review.sendManual'), r.send.text);
    // Nenhum: "Enviar" só mostra a query na pergunta para copiar; nada vai ao servidor.
    await page.click('#reviewSend');
    await page.waitForFunction(() => document.querySelector('#reviewSent'), null, { timeout: 10000 });
    ok('A4 Nenhum: nada é enviado; a Revisão diz onde copiar e colar', conta(x, 'POST /api/send') === 0 && (await rv(page)).sent === await T(page, 'review.manualThere'));
    // Substituir: a Revisão da #139 toma o lugar, sem perda.
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '139')));
    await esperaRevisao(page, "__t.review.id === '139' && __t.review.evidence");
    ok('A4 1.4 uma Revisão substitui a outra (uma instância)', (await rv(page)).query === T139.query && (await page.evaluate(() => document.querySelectorAll('#centerReviewTab').length)) === 1);

    // 9.6.2 (simulado): H2 recusada nesta consulta: Enviar desabilitado com o texto do servidor.
    x.fake['GET /api/evidence'] = (rr, _, url) => (url.searchParams.get('turn') === '135'
      ? rr.fulfill({ json: { ...structuredClone(ev135), checks: { h1: { status: 'passed', why: null }, h2: { status: 'refused', why: 'H2: ficha alterada (teste da bateria)' }, h3: { status: 'not_run', why: null } } } }) : false);
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135')));
    await esperaRevisao(page, "__t.review.id === '135' && __t.review.evidence");
    r = await rv(page);
    ok('A4 9.6.2 H2 recusada: Enviar desabilitado', r.send.disabled);
    ok('A4 9.6.2 o motivo traz o texto do servidor como veio', r.why === await T(page, 'review.whyRefused', { h: await T(page, 'evidence.h2'), why: 'H2: ficha alterada (teste da bateria)' }), r.why);
    ok('A4 7.1 o aviso não diz "bloqueado pelo servidor"', !PROIBIDAS.some(p => p.test(r.text)));
    // Falha de leitura: Enviar desabilitado, "Ler de novo" lê de novo.
    x.fake['GET /api/evidence'] = (rr, _, url) => (url.searchParams.get('turn') === '135' ? rr.fulfill({ status: 500, json: { error: 'evidência ilegível (teste)' } }) : false);
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135')));
    await esperaRevisao(page, "__t.review.evidenceError");
    r = await rv(page);
    ok('A4 7.4 falha de leitura do pacote: a mensagem, Enviar desabilitado e "Ler de novo"', r.text.includes('evidência ilegível (teste)') && r.send.disabled && r.why === await T(page, 'review.whyUnread')
      && await page.evaluate(() => Boolean(document.querySelector('#reviewBody [data-action="again"]'))));
    delete x.fake['GET /api/evidence'];
    await page.click('#reviewBody [data-action="again"]');
    await esperaRevisao(page, "__t.review.evidence");
    ok('A4 "Ler de novo" lê e habilita Enviar', !(await rv(page)).send.disabled);
    // Carregando: o pacote segurado.
    const lenta = segura();
    x.fake['GET /api/evidence'] = async (rr, _, url) => { if (url.searchParams.get('turn') !== '135') return false; await lenta.p; return rr.continue(); };
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135')));
    await page.waitForTimeout(400);
    r = await rv(page);
    ok('A4 7.2 carregando: "Carregando o pacote…" e "Carregando as conferências…"; Enviar espera', r.blocks.package.includes(await T(page, 'review.loading', { what: await T(page, 'review.what.package') }))
      && r.blocks.checks.includes(await T(page, 'review.loading', { what: await T(page, 'review.what.checks') })) && r.send.disabled && r.why === await T(page, 'review.whyLoading'));
    // Resposta atrasada: a Revisão da #139 abre antes de a da #135 chegar; a da #135 não a sobrescreve.
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '139')));
    await esperaRevisao(page, "__t.review.id === '139' && __t.review.evidence");
    lenta.solta();
    await page.waitForTimeout(800);
    r = await rv(page);
    ok('A4 resposta atrasada da #135 descartada: a Revisão aberta é a da #139, com a query e o pacote dela', r.title === await T(page, 'review.titleTurn', { id: '139' }) && r.query === T139.query
      && await page.evaluate(() => __t.review.evidence.turn) === '139');
    delete x.fake['GET /api/evidence'];

    // 4.3 (simulado): um fato do pacote revogado depois da preparação. Aviso, recomendação, sem bloqueio.
    const comFato = () => ({ ...structuredClone(ev135), package: { ...structuredClone(ev135.package), query: { ...structuredClone(ev135.package.query), reviewed_facts: [{ id: 'F1' }] } } });
    let revogado = true;
    x.fake['GET /api/evidence'] = (rr, _, url) => (url.searchParams.get('turn') === '135' ? rr.fulfill({ json: comFato() }) : false);
    x.fake['GET /api/facts'] = (rr, _, url) => rr.fulfill({ json: { facts: facts.map(f => (f.id === 'F1' && revogado ? { ...f, status: 'revoked' } : f)), limits: [] } });
    x.fake['POST /api/send'] = (rr, body) => rr.fulfill({ json: envioFalso(ENVIO, `${body.turn}-rev`, MARCA) });
    await page.evaluate(() => { const p = document.querySelector('#provider'); p.value = [...p.options].map(o => o.value).find(v => v !== 'manual'); p.dispatchEvent(new Event('change')); });
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135')));
    await esperaRevisao(page, "__t.review.id === '135' && __t.review.evidence && __t.review.facts");
    r = await rv(page);
    ok('A4 4.3 o aviso "O fato F1 foi revogado depois da preparação. Recomendado: preparar de novo."', r.revoked === 'O fato F1 foi revogado depois da preparação. Recomendado: preparar de novo.', r.revoked);
    ok('A4 4.3 Enviar continua possível e "Voltar ao rascunho" está lá', !r.send.disabled && await page.evaluate(() => Boolean(document.querySelector('#reviewBody [data-action="draft"]'))));
    ok('A4 4.3 nada diz que o servidor bloqueou', !PROIBIDAS.some(p => p.test(r.text)));
    const antesFatos = conta(x, 'GET /api/facts');
    await page.click('#reviewSend');
    await page.waitForFunction(() => document.querySelector('#reviewSent'), null, { timeout: 30000 });
    ok('A4 9.6.6 com o fato revogado à vista, Enviar envia (sem bloqueio); os fatos foram relidos antes', conta(x, 'POST /api/send') === 1 && conta(x, 'GET /api/facts') > antesFatos);
    // Revogado entre abrir e clicar: o clique relê, mostra o aviso e não envia; o clique seguinte envia.
    revogado = false;
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135')));
    await esperaRevisao(page, "__t.review.id === '135' && __t.review.evidence && __t.review.facts");
    ok('A4 sem fato revogado, sem aviso', (await rv(page)).revoked === '');
    revogado = true;
    const envios = conta(x, 'POST /api/send');
    await page.click('#reviewSend');
    await page.waitForFunction(() => document.querySelector('#reviewRevoked'), null, { timeout: 10000 });
    await page.waitForTimeout(300);
    ok('A4 4.3 revogado depois de abrir: o clique relê os fatos, mostra o aviso e não envia', conta(x, 'POST /api/send') === envios && (await rv(page)).revoked.includes('F1'));
    await page.click('#reviewSend');
    await page.waitForFunction(() => document.querySelector('#reviewSent'), null, { timeout: 30000 });
    ok('A4 4.3 o clique seguinte envia (o aviso não bloqueia)', conta(x, 'POST /api/send') === envios + 1);
    // Releitura que falha: "Não foi possível conferir revogações antes do envio"; Enviar continua.
    x.fake['GET /api/facts'] = rr => rr.fulfill({ status: 500, json: { error: 'fatos ilegíveis (teste)' } });
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135')));
    await esperaRevisao(page, "__t.review.id === '135' && __t.review.evidence && __t.review.factsError");
    r = await rv(page);
    ok('A4 4.3 releitura dos fatos falhou: "Não foi possível conferir revogações antes do envio", Enviar continua', r.revoked === await T(page, 'review.revokedUnread', { error: 'fatos ilegíveis (teste)' }) && !r.send.disabled, r.revoked);
    delete x.fake['GET /api/facts']; delete x.fake['GET /api/evidence'];

    // Resposta atrasada de um envio depois de trocar de run (T14): a Revisão fecha; a resposta fica na run dela.
    const preso = segura();
    x.fake['POST /api/send'] = async (rr, body) => { await preso.p; return rr.fulfill({ json: envioFalso(ENVIO, `${body.turn}-tarde`, MARCA) }); };
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '139')));
    await esperaRevisao(page, "__t.review.id === '139' && __t.review.evidence && __t.review.facts");
    await page.click('#reviewSend');
    await page.waitForTimeout(400);
    const navAntes = await page.evaluate(k => localStorage.getItem(k), NAV);
    await trocaRun(page, OUTRA);
    r = await rv(page);
    ok('A4 T14 trocar de run fecha a Revisão (Cidade na frente, sem aba)', !r.review && r.view === 'city' && !r.tab);
    ok('A4 T14 a Revisão guardada (da run deixada) não fica apontada: a vista guardada volta à Cidade', JSON.parse(navAntes ?? '{}').center?.view === 'review' && JSON.parse(await page.evaluate(k => localStorage.getItem(k), NAV) ?? '{}').center?.view === 'city');
    preso.solta();
    await page.waitForFunction(() => /139/.test(document.querySelector('#chatNote').textContent), null, { timeout: 30000 });
    ok('A4 a resposta atrasada da #139 é dita como da outra run e não aparece na conversa aberta', (await page.evaluate(() => document.querySelector('#chatNote').textContent)) === await T(page, 'turn.lateAnswer', { turn: '139', where: await T(page, 'turn.whereRun', { run: RUN }) })
      && !(await page.evaluate(() => [...document.querySelectorAll('#turns .bubble.reply')].some(b => b.innerText.includes('139-tarde')))));
    ok('A4 nenhuma Revisão reaberta pela resposta atrasada', !(await rv(page)).review);
  }
  });

  // ================================================================ A5. T17: a Revisão de uma pergunta volta ao recarregar
  await bloco(async () => {
  x = await pagina(browser);
  {
    const page = x.page;
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135')));
    await esperaRevisao(page, "__t.review.evidence");
    const nav = JSON.parse(await page.evaluate(k => localStorage.getItem(k), NAV));
    ok('A5 5.3 abrir a Revisão guarda a vista central com a pergunta ({view, turn})', JSON.stringify(nav.center) === JSON.stringify({ view: 'review', turn: '135' }), JSON.stringify(nav.center));
    await page.reload();
    await page.waitForFunction(() => window.__t && __t.studio.run && __t.studio.turnsState === 'ready', null, { timeout: 120000 });
    await esperaRevisao(page, "__t.review.id === '135' && __t.review.evidence");
    r = await rv(page);
    ok('A5 T17 recarregar reabre a Revisão da #135 (refeita do registro), na frente', r.view === 'review' && r.query === T135.query);
    ok('A5 recarregar não grava nada no servidor', !x.writes.length);
    // Uma pergunta que não existe mais: a Cidade.
    await page.evaluate(k => { const n = JSON.parse(localStorage.getItem(k)); n.center = { view: 'review', turn: '99999' }; localStorage.setItem(k, JSON.stringify(n)); }, NAV);
    await page.reload();
    await page.waitForFunction(() => window.__t && __t.studio.run && __t.studio.turnsState === 'ready', null, { timeout: 120000 });
    await page.waitForTimeout(800);
    ok('A5 Revisão guardada de uma pergunta que não existe: Cidade, sem erro', !(await rv(page)).review && await page.evaluate(() => __t.navigation.center) === 'city');
    // Trocar de run com a Revisão aberta: o ponto guardado volta à Cidade; recarregar na outra run não abre a #135
    // dela (que existe, com outro conteúdo).
    await page.evaluate(k => { const n = JSON.parse(localStorage.getItem(k)); n.center = { view: 'review', turn: '135' }; localStorage.setItem(k, JSON.stringify(n)); }, NAV);
    await page.reload();
    await page.waitForFunction(() => window.__t && __t.studio.run && __t.studio.turnsState === 'ready', null, { timeout: 120000 });
    await esperaRevisao(page, "__t.review.id === '135'");
    await pronta(page);
    await trocaRun(page, OUTRA);
    const nav2 = JSON.parse(await page.evaluate(k => localStorage.getItem(k), NAV));
    ok('A5 trocar de run com a Revisão guardada: a vista guardada volta à Cidade (a run escolhida é gravada)', nav2.center?.view === 'city' && nav2.run === OUTRA, JSON.stringify(nav2));
    ok('A5 a outra run também tem uma #135', await page.evaluate(() => Boolean(__t.studio.turns.find(t => t.id === '135'))));
    await page.reload();
    await page.waitForFunction(() => window.__t && __t.studio.run && __t.studio.turnsState === 'ready', null, { timeout: 120000 });
    await page.waitForTimeout(800);
    ok('A5 recarregar na outra run não abre a #135 dela', !(await rv(page)).review && await page.evaluate(() => __t.studio.run) === OUTRA);
  }
  });

  // ================================================================ B. Lote (4.5, T7): Separar
  await bloco(async () => {
  x = await pagina(browser);
  {
    const page = x.page;
    // A função escolhida em Separar com o maior número de linhas que a chamam.
    await page.click('#areas [data-area="explore"]');
    await page.evaluate(() => document.querySelector('[data-explore="hot"]')?.click());
    const escolha = await page.evaluate(async () => {
      const pick = document.querySelector('#hotPick');
      let best = null;
      for (const o of [...pick.options].filter(o => o.value)) {
        pick.value = o.value; pick.dispatchEvent(new Event('change'));
        const b = [...document.querySelectorAll('#hotList .hot-sep button.primary')].find(x => /\(\d+ /.test(x.textContent));
        const n = b ? Number(b.textContent.match(/\((\d+) /)[1]) : 0;
        if (n >= 6 && (!best || n < best.n)) best = { value: o.value, n };
      }
      if (best) { pick.value = best.value; pick.dispatchEvent(new Event('change')); }
      return best;
    });
    ok('B0 há uma função com 6 ou mais linhas que a chamam, para o lote', Boolean(escolha), JSON.stringify(escolha));
    const N = escolha.n;
    // Criação: a segunda falha no servidor (simulado); as outras são cópias da #135 com ids 9002xx.
    let criadas = 0;
    x.fake['POST /api/gathered-question'] = rr => { criadas++; if (criadas === 2) return rr.fulfill({ status: 500, json: { error: 'criação recusada (teste)' } });
      return rr.fulfill({ json: falsa(T135, 900200 + criadas, { question: JSON.parse(rr.request().postData()).question, gathered: { ...structuredClone(T135.gathered), nodes: JSON.parse(rr.request().postData()).nodes, origin: JSON.parse(rr.request().postData()).origin } }) }); };
    // Conferências: a terceira pergunta criada tem H3 recusada (simulado).
    x.fake['GET /api/evidence'] = (rr, _, url) => { const id = url.searchParams.get('turn'); if (!id.startsWith('9002')) return false;
      const ev = { ...structuredClone(ev135), turn: id };
      if (id === '900203') ev.checks.h3 = { status: 'refused', why: 'H3: query mudou (teste da bateria)' };
      return rr.fulfill({ json: ev }); };
    const btn = await page.evaluate(() => [...document.querySelectorAll('#hotList .hot-sep button.primary')].find(x => /\(\d+ /.test(x.textContent)).textContent);
    await page.evaluate(() => [...document.querySelectorAll('#hotList .hot-sep button.primary')].find(x => /\(\d+ /.test(x.textContent)).click());
    await esperaRevisao(page, "__t.review.kind === 'lot' && !__t.review.creating && __t.review.items.every(i => !i.turn || i.auto || i.evidence)");
    await page.waitForTimeout(300);
    r = await rv(page);
    let it = await itens(page);
    const corpos = x.reqs.filter(q => q.chave === 'POST /api/gathered-question').map(q => q.body);
    ok(`B1 T7 "${btn}": uma criação por linha (${N}), uma depois da outra`, corpos.length === N, corpos.length);
    ok('B1 9.7.1 cada pergunta com uma fonte só, origem hot, sem fatos', corpos.every(c => c.nodes.length === 1 && c.origin.kind === 'hot' && c.with_facts === false));
    const nosItens = await page.evaluate(() => __t.review.items.map(i => [i.node, i.summary]));
    ok('B1 9.7.2 cada corpo leva só a fonte da própria linha (nós e resumo de cada item, na ordem)', corpos.every((c, k) => c.nodes[0] === nosItens[k][0] && c.origin.summary === nosItens[k][1]), JSON.stringify(nosItens.slice(0, 2)));
    ok('B1 criar NÃO envia: nenhum POST /api/send', conta(x, 'POST /api/send') === 0);
    ok('B1 nenhuma decisão é tomada pelo lote: nenhum POST /api/review', conta(x, 'POST /api/review') === 0);
    ok('B1 a Revisão do lote abre no centro, com N itens', r.view === 'review' && r.review.kind === 'lot' && it.length === N && r.title === await T(page, 'review.titleLot', { n: N }), r.title);
    ok('B1 4.5 criação com falha: a falha aparece no item, e as outras continuaram', it[1].text.includes('criação recusada (teste)') && !it[1].turn && it.filter(i => i.turn).length === N - 1);
    ok('B1 4.5 todas as criadas vêm marcadas, menos a com H3 recusada', it.filter(i => i.turn && i.turn !== '900203').every(i => i.box.checked) && !it.find(i => i.turn === '900203').box.checked);
    ok('B1 4.5 a com H3 recusada é desmarcada automaticamente, com o motivo do servidor, e não pode ser marcada', it.find(i => i.turn === '900203').text.includes('H3: query mudou (teste da bateria)') && it.find(i => i.turn === '900203').box.disabled);
    const turnosLote = await page.evaluate(() => __t.review.items.filter(i => i.turn).map(i => ({ id: i.turn.id, query: i.turn.query, question: i.turn.question })));
    ok('B1 4.5 cada item mostra o próprio texto e a própria query', turnosLote.every(tl => { const i = it.find(z => z.turn === tl.id); return i.text.includes(tl.question) && i.query === tl.query; }));
    const PREP = await T(page, 'state.prepared');
    ok('B1 4.5 cada item mostra o próprio H1 a H3 e o próprio estado ("Preparada, não enviada")', it.filter(i => i.turn).every(i => /H1[^·:]*: [^·]+ · H2[^·:]*: [^·]+ · H3[^·:]*: /.test(i.text) && i.text.includes(PREP)));
    ok('B1 as perguntas do lote entram na linha do tempo, preparadas', await page.evaluate(ids => ids.every(id => document.querySelector(`#turns .turn[data-turn="${id}"]`)), turnosLote.map(t => t.id)));
    ok('B1 o modelo do lote é fixo e dito (qwen)', r.text.includes(await page.evaluate(() => __t.t('lot.model', { model: 'Ollama · verificavel-qwen3coder' }))));
    ok('B1 a Revisão do lote diz que enviar não aprova nada', r.text.includes(await T(page, 'lot.notApprove')));
    ok('B1 "Não conferido nesta revisão" também no lote', r.text.includes(await T(page, 'review.unchecked')));

    // Desmarcar uma: a seleção conferível antes de enviar.
    const marcadas = it.filter(i => i.box.checked).length;
    ok('B2 o botão diz quantas vão: "Enviar as marcadas (k de N)"', r.lotSend.text === await T(page, 'lot.send', { k: marcadas, n: N }), r.lotSend.text);
    const desmarcada = it.find(i => i.box.checked).turn;
    await page.click(`#reviewBody .rv-item[data-turn="${desmarcada}"] input`);
    r = await rv(page);
    ok('B2 desmarcar muda a conta do botão', r.lotSend.text === await T(page, 'lot.send', { k: marcadas - 1, n: N }), r.lotSend.text);
    // Envio: um por vez. A primeira marcada responde, a segunda falha, a terceira fica segurada e depois responde.
    const vao = (await itens(page)).filter(i => i.box.checked && !i.box.disabled).map(i => i.turn);
    const ordem = [], emVoo = { agora: 0, max: 0 };
    const terceira = segura();
    x.fake['POST /api/send'] = async (rr, body) => {
      ordem.push(body.turn); emVoo.agora++; emVoo.max = Math.max(emVoo.max, emVoo.agora);
      const k = ordem.length;
      try {
        if (k === 2) return rr.fulfill({ status: 502, json: { error: 'ollama caiu (teste)' } });
        if (k === 3) await terceira.p;
        return rr.fulfill({ json: envioFalso(ENVIO, `${body.turn}-lote`, MARCA) });
      } finally { emVoo.agora--; }
    };
    await page.click('#lotSend');
    await page.waitForFunction(() => __t.review.sending?.k === 3, null, { timeout: 60000 });
    await page.waitForTimeout(500);
    r = await rv(page);
    ok('B3 7.2 "Enviando k de N" no lote e na aba, durante o envio', r.why === await T(page, 'lot.whySending', { k: 3, n: vao.length }) && r.tabText.includes(await T(page, 'review.tabSending', { k: 3, n: vao.length })), `${r.why} | ${r.tabText}`);
    ok('B3 durante o envio, nenhuma marca pode mudar', (await itens(page)).every(i => i.box.disabled));
    ok('B3 9.7 com a terceira segurada, nenhuma outra foi enviada (uma por vez)', ordem.length === 3);
    terceira.solta();
    await page.waitForFunction(() => __t.review.done, null, { timeout: 60000 });
    it = await itens(page);
    r = await rv(page);
    ok('B3 4.5 só as marcadas foram enviadas, na ordem do lote, uma por vez', JSON.stringify(ordem) === JSON.stringify(vao) && emVoo.max === 1, `${ordem} / máx ${emVoo.max}`);
    ok('B3 cada envio leva a própria pergunta, a run dela e o modelo fixo', x.reqs.filter(q => q.chave === 'POST /api/send').every((q, k) => q.body.turn === vao[k] && q.body.run === RUN && q.body.provider === 'ollama' && q.body.modelfile === 'verificavel-qwen3coder'));
    ok('B3 9.7.3 a desmarcada não foi enviada e continua "Preparada, não enviada"', !ordem.includes(desmarcada) && it.find(i => i.turn === desmarcada).result === 'unmarked'
      && await page.evaluate(id => document.querySelector(`#turns .turn[data-turn="${id}"] .state-tag`)?.dataset.state, desmarcada) === 'prepared');
    ok('B3 a com H3 recusada não foi enviada', !ordem.includes('900203'));
    ok('B3 9.7.4 a falha da segunda não impediu a terceira nem as seguintes', it.find(i => i.turn === vao[1]).result === 'failed' && vao.slice(2).every(id => it.find(i => i.turn === id).result === 'sent'));
    ok('B3 9.7.4 a falha aparece no próprio item e na própria pergunta', it.find(i => i.turn === vao[1]).text.includes('ollama caiu (teste)')
      && (await page.evaluate(id => document.querySelector(`#turns .turn[data-turn="${id}"]`).innerText, vao[1])).includes('ollama caiu (teste)'));
    ok('B3 a falha não aparece nas outras perguntas', !(await page.evaluate(ids => ids.some(id => document.querySelector(`#turns .turn[data-turn="${id}"]`).innerText.includes('ollama caiu (teste)')), [vao[0], ...vao.slice(2)])));
    ok('B3 cada resultado é o da própria pergunta (o envio de cada uma tem o id dela)', vao.filter((_, k) => k !== 1).every(id => it.find(i => i.turn === id).text.includes(`${id}-lote`)));
    ok('B3 a resposta segurada (atrasada) caiu na própria pergunta, não em outra', await page.evaluate(id => Boolean(document.querySelector(`#turns .turn[data-turn="${id}"] [data-send="${id}-lote"]`)) && document.querySelectorAll(`#turns [data-send="${id}-lote"]`).length === 1, vao[2]));
    ok('B3 7.2 concluído: "Envio do lote concluído: …"', r.note === await T(page, 'lot.done', { sent: vao.length - 1, failed: 1, other: it.filter(i => ['unmarked', 'skipped', 'notSent'].includes(i.result)).length }), r.note);
    ok('B3 o lote não aprovou nada: nenhum POST /api/review', conta(x, 'POST /api/review') === 0);
    // "Aprovar todos" é outra ação, fora da Revisão do lote.
    ok('B3 "Aprovar todos…" não está na Revisão do lote', !(await page.evaluate(t => [...document.querySelectorAll('#reviewBody button')].some(b => b.textContent === t), await T(page, 'hot.approveAll'))));
    ok('B3 "Aprovar todos…" aparece em Onde pesa, à parte', await page.evaluate(() => { const b = document.querySelector('#hotApprove'); return Boolean(b) && !b.hidden; }));
    // Reenviar a desmarcada depois, marcando-a: só ela vai.
    await page.click(`#reviewBody .rv-item[data-turn="${desmarcada}"] input`);
    x.fake['POST /api/send'] = (rr, body) => { ordem.push(body.turn); return rr.fulfill({ json: envioFalso(ENVIO, `${body.turn}-lote`, MARCA) }); };
    const antesDe = ordem.length;
    await page.click('#lotSend');
    await page.waitForFunction(() => !__t.review.sending && __t.review.done, null, { timeout: 60000 });
    await page.waitForTimeout(300);
    ok('B4 marcar depois e enviar as marcadas: vão a recém-marcada e a que falhou (continua marcada); as já enviadas não vão de novo', JSON.stringify(ordem.slice(antesDe)) === JSON.stringify([desmarcada, vao[1]].sort()), ordem.slice(antesDe).join());
    junta(x);

    // B5. Troca de run durante o envio do lote: as que faltam não saem; a enviada responde na run dela.
    const segundo = segura();
    criadas = 10;
    delete x.fake['POST /api/send'];
    await page.evaluate(() => [...document.querySelectorAll('#hotList .hot-sep button.primary')].find(x => /\(\d+ /.test(x.textContent)).click());
    await esperaRevisao(page, "__t.review.kind === 'lot' && !__t.review.creating && __t.review.items.every(i => !i.turn || i.auto || i.evidence)");
    const lote2 = (await itens(page)).filter(i => i.box.checked).map(i => i.turn);
    const ordem2 = [];
    x.fake['POST /api/send'] = async (rr, body) => { ordem2.push(body.turn); await segundo.p; return rr.fulfill({ json: envioFalso(ENVIO, `${body.turn}-b5`, MARCA) }); };
    await page.click('#lotSend');
    await page.waitForFunction(() => __t.review.sending?.k === 1, null, { timeout: 30000 });
    await page.waitForTimeout(300);
    await trocaRun(page, OUTRA);
    ok('B5 T14 trocar de run fecha a Revisão do lote', !(await rv(page)).review);
    segundo.solta();
    await page.waitForTimeout(1500);
    ok('B5 depois da troca, nenhuma outra pergunta do lote foi enviada', ordem2.length === 1 && ordem2[0] === lote2[0], ordem2.join());
    ok('B5 a resposta da primeira foi dita como da outra run, sem entrar na conversa aberta', (await page.evaluate(() => document.querySelector('#chatNote').textContent)).includes(lote2[0])
      && !(await page.evaluate(() => [...document.querySelectorAll('#turns .bubble.reply')].some(b => b.innerText.includes('-b5')))));
    ok('B5 "Aprovar todos…" não aparece na outra run (o lote é da run dele)', await page.evaluate(() => !document.querySelector('#hotApprove') || document.querySelector('#hotApprove').hidden));
    // B6. Troca de run durante a criação: as que faltam não são criadas.
    await trocaRun(page, RUN);
    await page.click('#areas [data-area="explore"]');
    await page.evaluate(v => { const p = document.querySelector('#hotPick'); p.value = v; p.dispatchEvent(new Event('change')); }, escolha.value);
    const criacao = segura();
    let criadas3 = 0;
    x.fake['POST /api/gathered-question'] = async rr => { criadas3++; if (criadas3 === 2) await criacao.p; return rr.fulfill({ json: falsa(T135, 900300 + criadas3) }); };
    await page.evaluate(() => [...document.querySelectorAll('#hotList .hot-sep button.primary')].find(x => /\(\d+ /.test(x.textContent)).click());
    await page.waitForFunction(() => __t.review?.creating?.k === 2, null, { timeout: 30000 });
    await trocaRun(page, OUTRA);
    criacao.solta();
    await page.waitForTimeout(1500);
    ok('B6 troca de run durante a criação: as que faltavam não são criadas', criadas3 === 2, criadas3);
    ok('B6 a criada depois da troca não entra na conversa da outra run', !(await page.evaluate(() => Boolean(document.querySelector('#turns .turn[data-turn="900302"]')))));
    ok('B6 nada foi enviado nem decidido', !x.reqs.some(q => q.chave === 'POST /api/review'));
  }
  });

  // ================================================================ C. "Aprovar todos" (depois das respostas)
  await bloco(async () => {
  x = await pagina(browser);
  {
    const page = x.page;
    // O lote vem de "Conferir fatos" (askFacts), com três perguntas criadas aqui (900401 a 900403).
    let criadas = 0;
    x.fake['POST /api/gathered-question'] = rr => { criadas++; return rr.fulfill({ json: falsa(T135, 900400 + criadas, { gathered: { ...structuredClone(T135.gathered), origin: { kind: 'hot', summary: 'c' } } }) }); };
    x.fake['GET /api/evidence'] = (rr, _, url) => (url.searchParams.get('turn').startsWith('9004') ? rr.fulfill({ json: { ...structuredClone(ev135), turn: url.searchParams.get('turn') } }) : false);
    const simbolos = await page.evaluate(() => __t.hot.rows.filter(r => r.symbol).slice(0, 3).map(r => r.symbol));
    await page.evaluate(s => __t.askFacts(s.map((symbol, k) => ({ symbol, question: `Fato ${k + 1}?`, summary: `fato ${k + 1}` }))), simbolos);
    await esperaRevisao(page, "__t.review.kind === 'lot' && !__t.review.creating");
    ok('C0 "Conferir fatos": 3 perguntas, uma fonte cada, nada enviado', criadas === 3 && x.reqs.filter(q => q.chave === 'POST /api/gathered-question').every((q, k) => JSON.stringify(q.body.nodes) === JSON.stringify([simbolos[k]])) && conta(x, 'POST /api/send') === 0);
    // As respostas (simuladas, como /api/turns as devolveria depois dos envios): A com dois FACTs aprováveis e itens
    // que não são; B com FACT em envio reprovado e FACT em envio sem vínculo; C sem envio.
    const claim = (id, kind, text) => ({ id, kind, text, citations: [], premises: [], location: null });
    const sendA = { ...structuredClone(ENVIO), id: 'A-1', reviews: { C2: { decision: 'approved', fact_id: 'F77', note: 'já', reviewed: 'ontem' } },
      check: { ...structuredClone(ENVIO.check), report: { ...structuredClone(ENVIO.check.report), mechanical_status: 'passed' },
        claims: [claim('C1', 'FACT', 'A soma é comutativa.'), claim('C2', 'FACT', 'Já aprovado antes.'), claim('C3', 'LACUNA', 'Falta o corpo de x.'),
          claim('C4', 'INFERENCE', 'Logo, y.'), claim('C5', 'HYPOTHESIS', 'Talvez z.'), claim('C6', 'FACT', 'Retorna None sem dados.')] } };
    const sendB1 = { ...structuredClone(ENVIO), id: 'B-1', reviews: {}, check: { ...structuredClone(ENVIO.check), report: { ...structuredClone(ENVIO.check.report), mechanical_status: 'failed' }, claims: [claim('C1', 'FACT', 'Reprovado.')] } };
    const sendB2 = { ...structuredClone(ENVIO), id: 'B-2', reviews: {}, link: null, check: { ...structuredClone(ENVIO.check), report: { ...structuredClone(ENVIO.check.report), mechanical_status: 'passed' }, claims: [claim('C1', 'FACT', 'Sem vínculo.')] } };
    let decididoFora = false;
    const respostas = () => [falsa(T135, '900401', { sends: [decididoFora ? { ...structuredClone(sendA), reviews: { ...sendA.reviews, C1: { decision: 'approved', fact_id: 'F78', note: 'fora', reviewed: 'agora' } } } : structuredClone(sendA)] }),
      falsa(T135, '900402', { sends: [structuredClone(sendB1), structuredClone(sendB2)] }), falsa(T135, '900403')];
    x.fake['GET /api/turns'] = async (rr, _, url) => { if (url.searchParams.get('run') !== RUN) return false; const res = await rr.fetch(); return rr.fulfill({ json: [...(await res.json()), ...respostas()] }); };
    const decisoes = [];
    x.fake['POST /api/review'] = (rr, body) => { decisoes.push(body); if (body.claim === 'C6') return rr.fulfill({ status: 400, json: { error: 'C6: a citação não confere mais (teste)' } }); return rr.fulfill({ json: { claim_id: body.claim, decision: 'approved', fact_id: 'F90' } }); };
    await page.click('#areas [data-area="explore"]');
    await page.evaluate(() => { document.querySelector('[data-explore="hot"]')?.click(); __t.renderHot(); });
    ok('C1 "Aprovar todos…" está em Onde pesa para o lote desta run', await page.evaluate(() => { const b = document.querySelector('#hotApprove'); return Boolean(b) && !b.hidden && b.textContent; }) === await T(page, 'hot.approveAll'));
    await page.evaluate(() => document.querySelector('#hotApprove').click());
    await dialogo(page);
    const previa = await page.evaluate(() => ({ title: document.querySelector('#askTitle').textContent, go: [...document.querySelectorAll('#approvePreview .approve-go li')].map(l => [l.dataset.turn, l.dataset.claim, l.innerText]),
      not: [...document.querySelectorAll('#approvePreview .approve-not li')].map(l => [l.dataset.turn, l.dataset.claim, l.innerText]), input: !document.querySelector('#askInput').hidden, ok: document.querySelector('#askOk').textContent }));
    ok('C1 a prévia lista os aprováveis: os FACTs conferidos, com vínculo, ainda não decididos (C1 e C6 da #900401)', JSON.stringify(previa.go.map(g => g[0] + g[1])) === '["900401C1","900401C6"]', JSON.stringify(previa.go.map(g => g[0] + g[1])));
    ok('C1 a prévia identifica cada item (pergunta, envio, claim, tipo e texto)', previa.go[0][2].includes('#900401') && previa.go[0][2].includes('A-1') && previa.go[0][2].includes('C1') && previa.go[0][2].includes('A soma é comutativa.'));
    const MEC = await T(page, 'review.mechanical'), SEMV = await T(page, 'review.noLink');
    const motivo = (turn, cl) => previa.not.find(n => n[0] === turn && n[1] === cl)?.[2] ?? '';
    ok('C1 não aprovável: FACT já aprovado, com o fato', motivo('900401', 'C2').includes(await T(page, 'approve.why.approved', { id: 'F77' })));
    ok('C1 não aprovável: LACUNA, com o motivo da aprovação uma a uma', motivo('900401', 'C3').includes(await T(page, 'review.why.lacuna')));
    ok('C1 não aprovável: INFERENCE, com o motivo', motivo('900401', 'C4').includes(await T(page, 'review.why.inference')));
    ok('C1 não aprovável: HYPOTHESIS ("Aprovar todos" só aprova FACT)', motivo('900401', 'C5').includes(await T(page, 'approve.why.notFact', { kind: 'HYPOTHESIS' })));
    ok('C1 não aprovável: FACT de envio que não passou na conferência mecânica', motivo('900402', 'C1') !== '' && previa.not.filter(n => n[0] === '900402').some(n => n[2].includes(MEC)));
    ok('C1 não aprovável: FACT de envio sem vínculo', previa.not.filter(n => n[0] === '900402').some(n => n[2].includes(SEMV)));
    ok('C1 pergunta sem envio aparece como fora da operação, com o motivo', motivo('900403', '').includes(await T(page, 'approve.why.notSent')));
    ok('C1 a prévia pede a nota e diz quantos vão ("Aprovar 2")', previa.input && previa.ok === await T(page, 'approve.ok', { n: 2 }) && previa.title === await T(page, 'hot.approveTitle', { n: 2 }), previa.ok);
    ok('C1 antes de confirmar, nada foi decidido', decisoes.length === 0);
    // Cancelar: nada.
    await page.click('#askDialog [data-close]').catch(async () => { await page.keyboard.press('Escape'); });
    await fechado(page);
    ok('C2 cancelar a prévia: nenhuma decisão', decisoes.length === 0);
    // Nota vazia: a mesma exigência da aprovação uma a uma.
    await page.evaluate(() => document.querySelector('#hotApprove').click());
    await dialogo(page);
    await page.click('#askOk');
    await page.waitForTimeout(300);
    ok('C2 sem nota não aprova (a exigência da aprovação uma a uma)', decisoes.length === 0 && await page.evaluate(() => document.querySelector('#askDialog').open && !document.querySelector('#askNote').hidden));
    await page.fill('#askInput', 'conferido na leitura do código');
    await page.click('#askOk');
    await page.waitForFunction(() => document.querySelector('#approveResult'), null, { timeout: 30000 });
    const resultado = await page.evaluate(() => ({ done: [...document.querySelectorAll('#approveResult .approve-done li')].map(l => [l.dataset.turn, l.dataset.claim, l.innerText]),
      failed: [...document.querySelectorAll('#approveResult .approve-failed li')].map(l => [l.dataset.turn, l.dataset.claim, l.innerText]), title: document.querySelector('#askTitle').textContent }));
    ok('C3 só os aprováveis foram decididos, cada um no seu pedido (2 POST /api/review)', decisoes.length === 2 && JSON.stringify(decisoes.map(d => d.turn + d.send + d.claim)) === '["900401A-1C1","900401A-1C6"]', JSON.stringify(decisoes.map(d => d.claim)));
    ok('C3 cada pedido leva a decisão "approved", a nota e a run do lote', decisoes.every(d => d.decision === 'approved' && d.note === 'conferido na leitura do código' && d.run === RUN && d.key === CHAVE_RENDERER));
    ok('C3 nada foi enviado por "Aprovar todos"', conta(x, 'POST /api/send') === 0);
    ok('C3 resultado por item: C1 aprovado como F90', resultado.done.length === 1 && resultado.done[0][1] === 'C1' && resultado.done[0][2].includes(await T(page, 'approve.done', { id: 'F90' })), JSON.stringify(resultado.done));
    ok('C3 falha parcial: C6 não aprovado, com a mensagem do servidor como veio', resultado.failed.length === 1 && resultado.failed[0][1] === 'C6' && resultado.failed[0][2].includes('C6: a citação não confere mais (teste)'), JSON.stringify(resultado.failed));
    await page.click('#askOk');
    await fechado(page);
    ok('C3 a nota de Onde pesa resume: 1 aprovado, 1 não aprovado, 7 fora da operação', await page.evaluate(() => document.querySelector('#hotNote').textContent) === await T(page, 'approve.summary', { ok: 1, failed: 1, not: 7 }), await page.evaluate(() => document.querySelector('#hotNote').textContent));
    // Decidido depois da prévia: não é aprovado de novo.
    decisoes.length = 0;
    await page.evaluate(() => document.querySelector('#hotApprove').click());
    await dialogo(page);
    decididoFora = true;
    await page.fill('#askInput', 'segunda vez');
    await page.click('#askOk');
    await page.waitForFunction(() => document.querySelector('#approveResult'), null, { timeout: 30000 });
    const r2 = await page.evaluate(() => [...document.querySelectorAll('#approveResult .approve-failed li')].map(l => [l.dataset.claim, l.innerText]));
    ok('C4 um item decidido entre a prévia e a confirmação não é aprovado de novo', !decisoes.some(d => d.claim === 'C1') && r2.find(z => z[0] === 'C1')?.[1].includes(await T(page, 'approve.why.changed')), JSON.stringify(r2));
    await page.click('#askOk');
    await fechado(page);
    // Nenhum aprovável: só a prévia, sem nota e sem decisão.
    x.fake['GET /api/turns'] = async (rr, _, url) => { if (url.searchParams.get('run') !== RUN) return false; const res = await rr.fetch(); return rr.fulfill({ json: [...(await res.json()), falsa(T135, '900402', { sends: [structuredClone(sendB1)] }), falsa(T135, '900403')] }); };
    decisoes.length = 0;
    await page.evaluate(() => document.querySelector('#hotApprove').click());
    await dialogo(page);
    const vazio = await page.evaluate(() => ({ title: document.querySelector('#askTitle').textContent, input: !document.querySelector('#askInput').hidden, not: document.querySelectorAll('#approvePreview .approve-not li').length }));
    ok('C5 nenhum FACT aprovável: "Nenhum fato a aprovar neste lote", sem nota, com os motivos', vazio.title === await T(page, 'approve.noneTitle') && !vazio.input && vazio.not === 2, JSON.stringify(vazio));
    await page.click('#askOk');
    await fechado(page);
    ok('C5 nenhuma decisão', decisoes.length === 0);
    // Outra run: "Aprovar todos" não age.
    await trocaRun(page, OUTRA);
    const leiturasAntes = x.reqs.filter(q => q.chave === 'GET /api/turns').length;
    await page.evaluate(() => __t.approveHotFacts(document.createElement('button'), document.querySelector('#hotNote')));
    await page.waitForTimeout(400);
    ok('C6 trocar de run esquece o lote: "Aprovar todos" some, e chamá-lo não lê nem decide nada', await page.evaluate(() => __t.hotBatch === null && (!document.querySelector('#hotApprove') || document.querySelector('#hotApprove').hidden))
      && decisoes.length === 0 && x.reqs.filter(q => q.chave === 'GET /api/turns').length === leiturasAntes);
  }
  });

  // ================================================================ D. EN, tema escuro, tela estreita, teclado, chaves
  await bloco(async () => {
  x = await pagina(browser, { scheme: 'dark' });
  {
    const page = x.page;
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135')));
    await esperaRevisao(page, "__t.review.evidence && __t.review.facts");
    const cor = await page.evaluate(() => { const e = document.querySelector('#reviewBody .rv-label'); const s = getComputedStyle(e); let b = e; let bg = 'rgba(0, 0, 0, 0)';
      while (b && (bg === 'rgba(0, 0, 0, 0)' || bg === 'transparent')) { bg = getComputedStyle(b).backgroundColor; b = b.parentElement; } return [s.color, bg]; });
    ok('D tema escuro: o rótulo dos blocos tem contraste 4,5 ou mais', contraste(cor) >= 4.5, `${contraste(cor).toFixed(2)} ${cor}`);
    await page.evaluate(() => __t.setLang('en'));
    await page.waitForTimeout(300);
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135')));
    await esperaRevisao(page, "__t.review.evidence && __t.review.facts");
    r = await rv(page);
    ok('D EN: título e blocos em inglês', r.title === 'Review of question #135' && r.blocks.unchecked.includes('Not checked in this review: identity with the send (D3), SYSTEM and skills (D4), refusal for a revoked fact (D7)') && r.send.text.startsWith('Send'), r.title);
    ok('D 9.6.4 nenhuma frase proibida na Revisão (EN)', !PROIBIDAS.some(p => p.test(r.text)), PROIBIDAS.find(p => p.test(r.text)));
    const chaves = await page.evaluate(() => Object.entries(__t.STRINGS).filter(([k]) => /^(review|lot|approve)\.|^route\.review$|^composer\.prepare|^col\.review$|^help\.review$/.test(k)));
    ok(`D 9.12.2 as ${chaves.length} chaves do F10 existem em PT e EN, não vazias`, chaves.length > 100 && chaves.every(([, v]) => Array.isArray(v) && v.length === 2 && v[0] && v[1]), chaves.filter(([, v]) => !(v[0] && v[1])).map(([k]) => k).join());
    ok('D 9.6.4 nenhuma das chaves do F10 tem frase proibida (PT e EN)', !chaves.some(([, v]) => v.some(s => PROIBIDAS.some(p => p.test(s)))), chaves.filter(([, v]) => v.some(s => PROIBIDAS.some(p => p.test(s)))).map(([k]) => k).join());
    await page.evaluate(() => __t.setLang('pt'));
    // Teclado: a aba da Revisão pela barra; Enviar e as ações focáveis; a casa do lote pelo espaço.
    await page.evaluate(() => __t.closeReview());
    await page.evaluate(() => __t.openReview(__t.studio.turns.find(t => t.id === '135'), { focus: false }));
    await esperaRevisao(page, "__t.review.evidence && __t.review.facts");
    await page.click('#centerCity');
    await page.focus('#centerCity');
    let alvo = null;
    for (let k = 0; k < 8 && alvo !== 'centerReview'; k++) { await page.keyboard.press('Tab'); alvo = await page.evaluate(() => document.activeElement?.id); }
    ok('D teclado: a aba da Revisão se alcança pelo Tab na barra de vistas', alvo === 'centerReview', alvo);
    await page.keyboard.press('Enter');
    ok('D teclado: Enter na aba traz a Revisão à frente', await page.evaluate(() => __t.navigation.center) === 'review');
    await page.focus('#reviewSend');
    ok('D teclado: Enviar recebe foco', await page.evaluate(() => document.activeElement?.id) === 'reviewSend');
    const focoVisivel = await page.evaluate(() => { const e = document.querySelector('#reviewSend'); e.focus(); return getComputedStyle(e).outlineStyle; });
    ok('D teclado: o foco é visível em Enviar', focoVisivel !== 'none', focoVisivel);
  }
  });
  await bloco(async () => {
  x = await pagina(browser, { size: { width: 600, height: 900 } });
  {
    const page = x.page;
    await page.evaluate(() => { document.querySelector('#colSwitch [data-col="question"]').click(); __t.activate('135', false); });
    await page.waitForTimeout(300);
    await page.evaluate(() => document.querySelector('#turns .turn[data-turn="135"] [data-action="review"]').click());
    await esperaRevisao(page, "__t.review.evidence && __t.review.facts");
    const tela = await page.evaluate(() => ({ col: document.querySelector('#app').dataset.col, label: document.querySelector('#colSwitch [data-col="center"]').textContent,
      announce: document.querySelector('#announce').textContent, scroll: document.documentElement.scrollWidth - document.documentElement.clientWidth, shown: document.querySelector('#reviewView').offsetParent !== null }));
    ok('D tela estreita: "Ver revisão" leva à coluna do centro, anunciada', tela.col === 'center' && tela.shown && tela.announce.length > 0, JSON.stringify(tela));
    ok('D tela estreita: o alternador diz "Centro: Revisão #135"', tela.label === await T(page, 'col.review', { what: await T(page, 'review.tabTurn', { id: '135' }) }), tela.label);
    ok('D tela estreita: sem rolagem horizontal', tela.scroll <= 0, tela.scroll);
    // Lote na tela estreita, com o teclado: a casa de um item muda com o espaço.
    let criadas = 0;
    x.fake['POST /api/gathered-question'] = rr => { criadas++; return rr.fulfill({ json: falsa(T135, 900500 + criadas) }); };
    x.fake['GET /api/evidence'] = (rr, _, url) => (url.searchParams.get('turn').startsWith('9005') ? rr.fulfill({ json: { ...structuredClone(ev135), turn: url.searchParams.get('turn') } }) : false);
    const simbolos = await page.evaluate(() => __t.hot.rows.filter(r => r.symbol).slice(0, 2).map(r => r.symbol));
    await page.evaluate(s => __t.askFacts(s.map((symbol, k) => ({ symbol, question: `Fato ${k + 1}?`, summary: `fato ${k + 1}` }))), simbolos);
    await esperaRevisao(page, "__t.review.kind === 'lot' && !__t.review.creating && __t.review.items.every(i => i.evidence)");
    await page.focus('#reviewBody .rv-item[data-item="1"] input');
    await page.keyboard.press('Space');
    ok('D teclado: espaço desmarca um item do lote', (await itens(page))[0].box.checked === false && (await rv(page)).lotSend.text === await T(page, 'lot.send', { k: 1, n: 2 }));
    ok('D tela estreita: o lote sem rolagem horizontal', await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth) <= 0);
  }
  });

  // ================================================================ fim
  ok('fim nenhum erro de página', !todosErros.length, todosErros.slice(0, 3).join(' | '));
  ok('fim nenhuma escrita escapou para o servidor (todas respondidas aqui)', !todasEscritas.length, [...new Set(todasEscritas)].join());
  ok('fim o perfil do projeto está igual em bytes', sha(fs.readFileSync(perfil)) === perfilAntes);
  await browser.close();
})().catch(e => { console.error(e); ok('a bateria terminou sem exceção', false, e.message); process.exit(1); });

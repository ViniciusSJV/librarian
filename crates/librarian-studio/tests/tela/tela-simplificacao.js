// A simplificação de 17/11/2026, na página real, num navegador sem janela, contra o servidor real e o projeto
// renderer: quem responde é um perfil da biblioteca de modelos (sem Modelfile, escada ou skills no compositor);
// toda pergunta só é preparada e abre na vista Pacote, que mostra o pacote, as conferências H1 a H4, a tarefa e o
// idioma; Nenhum não envia; a Seleção e o Pacote com os nomes novos, em PT e EN; a escada oferecida só no Pacote,
// sobre o mesmo pacote, no modo Melhorar; a biblioteca só leitura. Nada chega a um modelo: /api/send é abortado e
// /api/ladder é respondido pela bateria, ambos contados. A raiz é montada em LIBRARIAN_TESTE_SAIDA com a página, os
// modelos e as skills deste repositório, e o .studio dela começa vazio (a bateria aprova nele só o comando
// `compilar`, que nada executa). Desde 18/11, o clique na escada é conferido: um único /api/ladder, sobre o pacote
// revisado, sem rodadas automáticas.
const path = require('path');
const { spawn } = require('child_process');
const fs = require('fs');
const pede = nome => { const v = process.env[nome]; if (!v) { console.error(`Falta a variável de ambiente ${nome}: veja o README.md desta pasta`); process.exit(2); } return v; };
const { chromium } = require(process.env.LIBRARIAN_TESTE_PLAYWRIGHT || 'playwright-core');
const exe = pede('LIBRARIAN_TESTE_STUDIO'), renderer = pede('LIBRARIAN_TESTE_RENDERER'), saida = pede('LIBRARIAN_TESTE_SAIDA');
const repo = path.join(__dirname, '../../../..');
const root = path.join(saida, 'raiz-simplificacao'), shots = path.join(saida, 'simplificacao');
fs.rmSync(root, { recursive: true, force: true });
fs.mkdirSync(path.join(root, 'crates/librarian-studio/web'), { recursive: true });
fs.copyFileSync(path.join(repo, 'crates/librarian-studio/web/index.html'), path.join(root, 'crates/librarian-studio/web/index.html'));
for (const dir of ['modelfiles', 'skills']) fs.cpSync(path.join(repo, dir), path.join(root, dir), { recursive: true });
const PORT = 8843;
let failures = 0;
const ok = (name, cond, extra = '') => { console.log(`${cond ? 'ok  ' : 'FALHA'} ${name}${cond ? '' : ' :: ' + extra}`); if (!cond) failures++; };

(async () => {
  fs.mkdirSync(shots, { recursive: true });
  const server = spawn(exe, ['--port', String(PORT)], { env: { ...process.env, LIBRARIAN_ROOT: root }, stdio: 'ignore' });
  for (let i = 0; i < 50; i++) { try { const r = await fetch(`http://127.0.0.1:${PORT}/api/status`); await r.text(); if (r.ok) break; } catch {} await new Promise(r => setTimeout(r, 200)); }
  // Chrome, como as outras baterias; LIBRARIAN_TESTE_NAVEGADOR aponta outro executável do Chromium (o Edge, por exemplo).
  const browser = await chromium.launch(process.env.LIBRARIAN_TESTE_NAVEGADOR ? { executablePath: process.env.LIBRARIAN_TESTE_NAVEGADOR, headless: true } : { channel: 'chrome', headless: true });
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [], blocked = [];
  page.on('console', m => { if (m.type() === 'error' && !/favicon/.test(m.text())) errors.push(m.text()); });
  page.on('pageerror', e => errors.push(String(e)));
  const failed = [];
  page.on('response', r => { if (r.status() >= 400) failed.push(`${r.status()} ${new URL(r.url()).pathname}`); });
  // /api/ladder (18/11): the page's body is kept and answered here, so the click is seen and nothing climbs.
  const ladderBodies = [];
  await page.route(/\/api\/send$/, r => { blocked.push(r.request().url()); r.abort(); });
  await page.route(/\/api\/ladder$/, r => {
    blocked.push(r.request().url());
    const body = JSON.parse(r.request().postData() || '{}');
    ladderBodies.push(body);
    r.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ passed: false, turn: body.turn, nodes: [], steps: [] }) });
  });
  try {
    // One gate command approved in this root (only its .studio), so the ladder can be clicked; nothing runs it.
    const api = async (method, url, body) => (await fetch(`http://127.0.0.1:${PORT}${url}`, { method, headers: { 'content-type': 'application/json' }, body: body && JSON.stringify(body) })).json();
    const { key } = await api('POST', '/api/project', { root: renderer });
    const compilar = (await api('GET', `/api/commands?key=${encodeURIComponent(key)}`)).commands.find(c => c.name === 'compilar');
    await api('POST', '/api/commands/approve', { key, name: 'compilar', sha256: compilar.sha256 });
    await page.goto(`http://127.0.0.1:${PORT}/`);
    await page.waitForFunction(() => document.body.classList.contains('studio'));
    // The composer: who answers is a profile of the library; no Modelfile, no ladder, no skills.
    const providers = await page.$$eval('#provider option', o => o.map(x => [x.value, x.textContent]));
    ok('composer offers Nenhum and the two profiles of the library', JSON.stringify(providers.map(p => p[0])) === '["manual","qwen3coder","claude"]', JSON.stringify(providers));
    ok('profile labels come from the library', providers.some(p => p[1] === 'Local: Qwen3-Coder') && providers.some(p => p[1] === 'Remoto: Claude'), JSON.stringify(providers));
    for (const id of ['#modelfile', '#ladderOn', '#skillsRow', '#editModelfile', '#mfSave', '#mfNew'])
      ok(`no ${id} on the page`, (await page.$(id)) === null);
    // Open the renderer and generate an edition.
    await page.fill('#projectPath', renderer);
    await page.click('#openProject');
    await page.waitForFunction(() => !document.querySelector('#projectBody').hidden, null, { timeout: 30000 });
    await page.click('#generate');
    await page.waitForFunction(() => !document.querySelector('#question').disabled, null, { timeout: 180000 });
    // Settings: the language is decided in one place.
    await page.click('[data-area="settings"]');
    const languages = await page.$$eval('#language option', o => o.map(x => x.value));
    ok('settings has the project language (pt, en)', JSON.stringify(languages) === '["pt","en"]', JSON.stringify(languages));
    // The model library, read only.
    await page.click('[data-area="modelfiles"]');
    await page.click('#toolModelfiles');
    const names = await page.$$eval('#mfSelect option', o => o.map(x => x.value));
    ok('the library lists the rendered Modelfiles', names.includes('verificavel-qwen3coder') && names.includes('performance-claude'), names.join(','));
    ok('no gemma4 nor gpt-oss', names.every(n => !/gemma|gptoss/.test(n)), names.join(','));
    ok('the Modelfile text is read only', await page.$eval('#mfContent', e => e.readOnly && e.value.includes('SYSTEM """')));
    ok('the library says the defaults of each action', (await page.textContent('#mfLibrary')).includes('ladder: Remoto: Claude · haiku'), await page.textContent('#mfLibrary'));
    await page.screenshot({ path: `${shots}/1-modelos.png` });
    await page.click('#modelfileDialog [data-close]');
    // A free question only prepares: the package opens in the Package view, and nothing is sent.
    await page.fill('#question', 'Onde a câmera calcula a direção dos raios?');
    ok('the composer button prepares', /Preparar/.test(await page.textContent('#ask')), await page.textContent('#ask'));
    await page.click('#ask');
    await page.waitForFunction(() => !document.querySelector('#reviewView').hidden, null, { timeout: 60000 });
    ok('the Package view opens', /Pacote da pergunta #\d+/.test(await page.textContent('#reviewTitle')), await page.textContent('#reviewTitle'));
    await page.waitForFunction(() => document.querySelector('[data-block="checks"]')?.textContent.includes('H1'), null, { timeout: 30000 });
    const checks = await page.textContent('[data-block="checks"]');
    ok('the Package view shows H1 to H4', /H1/.test(checks) && /H4/.test(checks), checks.slice(0, 300));
    ok('the Package view shows the package and its provenance', /normalização .* código/.test(await page.textContent('[data-block="package"]')), await page.textContent('[data-block="package"]'));
    ok('the chosen block says the task and the language', /tarefa: responder em claims · idioma: Português/.test(await page.textContent('[data-block="chosen"]')), await page.textContent('[data-block="chosen"]'));
    ok('no ladder for a question that only locates', (await page.$('#reviewLadder')) === null);
    ok('nothing was sent', blocked.length === 0, blocked.join(','));
    await page.screenshot({ path: `${shots}/2-pacote.png` });
    // The conversation: "Ver pacote", no send of its own, no evidence section of its own.
    const turnButtons = await page.$$eval('#turns .turn button', b => b.map(x => x.textContent));
    ok('the question has "Ver pacote"', turnButtons.includes('Ver pacote'), turnButtons.join('|'));
    ok('the conversation sends nothing by itself', !turnButtons.some(t => /Enviar/.test(t)), turnButtons.join('|'));
    ok('no evidence section left in the conversation', (await page.$('#turns details.evidence')) === null);
    // Nenhum: shows the query to copy; no request is made.
    await page.selectOption('#provider', 'manual');
    await page.click('#reviewSend');
    await page.waitForTimeout(500);
    ok('Nenhum sends nothing', blocked.length === 0, blocked.join(','));
    // The local profile: the send is the Package view's, with the profile (aborted here).
    await page.selectOption('#provider', 'claude');
    const sendText = await page.textContent('#reviewSend');
    ok('the send names the profile', /Remoto: Claude/.test(sendText), sendText);
    // The Selection: join a declaration from the search, prepare, and the Package view opens again.
    await page.click('[data-area="explore"]');
    await page.fill('#query', 'ray_from_pixel');
    await page.waitForSelector('#results li', { timeout: 10000 });
    await page.click('#results li');
    await page.waitForSelector('#inspector button', { timeout: 10000 });
    const join = await page.$$('#inspector button');
    let joined = false;
    for (const b of join) if (/Juntar ao rascunho/.test(await b.textContent())) { await b.click(); joined = true; break; }
    ok('the inspection joins the draft', joined);
    await page.waitForFunction(() => !document.querySelector('#builderView').hidden, null, { timeout: 10000 }).catch(() => {});
    if (await page.isHidden('#builderView')) await page.click('#centerBuilder').catch(() => {});
    ok('the Selection view is named Seleção', (await page.textContent('#builderTitle')).trim() === 'Seleção', await page.textContent('#builderTitle'));
    ok('the tab says Seleção', /Seleção/.test(await page.textContent('#centerBuilderTitle')), await page.textContent('#centerBuilderTitle'));
    await page.screenshot({ path: `${shots}/3-selecao.png` });
    await page.fill('#question', 'O que esta função calcula?');
    ok('the composer prepares the selection', /Preparar \(1 fonte/.test(await page.textContent('#ask')), await page.textContent('#ask'));
    await page.click('#ask');
    await page.waitForFunction(() => !document.querySelector('#reviewView').hidden && /Pacote da pergunta/.test(document.querySelector('#reviewTitle').textContent), null, { timeout: 60000 });
    ok('the Package view opens for the selection', true);
    ok('still nothing sent', blocked.length === 0, blocked.join(','));
    ok('Localizar: no ladder in the Package view', (await page.$('#reviewLadder')) === null);
    // Melhorar: the ladder is offered on this same package, in the Package view (never from the composer).
    await page.click('#modeSeg [data-mode="improve"]');
    await page.click('#askOk');
    await page.waitForFunction(() => document.querySelector('#modeSeg [data-mode="improve"]').getAttribute('aria-pressed') === 'true', null, { timeout: 10000 });
    await page.fill('#question', 'Como deixar esta função mais rápida?');
    await page.click('#ask');
    await page.waitForFunction(() => document.querySelector('#reviewLadder'), null, { timeout: 60000 });
    // The package read (H1 on screen), so the button's state is not the one of "loading".
    await page.waitForFunction(() => document.querySelector('[data-block="checks"]')?.textContent.includes('H1'), null, { timeout: 30000 });
    const ladder = await page.$eval('#reviewLadder', b => ({ text: b.textContent, disabled: b.disabled, title: b.title }));
    ok('Melhorar: the Package view offers the ladder with the library model', /escada · Remoto: Claude · haiku/.test(ladder.text), JSON.stringify(ladder));
    ok('the ladder waits for an approved gate command', ladder.disabled === ((await page.textContent('#reviewBody')).includes('nenhum comando aprovado')), JSON.stringify(ladder));
    ok('the chosen block says the change task', /tarefa: propor mudança/.test(await page.textContent('[data-block="chosen"]')), await page.textContent('[data-block="chosen"]'));
    ok('and still nothing sent', blocked.length === 0, blocked.join(','));
    await page.screenshot({ path: `${shots}/5-melhorar.png` });
    // The ladder, clicked: one request, on the reviewed package (turn), with no automatic round (18/11).
    const reviewed = (await page.textContent('#reviewTitle')).match(/#(\d+)/)?.[1];
    ok('the ladder is enabled with an approved gate command', !(await page.$eval('#reviewLadder', b => b.disabled)));
    await page.click('#reviewLadder');
    await page.waitForFunction(() => !document.querySelector('#ladderLog').hidden, null, { timeout: 30000 });
    await page.waitForTimeout(500);
    const sent = ladderBodies[0] ?? {};
    ok('the ladder makes one request', ladderBodies.length === 1, JSON.stringify(ladderBodies));
    ok('the ladder climbs on the reviewed package', typeof sent.turn === 'string' && Number(sent.turn) === Number(reviewed),`${JSON.stringify(sent)} reviewed=${reviewed}`);
    ok('the ladder asks for no automatic round', sent.retries === 0 && sent.lacuna_rounds === 0, JSON.stringify(sent));
    ok('the ladder sends no question of its own', ['question', 'nodes', 'origin', 'with_facts', 'models'].every(k => !(k in sent)), JSON.stringify(sent));
    ok('the gate is the approved command', JSON.stringify(sent.gate) === '["compilar"]', JSON.stringify(sent));
    ok('no send besides the ladder', blocked.filter(u => !/\/api\/ladder$/.test(u)).length === 0, blocked.join(','));
    // English.
    await page.click('[data-lang="en"]');
    ok('EN: the Package view title', /Package of question/.test(await page.textContent('#reviewTitle')), await page.textContent('#reviewTitle'));
    ok('EN: the Selection title', (await page.textContent('#builderTitle')).trim() === 'Selection', await page.textContent('#builderTitle'));
    await page.screenshot({ path: `${shots}/4-en.png` });
  } catch (e) {
    ok('scenario ran to the end', false, e.stack);
    await page.screenshot({ path: `${shots}/erro.png` }).catch(() => {});
  } finally {
    // A 404 of /api/hotspots is "no measure yet", which the page says as such: the only failed request expected.
    const unexpected = failed.filter(f => f !== '404 /api/hotspots');
    ok('no failed request but the expected one', unexpected.length === 0, failed.join(', '));
    ok('no error in the console but that 404', errors.filter(e => !/status of 404/.test(e)).length === 0, errors.join('\n'));
    await browser.close();
    server.kill();
    console.log(failures ? `${failures} FALHA(S)` : 'tudo ok');
    process.exit(failures ? 1 : 0);
  }
})();

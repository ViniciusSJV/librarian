// O cartão de desempenho na tela real: cada estado da rota diz de onde veio a resposta, e nada mais.
// Nas cópias do renderer (com medição) e do projeto Python (sem). Nada é enviado nem executado.
const { chromium, RENDERER, PYTHON } = require('./ambiente');
const HOOK = "window.__t = { state, studio, t, loadRun, showTab, askPerf, get hot() { return hot; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d !== '' ? '  -> ' + d : ''}`); if (!pass) process.exitCode = 1; };
const MEDIDA = 'Respondido pela medição, sem nenhum modelo.';
const AMBIGUO = 'É preciso escolher a função: a medição não foi consultada, e nenhum modelo foi usado.';
const NAO_ACHOU = 'Nenhuma função corresponde ao nome pedido: a medição não foi consultada, e nenhum modelo foi usado.';
const TODAS = [MEDIDA, AMBIGUO, NAO_ACHOU];
async function open(browser, B, project, run, log) {
  const page = await (await browser.newContext({ viewport: { width: 1500, height: 1000 } })).newPage();
  page.on('pageerror', e => log.errors.push(e.message));
  await page.route(/\/api\/(send|ladder|generate|instrument\/run|commands\/run|commands\/approve|paste|review)\b/, r => { log.sent++; return r.abort(); });
  await page.route(`${B}/`, async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  await page.fill('#projectPath', project);
  await page.click('#openProject');
  await page.waitForFunction(() => __t.studio.project, null, { timeout: 60000 });
  await page.evaluate(x => __t.loadRun(x), run);
  await page.waitForFunction(x => __t.studio.run === x && __t.studio.measureAll, run, { timeout: 120000 });
  await page.waitForTimeout(800);
  return page;
}
// A pergunta na página real, o que a API respondeu e o que o cartão mostra.
const card = (page, question) => page.evaluate(async q => {
  let api = null; const before = window.fetch;
  window.fetch = async (...a) => { const r = await before(...a); if (String(a[0]).includes('/api/performance-question')) api = await r.clone().json(); return r; };
  const shown = await __t.askPerf(q);
  window.fetch = before;
  const cards = [...document.querySelectorAll('.perf-card')], last = cards.at(-1);
  return { shown, api, text: shown === false ? '' : last?.innerText ?? '', lines: shown === false ? [] : [...(last?.querySelectorAll('div.muted') ?? [])].map(d => d.textContent.trim()), buttons: shown === false ? [] : [...(last?.querySelectorAll('button') ?? [])].map(b => b.textContent.trim()) };
}, question);
const only = (c, want) => { const seen = TODAS.filter(t => c.lines.some(l => l.startsWith(t))); return want === null ? seen.length === 0 : seen.length === 1 && seen[0] === want; };
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const log = { errors: [], sent: 0 };
  const rs = await open(browser, 'http://127.0.0.1:8841', RENDERER, '20261104-020244', log);
  await rs.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  const CASOS = [
    ['resolvido, com contagem', 'Otimize o Tuple::dot', 'resolved', true, MEDIDA],
    ['resolvido, sem contagem: a medição foi consultada', 'Otimize o Stripe::default', 'resolved', true, MEDIDA],
    ['resolvido, zero na entrada', 'Otimize o Camera::new', 'resolved', true, MEDIDA],
    ['ambíguo', 'Otimize default', 'ambiguous', false, AMBIGUO],
    ['ambíguo, nomes idênticos', 'Otimize o main', 'ambiguous', false, AMBIGUO],
    ['ambíguo, por "onde"', 'Onde melhorar o default?', 'ambiguous', false, AMBIGUO],
    ['não encontrado', 'Otimize o Inexistente::default', 'not_found', false, NAO_ACHOU],
    ['dois nomes, um não existe', 'Otimize Tuple::dot e Cube::nada', 'not_found', false, NAO_ACHOU],
    ['dois nomes que existem', 'Otimize Tuple::dot e Tuple::cross', 'ambiguous', false, AMBIGUO],
    ['sem alvo: quais pesam', 'Quais funções pesam?', 'none', true, MEDIDA],
    ['sem alvo: onde otimizar', 'Onde dá para otimizar?', 'none', true, MEDIDA],
    // O nome é ambíguo, mas a pergunta é sobre a medição inteira: quem respondeu foi a medição.
    ['nome ambíguo numa pergunta sobre a medição', 'Quais default pesam?', 'ambiguous', true, MEDIDA],
  ];
  for (const [nome, q, status, medida, linha] of CASOS) {
    const c = await card(rs, q);
    ok(`${nome}: estado e origem na API`, c.api?.route.target.status === status && Boolean(c.api?.execution) === medida, `${c.api?.route.target.status}, execution=${c.api?.execution ?? null}`);
    ok(`${nome}: o cartão diz só a origem certa`, only(c, linha), c.lines.filter(l => TODAS.some(t => l.startsWith(t))).join(' | '));
    if (!medida) ok(`${nome}: sem fato nem mudança, e o modelo segue à escolha da pessoa`, !c.api.facts.length && !c.api.changes.length && c.buttons.some(b => b.includes('Perguntar ao modelo')), c.buttons.join(', '));
  }
  const semIntencao = await card(rs, 'Bom dia');
  ok('sem intenção: nenhum cartão, a pergunta segue ao modelo', semIntencao.shown === false && semIntencao.api.route.target.status === 'none');
  await rs.context().close();
  // Projeto sem medição: nada pode dizer que a medição respondeu.
  const py = await open(browser, 'http://127.0.0.1:8842', PYTHON, '20261029-053623', log);
  const SEM = [
    ['sem medição: resolvido pede medição', 'Otimize o run_migrations_online', 'resolved', null, true],
    ['sem medição: quais pesam pede medição', 'Quais funções pesam?', 'none', null, true],
    ['sem medição: ambíguo pede escolha', 'Otimize upgrade', 'ambiguous', AMBIGUO, null],
    ['sem medição: não encontrado', 'Otimize o Inexistente.upgrade', 'not_found', NAO_ACHOU, null],
  ];
  for (const [nome, q, status, linha, pede] of SEM) {
    const c = await card(py, q);
    ok(nome, c.api?.route.target.status === status && !c.api.execution && only(c, linha) && (pede === null ? !c.api.needs_measure : c.api.needs_measure === true), `${c.api?.route.target.status}: ${c.lines.filter(l => TODAS.some(t => l.startsWith(t))).join(' | ') || 'sem linha de origem'}`);
  }
  await py.screenshot({ path: process.argv[2] + '/cartao-py.png' });
  ok('nenhum envio', log.sent === 0);
  ok('sem erro de página', log.errors.length === 0, log.errors.join(' | ').slice(0, 200));
  await browser.close();
})().catch(e => { console.error(e); process.exit(2); });

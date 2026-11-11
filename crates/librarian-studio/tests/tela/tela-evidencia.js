// Etapa 4, Passo 14 (B9): o pacote de evidências e as conferências de cada pergunta, na página real,
// contra o que GET /api/evidence diz: português e inglês, tema escuro, tela estreita, declared_by visível.
// Nada é enviado: /api/send, /api/ladder e as execuções são abortados. Cria uma pergunta na raiz (como etapa2c).
const { chromium, RENDERER, CHAVE_RENDERER } = require('./ambiente');
const OUT = process.argv[2];
const B = 'http://127.0.0.1:8841', run = '20261104-020244';
const HOOK = "window.__t = { state, studio, t, loadRun, renderTurns, toggleGathered, absorbSuggestion, selectionOnRecord, get gathered() { return gathered; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + d : ''}`); if (!pass) process.exitCode = 1; };
const get = async path => (await fetch(`${B}${path}`)).json();
const evidence = turn => get(`/api/evidence?key=${CHAVE_RENDERER}&run=${run}&turn=${turn}`);

const start = async (browser, errors, counter, { size = { width: 1500, height: 1000 }, scheme = 'light', lang = null } = {}) => {
  const page = await (await browser.newContext({ viewport: size, colorScheme: scheme })).newPage();
  page.on('pageerror', e => errors.push(e.message));
  await page.route(/\/api\/(send|ladder|generate|instrument\/run|commands\/run|commands\/approve|paste|review)\b/, r => { counter.sent++; return r.abort(); });
  await page.route(`${B}/`, async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  if (lang) await page.click(`[data-lang="${lang}"]`);
  await page.fill('#projectPath', RENDERER);
  await page.click('#openProject');
  await page.waitForFunction(() => __t.studio.project, null, { timeout: 60000 });
  await page.evaluate(x => __t.loadRun(x), run);
  await page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 60000 });
  return page;
};
// Abre a seção do pacote de uma pergunta, como a pessoa abre, e devolve o que ela mostra.
const open = async (page, turn) => {
  await page.evaluate(id => { const x = __t.studio.turns.find(t => t.id === id); x.expanded = true; __t.renderTurns(); }, turn);
  await page.evaluate(id => document.querySelector(`details.evidence[data-turn="${id}"] summary`).click(), turn);
  await page.waitForFunction(id => { const b = document.querySelector(`details.evidence[data-turn="${id}"] .evidence-body`); return b && b.textContent.includes(__t.t('evidence.recorded')); }, turn, { timeout: 60000 });
  return page.evaluate(id => {
    const d = document.querySelector(`details.evidence[data-turn="${id}"]`), body = d.querySelector('.evidence-body');
    const tags = [...body.querySelectorAll('.tag')].filter(x => x.textContent.startsWith('declared_by'));
    return { text: body.innerText, tags: tags.map(x => x.textContent), visible: tags.every(x => x.offsetParent !== null && getComputedStyle(x).visibility !== 'hidden'),
      wide: body.scrollWidth - body.clientWidth, page: document.documentElement.scrollWidth - innerWidth,
      colors: [getComputedStyle(document.body).backgroundColor, getComputedStyle(body).color] };
  }, turn);
};
// O que a seção precisa mostrar, nas palavras da página, a partir da resposta da API.
const expected = (page, data) => page.evaluate(d => {
  const lines = ['h1', 'h2', 'h3'].map(h => `${__t.t('evidence.' + h)}: ${__t.t('evidence.status.' + d.checks[h].status)}`);
  lines.push(`${__t.t('evidence.h4')}: ${d.h4 ? __t.t('evidence.h4.' + d.h4.status) : __t.t('evidence.h4.none')}`);
  lines.push(...d.warnings.map(w => __t.t('evidence.warning.' + w)), __t.t('evidence.h4Note'), __t.t('evidence.limits'));
  if (!d.package) lines.push(__t.t('evidence.noPackage'));
  return lines;
}, data);
const luminance = css => { const [r, g, b] = css.match(/\d+/g).map(Number).map(v => v / 255); return 0.2126 * r + 0.7152 * g + 0.0722 * b; };

(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const errors = [], counter = { sent: 0 };
  const turnsBefore = (await get(`/api/turns?key=${CHAVE_RENDERER}&run=${run}`)).length;

  // Uma pergunta com seleção, feita pelas funções da própria página: a fonte juntada à mão e a sugestão absorvida.
  const page = await start(browser, errors, counter);
  const made = await page.evaluate(async ([key, wanted]) => {
    const node = __t.state.nodes.find(n => n.category === 'symbol' && n.kind === 'function');
    __t.toggleGathered(node.id);
    const post = async (path, body) => (await fetch(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })).json();
    const suggested = await post('/api/envelope/suggest', { key, run: wanted, nodes: [node.id] });
    __t.absorbSuggestion(suggested, [node.id]);
    const turn = await post('/api/gathered-question', { key, run: wanted, question: 'Bateria do Passo 14: o que muda?', nodes: __t.gathered,
      origin: { kind: 'selection', summary: 'bateria', selection: __t.selectionOnRecord() } });
    __t.studio.turns = await (await fetch(`/api/turns?key=${key}&run=${wanted}`)).json();
    __t.renderTurns();
    return turn.id;
  }, [CHAVE_RENDERER, run]);
  ok('a pergunta com seleção foi criada', typeof made === 'string', made);
  const data = await evidence(made);
  ok('API: pacote e registro de seleção gravados', data.package && data.selection && data.package.selection, Object.keys(data).join(','));
  ok('API: H1 a H3 conferidos, H4 dito à parte', ['h1', 'h2', 'h3'].every(h => data.checks[h].status === 'passed') && ['reproduced', 'not_replayable'].includes(data.h4.status), JSON.stringify(data.checks));
  const divergence = data.package.measure.measure_divergence;
  ok('API: pergunta nova em formato 2, com a comparação das duas medições e sem o aviso do formato 1',
    data.package.format_version === 2 && data.mark.format_version === 2 && divergence && ['same', 'diverges', 'not_comparable', 'not_applicable'].includes(divergence.status)
      && !('measure_diverges' in data.package.measure) && data.warnings.length === 0, JSON.stringify(data.package.measure));
  // O texto da comparação, nas palavras da página, a partir do que a API diz.
  const divergenceText = (page, d) => page.evaluate(d => {
    const reasons = (d.reasons ?? []).map(r => `${__t.t('evidence.side.' + r.side)} ${__t.t('evidence.cause.' + r.read_status)}`).join('; ');
    return __t.t('evidence.divergence', { state: __t.t('evidence.divergence.' + d.status, { reasons }) });
  }, d);

  // Português, tema claro, tela larga.
  let shown = await open(page, made);
  let want = await expected(page, data);
  ok('PT: a tela diz o que a API diz (H1 a H4, aviso, notas)', want.every(l => shown.text.includes(l)), want.filter(l => !shown.text.includes(l)).join(' | '));
  ok('PT: declared_by de cada ficha, visível', shown.tags.filter(x => /^declared_by /.test(x)).length >= data.package.fichas.length && shown.visible
    && data.package.fichas.every(f => shown.tags.includes(`declared_by ${f.declared_by}`)), shown.tags.join(', '));
  ok('PT: a comparação das duas medições aparece como a API diz, e nada do formato 1', shown.text.includes(await divergenceText(page, divergence))
    && !shown.text.includes('measure_diverges') && !shown.text.includes(await page.evaluate(() => __t.t('evidence.warning.measure_diverges_not_computed'))), await divergenceText(page, divergence));
  ok('PT: a medição mais recente vem à parte da gravada', shown.text.includes(await page.evaluate(() => __t.t('evidence.latest', { latest: '' }).trim().replace(/:$/, ''))));
  ok('PT: nenhum "aprovado": a tela diz o que foi conferido, não mais', !/aprovad/i.test(shown.text), shown.text.match(/.{0,40}aprovad.{0,40}/i)?.[0]);
  ok('consultar não muda o que a API diz', JSON.stringify(await evidence(made)) === JSON.stringify(data));

  // Uma pergunta anterior às marcas, se a raiz tiver: sem pacote, sem conferência, H4 não se aplica.
  const turns = await get(`/api/turns?key=${CHAVE_RENDERER}&run=${run}`);
  let older = null;
  for (const t of turns) { const e = await evidence(t.id); if (e.mark === null) { older = [t.id, e]; break; } }
  if (older) {
    const [id, e] = older;
    const s = await open(page, id);
    const w = await expected(page, e);
    ok('pergunta antiga: sem pacote, nada a conferir, H4 não se aplica', w.every(l => s.text.includes(l)) && e.checks.h1.status === 'not_applicable' && e.h4.status === 'not_applicable', w.filter(l => !s.text.includes(l)).join(' | '));
  } else {
    console.log('INFO  nenhuma pergunta anterior às marcas nesta raiz: caso coberto pelos testes da API');
  }

  // Um pacote formato 1 já gravado na raiz (por baterias anteriores à P1): o valor como gravado, com o aviso.
  let formatOne = null;
  for (const t of turns.slice().reverse()) { const e = await evidence(t.id); if (e.package?.format_version === 1) { formatOne = [t.id, e]; break; } }
  if (formatOne) {
    const [id, e] = formatOne;
    const s = await open(page, id);
    const w = await expected(page, e);
    ok('formato 1: H1 a H3 conferidos agora, o aviso de measure_diverges e o valor como gravado',
      ['h1', 'h2', 'h3'].every(h => e.checks[h].status === 'passed') && e.warnings.includes('measure_diverges_not_computed') && e.package.measure.measure_diverges === false
        && !('measure_divergence' in e.package.measure) && w.every(l => s.text.includes(l)) && s.text.includes('measure_diverges') && s.text.includes('false')
        && !s.text.includes(await page.evaluate(() => __t.t('evidence.divergence', { state: '' }).split(':')[0])), `${id} ${w.filter(l => !s.text.includes(l)).join(' | ')}`);
  } else {
    console.log('INFO  nenhum pacote formato 1 nesta raiz: caso coberto pela fixture congelada nos testes da API');
  }

  // Inglês.
  const en = await start(browser, errors, counter, { lang: 'en' });
  shown = await open(en, made);
  want = await expected(en, data);
  ok('EN: the page says what the API says', want.every(l => shown.text.includes(l)), want.filter(l => !shown.text.includes(l)).join(' | '));
  ok('EN: no "approved": the page says what was checked, no more', !/approved/i.test(shown.text));
  ok('EN: textos em inglês, não em português', !shown.text.includes('conferido') && shown.text.includes('checked'));
  ok('EN: the two measures compared, as the API says', shown.text.includes(await divergenceText(en, divergence)) && /measures compared/.test(shown.text), await divergenceText(en, divergence));
  // Cada estado e cada motivo tem texto próprio nos dois idiomas, e nenhum vira a chave crua.
  const keys = ['same', 'diverges', 'not_comparable', 'not_applicable'].map(s => 'evidence.divergence.' + s)
    .concat(['evidence.side.at_question', 'evidence.side.selection_basis', 'evidence.cause.unreadable', 'evidence.cause.not_recorded']);
  const texts = async p => p.evaluate(keys => keys.map(k => __t.t(k, { reasons: 'X' })), keys);
  const [pt, english] = [await texts(page), await texts(en)];
  ok('PT e EN: cada estado e motivo da comparação tem texto, diferente entre os idiomas', pt.every((x, i) => x !== keys[i] && english[i] !== keys[i] && x !== english[i])
    && new Set(pt).size === keys.length, JSON.stringify([pt, english]));
  if (formatOne) {
    const s = await open(en, formatOne[0]);
    ok('EN: format 1 with its warning', s.text.includes(await en.evaluate(() => __t.t('evidence.warning.measure_diverges_not_computed'))) && s.text.includes('measure_diverges recorded: false'));
  }

  // Tema escuro.
  const dark = await start(browser, errors, counter, { scheme: 'dark' });
  shown = await open(dark, made);
  const [bg, fg] = shown.colors.map(luminance);
  ok('escuro: fundo escuro, texto claro', bg < 0.2 && fg > 0.6, shown.colors.join(' / '));
  ok('escuro: declared_by visível', shown.visible && shown.tags.length >= data.package.fichas.length);

  // Tela estreita.
  const narrow = await start(browser, errors, counter, { size: { width: 390, height: 844 }, scheme: 'dark' });
  shown = await open(narrow, made);
  ok('estreita: sem rolagem horizontal na seção nem na página', shown.wide <= 1 && shown.page <= 1, `seção ${shown.wide}, página ${shown.page}`);
  ok('estreita: declared_by visível', shown.visible && shown.tags.length >= data.package.fichas.length);
  if (OUT) await narrow.screenshot({ path: `${OUT}/tela-evidencia-estreita.png`, fullPage: false });

  ok('nada enviado', counter.sent === 0, String(counter.sent));
  ok('uma pergunta a mais, a da bateria', (await get(`/api/turns?key=${CHAVE_RENDERER}&run=${run}`)).length === turnsBefore + 1);
  ok('sem erros na página', errors.length === 0, errors.join(' | '));
  await browser.close();
})().catch(e => { console.error(e); process.exit(1); });

// Passo 4, entrega 4A: os estados de cobertura em Onde pesa, na página real, contra o que a API diz.
// Nada é enviado: /api/send, /api/ladder e as execuções são abortados.
const { chromium, RENDERER, PYTHON, CHAVE_RENDERER } = require('./ambiente');
const OUT = process.argv[2];
const B = 'http://127.0.0.1:8841', P = 'http://127.0.0.1:8842', run = '20261104-020244', runPy = '20261029-053623';
const HOOK = "window.__t = { state, studio, t, showTab, loadRun, renderHot, get hot() { return hot; }, set hot(v) { hot = v; }, get hotStatus() { return hotStatus; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + d : ''}`); if (!pass) process.exitCode = 1; };
const ZERO = { entry_zero: 'a instrução de entrada contou zero', no_line: 'recebeu a instrução, e nenhuma linha foi impressa para ela', own_zero: 'o contador do projeto contou zero',
  several_zero: 'as linhas contadas ficaram em zero', unknown_zero: 'contagem zero de origem desconhecida' };
const REASON = { test: 'teste', skipped_path: 'caminho deixado de fora no perfil', no_body: 'sem corpo', const_fn: 'const fn, que não recebe contador',
  already_counts: 'o contador aparece no texto, e nenhuma contagem foi impressa', language_not_supported: 'a linguagem não recebe contador',
  shared_line: 'divide a linha com outra função: as passagens não podem ser atribuídas a nenhuma das duas', not_recorded: 'medição antiga: o motivo não foi guardado', not_classified: 'o registro não classifica esta função' };
const NO_PASSES = '#hotList-counted_no_passes', NO_COVERAGE = '#hotList-no_coverage';
const line = (f, why) => `${f.name} ${f.path} · ${why}`;
const sorted = list => [...list].sort();
const same = (a, b) => JSON.stringify(sorted(a)) === JSON.stringify(sorted(b));
const open = async (page, list) => { await page.evaluate(s => { document.querySelector(s).open = true; }, list); await page.waitForTimeout(100); };
const items = (page, list) => page.evaluate(s => [...document.querySelectorAll(`${s} li`)].map(li => li.textContent), list);
const start = async (browser, base, project, wanted, errors, counter, size = { width: 1500, height: 1000 }, scheme = 'light', lang = null) => {
  const page = await (await browser.newContext({ viewport: size, colorScheme: scheme })).newPage();
  page.on('pageerror', e => errors.push(e.message));
  await page.route(/\/api\/(send|ladder|generate|instrument\/run|commands\/run|commands\/approve|paste|review)\b/, r => { counter.sent++; return r.abort(); });
  await page.route(`${base}/`, async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.goto(`${base}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  if (lang) await page.click(`[data-lang="${lang}"]`);
  await page.fill('#projectPath', project);
  await page.click('#openProject');
  await page.waitForFunction(() => __t.studio.project, null, { timeout: 60000 });
  await page.evaluate(x => __t.loadRun(x), wanted);
  return page;
};
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const errors = [], counter = { sent: 0 };
  const api = await (await fetch(`${B}/api/hotspots?key=${CHAVE_RENDERER}&run=${run}`)).json();
  const functions = api.coverage.functions;
  const page = await start(browser, B, RENDERER, run, errors, counter);
  await page.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await page.waitForTimeout(500);
  await page.evaluate(() => __t.showTab('inventory'));

  // O ranking: as mesmas linhas, identificado como Top 20, sem se dizer inventário.
  const ranking = await page.evaluate(() => { const h = [...document.querySelectorAll('#hotList h3')].map(x => x.textContent);
    return { title: h.find(x => /Maior contagem observada/.test(x)), note: document.querySelector('#hotRankingNote')?.textContent ?? '', rows: [...document.querySelectorAll('#hotList ol.claims > li')].length }; });
  ok('o título identifica o Top 20 e mantém o critério', /^Top 20 · Maior contagem observada, com o tempo instrumentado como referência$/.test(ranking.title ?? ''), ranking.title);
  ok('a nota diz que o ranking não são todas as funções com passagens', /não todas as funções com passagens observadas/.test(ranking.note), ranking.note);
  ok('o ranking segue com 20 linhas', ranking.rows === 20, String(ranking.rows));

  // As listas existem, recolhidas, sem quantidades.
  const summaries = await page.evaluate(s => s.map(x => { const d = document.querySelector(x); return d && { open: d.open, text: d.querySelector('summary').textContent }; }), [NO_PASSES, NO_COVERAGE]);
  ok('as duas listas existem e começam recolhidas', summaries.every(x => x && !x.open), JSON.stringify(summaries));
  ok('os títulos são os aprovados', summaries[0]?.text === 'Contadas sem passagens observadas' && summaries[1]?.text === 'Sem cobertura', JSON.stringify(summaries));
  ok('nenhum título traz quantidade', summaries.every(x => x && !/\d/.test(x.text)));
  const collapsed = await page.evaluate(() => document.querySelector('#hotList').innerText);
  ok('recolhidas, o conteúdo não aparece no texto da tela', !collapsed.includes('nenhuma linha foi impressa'));
  const order = await page.evaluate(([a, b]) => { const box = document.querySelector('#hotList'), kids = [...box.children]; const at = q => kids.indexOf(box.querySelector(q)); return [at('ol.claims'), at(a), at(b)]; }, [NO_PASSES, NO_COVERAGE]);
  ok('a ordem é ranking, sem passagens, sem cobertura', order[0] >= 0 && order[0] < order[1] && order[1] < order[2], order.join());

  // Abertas: os estados reais, comparados com a resposta da API.
  await open(page, NO_PASSES);
  await open(page, NO_COVERAGE);
  const noPasses = await items(page, NO_PASSES), noCoverage = await items(page, NO_COVERAGE);
  const wantPasses = functions.filter(f => f.coverage_state === 'counted_no_passes'), wantCoverage = functions.filter(f => f.coverage_state === 'no_coverage');
  ok('sem passagens: exatamente as da API, com o motivo de zero_evidence', wantPasses.length > 0 && same(noPasses, wantPasses.map(f => line(f, ZERO[f.zero_evidence]))), `${noPasses.length} de ${wantPasses.length}`);
  ok('sem cobertura: exatamente as da API, com o motivo de exclusion_reason', wantCoverage.length > 0 && same(noCoverage, wantCoverage.map(f => line(f, REASON[f.exclusion_reason]))), `${noCoverage.length} de ${wantCoverage.length}`);
  ok('quem teve passagens não entra em nenhuma das listas', !functions.filter(f => f.coverage_state === 'counted_and_ran').some(f => [...noPasses, ...noCoverage].some(x => x.startsWith(`${f.name} ${f.path} `))));
  ok('nesta rodada antiga, o motivo é not_recorded e nenhum é inventado', noCoverage.every(x => x.endsWith(REASON.not_recorded)));
  const notes = await page.evaluate(s => s.map(x => document.querySelector(`${x} p`).textContent), [NO_PASSES, NO_COVERAGE]);
  ok('a nota de sem passagens é o texto aprovado', notes[0] === 'A medição não registrou passagens por estas funções. Isso não prova que elas não executaram: ausência de contagem observada não é prova de ausência de execução.', notes[0]);
  ok('a nota da cobertura não afirma que não executou', /não diz se elas executaram/.test(notes[1]), notes[1]);
  const opened = await page.evaluate(([a, b]) => document.querySelector(a).innerText + document.querySelector(b).innerText, [NO_PASSES, NO_COVERAGE]);
  ok('nenhuma das listas diz "não rodou" ou "mais pesada"', !/não rodou|mais pesada/i.test(opened));
  await page.evaluate(s => { document.querySelector(s).scrollIntoView(); }, NO_PASSES);
  await page.screenshot({ path: `${OUT}/4a-abertas.png` });

  // O estado aberto ou fechado sobrevive a uma escolha em Separar.
  await page.evaluate(s => { document.querySelector(s).open = false; }, NO_COVERAGE);
  await page.waitForTimeout(100);
  const choice = await page.evaluate(() => __t.hot.rows.find(x => x.symbol && x.calls > 0).at);
  await page.selectOption('#hotPick', choice);
  await page.waitForTimeout(200);
  const kept = await page.evaluate(s => s.map(x => document.querySelector(x).open), [NO_PASSES, NO_COVERAGE]);
  ok('a escolha em Separar mantém uma aberta e outra fechada', kept[0] === true && kept[1] === false, kept.join());

  // Os casos do contrato que esta rodada real não tem. Dados simulados na página: não vêm da API.
  const before = await page.evaluate(() => { const h = __t.hot, f = h.coverage.functions.slice(0, 16);
    const zeros = ['entry_zero', 'no_line', 'own_zero', 'several_zero', 'unknown_zero'];
    const reasons = ['test', 'skipped_path', 'no_body', 'const_fn', 'already_counts', 'language_not_supported', 'shared_line', 'not_classified'];
    const made = [...zeros.map((z, i) => ({ ...f[i], coverage_state: 'counted_no_passes', exclusion_reason: null, zero_evidence: z })),
      ...reasons.map((r, i) => ({ ...f[i + 5], coverage_state: 'no_coverage', exclusion_reason: r, zero_evidence: null })),
      { ...f[13], coverage_state: 'counted_and_ran', exclusion_reason: null, zero_evidence: null }, { ...f[14], coverage_state: 'outro_estado', exclusion_reason: null, zero_evidence: null }];
    __t.hot = { ...h, rows: [{ at: 'sem/funcao.rs:1', calls: 9e9, cpu_ms: null, symbol: null, name: null, path: 'sem/funcao.rs', count_kind: 'unknown' }, ...h.rows], coverage: { ...h.coverage, functions: made } };
    __t.renderHot(); return made; });
  await open(page, NO_COVERAGE);
  const simPasses = await items(page, NO_PASSES), simCoverage = await items(page, NO_COVERAGE);
  ok('simulado: cada valor de zero_evidence tem seu texto', same(simPasses, before.filter(f => f.coverage_state === 'counted_no_passes').map(f => line(f, ZERO[f.zero_evidence]))), simPasses.join(' | ').slice(0, 300));
  ok('simulado: cada motivo de exclusão tem seu texto', same(simCoverage, before.filter(f => f.coverage_state === 'no_coverage').map(f => line(f, REASON[f.exclusion_reason]))), simCoverage.join(' | ').slice(0, 300));
  ok('simulado: shared_line é sem cobertura, sem virar outro estado', simCoverage.some(x => x.endsWith(REASON.shared_line)) && !simPasses.some(x => x.endsWith(REASON.shared_line)));
  const strange = before.at(-1);
  ok('simulado: um estado desconhecido não é forçado em nenhuma lista', ![...simPasses, ...simCoverage].some(x => x.startsWith(`${strange.name} ${strange.path} `)));
  const nullRow = await page.evaluate(() => { const first = document.querySelector('#hotList ol.claims > li'); return first.textContent; });
  ok('simulado: linha sem função fica só no ranking, pelo lugar, sem estado', nullRow.includes('sem/funcao.rs:1') && ![...simPasses, ...simCoverage].some(x => x.includes('sem/funcao.rs')));
  const empty = await page.evaluate(([a, b]) => { const h = __t.hot; __t.hot = { ...h, coverage: { ...h.coverage, functions: [] } }; __t.renderHot();
    return [a, b].map(s => ({ items: document.querySelectorAll(`${s} li`).length, text: document.querySelector(s).querySelectorAll('p')[1]?.textContent })); }, [NO_PASSES, NO_COVERAGE]);
  ok('simulado: lista vazia diz que não há nenhuma, sem número', empty.every(x => x.items === 0 && x.text === 'Nenhuma função neste estado.'), JSON.stringify(empty));

  // Em inglês, com a página nesse idioma antes de abrir o projeto: trocar depois não redesenha o cartão,
  // o que já acontecia antes desta entrega e vale para o cartão inteiro.
  const english = await start(browser, B, RENDERER, run, errors, counter, undefined, undefined, 'en');
  await english.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await english.waitForTimeout(500);
  await english.evaluate(() => __t.showTab('inventory'));
  const en = await english.evaluate(([a, b]) => ({ a: document.querySelector(a).querySelector('summary').textContent, b: document.querySelector(b).querySelector('summary').textContent,
    title: [...document.querySelectorAll('#hotList h3')].map(x => x.textContent).find(x => /Top 20/.test(x)), all: document.querySelector('#hotList').textContent, note: document.querySelector(`${a} p`).textContent }), [NO_PASSES, NO_COVERAGE]);
  ok('em inglês, a nota de sem passagens é o texto aprovado', en.note === 'The measurement recorded no passes through these functions. This does not prove they did not execute: absence of observed counts is not proof of absence of execution.', en.note);
  ok('em inglês, os títulos mudam', en.a === 'Counted with no passes observed' && en.b === 'No coverage' && /^Top 20 · Largest count observed/.test(en.title ?? ''), JSON.stringify([en.a, en.b, en.title]));
  ok('em inglês, nenhuma chave crua nem frase em português nas listas', !/hot\.(zero|reason)\.|medição antiga|Contadas sem/.test(en.all));
  const allKeys = await english.evaluate(([z, r]) => [...z.map(k => __t.t(`hot.zero.${k}`)), ...r.map(k => __t.t(`hot.reason.${k}`))], [Object.keys(ZERO), Object.keys(REASON)]);
  ok('em inglês, todo motivo tem texto', allKeys.every(x => !x.startsWith('hot.')) && !allKeys.some(x => /contou|medição|sem corpo/.test(x)), allKeys.filter(x => x.startsWith('hot.')).join());

  // Sem medição não há estados: nada de "sem cobertura" para o projeto inteiro.
  const python = await start(browser, P, PYTHON, runPy, errors, counter);
  await python.waitForFunction(() => __t.hotStatus === 'none', null, { timeout: 120000 });
  await python.evaluate(() => __t.showTab('inventory'));
  await python.waitForTimeout(300);
  const pyList = await python.evaluate(([a, b]) => [document.querySelector(a), document.querySelector(b)].every(x => x === null) , [NO_PASSES, NO_COVERAGE]);
  ok('Python sem medição: nenhuma das listas aparece', pyList);

  // Tema escuro e tela estreita: as listas abrem sem estourar a largura.
  const dark = await start(browser, B, RENDERER, run, errors, counter, { width: 420, height: 800 }, 'dark');
  await dark.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await dark.waitForTimeout(500);
  await dark.evaluate(() => __t.showTab('inventory'));
  await open(dark, NO_PASSES);
  await dark.evaluate(s => document.querySelector(s).scrollIntoView(), NO_PASSES);
  await dark.waitForTimeout(200);
  const overflow = await dark.evaluate(s => { const d = document.querySelector(s); return d.scrollWidth - d.clientWidth; }, NO_PASSES);
  ok('tela estreita: a lista aberta não estoura a largura', overflow <= 1, String(overflow));
  await dark.screenshot({ path: `${OUT}/4a-escuro-estreito.png` });

  ok('nenhum envio', counter.sent === 0, String(counter.sent));
  ok('sem erro de página', errors.length === 0, errors.join(' | ').slice(0, 300));
  await browser.close();
})().catch(e => { console.error(e); process.exit(2); });

// Passo 6: contagem manual, projeto sem fonte Rust e a mesma pergunta antes e depois da medição, na página real,
// com projetos descartáveis criados aqui e comandos locais (git grep). Nenhum modelo: /api/send e /api/ladder são abortados.
const { chromium } = require('./ambiente');
const { spawnSync } = require('child_process');
const fs = require('fs'), path = require('path');
const OUT = process.argv[2], B = 'http://127.0.0.1:8842';
const HOOK = "window.__t = { state, studio, t, showTab, loadRun, setMode, renderGathered, get hotStatus() { return hotStatus; }, get suggestion() { return suggestion; }, set suggestion(v) { suggestion = v; }, get gathered() { return gathered; }, set gathered(v) { gathered = v; }, get seeds() { return seeds; }, set seeds(v) { seeds = v; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 260) : ''}`); if (!pass) process.exitCode = 1; };
const call = async (m, p, body) => { const r = await fetch(B + p, { method: m, headers: { 'content-type': 'application/json' }, body: body && JSON.stringify(body) }); return { status: r.status, body: await r.json().catch(() => null) }; };
const must = async (...a) => { const r = await call(...a); if (r.status >= 400) throw new Error(`${a[1]}: ${r.status} ${JSON.stringify(r.body)}`); return r.body; };
const APP = 'def plan_route(items):\n    return [helper(i) for i in items]\n\n\ndef helper(x):\n    return x + 1\n';
const PRINTED = 'librarian-measure calls=7 at=src/app.py:1\nlibrarian-measure calls=3 at=src/app.py:2\n';
const LIB = 'pub fn roda() -> u32 {\n    nao_roda()\n}\npub fn nao_roda() -> u32 {\n    1\n}\npub const P: &str = "\nlibrarian-measure calls=7 at=src/lib.rs:1\n";\n';
const grep = file => ({ argv: ['git', 'grep', '-h', 'librarian-measure', '--', file], repeat: 1 });
const NO_INSTRUMENT = /não declara um instrument/, NO_SOURCE = 'Não disponível neste projeto: a contagem por função só é inserida em fontes Rust, e esta edition não tem nenhuma.';
const MANUAL = /«manual» imprimiu contagens manuais em 2 linha\(s\)\. Elas ficam fora de Onde pesa: veja-as em Investigar/;
const SAYS_NONE = 'esta edition não tem essa medição';

const make = (name, files, profile) => {
  const root = path.join(OUT, 'passo6', name);
  fs.rmSync(root, { recursive: true, force: true });
  fs.mkdirSync(path.join(root, 'src'), { recursive: true });
  for (const [f, text] of Object.entries(files)) fs.writeFileSync(path.join(root, f), text);
  fs.writeFileSync(path.join(root, 'librarian.json'), JSON.stringify({ project_root: '.', ...profile }));
  for (const args of [['init', '-q'], ['add', '-A'], ['-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '-m', 'x']]) {
    const r = spawnSync('git', ['-C', root, ...args], { encoding: 'utf8' });
    if (r.status) throw new Error(r.stderr);
  }
  return root;
};
const generate = async root => { const { key } = await must('POST', '/api/project', { root }); const { id: run } = await must('POST', '/api/generate', { key }); return { key, run }; };
const approve = async ({ key }, name) => { const c = await must('GET', `/api/commands?key=${key}`); await must('POST', '/api/commands/approve', { key, name, sha256: c.commands.find(x => x.name === name).sha256 }); };
const open = async (browser, root, run, errors, counter, opts = {}) => {
  const page = await (await browser.newContext({ viewport: opts.size ?? { width: 1500, height: 1000 }, colorScheme: opts.scheme ?? 'light' })).newPage();
  page.on('pageerror', e => errors.push(e.message));
  await page.route(/\/api\/(send|ladder|ask|generate|instrument\/run|commands\/run|commands\/approve|paste|review)\b/, r => { counter.sent++; return r.abort(); });
  await page.route(`${B}/`, async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  if (opts.lang) await page.click(`[data-lang="${opts.lang}"]`);
  await page.fill('#projectPath', root);
  await page.click('#openProject');
  await page.waitForFunction(() => __t.studio.project, null, { timeout: 60000 });
  await page.evaluate(x => __t.loadRun(x), run);
  await page.waitForFunction(() => ['ready', 'none', 'failed'].includes(__t.hotStatus) && __t.studio.measureAll, null, { timeout: 60000 });
  await page.waitForTimeout(400);
  await page.evaluate(() => __t.showTab('inventory'));
  return page;
};
const hotState = page => page.evaluate(() => ({ text: document.querySelector('#hotState').textContent, hidden: document.querySelector('#hotState').hidden, off: document.querySelector('#hotMeasure').disabled }));
// A pergunta pelo campo real; devolve o cartão da conversa, sem nenhum modelo.
const ask = async (page, question) => {
  const before = await page.evaluate(() => document.querySelector('#turns').innerText.length);
  await page.evaluate(() => { const s = document.querySelector('#provider'); s.value = 'manual'; s.dispatchEvent(new Event('change')); });
  await page.fill('#question', question);
  await page.click('#ask');
  await page.waitForFunction(n => document.querySelector('#turns').innerText.length > n, before, { timeout: 30000 });
  await page.waitForTimeout(200);
  return page.evaluate(() => document.querySelector('#turns').innerText);
};
const panel = async (page, label) => {
  const id = await page.evaluate(l => __t.state.nodes.find(n => n.label === l || String(n.label).endsWith(l))?.id, label);
  await page.evaluate(x => { __t.suggestion = null; __t.gathered = [x]; __t.seeds = [x]; __t.renderGathered(); }, id);
  await page.evaluate(() => [...document.querySelectorAll('#gathered button')].find(x => x.textContent === __t.t('envelope.suggest')).click());
  await page.waitForFunction(() => __t.suggestion && document.querySelector('#suggestion'), null, { timeout: 30000 });
  return page.evaluate(() => ({ text: document.querySelector('#suggestion').innerText, measured: __t.suggestion.measured, coverage: __t.suggestion.coverage }));
};

(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const errors = [], counter = { sent: 0 };

  // Python puro, sem instrument, com e sem contagem manual.
  const py = make('python', { 'src/app.py': APP, 'src/printed.txt': PRINTED }, { commands: { manual: grep('src/printed.txt') } });
  const a = await generate(py);
  let c = await must('GET', `/api/commands?key=${a.key}&run=${a.run}`);
  ok('D7, API: Python sem instrument diz no_source, com um único motivo', c.measure_all.reason === 'no_source' && c.measure_all.available === false && !('detail' in c.measure_all), JSON.stringify(c.measure_all));
  ok('D6, API: sem execução, manual_counts é vazio e fica fora de measure_all', Array.isArray(c.manual_counts) && !c.manual_counts.length && !('manual_counts' in c.measure_all), JSON.stringify(c.manual_counts));
  let page = await open(browser, py, a.run, errors, counter);
  let st = await hotState(page);
  ok('D7, tela: diz que falta fonte Rust, com o botão desligado', st.text === NO_SOURCE && st.off, st.text);
  ok('D7, tela: não sugere declarar instrument', !NO_INSTRUMENT.test(st.text));
  let turns = await ask(page, 'Quais funções pesam?');
  ok('sem contagem manual, a conversa segue dizendo que não há medição', turns.includes(SAYS_NONE) && turns.includes(NO_SOURCE), turns.slice(-260));
  ok('sem contagem manual, nenhuma menção a contagens manuais', !/contagens manuais/.test(turns));
  let p = await panel(page, 'plan_route');
  ok('sem contagem manual, o painel diz "Sem medição" como antes', p.text.includes('Sem medição nesta edition') && !/manuais/.test(p.text), p.text.slice(0, 160));

  await approve(a, 'manual');
  const ran = await must('POST', '/api/commands/run', { key: a.key, run: a.run, name: 'manual' });
  c = await must('GET', `/api/commands?key=${a.key}&run=${a.run}`);
  ok('D6, API: a execução aparece em manual_counts', JSON.stringify(c.manual_counts) === JSON.stringify([{ command: 'manual', execution: ran.id, lines: 2 }]), JSON.stringify(c.manual_counts));
  const hot404 = await call('GET', `/api/hotspots?key=${a.key}&run=${a.run}`);
  ok('invariante: /api/hotspots continua 404 sem Medir tudo', hot404.status === 404, hot404.status);
  page = await open(browser, py, a.run, errors, counter);
  st = await hotState(page);
  ok('D6, tela: Onde pesa diz que não dá para medir e que há contagens manuais', st.text.startsWith(NO_SOURCE) && MANUAL.test(st.text), st.text);
  turns = await ask(page, 'Quais funções pesam?');
  ok('D6, conversa: não diz simplesmente que não há medição', !turns.includes(SAYS_NONE), turns.slice(-300));
  ok('D6, conversa: aponta as contagens manuais e Investigar', /«manual» imprimiu contagens manuais em 2 linhas/.test(turns) && /Investigar/.test(turns), turns.slice(-300));
  p = await panel(page, 'plan_route');
  ok('D6, painel: aponta as contagens manuais, sem dizer "Sem medição"', /contagens manuais em Investigar/.test(p.text) && !p.text.includes('Sem medição nesta edition'), p.text.slice(0, 200));
  ok('invariante: a sugestão continua sem medição, para a seleção', p.measured === false && p.coverage === null, JSON.stringify([p.measured, p.coverage]));
  ok('invariante: nenhum estado de cobertura por item sem medição', !/sem cobertura|passagens observadas/.test(p.text));
  // Investigar mostra de fato o que a tela aponta.
  await page.evaluate(() => __t.showTab('inventory'));
  await page.click('#inventoryBuild');
  await page.waitForFunction(() => /medido|Medida por origem/.test(document.querySelector('#inventoryList, #inventory, [data-panel="inventory"]')?.innerText ?? document.body.innerText), null, { timeout: 60000 });
  const shown = await page.evaluate(() => document.body.innerText);
  ok('D6: as contagens estão mesmo em Investigar, ao montar o inventário', /Medida por origem: 2 linha/.test(shown), (shown.match(/Medida por origem[^\n]*/) ?? [''])[0]);
  await page.screenshot({ path: `${OUT}/passo6-python.png` });

  // Em inglês, e trocando o idioma com a tela aberta.
  const en = await open(browser, py, a.run, errors, counter, { lang: 'en' });
  st = await hotState(en);
  ok('em inglês: motivo e nota', /only goes into Rust sources/.test(st.text) && /«manual» printed manual counts on 2 line\(s\)\. They stay out of Where the time goes: see them in Investigate/.test(st.text) && !/contagens|Não disponível/.test(st.text), st.text);
  p = await panel(en, 'plan_route');
  ok('em inglês: painel', /There are manual counts in Investigate/.test(p.text) && !/Sem Medir/.test(p.text), p.text.slice(0, 200));
  await en.click('[data-lang="pt"]');
  await en.waitForTimeout(200);
  st = await hotState(en);
  ok('trocando para português: a nota muda', MANUAL.test(st.text), st.text);

  // Misto: Medir tudo possível, com contagem manual, e a mesma pergunta antes e depois.
  const mix = make('misto', { 'src/app.py': APP, 'src/lib.rs': LIB, 'src/printed.txt': PRINTED }, { commands: { contagem: grep('src/lib.rs'), manual: grep('src/printed.txt') }, instrument: { statement: 'crate::count!();', command: 'contagem' } });
  const b = await generate(mix);
  await approve(b, 'contagem');
  await approve(b, 'manual');
  await must('POST', '/api/commands/run', { key: b.key, run: b.run, name: 'manual' });
  page = await open(browser, mix, b.run, errors, counter);
  st = await hotState(page);
  ok('misto sem Medir tudo: não diz "ainda não medido", e aponta as contagens', /O Medir tudo ainda não foi feito/.test(st.text) && MANUAL.test(st.text) && !/Ainda não medido/.test(st.text) && !st.off, st.text);
  const questions = ['Otimize o plan_route', 'Otimize o roda', 'Otimize o nao_roda', 'Otimize o relatorio', 'Quais funções pesam?', 'Onde otimizar?'];
  const routes = () => Promise.all(questions.map(async question => (await must('POST', '/api/performance-question', { key: b.key, run: b.run, question })).route));
  const before = await routes();
  await must('POST', '/api/instrument/run', { key: b.key, run: b.run });
  const after = await routes();
  ok('D8: a mesma rota antes e depois da medição', JSON.stringify(before) === JSON.stringify(after), questions.filter((_, i) => JSON.stringify(before[i]) !== JSON.stringify(after[i])).join());
  ok('D8: nome de função Python e Rust reconhecidos', before[0].intent === 'optimize' && before[0].target.status === 'resolved' && before[1].target.status === 'resolved' && before[2].target.status === 'resolved');
  ok('D8: o que não é função segue sem intenção, antes e depois', before[3].intent === null && after[3].intent === null);
  const hot = await must('GET', `/api/hotspots?key=${b.key}&run=${b.run}`);
  ok('invariante: a contagem manual não entra em rows', hot.rows.length > 0 && hot.rows.every(r => !r.at.startsWith('src/app.py')), hot.rows.map(r => r.at).join());
  ok('invariante: o Python continua sem cobertura, com o motivo da linguagem', hot.coverage.functions.filter(f => f.path === 'src/app.py').every(f => f.coverage_state === 'no_coverage' && f.exclusion_reason === 'language_not_supported'));
  page = await open(browser, mix, b.run, errors, counter);
  st = await hotState(page);
  ok('com Medir tudo: a nota some, e o ranking aparece', st.hidden && !/contagens manuais/.test(st.text) && await page.evaluate(() => document.querySelectorAll('#hotList ol.claims > li').length > 0), st.text);
  p = await panel(page, 'roda');
  ok('com Medir tudo: a sugestão mede, e não fala em contagens manuais', p.measured === true && !/manuais/.test(p.text), String(p.measured));

  // O comando contou e depois falhou: Investigar lê a última execução, e nada pode apontar para lá.
  const fail = make('falha', { 'src/app.py': APP, 'src/printed.txt': PRINTED }, { commands: { manual: grep('src/printed.txt') } });
  const d = await generate(fail);
  await approve(d, 'manual');
  await must('POST', '/api/commands/run', { key: d.key, run: d.run, name: 'manual' });
  c = await must('GET', `/api/commands?key=${d.key}&run=${d.run}`);
  ok('auditoria: antes da falha, a execução aparece', c.manual_counts.length === 1 && c.manual_counts[0].lines === 2, JSON.stringify(c.manual_counts));
  await new Promise(z => setTimeout(z, 1100)); // A ordem é pelo início da execução, em segundos.
  fs.writeFileSync(path.join(fail, 'librarian.json'), JSON.stringify({ project_root: '.', commands: { manual: { argv: ['git', 'nao-existe'], repeat: 1 } } }));
  await approve(d, 'manual');
  await call('POST', '/api/commands/run', { key: d.key, run: d.run, name: 'manual' });
  c = await must('GET', `/api/commands?key=${d.key}&run=${d.run}`);
  ok('auditoria: depois da falha, manual_counts vem vazio', Array.isArray(c.manual_counts) && !c.manual_counts.length, JSON.stringify(c.manual_counts));
  page = await open(browser, fail, d.run, errors, counter);
  st = await hotState(page);
  ok('auditoria: Onde pesa não aponta contagens manuais', st.text === NO_SOURCE, st.text);
  turns = await ask(page, 'Quais funções pesam?');
  ok('auditoria: a conversa não aponta contagens manuais', turns.includes(SAYS_NONE) && !/contagens manuais/.test(turns), turns.slice(-260));
  p = await panel(page, 'plan_route');
  ok('auditoria: o painel não aponta contagens manuais', p.text.includes('Sem medição nesta edition') && !/manuais/.test(p.text), p.text.slice(0, 160));
  await page.evaluate(() => __t.showTab('inventory'));
  await page.click('#inventoryBuild');
  await page.waitForFunction(() => !document.querySelector('#inventorySearch').hidden, null, { timeout: 60000 });
  const failed = await page.evaluate(() => document.body.innerText);
  ok('auditoria: Investigar, com a mesma página, também não mostra as linhas', !/Medida por origem/.test(failed), (failed.match(/Medida por origem[^\n]*/) ?? [''])[0]);

  // Tema escuro e tela estreita.
  const dark = await open(browser, py, a.run, errors, counter, { size: { width: 420, height: 800 }, scheme: 'dark' });
  const wide = await dark.evaluate(() => { const e = document.querySelector('#hotState'); return e.scrollWidth - e.clientWidth; });
  ok('tela estreita: a nota não transborda', wide <= 1, String(wide));
  await dark.screenshot({ path: `${OUT}/passo6-escuro-estreito.png` });

  ok('nenhum envio pela página', counter.sent === 0, String(counter.sent));
  ok('sem erro de página', errors.length === 0, errors.join(' | '));
  await browser.close();
})().catch(e => { console.error(e); process.exit(2); });

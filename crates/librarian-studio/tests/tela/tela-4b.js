// Passo 4, entrega 4B: os estados de cobertura no painel da sugestão, na página real, contra o que a API diz.
// Nada é enviado: /api/send, /api/ladder e as execuções são abortados.
const { chromium, RENDERER, PYTHON, CHAVE_RENDERER, CHAVE_PYTHON } = require('./ambiente');
const OUT = process.argv[2];
const B = 'http://127.0.0.1:8841', P = 'http://127.0.0.1:8842', run = '20261104-020244', runPy = '20261029-053623';
const HOOK = "window.__t = { state, studio, t, showTab, loadRun, renderGathered, toggleGathered, selectionOnRecord, absorbSuggestion, get hot() { return hot; }, set hot(v) { hot = v; }, get hotStatus() { return hotStatus; }, get suggestion() { return suggestion; }, set suggestion(v) { suggestion = v; }, get gathered() { return gathered; }, set gathered(v) { gathered = v; }, get seeds() { return seeds; }, set seeds(v) { seeds = v; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + d : ''}`); if (!pass) process.exitCode = 1; };
const STATE = { counted_and_ran: 'contada com passagens observadas', counted_no_passes: 'contada sem passagens observadas', no_coverage: 'sem cobertura' };
const STATE_EN = { counted_and_ran: 'counted with passes observed', counted_no_passes: 'counted with no passes observed', no_coverage: 'no coverage' };
const ZERO = { entry_zero: 'a instrução de entrada contou zero', no_line: 'recebeu a instrução, e nenhuma linha foi impressa para ela', own_zero: 'o contador do projeto contou zero',
  several_zero: 'as linhas contadas ficaram em zero', unknown_zero: 'contagem zero de origem desconhecida' };
const REASON = { test: 'teste', skipped_path: 'caminho deixado de fora no perfil', no_body: 'sem corpo', const_fn: 'const fn, que não recebe contador',
  already_counts: 'o contador aparece no texto, e nenhuma contagem foi impressa', language_not_supported: 'a linguagem não recebe contador',
  shared_line: 'divide a linha com outra função: as passagens não podem ser atribuídas a nenhuma das duas', not_recorded: 'medição antiga: o motivo não foi guardado', not_classified: 'o registro não classifica esta função' };
const NOTE = 'Passagens observadas não dizem que a função pesa. Ausência de passagens observadas não prova que ela não executou, e ausência de cobertura não diz se ela executou.';
const NOTE_EN = 'Observed passes do not say the function is heavy. Absence of observed passes does not prove it did not execute, and absence of coverage does not say whether it executed.';
const expected = (e, words = STATE, zero = ZERO, reason = REASON) => {
  if (!words[e.coverage_state]) return null;
  const why = e.coverage_state === 'counted_no_passes' ? zero[e.zero_evidence] : e.coverage_state === 'no_coverage' ? reason[e.exclusion_reason] : null;
  return why ? `${words[e.coverage_state]} ${why}` : words[e.coverage_state];
};
const suggest = async (base, key, wanted, nodes) => (await fetch(`${base}/api/envelope/suggest`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run: wanted, nodes }) })).json();
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
// O alvo no envelope e o clique real em Sugerir envelope, com tudo o que a sugestão traz aberto.
const clickSuggest = async (page, seed) => {
  await page.evaluate(id => { __t.suggestion = null; __t.gathered = [id]; __t.seeds = [id]; __t.renderGathered(); }, seed);
  await page.evaluate(() => [...document.querySelectorAll('#gathered button')].find(x => x.textContent === __t.t('envelope.suggest')).click());
  await page.waitForFunction(() => __t.suggestion && document.querySelector('#suggestion'), null, { timeout: 60000 });
  await page.evaluate(() => document.querySelectorAll('#suggestion details').forEach(d => { d.open = true; }));
  await page.waitForTimeout(100);
};
// Desenha uma sugestão dada: dados simulados na página, não vêm da API.
const draw = (page, made) => page.evaluate(s => { __t.suggestion = s; __t.renderGathered(); document.querySelectorAll('#suggestion details').forEach(d => { d.open = true; }); }, made);
const rows = page => page.evaluate(() => [...document.querySelectorAll('#suggestion label.item')].map(l => ({ coverage: l.querySelector('.coverage')?.textContent.trim() ?? null, tag: l.querySelector('.coverage .tag')?.textContent ?? null,
  why: l.querySelector('.why')?.textContent ?? null })));
const note = page => page.evaluate(() => document.querySelector('#suggestionCoverageNote')?.textContent ?? null);
const panel = page => page.evaluate(() => document.querySelector('#suggestion').innerText);
// As linhas sem a parte de cobertura: o que o painel já mostrava.
const rest = page => page.evaluate(() => [...document.querySelectorAll('#suggestion label.item')].map(l => { const c = l.cloneNode(true); c.querySelector('.coverage')?.remove(); return c.textContent; }));
const entry = (name, more) => ({ id: `sim:${name}`, name, path: 'sim/a.rs', kind: 'function', start_line: 1, end_line: 2, role: 'cost', bytes: 10, calls: null, facts: [], reasons: [], left_out: null, ...more });
const base = (items, more) => ({ items, left_out: [], gaps: [], behavior: [], measured: true, budget_bytes: 6144, used_bytes: 100, coverage: { total: 1 }, ...more });

(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const errors = [], counter = { sent: 0 };

  // Um alvo real com os três estados na resposta da API, escolhido pela própria resposta.
  const hotspots = await (await fetch(`${B}/api/hotspots?key=${CHAVE_RENDERER}&run=${run}`)).json();
  const tryThese = [...new Set(hotspots.rows.filter(r => r.symbol).map(r => r.symbol))].slice(0, 40);
  let seed = null, answer = null, best = -1;
  for (const id of tryThese) {
    const got = await suggest(B, CHAVE_RENDERER, run, [id]);
    const seen = new Set([...got.items, ...got.left_out].map(e => e.coverage_state).filter(Boolean)).size;
    if (seen > best) { best = seen; seed = id; answer = got; }
    if (best === 3) break;
  }
  const all = [...answer.items, ...answer.left_out];
  const states = new Set(all.map(e => e.coverage_state).filter(Boolean));
  console.log(`alvo: ${all.find(e => e.id === seed)?.name}, estados na API: ${[...states].join(', ')}`);
  ok('a API traz coverage_state em cada item, nulo no que não é função', all.every(e => 'coverage_state' in e) && all.filter(e => !['function', 'method', 'trait_method'].includes(e.kind)).every(e => e.coverage_state === null));
  ok('o alvo real traz ao menos dois dos estados', states.size >= 2, [...states].join());

  const page = await start(browser, B, RENDERER, run, errors, counter);
  await page.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await page.waitForTimeout(500);
  const requests = [];
  page.on('request', r => requests.push({ url: r.url(), body: r.postData() }));
  await clickSuggest(page, seed);

  // O caminho real: cada linha, na ordem, com o estado que a API deu a ela.
  const shown = await rows(page), want = all.map(e => expected(e));
  ok('uma linha por item da API', shown.length === all.length, `${shown.length} de ${all.length}`);
  const diff = shown.map((r, i) => r.coverage === want[i] ? null : `${all[i].name}: ${r.coverage} / ${want[i]}`).filter(Boolean);
  ok('real: cada linha mostra o estado e o motivo que a API deu', diff.length === 0, diff.slice(0, 3).join(' | '));
  ok('real: quem não é função não recebe estado', shown.every((r, i) => all[i].coverage_state !== null || r.coverage === null));
  ok('real: a nota aparece uma vez, com o texto aprovado', (await note(page)) === NOTE, await note(page) ?? '');
  await page.screenshot({ path: `${OUT}/4b-painel.png` });

  // A origem é só a sugestão.
  const hotAsked = requests.filter(r => r.url.includes('/api/hotspots')).length, suggestAsked = requests.filter(r => r.url.includes('/api/envelope/suggest'));
  ok('o painel não pede /api/hotspots', hotAsked === 0, String(hotAsked));
  ok('o pedido da sugestão continua sendo key, run e nodes', suggestAsked.length === 1 && JSON.stringify(Object.keys(JSON.parse(suggestAsked[0].body)).sort()) === '["key","nodes","run"]', suggestAsked[0]?.body);
  const withHot = await rows(page);
  await page.evaluate(() => { __t.hot = null; __t.renderGathered(); document.querySelectorAll('#suggestion details').forEach(d => { d.open = true; }); });
  const noHot = await rows(page);
  ok('sem o estado de /api/hotspots na página, os estados são os mesmos', JSON.stringify(noHot.map(r => r.coverage)) === JSON.stringify(withHot.map(r => r.coverage)));

  // Os casos que a rodada real não tem. Dados simulados na página: não vêm da API.
  const sim = await start(browser, B, RENDERER, run, errors, counter);
  await sim.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await sim.waitForTimeout(500);
  await sim.evaluate(id => { __t.gathered = [id]; __t.seeds = [id]; __t.absorbSuggestion({ items: [], left_out: [], gaps: [], measured: true, budget_bytes: 1, used_bytes: 1 }, [id]); }, seed);
  const zeros = Object.keys(ZERO).map(z => entry(`z_${z}`, { coverage_state: 'counted_no_passes', zero_evidence: z, exclusion_reason: null }));
  const reasons = Object.keys(REASON).map(r => entry(`r_${r}`, { coverage_state: 'no_coverage', exclusion_reason: r, zero_evidence: null }));
  const three = [entry('ran', { coverage_state: 'counted_and_ran', calls: 5, exclusion_reason: null, zero_evidence: null }), ...zeros, ...reasons];
  await draw(sim, base(three));
  const simRows = await rows(sim);
  ok('simulado: passagens observadas não traz motivo', simRows[0].coverage === STATE.counted_and_ran, simRows[0].coverage);
  ok('simulado: cada zero_evidence tem seu texto', zeros.every((e, i) => simRows[i + 1].coverage === expected(e)), simRows.slice(1, 6).map(r => r.coverage).join(' | ').slice(0, 300));
  ok('simulado: cada exclusion_reason tem seu texto', reasons.every((e, i) => simRows[i + 6].coverage === expected(e)), simRows.slice(6).map(r => r.coverage).join(' | ').slice(0, 300));
  ok('simulado: os três títulos são distintos', new Set(simRows.map(r => r.tag)).size === 3, [...new Set(simRows.map(r => r.tag))].join(' | '));
  const said = await panel(sim);
  ok('nenhum texto diz "não rodou" ou "mais pesada"', !/não rodou|mais pesada/i.test(said));
  ok('sem passagens não vira sem cobertura, nem o contrário', simRows.slice(1, 6).every(r => r.tag === STATE.counted_no_passes) && simRows.slice(6).every(r => r.tag === STATE.no_coverage));

  // Ausente e desconhecido: nada é inventado.
  const strange = [entry('sem_campo'), entry('nulo', { coverage_state: null }), entry('outro', { coverage_state: 'outro_estado', exclusion_reason: 'test' }),
    entry('nome_do_objeto', { coverage_state: 'constructor' }), entry('zero_estranho', { coverage_state: 'counted_no_passes', zero_evidence: 'outro_zero' }),
    entry('zero_ausente', { coverage_state: 'counted_no_passes' }), entry('motivo_estranho', { coverage_state: 'no_coverage', exclusion_reason: 'outro_motivo' }), entry('motivo_nulo', { coverage_state: 'no_coverage', exclusion_reason: null })];
  await draw(sim, base(strange));
  const sr = await rows(sim), strangeText = await panel(sim);
  ok('campo ausente, nulo ou estado desconhecido: nenhum estado na linha', sr.slice(0, 4).every(r => r.coverage === null), JSON.stringify(sr.slice(0, 4).map(r => r.coverage)));
  ok('motivo desconhecido, ausente ou nulo: fica só o estado, sem motivo', sr[4].coverage === STATE.counted_no_passes && sr[5].coverage === STATE.counted_no_passes && sr[6].coverage === STATE.no_coverage && sr[7].coverage === STATE.no_coverage, JSON.stringify(sr.slice(4).map(r => r.coverage)));
  ok('nenhuma chave crua nem valor do contrato na tela', !/envelope\.coverage|hot\.zero|hot\.reason|outro_|undefined|constructor/.test(strangeText), strangeText.slice(0, 200));
  await draw(sim, base([entry('sem_campo'), entry('outro', { coverage_state: 'outro_estado' })]));
  ok('sem nenhum estado reconhecido, a nota não aparece', (await note(sim)) === null);

  // A seleção lê a mediana, e o estado, qualquer repetição: "sem passagens" não pode ficar ao lado de outro estado.
  const NO_PASSES_OUT = 'sem passagens na medição', OTHER_OUT = 'a seleção não encontrou contagem acima de zero';
  const out = (name, more) => entry(name, { left_out: 'did_not_run', ...more });
  await draw(sim, base([], { left_out: [out('mediana_zero', { coverage_state: 'counted_and_ran', calls: 0 }), out('linha_dividida', { coverage_state: 'no_coverage', exclusion_reason: 'shared_line' }),
    out('sem_passagens', { coverage_state: 'counted_no_passes', zero_evidence: 'no_line' }), out('sem_estado'), out('estado_estranho', { coverage_state: 'outro_estado' }), entry('orcamento', { left_out: 'budget', coverage_state: 'counted_and_ran' })] }));
  const rd = await rows(sim);
  ok('mediana zero com máximo positivo: não diz "sem passagens" ao lado de passagens observadas', rd[0].why === OTHER_OUT && rd[0].tag === STATE.counted_and_ran, JSON.stringify(rd[0]));
  ok('shared_line: não diz "sem passagens" ao lado de sem cobertura', rd[1].why === OTHER_OUT && rd[1].tag === STATE.no_coverage, JSON.stringify(rd[1]));
  ok('sem passagens observadas continua dizendo "sem passagens na medição"', rd[2].why === NO_PASSES_OUT && rd[2].tag === STATE.counted_no_passes, JSON.stringify(rd[2]));
  ok('sem estado reconhecido, o texto da seleção é o de antes', rd[3].why === NO_PASSES_OUT && rd[4].why === NO_PASSES_OUT, JSON.stringify(rd.slice(3, 5)));
  ok('outro motivo de ficar de fora não muda', rd[5].why === 'não coube no orçamento', rd[5].why);
  const kept = await sim.evaluate(id => { const made = { items: [{ id, name: 'alvo', path: 'a', kind: 'function', reasons: [] }], left_out: [{ id: 'sim:x', name: 'x', path: 'a', kind: 'function', reasons: [], left_out: 'did_not_run', coverage_state: 'counted_and_ran' }], gaps: [], measured: true, budget_bytes: 1, used_bytes: 1 };
    __t.gathered = [id]; __t.seeds = [id]; __t.absorbSuggestion(made, [id]); return __t.selectionOnRecord().left_out.find(e => e.id === 'sim:x'); }, seed);
  ok('o registro continua com did_not_run: só muda o texto da tela', kept?.why === 'did_not_run' && kept?.left_out_why === 'did_not_run', JSON.stringify(kept?.why));

  // Sem medição não é "sem cobertura": o servidor diz no_coverage, e a tela não diz que a medição não cobriu.
  const noMeasure = base([entry('a', { coverage_state: 'no_coverage', exclusion_reason: null }), entry('b', { coverage_state: 'no_coverage', exclusion_reason: null })], { measured: false, coverage: null });
  // Esta rodada real tem contagens manuais; aqui a edition não tem nenhuma. O caso com elas está em tela-passo6.js.
  await sim.evaluate(() => { __t.studio.manualCounts = []; });
  await draw(sim, noMeasure);
  const nm = await rows(sim), nmText = await panel(sim);
  ok('simulado, sem medição: nenhum estado na linha, e a nota não aparece', nm.every(r => r.coverage === null) && (await note(sim)) === null);
  ok('simulado, sem medição: o aviso de sempre continua', nmText.includes('Sem medição nesta edition'));
  ok('simulado, sem medição: não diz "sem cobertura"', !/sem cobertura/i.test(nmText));
  await draw(sim, base([entry('a', { coverage_state: 'no_coverage', exclusion_reason: 'not_recorded' })]));
  ok('simulado, com medição: o aviso de sem medição não aparece', !(await panel(sim)).includes('Sem medição nesta edition'));

  // O painel continua igual no resto: mesmos textos, mesmas marcas, mesmas listas.
  await sim.evaluate(id => { __t.gathered = [id]; __t.seeds = [id]; }, seed);
  await draw(sim, answer);
  const withFields = await rest(sim), counts = await sim.evaluate(() => document.querySelectorAll('#suggestion details').length);
  const stripped = JSON.parse(JSON.stringify(answer));
  for (const e of [...stripped.items, ...stripped.left_out]) { delete e.coverage_state; delete e.exclusion_reason; delete e.zero_evidence; }
  delete stripped.coverage;
  await draw(sim, stripped);
  const withoutFields = await rest(sim);
  ok('sem os campos, as linhas têm exatamente o mesmo texto de antes', JSON.stringify(withFields) === JSON.stringify(withoutFields), `${withFields.length} linhas`);
  ok('sem os campos, nenhuma linha traz estado', (await rows(sim)).every(r => r.coverage === null));
  ok('o que ficou de fora, as lacunas e o comportamento continuam nas dobras', counts === [answer.left_out, answer.gaps, answer.behavior].filter(x => x.length).length, String(counts));
  const whyKept = await rows(sim);
  ok('o motivo da seleção continua no mesmo lugar', answer.left_out.length === 0 || whyKept[answer.items.length].why.startsWith(await sim.evaluate(k => __t.t(`envelope.out.${k}`), answer.left_out[0].left_out)));

  await draw(sim, answer);
  const candidate = answer.items.find(e => e.id !== seed && !e.id.startsWith('header:')) ?? answer.items[0];
  const toggled = await sim.evaluate(id => { const before = __t.gathered.includes(id); const i = [...document.querySelectorAll('#suggestion label.item')].find(l => l.querySelector('.mono')?.textContent === __t.suggestion.items.find(x => x.id === id).name.replace(/^impl (.+ for )?/, ''));
    i.querySelector('input').click(); return { before, after: __t.gathered.includes(id) }; }, candidate.id);
  ok('a marca da linha continua juntando e tirando', toggled.before !== toggled.after, JSON.stringify(toggled));

  // O registro da seleção é o mesmo: o estado é só para a pessoa.
  const records = await sim.evaluate(([full, bare, id]) => { const once = s => { __t.gathered = [id]; __t.seeds = [id]; __t.absorbSuggestion(s, [id]); return JSON.stringify(__t.selectionOnRecord()); }; return [once(full), once(bare)]; }, [answer, stripped, seed]);
  ok('origin.selection não leva nenhum campo de cobertura', !/coverage_state|exclusion_reason|zero_evidence/.test(records[0]));
  // Formato 2: cada absorção acrescenta o hash da sugestão, na ordem; fora isso, o registro é o mesmo.
  const [first, second] = records.map(r => JSON.parse(r));
  const sem = r => JSON.stringify({ ...r, suggestions: undefined });
  ok('origin.selection é igual com e sem os campos', sem(first) === sem(second) && JSON.stringify(second.suggestions) === JSON.stringify([...first.suggestions, answer.suggestion]),
    `${first.suggestions.length} e ${second.suggestions.length} sugestões`);

  // Em inglês, com a página nesse idioma antes de abrir o projeto.
  const english = await start(browser, B, RENDERER, run, errors, counter, undefined, undefined, 'en');
  await english.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await english.waitForTimeout(500);
  await clickSuggest(english, seed);
  const enRows = await rows(english), enText = await panel(english);
  ok('em inglês, a nota é o texto aprovado', (await note(english)) === NOTE_EN, await note(english) ?? '');
  ok('em inglês, cada linha tem o título em inglês', enRows.every((r, i) => (all[i].coverage_state ? r.tag === STATE_EN[all[i].coverage_state] : r.tag === null)), [...new Set(enRows.map(r => r.tag))].join(' | '));
  ok('em inglês, nenhuma chave crua nem frase em português', !/envelope\.coverage|hot\.(zero|reason)\.|contada|sem cobertura|medição antiga/.test(enText));
  const enKeys = await english.evaluate(k => k.map(x => __t.t(x)), [...Object.keys(STATE).map(k => `envelope.coverage.${k}`), 'envelope.coverageNote']);
  ok('em inglês, os três estados e a nota têm texto', JSON.stringify(enKeys) === JSON.stringify([...Object.values(STATE_EN), NOTE_EN]), enKeys.join(' | '));

  ok('em inglês, o motivo da seleção sem contagem tem texto', await english.evaluate(() => __t.t('envelope.out.did_not_run_other')) === 'the selection found no count above zero');

  // A troca de idioma com o painel aberto: desenha de novo, nas mesmas linhas, com as dobras como estavam.
  const swap = await start(browser, B, RENDERER, run, errors, counter);
  await swap.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await swap.waitForTimeout(500);
  await clickSuggest(swap, seed);
  const pt = await rows(swap);
  await swap.click('[data-lang="en"]');
  await swap.waitForTimeout(200);
  const toEn = await rows(swap), toEnNote = await note(swap), toEnText = await panel(swap);
  ok('painel aberto, para inglês: as mesmas linhas, com os títulos em inglês', toEn.length === pt.length && toEn.every((r, i) => (all[i].coverage_state ? r.tag === STATE_EN[all[i].coverage_state] : r.tag === null)), [...new Set(toEn.map(r => r.tag))].join(' | '));
  ok('painel aberto, para inglês: a nota muda', toEnNote === NOTE_EN, toEnNote ?? '');
  ok('painel aberto, para inglês: nada fica em português', !/contada|sem cobertura|medição antiga|sem passagens/.test(toEnText));
  ok('painel aberto, para inglês: as dobras seguem abertas', await swap.evaluate(() => [...document.querySelectorAll('#suggestion details')].every(d => d.open)));
  await swap.click('[data-lang="pt"]');
  await swap.waitForTimeout(200);
  ok('painel aberto, de volta ao português: igual ao início', JSON.stringify(await rows(swap)) === JSON.stringify(pt) && (await note(swap)) === NOTE);

  // Sem medição de verdade: a API diz no_coverage com motivo nulo, e a tela não repete isso como "sem cobertura".
  const pyAll = await fetch(`${P}/api/hotspots?key=${CHAVE_PYTHON}&run=${runPy}`);
  ok('Python: a rodada não tem medição', pyAll.status === 404, String(pyAll.status));
  const python = await start(browser, P, PYTHON, runPy, errors, counter);
  await python.waitForFunction(() => __t.hotStatus === 'none', null, { timeout: 120000 });
  const pySeed = await python.evaluate(() => __t.state.nodes.find(n => ['function', 'method'].includes(n.kind))?.id);
  const pyAnswer = await suggest(P, CHAVE_PYTHON, runPy, [pySeed]);
  ok('Python: a API diz coverage nulo, measured falso, e no_coverage sem motivo no alvo', pyAnswer.coverage === null && pyAnswer.measured === false && pyAnswer.items.find(e => e.id === pySeed)?.coverage_state === 'no_coverage' && pyAnswer.items.find(e => e.id === pySeed)?.exclusion_reason === null, JSON.stringify(pyAnswer.items.find(e => e.id === pySeed)?.coverage_state));
  await clickSuggest(python, pySeed);
  const pyRows = await rows(python), pyText = await panel(python);
  ok('Python sem medição: nenhuma linha traz estado', pyRows.length > 0 && pyRows.every(r => r.coverage === null), String(pyRows.length));
  ok('Python sem medição: diz que não há medição, e não "sem cobertura"', pyText.includes('Sem medição nesta edition') && !/sem cobertura/i.test(pyText));
  ok('Python sem medição: a nota não aparece', (await note(python)) === null);

  // Tema escuro e tela estreita.
  const dark = await start(browser, B, RENDERER, run, errors, counter, { width: 420, height: 800 }, 'dark');
  await dark.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await dark.waitForTimeout(500);
  await dark.click('#colSwitch [data-col="question"]');
  await clickSuggest(dark, seed);
  // F5: o painel da sugestão fica no Builder, no Centro; a pessoa chega a ele por "Abrir no Builder".
  await dark.click('#draftOpen');
  ok('tela estreita: o painel da sugestão está à vista, no Builder', await dark.evaluate(() => { const d = document.querySelector('#suggestion'); return Boolean(d && d.getClientRects().length && d.offsetParent !== null); }));
  // O painel já transborda nessa largura sem os estados, por outras linhas: a medida é não piorar.
  const overflow = () => dark.evaluate(() => { const d = document.querySelector('#suggestion'); d.querySelectorAll('details').forEach(x => { x.open = true; }); return d.scrollWidth - d.clientWidth; });
  const withStates = await overflow();
  const wide = await dark.evaluate(() => [...document.querySelectorAll('#suggestion .coverage')].filter(c => c.scrollWidth > c.clientWidth + 1).length);
  await draw(dark, stripped);
  const withoutStates = await overflow();
  ok('tela estreita: os estados não aumentam o transbordo do painel', withStates <= withoutStates, `${withStates} com, ${withoutStates} sem`);
  ok('tela estreita: nenhuma linha de estado transborda', wide === 0, String(wide));
  await draw(dark, answer);
  await dark.evaluate(() => document.querySelector('#suggestion').scrollIntoView());
  await dark.waitForTimeout(200);
  await dark.screenshot({ path: `${OUT}/4b-escuro-estreito.png` });

  ok('nenhum envio', counter.sent === 0, String(counter.sent));
  ok('sem erro de página', errors.length === 0, errors.join(' | ').slice(0, 300));
  await browser.close();
})().catch(e => { console.error(e); process.exit(2); });

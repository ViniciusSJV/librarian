// O zero real do renderer na tela e na API, nas cópias: r3 (Camera::new, zero depois do reset) e r1
// (World::new fora do envelope como "sem passagens"). Nada é enviado: envios e execuções são abortados.
const { chromium, RENDERER, CASOS } = require('./ambiente');
const fs = require('fs');
const B = 'http://127.0.0.1:8841', project = RENDERER, r3 = '20261104-020244', r1 = '20261104-014710';
const caso = JSON.parse(fs.readFileSync(CASOS)).find(x => x.id === 'r1-inversa');
const HOOK = "window.__t = { state, studio, t, loadRun, showTab, openPath, idle, ran, measuredOn, renderSuggestion: typeof renderSuggestion === 'function' ? renderSuggestion : null, suggestEnvelope, toggleGathered, get hot() { return hot; }, get city() { return city; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d !== '' ? '  -> ' + d : ''}`); if (!pass) process.exitCode = 1; };
const FALSO = /não rodou neste comando|não rodaram neste comando|não executaram|não foram chamadas/;
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await (await browser.newContext({ viewport: { width: 1500, height: 1000 } })).newPage();
  const errors = []; let sent = 0;
  page.on('pageerror', e => errors.push(e.message));
  await page.route(/\/api\/(send|ladder|generate|instrument\/run|commands\/run|commands\/approve|paste|review)\b/, r => { sent++; return r.abort(); });
  await page.route(`${B}/`, async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  await page.fill('#projectPath', project);
  await page.click('#openProject');
  await page.waitForFunction(() => __t.studio.project, null, { timeout: 60000 });
  await page.evaluate(x => __t.loadRun(x), r3);
  await page.waitForFunction(() => __t.hot, null, { timeout: 120000 });
  await page.waitForTimeout(600);
  const r = await page.evaluate(async () => {
    const key = __t.studio.project.key, run = __t.studio.run, hot = __t.hot;
    const q = async question => (await (await fetch('/api/performance-question', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run, question }) })).json());
    const fn = n => hot.coverage.functions.find(f => f.name === n);
    const last = n => n.split('::').pop(); const silent = hot.coverage.functions.find(f => f.zero_evidence === 'no_line' && !hot.rows.some(y => y.name && last(y.name) === last(f.name)));
    return { cov: hot.coverage, camera: fn('impl Camera::new'), silent, idleCamera: __t.idle(fn('impl Camera::new').symbol), status: document.querySelector('#status')?.innerText ?? '',
      ask: (await q('Otimize o Camera::new')), silentAsk: (await q(`Otimize o ${silent.name.replace(/^impl /, '')}`)), heavy: (await q('Quais funções pesam?')), dot: (await q('Otimize o Tuple::dot')), limits: hot.limits.join(' '),
      texts: { ran: __t.t('path.ran'), notRan: __t.t('path.notRan'), out: __t.t('envelope.out.did_not_run'), res: __t.t('path.resolution.measured'), why: __t.t('why.called_measured', { t: 'x', lines: '1' }), help: __t.t('help.pathRan') },
      visibleFilter: document.querySelector('#pathRanRow')?.innerText ?? '' };
  });
  const c = r.cov;
  ok('estados do r3 somam o total', c.counted_and_ran + c.counted_no_passes + c.no_coverage === c.total, `${c.counted_and_ran}+${c.counted_no_passes}+${c.no_coverage}=${c.total}`);
  ok('nome antigo fora da API', !('counted_not_run' in c));
  ok('r3: 38 com zero na entrada e 125 sem linha', c.zero_evidence.entry_zero === 38 && c.zero_evidence.no_line === 125, JSON.stringify(c.zero_evidence));
  ok('Camera::new: sem passagens, zero na entrada', r.camera.coverage_state === 'counted_no_passes' && r.camera.zero_evidence === 'entry_zero', `${r.camera.coverage_state}/${r.camera.zero_evidence}`);
  ok('Camera::new: resposta não diz que não rodou', /A instrução de entrada de Camera::new contou zero nesta medição; isso não prova/.test(r.ask.answer[0]) && !FALSO.test(r.ask.answer.join(' ')), r.ask.answer[0].slice(0, 120));
  ok('sem linha: resposta diferente do zero', /recebeu a instrução e a medição não imprimiu contagem/.test(r.silentAsk.answer[0]) && !FALSO.test(r.silentAsk.answer.join(' ')), r.silentAsk.answer[0].slice(0, 110));
  ok('resumo separa os quatro casos, com os números da medição', r.heavy.answer.at(-1) === `Esta medição tem contagem ou instrução em ${c.counted} de ${c.total} funções: ${c.counted_and_ran} com contagem acima de zero; 38 com zero impresso; 125 que receberam a instrução e não imprimiram linha; ${c.no_coverage} sem cobertura. Nenhum desses casos prova que a função não rodou.`, r.heavy.answer.at(-1));
  ok('texto ao modelo sem afirmação de execução', !FALSO.test(r.dot.changes[0].text) && /38 com zero impresso; 125 que receberam/.test(r.dot.changes[0].text));
  ok('avisos sem "não rodou neste comando"', !FALSO.test(r.limits) && /não observou passagens positivas/.test(r.limits));
  ok('cidade: funções com contagem acima de zero', /81 funções com contagem acima de zero/.test(r.status) && !/rodaram/.test(r.status), r.status.slice(0, 70));
  ok('filtro e etiqueta do fluxograma', r.texts.ran === 'só o que teve contagem' && r.texts.notRan === 'sem contagem');
  ok('homônimos falam de contagem', /só este teve contagem/.test(r.texts.res) && /só este teve contagem/.test(r.texts.why));
  ok('ajuda do filtro não afirma execução', /o que não prova que não rodaram/.test(r.texts.help) && !/não rodaram nenhuma/.test(r.texts.help));
  ok('idle continua igual para Camera::new', r.idleCamera === true);
  // O rótulo na sugestão real: World::new e Camera::new ficam de fora em r1, como antes.
  await page.evaluate(x => __t.loadRun(x), r1);
  await page.waitForFunction(x => __t.studio.run === x && __t.hot, r1, { timeout: 120000 });
  await page.waitForTimeout(800);
  await page.evaluate(ids => { for (const id of ids) __t.toggleGathered(id); }, caso.seed_ids);
  await page.evaluate(() => __t.suggestEnvelope(document.createElement('button')));
  await page.waitForTimeout(1200);
  const box = await page.evaluate(async () => { const s = document.querySelector('#suggestion'); s?.querySelectorAll('details').forEach(d => d.open = true); return s?.innerText ?? ''; });
  ok('sugestão: rótulo novo nas que ficaram de fora', /sem passagens na medição/.test(box) && !/não rodou/.test(box), (box.match(/[^\n]*sem passagens na medição[^\n]*/) ?? [''])[0].slice(0, 90));
  // A rota, no renderer real: a medição não escolhe o alvo.
  const rota = await page.evaluate(async () => { const key = __t.studio.project.key, run = __t.studio.run;
    const q = async question => (await (await fetch('/api/performance-question', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run, question }) })).json());
    return { stripe: await q('Otimize o Stripe::default'), sphere: await q('Otimize o Sphere::default'), many: await q('Otimize default'), main: await q('Otimize o main'), none: await q('Otimize o Inexistente::default'), dot: await q('Otimize o Tuple::dot'), where: await q('Onde dá para otimizar?') }; });
  ok('rota: Stripe::default é a função pedida', rota.stripe.route.function === 'Default for Stripe::default' && rota.stripe.answer.join(' ').includes('Default for Stripe::default') && !rota.stripe.answer.join(' ').includes('Sphere'), rota.stripe.answer[0].slice(0, 110));
  ok('rota: Sphere::default é outra função', rota.sphere.route.function === 'Default for Sphere::default' && rota.sphere.route.target.symbol !== rota.stripe.route.target.symbol);
  ok('rota: default sozinho é ambíguo', rota.many.route.target.status === 'ambiguous' && rota.many.route.target.candidates.length === 12 && rota.many.changes.length === 0, rota.many.answer[0].slice(0, 130));
  ok('rota: main existe 5 vezes e é ambíguo', rota.main.route.target.status === 'ambiguous' && rota.main.route.target.candidates.length === 5, rota.main.answer[0].slice(0, 130));
  ok('rota: nome qualificado inexistente não vira outro', rota.none.route.target.status === 'not_found' && rota.none.changes.length === 0, rota.none.answer[0].slice(0, 110));
  ok('rota: pergunta existente continua igual', rota.dot.route.function === 'Tuple::dot' && rota.dot.changes.length === 1 && rota.dot.changes[0].symbol === rota.dot.route.target.symbol);
  ok('rota: onde otimizar continua sem alvo', rota.where.route.intent === 'where' && rota.where.route.function === null && rota.where.changes.length === 0);
  const varios = await page.evaluate(async () => { const key = __t.studio.project.key, run = __t.studio.run;
    const q = async question => (await (await fetch('/api/performance-question', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run, question }) })).json());
    return { a: await q('Otimize Tuple::dot e Cube::nada'), b: await q('Otimize Cube::nada e Tuple::dot'), dois: await q('Otimize Tuple::dot e Tuple::cross'), inteiro: await q('Otimize o ops::Mul<Tuple> for Matrix<4>::mul'), arquivo: await q('Otimize dot em src/tuple.rs'), comArquivo: await q('Otimize Tuple::dot em src/tuple.rs') }; });
  for (const [n, c] of [['Tuple::dot e Cube::nada', varios.a], ['Cube::nada e Tuple::dot', varios.b]]) ok('rota: ' + n + ' não ignora o que falta', c.route.target.status === 'not_found' && c.route.target.written === 'Cube::nada' && !c.changes.length && !c.execution && c.answer.join(' ').includes('Também pedida, e encontrada: Tuple::dot'), c.answer.join(' ').slice(0, 120));
  ok('rota: dois nomes que existem', varios.dois.route.target.status === 'ambiguous' && !varios.dois.changes.length && /nomeia 2 funções/.test(varios.dois.answer[0]), varios.dois.answer[0].slice(0, 110));
  ok('rota: nome real com dois :: é um nome só', varios.inteiro.route.function === 'ops::Mul<Tuple> for Matrix<4>::mul' && varios.inteiro.changes.length === 1 && !varios.inteiro.route.target.targets, varios.inteiro.answer[0].slice(0, 90));
  ok('rota: arquivo não é função', varios.arquivo.route.function === 'Tuple::dot' && varios.comArquivo.route.function === 'Tuple::dot' && varios.comArquivo.changes.length === 1);
  ok('nenhum envio', sent === 0);
  ok('sem erro de página', errors.length === 0, errors.join(' | ').slice(0, 200));
  await browser.close();
})().catch(e => { console.error(e); process.exit(2); });

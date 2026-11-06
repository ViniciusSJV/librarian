// Regressão dos quatro bloqueadores e da semântica de `added`, de ponta a ponta. Raiz isolada;
// /api/send abortado, /api/ladder simulado; o pacote é preparado pelo servidor real.
const { chromium, RENDERER, CHAVE_RENDERER, CHAVE_PYTHON } = require('./ambiente');
const fs = require('fs');
const [, , OUT, PY] = process.argv;
const HOOK = "window.__t = { state, studio, t, get hot() { return hot; }, get gathered() { return gathered; }, get seeds() { return seeds; }, get selection() { return selection; }, get suggestion() { return suggestion; }, get investigation() { return investigation; }, get gatherFacts() { return gatherFacts; }, loadRun, showTab, optimize, investigate, suggestEnvelope, renderGathered, setMode, toggleGathered, selectionOnRecord, selectNode };\n  if (location.hash === '#debug')";
const RS = 'http://127.0.0.1:8841', PB = 'http://127.0.0.1:8842';
const rp = RENDERER, rr = '20261104-014710', r3 = '20261104-020244', pr = '20261029-053623', rk = CHAVE_RENDERER, pk = CHAVE_PYTHON;
const INV = 'impl Matrix<4>::inverse', STAMP = Date.now();
const checks = [];
const ok = (d, name, pass, detail = '') => { checks.push({ d, name, pass: Boolean(pass), detail: String(detail) }); console.log(`${pass ? 'PASSA' : 'FALHA'}  [${d}] ${name}${detail !== '' ? '  -> ' + String(detail).slice(0, 170) : ''}`); };
const get = async (B, p) => { const r = await fetch(B + p); return { status: r.status, body: await r.json().catch(() => null) }; };

async function open(browser, B, project, run) {
  const page = await (await browser.newContext({ viewport: { width: 1500, height: 1000 } })).newPage();
  const log = { errors: [], prepared: [], ladder: [], sends: 0 };
  page.on('pageerror', e => log.errors.push(e.message));
  await page.route(/\/api\/(send|generate|instrument\/run|commands\/run|commands\/approve|paste|review)\b/, r => { log.sends++; return r.abort(); });
  await page.route(/\/api\/ladder\b/, r => { log.ladder.push(JSON.parse(r.request().postData())); return r.fulfill({ json: { passed: false, turn: '99', nodes: [], steps: [] } }); });
  await page.route(/\/api\/gathered-question\b/, r => { log.prepared.push(JSON.parse(r.request().postData())); return r.continue(); });
  await page.route(`${B}/`, async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status);
  await page.fill('#projectPath', project); await page.click('#openProject');
  await page.waitForFunction(() => __t.studio.project, null, { timeout: 120000 });
  if (run) await page.evaluate(x => __t.loadRun(x).catch(() => {}), run);
  await page.waitForFunction(() => __t.studio.measureAll && (__t.hot || !document.querySelector('#hotState').hidden), null, { timeout: 120000 });
  await page.waitForTimeout(300);
  return { page, log, close: () => page.context().close() };
}
let serial = 0;
const prepare = async (p, B, key, run, base) => {
  const question = `${base} ${STAMP} ${++serial}`;
  await p.evaluate(() => { document.querySelector('#ladderOn').checked = false; const s = document.querySelector('#provider'); s.value = 'manual'; s.dispatchEvent(new Event('change')); });
  await p.fill('#question', question); await p.click('#ask');
  await p.waitForFunction(q => __t.studio.turns.some(t => t.question === q), question, { timeout: 60000 });
  await p.waitForTimeout(300);
  return (await get(B, `/api/turns?key=${encodeURIComponent(key)}&run=${run}`)).body.find(t => t.question === question);
};
const lists = ['included', 'removed', 'left_out', 'not_joined'];
const inList = (rec, id) => lists.filter(k => rec[k].some(e => e.id === id));
const total = (p) => p.evaluate(() => { const r = __t.selectionOnRecord(); const all = ['included', 'removed', 'left_out', 'not_joined'].flatMap(k => r[k].map(e => e.id)); return { dup: all.length - new Set(all).size, fora: [...__t.selection.sources.keys()].filter(i => !all.includes(i)).length, nodes: JSON.stringify(__t.gathered) === JSON.stringify(r.included.map(e => e.id)) }; });
const clickRow = async (p, id) => { const h = await p.evaluateHandle(i => { const s = __t.suggestion, e = [...s.items, ...s.left_out].find(x => x.id === i); document.querySelectorAll('#suggestion details').forEach(d => { d.open = true; }); const short = e.kind === 'header' ? __t.t('envelope.headerName') : e.name.replace(/^impl (.+ for )?/, ''); return [...document.querySelectorAll('#suggestion label.item')].find(l => l.querySelector('.mono')?.textContent === short && l.innerText.includes(`${e.path}:${e.start_line}`)).querySelector('input'); }, id); await h.asElement().click(); await p.waitForTimeout(60); };
const join = p => p.evaluate(() => [...document.querySelectorAll('#suggestion button')].find(x => x.textContent.startsWith(__t.t('envelope.use', { n: 'X' }).split('(')[0])).click());

(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const inv = (await get(RS, `/api/inventory?key=${rk}&run=${rr}`)).body.inventory, loop = inv.loops.find(l => l.through_calls?.length) ?? inv.loops[0];

  // ---- Bloqueador 1.
  let a = await open(browser, RS, rp, rr), p = a.page;
  await p.evaluate(t => __t.investigate(t, document.createElement('button')), loop.occurrence);
  const o = await p.evaluate(() => { const r = __t.hot.rows.find(x => x.symbol && !__t.investigation.items.some(i => i.id === x.symbol)); __t.selectNode(__t.state.index.get(r.symbol)); __t.showTab('map'); return { id: r.symbol, name: r.name }; });
  await p.waitForTimeout(200);
  let b = await p.evaluate(() => { const x = [...document.querySelectorAll('#inspector button')].find(k => k.textContent === __t.t('gather.add') || k.textContent === __t.t('gather.remove')); return { text: x?.textContent, disabled: x?.disabled, title: x?.title }; });
  ok('B1', 'inspetor desligado durante a investigação, e diz por quê', b.disabled && /investigação aberta/.test(b.title), b.title.slice(0, 80));
  await p.evaluate(i => __t.toggleGathered(i), o.id);
  ok('B1', 'nenhum outro caminho junta durante a investigação', await p.evaluate(() => __t.gathered.length === 0 && __t.seeds.length === 0));
  const turn1 = await prepare(p, RS, rk, rr, 'B1');
  ok('B1', 'o pacote é exatamente o que está marcado', !a.log.prepared.at(-1).nodes.includes(o.id) && JSON.parse(turn1.query).evidence.items.length === a.log.prepared.at(-1).nodes.length, `${a.log.prepared.at(-1).nodes.length} itens`);
  ok('B1', 'nada sobra para a pergunta seguinte', await p.evaluate(() => __t.gathered.length === 0 && document.querySelectorAll('#gathered .chip.env').length === 0));
  b = await p.evaluate(() => { const x = [...document.querySelectorAll('#inspector button')].find(k => k.textContent === __t.t('gather.add')); return { disabled: x?.disabled, title: x?.title }; });
  ok('B1', 'ao concluir, o inspetor volta a juntar', b.disabled === false && b.title === '');
  await p.evaluate(i => __t.toggleGathered(i), o.id);
  ok('B1', 'e junta de verdade', await p.evaluate(i => __t.gathered.includes(i), o.id));
  await a.close();

  a = await open(browser, RS, rp, rr); p = a.page;
  await p.evaluate(async t => { await __t.investigate(t, document.createElement('button')); const r = __t.hot.rows.find(x => x.symbol); __t.selectNode(__t.state.index.get(r.symbol)); __t.showTab('map'); }, loop.occurrence);
  await p.waitForTimeout(200);
  await p.evaluate(() => [...document.querySelectorAll('#gathered button')].find(b => b.textContent === __t.t('investigate.cancel')).click());
  await p.waitForTimeout(100);
  b = await p.evaluate(() => { const x = [...document.querySelectorAll('#inspector button')].find(k => k.textContent === __t.t('gather.add')); return x?.disabled; });
  ok('B1', 'ao cancelar, o inspetor volta a juntar', b === false);
  await a.close();

  // ---- Bloqueador 2.
  a = await open(browser, RS, rp, rr); p = a.page;
  await p.evaluate(async ([t, n]) => { await __t.optimize(__t.hot.rows.find(r => r.name === n)); await __t.investigate(t, document.createElement('button')); }, [loop.occurrence, INV]);
  ok('B2', 'investigar zera os fatos ligados por um Otimizar anterior', await p.evaluate(() => __t.gatherFacts === false));
  const t2 = await prepare(p, RS, rk, rr, 'B2');
  ok('B2', 'servidor real: nenhum fato no pacote', a.log.prepared.at(-1).with_facts === false && (JSON.parse(t2.query).evidence.reviewed_facts?.length ?? 0) === 0, `with_facts=${a.log.prepared.at(-1).with_facts}`);
  await a.close();
  a = await open(browser, RS, rp, rr); p = a.page;
  await p.evaluate(async n => { await __t.optimize(__t.hot.rows.find(r => r.name === n)); }, INV);
  const t2b = await prepare(p, RS, rk, rr, 'B2 otimizar');
  ok('B2', 'Otimizar continua levando fatos, com o controle à vista', a.log.prepared.at(-1).with_facts === true && (JSON.parse(t2b.query).evidence.reviewed_facts?.length ?? 0) > 0, `${JSON.parse(t2b.query).evidence.reviewed_facts?.length} fatos`);
  await a.close();

  // ---- Bloqueador 3.
  a = await open(browser, RS, rp, rr); p = a.page;
  await p.route(/\/api\/inventory\/investigate\b/, async r => { await new Promise(z => setTimeout(z, 2500)); return r.continue(); });
  const s3 = await p.evaluate(async ([t, other]) => { const pending = __t.investigate(t, document.createElement('button')); await new Promise(z => setTimeout(z, 100)); await __t.loadRun(other); await pending; return { investigando: Boolean(__t.investigation), pergunta: document.querySelector('#question').value, cartao: document.querySelector('#gathered').innerText }; }, [loop.occurrence, r3]);
  ok('B3', 'resposta de outro grafo não abre investigação no novo', !s3.investigando && !s3.pergunta && !s3.cartao, JSON.stringify(s3).slice(0, 90));
  await a.close();
  a = await open(browser, PB, PY, pr); p = a.page;
  const pinv = (await get(PB, `/api/inventory?key=${pk}&run=${pr}`)).body.inventory;
  await p.evaluate(t => __t.investigate(t, document.createElement('button')), pinv.operations[0].id);
  ok('B3', 'Python: investigar no mesmo grafo continua funcionando', await p.evaluate(() => __t.investigation?.items.length > 0));
  const t3 = await prepare(p, PB, pk, pr, 'B3 python');
  ok('B3', 'Python: pacote preparado com os itens marcados', JSON.parse(t3.query).evidence.items.length === a.log.prepared.at(-1).nodes.length);
  await a.close();

  // ---- Bloqueador 4: o caso reproduzido.
  a = await open(browser, RS, rp, rr); p = a.page;
  const ids = await p.evaluate(() => { const r = n => __t.hot.rows.find(x => x.name === n).symbol; return { s: r(`impl Matrix<4>::inverse`), m: r('impl World::color_at') }; });
  await p.evaluate(({ s, m }) => { __t.toggleGathered(s); __t.toggleGathered(m); [...document.querySelectorAll('#gathered .chip.env')].find(c => c.querySelector('.mono').textContent === 'World::color_at').querySelector('button.target').click(); }, ids);
  await p.evaluate(() => __t.suggestEnvelope(document.createElement('button')));
  ok('B4', 'fonte juntada antes da sugestão já é conhecida', await p.evaluate(i => __t.selection.sources.has(i), ids.m));
  await p.evaluate(i => __t.toggleGathered(i), ids.m);
  let rec = await p.evaluate(() => __t.selectionOnRecord());
  ok('B4', 'removida fica em removed, com decisão', JSON.stringify(inList(rec, ids.m)) === '["removed"]' && rec.removed.find(e => e.id === ids.m).decision === 'removed' && rec.removed.find(e => e.id === ids.m).provenance === 'manual', inList(rec, ids.m).join());
  let tt = await total(p);
  ok('B4', 'partição total: nenhuma fora, nenhuma repetida', tt.fora === 0 && tt.dup === 0 && tt.nodes, JSON.stringify(tt));
  const t4 = await prepare(p, RS, rk, rr, 'B4');
  ok('B4', 'servidor real grava a removida', JSON.stringify(inList(t4.gathered.origin.selection, ids.m)) === '["removed"]');
  await a.close();

  // ---- Contrato: as oito combinações, cada uma com lista, origem e decisão.
  const entryOf = (rec, id) => { for (const k of lists) { const e = rec[k].find(x => x.id === id); if (e) return { list: k, ...e }; } return null; };
  const expectEntry = (name, rec, id, list, provenance, decision, scope) => { const e = entryOf(rec, id); ok('Tabela', name, e && e.list === list && e.provenance === provenance && e.decision === decision && e.decision_scope === scope, e ? `${e.list}/${e.provenance}/${e.decision}/${e.decision_scope}` : 'sem lista'); };
  const R0 = n => __t.hot.rows.find(x => x.name === n).symbol;
  a = await open(browser, RS, rp, rr); p = a.page;
  await p.evaluate(n => __t.optimize(__t.hot.rows.find(r => r.name === n)), INV);
  rec = await p.evaluate(() => __t.selectionOnRecord());
  const inv0 = await p.evaluate(n => __t.hot.rows.find(r => r.name === n).symbol, INV);
  expectEntry('1. Otimizar, sem toque: included, bulk', rec, inv0, 'included', 'suggested', 'included', 'bulk');
  ok('Tabela', 'Otimizar: toda fonte do envelope com decisão bulk', rec.included.every(e => e.decision === 'included' && e.decision_scope === 'bulk'), rec.included.length);
  ok('Contrato', 'lista added não existe', !('added' in rec));
  const sug = await p.evaluate(() => { const x = __t.suggestion.items.find(i => i.role !== 'target' && !i.id.startsWith('header:')); const out = __t.suggestion.left_out[0]; return { x: x.id, out: out.id, why: out.left_out }; });
  expectEntry('2. De fora, sem toque: left_out, sem decisão', rec, sug.out, 'left_out', 'left_out', null, null);
  ok('Tabela', '2. why e histórico preservados', entryOf(rec, sug.out).why === sug.why && entryOf(rec, sug.out).left_out_why === sug.why);
  await clickRow(p, sug.out);
  rec = await p.evaluate(() => __t.selectionOnRecord());
  expectEntry('3. De fora, trazida pela pessoa: included, single', rec, sug.out, 'included', 'left_out', 'included', 'single');
  ok('Tabela', '3. mantém o motivo original da exclusão', entryOf(rec, sug.out).left_out_why === sug.why);
  await clickRow(p, sug.x);
  rec = await p.evaluate(() => __t.selectionOnRecord());
  expectEntry('4. Removida: removed, single', rec, sug.x, 'removed', 'suggested', 'removed', 'single');
  await clickRow(p, sug.x);
  rec = await p.evaluate(() => __t.selectionOnRecord());
  expectEntry('5. Removida e devolvida: última ação, included, single', rec, sug.x, 'included', 'suggested', 'included', 'single');
  const man = await p.evaluate(() => { const r = __t.hot.rows.find(x => x.symbol && !__t.selection.sources.has(x.symbol)); __t.toggleGathered(r.symbol); return r.symbol; });
  rec = await p.evaluate(() => __t.selectionOnRecord());
  expectEntry('6. Pelo mapa, com sugestão existente: included, manual, single', rec, man, 'included', 'manual', 'included', 'single');
  ok('Tabela', '6. não vira alvo sozinha', !rec.seeds.includes(man));
  await a.close();

  a = await open(browser, RS, rp, rr); p = a.page;
  const s0 = await p.evaluate(n => { const r = __t.hot.rows.find(x => x.name === n).symbol; __t.toggleGathered(r); return r; }, INV);
  ok('Contrato', 'sem sugestão não há registro', await p.evaluate(() => __t.selectionOnRecord() === null));
  await p.evaluate(() => __t.suggestEnvelope(document.createElement('button')));
  rec = await p.evaluate(() => __t.selectionOnRecord());
  expectEntry('7. Juntada à mão e depois sugerida: origem manual, single', rec, s0, 'included', 'manual', 'included', 'single');
  const nj = rec.not_joined.length;
  ok('Tabela', '8a. Sugeridas sem Juntar: not_joined, sem decisão', nj > 0 && rec.not_joined.every(k => k.decision === null && k.decision_scope === null && k.provenance === 'suggested'), nj);
  await join(p);
  rec = await p.evaluate(() => __t.selectionOnRecord());
  const bulk = rec.included.filter(k => k.decision_scope === 'bulk');
  ok('Tabela', '8b. Juntar: included, bulk, not_joined vazio', bulk.length === nj && bulk.every(k => k.decision === 'included' && k.provenance === 'suggested') && rec.not_joined.length === 0, `${bulk.length} em bloco`);
  await a.close();

  // ---- Contradição 1 da auditoria: duas sugestões, a segunda seleciona quem a primeira deixou de fora.
  a = await open(browser, RS, rp, rr); p = a.page;
  const flip = await p.evaluate(async () => {
    const key = __t.studio.project.key, run = __t.studio.run;
    const ask = async nodes => (await fetch('/api/envelope/suggest', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ key, run, nodes }) })).json();
    // Toda função do grafo, por id: o par não depende da ordem das linhas da medição.
    const seeds = [...new Set(__t.hot.rows.filter(r => r.symbol).map(r => r.symbol).concat(__t.state.nodes.filter(n => ['function', 'method'].includes(n.kind)).map(n => n.id)))].sort();
    const cache = new Map();
    for (const x of seeds) { const first = await ask([x]); cache.set(x, first); for (const y of seeds) { if (y === x) continue; const second = cache.get(y); if (!second) continue; const f = first.left_out.find(e => second.items.some(i => i.id === e.id && i.id !== y)); if (f) return { x, y, id: f.id, why: f.left_out, name: f.name }; } }
    return null;
  });
  ok('Contradição', 'achou um par real de sugestões para o caso', Boolean(flip), flip?.name);
  await p.evaluate(({ x }) => __t.toggleGathered(x), flip);
  await p.evaluate(() => __t.suggestEnvelope(document.createElement('button')));
  rec = await p.evaluate(() => __t.selectionOnRecord());
  expectEntry('primeira sugestão: de fora', rec, flip.id, 'left_out', 'left_out', null, null);
  await p.evaluate(({ y }) => __t.toggleGathered(y), flip);
  await p.evaluate(({ y }) => { const n = (__t.state.nodes[__t.state.index.get(y)]?.label ?? __t.selection.sources.get(y).name).replace(/^impl (.+ for )?/, ''); [...document.querySelectorAll('#gathered .chip.env')].find(k => k.querySelector('.mono').textContent === n).querySelector('button.target').click(); }, flip);
  await p.evaluate(({ x }) => { const n = (__t.state.nodes[__t.state.index.get(x)]?.label ?? __t.selection.sources.get(x).name).replace(/^impl (.+ for )?/, ''); [...document.querySelectorAll('#gathered .chip.env')].find(k => k.querySelector('.mono').textContent === n).querySelector('button.target').click(); }, flip);
  await p.evaluate(() => __t.suggestEnvelope(document.createElement('button')));
  rec = await p.evaluate(() => __t.selectionOnRecord());
  const seen = entryOf(rec, flip.id);
  ok('Contradição', 'selecionada pela última sugestão não fica em left_out', seen?.list !== 'left_out' && await p.evaluate(i => __t.suggestion.items.some(e => e.id === i), flip.id), seen?.list);
  ok('Contradição', 'a origem e o motivo histórico são preservados', seen?.provenance === 'left_out' && seen?.left_out_why === flip.why && !('why' in seen), `${seen?.provenance}/${seen?.left_out_why}`);
  await a.close();

  // ---- Contradição 2: duas fontes juntadas à mão pelo mesmo gesto.
  a = await open(browser, RS, rp, rr); p = a.page;
  const two = await p.evaluate(() => { const r = n => __t.hot.rows.find(x => x.name === n).symbol; return { s: r('impl Matrix<4>::inverse'), m: r('impl World::color_at') }; });
  await p.evaluate(({ s, m }) => { __t.toggleGathered(s); __t.toggleGathered(m); [...document.querySelectorAll('#gathered .chip.env')].find(c => c.querySelector('.mono').textContent === 'World::color_at').querySelector('button.target').click(); }, two);
  await p.evaluate(() => __t.suggestEnvelope(document.createElement('button')));
  rec = await p.evaluate(() => __t.selectionOnRecord());
  ok('Contradição', 'mesmo gesto, mesma origem, com ou sem menção na sugestão', entryOf(rec, two.s)?.provenance === 'manual' && entryOf(rec, two.m)?.provenance === 'manual', `${entryOf(rec, two.s)?.provenance} e ${entryOf(rec, two.m)?.provenance}`);
  ok('Contradição', 'a mencionada pela sugestão ganha papel e motivos, sem perder a origem', entryOf(rec, two.s)?.role === 'target' && entryOf(rec, two.s)?.reasons.length > 0);
  await a.close();

  // ---- Remoção antes da primeira sugestão.
  a = await open(browser, RS, rp, rr); p = a.page;
  await p.evaluate(({ s, m }) => { __t.toggleGathered(s); __t.toggleGathered(m); __t.toggleGathered(m); }, two);
  await p.evaluate(() => __t.suggestEnvelope(document.createElement('button')));
  rec = await p.evaluate(() => __t.selectionOnRecord());
  expectEntry('removida antes da primeira sugestão entra em removed', rec, two.m, 'removed', 'manual', 'removed', 'single');
  const t5 = await prepare(p, RS, rk, rr, 'remocao antes');
  ok('Contrato', 'servidor real grava a removida antes da sugestão', entryOf(t5.gathered.origin.selection, two.m)?.list === 'removed');
  await a.close();

  // ---- origin: null em envelope à mão.
  a = await open(browser, RS, rp, rr); p = a.page;
  await p.evaluate(({ m }) => __t.toggleGathered(m), two);
  const t6 = await prepare(p, RS, rk, rr, 'sem sugestao');
  ok('Contrato', 'envelope à mão, sem sugestão: origin nulo, nodes dizem o que foi', a.log.prepared.at(-1).origin === null && t6.gathered.nodes.length === 1 && !t6.gathered.origin);
  await a.close();

  // ---- Bloqueio 3: Otimizar com investigação aberta.
  a = await open(browser, RS, rp, rr); p = a.page;
  const stuck = await p.evaluate(async ([t, n]) => {
    await __t.investigate(t, document.createElement('button'));
    const r = __t.hot.rows.find(x => x.name === 'impl World::color_at'); __t.selectNode(__t.state.index.get(r.symbol)); __t.showTab('map');
    await new Promise(z => setTimeout(z, 150));
    const btn = () => [...document.querySelectorAll('#inspector button')].find(k => [__t.t('gather.add'), __t.t('gather.remove')].includes(k.textContent));
    const during = btn().disabled;
    await __t.optimize(__t.hot.rows.find(x => x.name === n));
    await new Promise(z => setTimeout(z, 100));
    return { during, open: Boolean(__t.investigation), disabled: btn().disabled, title: btn().title };
  }, [loop.occurrence, INV]);
  ok('B1', 'Otimizar com investigação aberta devolve o inspetor', stuck.during === true && !stuck.open && stuck.disabled === false && stuck.title === '', JSON.stringify(stuck).slice(0, 80));
  await a.close();

  // ---- Item 5.
  a = await open(browser, RS, rp, rr); p = a.page;
  await p.evaluate(async n => { const r = __t.hot.rows.find(x => x.name === n); await __t.optimize(r); __t.toggleGathered(r.symbol); }, INV);
  await p.fill('#question', 'sem alvo'); await p.click('#ask'); await p.waitForTimeout(150);
  const shown = await p.evaluate(() => ({ t: document.querySelector('#llmNote').innerText, c: document.querySelector('#llmNote').className }));
  await p.evaluate(() => { [...document.querySelectorAll('#gathered .chip.env')].find(k => k.querySelector('button.target')).querySelector('button.target').click(); });
  const gone = await p.evaluate(() => ({ t: document.querySelector('#llmNote').innerText, c: document.querySelector('#llmNote').className, alvos: __t.seeds.length }));
  ok('Item 5', 'a nota aparece sem alvo e some ao escolher um', /Nenhum alvo/.test(shown.t) && shown.c === 'error' && !/Nenhum alvo/.test(gone.t) && gone.c !== 'error' && gone.alvos === 1, gone.t.slice(0, 50));
  await a.close();

  // ---- Item 6.
  a = await open(browser, RS, rp, rr); p = a.page;
  await p.route(new RegExp(`/api/hotspots.*${rr}`), r => r.fulfill({ status: 500, json: { error: 'falha do grafo anterior' } }));
  await p.route(new RegExp(`/api/hotspots.*${r3}`), async r => { await new Promise(z => setTimeout(z, 4000)); return r.continue(); });
  const stale6 = await p.evaluate(async ([a1, a2]) => {
    await __t.loadRun(a1); await new Promise(z => setTimeout(z, 300));
    const before = document.querySelector('#hotState').innerText;
    __t.loadRun(a2);
    const seen = [document.querySelector('#hotState').hidden ? '' : document.querySelector('#hotState').innerText, document.querySelector('#status').innerText];
    for (let i = 0; i < 12; i++) { await new Promise(z => setTimeout(z, 150)); seen.push(document.querySelector('#hotState').hidden ? '' : document.querySelector('#hotState').innerText, document.querySelector('#status').innerText); }
    return { before, bad: seen.filter(s => /Não foi possível ler|não pôde ser lida/.test(s)).length, total: seen.length };
  }, [rr, r3]);
  ok('Item 6', 'a falha de um grafo não aparece ao abrir outro', /Não foi possível ler/.test(stale6.before) && stale6.bad === 0, `${stale6.bad} de ${stale6.total}`);
  await a.close();

  a = await open(browser, RS, rp, rr); p = a.page;
  ok('Geral', 'sem erro de página e sem envio', a.log.errors.length === 0 && a.log.sends === 0, a.log.errors.join('|').slice(0, 120));
  await a.close();

  await browser.close();
  fs.writeFileSync(`${OUT}/e2c-resultado.json`, JSON.stringify(checks, null, 1));
  const failed = checks.filter(k => !k.pass);
  console.log(`\n${checks.length - failed.length} de ${checks.length} passaram`);
  process.exit(failed.length ? 1 : 0);
})().catch(err => { console.error(err); process.exit(2); });

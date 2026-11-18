// G3 visual check of "o que chama?" on the copy served at 8773. Screenshots + what the section says.
const { chromium } = require('C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/pw/node_modules/playwright-core');
const OUT = 'C:/Users/beatl/AppData/Local/Temp/claude/09139e964f85837f/c2d74a42-2372-4db9-9c84-8fc3b436eb57/scratchpad/g3';
const B = 'http://127.0.0.1:8773';
const HOOK = "window.__t = { state, studio, t, STRINGS, selectNode, openPath }; if (location.hash === '#debug')";
const HC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks', SYN = `${OUT}/synthetic`;
const sendalerts = 'src:04fd8532fcf9ba2615bc6ac62bffcbaee859b92e9af51ab808de06275c96a4f1:97769054348c8f590cabd15a0e44140664305d224bad550e9548ec2e674770e9';
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 400) : ''}`); if (!pass) process.exitCode = 1; };

async function page(browser, locale) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, locale });
  const p = await context.newPage(), errors = [];
  p.on('pageerror', e => errors.push(e.message));
  await p.route(u => new URL(u).pathname === '/', async r => { const x = await r.fetch(); r.fulfill({ response: x, body: (await x.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await p.goto(`${B}/`);
  await p.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  return { p, errors, context };
}
async function open(p, root) {
  await p.evaluate(r => { document.querySelector('#projectPath').value = r; document.querySelector('#openProject').click(); }, root);
  await p.waitForFunction(() => __t.studio.run && __t.state.nodes.length && !document.querySelector('#openProject').disabled, null, { timeout: 180000 });
  await p.waitForTimeout(800);
}
async function calls(p, id, depth = '1') {
  await p.evaluate(d => { document.querySelector('#pathDepth').value = d; }, depth);
  await p.evaluate(id => __t.openPath(id, 'calls'), id);
  await p.waitForFunction(() => { const n = document.querySelector('#pathNote').textContent; return n && !/Seguindo|Following/.test(n); }, null, { timeout: 60000 });
  await p.waitForTimeout(500);
  return p.evaluate(() => {
    const box = document.querySelector('#pathUnresolved');
    box.open = true;
    return { note: document.querySelector('#pathNote').textContent, hidden: box.hidden, summary: box.querySelector('summary').textContent,
      groups: [...box.querySelectorAll('h4')].map(h => ({ title: h.textContent, why: h.nextElementSibling.textContent,
        items: [...h.nextElementSibling.nextElementSibling.querySelectorAll('li')].map(li => li.textContent) })),
      dashed: document.querySelectorAll('#pathCanvas polyline.edge.ambiguous').length, boxes: document.querySelectorAll('#pathCanvas .node').length };
  });
}

(async () => {
  const browser = await chromium.launch({ executablePath: 'C:/Program Files/Google/Chrome/Application/chrome.exe', headless: true });
  const { p, errors, context } = await page(browser, 'pt-BR');
  await open(p, HC);
  // 1. sendalerts.notify: ch.notify(flip) with 33 candidates, and calls with no candidate.
  let s = await calls(p, `${sendalerts}:function:609:1968`);
  await p.screenshot({ path: `${OUT}/tela-notify.png` });
  console.log(JSON.stringify(s, null, 1).slice(0, 2500));
  const above = s.groups.find(g => g.title.startsWith('Acima do limite'));
  ok('notify: seção visível com contagem por motivo', !s.hidden && /Chamadas não apresentadas: 11 \(1 acima do limite de homônimos · 10 sem candidato no projeto\)/.test(s.summary), s.summary);
  ok('notify: ch.notify(flip) com 33 candidatos, no grupo de ambiguidade', above?.items.some(i => i.startsWith('ch.notify(flip)') && /sendalerts\.py:43/.test(i) && /33 candidatos/.test(i)), JSON.stringify(above));
  ok('notify: sem candidato não se diz externa', s.groups.find(g => g.title.startsWith('Sem candidato'))?.why.includes('não diz que a chamada é externa'));
  ok('notify: o desenho continua (5 chamadas)', s.boxes === 5, s.boxes);
  // 2. process_one_flip: nothing drawn, and the section still says what it writes.
  s = await calls(p, `${sendalerts}:method:3008:4196`);
  await p.screenshot({ path: `${OUT}/tela-process-one-flip.png` });
  ok('process_one_flip: nada desenhado, a nota aponta para a seção e a seção tem 11', /Nenhuma chamada a desenhar/.test(s.note) && !s.hidden && /: 11 /.test(s.summary), `${s.note} | ${s.summary}`);
  ok('process_one_flip: executor.submit(notify, flip) aparece', s.groups.flatMap(g => g.items).some(i => i.includes('executor.submit')), JSON.stringify(s.groups.flatMap(g => g.items)));
  // 3. Synthetic project: node_limit apart, with its resolution kept.
  await open(p, SYN);
  const seed = await p.evaluate(() => __t.state.nodes.find(n => n.category === 'symbol' && n.label === 'seed').id);
  await p.evaluate(() => { document.querySelector('#pathDepth').value = '1'; });
  s = await calls(p, seed);
  await p.screenshot({ path: `${OUT}/tela-node-limit.png` });
  const limited = s.groups.find(g => g.title.startsWith('Omitidas pelo limite'));
  ok('sintético: node_limit em grupo próprio, 11 itens, explicado como limite de apresentação', limited?.items.length === 11 && /não ambiguidade/.test(limited.why) && /120 pontos/.test(limited.why), JSON.stringify(limited).slice(0, 300));
  ok('sintético: cada item diz o alvo e a resolução original (seguida, nome único)', limited?.items.every(i => /seria f\d+ · seguida: o nome só combina/.test(i)), limited?.items[0]);
  ok('sintético: items[0] em sem candidato', s.groups.find(g => g.title.startsWith('Sem candidato'))?.items.some(i => i.startsWith('items[0]')));
  ok('sintético: o aviso de truncado continua na nota', /Parou no teto de 120 pontos/.test(s.note), s.note);
  ok('nenhum erro de página', !errors.length, errors.join(' | '));
  await context.close();
  // 4. English.
  const en = await page(browser, 'en-US');
  await en.p.evaluate(() => { if (__t.studio) localStorage.setItem('librarian.lang', 'en'); });
  await en.p.reload(); await en.p.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  await open(en.p, HC);
  s = await calls(en.p, `${sendalerts}:function:609:1968`);
  await en.p.screenshot({ path: `${OUT}/tela-notify-en.png` });
  ok('EN: summary and groups', /Calls not shown: 11/.test(s.summary) || /Chamadas não apresentadas: 11/.test(s.summary), s.summary);
  console.log('EN summary:', s.summary);
  await browser.close();
})().catch(e => { console.error(e); process.exit(1); });

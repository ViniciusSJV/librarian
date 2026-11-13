// F1: estado local e persistência (especificação, seções 5.2 a 5.8), na página real, num Chrome sem janela.
// Parte A, isolada: a página servida por um servidor estático deste script (sem /api, como no modo arquivo),
// sem dados de teste. Exercita o módulo `localState`: navegação por projeto, rascunho por projeto e run,
// identidade da edition, duas abas (9.2.5), armazenamento bloqueado (9.2.6) e cheio.
// Parte B, só quando o servidor de rodar.js responde em 8841: a página atual não grava nada do F1 no
// navegador nem no perfil ao abrir o projeto, trocar de run e recarregar, e funciona com o armazenamento bloqueado.
// Nada é enviado a modelo: a parte B aborta envio, geração, execuções e criação de perguntas.
const http = require('http');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium } = require(process.env.LIBRARIAN_TESTE_PLAYWRIGHT || 'playwright-core');

const PAGINA = path.join(__dirname, '../../web/index.html');
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + d : ''}`); if (!pass) process.exitCode = 1; };
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const F1_KEY = /^librarian\.(nav|draft)\.v1:/;
const K1 = 'proj-a-0001', K2 = 'proj-b-0002', R1 = '20261111-000001', R2 = '20261111-000002', CAT_A = 'a'.repeat(64), CAT_B = 'b'.repeat(64);

// Bloqueado: o acesso ao localStorage lança, como num navegador com dados do site bloqueados.
const BLOQUEIA = () => Object.defineProperty(window, 'localStorage', { get() { throw new DOMException('O acesso ao armazenamento foi negado', 'SecurityError'); } });
// Cheio sob demanda: com window.__cheio, gravar um rascunho lança QuotaExceededError.
const ENCHE = () => {
  const original = Storage.prototype.setItem;
  Storage.prototype.setItem = function (k, v) {
    if (window.__cheio && String(k).startsWith('librarian.draft.')) throw new DOMException('cota excedida', 'QuotaExceededError');
    return original.call(this, k, v);
  };
};

function servidorEstatico() {
  const s = http.createServer((req, res) => {
    const { pathname } = new URL(req.url, 'http://x');
    if (pathname === '/' || pathname === '/index.html') { res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' }); res.end(fs.readFileSync(PAGINA)); return; }
    res.writeHead(404, { 'Content-Type': 'application/json' }); res.end('{"error":"sem servidor do Studio"}');
  });
  return new Promise(r => s.listen(0, '127.0.0.1', () => r(s)));
}

async function abre(context, base, errors) {
  const page = await context.newPage();
  page.on('pageerror', e => errors.push(e.message));
  await page.goto(`${base}/`);
  await page.waitForFunction(() => typeof localState === 'object', null, { timeout: 30000 });
  return page;
}
const chaves = page => page.evaluate(() => Object.keys(localStorage).sort());
const guardado = (page, name) => page.evaluate(n => localStorage.getItem(n), name);
const status = (page, k, r, c) => page.evaluate(([k, r, c]) => localState.openDraft({ key: k, run: r, catalog: c }).status(), [k, r, c]);
// Tenta fechar com beforeunload: devolve se o navegador perguntou antes de sair.
async function avisaAoSair(page) {
  await page.mouse.click(5, 5); // o aviso só aparece depois de um gesto da pessoa
  let perguntou = false;
  page.on('dialog', d => { if (d.type() === 'beforeunload') { perguntou = true; d.accept(); } });
  await page.close({ runBeforeUnload: true });
  await new Promise(z => setTimeout(z, 1500));
  return perguntou;
}

async function parteA(browser) {
  const servidor = await servidorEstatico();
  const base = `http://127.0.0.1:${servidor.address().port}`;
  const errors = [];
  try {
    // ---- carga: o módulo existe e não grava nada sozinho
    let ctx = await browser.newContext();
    let page = await abre(ctx, base, errors);
    ok('A1 o módulo localState carrega sem erro na página', errors.length === 0, errors.join(' | '));
    ok('A1 carregar a página não grava chave de navegação nem de rascunho', !(await chaves(page)).some(k => F1_KEY.test(k)), (await chaves(page)).join(','));

    // ---- navegação por projeto (5.3)
    const nav = await page.evaluate(([K1, K2, R1, R2]) => {
      const vazio = localState.nav.read(K1);
      localState.nav.update(K1, { run: R1, area: 'project', active_turn: { [R1]: '03' }, mode: 'improve', allow_patch: true });
      localState.nav.update(K1, { active_turn: { [R2]: 7 }, center: { view: 'builder', turn: '05', extra: 1 } });
      localState.nav.update(K2, { run: R2 });
      const bruto = JSON.parse(localStorage.getItem(`librarian.nav.v1:${K1}`));
      localStorage.setItem('librarian.nav.v1:quebrado', '{não é json');
      return { vazio, a: localState.nav.read(K1), b: localState.nav.read(K2), bruto, quebrado: localState.nav.read('quebrado') };
    }, [K1, K2, R1, R2]);
    ok('A2 sem nada guardado, a navegação vem vazia', nav.vazio.ok && nav.vazio.nav === null, JSON.stringify(nav.vazio));
    ok('A2 a navegação guarda run, área, vista central e a pergunta ativa por run', same(nav.a.nav, { run: R1, area: 'project', explore_view: null, center: { view: 'builder', turn: '05' }, active_turn: { [R1]: '03', [R2]: 7 } }), JSON.stringify(nav.a.nav));
    ok('A2 a política do modo nunca vai para o navegador (campos fora do formato são descartados)', !('mode' in nav.bruto) && !('allow_patch' in nav.bruto), JSON.stringify(nav.bruto));
    ok('A2 cada projeto tem a própria navegação', nav.b.nav.run === R2 && same(nav.b.nav.active_turn, {}), JSON.stringify(nav.b.nav));
    ok('A2 navegação ilegível volta vazia e marcada como inválida, sem erro', nav.quebrado.ok && nav.quebrado.nav === null && nav.quebrado.invalid === true, JSON.stringify(nav.quebrado));

    // ---- rascunho por projeto e run (5.4, 5.6)
    const corpo = { builder: { included: [{ id: 'n1', name: 'Matrix::from' }], removed: [], left_out: [], not_joined: [] }, composer: { text: 'Otimize o Matrix::from', ladder: true } };
    const r = await page.evaluate(([K1, R1, CAT_A, corpo]) => {
      const s = localState.openDraft({ key: K1, run: R1, catalog: CAT_A });
      const antes = s.status();
      const gravou = s.update(corpo);
      return { antes, gravou, depois: s.status(), doc: JSON.parse(localStorage.getItem(localState.draftName(K1, R1))), tab: localState.tab };
    }, [K1, R1, CAT_A, corpo]);
    ok('A3 sem rascunho guardado, o estado é vazio e guardado no navegador', r.antes.state === 'empty' && r.antes.storage === 'stored', JSON.stringify(r.antes));
    ok('A3 o rascunho fica em librarian.draft.v1:<key>:<run>', r.gravou.ok && r.doc !== null);
    ok('A3 o cabeçalho tem esquema, key, run, catalog_sha256, saved_at, tab e seq',
      r.doc.schema === 'librarian.draft.v1' && r.doc.key === K1 && r.doc.run === R1 && r.doc.catalog_sha256 === CAT_A && !Number.isNaN(Date.parse(r.doc.saved_at)) && r.doc.tab === r.tab && Number.isInteger(r.doc.seq),
      JSON.stringify({ ...r.doc, body: '…' }));
    ok('A3 o corpo guardado é o mesmo objeto', same(r.doc.body, corpo));
    ok('A3 depois de gravar: rascunho guardado, nada pendente, com a data da gravação', r.depois.state === 'draft' && r.depois.storage === 'stored' && !r.depois.dirty && r.depois.saved_at === r.doc.saved_at, JSON.stringify(r.depois));

    await page.reload();
    await page.waitForFunction(() => typeof localState === 'object');
    const rec = await page.evaluate(([K1, R1, CAT_A]) => { const s = localState.openDraft({ key: K1, run: R1, catalog: CAT_A }); return { st: s.status(), body: s.body }; }, [K1, R1, CAT_A]);
    ok('A4 recarregar restaura o mesmo objeto (9.2.1, no módulo)', rec.st.state === 'restored' && same(rec.body, corpo), JSON.stringify(rec.st.state));
    const outra = await page.evaluate(([K1, K2, R1, R2, CAT_A]) => ({
      run: (() => { const s = localState.openDraft({ key: K1, run: R2, catalog: CAT_A }); return { st: s.status().state, body: s.body }; })(),
      proj: (() => { const s = localState.openDraft({ key: K2, run: R1, catalog: CAT_A }); return { st: s.status().state, body: s.body }; })(),
    }), [K1, K2, R1, R2, CAT_A]);
    ok('A4 o rascunho de uma run não aparece em outra run (9.2.3)', outra.run.st === 'empty' && outra.run.body === null, JSON.stringify(outra.run));
    ok('A4 nem em outro projeto com a mesma run', outra.proj.st === 'empty' && outra.proj.body === null, JSON.stringify(outra.proj));

    // ---- grafo trocado: catalog_sha256 diferente (5.5, passo 3; 9.2.4)
    const antesB = await guardado(page, `librarian.draft.v1:${K1}:${R1}`);
    const ed = await page.evaluate(([K1, R1, CAT_B]) => {
      const s = localState.openDraft({ key: K1, run: R1, catalog: CAT_B });
      const st = s.status(), body = s.body;
      const tentativa = s.update({ composer: { text: 'outro' } });
      return { st, body, tentativa, depois: s.status() };
    }, [K1, R1, CAT_B]);
    ok('A5 grafo trocado: o rascunho de outra edition não é aplicado', ed.st.state === 'incompatible' && ed.st.reason === 'edition' && ed.body === null, JSON.stringify({ state: ed.st.state, reason: ed.st.reason }));
    ok('A5 o rascunho incompatível continua visível para mostrar os nomes', ed.st.kept?.body?.builder?.included?.[0]?.name === 'Matrix::from');
    ok('A5 editar sem escolher não sobrescreve o rascunho da outra edition', !ed.tentativa.ok && (await guardado(page, `librarian.draft.v1:${K1}:${R1}`)) === antesB, JSON.stringify(ed.tentativa));
    ok('A5 a edição nova fica só nesta página, marcada como pendente', ed.depois.storage === 'page-only' && ed.depois.dirty, JSON.stringify(ed.depois));
    const desc = await page.evaluate(([K1, R1, CAT_B]) => {
      const s = localState.openDraft({ key: K1, run: R1, catalog: CAT_B });
      s.update({ composer: { text: 'outro' } });
      const r = s.discard();
      return { r, st: s.status(), doc: JSON.parse(localStorage.getItem(localState.draftName(K1, R1))) };
    }, [K1, R1, CAT_B]);
    ok('A5 Descartar remove o incompatível e guarda o rascunho desta edition', desc.r.ok && desc.doc.catalog_sha256 === CAT_B && desc.doc.body.composer.text === 'outro' && desc.st.state === 'draft', JSON.stringify(desc.st));

    // ---- identidade e formato (5.5, passos 1 e 2)
    const fmt = await page.evaluate(([K1, R2, CAT_A]) => {
      const nome = localState.draftName(K1, R2), res = {};
      localStorage.setItem(nome, JSON.stringify({ schema: 'librarian.draft.v1', key: 'outro-projeto', run: R2, catalog_sha256: CAT_A, tab: 'x', seq: 1, body: { a: 1 } }));
      res.identidade = localState.openDraft({ key: K1, run: R2, catalog: CAT_A }).status();
      for (const [nomeCaso, valor] of [['lixo', '{não é json'], ['esquema', JSON.stringify({ schema: 'librarian.draft.v0', key: K1, run: R2, catalog_sha256: CAT_A, body: {} })]]) {
        localStorage.setItem(nome, valor);
        const s = localState.openDraft({ key: K1, run: R2, catalog: CAT_A });
        const st = s.status(), t = s.update({ b: 2 });
        res[nomeCaso] = { state: st.state, body: s.body, tentativa: t.ok, intacto: localStorage.getItem(nome) === valor, pendente: s.status().dirty };
        s.close();
      }
      localStorage.removeItem(nome);
      return res;
    }, [K1, R2, CAT_A]);
    ok('A6 key ou run do cabeçalho diferentes: não aplica', fmt.identidade.state === 'incompatible' && fmt.identidade.reason === 'identity', JSON.stringify(fmt.identidade.state));
    ok('A6 texto ilegível: formato desconhecido, não aplica nem sobrescreve', fmt.lixo.state === 'unknown-format' && !fmt.lixo.tentativa && fmt.lixo.intacto, JSON.stringify(fmt.lixo));
    ok('A6 esquema desconhecido: não aplica nem sobrescreve', fmt.esquema.state === 'unknown-format' && !fmt.esquema.tentativa && fmt.esquema.intacto, JSON.stringify(fmt.esquema));
    ok('A6 a edição feita sem escolher fica pendente (só nesta página)', fmt.lixo.pendente && fmt.esquema.pendente);

    // ---- esvaziar
    const vazio = await page.evaluate(([K2, R2, CAT_A]) => {
      const s = localState.openDraft({ key: K2, run: R2, catalog: CAT_A });
      s.update({ x: 1 }); const tinha = localStorage.getItem(localState.draftName(K2, R2)) !== null;
      s.update(null);
      return { tinha, some: localStorage.getItem(localState.draftName(K2, R2)) === null, st: s.status() };
    }, [K2, R2, CAT_A]);
    ok('A7 esvaziar o rascunho remove a chave, sem nada pendente', vazio.tinha && vazio.some && vazio.st.state === 'empty' && !vazio.st.dirty, JSON.stringify(vazio));
    // Reabrir o mesmo rascunho substitui o anterior; fechar o antigo depois não desliga o novo.
    const reaberto = await page.evaluate(([K2, R2, CAT_A]) => {
      const antigo = localState.openDraft({ key: K2, run: R2, catalog: CAT_A }), novo = localState.openDraft({ key: K2, run: R2, catalog: CAT_A });
      antigo.close();
      novo.update({ y: 1 });
      const outro = JSON.stringify({ schema: 'librarian.draft.v1', key: K2, run: R2, catalog_sha256: CAT_A, tab: 'aba-fantasma', seq: 1, body: { y: 2 } });
      window.dispatchEvent(new StorageEvent('storage', { key: novo.name, newValue: outro }));
      const st = novo.status().state;
      novo.update(null); novo.close();
      return st;
    }, [K2, R2, CAT_A]);
    ok('A7 fechar um rascunho já substituído não desliga o que o substituiu', reaberto === 'conflict', reaberto);
    ok('A7 com o rascunho guardado, sair não pergunta nada', !(await avisaAoSair(page)));
    await ctx.close();

    // ---- duas abas (5.7; 9.2.5)
    ctx = await browser.newContext();
    const p1 = await abre(ctx, base, errors), p2 = await abre(ctx, base, errors);
    const abrir = (p, body) => p.evaluate(([K1, R1, CAT_A, body]) => {
      window.__s = localState.openDraft({ key: K1, run: R1, catalog: CAT_A });
      window.__eventos = [];
      __s.onChange(st => __eventos.push(st.state));
      if (body !== undefined) __s.update(body);
      return { st: __s.status(), body: __s.body };
    }, [K1, R1, CAT_A, body]);
    await abrir(p1, { v: 1 });
    const b2 = await abrir(p2);
    ok('A8 a segunda aba restaura o rascunho gravado pela primeira', b2.st.state === 'restored' && same(b2.body, { v: 1 }), JSON.stringify(b2));
    await p2.evaluate(() => __s.update({ v: 2 }));
    await p1.waitForFunction(() => __s.status().state === 'conflict', null, { timeout: 5000 }).catch(() => {});
    const c1 = await p1.evaluate(() => ({ st: __s.status(), body: __s.body, eventos: __eventos }));
    ok('A8 outra aba grava: esta entra em "alterado em outra aba" (9.2.5)', c1.st.state === 'conflict' && c1.eventos.includes('conflict'), JSON.stringify(c1.st.state));
    ok('A8 a versão da outra aba fica visível, sem mesclar com a desta', same(c1.st.theirs?.body, { v: 2 }) && same(c1.body, { v: 1 }), JSON.stringify({ theirs: c1.st.theirs?.body, mine: c1.body }));
    const guardadoAntes = await guardado(p1, `librarian.draft.v1:${K1}:${R1}`);
    const so = await p1.evaluate(() => __s.update({ v: 3 }));
    ok('A8 em conflito é só leitura: editar é recusado e nada é gravado', !so.ok && so.reason === 'conflict' && (await guardado(p1, `librarian.draft.v1:${K1}:${R1}`)) === guardadoAntes && same(await p1.evaluate(() => __s.body), { v: 1 }), JSON.stringify(so));
    const carregar = await p1.evaluate(() => { __s.loadTheirs(); return { st: __s.status(), body: __s.body }; });
    ok('A8 "Carregar a versão da outra aba" traz exatamente a dela', carregar.st.state === 'restored' && same(carregar.body, { v: 2 }), JSON.stringify(carregar));
    await p1.evaluate(() => __s.update({ v: 4 }));
    await p2.waitForFunction(() => __s.status().state === 'conflict', null, { timeout: 5000 }).catch(() => {});
    const manter = await p2.evaluate(() => { const antes = __s.status().state; const r = __s.keepMine(); return { antes, r, st: __s.status(), doc: JSON.parse(localStorage.getItem(__s.name)) }; });
    ok('A8 "Manter a minha (sobrescrever)" grava a desta aba inteira, sem mesclar', manter.antes === 'conflict' && manter.r.ok && same(manter.doc.body, { v: 2 }) && manter.st.state === 'draft', JSON.stringify(manter));
    await p1.waitForFunction(() => __s.status().state === 'conflict', null, { timeout: 5000 }).catch(() => {});
    ok('A8 e a primeira aba passa a "alterado em outra aba"', (await p1.evaluate(() => __s.status().state)) === 'conflict');
    // Evento perdido: outra versão no armazenamento sem aviso (gravada aqui, onde o evento não chega).
    const perdido = await p2.evaluate(() => {
      const outro = JSON.stringify({ ...JSON.parse(localStorage.getItem(__s.name)), tab: 'aba-fantasma', seq: 99, body: { v: 'fantasma' } });
      localStorage.setItem(__s.name, outro);
      const r = __s.update({ v: 5 });
      return { r, st: __s.status().state, intacto: localStorage.getItem(__s.name) === outro };
    });
    ok('A8 sem o evento, gravar confere a versão guardada e não sobrescreve a de outra aba', !perdido.r.ok && perdido.st === 'conflict' && perdido.intacto, JSON.stringify(perdido));
    await p2.evaluate(() => localStorage.clear());
    await p1.evaluate(() => __s.loadTheirs());
    await p2.evaluate(() => { __s.loadTheirs(); __s.update({ v: 6 }); });
    await p1.waitForFunction(() => __s.status().state === 'conflict', null, { timeout: 5000 }).catch(() => {});
    await p2.evaluate(() => localStorage.clear());
    await p1.waitForFunction(() => __s.status().theirs === null, null, { timeout: 5000 }).catch(() => {});
    ok('A8 armazenamento limpo em outra aba também é "alterado em outra aba"', (await p1.evaluate(() => __s.status())).theirs === null);
    await ctx.close();

    // ---- armazenamento bloqueado (5.8; 9.2.6)
    ctx = await browser.newContext();
    await ctx.addInitScript(BLOQUEIA);
    const erros = [];
    page = await abre(ctx, base, erros);
    const blq = await page.evaluate(([K1, R1, CAT_A]) => {
      const s = localState.openDraft({ key: K1, run: R1, catalog: CAT_A });
      const antes = s.status(), r = s.update({ composer: { text: 'só aqui' } });
      return { antes, r, depois: s.status(), body: s.body, nav: localState.nav.read(K1), navw: localState.nav.update(K1, { run: R1 }),
        lang: document.documentElement.lang, h1: document.querySelector('h1')?.textContent };
    }, [K1, R1, CAT_A]);
    ok('A9 armazenamento bloqueado: a página carrega sem erro', erros.length === 0 && /^(pt-BR|en)$/.test(blq.lang) && !!blq.h1, erros.join(' | '));
    ok('A9 o rascunho fica "só nesta página", com o motivo', blq.antes.storage === 'page-only' && blq.antes.storage_reason === 'unavailable', JSON.stringify(blq.antes));
    ok('A9 editar funciona na memória, sem exceção', !blq.r.ok && blq.r.reason === 'unavailable' && same(blq.body, { composer: { text: 'só aqui' } }) && blq.depois.dirty, JSON.stringify(blq.r));
    ok('A9 a navegação também falha sem exceção', !blq.nav.ok && !blq.navw.ok, JSON.stringify([blq.nav, blq.navw]));
    ok('A9 com rascunho não vazio só nesta página, sair pede confirmação (9.2.6)', await avisaAoSair(page));
    page = await abre(ctx, base, erros);
    await page.evaluate(([K1, R1, CAT_A]) => localState.openDraft({ key: K1, run: R1, catalog: CAT_A }), [K1, R1, CAT_A]);
    ok('A9 com o rascunho vazio, sair não pergunta nada', !(await avisaAoSair(page)));
    await ctx.close();

    // ---- armazenamento cheio (5.8)
    ctx = await browser.newContext();
    await ctx.addInitScript(ENCHE);
    page = await abre(ctx, base, errors);
    const cheio = await page.evaluate(([K1, R1, CAT_A]) => {
      const s = localState.openDraft({ key: K1, run: R1, catalog: CAT_A });
      s.update({ v: 1 }); const v1 = localStorage.getItem(s.name);
      window.__cheio = true;
      const r = s.update({ v: 2 }), st = s.status(), mantido = localStorage.getItem(s.name) === v1;
      window.__cheio = false;
      const r3 = s.update({ v: 3 });
      window.__s = s;
      return { r, st, mantido, r3, st3: s.status(), body: JSON.parse(localStorage.getItem(s.name)).body };
    }, [K1, R1, CAT_A]);
    ok('A10 cota excedida: "só nesta página", com o motivo', !cheio.r.ok && cheio.r.reason === 'quota' && cheio.st.storage === 'page-only' && cheio.st.storage_reason === 'quota' && cheio.st.dirty, JSON.stringify(cheio.st));
    ok('A10 a última versão guardada fica como estava', cheio.mantido);
    ok('A10 com espaço de novo, a gravação seguinte volta a guardar', cheio.r3.ok && cheio.st3.storage === 'stored' && !cheio.st3.dirty && same(cheio.body, { v: 3 }), JSON.stringify(cheio.st3));
    await page.evaluate(() => { window.__cheio = true; __s.update({ v: 4 }); });
    ok('A10 com a última edição sem guardar, sair pede confirmação', await avisaAoSair(page));
    await ctx.close();
  } finally { servidor.close(); }
}

async function parteB(browser) {
  const B = 'http://127.0.0.1:8841';
  const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS, RENDERER = process.env.LIBRARIAN_TESTE_RENDERER;
  const CHAVE = process.env.LIBRARIAN_TESTE_CHAVE_RENDERER || 'renderer-librarian-26c4a0fd';
  try { const r = await fetch(`${B}/api/status`); await r.arrayBuffer(); if (!r.ok) throw 0; } catch { console.log('-- parte B pulada: sem servidor em 8841'); return; }
  if (!RAIZ || !RENDERER) { console.log('-- parte B pulada: faltam LIBRARIAN_TESTE_RAIZ_RS e LIBRARIAN_TESTE_RENDERER'); return; }
  const perfil = path.join(RAIZ, '.studio/projects', CHAVE, 'project.json');
  const sha = () => crypto.createHash('sha256').update(fs.readFileSync(perfil)).digest('hex');
  const ABORTA = /\/api\/(send|ladder|generate|instrument\/run|commands\/run|commands\/approve|paste|review|continue|gathered-question|ask|path-questions|lexicon\/suggest|project\/export)\b/;
  const antes = sha();
  for (const bloqueado of [false, true]) {
    const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    if (bloqueado) await ctx.addInitScript(BLOQUEIA);
    const page = await ctx.newPage(), erros = [], escritas = [];
    page.on('pageerror', e => erros.push(e.message));
    page.on('request', r => { if (r.url().includes('/api/') && r.method() !== 'GET' && !r.url().endsWith('/api/project') && !r.url().endsWith('/api/calls')) escritas.push(`${r.method()} ${new URL(r.url()).pathname}`); });
    page.on('request', r => { if (r.url().endsWith('/api/project') && r.method() === 'PUT') escritas.push('PUT /api/project'); });
    await page.route(ABORTA, r => r.abort());
    const nome = bloqueado ? 'B2 (armazenamento bloqueado)' : 'B1';
    await page.goto(`${B}/`);
    await page.waitForFunction(() => document.body.classList.contains('studio'), null, { timeout: 60000 });
    let turnos = page.waitForResponse(r => r.url().includes('/api/turns'), { timeout: 120000 });
    await page.fill('#projectPath', RENDERER);
    await page.click('#openProject');
    await turnos;
    const runs = await page.$$eval('#runs option', os => os.map(o => o.value));
    const atual = await page.$eval('#runs', s => s.value);
    const outra = runs.find(r => r && r !== atual);
    ok(`${nome} abrir o projeto carrega a run mais recente, como antes`, atual === runs[0], `${atual} de ${runs.join(',')}`);
    turnos = page.waitForResponse(r => r.url().includes('/api/turns'), { timeout: 120000 });
    await page.selectOption('#runs', outra);
    await turnos;
    ok(`${nome} trocar de run funciona`, (await page.$eval('#runs', s => s.value)) === outra);
    if (!bloqueado) {
      // F9 (2.1, T17, 5.3): trocar de run é uma escolha da pessoa e guarda a run na navegação do projeto.
      const navTroca = await page.evaluate(k => localStorage.getItem(`librarian.nav.v1:${k}`), CHAVE);
      turnos = page.waitForResponse(r => r.url().includes('/api/turns'), { timeout: 120000 });
      await page.reload();
      await turnos;
      ok('B1 recarregar volta à run escolhida pela pessoa (F9: a run guardada do projeto)', (await page.$eval('#runs', s => s.value)) === outra);
      // Abrir e recarregar não gravam navegação: ela só é gravada num movimento ou numa escolha da pessoa, para
      // nunca levar o ponto de um projeto a outro (correção do F2). Trocar de run grava só a run (F9).
      const semMovimento = await page.evaluate(() => Object.keys(localStorage).sort());
      const navRecarga = await page.evaluate(k => localStorage.getItem(`librarian.nav.v1:${k}`), CHAVE);
      ok('B1 abrir e recarregar não gravam navegação nem rascunho; trocar de run grava só a run, na navegação do projeto (F9)', semMovimento.filter(k => F1_KEY.test(k)).join() === `librarian.nav.v1:${CHAVE}`
        && navRecarga === navTroca && JSON.parse(navTroca ?? '{}').run === outra && JSON.parse(navTroca ?? '{}').area === null && !/mode|allow_/.test(navTroca ?? ''), `${semMovimento.join(',')} ${navTroca} ${navRecarga}`);
      await page.click('#areas [data-area="ruler"]');
      const ks = await page.evaluate(() => Object.keys(localStorage).sort());
      const nav = await page.evaluate(k => localStorage.getItem(`librarian.nav.v1:${k}`), CHAVE);
      ok('B1 um movimento grava só a navegação do projeto (F2), com área e vista de Explorar, e nenhum rascunho', !ks.some(k => /^librarian\.draft\.v1:/.test(k))
        && ks.filter(k => F1_KEY.test(k)).join() === `librarian.nav.v1:${CHAVE}` && /"area":"ruler"/.test(nav ?? '') && !/mode|allow_/.test(nav ?? ''), `${ks.join(',')} ${nav}`);
      ok('B1 as chaves no navegador são as de antes, mais a navegação, e sem librarian.tab (F2)', ks.every(k => ['librarian.lang', 'librarian.mode', 'librarian.project'].includes(k) || k.startsWith('librarian.llm.') || k === 'librarian.skills' || k === `librarian.nav.v1:${CHAVE}`), ks.join(','));
    }
    ok(`${nome} sem erro na página`, erros.length === 0, erros.join(' | '));
    ok(`${nome} abrir, trocar de run e recarregar não gravam nada no servidor`, escritas.length === 0, escritas.join(','));
    await ctx.close();
  }
  ok('B3 o perfil (project.json) fica igual em bytes', sha() === antes);
}

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  try { await parteA(browser); await parteB(browser); } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exit(1); });

// F8: a conversa e o histórico (especificação 4.7, 4.8, 7.4, 8.4 item 8; critérios 9.3.4, 9.9 e 9.11), na página
// real, num Chrome sem janela, contra o servidor de rodar.js (8841).
// Casos: a linha do tempo na ordem dos números e os estados derivados do registro de cada pergunta, conferidos
// contra /api/turns, /api/executions e /api/facts calculados aqui; a política e os critérios usados pela pergunta,
// separados do padrão atual; cada envio em três camadas; LACUNA e INFERENCE sem "Aprovar", com o motivo; as
// execuções e os fatos de cada pergunta; Copiar diff (bytes iguais a check.diffs[claim], linhas e arquivos,
// nenhuma escrita no projeto nem em .studio, área de transferência recusada, proposta sem diff derivado); a
// pergunta ativa guardada por run; respostas atrasadas e de outra run ou projeto (perguntar, enviar, cadeia,
// decidir, apagar, rodar com a mudança); duas perguntas em andamento; os estados vazio, carregando e ilegível;
// a resposta colada não conferida; falhas de leitura; tela estreita, PT e EN, temas, contraste e teclado.
// Nada é enviado a modelo e nada é gravado no servidor: as escritas que a bateria precisa ver são respondidas
// aqui (algumas seguradas até a troca de run); as outras são abortadas e contadas.
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { chromium, RENDERER, CHAVE_RENDERER } = require('./ambiente');
const B = 'http://127.0.0.1:8841', RUN = '20261104-020244', OUTRA = '20261104-014710';
const RAIZ = process.env.LIBRARIAN_TESTE_RAIZ_RS;
const HOOK = "window.__t = { state, studio, t, STRINGS, loadRun, activate, renderTurns, sendTurn, continueTurn, askModel, setLang, localState,"
  + " get askSeq() { return askSeq; } }; if (location.hash === '#debug')";
const ok = (n, pass, d = '') => { console.log(`${pass ? 'PASSA' : 'FALHA'}  ${n}${d ? '  -> ' + String(d).slice(0, 300) : ''}`); if (!pass) process.exitCode = 1; };
const perfil = path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'project.json');
const sha = b => crypto.createHash('sha256').update(b).digest('hex');
const NAV = `librarian.nav.v1:${CHAVE_RENDERER}`;
const get = async p => (await fetch(B + p)).json();
const num = id => (/^\d+$/.test(id) ? Number(id) : Infinity);
const segura = () => { let solta; const p = new Promise(r => { solta = r; }); return { p, solta }; };
const LEITURA = (method, pathname) => method === 'GET'
  || (method === 'POST' && ['/api/project', '/api/calls', '/api/envelope/suggest', '/api/inventory/investigate', '/api/performance-question'].includes(pathname));
const contraste = cores => {
  const L = k => { const m = k.match(/[\d.]+/g).slice(0, 3).map(Number).map(v => v / 255).map(v => v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4); return 0.2126 * m[0] + 0.7152 * m[1] + 0.0722 * m[2]; };
  const [hi, lo] = [L(cores[0]), L(cores[1])].sort((p, q) => q - p);
  return (hi + 0.05) / (lo + 0.05);
};
// Uma fotografia de uma árvore (caminho, tamanho e data de cada arquivo): Copiar diff não pode escrever nada.
function arvore(raiz, fora = ['.git', 'target', 'node_modules']) {
  const out = [];
  const anda = d => { for (const e of fs.readdirSync(d, { withFileTypes: true })) { if (fora.includes(e.name)) continue; const p = path.join(d, e.name);
    if (e.isDirectory()) anda(p); else { const s = fs.statSync(p); out.push(`${path.relative(raiz, p)}|${s.size}|${s.mtimeMs}`); } } };
  anda(raiz);
  return sha(out.sort().join('\n'));
}

// Os estados de 4.7 calculados aqui, do registro, sem a página.
function estados(turn, execs) {
  const sends = turn.sends ?? [], reviews = sends.flatMap(s => Object.values(s.reviews ?? {}));
  const respondidos = sends.filter(s => s.text);
  return [
    turn.prepare_error ? 'refused' : null,
    !turn.prepare_error && !sends.length ? 'prepared' : null,
    respondidos.length ? 'answered' : null,
    sends.length && !respondidos.length ? 'noText' : null,
    sends.some(s => (s.check?.claims ?? []).some(k => !s.reviews?.[k.id])) ? 'pending' : null,
    reviews.some(r => r.decision === 'approved') ? 'approved' : null,
    reviews.some(r => r.decision === 'rejected') ? 'rejected' : null,
    execs.some(x => x.change && String(x.change.turn) === String(turn.id)) ? 'executed' : null,
  ].filter(Boolean).join();
}
const arquivos = diff => { const f = []; for (const l of diff.split('\n')) { const m = l.match(/^\+\+\+ b\/(.+)$/); if (m && !f.includes(m[1])) f.push(m[1]); } return f; };

async function pagina(browser, { size = { width: 1440, height: 900 }, scheme = 'light', locale = 'pt-BR', storage = null, run = RUN, antes = null } = {}) {
  const context = await browser.newContext({ viewport: size, colorScheme: scheme, locale });
  await context.grantPermissions(['clipboard-read', 'clipboard-write'], { origin: B });
  if (storage) await context.addInitScript(s => { if (!sessionStorage.getItem('f8.semeado')) { for (const [k, v] of Object.entries(s)) localStorage.setItem(k, v); sessionStorage.setItem('f8.semeado', '1'); } }, storage);
  // O que a página entrega à área de transferência, como veio (no Windows, ler de volta troca LF por CRLF).
  await context.addInitScript(() => { const c = navigator.clipboard; if (!c) return; const w = c.writeText.bind(c); window.__copiado = []; c.writeText = v => { window.__copiado.push(v); return w(v); }; });
  const page = await context.newPage();
  const x = { page, context, errors: [], writes: [], reqs: [], fake: {} };
  page.on('pageerror', e => x.errors.push(e.message));
  await page.route(u => new URL(u).pathname === '/', async r => { const y = await r.fetch(); r.fulfill({ response: y, body: (await y.text()).replace("if (location.hash === '#debug')", HOOK) }); });
  await page.route(u => new URL(u).pathname.startsWith('/api/'), async r => {
    const req = r.request(), url = new URL(req.url()), { pathname } = url, method = req.method(), chave = `${method} ${pathname}`;
    const raw = req.postData(), body = raw ? JSON.parse(raw) : null;
    x.reqs.push({ chave, url: req.url(), raw, body, run: url.searchParams.get('run') });
    // O projeto fictício (6g) não existe no servidor: as leituras dele são as do renderer.
    if (method === 'GET' && url.searchParams.get('key') === 'ficticio-f8-00000000') { url.searchParams.set('key', CHAVE_RENDERER); return r.continue({ url: url.toString() }); }
    const f = x.fake[chave];
    if (f) { const feito = await f(r, body, url); if (feito !== false) return; }
    if (LEITURA(method, pathname)) return r.continue();
    x.writes.push(chave);
    return r.abort();
  });
  if (antes) await antes(x);
  await page.goto(`${B}/`);
  await page.waitForFunction(() => window.__t && __t.studio.status, null, { timeout: 60000 });
  if (await page.evaluate(() => __t.studio.project?.key) !== CHAVE_RENDERER) await abre(page, RENDERER, CHAVE_RENDERER);
  await page.waitForFunction(() => __t.studio.run && __t.state.nodes.length, null, { timeout: 120000 });
  if (run && await page.evaluate(() => __t.studio.run) !== run) await trocaRun(page, run);
  await pronta(page);
  return x;
}
const pronta = page => page.waitForFunction(() => __t.studio.turnsState !== 'loading' && (__t.studio.facts || __t.studio.factsError) && (__t.studio.executions || __t.studio.executionsError), null, { timeout: 120000 }).then(() => page.waitForTimeout(400));
async function abre(page, root, key) {
  await page.evaluate(r => { document.querySelector('#projectPath').value = r; document.querySelector('#openProject').click(); }, root);
  await page.waitForFunction(k => __t.studio.project?.key === k && !document.querySelector('#openProject').disabled, key, { timeout: 120000 });
  await page.waitForTimeout(600);
}
async function trocaRun(page, run, { espera = true } = {}) {
  await page.selectOption('#runs', run);
  if (!espera) return;
  await page.waitForFunction(r => __t.studio.run === r && __t.state.nodes.length && __t.state.graph && __t.studio.turnsState !== 'loading', run, { timeout: 120000 });
  await pronta(page);
}
const fecha = x => x.context.close();
const dialogo = page => page.waitForFunction(() => document.querySelector('#askDialog').open, null, { timeout: 30000 });
const confirma = async page => { await page.click('#askOk'); await page.waitForTimeout(300); };
const nota = page => page.evaluate(() => document.querySelector('#chatNote').textContent);
const T = (page, k, v = {}) => page.evaluate(([k, v]) => __t.t(k, v), [k, v]);
const expande = (page, id) => page.evaluate(id => { const x = __t.studio.turns.find(t => t.id === id); x.expanded = true; __t.renderTurns(); }, id);
const turnoDom = (page, id, sel) => page.evaluate(([id, sel]) => { const d = document.querySelector(`#turns .turn[data-turn="${id}"]`); const e = d && (sel ? d.querySelector(sel) : d); return e ? e.textContent : null; }, [id, sel]);
// Uma pergunta falsa (cópia de uma real, com outro id e sem envios), para as criações respondidas aqui.
const falsa = (base, id, extra = {}) => ({ ...structuredClone(base), id, question: `Pergunta F8 ${id}`, sends: [], comparison: null, ...extra });

(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const perfilAntes = sha(fs.readFileSync(perfil));
  const turns = await get(`/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`), turnsOutra = await get(`/api/turns?key=${CHAVE_RENDERER}&run=${OUTRA}`);
  const execs = await get(`/api/executions?key=${CHAVE_RENDERER}&run=${RUN}`), facts = (await get(`/api/facts?key=${CHAVE_RENDERER}&run=${RUN}`)).facts;
  const project = await get(`/api/project?key=${CHAVE_RENDERER}`);
  const ultimo = [...turns].sort((a, b) => num(a.id) - num(b.id)).at(-1);
  const todosErros = [], todasEscritas = [];
  const junta = x => { todosErros.push(...x.errors); todasEscritas.push(...x.writes); };
  let x;

  // ================================================================ 1. linha do tempo, estados e histórico (4.7, 9.11)
  x = await pagina(browser);
  {
    const ordem = await x.page.evaluate(() => [...document.querySelectorAll('#turns > .turn')].map(d => d.dataset.turn));
    const esperada = turns.map(t => t.id).sort((a, b) => num(a) - num(b));
    ok('1 4.7 a linha do tempo tem as perguntas da run, na ordem dos números', JSON.stringify(ordem) === JSON.stringify(esperada), ordem.slice(8, 14).join());
    ok('1 o servidor lista em ordem de texto ("100" antes de "11"): a página reordena', turns.map(t => t.id).join() !== esperada.join());
    ok('1 sem pergunta guardada, a ativa é a de maior número', await x.page.evaluate(() => __t.studio.activeTurn) === ultimo.id, await x.page.evaluate(() => __t.studio.activeTurn));
    const dom = await x.page.evaluate(() => Object.fromEntries([...document.querySelectorAll('#turns > .turn')].map(d => [d.dataset.turn, [...d.querySelector('.turn-sum').querySelectorAll('.state-tag')].map(s => s.dataset.state).join()])));
    const errados = turns.filter(t => dom[t.id] !== estados(t, execs));
    ok(`1 4.7 os estados de cada uma das ${turns.length} perguntas são os do registro`, !errados.length, errados.slice(0, 3).map(t => `${t.id}: ${dom[t.id]} ≠ ${estados(t, execs)}`).join('; '));
    ok('1 há perguntas respondidas, com decisões pendentes, com fato aprovado e com mudança executada', ['answered', 'pending', 'approved', 'executed'].every(s => Object.values(dom).some(v => v.split(',').includes(s))));
    const textoEstado = await x.page.evaluate(() => [...document.querySelectorAll('#turns .state-tag')].map(s => s.textContent));
    ok('1 7.1 os estados são escritos, não só cor', textoEstado.length && textoEstado.every(s => s.trim().length > 3), textoEstado.slice(0, 3).join(' | '));

    // 9.11: perguntas anteriores à Etapa 4, formato 1 e formato 2.
    const marca = id => { try { return JSON.parse(fs.readFileSync(path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'runs', RUN, 'turns', id, 'question.json'), 'utf8')).evidence ?? null; } catch { return undefined; } };
    const formato = id => { try { return JSON.parse(fs.readFileSync(path.join(RAIZ, '.studio/projects', CHAVE_RENDERER, 'runs', RUN, 'turns', id, 'evidence.json'), 'utf8')).format_version; } catch { return null; } };
    const semPacote = turns.find(t => marca(t.id) === null), f1 = turns.find(t => marca(t.id) && formato(t.id) === 1), f2 = turns.find(t => marca(t.id) && formato(t.id) === 2);
    for (const [nome, t] of [['anterior à Etapa 4 (sem pacote)', semPacote], ['pacote de formato 1', f1], ['pacote de formato 2', f2]]) {
      if (!t) { ok(`1 9.11 ${nome}: existe na run`, false); continue; }
      await expande(x.page, t.id);
      const r = await x.page.evaluate(id => { const d = document.querySelector(`#turns .turn[data-turn="${id}"]`);
        return { usado: Boolean(d.querySelector('.used')), evid: Boolean(d.querySelector('details.evidence')), camadas: d.querySelectorAll('.bubble.reply .layer').length, envios: d.querySelectorAll('.bubble.reply').length, reg: Boolean(d.querySelector('.records')) }; }, t.id);
      ok(`1 9.11 ${nome} (#${t.id}): pergunta, pacote, envios em três camadas e registros`, r.usado && r.evid && r.reg && r.camadas === 3 * t.sends.length && r.envios === t.sends.length, JSON.stringify(r));
    }

    // 4.7 item 4: as execuções e os fatos de cada pergunta.
    const porTurno = {}; for (const e of execs) if (e.change) porTurno[e.change.turn] = (porTurno[e.change.turn] ?? 0) + 1;
    const [maisExec, n] = Object.entries(porTurno).sort((a, b) => b[1] - a[1])[0];
    await expande(x.page, maisExec);
    const reg = await x.page.evaluate(id => { const d = document.querySelector(`#turns .turn[data-turn="${id}"] .records`);
      return { n: d.querySelectorAll('.records-runs > li').length, open: d.open, sum: d.querySelector('summary').textContent, linhas: [...d.querySelectorAll('.records-runs > li > details > summary')].map(s => s.textContent) }; }, maisExec);
    ok(`1 4.7 a pergunta #${maisExec} lista as ${n} execuções com a mudança dela`, reg.n === n && reg.open && reg.sum === await T(x.page, 'records.title', { runs: n, facts: facts.filter(f => f.run === RUN && f.turn === maisExec).length }), JSON.stringify(reg).slice(0, 200));
    const linhas = await x.page.evaluate(id => [...document.querySelectorAll(`#turns .turn[data-turn="${id}"] .records-runs > li > details > summary`)].every(s => s.textContent.includes(__t.t('executions.changeShort', { turn: id }))), maisExec);
    ok('1 cada linha diz a mudança da pergunta', linhas);
    const semMudanca = execs.filter(e => !e.change).length;
    await x.page.evaluate(() => { for (const t of __t.studio.turns) t.expanded = true; __t.renderTurns(); });
    const totalDom = await x.page.evaluate(() => document.querySelectorAll('#turns .records-runs > li').length);
    const comMudanca = execs.filter(e => e.change && turns.some(t => t.id === String(e.change.turn))).length;
    ok(`1 9.11 execuções sem mudança (${semMudanca}) não aparecem em nenhuma pergunta; as ${comMudanca} com mudança, cada uma na sua`, totalDom === comMudanca, `${totalDom}`);
    const fatoAqui = facts.find(f => f.run === RUN);
    const fatosDom = await x.page.evaluate(id => [...document.querySelectorAll(`#turns .turn[data-turn="${id}"] .records-facts > li`)].map(l => l.textContent), fatoAqui.turn);
    ok(`1 4.7 o fato ${fatoAqui.id} aparece na pergunta #${fatoAqui.turn}, com o estado`, fatosDom.length === facts.filter(f => f.run === RUN && f.turn === fatoAqui.turn).length && fatosDom[0].includes(fatoAqui.id) && fatosDom[0].includes(await T(x.page, `facts.${fatoAqui.status}`)), fatosDom.join(' | '));
    const fatosOutros = await x.page.evaluate(() => document.querySelectorAll('#turns .records-facts > li').length);
    ok('1 fatos de outras runs com o mesmo número de pergunta não aparecem', fatosOutros === facts.filter(f => f.run === RUN).length, `${fatosOutros}`);
  }
  junta(x); await fecha(x);

  // 9.11.2: fatos desatualizados e revogados (o acervo respondido aqui, com os estados trocados).
  {
    const alvo = facts.find(f => f.run === RUN);
    x = await pagina(browser, { antes: async x => {
      x.fake['GET /api/facts'] = async (r, _b, url) => { if (url.searchParams.get('run') !== RUN) return false;
        const f = structuredClone(facts); f.push({ ...structuredClone(alvo), id: 'F-rev', status: 'revoked', status_reason: 'revogado pela bateria F8' });
        f.find(y => y.id === alvo.id).status = 'stale'; f.find(y => y.id === alvo.id).status_reason = 'desatualizado pela bateria F8';
        return r.fulfill({ json: { facts: f, limits: '' } }); };
    } });
    await expande(x.page, alvo.turn);
    const li = await x.page.evaluate(id => [...document.querySelectorAll(`#turns .turn[data-turn="${id}"] .records-facts > li`)].map(l => l.textContent), alvo.turn);
    ok('1 9.11.2 fato desatualizado e fato revogado aparecem com o estado e o motivo', li.some(s => s.includes(alvo.id) && s.includes('desatualizado') && s.includes('desatualizado pela bateria F8')) && li.some(s => s.includes('F-rev') && s.includes('revogado') && s.includes('revogado pela bateria F8')), li.join(' | '));
    junta(x); await fecha(x);
  }

  // ================================================================ 2. política e critérios usados (9.3.4)
  x = await pagina(browser);
  {
    const q = JSON.parse(ultimo.query);
    await expande(x.page, ultimo.id);
    const u = await x.page.evaluate(id => { const d = document.querySelector(`#turns .turn[data-turn="${id}"] .used`);
      return { pol: d.querySelector('.used-policy').textContent, agora: d.querySelector('.used-policy-now')?.textContent, crit: d.querySelector('.used-criteria summary')?.textContent,
        texto: d.querySelector('.used-criteria pre')?.textContent, critAgora: d.querySelector('.used-criteria-now')?.textContent, label: d.children[1].textContent }; }, ultimo.id);
    // allow_edits (ou o antigo allow_patch) só é escrito na query quando é verdadeiro.
    const patch = q.allow_edits === true || q.allow_patch === true;
    const modo = q.policy.allow_hypothesis && patch ? 'improve' : !q.policy.allow_hypothesis && !patch ? 'locate' : null;
    const pol = modo ? await T(x.page, `mode.${modo}`) : await T(x.page, 'mode.custom', { h: await T(x.page, q.policy.allow_hypothesis ? 'field.yes' : 'field.no'), p: await T(x.page, patch ? 'field.yes' : 'field.no') });
    ok('2 9.3.4 a política gravada com a pergunta (da query de claims)', u.pol === await T(x.page, 'used.policy', { policy: pol, inference: await T(x.page, q.policy.allow_inference ? 'field.yes' : 'field.no') }), u.pol);
    const mesmaPol = project.settings.allow_hypothesis === q.policy.allow_hypothesis && project.settings.allow_patch === patch;
    ok('2 9.3.4 separada da política atual do projeto', u.agora === (mesmaPol ? await T(x.page, 'used.policySame') : await T(x.page, 'used.policyDiffers', { current: await T(x.page, `mode.${project.settings.allow_hypothesis ? 'improve' : 'locate'}`) })), u.agora);
    ok('2 9.3.4 os critérios usados: o texto e o hash gravados com a pergunta', u.texto === ultimo.criteria && u.crit.includes(ultimo.criteria_sha256.slice(0, 12)), u.crit);
    ok('2 9.3.4 comparados com o padrão atual', u.critAgora === await T(x.page, project.settings.criteria === ultimo.criteria ? 'used.criteriaSame' : 'used.criteriaDiffer'), u.critAgora);
    ok('2 4.1 rótulo "Gravados com a pergunta; podem diferir do perfil atual"', u.label === 'Gravados com a pergunta; podem diferir do perfil atual', u.label);
  }
  junta(x); await fecha(x);
  // O padrão e a política atuais mudam (perfil respondido aqui): a pergunta continua mostrando os dela.
  x = await pagina(browser, { antes: async x => {
    const outro = { ...structuredClone(project.settings), criteria: 'Padrão novo da bateria F8', allow_hypothesis: !project.settings.allow_hypothesis, allow_patch: !project.settings.allow_patch };
    x.fake['POST /api/project'] = r => r.fulfill({ json: { key: CHAVE_RENDERER, settings: outro, runs: project.runs } });
    x.fake['GET /api/project'] = r => r.fulfill({ json: { key: CHAVE_RENDERER, settings: outro, runs: project.runs } });
  } });
  {
    await expande(x.page, ultimo.id);
    const u = await x.page.evaluate(id => { const d = document.querySelector(`#turns .turn[data-turn="${id}"] .used`);
      return { texto: d.querySelector('.used-criteria pre')?.textContent, critAgora: d.querySelector('.used-criteria-now')?.textContent, padrao: d.querySelector('.used-default pre')?.textContent, agora: d.querySelector('.used-policy-now')?.textContent }; }, ultimo.id);
    ok('2 9.3.4 com o padrão mudado: os usados continuam os da pergunta', u.texto === ultimo.criteria);
    ok('2 9.3.4 "Diferentes dos critérios padrão atuais", com o padrão atual à parte', u.critAgora === 'Diferentes dos critérios padrão atuais do projeto.' && u.padrao === 'Padrão novo da bateria F8', JSON.stringify(u));
    ok('2 com a política mudada: "A política atual do projeto é outra"', u.agora?.startsWith('A política atual do projeto é outra:'), u.agora);
    ok('2 o rascunho do compositor não aparece como critério usado', !(await turnoDom(x.page, ultimo.id, '.used')).includes('Diferem do padrão'));
  }
  junta(x); await fecha(x);
  // Pergunta em prosa e pergunta sem query de claims (registros respondidos aqui).
  x = await pagina(browser, { antes: async x => {
    x.fake['GET /api/turns'] = async (r, _b, url) => { if (url.searchParams.get('run') !== RUN) return false;
      const t = structuredClone(turns), a = t.find(y => y.id === '03'), b = t.find(y => y.id === '05');
      a.answer_format = 'prose'; for (const s of a.sends) { delete s.check; s.skills = [{ name: 'skill-f8', sha256: 'abcdef0123456789abcd' }]; }
      b.claims_error = 'claims recusado pela bateria'; b.query = null; b.criteria = null; b.criteria_sha256 = null;
      const c = t.find(y => y.id === '04'); c.sends[0].check.claims[0].kind = 'INFERENCE'; c.sends[0].check.claims.push({ ...c.sends[0].check.claims[0], id: 'H-sem', kind: 'HYPOTHESIS', edits: null });
      return r.fulfill({ json: t }); };
  } });
  {
    for (const id of ['03', '04', '05']) await expande(x.page, id);
    ok('2 pergunta em prosa: sem política de claims, dito', await turnoDom(x.page, '03', '.used-policy') === await T(x.page, 'used.policyProse'));
    ok('2 pergunta sem query de claims: política não gravada; sem critérios gravados', await turnoDom(x.page, '05', '.used-policy') === await T(x.page, 'used.policyNone') && await turnoDom(x.page, '05', '.used-criteria') === await T(x.page, 'used.noCriteria'));
    // 3 camadas: prosa, skills com hash, INFERENCE e hipótese sem mudança.
    const prosa = await x.page.evaluate(() => { const d = document.querySelector('#turns .turn[data-turn="03"] .bubble.reply'); return { chk: d.querySelector('.layer-checked').textContent, dec: d.querySelector('.layer-decide').textContent, chegou: d.querySelector('.layer-arrived').textContent }; });
    ok('3 resposta em prosa: camada 2 diz que não há conferência de itens; camada 3, que não há o que decidir', prosa.chk.includes(await T(x.page, 'layer.notClaims')) && prosa.dec.includes(await T(x.page, 'decide.notClaims')));
    ok('3 camada 1: skills com o hash gravado no envio', prosa.chegou.includes('skill-f8 (sha256 abcdef012345…)'), prosa.chegou.slice(0, 200));
    const dec = await x.page.evaluate(() => [...document.querySelectorAll('#turns .turn[data-turn="04"] .decisions > li')].map(l => ({ kind: l.dataset.kind, txt: l.textContent, aprovar: [...l.querySelectorAll('button')].some(b => b.textContent.startsWith(__t.t('review.approve').slice(0, 7))) })));
    const inf = dec.find(d => d.kind === 'INFERENCE'), hip = dec.find(d => d.kind === 'HYPOTHESIS');
    ok('3 [A] INFERENCE sem "Aprovar", com o motivo', inf && !inf.aprovar && inf.txt.includes(await T(x.page, 'review.why.inference')), inf?.txt);
    ok('3 hipótese sem mudança derivada sem "Aprovar", com o motivo', hip && !hip.aprovar && hip.txt.includes(await T(x.page, 'review.why.hypothesis')), hip?.txt);
  }
  junta(x); await fecha(x);

  // ================================================================ 3. três camadas e Copiar diff (4.7, 4.8, 9.9)
  const comDiff = turns.find(t => t.sends.some(s => (s.check?.claims ?? []).some(k => k.edits && s.check.diffs?.[k.id])));
  const envio = comDiff.sends.find(s => (s.check?.claims ?? []).some(k => k.edits && s.check.diffs?.[k.id]));
  const claim = envio.check.claims.find(k => k.edits && envio.check.diffs?.[k.id]), diff = envio.check.diffs[claim.id];
  const semDiff = turns.flatMap(t => t.sends.flatMap(s => (s.check?.claims ?? []).filter(k => k.edits && !s.check.diffs?.[k.id]).map(k => [t, s, k])))[0];
  const lacuna = turns.flatMap(t => t.sends.flatMap(s => (s.check?.claims ?? []).filter(k => k.kind === 'LACUNA' && !s.reviews?.[k.id]).map(k => [t, s, k])))[0];
  x = await pagina(browser);
  {
    await expande(x.page, comDiff.id);
    const c = await x.page.evaluate(([id, send]) => { const r = document.querySelector(`#turns .turn[data-turn="${id}"] .bubble.reply[data-send="${send}"]`);
      return { titulos: [...r.querySelectorAll('.layer-title')].map(e => e.textContent), aviso: r.querySelector('.layer-checked .layer-warning')?.textContent,
        decisoes: r.querySelectorAll('.layer-decide .decisions > li').length, botoes2: [...r.querySelectorAll('.layer-checked button')].map(b => b.textContent),
        recebeu: r.querySelector('.layer-arrived').textContent }; }, [comDiff.id, envio.id]);
    ok('3 4.7 cada envio em três camadas, na ordem', JSON.stringify(c.titulos) === JSON.stringify(['1. O que chegou', '2. O que o Librarian conferiu', '3. O que você decide']), c.titulos.join(' | '));
    ok('3 4.7 aviso fixo na camada 2', c.aviso === 'Referência conferida não significa que a relação semântica esteja comprovada.', c.aviso);
    ok('3 4.7 uma decisão por item, na camada 3', c.decisoes === envio.check.claims.length, `${c.decisoes}`);
    ok('3 as decisões saíram da camada 2', !c.botoes2.some(b => /Aprovar|Recusar|Revogar/.test(b)), c.botoes2.join(' | '));
    ok('3 camada 1: "O que o modelo recebeu", com o arquivo e o hash do vínculo', envio.link ? c.recebeu.includes(envio.link.query_sha256.slice(0, 12)) && c.recebeu.includes('O que o modelo recebeu') : c.recebeu.includes('não registrou vínculo'), c.recebeu.slice(-200));
    if (lacuna) {
      await expande(x.page, lacuna[0].id);
      const l = await x.page.evaluate(([id, send, k]) => { const li = document.querySelector(`#turns .turn[data-turn="${id}"] .bubble.reply[data-send="${send}"] .decisions > li[data-claim="${k}"]`); return li && { txt: li.textContent, botoes: [...li.querySelectorAll('button')].map(b => b.textContent) }; }, [lacuna[0].id, lacuna[1].id, lacuna[2].id]);
      ok('3 [A] LACUNA sem "Aprovar", com o motivo (Recusar continua)', l && !l.botoes.some(b => b.startsWith('Aprovar')) && l.botoes.includes('Recusar') && l.txt.includes(await T(x.page, 'review.why.lacuna')), JSON.stringify(l));
    } else ok('3 há uma LACUNA na run', false);

    // Copiar diff.
    const antesProj = arvore(path.join(RAIZ, '.studio')), antesRen = arvore(RENDERER), escritas = x.writes.length;
    const alvo = await x.page.evaluate(([id, send, d]) => { const ch = [...document.querySelectorAll(`#turns .turn[data-turn="${id}"] .bubble.reply[data-send="${send}"] details.change`)].find(e => e.querySelector('pre.diff')?.textContent === d);
      if (!ch) return null; ch.querySelector('.copy-diff').setAttribute('data-alvo', '1');
      // Uma resposta anterior fica dobrada: abre o caminho até a proposta, como a pessoa faria.
      for (let e = ch; e; e = e.parentElement) if (e.tagName === 'DETAILS') e.open = true; return { yours: ch.querySelector('.change-yours')?.textContent, run: [...ch.querySelectorAll('button')].some(b => b.textContent.startsWith('Rodar')) }; }, [comDiff.id, envio.id, diff]);
    ok('4 4.7 a proposta mostra o diff, "Rodar com esta mudança" e o aviso de que aplicar é com você', alvo && alvo.run && alvo.yours === 'Aplicar ao repositório é com você: aplique, faça commit e gere uma nova run.', JSON.stringify(alvo));
    await x.page.click('.copy-diff[data-alvo="1"]');
    await x.page.waitForTimeout(300);
    const copiado = await x.page.evaluate(() => window.__copiado.at(-1)), lido = await x.page.evaluate(() => navigator.clipboard.readText());
    ok('4 9.9.1 o texto copiado é exatamente check.diffs[claim] (bytes)', sha(Buffer.from(copiado, 'utf8')) === sha(Buffer.from(diff, 'utf8')), `${copiado?.length} × ${diff.length}`);
    const crlf = lido.includes('\r\n');
    ok('4 a área de transferência devolve o mesmo texto (a menos do fim de linha que o sistema troca)', lido.replace(/\r\n/g, '\n') === diff, `lido ${lido.length}, CRLF: ${crlf}`);
    console.log(`NOTA  área de transferência: lida de volta com ${crlf ? 'CRLF (o sistema trocou o fim de linha)' : 'LF, igual'}`);
    const confirmacao = await x.page.evaluate(() => document.querySelector('.copy-diff[data-alvo="1"]').parentElement.querySelector('.copy-note').textContent);
    const nLinhas = diff.split('\n').length - (diff.endsWith('\n') ? 1 : 0);
    ok('4 9.9.2 a confirmação diz as linhas e os arquivos, e que nada foi aplicado', confirmacao === `Diff copiado: ${nLinhas} linhas, arquivos: ${arquivos(diff).join(', ')}. Nada foi aplicado ao projeto.`, confirmacao);
    ok('4 9.9.3 nenhuma escrita: .studio e o projeto iguais, nenhuma requisição de escrita', arvore(path.join(RAIZ, '.studio')) === antesProj && arvore(RENDERER) === antesRen && x.writes.length === escritas, x.writes.slice(escritas).join());
    // Pelo teclado.
    await x.page.evaluate(() => { window.__copiado = []; });
    await x.page.focus('.copy-diff[data-alvo="1"]');
    await x.page.keyboard.press('Enter');
    await x.page.waitForTimeout(300);
    ok('4 9.12.6 Copiar diff pelo teclado', await x.page.evaluate(() => window.__copiado.at(-1)) === diff);
    // Área de transferência recusada.
    await x.page.evaluate(() => { navigator.clipboard.writeText = () => Promise.reject(new Error('negado pela bateria')); });
    await x.page.click('.copy-diff[data-alvo="1"]');
    await x.page.waitForTimeout(300);
    const caixa = await x.page.evaluate(() => { const n = document.querySelector('.copy-diff[data-alvo="1"]').parentElement.querySelector('.copy-note'); const b = n.querySelector('textarea');
      return b && { igual: b.value, sel: [b.selectionStart, b.selectionEnd], foco: document.activeElement === b, msg: n.querySelector('.error')?.textContent }; });
    ok('4 9.9.4 sem área de transferência: o diff fica selecionado num campo, com a mensagem', caixa && caixa.igual === diff && caixa.sel[0] === 0 && caixa.sel[1] === diff.length && caixa.foco
      && caixa.msg === 'Não foi possível copiar automaticamente; o diff está selecionado para copiar', JSON.stringify({ ...caixa, igual: caixa?.igual === diff }));
    // Sem diff derivado.
    if (semDiff) {
      await expande(x.page, semDiff[0].id);
      const s = await x.page.evaluate(([id, send]) => [...document.querySelectorAll(`#turns .turn[data-turn="${id}"] .bubble.reply[data-send="${send}"] details.change.no-diff`)].map(d => ({ txt: d.textContent, botao: Boolean(d.querySelector('.copy-diff')) })), [semDiff[0].id, semDiff[1].id]);
      ok('4 4.8 proposta sem diff derivado: sem botão, com o motivo', s.length && s.every(d => !d.botao && d.txt.includes('Esta proposta não tem diff derivado')), JSON.stringify(s).slice(0, 200));
    } else ok('4 há proposta sem diff derivado na run', false);
  }
  junta(x); await fecha(x);

  // ================================================================ 5. pergunta ativa guardada por run (4.7, 5.3)
  x = await pagina(browser);
  {
    const escolhida = '50';
    const navAntes = await x.page.evaluate(k => localStorage.getItem(k), NAV);
    await x.page.evaluate(id => document.querySelector(`#turns .turn[data-turn="${id}"] .bubble.user`).click(), escolhida);
    await x.page.waitForTimeout(300);
    const nav = JSON.parse(await x.page.evaluate(k => localStorage.getItem(k), NAV) ?? '{}');
    ok('5 4.7 clicar numa pergunta a guarda como ativa da run', nav.active_turn?.[RUN] === escolhida && await x.page.evaluate(() => __t.studio.activeTurn) === escolhida, JSON.stringify(nav.active_turn));
    ok('5 abrir o projeto não gravou a pergunta ativa', !navAntes || !JSON.parse(navAntes).active_turn?.[RUN]);
    await trocaRun(x.page, OUTRA);
    const ultOutra = [...turnsOutra].sort((a, b) => num(a.id) - num(b.id)).at(-1).id;
    ok('5 outra run: a ativa é a dela (a de maior número)', await x.page.evaluate(() => __t.studio.activeTurn) === ultOutra);
    await trocaRun(x.page, RUN);
    ok('5 de volta à run: a pergunta guardada volta a ser a ativa', await x.page.evaluate(() => __t.studio.activeTurn) === escolhida);
    const navAntesReload = await x.page.evaluate(k => localStorage.getItem(k), NAV);
    await x.page.reload();
    await x.page.waitForFunction(() => window.__t && __t.studio.run && __t.studio.turnsState === 'ready', null, { timeout: 120000 });
    if (await x.page.evaluate(() => __t.studio.run) !== RUN) await trocaRun(x.page, RUN); else await pronta(x.page);
    ok('5 T17 recarregar: a pergunta guardada da run volta, sem regravar a navegação', await x.page.evaluate(() => __t.studio.activeTurn) === escolhida && await x.page.evaluate(k => localStorage.getItem(k), NAV) === navAntesReload);
    ok('5 o rascunho não é tocado ao ativar', await x.page.evaluate(() => !Object.keys(localStorage).some(k => k.startsWith('librarian.draft'))));
  }
  junta(x); await fecha(x);

  // ================================================================ 6. respostas atrasadas e de outra run (8.4 item 8)
  const choice = { provider: 'claude-cli', modelfile: null, claude_model: 'f8', skills: [] };
  x = await pagina(browser);
  {
    // 6a. Perguntar: a pergunta criada depois da troca de run não entra na outra run e não é enviada.
    const h = segura();
    x.fake['POST /api/ask'] = async (r, body) => { await h.p; r.fulfill({ json: falsa(ultimo, '9001', { question: body.question }) }); };
    await x.page.evaluate(() => { const p = document.querySelector('#provider'); if ([...p.options].some(o => o.value === 'claude-cli')) p.value = 'claude-cli'; });
    x.page.evaluate(() => __t.askModel('Pergunta F8 atrasada'));
    await x.page.waitForFunction(() => true); await x.page.waitForTimeout(400);
    const corpo = x.reqs.find(q => q.chave === 'POST /api/ask');
    ok('6a o pedido sai para a run aberta no clique', corpo?.body.run === RUN, corpo?.raw);
    await trocaRun(x.page, OUTRA);
    h.solta(); await x.page.waitForTimeout(800);
    const r = await x.page.evaluate(() => ({ run: __t.studio.run, falsa: __t.studio.turns.some(t => t.id === '9001'), dom: Boolean(document.querySelector('#turns .turn[data-turn="9001"]')) }));
    ok('6a a pergunta da run anterior não entra na conversa da run nova', r.run === OUTRA && !r.falsa && !r.dom, JSON.stringify(r));
    ok('6a o aviso diz onde ela ficou', await nota(x.page) === await T(x.page, 'turn.lateCreated', { turn: '9001', where: `run ${RUN}` }), await nota(x.page));
    ok('6a nada é enviado dali', !x.writes.includes('POST /api/send') && !x.reqs.some(q => q.chave === 'POST /api/send'));
    delete x.fake['POST /api/ask'];
  }
  {
    // 6b. Enviar: a resposta que chega depois da troca é gravada na run da pergunta e não aparece na outra.
    await trocaRun(x.page, RUN);
    const h = segura();
    x.fake['POST /api/send'] = async r => { await h.p; r.fulfill({ json: { id: '99-f8', state: 'completed', provider: 'claude-cli', text: 'Resposta falsa da bateria F8', link: null } }); };
    x.page.evaluate(([id, c]) => __t.sendTurn(__t.studio.turns.find(t => t.id === id), c), [ultimo.id, choice]);
    await x.page.waitForTimeout(400);
    const tag = await turnoDom(x.page, ultimo.id, '.state-tag[data-state="sending"]');
    ok('6b 7.4 em andamento: "Enviando" na pergunta', tag?.startsWith('Enviando:'), tag);
    const envioReq = x.reqs.find(q => q.chave === 'POST /api/send');
    ok('6b o envio vai para a run e a pergunta dela', envioReq?.body.run === RUN && envioReq.body.turn === ultimo.id, envioReq?.raw);
    await trocaRun(x.page, OUTRA);
    const n = x.reqs.length;
    h.solta(); await x.page.waitForTimeout(1000);
    const depois = x.reqs.slice(n);
    ok('6b a comparação é relida na run da pergunta, não na aberta', depois.some(q => q.chave === 'GET /api/turns' && q.run === RUN) && !depois.some(q => q.chave === 'GET /api/turns' && q.run === OUTRA), depois.map(q => `${q.chave} ${q.run}`).join());
    ok('6b a resposta não aparece na run aberta', !(await x.page.evaluate(() => document.querySelector('#turns').textContent)).includes('Resposta falsa da bateria F8'));
    ok('6b o aviso diz que a resposta foi gravada lá', await nota(x.page) === await T(x.page, 'turn.lateAnswer', { turn: ultimo.id, where: `run ${RUN}` }), await nota(x.page));
    // 6c. Sair e voltar antes da resposta: o envio continua "em andamento" na pergunta, e a resposta chega nela.
    await trocaRun(x.page, RUN);
    const h2 = segura();
    x.fake['POST /api/send'] = async r => { await h2.p; r.fulfill({ json: { id: '98-f8', state: 'completed', provider: 'claude-cli', text: 'Outra resposta falsa', link: null } }); };
    x.page.evaluate(([id, c]) => __t.sendTurn(__t.studio.turns.find(t => t.id === id), c), [ultimo.id, choice]);
    await x.page.waitForTimeout(300);
    await trocaRun(x.page, OUTRA); await trocaRun(x.page, RUN);
    ok('6c de volta à run: o envio continua "em andamento" na pergunta', Boolean(await turnoDom(x.page, ultimo.id, '.state-tag[data-state="sending"]')));
    await x.page.evaluate(() => { document.querySelector('#chatNote').className = 'muted'; document.querySelector('#chatNote').textContent = ''; });
    h2.solta(); await x.page.waitForTimeout(1000);
    ok('6c a resposta chega na run aberta, que é a dela: sem aviso e sem "em andamento"', !(await turnoDom(x.page, ultimo.id, '.state-tag[data-state="sending"]')) && !(await nota(x.page)).includes('A run mudou'), await nota(x.page));
    delete x.fake['POST /api/send'];
  }
  {
    // 6d. Duas perguntas em andamento: a mais antiga, que chega depois, entra no lugar dela e não toma a ativa.
    await x.page.evaluate(() => { const p = document.querySelector('#provider'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
    const h = segura(); let k = 0;
    x.fake['POST /api/ask'] = async (r, body) => { const id = ++k === 1 ? '9101' : '9102'; if (id === '9101') await h.p; r.fulfill({ json: falsa(ultimo, id, { question: body.question }) }); };
    x.page.evaluate(() => __t.askModel('Primeira F8'));
    await x.page.waitForTimeout(300);
    await x.page.evaluate(() => __t.askModel('Segunda F8'));
    await x.page.waitForTimeout(400);
    h.solta(); await x.page.waitForTimeout(600);
    const r = await x.page.evaluate(() => ({ ativa: __t.studio.activeTurn, ordem: [...document.querySelectorAll('#turns > .turn')].map(d => d.dataset.turn).slice(-2) }));
    ok('6d a mais nova continua ativa; as duas na ordem', r.ativa === '9102' && r.ordem.join() === '9101,9102', JSON.stringify(r));
    ok('6d a pergunta guardada da run é a mais nova', JSON.parse(await x.page.evaluate(k => localStorage.getItem(k), NAV)).active_turn?.[RUN] === '9102');
    delete x.fake['POST /api/ask'];
  }
  {
    // 6e. Cadeia: continua a pergunta na run dela; criada depois da troca, não entra na outra.
    await x.page.evaluate(() => { const p = document.querySelector('#provider'); p.value = 'claude-cli'; p.dispatchEvent(new Event('change')); });
    const de = turns.find(t => t.sends.some(s => s.check?.report.mechanical_status === 'passed'));
    await x.page.evaluate(() => { __t.studio.turns = __t.studio.turns.filter(t => !String(t.id).startsWith('91')); __t.renderTurns(); });
    await expande(x.page, de.id);
    const h = segura();
    x.fake['POST /api/continue'] = async (r, body) => { await h.p; r.fulfill({ json: falsa(ultimo, '9201', { question: body.question }) }); };
    await x.page.evaluate(id => [...document.querySelectorAll(`#turns .turn[data-turn="${id}"] button`)].find(b => b.textContent === __t.t('chain.button')).click(), de.id);
    await dialogo(x.page);
    await x.page.fill('#askInput', 'Cadeia F8');
    await x.page.evaluate(() => { const m = document.querySelector('#askDialog'); [...m.querySelectorAll('.seg button')].find(b => b.textContent === __t.t('mode.locate')).click(); });
    await confirma(x.page);
    await x.page.waitForTimeout(300);
    const c = x.reqs.find(q => q.chave === 'POST /api/continue');
    ok('6e a cadeia sai da pergunta e da run dela', c?.body.run === RUN && c.body.turn === de.id, c?.raw);
    await trocaRun(x.page, OUTRA);
    h.solta(); await x.page.waitForTimeout(800);
    ok('6e criada depois da troca: não entra na outra run, com o aviso', !(await x.page.evaluate(() => __t.studio.turns.some(t => t.id === '9201'))) && await nota(x.page) === await T(x.page, 'turn.lateCreated', { turn: '9201', where: `run ${RUN}` }), await nota(x.page));
    delete x.fake['POST /api/continue'];
  }
  {
    // 6f. Decidir, apagar e rodar com a mudança: gravados na run da pergunta; nada muda na run aberta.
    await trocaRun(x.page, RUN);
    const alvo = lacuna;
    await expande(x.page, alvo[0].id);
    const h = segura();
    x.fake['POST /api/review'] = async r => { await h.p; r.fulfill({ json: {} }); };
    await x.page.evaluate(([id, send, k]) => [...document.querySelectorAll(`#turns .turn[data-turn="${id}"] .bubble.reply[data-send="${send}"] .decisions > li[data-claim="${k}"] button`)].find(b => b.textContent === 'Recusar').click(), [alvo[0].id, alvo[1].id, alvo[2].id]);
    await dialogo(x.page); await confirma(x.page);
    const rv = x.reqs.find(q => q.chave === 'POST /api/review');
    ok('6f a decisão vai para a run e a pergunta do item', rv?.body.run === RUN && rv.body.turn === alvo[0].id && rv.body.claim === alvo[2].id, rv?.raw);
    await trocaRun(x.page, OUTRA);
    const n = x.reqs.length, antes = await x.page.evaluate(() => JSON.stringify(__t.studio.turns.map(t => [t.id, (t.sends ?? []).length])));
    h.solta(); await x.page.waitForTimeout(800);
    ok('6f decisão atrasada: relida na run do item, aviso, nada muda na run aberta', x.reqs.slice(n).every(q => q.chave !== 'GET /api/turns' || q.run === RUN)
      && await x.page.evaluate(() => JSON.stringify(__t.studio.turns.map(t => [t.id, (t.sends ?? []).length]))) === antes
      && await nota(x.page) === await T(x.page, 'turn.lateReview', { turn: alvo[0].id, where: `run ${RUN}`, claim: alvo[2].id }), await nota(x.page));
    delete x.fake['POST /api/review'];

    await trocaRun(x.page, RUN);
    const h2 = segura();
    x.fake['DELETE /api/turn'] = async (r, body) => { await h2.p; r.fulfill({ json: turns.filter(t => t.id !== body.turn) }); };
    await expande(x.page, ultimo.id);
    await x.page.evaluate(id => { const d = document.querySelector(`#turns .turn[data-turn="${id}"]`); [...d.querySelectorAll('button')].find(b => b.textContent === '⋯').click(); [...d.querySelectorAll('button')].find(b => b.textContent === 'Apagar').click(); }, ultimo.id);
    await dialogo(x.page); await confirma(x.page);
    const del = x.reqs.find(q => q.chave === 'DELETE /api/turn');
    ok('6f apagar vai para a run da pergunta', del?.body.run === RUN && del.body.turn === ultimo.id, del?.raw);
    await trocaRun(x.page, OUTRA);
    const nOutra = await x.page.evaluate(() => __t.studio.turns.length);
    h2.solta(); await x.page.waitForTimeout(800);
    ok('6f apagada depois da troca: a lista da run aberta não é trocada pela da outra', await x.page.evaluate(() => __t.studio.turns.length) === nOutra && nOutra === turnsOutra.length
      && await nota(x.page) === await T(x.page, 'turn.lateDeleted', { turn: ultimo.id, where: `run ${RUN}` }), await nota(x.page));
    delete x.fake['DELETE /api/turn'];

    await trocaRun(x.page, RUN);
    const h3 = segura();
    const exec = execs.find(e => e.change && e.change.turn === comDiff.id) ?? execs.find(e => e.change);
    x.fake['POST /api/commands/run'] = async r => { await h3.p; r.fulfill({ json: exec }); };
    await expande(x.page, comDiff.id);
    const nome = await x.page.evaluate(([id, send, d]) => { const ch = [...document.querySelectorAll(`#turns .turn[data-turn="${id}"] .bubble.reply[data-send="${send}"] details.change`)].find(e => e.querySelector('pre.diff')?.textContent === d);
      const b = [...ch.querySelectorAll('button')].find(b => b.textContent.startsWith('Rodar')); b.click(); return b.textContent; }, [comDiff.id, envio.id, diff]);
    await x.page.waitForTimeout(300);
    const run = x.reqs.find(q => q.chave === 'POST /api/commands/run');
    ok('6f rodar com a mudança vai para a run da pergunta, com a mudança dela', run?.body.run === RUN && run.body.change.turn === comDiff.id && run.body.change.claim === claim.id, run?.raw);
    await trocaRun(x.page, OUTRA);
    h3.solta(); await x.page.waitForTimeout(800);
    const cmd = run?.body.name;
    ok('6f execução atrasada: o aviso cita o comando, a pergunta e a run', await nota(x.page) === await T(x.page, 'turn.lateExecution', { turn: comDiff.id, where: `run ${RUN}`, name: cmd }), `${nome} · ${await nota(x.page)}`);
    ok('6f a execução atrasada não aparece na pergunta de mesmo número da run aberta', !(await x.page.evaluate(id => document.querySelector(`#turns .turn[data-turn="${id}"] .records-runs`)?.textContent ?? '', comDiff.id)).includes(exec.started));
    delete x.fake['POST /api/commands/run'];
  }
  {
    // 6g. Outro projeto: a pergunta criada depois de abrir outro projeto não entra nele.
    await trocaRun(x.page, RUN);
    const RAIZ_B = 'C:/projeto-ficticio-da-bateria/f8', CHAVE_B = 'ficticio-f8-00000000';
    x.fake['POST /api/project'] = async (r, body) => (body.root === RAIZ_B ? r.fulfill({ json: { key: CHAVE_B, settings: { ...structuredClone(project.settings), name: 'ficticio-f8', root: RAIZ_B }, runs: [] } }) : false);
    const h = segura();
    x.fake['POST /api/ask'] = async (r, body) => { await h.p; r.fulfill({ json: falsa(ultimo, '9301', { question: body.question }) }); };
    x.page.evaluate(() => __t.askModel('Pergunta F8 de outro projeto'));
    await x.page.waitForTimeout(300);
    await abre(x.page, RAIZ_B, CHAVE_B);
    h.solta(); await x.page.waitForTimeout(800);
    ok('6g outro projeto: nada da pergunta atrasada entra nele, e o aviso diz o projeto', !(await x.page.evaluate(() => __t.studio.turns.length))
      && await nota(x.page) === await T(x.page, 'turn.lateCreated', { turn: '9301', where: `run ${RUN} do projeto ${CHAVE_RENDERER}` }), await nota(x.page));
    ok('6g 7.4 projeto sem run: a conversa não mostra perguntas nem "vazio" de run', await x.page.evaluate(() => !document.querySelector('#turns .turn') && !document.querySelector('#turns .turns-state')));
  }
  junta(x); await fecha(x);

  // ================================================================ 7. estados da conversa (7.4) e falhas de leitura
  x = await pagina(browser);
  {
    const h = segura();
    x.fake['GET /api/turns'] = async (r, _b, url) => { if (url.searchParams.get('run') !== OUTRA) return false; await h.p; return r.fulfill({ json: [] }); };
    await trocaRun(x.page, OUTRA, { espera: false });
    await x.page.waitForFunction(() => __t.studio.turnsState === 'loading');
    const carregando = await x.page.evaluate(() => ({ txt: document.querySelector('#turns .turns-state')?.textContent, turns: document.querySelectorAll('#turns .turn').length }));
    ok('7 7.4 carregando: "Carregando as perguntas desta run…", sem as perguntas da run anterior', carregando.txt === 'Carregando as perguntas desta run…' && carregando.turns === 0, JSON.stringify(carregando));
    h.solta(); await pronta(x.page);
    ok('7 7.4 vazio: "Nenhuma pergunta nesta run ainda."', await x.page.evaluate(() => document.querySelector('#turns .turns-state')?.textContent) === 'Nenhuma pergunta nesta run ainda.');
    x.fake['GET /api/turns'] = async (r, _b, url) => (url.searchParams.get('run') === RUN ? r.fulfill({ status: 500, json: { error: 'falha de leitura da bateria F8' } }) : false);
    await trocaRun(x.page, RUN);
    ok('7 7.4 falha de leitura das perguntas, com a mensagem do servidor', await x.page.evaluate(() => document.querySelector('#turns .turns-state.error')?.textContent) === 'Não foi possível ler as perguntas desta run: falha de leitura da bateria F8');
    delete x.fake['GET /api/turns'];
    x.fake['GET /api/executions'] = r => r.fulfill({ status: 500, json: { error: 'execuções ilegíveis F8' } });
    x.fake['GET /api/facts'] = r => r.fulfill({ status: 500, json: { error: 'fatos ilegíveis F8' } });
    await trocaRun(x.page, OUTRA); await trocaRun(x.page, RUN);
    await expande(x.page, ultimo.id);
    const reg = await turnoDom(x.page, ultimo.id, '.records');
    ok('7 4.7 falhas de leitura de execuções e de fatos ditas à parte, na pergunta', reg.includes('Não foi possível ler o histórico de execuções: execuções ilegíveis F8') && reg.includes('Não foi possível ler os fatos: fatos ilegíveis F8'), reg.slice(0, 200));
    delete x.fake['GET /api/executions']; delete x.fake['GET /api/facts'];
    // Resposta colada não conferida.
    await trocaRun(x.page, OUTRA); await trocaRun(x.page, RUN);
    await x.page.evaluate(() => { const p = document.querySelector('#provider'); p.value = 'manual'; p.dispatchEvent(new Event('change')); });
    await expande(x.page, ultimo.id);
    await x.page.fill(`#turns .turn[data-turn="${ultimo.id}"] .paste textarea`, '{"colado": "F8"}');
    ok('7 7.4 resposta colada não conferida: dito junto à caixa', await x.page.evaluate(id => { const p = document.querySelector(`#turns .turn[data-turn="${id}"] .paste .unsaved`); return p && !p.hidden ? p.textContent : ''; }, ultimo.id) === await T(x.page, 'paste.unchecked'));
    await trocaRun(x.page, OUTRA); await trocaRun(x.page, RUN); await expande(x.page, ultimo.id);
    ok('7 a resposta colada sobrevive à troca de run e aparece no estado da pergunta', await x.page.evaluate(id => document.querySelector(`#turns .turn[data-turn="${id}"] .paste textarea`).value, ultimo.id) === '{"colado": "F8"}'
      && await turnoDom(x.page, ultimo.id, '.state-tag[data-state="pasted"]') === 'Resposta colada não conferida');
    x.fake['POST /api/paste'] = r => r.fulfill({ status: 400, json: { error: 'recusa da bateria F8' } });
    await x.page.evaluate(id => [...document.querySelectorAll(`#turns .turn[data-turn="${id}"] .paste button`)].find(b => b.textContent === __t.t('paste.button')).click(), ultimo.id);
    await x.page.waitForTimeout(500);
    ok('7 conferir a colada recusada: a mensagem do servidor, e o texto continua', (await turnoDom(x.page, ultimo.id, '.paste')).includes('recusa da bateria F8') && await x.page.evaluate(id => document.querySelector(`#turns .turn[data-turn="${id}"] .paste textarea`).value, ultimo.id) === '{"colado": "F8"}');
    const p = x.reqs.find(q => q.chave === 'POST /api/paste');
    ok('7 a colada vai para a run e a pergunta dela', p?.body.run === RUN && p.body.turn === ultimo.id);
    delete x.fake['POST /api/paste'];
  }
  junta(x); await fecha(x);

  // ================================================================ 8. tela estreita, EN, temas, contraste, frases proibidas, chaves
  for (const [scheme, locale] of [['dark', 'en-US'], ['light', 'pt-BR']]) {
    x = await pagina(browser, { size: { width: 390, height: 844 }, scheme });
    if (locale === 'en-US') { await x.page.evaluate(() => document.querySelector('[data-lang="en"]').click()); await x.page.waitForTimeout(400); }
    await x.page.click('#colSwitch [data-col="question"]');
    await expande(x.page, comDiff.id);
    await x.page.waitForTimeout(300);
    const m = await x.page.evaluate(id => { const d = document.querySelector(`#turns .turn[data-turn="${id}"]`);
      return { larg: document.documentElement.scrollWidth <= innerWidth + 1, turns: document.querySelector('#turns').scrollWidth <= document.querySelector('#turns').clientWidth + 1,
        titulo: d.querySelector('.layer-title').textContent, usado: d.querySelector('.used strong').textContent }; }, comDiff.id);
    ok(`8 ${scheme}/${locale} 390 px: sem rolagem horizontal na página nem na conversa`, m.larg && m.turns, JSON.stringify(m));
    ok(`8 ${scheme}/${locale} textos no idioma`, locale === 'en-US' ? m.titulo === '1. What arrived' && m.usado === 'Used by this question' : m.titulo === '1. O que chegou' && m.usado === 'Usados por esta pergunta', JSON.stringify(m));
    const cores = await x.page.evaluate(() => {
      const fundo = e => { while (e) { const c = getComputedStyle(e).backgroundColor; if (c && !c.endsWith(', 0)') && c !== 'transparent') return c; e = e.parentElement; } return getComputedStyle(document.body).backgroundColor; };
      return ['.state-tag', '.layer-warning', '.used .muted', '.state-tag.passed'].map(s => { const e = document.querySelector(`#turns ${s}`); return e ? [getComputedStyle(e).color, fundo(e)] : null; }).filter(Boolean); });
    for (const c of cores) ok(`8 ${scheme}/${locale} contraste ≥ 4,5`, contraste(c) >= 4.5, `${c.join(' / ')} = ${contraste(c).toFixed(2)}`);
    // 7.1 e 9.6.4: nenhum rótulo da página afirma garantia, validação, vencedora ou bloqueio pelo servidor.
    await x.page.evaluate(() => { for (const t of __t.studio.turns) t.expanded = true; __t.renderTurns(); });
    const rotulos = await x.page.evaluate(() => [...document.querySelectorAll('#turns :is(.layer-title, .layer-warning, .used, .records > summary, .state-tag, .no-approve, .change-yours, .turns-state)')].map(e => e.textContent).join(' \n '));
    ok(`8 ${scheme}/${locale} 7.1 nenhuma frase proibida nos rótulos da conversa`, !/garantid|validad|vencedor|bloqueado pelo servidor|guarantee|validated|winner|blocked by the server/i.test(rotulos), rotulos.match(/.{0,40}(garantid|validad|vencedor|guarantee|validated|winner).{0,40}/i)?.[0]);
    junta(x); await fecha(x);
  }
  {
    const src = fs.readFileSync(path.join(__dirname, '../../web/index.html'), 'utf8');
    const chaves = [...src.matchAll(/^\s*'((?:turn\.(?:where|moved|late|notSent|idLine)\w*|chain\.movedBefore|turns|state|used|records|layer|decide|diff|paste\.unchecked|review\.why|reply\.(?:received|receivedUnlinked|noLink|link))[A-Za-z.]*)': \['((?:[^'\\]|\\.)*)',\s*'((?:[^'\\]|\\.)*)'\]/gm)];
    ok('8 9.12.2 as chaves do F8 têm PT e EN, com as mesmas variáveis', chaves.length >= 70 && chaves.every(m => m[2] && m[3] && JSON.stringify((m[2].match(/\{\w+\}/g) ?? []).sort()) === JSON.stringify((m[3].match(/\{\w+\}/g) ?? []).sort())), `${chaves.length}`);
    ok('8 as chaves que o F8 tirou não ficaram (turn.criteria, turn.noCriteria, turn.notSent)', !/^\s*'turn\.(criteria|noCriteria|notSent)':/m.test(src));
  }

  // ================================================================ 9. nada gravado
  ok('9 sem erros de página', !todosErros.length, todosErros.slice(0, 3).join(' | '));
  ok('9 o perfil ficou igual em bytes', sha(fs.readFileSync(perfil)) === perfilAntes);
  ok('9 as perguntas da run ficam as mesmas no servidor', (await get(`/api/turns?key=${CHAVE_RENDERER}&run=${RUN}`)).length === turns.length);
  ok('9 nenhuma escrita pedida além das respondidas aqui (nenhuma abortada)', !todasEscritas.length, [...new Set(todasEscritas)].join(', '));
  await browser.close();
})().catch(e => { console.error(e); process.exit(1); });

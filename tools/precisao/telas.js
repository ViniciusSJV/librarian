// Runs every live screen battery, one at a time, and says how long each takes (DEC-47: every Parada 2 runs them all).
// A battery fails if a check fails, if it exits with an error (it fell halfway, timed out or was killed), or if it runs
// another number of checks than registered here. Exit code: 0 all passed; 1 any battery failed; 2 could not start.
//   node tools/precisao/telas.js [--so <nome>] [--saida <pasta>]
// Needs Google Chrome (installed on the machine) and playwright-core (`npm ci --prefix tools/precisao`). The retired
// batteries, with the reason and what they covered, are in crates/librarian-studio/tests/tela/APOSENTADAS.md.
'use strict';
const fs = require('fs'), os = require('os'), path = require('path'), net = require('net'), { spawn, spawnSync } = require('child_process');
const L = require('./lib');
const opt = L.parseArgs(process.argv.slice(2));
const FASE2 = 'docs/reports/fase-2/dados', STUDIO = 'crates/librarian-studio/tests/tela';
// The checks of the last run approved by the user: a battery that changes changes its number here, in the same commit.
// `args`: what the battery takes on the command line (tela-passo6 the output folder; tela-24d would take the binary
// folder, and gets none, so it uses target/release, built here). `exige`: a line its output must have.
const VIVAS = [
  { nome: '2.1', arquivo: `${FASE2}/2.1/tela-21.js`, verificacoes: 51 },
  { nome: '2.2', arquivo: `${FASE2}/2.2/tela-22.js`, verificacoes: 56 },
  { nome: '2.3', arquivo: `${FASE2}/2.3/tela-23.js`, verificacoes: 84 },
  { nome: '2.3b', arquivo: `${FASE2}/2.3b/tela-23b.js`, verificacoes: 102 },
  { nome: '2.4a', arquivo: `${FASE2}/2.4a/tela-24a.js`, verificacoes: 88 },
  { nome: '2.4c', arquivo: `${FASE2}/2.4c/tela-24c.js`, verificacoes: 193 },
  { nome: '2.4d-0', arquivo: `${FASE2}/2.4d0/tela-24d0.js`, verificacoes: 240 },
  { nome: '2.4d', arquivo: `${FASE2}/2.4d/tela-24d.js`, verificacoes: 71 },
  { nome: 'g3', arquivo: 'docs/reports/piloto-1b/dados/g3/tela-g3.js', verificacoes: 14 },
  // Part A only: part B is retired (APOSENTADAS.md); it runs only with a server on 8841 and LIBRARIAN_TESTE_RAIZ_RS.
  { nome: 'tela-estado', arquivo: `${STUDIO}/tela-estado.js`, verificacoes: 46, exige: /^-- parte B pulada/m, porta: 8841 },
  { nome: 'tela-simplificacao', arquivo: `${STUDIO}/tela-simplificacao.js`, verificacoes: 47 },
  { nome: 'tela-passo6', arquivo: `${STUDIO}/tela-passo6.js`, verificacoes: 36, servidor: 8842, args: dir => [dir] },
];
// Variables of the retired batteries, never passed on: with them, part B of tela-estado would run.
const FORA = ['LIBRARIAN_TESTE_RAIZ_RS', 'LIBRARIAN_TESTE_RAIZ_PY', 'LIBRARIAN_TESTE_COPIAS', 'LIBRARIAN_TESTE_CASOS', 'LIBRARIAN_TESTE_CHAVE_RENDERER', 'LIBRARIAN_TESTE_CHAVE_PYTHON'];
const CHROME = 'C:/Program Files/Google/Chrome/Application/chrome.exe';

const ocupada = porta => new Promise(r => { const c = net.connect(porta, '127.0.0.1', () => { c.end(); r(true); }).on('error', () => r(false)); });
const sai = (motivo) => { console.error(motivo); process.exit(2); };

(async () => {
  let PLAYWRIGHT;
  try { PLAYWRIGHT = L.playwright(); require(PLAYWRIGHT); } catch (e) { sai(e.message); }
  if (process.platform === 'win32' && !fs.existsSync(CHROME)) sai(`Google Chrome não encontrado em ${CHROME}`);
  const lista = VIVAS.filter(v => !opt.so || v.nome === opt.so);
  if (!lista.length) sai(`nenhuma bateria viva com o nome ${opt.so}`);
  const out = path.resolve(opt.saida ?? fs.mkdtempSync(path.join(os.tmpdir(), 'librarian-telas-')));
  fs.mkdirSync(out, { recursive: true });
  let bin;
  try { bin = L.prepareBinaries(path.join(L.REPO, 'target/release')); } catch (e) { sai(`a compilação falhou: ${e.message}`); }
  console.log(L.binaryLine(bin));
  const exe = path.join(L.REPO, 'target/release', process.platform === 'win32' ? 'librarian_studio.exe' : 'librarian_studio');
  const base = Object.fromEntries(Object.entries(process.env).filter(([k]) => !FORA.includes(k)));
  let falhou = false, total = 0, falhas = 0;
  const t0 = Date.now();
  for (const v of lista) {
    const dir = path.join(out, v.nome), t = Date.now(), motivos = [];
    fs.mkdirSync(dir, { recursive: true });
    const env = { ...base, TELA_OUT: path.join(dir, 'img'), LIBRARIAN_TESTE_PLAYWRIGHT: PLAYWRIGHT, LIBRARIAN_TESTE_STUDIO: exe, LIBRARIAN_TESTE_SAIDA: dir,
      LIBRARIAN_TESTE_RENDERER: process.env.LIBRARIAN_TESTE_RENDERER || path.join(L.REPO, '../renderer'), LIBRARIAN_TESTE_PYTHON: process.env.LIBRARIAN_TESTE_PYTHON || 'sem-projeto' };
    let servidor, r = null;
    // A server already answering on the battery's port would be measured instead of ours (or, on 8841, run part B).
    for (const porta of [v.servidor, v.porta].filter(Boolean)) if (await ocupada(porta)) motivos.push(`a porta ${porta} já está em uso`);
    if (!motivos.length && v.servidor) {
      const raiz = path.join(dir, 'raiz');
      fs.mkdirSync(path.join(raiz, 'crates/librarian-studio/web'), { recursive: true });
      fs.copyFileSync(path.join(L.REPO, 'crates/librarian-studio/web/index.html'), path.join(raiz, 'crates/librarian-studio/web/index.html'));
      for (const d of ['modelfiles', 'skills']) fs.cpSync(path.join(L.REPO, d), path.join(raiz, d), { recursive: true });
      servidor = spawn(exe, ['--port', String(v.servidor)], { stdio: 'ignore', env: { ...base, LIBRARIAN_ROOT: raiz, LIBRARIAN_COPIES: path.join(dir, 'copias') } });
      let pronto = false;
      for (let i = 0; i < 300 && !pronto; i++) { try { const x = await fetch(`http://127.0.0.1:${v.servidor}/api/status`); await x.arrayBuffer(); pronto = x.ok; } catch { } if (!pronto) await new Promise(z => setTimeout(z, 100)); }
      if (!pronto) motivos.push(`o servidor da porta ${v.servidor} não ficou pronto`);
    }
    if (!motivos.length) r = spawnSync(process.execPath, [path.join(L.REPO, v.arquivo), ...(v.args ? v.args(dir) : [])], { encoding: 'utf8', env, maxBuffer: 1 << 27, timeout: 900000, cwd: L.REPO });
    servidor?.kill();
    const stdout = r?.stdout ?? '';
    fs.writeFileSync(path.join(dir, 'saida.txt'), stdout + (r?.stderr ?? ''));
    const linhas = stdout.split(/\r?\n/);
    const passa = linhas.filter(l => /^(PASSA|ok  )/.test(l)).length, falha = linhas.filter(l => l.startsWith('FALHA')), puladas = linhas.filter(l => /pulad/i.test(l)).length;
    if (r) {
      if (falha.length) motivos.push(`${falha.length} falha(s)`);
      if (r.status !== 0) motivos.push(`saída ${r.status ?? r.signal ?? r.error?.code}`);
      if (passa + falha.length !== v.verificacoes) motivos.push(`${passa + falha.length} verificações rodadas, registradas ${v.verificacoes}`);
      if (v.exige && !v.exige.test(stdout)) motivos.push(`a saída não tem ${v.exige}`);
    }
    console.log(`${motivos.length ? 'FALHA' : 'ok   '} ${v.nome}: ${passa + falha.length} rodadas de ${v.verificacoes}, ${passa} passam, ${falha.length} falham, ${puladas} linha(s) "pulada", ${((Date.now() - t) / 1000).toFixed(0)} s${motivos.length ? ` (${motivos.join('; ')})` : ''}`);
    for (const f of falha) console.log('        ' + f.slice(0, 300));
    if (r && r.status !== 0 && !falha.length) console.log('        ' + (r.stderr || stdout).trim().split(/\r?\n/).slice(-3).join(' | ').slice(0, 400));
    falhou ||= motivos.length > 0; total += passa; falhas += falha.length;
  }
  const parcial = lista.length < VIVAS.length ? ` PARCIAL (--so ${opt.so}: ${lista.length} de ${VIVAS.length} baterias)` : '';
  console.log(`${falhou ? 'FALHA' : 'ok'}${parcial}: ${lista.length} baterias, ${total} verificações passam, ${falhas} falham, ${((Date.now() - t0) / 60000).toFixed(1)} min; saídas em ${out}`);
  process.exit(falhou ? 1 : 0);
})().catch(e => { console.error(e); process.exit(2); });

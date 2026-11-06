// Sobe os dois servidores nas raízes de teste, roda as seis baterias e encerra os servidores.
// As baterias gravam nas raízes e etapa2c cria turnos: por isso a raiz real do repositório é recusada.
const { spawn, spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const { PYTHON } = require('./ambiente');

const pede = nome => { const v = process.env[nome]; if (!v) { console.error(`Falta a variável de ambiente ${nome}: veja README.md`); process.exit(2); } return v; };
const STUDIO = pede('LIBRARIAN_TESTE_STUDIO'), RAIZ_RS = pede('LIBRARIAN_TESTE_RAIZ_RS'), RAIZ_PY = pede('LIBRARIAN_TESTE_RAIZ_PY'), COPIAS = pede('LIBRARIAN_TESTE_COPIAS'), SAIDA = pede('LIBRARIAN_TESTE_SAIDA');
const REPO = path.resolve(__dirname, '../../../..'), PAGINA = 'crates/librarian-studio/web/index.html';
const BATERIAS = ['tela-cartao', 'tela-zero', 'tela-origem', 'tela-nomes', 'tela-passo2', 'tela-4a', 'etapa2c'];
const sai = motivo => { console.error(motivo); process.exit(2); };

for (const raiz of [RAIZ_RS, RAIZ_PY]) {
  if (path.resolve(raiz).toLowerCase() === REPO.toLowerCase()) sai(`Raiz recusada: ${raiz} é o próprio repositório, e as baterias gravam na raiz.`);
  // O servidor serve a página da raiz: uma cópia antiga testaria outra página.
  if (!fs.readFileSync(path.join(raiz, PAGINA)).equals(fs.readFileSync(path.join(REPO, PAGINA)))) sai(`A página de ${raiz} difere da do repositório: copie ${PAGINA} para a raiz antes.`);
}
fs.mkdirSync(SAIDA, { recursive: true });

const servidor = (raiz, porta) => spawn(STUDIO, ['--port', String(porta)], { stdio: 'ignore', env: { ...process.env, LIBRARIAN_ROOT: raiz, LIBRARIAN_COPIES: COPIAS } });
const pronto = async porta => { for (let i = 0; i < 300; i++) { try { const r = await fetch(`http://127.0.0.1:${porta}/api/status`); await r.arrayBuffer(); if (r.ok) return; } catch {} await new Promise(z => setTimeout(z, 200)); } sai(`O servidor da porta ${porta} não respondeu.`); };

(async () => {
  for (const porta of [8841, 8842]) { try { await fetch(`http://127.0.0.1:${porta}/api/status`); sai(`A porta ${porta} já está em uso.`); } catch {} }
  const servidores = [servidor(RAIZ_RS, 8841), servidor(RAIZ_PY, 8842)];
  // Em qualquer saída, inclusive numa queda deste script: nenhum servidor fica para trás.
  process.on('exit', () => { for (const s of servidores) s.kill(); });
  let falhou = false, passa = 0, falha = 0;
  try {
    await pronto(8841); await pronto(8842);
    for (const nome of BATERIAS) {
      const r = spawnSync(process.execPath, [path.join(__dirname, `${nome}.js`), SAIDA, ...(nome === 'etapa2c' ? [PYTHON] : [])], { encoding: 'utf8', maxBuffer: 1 << 26 });
      process.stdout.write(r.stdout); process.stderr.write(r.stderr);
      const p = (r.stdout.match(/^PASSA/gm) ?? []).length, f = (r.stdout.match(/^FALHA/gm) ?? []).length;
      console.log(`== ${nome}: saída ${r.status}, ${p} PASSA, ${f} FALHA`);
      passa += p; falha += f; falhou ||= r.status !== 0 || f > 0;
    }
  } catch (e) { console.error(e); falhou = true; }
  console.log(`== total: ${passa} PASSA, ${falha} FALHA`);
  process.exit(falhou ? 1 : 0);
})();

// Passo 2.4d: as baterias de tela dos passos anteriores da Fase 2, com a página e o binário de antes (47e4ece) e os de agora.
// Roda cópias numa pasta de saída (nada é escrito nos dados dos passos).   node antigas24d.js <saida> <bin antes> <pagina antes>
'use strict';
const fs = require('fs'), path = require('path'), { spawnSync } = require('child_process');
const [out, binAntes, paginaAntes] = process.argv.slice(2);
const DADOS = path.join(__dirname, '..');
const B = ['2.1/tela-21', '2.2/tela-22', '2.3/tela-23', '2.3b/tela-23b', '2.4a/tela-24a', '2.4b/tela-24b', '2.4c/tela-24c', '2.4d0/tela-24d0'];
for (const b of B) {
  const r = {};
  for (const quando of ['antes', 'agora']) {
    const dir = path.join(out, quando, path.dirname(b)), name = path.basename(b) + '.js';
    fs.rmSync(dir, { recursive: true, force: true }); fs.mkdirSync(dir, { recursive: true });
    fs.copyFileSync(path.join(DADOS, path.dirname(b), 'seeds.json'), path.join(dir, 'seeds.json'));
    let s = fs.readFileSync(path.join(DADOS, b + '.js'), 'utf8');
    if (quando === 'antes') {
      s = s.replaceAll("path.join(REPO, 'target/release')", JSON.stringify(binAntes)).replaceAll("fs.copyFileSync(path.join(REPO, 'crates/librarian-studio/web/index.html'),", `fs.copyFileSync(${JSON.stringify(paginaAntes)},`);
      if (!s.includes(paginaAntes.replaceAll('\\', '\\\\')) && !s.includes(paginaAntes)) throw new Error(`${b}: página não trocada`);
    }
    fs.writeFileSync(path.join(dir, name), s);
    const x = spawnSync(process.execPath, [path.join(dir, name)], { encoding: 'utf8', maxBuffer: 1 << 26, timeout: 900000, env: { ...process.env, TELA_OUT: path.join(dir, 'img') } });
    fs.writeFileSync(path.join(dir, 'saida.txt'), x.stdout + x.stderr);
    const lines = x.stdout.split(/\r?\n/);
    r[quando] = { saida: x.status, passa: lines.filter(l => l.startsWith('PASSA')).length, falhas: lines.filter(l => l.startsWith('FALHA')).map(l => l.split('  -> ')[0]) };
  }
  const novas = r.agora.falhas.filter(f => !r.antes.falhas.includes(f));
  console.log(`${b}: antes ${r.antes.passa} passam, ${r.antes.falhas.length} falham (saída ${r.antes.saida}); agora ${r.agora.passa} passam, ${r.agora.falhas.length} falham (saída ${r.agora.saida}); falhas novas: ${novas.length}`);
  for (const f of novas) console.log('   ' + f);
}

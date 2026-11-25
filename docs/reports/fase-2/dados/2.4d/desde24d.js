// Passo 2.4d (pendência das baterias antigas): desde quando falham as do 2.2, 2.3 e 2.4b. Compila cada passo numa cópia
// descartável do repositório e roda a bateria com a página e o binário dele.   node desde24d.js <clone> <saida>
'use strict';
const fs = require('fs'), path = require('path'), { spawnSync, execFileSync } = require('child_process');
const [clone, out] = process.argv.slice(2), DADOS = path.join(__dirname, '..');
const C = [['2.2', 'a34059c'], ['2.3a', '5589912'], ['2.3b', '5a26630'], ['2.3c', 'efb346a'], ['2.4a', 'fd388cd'], ['2.4b', '6dc0e4a'], ['2.4c', '284243b'], ['2.4d-0', '504ac2e']];
const B = [['2.2/tela-22', '2.2'], ['2.3/tela-23', '2.3a'], ['2.4b/tela-24b', '2.4b']];
for (const [passo, sha] of C) {
  execFileSync('git', ['-C', clone, '-c', 'core.longpaths=true', 'checkout', '-q', '-f', sha]);
  execFileSync('cargo', ['build', '--release', '--locked', '-p', 'librarian-studio', '-p', 'librarian-ingest'], { cwd: clone, stdio: 'ignore' });
  const bin = path.join(out, 'bin-' + passo).replaceAll('\\', '/');
  fs.mkdirSync(bin, { recursive: true });
  for (const e of ['librarian_studio.exe', 'librarian_ingest.exe']) fs.copyFileSync(path.join(clone, 'target/release', e), path.join(bin, e));
  const page = path.join(clone, 'crates/librarian-studio/web/index.html').replaceAll('\\', '/');
  for (const [b, nasce] of B) {
    if (C.findIndex(c => c[0] === passo) < C.findIndex(c => c[0] === nasce)) continue;
    const dir = path.join(out, passo, path.dirname(b)), name = path.basename(b) + '.js';
    fs.mkdirSync(dir, { recursive: true });
    fs.copyFileSync(path.join(DADOS, path.dirname(b), 'seeds.json'), path.join(dir, 'seeds.json'));
    const s = fs.readFileSync(path.join(DADOS, b + '.js'), 'utf8').replaceAll("path.join(REPO, 'target/release')", JSON.stringify(bin)).replaceAll("fs.copyFileSync(path.join(REPO, 'crates/librarian-studio/web/index.html'),", `fs.copyFileSync(${JSON.stringify(page)},`);
    fs.writeFileSync(path.join(dir, name), s);
    const x = spawnSync(process.execPath, [path.join(dir, name)], { encoding: 'utf8', maxBuffer: 1 << 26, timeout: 600000, env: { ...process.env, TELA_OUT: path.join(dir, 'img') } });
    fs.writeFileSync(path.join(dir, 'saida.txt'), x.stdout + x.stderr);
    const l = x.stdout.split(/\r?\n/), f = l.filter(t => t.startsWith('FALHA')).map(t => t.split('  -> ')[0].slice(7));
    console.log(`${b} no ${passo} (${sha}): ${l.filter(t => t.startsWith('PASSA')).length} passam, ${f.length} falham (saída ${x.status})${f.length ? ': ' + f.join(' | ') : ''}`);
  }
}
console.log('FIM');

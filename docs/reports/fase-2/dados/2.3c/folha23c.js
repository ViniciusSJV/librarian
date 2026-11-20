// Generates docs/reports/fase-2/2.3c-revisao-amostra.md from the 2.3c entries of the fixtures (HC-98 to
// HC-112, RT-66 and RT-67) and the gate's answers. node folha23c.js <gate resultado.json>
'use strict';
const fs = require('fs');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const SRC = { hc: 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks', rt: 'C:/Users/beatl/Documents/Projetos/renderer' };
const gate = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const snip = (lang, file, from, to) => {
  const L = fs.readFileSync(`${SRC[lang]}/${file}`, 'utf8').split(/\r?\n/), out = [];
  for (let i = from; i <= to; i++) out.push(`${String(i).padStart(4)} | ${L[i - 1] ?? ''}`);
  return '```' + (lang === 'hc' ? 'python' : 'rust') + '\n// ' .replace('//', lang === 'hc' ? '#' : '//') + file + '\n' + out.join('\n') + '\n```';
};
// The line that binds the receiver: the last `x =` (or `let x =`) before the call, or the `def`/`fn`.
const binding = (lang, file, line, text) => {
  const L = fs.readFileSync(`${SRC[lang]}/${file}`, 'utf8').split(/\r?\n/);
  const v = text.match(/^([A-Za-z_]\w*)/)[1];
  const re = text.startsWith('request.') ? /^\s*(async\s+)?def\s/ : new RegExp(String.raw`^\s*(let\s+(mut\s+)?)?${v}\s*=`);
  for (let i = line - 1; i >= 1; i--) if (re.test(L[i - 1])) return i;
  return line;
};
const ESTRATO = {
  retorno_py: 'tipo pelo retorno anotado',
  campo_outro_arquivo: 'campo de uma classe de outro arquivo',
  retorno_confirma: 'o retorno confirma o `unique`',
  liga_nao_fecha: 'a chamada que liga não fecha com um alvo',
  cadeia: 'cadeia de dois níveis (não seguida)',
  retorno_rs: 'Rust: retorno de `T::f` ou `f`',
};
const CONTEXTO = {
  'HC-98': [['hc/lib/curl.py', 224, 233, '`curl.post` retorna `Response`.'], ['hc/lib/curl.py', 32, 38, '`Response.json`.']],
  'HC-99': [['hc/lib/curl.py', 224, 233, 'O mesmo `curl.post` do HC-98.']],
  'HC-100': [['hc/front/views.py', 187, 192, '`_get_rw_project_for_user` retorna `Project`, no mesmo arquivo.']],
  'HC-101': [['hc/front/views.py', 187, 192, 'O mesmo do HC-100.']],
  'HC-102': [['hc/accounts/http.py', 1, 12, '`AuthenticatedHttpRequest` declara `profile: Profile`.']],
  'HC-103': [['hc/accounts/http.py', 1, 12, 'O mesmo do HC-102.']],
  'HC-104': [['hc/api/decorators.py', 14, 19, '`ApiRequest` declara `project: Project`.']],
  'HC-105': [['hc/front/views.py', 140, 142, '`_get_rw_check_for_user` retorna `Check`.']],
  'HC-106': [['hc/api/transports.py', 105, 107, '`last_ping` retorna `Ping | None`.']],
  'HC-107': [['hc/accounts/models.py', 61, 64, '`ProfileManager.for_user` retorna `Profile`.']],
  'HC-110': [['hc/accounts/models.py', 525, 526, '`Project.transfer_request` retorna `Member | None`.'], ['hc/accounts/models.py', 625, 626, '`Member.can_accept`.']],
  'HC-111': [['hc/lib/curl.py', 37, 38, '`Response.json` retorna `JSONValue`, que não é classe.']],
  'RT-66': [['src/bounds.rs', 16, 24, '`Bounds::empty` retorna `Self`; `add_point` é de `Bounds`.']],
  'RT-67': [['src/world.rs', 146, 147, '`create_default_world` em src/world.rs.'], ['src/camera.rs', 291, 292, 'O homônimo do módulo de testes de src/camera.rs, com o mesmo retorno.']],
};
const out = [
  '# Passo 2.3c: folha de revisão da amostra HC-98 a HC-112, RT-66 e RT-67',
  '',
  'Escrita em 20/11/2026. Healthchecks em `6c05a7a`, ray tracer em `e38254b`. A Q5 da Parada 1 do 2.3c aprovou os estratos; a verdade de cada caso foi validada pelo usuário em 20/11/2026, na Parada 2 (HC-100, HC-110 e RT-67 conferidos no editor). Os 17 casos foram validados; a verdade está validada nas fixtures, e os estratos entraram na linha de base nova.',
  '',
  '"Antes" é a resposta de `/api/calls` com o binário do 2.3b (`f50b8f8`), registrada na fixture. "Agora" é a resposta com o binário do 2.3c, medida pelo portão.',
  '',
];
for (const [lang, file] of [['hc', 'amostra-hc.json'], ['rt', 'amostra-rt.json']]) {
  const fx = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/${file}`, 'utf8'));
  const now = Object.fromEntries(gate.projetos[lang].amostra.map(s => [s.id, s]));
  for (const c of fx.chamadas.filter(c => /^(HC-(9[89]|1\d\d)|RT-6[67])$/.test(c.id))) {
    const [path, line] = c.local.split(':');
    const n = now[c.id], lb = c.linha_de_base;
    const verdade = c.verdade.alvo ? c.verdade.alvo : `fora do projeto: ${c.verdade.externo}`;
    const from = Math.min(binding(lang, path, +line, c.texto), +line - 1);
    out.push(`## ${c.id} · ${ESTRATO[c.estrato]}`, '',
      `**A chamada:** \`${c.local}\`, escrita em \`${c.dono}\`: \`${c.texto}\`. ${/tests?[\/_]/.test(path) || /tests?_/.test(c.dono) ? 'Código de teste.' : 'Código de produção.'}`, '',
      `- Antes: \`${lb.resolution}\` (${lb.candidates})${lb.alvos.length ? ': ' + lb.alvos.join(', ') : ''}.`,
      `- Agora: \`${n?.resposta ?? 'não medida'}\` (categoria: ${n?.categoria ?? '—'}).`,
      '', snip(lang, path, Math.max(1, from), +line + 1), '');
    if (CONTEXTO[c.id]) {
      out.push('**Onde olhar também:**', '');
      for (const [f, a, b, text] of CONTEXTO[c.id]) out.push(`- ${text}`, '', snip(lang, f, a, b), '');
    }
    out.push(`**Verdade (validada pelo usuário em 20/11/2026, na Parada 2${['HC-100', 'HC-110', 'RT-67'].includes(c.id) ? '; conferido no editor' : ''}):** ${verdade}.`, '', `**Justificativa:** ${c.justificativa}`, '', '**Validação:** concordo (usuário, 20/11/2026)', '');
  }
}
fs.writeFileSync(`${REPO}/docs/reports/fase-2/2.3c-revisao-amostra.md`, out.join('\n'));
console.log('folha gravada');

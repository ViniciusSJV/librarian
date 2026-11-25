// Gera docs/reports/fase-2/2.4-revisao-amostra.md com as entradas do 2.4a nas fixtures (HC-113 a HC-123,
// RT-68 a RT-70) e a resposta do portão. A verdade é a proposta pelo agente, até a validação do usuário.
//   node folha24a.js <resultado.json do portão>
'use strict';
const fs = require('fs');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const SRC = { hc: 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks', rt: 'C:/Users/beatl/Documents/Projetos/renderer' };
const gate = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const snip = (lang, file, from, to) => {
  const L = fs.readFileSync(`${SRC[lang]}/${file}`, 'utf8').split(/\r?\n/), out = [];
  for (let i = from; i <= to; i++) out.push(`${String(i).padStart(4)} | ${L[i - 1] ?? ''}`);
  return '```' + (lang === 'hc' ? 'python' : 'rust') + '\n' + (lang === 'hc' ? '# ' : '// ') + file + '\n' + out.join('\n') + '\n```';
};
const ESTRATO = {
  escopo_modulo: 'escopo: definição do próprio arquivo',
  escopo_fora: 'escopo: import de fora do projeto',
  escopo_local: 'escopo: variável da própria função',
  escopo_import: 'escopo: import do arquivo',
  escopo_rs: 'Rust: escopo do módulo',
};
// Onde a ligação do nome está escrita.
const CONTEXTO = {
  'HC-113': [['hc/settings.py', 40, 50, 'As duas `@overload` e a implementação de `envsecret`, no mesmo arquivo.']],
  'HC-114': [['hc/settings.py', 40, 50, 'As mesmas três `def` do HC-113.']],
  'HC-115': [['hc/lib/curl.py', 32, 34, 'A classe `Response` do próprio arquivo.'], ['hc/integrations/signal/transport.py', 30, 41, 'O homônimo `Signal.Response`, aninhado na classe `Signal`.']],
  'HC-116': [['hc/api/apps.py', 9, 9, 'O import de `Error`.'], ['hc/integrations/signal/transport.py', 40, 41, 'O `Signal.Error` que o grafo dava como único candidato.']],
  'HC-117': [['hc/api/apps.py', 9, 9, 'O mesmo import do HC-116.']],
  'HC-118': [['hc/api/apps.py', 9, 9, 'O mesmo import do HC-116.']],
  'HC-119': [['hc/accounts/migrations/0018_auto_20190112_1426.py', 11, 14, 'A ligação local de `Project`.']],
  'HC-120': [['hc/api/tests/test_sendalerts.py', 1, 20, 'Os imports do arquivo de teste.']],
  'HC-121': [['hc/logs/tests/test_sendlogs.py', 1, 12, 'Os imports do arquivo de teste.']],
  'HC-122': [['hc/integrations/signal/tests/test_signal_captcha.py', 43, 45, '`setup_mock` do próprio arquivo.']],
  'HC-123': [['hc/accounts/tests/test_senddeletionscheduled.py', 19, 21, '`counts` do próprio arquivo.']],
  'RT-68': [['src/world.rs', 143, 147, 'O `mod tests_world` e o seu `create_default_world`.']],
  'RT-69': [['src/camera.rs', 230, 232, 'O `mod tests_camera`, com `use super::*`.'], ['src/camera.rs', 291, 292, 'O `create_default_world` do próprio módulo.']],
  'RT-70': [['src/camera.rs', 230, 232, 'O `use super::*` do `mod tests_camera`.'], ['src/camera.rs', 143, 144, '`render_benchmark` do módulo pai.']],
};
const out = [
  '# Passo 2.4a: folha de revisão da amostra HC-113 a HC-123 e RT-68 a RT-70',
  '',
  'Escrita em 20/11/2026. Healthchecks em `6c05a7a`, ray tracer em `e38254b`. A Q8 da Parada 1 do 2.4 aprovou os estratos. **A verdade de cada caso é a proposta pelo agente** e só vale depois da validação do usuário nesta folha (DEC-22): marque "concordo" ou corrija em cada caso.',
  '',
  '"Antes" é a resposta de `/api/calls` com o binário do 2.3c (`6196976`), registrada na fixture. "Agora" é a resposta com o binário do 2.4a, medida pelo portão.',
  '',
];
for (const [lang, file] of [['hc', 'amostra-hc.json'], ['rt', 'amostra-rt.json']]) {
  const fx = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/${file}`, 'utf8'));
  const now = Object.fromEntries(gate.projetos[lang].amostra.map(s => [s.id, s]));
  for (const c of fx.chamadas.filter(c => /^(HC-1(1[3-9]|2[0-3])|RT-(6[89]|70))$/.test(c.id))) {
    const [path, line] = c.local.split(':');
    const n = now[c.id], lb = c.linha_de_base;
    const verdade = c.verdade.alvo ? `${c.verdade.relacao === 'instancia' ? 'instancia ' : ''}${c.verdade.alvo}` : `fora do projeto: ${c.verdade.externo}`;
    const test = /tests?[\/_]|tests_/.test(path) || /tests_|TestCase/.test(c.dono);
    out.push(`## ${c.id} · ${ESTRATO[c.estrato]}`, '',
      `**A chamada:** \`${c.local}\`, escrita em \`${c.dono}\`: \`${c.texto}\`. ${test ? 'Código de teste.' : 'Código de produção.'}`, '',
      `- Antes: \`${lb.resolution}\` (${lb.candidates})${lb.alvos.length ? ': ' + lb.alvos.join(', ') : ''}.`,
      `- Agora: \`${n?.resposta ?? 'não medida'}\` (categoria: ${n?.categoria ?? '—'}).`,
      '', snip(lang, path, Math.max(1, +line - 2), +line + 1), '');
    if (CONTEXTO[c.id]) {
      out.push('**Onde olhar também:**', '');
      for (const [f, a, b, text] of CONTEXTO[c.id]) out.push(`- ${text}`, '', snip(lang, f, a, b), '');
    }
    out.push(`**Verdade proposta pelo agente:** ${verdade}.`, '', `**Justificativa:** ${c.justificativa}`, '', '**Validação do usuário:** (pendente)', '');
  }
}
fs.writeFileSync(`${REPO}/docs/reports/fase-2/2.4-revisao-amostra.md`, out.join('\n'));
console.log('folha gravada');

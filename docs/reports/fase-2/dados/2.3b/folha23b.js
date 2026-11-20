// Generates docs/reports/fase-2/2.3b-revisao-amostra.md from the 2.3b entries of the fixtures (HC-88 to
// HC-97) and the gate's answers. node folha23b.js <gate resultado.json>
'use strict';
const fs = require('fs');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const HC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const gate = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const snip = (file, from, to) => {
  const L = fs.readFileSync(`${HC}/${file}`, 'utf8').split(/\r?\n/), out = [];
  for (let i = from; i <= to; i++) out.push(`${String(i).padStart(4)} | ${L[i - 1] ?? ''}`);
  return '```python\n# ' + file + '\n' + out.join('\n') + '\n```';
};
const ESTRATO = { decorador: 'chamada em decorador', campo_init: 'chamada no valor de um campo do `__init__`' };
// Where to look besides the call: [file, from, to, text].
const CONTEXTO = {
  'HC-88': [['hc/front/decorators.py', 13, 24, '`require_setting` devolve o decorador; o embrulho responde 404 sem a configuração.']],
  'HC-89': [['hc/api/decorators.py', 96, 120, '`cors` devolve o decorador que acrescenta os cabeçalhos CORS.']],
  'HC-90': [['hc/front/decorators.py', 13, 24, 'O mesmo `require_setting` do HC-88.']],
  'HC-91': [['hc/api/admin.py', 1, 12, 'O `admin` é `django.contrib.admin`, import de fora do projeto.']],
  'HC-92': [['hc/integrations/webhook/tests/test_notify.py', 1, 20, 'O `patch` vem de `unittest.mock`.']],
  'HC-93': [['hc/integrations/call/tests/test_add.py', 1, 8, 'O `override_settings` vem de `django.test`.']],
  'HC-94': [['hc/api/tests/test_smtpd.py', 42, 44, '`NullSink`, no mesmo arquivo, sem `__init__` próprio.']],
  'HC-95': [['hc/api/management/commands/sendalerts.py', 1, 20, 'O import de `ThreadPoolExecutor`.']],
  'HC-96': [['hc/lib/webauthn.py', 1, 16, 'O import de `Fido2Server`.']],
  'HC-97': [['hc/lib/webauthn.py', 1, 16, 'O import de `AttestedCredentialData`.']],
};
const fx = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/amostra-hc.json`, 'utf8'));
const now = Object.fromEntries(gate.projetos.hc.amostra.map(s => [s.id, s]));
const ex = Object.fromEntries((gate.projetos.hc.executores ?? []).map(e => [e.id, e]));
const out = [
  '# Passo 2.3b: folha de revisão da amostra HC-88 a HC-97',
  '',
  'Escrita em 20/11/2026 para a validação do usuário (DEC-22). Healthchecks em `6c05a7a`. Cada bloco traz a chamada, onde ela roda, a verdade proposta e o campo **Validação**. A verdade é **proposta pelo agente** até a sua validação.',
  '',
  '"Antes" é a resposta de `/api/calls` com o binário do 2.3a (`a11a582`), registrada na fixture. "Agora" é a resposta com o binário do 2.3b, medida pelo portão. "Executor" é a declaração que roda a chamada (DEC-40): o portão registra se o `/api/calls` dela mostra a chamada.',
  '',
];
for (const c of fx.chamadas.filter(c => /^HC-(8[89]|9\d)$/.test(c.id))) {
  const [file, line] = c.local.split(':');
  const n = now[c.id];
  const lb = c.linha_de_base;
  const verdade = c.verdade.alvo ? `${c.verdade.alvo}${c.verdade.relacao ? ` (relação: ${c.verdade.relacao})` : ''}` : `fora do projeto: ${c.verdade.externo}`;
  out.push(`## ${c.id} · ${ESTRATO[c.estrato]}`, '',
    `**A chamada:** \`${c.local}\`, escrita em \`${c.dono}\`: \`${c.texto}\`. ${/tests?\//.test(file) ? 'Código de teste.' : 'Código de produção.'}`, '',
    `- Antes: \`${lb.resolution}\` (${lb.candidates})${lb.alvos.length ? ': ' + lb.alvos.join(', ') : ''}${lb.relacao ? ` [${lb.relacao}]` : ''}.`,
    `- Agora: \`${n?.resposta ?? 'não medida'}\` (categoria: ${n?.categoria ?? '—'}).`,
    ...(c.verdade.executor ? [`- Executor: \`${c.verdade.executor}\`; o \`/api/calls\` dele ${ex[c.id]?.mostra ? 'mostra' : '**não** mostra'} a chamada.`] : []),
    '', snip(file, Math.max(1, +line - 2), +line + 2), '', '**Onde olhar também:**', '');
  for (const [f, a, b, text] of CONTEXTO[c.id] ?? []) out.push(`- ${text}`, '', snip(f, a, b), '');
  out.push(`**Verdade proposta:** ${verdade}.`, '', `**Justificativa:** ${c.justificativa}`, '', '**Validação:** ☐ concordo ☐ discordo (verdade certa: ______ )', '');
}
fs.writeFileSync(`${REPO}/docs/reports/fase-2/2.3b-revisao-amostra.md`, out.join('\n'));
console.log('folha gravada');

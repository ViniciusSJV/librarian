// Gera docs/reports/fase-2/2.4d0-revisao-amostra.md com HC-158 a HC-171 e a resposta do portão.   node folha24d0.js <resultado.json>
'use strict';
const fs = require('fs');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian', SRC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const gate = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const lines = f => fs.readFileSync(`${SRC}/${f}`, 'utf8').split(/\r?\n/);
const snip = (file, from, to) => {
  const L = lines(file), out = [];
  for (let i = from; i <= to; i++) out.push(`${String(i).padStart(4)} | ${L[i - 1] ?? ''}`);
  return '```python\n# ' + file + '\n' + out.join('\n') + '\n```';
};
const grep = (f, re) => lines(f).findIndex(l => re.test(l)) + 1;
const classLine = (file, line) => { const L = lines(file); for (let i = line - 1; i >= 0; i--) if (/^class /.test(L[i])) return i + 1; return null; };
const ESTRATO = { classe_pelo_nome: 'muda: de homonym a type', classe_pelo_nome_unica: 'guarda: já era única', classe_pelo_nome_fora: 'guarda: método de fora do projeto', classe_pelo_nome_atributo: 'guarda: o receptor ou o nome não é o que parece' };
const TV = 'hc/integrations/telegram/views.py', TT = 'hc/integrations/telegram/transport.py', SV = 'hc/integrations/signal/views.py', M = 'hc/api/models.py';
const EXTRA = {
  'HC-158': [['hc/api/decorators.py', 33, 35, 'A função solta homônima, que é um decorador de view.']],
  'HC-160': [[M, 929, 930, 'O load de EmailConf, que não é este.']],
  'HC-161': [[M, 909, 910, 'O load de PdConf, que não é este.']],
  'HC-163': [[SV, grep(SV, /import Signal\b|import .*\bSignal\b/), grep(SV, /import Signal\b|import .*\bSignal\b/), 'O import de Signal.'], [TT, 54, 55, 'O send de Telegram, que não é este.']],
  'HC-164': [[TT, 66, 69, 'O import dentro da função.']],
  'HC-165': [[SV, grep(SV, /import Signal\b|import .*\bSignal\b/), grep(SV, /import Signal\b|import .*\bSignal\b/), 'O import de Signal.']],
  'HC-167': [[TT, grep(TT, /class ErrorModel/), grep(TT, /class ErrorModel/), 'A classe aninhada: a base é do pydantic.']],
  'HC-168': [[TV, 39, 43, 'A classe: declara load, e não model_validate.']],
  'HC-169': [['hc/lib/emails.py', grep('hc/lib/emails.py', /import Thread/), grep('hc/lib/emails.py', /import Thread/), 'O import de Thread.']],
  'HC-170': [[M, 1437, 1440, 'A classe não declara objects: é o gerenciador do Django.']],
  'HC-171': [[M, 703, 704, 'A classe aninhada em Ping.']],
};
const out = ['# Passo 2.4d-0: folha de revisão da amostra HC-158 a HC-171', '',
  'Escrita em 25/11/2026. Healthchecks em `6c05a7a`. A Q5 da Parada 1 aprovou os estratos. A verdade, proposta pelo agente, foi **validada pelo usuário em 25/11/2026** (HC-158, HC-162, HC-163, HC-164, HC-167 e HC-169 conferidas no editor; DEC-22).', '',
  `"Antes" é a resposta de \`/api/calls\` com o binário do 2.4c (\`ac33664\`). "Agora" é a do 2.4d-0, medida pelo portão (fontes \`${process.argv[3]}\`). Mudam HC-158 a HC-163; as outras guardam contra uma regra larga demais. A HC-169 é um limite conhecido do 2.2: sai igual antes e agora.`, ''];
const fx = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/amostra-hc.json`, 'utf8'));
const now = Object.fromEntries(gate.projetos.hc.amostra.map(s => [s.id, s]));
for (const c of fx.chamadas.filter(c => Number(c.id.slice(3)) >= 158)) {
  const [path, line] = c.local.split(':'), n = now[c.id], lb = c.linha_de_base;
  const v = c.verdade.alvo ? c.verdade.alvo + (c.verdade.relacao === 'instancia' ? ' (instancia)' : '') : `fora do projeto: ${c.verdade.externo}`;
  out.push(`## ${c.id} · ${ESTRATO[c.estrato]}`, '', `**A chamada:** \`${c.local}\`, escrita em \`${c.dono}\`: \`${c.texto.replace(/\s+/g, ' ').slice(0, 100)}\`. ${/tests?\//.test(path) ? 'Código de teste.' : 'Código de produção.'}`, '',
    `- Antes: \`${lb.resolution}\` (${lb.candidates})${lb.alvos.length ? ': ' + lb.alvos.join(', ') : ''}.`, `- Agora: \`${n?.resposta}\` (categoria: ${n?.categoria}).`, '', snip(path, Math.max(1, +line - 2), +line + 1), '');
  const m = /\((hc\/[^:]+):(\d+)\)/.exec(c.verdade.alvo ?? '');
  if (m || EXTRA[c.id]) out.push('**Onde olhar também:**', '');
  if (m && !c.verdade.relacao) { const t = classLine(m[1], +m[2]); out.push('- A classe do alvo e o alvo.', '', snip(m[1], t, t), '', snip(m[1], +m[2], +m[2] + 2), ''); }
  for (const [f, a, b, text] of EXTRA[c.id] ?? []) out.push(`- ${text}`, '', snip(f, a, b), '');
  out.push(`**Verdade:** ${v}.`, '', `**Justificativa:** ${c.justificativa}`, '', `**Validação do usuário:** ${c.verdade_validada_por}`, '');
}
fs.writeFileSync(`${REPO}/docs/reports/fase-2/2.4d0-revisao-amostra.md`, out.join('\n'));

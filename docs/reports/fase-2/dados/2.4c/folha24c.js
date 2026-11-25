// Gera docs/reports/fase-2/2.4c-revisao-amostra.md com HC-144 a HC-157 e a resposta do portão.   node folha24c.js <resultado.json>
'use strict';
const fs = require('fs');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian', SRC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const gate = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const snip = (file, from, to) => {
  const L = fs.readFileSync(`${SRC}/${file}`, 'utf8').split(/\r?\n/), out = [];
  for (let i = from; i <= to; i++) out.push(`${String(i).padStart(4)} | ${L[i - 1] ?? ''}`);
  return '```python\n# ' + file + '\n' + out.join('\n') + '\n```';
};
const ESTRATO = { despacho: 'despacho com sobrescrita', despacho_sem_sobrescrita: 'há subclasses, e nenhuma sobrescreve', despacho_irma: 'outra classe declara o nome, sem ser subclasse', despacho_construtor: 'construtor exato', despacho_classe: 'chamada pelo nome da classe' };
const classLine = (file, line) => { const L = fs.readFileSync(`${SRC}/${file}`, 'utf8').split(/\r?\n/); for (let i = line - 1; i >= 0; i--) if (/^class /.test(L[i])) return i + 1; return null; };
const defLine = (file, line) => { const L = fs.readFileSync(`${SRC}/${file}`, 'utf8').split(/\r?\n/); for (let i = line - 1; i >= 0; i--) if (/^def /.test(L[i])) return i + 1; return line; };
const S = 'hc/integrations/';
const EXTRA = {
  'HC-144': [[S + 'slack/transport.py', 88, 94, 'O corpo da base, que diz que as subclasses podem sobrescrever.'], [S + 'discord/transport.py', 19, 27, 'Discord chama self.payload e sobrescreve fix_asterisks.']],
  'HC-153': [[S + 'call/views.py', 21, 33, 'As duas atribuições de form.'], ['hc/front/forms.py', 155, 157, 'A subclasse que sobrescreve get_json, PhoneUpDownForm.'], ['hc/front/forms.py', 168, 175, 'O get_json dela.']],
  'HC-154': [[S + 'sms/views.py', 20, 34, 'As atribuições de form.']],
  'HC-155': [['hc/api/tests/test_smtpd.py', 47, 49, 'TestServer.']],
};
const out = ['# Passo 2.4c: folha de revisão da amostra HC-144 a HC-157', '',
  'Escrita em 25/11/2026. Healthchecks em `6c05a7a`. A Q7 da Parada 1 aprovou os estratos. A verdade, proposta pelo agente, foi **validada pelo usuário em 25/11/2026** (HC-144 e HC-156 conferidos no editor; DEC-22).', '',
  '"Antes" é a resposta de `/api/calls` com o binário do 2.4b (`74faa87`). "Agora" é a do 2.4c, medida pelo portão. Só a HC-144 muda. As outras guardam contra uma regra larga demais. **As 2 ambíguas novas são HC-144 (muda de link_falso para ambígua) e HC-156 (já era homonym antes; só entra na amostra agora).** As outras 12 são corretas.', ''];
const fx = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/amostra-hc.json`, 'utf8'));
const now = Object.fromEntries(gate.projetos.hc.amostra.map(s => [s.id, s]));
for (const c of fx.chamadas.filter(c => Number(c.id.slice(3)) >= 144)) {
  const [path, line] = c.local.split(':'), n = now[c.id], lb = c.linha_de_base;
  const v = c.verdade.ambigua ? `ambígua, um destes ${c.verdade.ambigua.length} corpos: ${c.verdade.ambigua.map(x => x.replace(/ \(.*$/, '')).join(', ')}` : c.verdade.alvo ?? `fora do projeto: ${c.verdade.externo}`;
  out.push(`## ${c.id} · ${ESTRATO[c.estrato]}`, '', `**A chamada:** \`${c.local}\`, escrita em \`${c.dono}\`: \`${c.texto.replace(/\s+/g, ' ').slice(0, 100)}\`. ${/tests?\/|hc\/test\.py/.test(path) ? 'Código de teste.' : 'Código de produção.'}`, '',
    `- Antes: \`${lb.resolution}\` (${lb.candidates})${lb.alvos.length ? ': ' + lb.alvos.join(', ') : ''}.`, `- Agora: \`${n?.resposta}\` (categoria: ${n?.categoria}).`, '', snip(path, Math.max(1, +line - 2), +line + 1), '', '**Onde olhar também:**', '');
  const k = c.dono.includes('.') ? classLine(path, +line) : null; // a chamada de uma função não é de uma classe
  if (!k) out.push(`- A função que escreve a chamada, \`${c.dono}\` (não está numa classe).`, '', snip(path, defLine(path, +line), defLine(path, +line)), '');
  if (k) out.push('- A classe que escreve a chamada.', '', snip(path, k, k), '');
  const m = /\((hc\/[^:]+):(\d+)\)/.exec(c.verdade.alvo ?? '');
  if (m) { const t = classLine(m[1], +m[2]); out.push('- A classe do alvo e o alvo.', '', snip(m[1], t, t), '', snip(m[1], +m[2], +m[2] + 2), ''); }
  for (const [f, a, b, text] of EXTRA[c.id] ?? []) out.push(`- ${text}`, '', snip(f, a, b), '');
  out.push(`**Verdade:** ${v}.${c.verdade.observacao ? ' ' + c.verdade.observacao + '.' : ''}`, '', `**Justificativa:** ${c.justificativa}`, '', `**Validação do usuário:** ${c.verdade_validada_por}`, '');
}
fs.writeFileSync(`${REPO}/docs/reports/fase-2/2.4c-revisao-amostra.md`, out.join('\n'));

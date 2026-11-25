// Gera docs/reports/fase-2/2.4b-revisao-amostra.md com HC-124 a HC-143 e a resposta do portão.   node folha24b.js <resultado.json>
'use strict';
const fs = require('fs');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian', SRC = 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks';
const gate = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const snip = (file, from, to) => {
  const L = fs.readFileSync(`${SRC}/${file}`, 'utf8').split(/\r?\n/), out = [];
  for (let i = from; i <= to; i++) out.push(`${String(i).padStart(4)} | ${L[i - 1] ?? ''}`);
  return '```python\n# ' + file + '\n' + out.join('\n') + '\n```';
};
const ESTRATO = { heranca_self: 'método herdado por self', heranca_tipo: 'método herdado pelo tipo do receptor', heranca_super: 'super()', fora_da_cadeia: 'candidato fora da cadeia de bases', cls: 'cls num classmethod', propriedade: '@property de retorno anotado' };
const classLine = (file, line) => { const L = fs.readFileSync(`${SRC}/${file}`, 'utf8').split(/\r?\n/); for (let i = line - 1; i >= 0; i--) if (/^class /.test(L[i])) return i + 1; return null; };
const EXTRA = {
  'HC-134': [], 'HC-137': [['hc/api/models.py', 788, 792, 'O `Ping.get_kind_display` que o grafo dava como único candidato.']],
  'HC-141': [['hc/api/models.py', 1138, 1150, 'A propriedade, com o retorno anotado.'], ['hc/api/transports.py', 66, 74, 'O corpo abstrato.'], ['hc/integrations/slack/transport.py', 96, 98, 'Uma das 30 sobrescritas.']],
  'HC-142': [['hc/api/models.py', 1138, 1139, 'A propriedade, com o retorno anotado.'], ['hc/api/transports.py', 76, 85, 'O corpo de verdade (não é abstrato).']],
  'HC-143': [['hc/api/models.py', 1313, 1315, 'A propriedade `zulip`.']],
};
const out = ['# Passo 2.4b: folha de revisão da amostra HC-124 a HC-143', '',
  'Escrita em 24/11/2026. Healthchecks em `6c05a7a`. A Q6 da Parada 1 aprovou os estratos. A verdade, proposta pelo agente, foi validada pelo usuário em 25/11/2026: HC-124 a HC-139, HC-141 e HC-143 ok (HC-137 e HC-141 conferidos no editor; DEC-22). **A de HC-140 e HC-142 foi corrigida pelo usuário** para ambígua (o método do tipo estático mais as sobrescritas): os dois são `link_falso` conhecidos, até o 2.4c.', '',
  '"Antes" é a resposta de `/api/calls` com o binário do 2.4a (`c3e4b30`). "Agora" é a do 2.4b, medida pelo portão.', ''];
const fx = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/amostra-hc.json`, 'utf8'));
const now = Object.fromEntries(gate.projetos.hc.amostra.map(s => [s.id, s]));
for (const c of fx.chamadas.filter(c => Number(c.id.slice(3)) >= 124)) {
  const [path, line] = c.local.split(':'), n = now[c.id], lb = c.linha_de_base;
  const v = c.verdade.ambigua ? `ambígua, um destes ${c.verdade.ambigua.length} corpos: ${c.verdade.ambigua.map(x => x.replace(/ \(.*$/, '')).join(', ')}` : c.verdade.alvo ?? `fora do projeto: ${c.verdade.externo}`;
  out.push(`## ${c.id} · ${ESTRATO[c.estrato]}`, '', `**A chamada:** \`${c.local}\`, escrita em \`${c.dono}\`: \`${c.texto.replace(/\s+/g, ' ').slice(0, 100)}\`. ${/tests?\/|hc\/test\.py/.test(path) ? 'Código de teste.' : 'Código de produção.'}`, '',
    `- Antes: \`${lb.resolution}\` (${lb.candidates})${lb.alvos.length ? ': ' + lb.alvos.join(', ') : ''}.`, `- Agora: \`${n?.resposta}\` (categoria: ${n?.categoria}).`, '', snip(path, Math.max(1, +line - 2), +line + 1), '', '**Onde olhar também:**', '');
  const k = classLine(path, +line);
  if (k) out.push('- A classe que escreve a chamada.', '', snip(path, k, k), '');
  const m = /\((hc\/[^:]+):(\d+)\)/.exec(c.verdade.alvo ?? '');
  if (m) { const t = classLine(m[1], +m[2]); out.push('- A classe do alvo e o alvo.', '', snip(m[1], t, t), '', snip(m[1], +m[2], +m[2] + 2), ''); }
  for (const [f, a, b, text] of EXTRA[c.id] ?? []) out.push(`- ${text}`, '', snip(f, a, b), '');
  out.push(`**Verdade${['HC-140', 'HC-142'].includes(c.id) ? ' (corrigida pelo usuário)' : ''}:** ${v}.${c.verdade.observacao ? ' ' + c.verdade.observacao + '.' : ''}`, '', `**Justificativa:** ${c.justificativa}`, '', `**Validação do usuário:** ${c.verdade_validada_por}`, '');
}
fs.writeFileSync(`${REPO}/docs/reports/fase-2/2.4b-revisao-amostra.md`, out.join('\n'));

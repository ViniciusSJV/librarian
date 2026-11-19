// Generates docs/reports/fase-2/2.2-revisao-amostra.md from the 2.2 entries of the fixtures.
// node folha22.js <compara-amostra.json>   (answers of the binary for each id, from the gate run)
'use strict';
const fs = require('fs');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const SRC = { hc: 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks', rt: 'C:/Users/beatl/Documents/Projetos/renderer' };
const answers = process.argv[2] ? JSON.parse(fs.readFileSync(process.argv[2], 'utf8')) : {};
const lines = (lang, file) => fs.readFileSync(`${SRC[lang]}/${file}`, 'utf8').split(/\r?\n/);
const snip = (lang, file, from, to) => {
  const L = lines(lang, file); const out = [];
  for (let i = from; i <= to; i++) out.push(`${String(i).padStart(4)} | ${L[i - 1] ?? ''}`);
  return '```' + (lang === 'hc' ? 'python' : 'rust') + `\n# ${file}\n`.replace('#', lang === 'hc' ? '#' : '//') + out.join('\n') + '\n```';
};
// Origin of the receiver or the import, and the target's declaration, per id: [file, from, to, text].
const ORIGEM = {
  'HC-61': [['hc/lib/signing.py', 3, 3, '`import time`: o receptor `time` é o módulo da biblioteca padrão, de fora do projeto.']],
  'HC-62': [[null, 0, 0, 'O receptor é o literal `"\\n"`, uma `str`.']],
  'HC-63': [['hc/lib/curl.py', 32, 35, '`self.content` recebe `content: bytes` no `__init__` de `Response`.']],
  'HC-64': [['hc/api/tests/test_check_model.py', 278, 278, '`flip = Flip(owner=check)`: um model do Django.']],
  'HC-65': [['hc/api/tests/test_tokenbucket.py', 52, 52, '`obj = TokenBucket(...)`: um model do Django.']],
  'HC-66': [['hc/integrations/rocketchat/tests/test_add.py', 6, 9, '`self.client` é o cliente de teste do Django: `AddRocketChatTestCase` herda de `BaseTestCase`, que herda de `TestCase`.']],
  'HC-67': [['hc/accounts/models.py', 24, 24, '`from hc.lib import emails`: o receptor é o módulo `hc/lib/emails.py` do projeto.']],
  'HC-68': [['hc/integrations/matrix/forms.py', 5, 5, '`from hc.integrations.matrix import client`: o módulo `hc/integrations/matrix/client.py`.']],
  'HC-69': [['hc/api/transports.py', 10, 10, '`from hc.lib import curl`: o módulo `hc/lib/curl.py`.']],
  'HC-70': [['hc/accounts/models.py', 24, 24, '`from hc.lib import emails`: o módulo `hc/lib/emails.py`.']],
  'HC-71': [['hc/integrations/github/client.py', 8, 8, '`from hc.lib import curl`: o módulo `hc/lib/curl.py`.']],
  'HC-72': [[null, 0, 0, 'Chamada simples, sem receptor, dentro de `emails.login`, no próprio módulo `hc/lib/emails.py`.']],
  'HC-73': [['hc/integrations/webhook/views.py', 8, 8, '`from django.shortcuts import redirect, render`: chamada simples a uma função do Django.']],
  'HC-74': [['hc/lib/tests/test_curl.py', 10, 10, '`from hc.lib.curl import CurlError, request`.']],
  'HC-75': [['hc/lib/tests/test_emails.py', 9, 9, '`from hc.lib.emails import EmailThread, send`.']],
  'RT-56': [['src/bin/cena_video.rs', 105, 105, '`let mut balls: Vec<(f64, f64, f64)>`: `let` anotado com um tipo de fora do projeto, sem referência.']],
  'RT-57': [['src/obj.rs', 123, 123, 'Parâmetro `mut objects: Vec<Object>`.']],
  'RT-58': [['src/groups.rs', 11, 13, 'Campo `children: Vec<Object>`; a chamada está dentro do próprio `Groups::len`.']],
  'RT-59': [['src/world.rs', 15, 15, 'Campo `inverses: Vec<crate::matrix::Matrix<4>>`.']],
  'RT-60': [['src/tuple.rs', 9, 9, 'Campo `w: f64`.'], ['src/equivalent.rs', 11, 15, 'O projeto implementa `Equivalence<f64> for f64`: por isso o tipo de fora do projeto não derruba nada aqui.']],
  'RT-61': [[null, 0, 0, 'Chamada simples `drop(writer)`, sem receptor nem caminho: a função `drop` do prelúdio.']],
  'RT-62': [['src/obj.rs', 63, 63, '`let mut next = || ...`: `next` é uma closure local.']],
};
const ALVO = {
  'HC-67': ['hc/lib/emails.py', 112, 113], 'HC-68': ['hc/integrations/matrix/client.py', 26, 27], 'HC-69': ['hc/lib/curl.py', 55, 58],
  'HC-70': ['hc/lib/emails.py', 92, 93], 'HC-71': ['hc/lib/curl.py', 210, 212], 'HC-72': ['hc/lib/emails.py', 71, 72],
  'HC-74': ['hc/lib/curl.py', 55, 58], 'HC-75': ['hc/lib/emails.py', 71, 72], 'RT-60': ['src/equivalent.rs', 12, 14],
};
const MOTIVO = {
  metodo_funcao: 'a chamada tem receptor, e o único candidato do projeto é uma função solta (ou um método que o receptor, ligado a import de fora do projeto, não alcança)',
  espelho: 'a chamada não tem receptor, e os candidatos do projeto são métodos',
  tipo_derruba: 'o receptor tem tipo escrito de fora do projeto, sem referência nem `Box`/`Rc`/`Arc`, e o único candidato é método inerente de um tipo do projeto',
};
const NOMES = { metodo_funcao: 'método que ia para função solta', modulo_f: '`modulo.f()` legítimo', tipo_derruba: 'tipo derruba o `unique`', espelho: 'chamada simples não alcança método' };
const out = [];
out.push('# Passo 2.2: folha de revisão da amostra HC-61 a HC-75 e RT-56 a RT-62', '');
out.push('Escrita em 19/11/2026 para a validação do usuário (DEC-22). Healthchecks em `6c05a7a`, ray tracer em `e38254b`. Cada bloco traz a chamada, a origem do receptor ou do import, o alvo proposto (ou "sem candidato", com o motivo e o alvo externo) e um campo **Validação** em branco: preencha com `ok` ou com a correção.', '');
out.push('Os estratos ficam fora da comparação do portão até entrarem numa linha de base aprovada (DEC-37). Ao validar, a verdade de cada caso passa de "proposta pelo agente" para validada na fixture.', '');
out.push('"Agora" é a resposta de `/api/calls` com o binário do 2.2, medida pelo portão. "Antes" é a linha de base registrada na fixture.', '');
out.push('Os alvos externos alimentam a Fase 2B (externos conhecidos). Hoje eles saem como `no_candidate_in_project`, sem link.', '');
for (const lang of ['hc', 'rt']) {
  const fx = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/amostra-${lang}.json`, 'utf8'));
  for (const c of fx.chamadas.filter(c => /passo 2\.2/.test(c.verdade_validada_por))) {
    const [file, line] = c.local.split(':'); const n = +line;
    out.push(`## ${c.id} · ${NOMES[c.estrato]}`, '');
    const lb = c.linha_de_base;
    const now = answers[c.id];
    out.push(`**A chamada:** \`${c.local}\`, em \`${c.dono}\`: \`${c.texto}\`. ${/tests?\//.test(file) ? 'Código de teste.' : 'Código de produção.'}`, '');
    out.push(`- Antes: \`${lb.resolution}\` (${lb.candidates} candidato${lb.candidates === 1 ? '' : 's'})${lb.alvos.length ? ': ' + lb.alvos.map(a => '`' + a + '`').join(', ') : ''}.`);
    if (now) out.push(`- Agora: \`${now.resposta}\` (categoria: ${now.categoria}).`);
    out.push('', snip(lang, file, Math.max(1, n - 1), n + 1), '');
    out.push('**Origem do receptor ou do import:**', '');
    for (const [f, a, b, text] of ORIGEM[c.id]) { out.push(`- ${text}`); if (f) out.push('', snip(lang, f, a, b)); out.push(''); }
    if (c.verdade.alvo) {
      out.push(`**O alvo proposto:** \`${c.verdade.alvo}\`.`, '');
      const t = ALVO[c.id]; if (t) out.push(snip(lang, t[0], t[1], t[2]), '');
    } else {
      out.push(`**O alvo proposto:** sem candidato no projeto. Motivo: ${MOTIVO[c.estrato]}. Alvo externo: **${c.verdade.externo}**.`, '');
    }
    out.push(`**Justificativa:** ${c.justificativa}`, '');
    out.push('**Validação:** ', '');
  }
}
fs.writeFileSync(`${REPO}/docs/reports/fase-2/2.2-revisao-amostra.md`, out.join('\n'));
console.log('ok', out.length);

// Generates docs/reports/fase-2/2.3-revisao-amostra.md from the 2.3a entries of the fixtures and the gate's
// answers. node folha23.js <gate resultado.json>
'use strict';
const fs = require('fs');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const SRC = { hc: 'C:/Users/beatl/Documents/Projetos/pilotos/healthchecks', rt: 'C:/Users/beatl/Documents/Projetos/renderer' };
const gate = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const lines = (lang, file) => fs.readFileSync(`${SRC[lang]}/${file}`, 'utf8').split(/\r?\n/);
const snip = (lang, file, from, to) => {
  const L = lines(lang, file), out = [];
  for (let i = from; i <= to; i++) out.push(`${String(i).padStart(4)} | ${L[i - 1] ?? ''}`);
  return '```' + (lang === 'hc' ? 'python' : 'rust') + `\n${lang === 'hc' ? '#' : '//'} ${file}\n` + out.join('\n') + '\n```';
};
const ESTRATO = { tipo_py: 'tipo do receptor em Python', instancia: 'instanciação', receiver_py_teto: '`self.m()` acima do teto (2.3a-0)' };
// The origin of the receiver's type or of the class, per id: [file, from, to, text].
const ORIGEM = {
  'HC-76': [['hc/api/views.py', 198, 209, 'Parâmetro anotado `check: Check | None = None`, reatribuído por `Check.objects.get(..)` (de fora do projeto). A anotação vale para a função inteira, e `| None` vale como `Check`.']],
  'HC-77': [['hc/api/models.py', 624, 624, '`summary = DowntimeRecorder(..)`, classe do mesmo arquivo.']],
  'HC-78': [['hc/accounts/views.py', 41, 41, 'O import de `CreateHelper`.'], ['hc/accounts/views.py', 727, 727, '`helper = CreateHelper(..)`.']],
  'HC-79': [['hc/api/management/commands/smtpd.py', 105, 107, 'Campo do `__init__`: `self.stdout = stdout`, com `stdout: LogSink`, um `Protocol` (smtpd.py:28).'], ['hc/api/management/commands/smtpd.py', 140, 140, 'Em produção, o comando passa o próprio `self.stdout` (o `OutputWrapper` do Django).'], ['hc/api/tests/test_smtpd.py', 47, 49, 'Nos testes, `TestServer` usa `NullSink()`.']],
  'HC-80': [['hc/api/tests/test_check_model.py', 182, 182, '`check = Check(..)` no mesmo teste.']],
  'HC-81': [['hc/logs/tests/test_sendlogs.py', 9, 9, 'O import de `Command`: há 17 classes `Command` no projeto (acima do teto); o import diz qual é.']],
  'HC-82': [['hc/api/management/commands/sendalerts.py', 18, 18, 'O import de `Flip`.']],
  'HC-83': [['hc/integrations/webhook/views.py', 14, 14, '`forms` é o módulo `hc/integrations/webhook/forms.py`, do projeto.']],
  'HC-84': [['hc/integrations/github/transport.py', 8, 8, 'O import de `TransportError`, que declara `__init__` (o percurso segue nele).']],
  'HC-85': [['hc/integrations/victorops/tests/test_notify.py', 9, 9, 'O import de `Check`.']],
  'HC-86': [['hc/lib/tests/test_curl.py', 13, 13, 'O dublê `FakeCurl`, declarado no próprio arquivo de teste.']],
  'HC-87': [['hc/integrations/slack/tests/test_notify.py', 15, 16, '`_setup_data` da própria classe; há 9 `_setup_data` nos testes (acima do teto).']],
  'RT-63': [['src/bin/cena_video.rs', 26, 26, 'A struct de tupla `Sorteio`.']],
  'RT-64': [['src/object.rs', 50, 50, 'A variante `Object::Cone`; a struct `Cone` tem o mesmo nome.']],
  'RT-65': [['src/patterns.rs', 26, 26, 'A variante `Patterns::Stripe`; a struct `Stripe` tem o mesmo nome.']],
};
const out = [`# Passo 2.3a: folha de revisão da amostra HC-76 a HC-87 e RT-63 a RT-65

Escrita em 19/11/2026 para a validação do usuário (DEC-22). Healthchecks em \`6c05a7a\`, ray tracer em \`e38254b\`. Cada bloco traz a chamada, a origem do tipo do receptor ou da classe, o alvo proposto e um campo **Validação** em branco: preencha com \`ok\` ou com a correção. A verdade de HC-79 já foi dada pelo usuário (Q6 da Parada 1 do 2.3).

Os estratos ficam fora da comparação do portão até entrarem numa linha de base aprovada (DEC-37).

"Antes" é a resposta de \`/api/calls\` com o binário do 2.3a-0 (\`8c689d4\`), registrada na fixture. "Agora" é a resposta com o binário do 2.3a, medida pelo portão.
`];
for (const [lang, file] of [['hc', 'amostra-hc.json'], ['rt', 'amostra-rt.json']]) {
  const fx = JSON.parse(fs.readFileSync(`${REPO}/tools/precisao/fixtures/${file}`, 'utf8')).chamadas.filter(c => ORIGEM[c.id]);
  for (const c of fx) {
    const now = gate.projetos[lang].amostra.find(s => s.id === c.id);
    const [path, line] = c.local.split(':');
    const teste = /(^|\/)tests?\/|test_/.test(path) || /tests?_/.test(c.dono);
    const lb = c.linha_de_base;
    out.push(`## ${c.id} · ${ESTRATO[c.estrato]}`, '',
      `**A chamada:** \`${c.local}\`, em \`${c.dono}\`: \`${c.texto.split('\n')[0]}\`. Código de ${teste ? 'teste' : 'produção'}.`, '',
      `- Antes: \`${lb.resolution}\` (${lb.candidates})${lb.alvos.length ? ': ' + lb.alvos.map(a => '`' + a + '`').join(', ') : ''}.`,
      `- Agora: \`${now.resposta}\` (categoria: ${now.categoria}).`, '',
      snip(lang, path, Math.max(1, +line - 1), +line + 1), '', '**Origem do tipo ou da classe:**', '');
    for (const [f, a, b, text] of ORIGEM[c.id]) { out.push(`- ${text}`, ''); if (f) out.push(snip(lang, f, a, b), ''); }
    const v = c.verdade;
    out.push(`**O alvo proposto:** ${v.alvo ? '`' + v.alvo + '`' + (c.estrato === 'instancia' ? ', com `relation: instantiates`' : '') : 'ambígua; os corpos que podem rodar: ' + v.ambigua.map(x => '`' + x + '`').join(' e ') + '. Nenhum link único é certo'}.`, '',
      `**Justificativa:** ${c.justificativa}`, '',
      `**Validação:** ${c.verdade_validada_por.startsWith('usuário') ? `ok (${c.verdade_validada_por})` : ''}`, '');
  }
}
fs.writeFileSync(`${REPO}/docs/reports/fase-2/2.3-revisao-amostra.md`, out.join('\n'));
console.log('ok', out.length);

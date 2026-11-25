// Passo 2.4d-0, Parada 1: a amostra proposta HC-158 a HC-171 (verdade PROPOSTA pelo agente; não é a fixture).
//   node amostra24d0.js <dump-antes> <fixture-original> <fixture-com-os-novos>
'use strict';
const fs = require('fs');
const [dump, fx, out] = process.argv.slice(2);
const { load } = require('../2.4/load.js');
const P = load(dump, 'healthchecks-25597e41');
const A = 'TokenBucket.authorize (hc/api/models.py:1442)';
const L = [
  ['classe_pelo_nome', 'hc/api/models.py', 1479, 'TokenBucket.authorize(', { alvo: A }, 'staticmethod, mesmo módulo; a função solta homônima é hc/api/decorators.py:33 (DEC-38)'],
  ['classe_pelo_nome', 'hc/api/models.py', 1587, 'TokenBucket.authorize(', { alvo: A }, 'staticmethod, mesmo módulo, com palavra-chave'],
  ['classe_pelo_nome', 'hc/api/models.py', 1293, 'PdConf.load(', { alvo: 'PdConf.load (hc/api/models.py:909)' }, 'classmethod, mesmo módulo, entre três load'],
  ['classe_pelo_nome', 'hc/api/models.py', 1307, 'EmailConf.load(', { alvo: 'EmailConf.load (hc/api/models.py:929)' }, 'classmethod, mesmo módulo, entre três load'],
  ['classe_pelo_nome', 'hc/integrations/telegram/views.py', 57, 'TelegramCallback.load(', { alvo: 'TelegramCallback.load (hc/integrations/telegram/views.py:42)' }, 'classmethod, mesmo módulo, entre três load'],
  ['classe_pelo_nome', 'hc/integrations/signal/views.py', 119, 'Signal.send(', { alvo: 'Signal.send (hc/integrations/signal/transport.py:60)' }, 'classmethod, importada, entre dois send'],
  ['classe_pelo_nome_unica', 'hc/integrations/telegram/transport.py', 70, 'TokenBucket.authorize_telegram(', { alvo: 'TokenBucket.authorize_telegram (hc/api/models.py:1510)' }, 'import dentro da função'],
  ['classe_pelo_nome_unica', 'hc/integrations/signal/views.py', 84, 'Signal._read_replies(', { alvo: 'Signal._read_replies (hc/integrations/signal/transport.py:105)' }, 'classmethod, importada'],
  ['classe_pelo_nome_unica', 'hc/api/tests/test_tokenbucket.py', 71, 'TokenBucket.record_s3_get_object_error(', { alvo: 'TokenBucket.record_s3_get_object_error (hc/api/models.py:1583)' }, 'em teste'],
  ['classe_pelo_nome_fora', 'hc/integrations/telegram/transport.py', 41, 'Telegram.ErrorModel.model_validate_json(', { externo: 'pydantic: BaseModel.model_validate_json' }, 'classe aninhada; o método é do pydantic, na base de fora'],
  ['classe_pelo_nome_fora', 'hc/integrations/telegram/views.py', 50, 'TelegramCallback.model_validate(', { externo: 'pydantic: BaseModel.model_validate' }, 'a classe declara load; model_validate é do pydantic'],
  ['classe_pelo_nome_fora', 'hc/lib/emails.py', 19, 'Thread.__init__(', { externo: 'threading: Thread.__init__' }, 'objeto explícito; Thread é de fora'],
  ['classe_pelo_nome_atributo', 'hc/api/models.py', 1447, 'TokenBucket.objects.get_or_create(', { externo: 'Django: Manager.get_or_create' }, 'objects é atributo da classe, não classe; o gerenciador é do Django'],
  ['classe_pelo_nome_atributo', 'hc/integrations/email/tests/test_notify.py', 187, 'Ping.GetBodyError(', { alvo: 'Ping.GetBodyError (hc/api/models.py:703)', relacao: 'instancia' }, 'classe aninhada construída; em teste'],
];
const f = JSON.parse(fs.readFileSync(fx, 'utf8'));

let n = 158;
for (const [estrato, p, line, t, v, why] of L) {
  const c = [...P.calls.values()].find(c => c.path === p && c.text.startsWith(t) && (c.line === line));
  if (!c) { console.log('NÃO ACHADA', p, line, t, [...P.calls.values()].filter(c => c.path === p && c.text.startsWith(t)).map(c => c.line)); continue; }
  const id = `HC-${n++}`, occ = `${p}@${c.sb}-${c.eb}`, dup = f.chamadas.find(x => x.ocorrencia === occ);
  console.log(id, estrato, `${p}:${line}`, c.text.split('\n')[0].slice(0, 60), '| hoje', c.res, c.cands, c.targets.map(P.show).join('; '), dup ? `JÁ NA AMOSTRA ${dup.id}` : '');
  f.chamadas.push({ id, parte_c: false, estrato, ocorrencia: occ, local: `${p}:${line}`, texto: c.text, dono: P.N[c.owner].label, linha_de_base: { resolution: c.res, candidates: c.cands, alvos: c.targets.map(P.show) }, verdade: v, certeza: 'alta', justificativa: why, verdade_validada_por: 'PROPOSTA do agente' });
}
fs.writeFileSync(out, JSON.stringify(f, null, 1) + '\n');

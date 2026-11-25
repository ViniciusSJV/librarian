// Escreve tools/precisao/fixtures/amostra-usos.json: UB-1 a UB-16 (escolhidos) e UB-17 a UB-28 (sorteados, sorteios.json).
// A verdade é PROPOSTA pelo agente até o usuário validar (DEC-22).   node fixture24d.js
'use strict';
const fs = require('fs'), path = require('path');
const S = require('./sorteios.json').UB;
const lines = (f, ns) => ns.map(n => `${f}:${n}`);
const M = 'hc/api/models.py', V = 'hc/api/views.py', SL = 'hc/integrations/slack/transport.py';
const rows = [
  ['hc', `Webhook.is_noop (hc/integrations/webhook/transport.py:63)`, { usos: [], possiveis: lines(M, [1153, 1409]) }, 'self.transport e ch.transport são um Transport de classe escolhida por TRANSPORTS; Webhook é uma delas'],
  ['hc', `Discord.fix_asterisks (hc/integrations/discord/transport.py:25)`, { usos: [], possiveis: [`${SL}:72`] }, 'Discord herda payload de Slackalike, que chama self.fix_asterisks'],
  ['hc', `RocketChat.fix_asterisks (hc/integrations/rocketchat/transport.py:14)`, { usos: ['hc/integrations/rocketchat/transport.py:43'] }, 'self.fix_asterisks no payload da própria classe; sem subclasse'],
  ['hc', `TokenBucket.authorize (${M}:1442)`, { usos: lines(M, [1479, 1493, 1500, 1508, 1513, 1521, 1528, 1536, 1544, 1551, 1559, 1568, 1587]) }, 'staticmethod chamado pelo nome da classe'],
  ['hc', `authorize (hc/api/decorators.py:33)`, { usos: [], so_nome: lines(V, [455, 490, 516, 537, 563, 590, 612, 642]) }, '@authorize sem parênteses não é chamada escrita (2.5)'],
  ['hc', `Telegram.send (hc/integrations/telegram/transport.py:55)`, { usos: ['hc/integrations/telegram/transport.py:87', 'hc/integrations/telegram/transport.py:91', 'hc/integrations/telegram/views.py:82'] }, 'emails.py:25 é EmailMultiAlternatives.send (Django)'],
  ['hc', `Signal.send (hc/integrations/signal/transport.py:60)`, { usos: ['hc/integrations/signal/transport.py:179', 'hc/integrations/signal/views.py:119'] }, 'idem'],
  ['hc', `Check.ping (${M}:493)`, { usos: [`${V}:255`, 'hc/api/management/commands/smtpd.py:91', 'hc/front/tests/test_ping_details.py:130', 'hc/front/tests/test_ping_details.py:133'] }, 'check e self.check são Check'],
  ['hc', `EmailThread.__init__ (hc/lib/emails.py:18)`, { usos: ['hc/lib/emails.py:77', ...lines('hc/lib/tests/test_emails.py', [18, 28, 37])] }, 'EmailThread(..) roda o __init__'],
  ['hc', `Ping.get_body (${M}:772)`, { usos: [] }, 'smtpd.py e front/views.py chamam get_body de email.message.EmailMessage'],
  ['hc', `cors (hc/api/decorators.py:96)`, { usos: lines(V, [480, 488, 506, 550, 561, 588, 610, 640, 689, 697, 707, 745, 799]) }, '@cors(..): decora'],
  ['hc', `Slackalike.payload (${SL}:31)`, { usos: [`${SL}:97`, `${SL}:126`, 'hc/integrations/discord/transport.py:22', 'hc/integrations/mattermost/transport.py:15'] }, 'nenhuma subclasse sobrescreve payload'],
  ['rt', 'impl World::color_at (src/world.rs:109)', { usos: ['src/camera.rs:102', ...lines('src/world.rs', [70, 89, 249, 258, 282, 413])] }, 'patterns.rs:16 é de um Incuse, e World não implementa Incuse'],
  ['rt', 'impl Groups::intersect (src/groups.rs:86)', { usos: ['src/groups.rs:98', 'src/world.rs:40'] }, 'os dois são itens de Vec<Groups>'],
  ['rt', 'impl Groups::len (src/groups.rs:60)', { usos: ['src/groups.rs:115', 'src/obj.rs:203'] }, 'os dois estão em macro (assert_eq!, vec!), que o grafo não lê; os 4 ligados são de Vec e str'],
  ['rt', 'impl Camera::ray_from_pixel (src/camera.rs:81)', { usos: lines('src/camera.rs', [101, 263, 273, 285]) }, 'self e c são Camera'],
];
const names = i => S[i].usos.filter(x => x.startsWith('name ')).map(x => x.split(' ')[1]);
const drawn = [
  [{ usos: ['hc/integrations/rocketchat/transport.py:49'] }, 'ping é Ping'],
  [{ usos: ['hc/accounts/management/commands/sendinactivitynotices.py:83'] }, 'self.pause do próprio comando; cada comando tem o seu'],
  [{ usos: [] }, 'o executor de testes a instancia, fora do código; os 11 só por nome são o tipo do receptor self, não o nome'],
  [{ usos: [] }, 'idem (8)'],
  [{ usos: [], so_nome: names(4) }, 'todo receptor é um Flip (flip, self.flip, up_flip, f, f1, f2, dummy_flip, prev_flip, self em Flip), e os 2 argumentos são de Flip.objects.create'],
  [{ usos: [], so_nome: names(5) }, 'passada a RunPython: referência, não chamada (2.5)'],
  [{ usos: ['src/bin/cena_video.rs:34'] }, 'os outros 10 são Iterator::next de SplitWhitespace e Split'],
  [{ usos: S[7].usos.map(x => x.split(' ')[1]) }, 'Tuple::new pelo caminho; 5 em tuple.rs de produção, 14 em testes'],
  [{ usos: [] }, 'comps.object e containers.last() são Object: roda Object::material, que lê o campo cube.material, não o método'],
  [{ usos: [], so_nome: names(9) }, 'self, s e site são Site; calls em Site::new'],
  [{ usos: [], so_nome: ['src/patterns.rs:79', 'src/patterns.rs:187', 'src/patterns.rs:203', 'src/patterns.rs:203', 'src/patterns.rs:286'] }, 'os outros 15 são color_a de Stripe, Ring, Checkers e DefaultPattern'],
  [{ usos: ['src/object.rs:98', ...lines('src/triangle.rs', [153, 168, 183, 198, 213])], possiveis: ['src/object.rs:32'] }, 'object.rs:32 é o padrão do trait: self pode ser um Triangle'],
];
const simbolos = [
  ...rows.map(([projeto, simbolo, verdade, justificativa], i) => ({ id: `UB-${i + 1}`, estrato: 'escolhido', projeto, simbolo, verdade, justificativa, verdade_validada_por: null })),
  ...drawn.map(([verdade, justificativa], i) => ({ id: `UB-${i + 17}`, estrato: 'sorteado', projeto: S[i].key[0] === 'h' ? 'hc' : 'rt', simbolo: S[i].simbolo, verdade, justificativa, verdade_validada_por: null })),
];
fs.writeFileSync(path.join(__dirname, '../../../../../tools/precisao/fixtures/amostra-usos.json'), JSON.stringify({ nota: 'Amostra "quem usa B" (passo 2.4d). Verdade proposta pelo agente até verdade_validada_por dizer quem validou (DEC-22). usos: rodam B; possiveis: podem rodar B; so_nome: o nome escrito que é de B.', simbolos }, null, 1) + '\n');
console.log(simbolos.length);

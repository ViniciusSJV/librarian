// Passo 2.4c, Parada 2. Acrescenta às fixtures do portão os estratos HC-144 a HC-157 e os níveis do despacho, com o "atual"
// do binário do 2.4b (74faa87), e muda o alvo dos três N5 para ambíguo. Roda uma vez.
//   node fixtures24c.js <dump do Healthchecks, binário do 2.4b> <dump do sintético do portão, idem (dumpsint.js)>
'use strict';
const fs = require('fs'), path = require('path');
const { load } = require('../2.4/load.js');
const FIX = 'C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/fixtures';
const [dumpDir, sintDir] = process.argv.slice(2);
const VAL = 'proposta pelo agente (25/11/2026, Parada 2 do 2.4c); aguarda a validação do usuário na folha';
const P = 'hc/integrations/', T = 'hc/api/transports.py', F = 'hc/front/forms.py';
const hc = [
  ['HC-144', 'despacho', P + 'slack/transport.py:72', /^self\.fix_asterisks\(/, { ambigua: [`Discord.fix_asterisks (${P}discord/transport.py:25)`, `Slackalike.fix_asterisks (${P}slack/transport.py:88)`], observacao: 'despacho polimórfico: Discord sobrescreve fix_asterisks e não sobrescreve payload' }, '`self` em `Slackalike.payload`; `Discord(Slackalike)` sobrescreve `fix_asterisks`, e chama `self.payload`. Com `Slack` e `Mattermost`, roda o da base.'],
  ['HC-145', 'despacho_sem_sobrescrita', T + ':182', /^cls\._request\(/, { alvo: `HttpTransport._request (${T}:138)` }, '26 subclasses de `HttpTransport`, e nenhuma sobrescreve `_request`.'],
  ['HC-146', 'despacho_sem_sobrescrita', P + 'slack/transport.py:97', /^self\.payload\(/, { alvo: `Slackalike.payload (${P}slack/transport.py:31)` }, '3 subclasses de `Slackalike`, e nenhuma sobrescreve `payload`.'],
  ['HC-147', 'despacho_sem_sobrescrita', 'hc/test.py:96', /^self\.assertEmailContainsText\(/, { alvo: 'BaseTestCase.assertEmailContainsText (hc/test.py:79)' }, '187 subclasses de `BaseTestCase`, e nenhuma o sobrescreve.'],
  ['HC-148', 'despacho_sem_sobrescrita', 'hc/test.py:97', /^self\.assertEmailContainsHtml\(/, { alvo: 'BaseTestCase.assertEmailContainsHtml (hc/test.py:84)' }, 'Igual a HC-147.'],
  ['HC-149', 'despacho_irma', P + 'rocketchat/transport.py:43', /^self\.fix_asterisks\(/, { alvo: `RocketChat.fix_asterisks (${P}rocketchat/transport.py:14)` }, '`RocketChat(HttpTransport)` não tem subclasses; as outras `fix_asterisks` são de classes que não herdam dela.'],
  ['HC-150', 'despacho_irma', P + 'msteamsw/transport.py:82', /^self\.fix_asterisks\(/, { alvo: `MsTeamsWorkflow.fix_asterisks (${P}msteamsw/transport.py:14)` }, '`MsTeamsWorkflow(HttpTransport)` não tem subclasses.'],
  ['HC-151', 'despacho_irma', 'hc/front/tests/test_add_check.py:29', /^self\._payload\(/, { alvo: 'AddCheckTestCase._payload (hc/front/tests/test_add_check.py:14)' }, 'A classe de teste não tem subclasses; outra classe de teste declara `_payload`.'],
  ['HC-152', 'despacho_irma', 'hc/accounts/tests/test_notifications.py:25', /^self\._payload\(/, { alvo: 'NotificationsTestCase._payload (hc/accounts/tests/test_notifications.py:14)' }, 'Igual a HC-151.'],
  ['HC-153', 'despacho_construtor', P + 'call/views.py:26', /^form\.get_json\(/, { alvo: `PhoneNumberForm.get_json (${F}:151)` }, '`form = forms.PhoneNumberForm(..)` nas duas atribuições: o objeto é exatamente dela, embora `PhoneUpDownForm` sobrescreva `get_json`.'],
  ['HC-154', 'despacho_construtor', P + 'sms/views.py:24', /^form\.get_json\(/, { alvo: `PhoneUpDownForm.get_json (${F}:168)` }, '`form = forms.PhoneUpDownForm(..)`.'],
  ['HC-155', 'despacho_construtor', 'hc/api/tests/test_smtpd.py:228', /^TestServer\(\)\.process_rcpt\(/, { alvo: 'Server.process_rcpt (hc/api/management/commands/smtpd.py:109)' }, '`TestServer()` é exato, e `TestServer(Server)` não declara `process_rcpt`.'],
  ['HC-156', 'despacho_classe', P + 'telegram/views.py:82', /^Telegram\.send\(/, { alvo: `Telegram.send (${P}telegram/transport.py:55)` }, 'A chamada pelo nome da classe roda o método dela.'],
  ['HC-157', 'despacho_classe', 'hc/api/tests/test_tokenbucket.py:47', /^TokenBucket\.authorize_login_email\(/, { alvo: 'TokenBucket.authorize_login_email (hc/api/models.py:1481)' }, 'A chamada pelo nome da classe roda o método dela.'],
];
{
  const G = load(dumpDir, 'healthchecks-25597e41');
  const f = path.join(FIX, 'amostra-hc.json'), a = JSON.parse(fs.readFileSync(f, 'utf8'));
  for (const [id, estrato, local, re, verdade, justificativa] of hc) {
    if (a.chamadas.some(c => c.id === id)) throw new Error(`${id} já existe`);
    const i = local.lastIndexOf(':'), file = local.slice(0, i), line = Number(local.slice(i + 1));
    const found = [...G.calls.values()].filter(c => c.path === file && c.line === line && re.test(c.text) && c.kind === 'method_call_observed');
    if (found.length !== 1) throw new Error(`${id}: ${found.length} ocorrências`);
    const c = found[0];
    a.chamadas.push({ id, parte_c: false, estrato, ocorrencia: `${c.path}@${c.sb}-${c.eb}`, local, texto: c.text, dono: G.N[c.owner].label, linha_de_base: { resolution: c.res, candidates: c.cands ?? 0, alvos: c.targets.map(G.show).sort() }, verdade, certeza: 'alta', justificativa, verdade_validada_por: VAL });
  }
  a.selecao += '; HC-144 a HC-157: estratos do passo 2.4c (despacho, despacho_sem_sobrescrita, despacho_irma, despacho_construtor, despacho_classe), escolhidos à mão na Parada 1 do 2.4c';
  fs.writeFileSync(f, JSON.stringify(a, null, 1) + '\n');
}
{
  const G = load(sintDir, 'sint'), f = path.join(FIX, 'niveis.json');
  let s = fs.readFileSync(f, 'utf8');
  // Os três N5 cuja verdade era o tipo estático.
  for (const [id, por] of [['N5-py-sobrescrita', 'ambígua: Raiz.passo e HFilho.passo (2.4c; até o 2.4b, o alvo era o do tipo estático)'], ['N5-py-prop', 'ambígua: Transporte.enviar e Rapido.enviar, e roda o do objeto (2.4c; até o 2.4b, o alvo era o do tipo anotado)'], ['N5-py-prop-herdada', 'ambígua: Transporte.enviar e Rapido.enviar (2.4c); a @property é da base']]) {
    const L = s.split('\n'), i = L.findIndex(l => l.includes(`"id":"${id}"`));
    if (i < 0) throw new Error(id);
    const tail = L[i].endsWith(',') ? ',' : '', o = JSON.parse(L[i].trim().replace(/,$/, ''));
    o.alvo = { alvo: 'sem tipo', resolution: 'homonym', passo: '2.4c', por_que: por };
    L[i] = '  ' + JSON.stringify(o) + tail;
    s = L.join('\n');
  }
  const at = label => { const id = Object.keys(G.N).find(i => G.N[i].category === 'symbol' && G.N[i].label === label && G.declared[i]); return G.show(id); };
  const AMB = 'ambígua: ';
  const rows = [
    ['DP-py-parametro', 'd_parametro', 'x.andar(', null, AMB + 'Veiculo.andar e as três sobrescritas'],
    ['DP-py-self', 'Veiculo.rodar', 'self.andar(', null, AMB + 'self; Veiculo.andar e as três sobrescritas'],
    ['DP-py-cls', 'Veiculo.montar', 'cls.nome(', null, AMB + 'cls; Veiculo.nome e Carro.nome'],
    ['DP-py-retorno', 'd_retorno', 'x.andar(', null, AMB + 'o retorno anotado não é exato'],
    ['DP-py-campo', 'Garagem.do_campo', 'self.veiculo.andar(', null, AMB + 'o campo não é construtor exato (conservador)'],
    ['DP-py-prop', 'Garagem.da_propriedade', 'self.atual.andar(', null, AMB + '@property de retorno anotado'],
    ['DP-py-anotado', 'd_construtor_anotado', 'x.andar(', null, AMB + 'a anotação vence o construtor (conservador)'],
    ['DP-py-abstrato', 'd_abstrato', 'x.area(', null, AMB + 'Quadrado.area e Solida.area; os abstratos saem'],
    ['DP-py-abstrato-meio', 'd_abstrato_meio', 'x.area(', null, AMB + 'Quadrado.area e Solida.area'],
    ['DP-py-mistura', 'd_mistura', 'x.abrir(', null, AMB + 'a subclasse roda o da mistura, que vem antes da base'],
    ['DP-py-mistura-self', 'Ficha.usar', 'self.abrir(', null, AMB + 'Ficha.abrir e MisturaFicha.abrir'],
    ['DP-py-new', 'd_novo', 'x.agir(', null, AMB + '__new__ devolve a filha: o construtor não é exato'],
    ['DP-py-atributo', 'd_atributo', 'x.trancar(', null, AMB + 'a subclasse liga o nome a uma função solta'],
    ['DP-py-privado', 'Veiculo.ligar', 'self.__motor(', 'Veiculo.__motor', 'nome privado: a sobrescrita não é alcançada', 'receiver'],
    ['DP-py-meio', 'd_meio', 'x.andar(', 'Carro.andar', 'Jipe não o sobrescreve; as irmãs não contam', 'type'],
    ['DP-py-irma', 'd_irma', 'x.andar(', 'Moto.andar', 'as irmãs não contam', 'type'],
    ['DP-py-sem-sobrescrita', 'd_sem_sobrescrita', 'x.parar(', 'Veiculo.parar', 'nenhuma subclasse o sobrescreve', 'unique'],
    ['DP-py-construtor', 'd_construtor', 'x.andar(', 'Veiculo.andar', 'construtor exato', 'type'],
    ['DP-py-construtor-direto', 'd_construtor_direto', 'Veiculo().andar(', 'Veiculo.andar', 'construtor exato', 'type'],
    ['DP-py-super', 'Carro.pelo_super', 'super().andar(', 'Veiculo.andar', 'super()', 'type'],
    ['DP-py-classe', 'Veiculo.pelo_nome', 'Veiculo.andar(', null, 'T.m() pelo nome da classe: a regra não lhe dá tipo, e vale o que valia'],
    ['DP-py-fora-antes', 'd_fora_antes', 'x.girar(', 'Motor.girar', 'base de fora antes, na subclasse: não conta (limite)', 'unique'],
  ];
  const lines = rows.map(([id, dono, texto, alvo, por, res]) => {
    const c = [...G.calls.values()].find(c => G.N[c.owner].label === dono && c.text.startsWith(texto));
    if (!c) throw new Error(id);
    return '  ' + JSON.stringify({ id, projeto: 'py', dono, linha: c.line, texto, atual: { resolution: c.res, candidates: c.cands ?? 0, alvos: c.targets.map(G.show).sort() }, alvo: alvo ? { alvo: at(alvo), resolution: res, passo: '2.4c', por_que: por } : { alvo: 'sem tipo', resolution: 'homonym', passo: '2.4c', por_que: por } });
  });
  const end = s.lastIndexOf('\n ]');
  if (end < 0) throw new Error('niveis');
  s = s.slice(0, end) + ',\n' + lines.join(',\n') + s.slice(end);
  JSON.parse(s);
  fs.writeFileSync(f, s);
}
console.log('ok');

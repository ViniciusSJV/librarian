// Passo 2.4b, Parada 2. Acrescenta às fixtures do portão os estratos HC-124 a HC-143, a verdade do F23 (V2,
// validada pelo usuário em 24/11/2026) e os níveis da herança, com o "atual" do binário do 2.4a (c3e4b30).
// Roda uma vez.   node fixtures24b.js <dump do Healthchecks, binário do 2.4a> <dump do sintético do portão, idem>
'use strict';
const fs = require('fs'), path = require('path');
const { load } = require('../2.4/load.js');
const FIX = 'C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/fixtures';
const [dumpDir, sintDir] = process.argv.slice(2);
const VAL = 'proposta pelo agente (24/11/2026, Parada 2 do 2.4b); aguarda a validação do usuário na folha';
const overs = require('./py24b.json').rows.find(r => r.over).over.slice().sort();
if (overs.length !== 30) throw new Error('sobrescritas');
const F = { ambigua: overs, observacao: 'Transport.notify é abstrato (raise NotImplementedError); roda o da subclasse escolhida por Channel.transport (DEC-43)' };
const P = 'hc/integrations/', T = 'hc/api/transports.py';
const hc = [
  ['HC-124', 'heranca_self', P + 'slack/transport.py:97', /^self\.post\(/, { alvo: `HttpTransport.post (${T}:199)` }, '`Slackalike(HttpTransport)` não declara `post`; a cadeia é Slackalike, HttpTransport, Transport.'],
  ['HC-125', 'heranca_self', P + 'mattermost/transport.py:15', /^self\.payload\(/, { alvo: `Slackalike.payload (${P}slack/transport.py:31)` }, '`Mattermost(Slackalike)` não declara `payload`.'],
  ['HC-126', 'heranca_self', P + 'webhook/transport.py:96', /^self\.request\(/, { alvo: `HttpTransport.request (${T}:166)` }, '`Webhook(HttpTransport)` não declara `request`.'],
  ['HC-127', 'heranca_self', 'hc/accounts/tests/test_close_account.py:52', /^self\.set_sudo_flag\(/, { alvo: 'BaseTestCase.set_sudo_flag (hc/test.py:74)' }, 'A classe de teste herda de `BaseTestCase`, que declara `set_sudo_flag`, antes de `TestCase`.'],
  ['HC-128', 'heranca_tipo', 'hc/api/tests/test_smtpd.py:216', /^TestServer\(\)\.process_message\(/, { alvo: 'Server.process_message (hc/api/management/commands/smtpd.py:117)' }, '`TestServer(Server)` não declara `process_message`.'],
  ['HC-129', 'heranca_super', P + 'telegram/transport.py:15', /^super\(\)\.__init__\(/, { alvo: `TransportError.__init__ (${T}:57)` }, '`MigrationRequiredError(TransportError)`; `TransportError` declara `__init__`.'],
  ['HC-130', 'heranca_super', P + 'signal/transport.py:22', /^super\(\)\.__init__\(/, { alvo: `TransportError.__init__ (${T}:57)` }, '`SignalRateLimitFailure(TransportError)`.'],
  ['HC-131', 'heranca_super', P + 'pd/tests/test_add.py:14', /^super\(\)\.setUp\(/, { alvo: 'BaseTestCase.setUp (hc/test.py:25)' }, 'A classe de teste herda de `BaseTestCase`, que declara `setUp`.'],
  ['HC-132', 'heranca_super', 'hc/accounts/tests/test_add_webauthn.py:14', /^super\(\)\.setUp\(/, { alvo: 'BaseTestCase.setUp (hc/test.py:25)' }, 'Igual a HC-131.'],
  ['HC-133', 'fora_da_cadeia', P + 'webhook/forms.py:70', /^super\(\)\.clean\(/, { externo: 'Django: Form.clean' }, '`WebhookForm(forms.Form)`: só há base de fora depois da própria classe.'],
  ['HC-134', 'fora_da_cadeia', 'hc/test.py:26', /^super\(\)\.setUp\(/, { externo: 'Django: TestCase.setUp' }, '`BaseTestCase(TestCase)`: só há base de fora depois da própria classe.'],
  ['HC-135', 'fora_da_cadeia', 'hc/lib/signing.py:29', /^super\(\)\.sign\(/, { externo: 'Django: Signer.sign' }, '`super()` nunca alcança a própria classe; a base é do Django.'],
  ['HC-136', 'fora_da_cadeia', P + 'webhook/forms.py:49', /^super\(\)\.validate\(/, { externo: 'Django: forms.Field.validate' }, '`super()` nunca alcança a própria classe; a base é do Django.'],
  ['HC-137', 'fora_da_cadeia', 'hc/api/models.py:1049', /^self\.get_kind_display\(/, { externo: 'Django: o get_kind_display gerado para o campo Channel.kind' }, '`Ping.get_kind_display` não está na cadeia de `Channel` (só `models.Model`); o Django gera o método do campo com `choices`.'],
  ['HC-138', 'cls', T + ':211', /^cls\.request\(/, { alvo: `HttpTransport.request (${T}:166)` }, '`cls` é o primeiro parâmetro do `@classmethod` `post`, de `HttpTransport`.'],
  ['HC-139', 'cls', P + 'telegram/transport.py:65', /^cls\.post\(/, { alvo: `HttpTransport.post (${T}:199)` }, '`cls` no `@classmethod` `Telegram.send`; `Telegram(HttpTransport)` não declara `post`.'],
  ['HC-140', 'cls', T + ':162', /^cls\.raise_for_response\(/, { alvo: `HttpTransport.raise_for_response (${T}:133)`, observacao: 'despacho dinâmico: 11 subclasses o sobrescrevem; o corpo não é abstrato, vale o tipo estático' }, '`cls` no `@classmethod` `_request`, de `HttpTransport`.'],
  ['HC-141', 'propriedade', 'hc/api/models.py:1173', /^self\.transport\.notify\(/, F, '`Channel.transport -> transports.Transport`; `Transport.notify` só levanta NotImplementedError, e 30 subclasses o sobrescrevem (a verdade do F23).'],
  ['HC-142', 'propriedade', 'hc/api/models.py:1153', /^self\.transport\.is_noop\(/, { alvo: `Transport.is_noop (${T}:76)`, observacao: 'despacho dinâmico: 13 subclasses o sobrescrevem; o corpo não é abstrato, vale o tipo estático' }, '`Channel.transport -> transports.Transport`.'],
  ['HC-143', 'propriedade', P + 'zulip/transport.py:48', /^self\.channel\.zulip\.formatted_to\(/, { alvo: 'ZulipConf.formatted_to (hc/api/models.py:958)' }, '`Channel.zulip -> ZulipConf`.'],
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
    a.chamadas.push({ id, parte_c: false, estrato, ocorrencia: `${c.path}@${c.sb}-${c.eb}`, local, texto: c.text, dono: G.N[c.owner].label, linha_de_base: { resolution: c.res, candidates: c.cands ?? 0, alvos: c.targets.map(G.show).sort() }, verdade, certeza: 'alta', justificativa,
      verdade_validada_por: id === 'HC-141' ? 'usuário (24/11/2026, V2 da Parada 1 do 2.4b)' : VAL });
  }
  a.selecao += '; HC-124 a HC-143: estratos do passo 2.4b (heranca_self, heranca_tipo, heranca_super, fora_da_cadeia, cls, propriedade), escolhidos à mão na Parada 1 do 2.4b';
  fs.writeFileSync(f, JSON.stringify(a, null, 1) + '\n');
}
{ // F23
  const f = path.join(FIX, 'fatos.json'), s = fs.readFileSync(f, 'utf8');
  const old = '"verdade": { "alvo": "Transport.notify (hc/api/transports.py:66)", "observacao": "despacho dinâmico: a subclasse de Transport escolhida por Channel.transport" }';
  if (!s.includes(old)) throw new Error('F23');
  fs.writeFileSync(f, s.replace(old, () => '"verdade": ' + JSON.stringify(F)));
}
{ // níveis
  const G = load(sintDir, 'sint'), f = path.join(FIX, 'niveis.json');
  let s = fs.readFileSync(f, 'utf8');
  const H = 'pkg/heranca.py', A = 'pkg/heranca_apoio.py', R = 'pkg/heranca_regras.py';
  const at = (label) => { const id = Object.keys(G.N).find(i => G.N[i].category === 'symbol' && G.N[i].label === label && G.declared[i]); return G.show(id); };
  const rows = [
    ['N5-py-cadeia', 'c_cadeia', 'x.topo(', 'Raiz.topo', 'duas bases acima, em outro arquivo'],
    ['N5-py-generica', 'c_generica', 'x.topo(', 'Raiz.topo', 'base genérica, Caixa[int]'],
    ['N5-py-apelido', 'c_apelido', 'x.topo(', 'Raiz.topo', 'a base é um alias (as)'],
    ['N5-py-construtor', 'c_construtor', 'n.topo(', 'Raiz.topo', 'n = HNeto()'],
    ['N5-py-fora-depois', 'c_fora_depois', 'x.topo(', 'Raiz.topo', 'a base do projeto vem antes da de fora'],
    ['N5-py-fora-antes', 'c_fora_antes', 'x.topo(', null, 'a base de fora vem antes: não decide; só sai o candidato fora da cadeia'],
    ['N5-py-diamante', 'c_diamante', 'x.mm(', 'DC.mm', 'C3; em profundidade seria DA.mm'],
    ['N5-py-sobrescrita', 'c_sobrescrita', 'x.passo(', 'Raiz.passo', 'o tipo estático decide'],
    ['N5-py-atributo', 'c_atributo', 'x.escondido(', null, 'a classe liga o nome a uma função solta: não decide'],
    ['N5-py-instancia', 'c_instancia', 'x.escondido(', null, 'a instância liga o nome: não decide'],
    ['N5-py-getattr', 'c_getattr', 'x.topo(', null, '__getattr__: não decide'],
    ['N5-py-condicional', 'c_condicional', 'x.topo(', null, 'a base é ligada em if/else: não decide'],
    ['N5-py-derruba', 'c_derruba', 'x.inexistente(', null, 'a cadeia é toda do projeto e ninguém declara: sem candidato'],
    ['N5-py-so-fora', 'c_so_fora', 'x.topo(', null, 'só base de fora: nenhum candidato está na cadeia'],
    ['N5-py-super', 'HFilho.passo', 'super().passo(', 'Raiz.passo', 'super()'],
    ['N5-py-super-args', 'HFilho.passo_velho', 'super(HFilho, self).passo(', 'Raiz.passo', 'super(C, self)'],
    ['N5-py-super-coop', 'CoopA.mm', 'super().mm(', null, 'em CoopD continua em CoopB: não decide'],
    ['N5-py-super-mixin', 'MistSuper.mm', 'super().mm(', null, 'mixin sem base: não derruba'],
    ['N5-py-mixin', 'Mistura.ir', 'self.vizinho(', null, 'em Junta é Vizinha.vizinho: não derruba (até o 2.4a, derrubava)'],
    ['N5-py-cls', 'HFilho.fabrica_cls', 'cls.criar(', 'Raiz.criar', 'cls num classmethod'],
    ['N5-py-prop', 'c_prop', 'x.transporte.enviar(', 'Transporte.enviar', '@property com retorno anotado'],
    ['N5-py-prop-herdada', 'c_prop_herdada', 'x.transporte.enviar(', 'Transporte.enviar', 'a @property é da base'],
    ['N5-py-prop-sem-tipo', 'c_prop_sem_tipo', 'x.sem_tipo.enviar(', null, '@property sem anotação: não decide'],
    ['N5-py-abstrato', 'r_abstrato', 'x.mandar(', null, 'abstrato: homonym com as sobrescritas EnvioA e EnvioC'],
    ['N5-py-abstrato-meio', 'r_abstrato_meio', 'x.mandar(', 'EnvioA.mandar', 'o tipo do meio é concreto'],
    ['N5-py-abstrato-construtor', 'r_construtor', 'e.mandar(', 'Envio.mandar', 'construtor exato: roda o corpo abstrato'],
    ['N5-py-abstrato-super', 'EnvioSuper.vazio', 'super().vazio(', 'Envio.vazio', 'super(): roda o corpo abstrato'],
    ['N5-py-abc', 'r_abc', 'x.mandar(', null, '@abstractmethod: homonym com a sobrescrita'],
    ['N5-py-base-dinamica', 'r_base_dinamica', 'x.mm(', null, 'a base é uma expressão: não decide'],
    ['N5-py-meta', 'r_meta', 'x.mm(', null, 'metaclass: não decide'],
    ['N5-py-decorador', 'r_decorador', 'x.mm(', null, 'decorador de classe: não decide'],
  ];
  const lines = rows.map(([id, dono, texto, alvo, por]) => {
    const c = [...G.calls.values()].find(c => G.N[c.owner].label === dono && c.text.startsWith(texto));
    if (!c) throw new Error(id);
    const alvos = c.targets.map(G.show).sort();
    const cands = c.cands ?? 0;
    return '  ' + JSON.stringify({ id, projeto: 'py', dono, linha: c.line, texto, atual: { resolution: c.res, candidates: cands, alvos }, alvo: alvo ? { alvo: at(alvo), resolution: 'type', passo: '2.4b', por_que: por } : { alvo: 'sem tipo', resolution: /sem candidato|nenhum candidato/.test(por) ? 'no_candidate_in_project' : /^super()|CoopD|mixin sem base|expressão|^metaclass|decorador de classe/.test(por) ? 'above_candidate_limit' : 'homonym', passo: '2.4b', por_que: por } });
  });
  const end = s.lastIndexOf('\n ]');
  if (end < 0) throw new Error('niveis');
  s = s.slice(0, end) + ',\n' + lines.join(',\n') + s.slice(end);
  JSON.parse(s);
  fs.writeFileSync(f, s);
}
console.log('ok');

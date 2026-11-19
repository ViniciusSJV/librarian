// Adds the 2.2 strata to the sample fixtures (truth proposed by the agent, DEC-22).
'use strict';
const fs = require('fs');
const REPO = 'C:/Users/beatl/Documents/Projetos/librarian';
const sim = require('./simR2.json');
const P = 'proposta pelo agente (passo 2.2, 19/11)';
const CASES = {
  hc: [
    ['HC-61', 'metodo_funcao', 'hc/lib/signing.py@1125-1136', { externo: 'stdlib: time.time' }, 'import time (signing.py:3): o receptor é o módulo time da biblioteca padrão. O link atual vai para o método ChannelsAdmin.time (hc/api/admin.py:224).'],
    ['HC-62', 'metodo_funcao', 'hc/api/management/commands/sendalerts.py@1953-1968', { externo: 'builtin: str.join' }, 'O receptor é o literal "\\n" (str). O link atual vai para a função livre join de hc/integrations/matrix/client.py.'],
    ['HC-63', 'metodo_funcao', 'hc/lib/curl.py@953-974', { externo: 'builtin: bytes.decode' }, 'self.content é bytes (Response.__init__, content: bytes, curl.py:33-35). O link atual vai para a função livre decode de hc_extras.py.'],
    ['HC-64', 'metodo_funcao', 'hc/api/tests/test_check_model.py@10805-10816', { externo: 'Django: Model.save' }, 'flip = Flip(owner=check) (test_check_model.py:278); Flip é um model do Django e não sobrescreve save. Código de teste.'],
    ['HC-65', 'metodo_funcao', 'hc/api/tests/test_tokenbucket.py@1791-1801', { externo: 'Django: Model.save' }, 'obj = TokenBucket(...) (test_tokenbucket.py:52); TokenBucket é um model do Django e não sobrescreve save. Código de teste.'],
    ['HC-66', 'metodo_funcao', 'hc/integrations/rocketchat/tests/test_add.py@1290-1356', { externo: 'Django: django.test.Client.login' }, 'self.client é o cliente de teste do Django (BaseTestCase herda de TestCase). Os 3 candidatos atuais (ProfileAdmin.login e as funções login de accounts/views.py e lib/emails.py) estão errados. Código de teste.'],
    ['HC-67', 'modulo_f', 'hc/accounts/models.py@11408-11452', { alvo: 'report (hc/lib/emails.py:112)' }, 'from hc.lib import emails (accounts/models.py:24): emails.report é a função do módulo.'],
    ['HC-68', 'modulo_f', 'hc/integrations/matrix/forms.py@439-453', { alvo: 'join (hc/integrations/matrix/client.py:26)' }, 'from hc.integrations.matrix import client (matrix/forms.py:5): client.join é a função do módulo.'],
    ['HC-69', 'modulo_f', 'hc/api/transports.py@4690-4944', { alvo: 'request (hc/lib/curl.py:55)' }, 'from hc.lib import curl (transports.py:10): curl.request é a função do módulo, não o método HttpTransport.request.'],
    ['HC-70', 'modulo_f', 'hc/accounts/models.py@7089-7134', { alvo: 'transfer_request (hc/lib/emails.py:92)' }, 'from hc.lib import emails (accounts/models.py:24): emails.transfer_request é a função do módulo, não o método Project.transfer_request.'],
    ['HC-71', 'modulo_f', 'hc/integrations/github/client.py@3166-3196', { alvo: 'get (hc/lib/curl.py:210)' }, 'from hc.lib import curl (github/client.py:8): curl.get é a função do módulo.'],
    ['HC-72', 'espelho', 'hc/lib/emails.py@2957-2961', { alvo: 'send (hc/lib/emails.py:71)' }, 'Chamada simples send(...) em emails.login (emails.py:88-89): a função send do próprio módulo, não os métodos Signal.send e Telegram.send.'],
    ['HC-73', 'espelho', 'hc/integrations/webhook/views.py@1568-1574', { externo: 'Django: django.shortcuts.render' }, 'from django.shortcuts import redirect, render (webhook/views.py:8). Os candidatos atuais são três métodos render (linemode.py, asciitable.py), que uma chamada simples não alcança.'],
    ['HC-74', 'espelho', 'hc/lib/tests/test_curl.py@1657-1664', { alvo: 'request (hc/lib/curl.py:55)' }, 'from hc.lib.curl import CurlError, request (test_curl.py:10). Código de teste.'],
    ['HC-75', 'espelho', 'hc/lib/tests/test_emails.py@1410-1414', { alvo: 'send (hc/lib/emails.py:71)' }, 'from hc.lib.emails import EmailThread, send (test_emails.py:9). Código de teste.'],
  ],
  rt: [
    ['RT-56', 'tipo_derruba', 'src/bin/cena_video.rs@4541-4552', { externo: 'std: Vec::len' }, 'let mut balls: Vec<(f64, f64, f64)> (cena_video.rs:105). O link atual vai para impl Groups::len.'],
    ['RT-57', 'tipo_derruba', 'src/obj.rs@5280-5293', { externo: 'std: Vec::len' }, 'Parâmetro mut objects: Vec<Object> (obj.rs:123). O link atual vai para impl Groups::len.'],
    ['RT-58', 'tipo_derruba', 'src/groups.rs@1526-1545', { externo: 'std: Vec::len' }, 'Campo children: Vec<Object> (groups.rs:12). O link atual é Groups::len chamando a si mesmo.'],
    ['RT-59', 'tipo_derruba', 'src/world.rs@855-874', { externo: 'std: Vec::len' }, 'Campo inverses: Vec<crate::matrix::Matrix<4>> (world.rs:15). O link atual vai para impl Groups::len.'],
    ['RT-60', 'tipo_derruba', 'src/tuple.rs@525-547', { alvo: 'impl Equivalence<f64> for f64::equivalent (src/equivalent.rs:12)' }, 'Controle: w: f64 (tuple.rs:9), e o projeto implementa Equivalence<f64> for f64 (equivalent.rs:11). O tipo de fora do projeto não derruba: continua homonym, com o alvo entre os candidatos.'],
    ['RT-61', 'espelho', 'src/canvas.rs@4159-4163', { externo: 'std: std::mem::drop' }, 'drop(writer) em Canvas::to_png (canvas.rs:118): a função drop do prelúdio. O link atual vai para impl Drop for Guard::drop, um método.'],
    ['RT-62', 'espelho', 'src/obj.rs@2758-2762', { externo: 'closure local next (src/obj.rs:63), sem declaração no projeto' }, 'let mut next = || ... (obj.rs:63): next() chama a closure local. O link atual vai para impl Sorteio::next (src/bin/cena_video.rs:29).'],
  ],
};
const KEYS = { hc: 'healthchecks-25597e41', rt: 'renderer-9b78f3a9' };
for (const [lang, cases] of Object.entries(CASES)) {
  const f = `${REPO}/tools/precisao/fixtures/amostra-${lang}.json`;
  const fx = JSON.parse(fs.readFileSync(f, 'utf8'));
  const rows = sim[KEYS[lang]]; const G = JSON.parse(fs.readFileSync(`antes/${KEYS[lang]}.graph.json`, "utf8"));
  for (const [id, estrato, key, verdade, justificativa] of cases) {
    if (fx.chamadas.some(c => c.id === id || c.ocorrencia === key)) throw new Error('já existe: ' + id);
    const r = rows.find(x => x.key === key);
    if (!r) throw new Error('sem ocorrência: ' + key);
    const before = r.before;
    fx.chamadas.push({ id, parte_c: false, estrato, ocorrencia: key, local: r.at, texto: G.nodes[r.occ].label.split(String.fromCharCode(10))[0].replace(String.fromCharCode(13),''), dono: r.owner,
      linha_de_base: { resolution: before.r, candidates: r.cand, alvos: before.r === 'homonym' || before.t.length ? before.t : [] },
      verdade, certeza: 'alta', justificativa, verdade_validada_por: P });
  }
  fx.selecao += lang === 'hc'
    ? '; HC-61 a HC-75: estratos do passo 2.2 (metodo_funcao, modulo_f, espelho), escolhidos à mão na Parada 1 do 2.2, equilibrando código de teste e de produção'
    : '; RT-56 a RT-62: estratos do passo 2.2 (tipo_derruba, com um caso de controle, e espelho), escolhidos à mão na Parada 1 do 2.2';
  fs.writeFileSync(f, JSON.stringify(fx, null, 1) + '\n');
  console.log(lang, fx.chamadas.length);
}

// Step 2.3b: the sample's new strata (HC-88 to HC-97), from the run of the 2.3a binary
// (q23b-out): occurrence key, owner and the answer before 2.3b. Truth proposed by the agent.
'use strict';
const fs = require('fs');
const FX = 'C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/fixtures/amostra-hc.json';
const g = JSON.parse(fs.readFileSync('q23b-out/graph.json', 'utf8'));
const { res } = JSON.parse(fs.readFileSync('q23b-out/fora-do-corpo.json', 'utf8'));
const declared = {};
for (const e of g.edges) if (e.kind === 'declares') declared[e.to] = e.evidence;
const show = id => `${g.nodes[id].label} (${declared[id].path}:${declared[id].start_line})`;
const occAt = (path, line, text) => g.edges.filter(e => e.kind === 'observes' && e.evidence.path === path && e.evidence.start_line === line
  && /call_observed$/.test(g.nodes[e.to].kind) && g.nodes[e.to].label.startsWith(text.split("(")[0]));
const P = 'proposta pelo agente (Parada 2 do 2.3b, 20/11/2026)';
const casos = [
  ['HC-88', 'decorador', 'hc/integrations/slack/views.py', 24, 'require_setting(', { alvo: 'require_setting (hc/front/decorators.py:13)', relacao: 'decora' }, 'Decorador de topo da view `add`. Roda quando o módulo carrega e devolve o embrulho que só deixa a view responder com SLACK_ENABLED. O alvo é a função do projeto; a relação é "decora" (DEC-40).'],
  ['HC-89', 'decorador', 'hc/api/views.py', 480, 'cors(', { alvo: 'cors (hc/api/decorators.py:96)', relacao: 'decora' }, 'Decorador de topo da view `checks`; roda ao carregar o módulo (DEC-40).'],
  ['HC-90', 'decorador', 'hc/integrations/pd/views.py', 61, 'require_setting(', { alvo: 'require_setting (hc/front/decorators.py:13)', relacao: 'decora' }, 'Segundo decorador empilhado sobre `add_complete` (PD_APP_ID); mesma regra do HC-88.'],
  ['HC-91', 'decorador', 'hc/api/admin.py', 73, 'admin.display(', { externo: 'Django: django.contrib.admin.display' }, 'Decorador de método; `admin` é import de fora do projeto (django.contrib).'],
  ['HC-92', 'decorador', 'hc/integrations/webhook/tests/test_notify.py', 267, 'patch(', { externo: 'unittest.mock.patch' }, 'Decorador de método de teste; `patch` vem de unittest.mock.'],
  ['HC-93', 'decorador', 'hc/integrations/call/tests/test_add.py', 9, 'override_settings(', { externo: 'Django: django.test.override_settings' }, 'Decorador de classe de teste.'],
  ['HC-94', 'campo_init', 'hc/api/tests/test_smtpd.py', 49, 'NullSink', { alvo: 'NullSink (hc/api/tests/test_smtpd.py:42)', executor: 'TestServer.__init__ (hc/api/tests/test_smtpd.py:48)' }, '`self.stdout = NullSink()` no `__init__`: instancia a classe do próprio arquivo (sem `__init__` próprio). Escrita no campo `TestServer.stdout`, roda no `TestServer.__init__`.'],
  ['HC-95', 'campo_init', 'hc/api/management/commands/sendalerts.py', 63, 'ThreadPoolExecutor', { externo: 'concurrent.futures.ThreadPoolExecutor', executor: 'Command.__init__ (hc/api/management/commands/sendalerts.py:61)' }, 'Classe da biblioteca padrão, no valor de `self.executor` do `__init__`.'],
  ['HC-96', 'campo_init', 'hc/lib/webauthn.py', 20, 'Fido2Server', { externo: 'fido2.server.Fido2Server', executor: 'CreateHelper.__init__ (hc/lib/webauthn.py:18)' }, 'Classe do pacote fido2, no valor de `self.server` do `__init__`.'],
  ['HC-97', 'campo_init', 'hc/lib/webauthn.py', 56, 'AttestedCredentialData', { externo: 'fido2.webauthn.AttestedCredentialData', executor: 'GetHelper.__init__ (hc/lib/webauthn.py:53)' }, 'Classe do pacote fido2, dentro de uma comprehension no valor de `self.credentials` do `__init__`.'],
];
const fx = JSON.parse(fs.readFileSync(FX, 'utf8'));
const out = [];
for (const [id, estrato, path, line, text, verdade, why] of casos) {
  const found = occAt(path, line, text);
  if (found.length !== 1) throw new Error(`${id}: ${found.length} ocorrências`);
  const e = found[0], occ = `${path}@${e.evidence.start_byte}-${e.evidence.end_byte}`;
  // The four field calls are in a function body, outside fora-do-corpo: their answer before 2.3b is
  // the one measured in the Parada 1 (casos.json): NullSink unique instantiates, the rest without candidate.
  const r = res.find(x => x.occ === e.to) ?? (id === 'HC-94' ? { resolution: 'unique', relation: 'instantiates', targets: ['class NullSink (hc/api/tests/test_smtpd.py:42)'] } : { resolution: 'no_candidate_in_project', targets: [] });
  const alvos = (r?.targets ?? []).map(t => t.replace(/^\w+ /, ''));
  out.push({ id, parte_c: false, estrato, ocorrencia: occ, local: `${path}:${line}`, texto: g.nodes[e.to].label.split(/\r?\n/)[0], dono: g.nodes[e.from].label,
    linha_de_base: { resolution: r?.resolution ?? '?', candidates: alvos.length, alvos, ...(r?.relation && r.relation !== 'calls' ? { relacao: r.relation } : {}) },
    verdade, certeza: 'alta', justificativa: why, verdade_validada_por: P });
}
if (process.argv[2] === '--gravar') {
  fx.chamadas = fx.chamadas.filter(c => !casos.some(k => k[0] === c.id)).concat(out);
  fs.writeFileSync(FX, JSON.stringify(fx, null, 1) + '\n');
}
console.log(JSON.stringify(out.map(o => [o.id, o.ocorrencia, o.dono, o.linha_de_base]), null, 0));

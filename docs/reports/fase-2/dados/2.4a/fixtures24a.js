// Passo 2.4a, Parada 2. Acrescenta às fixtures do portão os estratos aprovados na Q8 da Parada 1
// (HC-113 a HC-123, RT-68 a RT-70; verdade proposta pelo agente, aguardando a validação do usuário na
// folha) e os níveis do escopo e do alias, com o "atual" do binário do 2.3c (6196976, numa worktree).
// Roda uma vez.   node fixtures24a.js <dump-dir do binário do 2.3c> <antes.jsonl>
'use strict';
const fs = require('fs'), path = require('path');
const { load } = require('../2.4/load.js');
const FIX = 'C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/fixtures';
const [dumpDir, antesFile] = process.argv.slice(2);
const VAL = 'proposta pelo agente (20/11/2026, Parada 2 do 2.4a); aguarda a validação do usuário na folha';
const hc = [
  ['HC-113', 'escopo_modulo', 'hc/settings.py:69', 'envsecret', { alvo: 'envsecret (hc/settings.py:49)' }, '`envsecret` tem duas `@overload` (linhas 41 e 45) e a implementação (49), todas no próprio arquivo; a chamada roda na carga do módulo, depois das três, e vale a última `def`.'],
  ['HC-114', 'escopo_modulo', 'hc/settings.py:344', 'envsecret', { alvo: 'envsecret (hc/settings.py:49)' }, 'Igual a HC-113.'],
  ['HC-115', 'escopo_modulo', 'hc/lib/curl.py:206', 'Response', { alvo: 'Response (hc/lib/curl.py:32)', relacao: 'instancia' }, '`Response` é a classe do próprio arquivo; o outro candidato, `Signal.Response`, é uma classe aninhada em hc/integrations/signal/transport.py, que o nome não alcança daqui.'],
  ['HC-116', 'escopo_fora', 'hc/api/apps.py:38', 'Error', { externo: 'django.core.checks.Error' }, '`from django.core.checks import Error` (hc/api/apps.py:9); `Signal.Error` (hc/integrations/signal/transport.py:40) não é alcançável por esse nome.'],
  ['HC-117', 'escopo_fora', 'hc/api/apps.py:66', 'Error', { externo: 'django.core.checks.Error' }, 'Igual a HC-116.'],
  ['HC-118', 'escopo_fora', 'hc/api/apps.py:121', 'Error', { externo: 'django.core.checks.Error' }, 'Igual a HC-116.'],
  ['HC-119', 'escopo_local', 'hc/accounts/migrations/0018_auto_20190112_1426.py:16', 'Project', { externo: 'modelo histórico de apps.get_model("accounts", "Project")' }, '`Project = apps.get_model("accounts", "Project")` na própria função (linha 13): uma variável, o modelo histórico da migração, não a classe `Project` de hc/accounts/models.py.'],
  ['HC-120', 'escopo_import', 'hc/api/tests/test_sendalerts.py:116', 'notify', { alvo: 'notify (hc/api/management/commands/sendalerts.py:24)' }, '`from hc.api.management.commands.sendalerts import Command, notify`.'],
  ['HC-121', 'escopo_import', 'hc/logs/tests/test_sendlogs.py:25', 'Command', { alvo: 'Command (hc/logs/management/commands/sendlogs.py:14)', relacao: 'instancia' }, '`from hc.logs.management.commands.sendlogs import Command`.'],
  ['HC-122', 'escopo_modulo', 'hc/integrations/signal/tests/test_signal_captcha.py:66', 'setup_mock', { alvo: 'setup_mock (hc/integrations/signal/tests/test_signal_captcha.py:43)' }, '`setup_mock` é a função do próprio arquivo; o homônimo está em hc/integrations/signal/tests/test_notify.py.'],
  ['HC-123', 'escopo_modulo', 'hc/accounts/tests/test_senddeletionscheduled.py:44', 'counts', { alvo: 'counts (hc/accounts/tests/test_senddeletionscheduled.py:19)' }, '`counts` é a função do próprio arquivo; o homônimo está em hc/accounts/tests/test_sendinactivitynotices.py.'],
];
const rt = [
  ['RT-68', 'escopo_rs', 'src/world.rs:192', 'create_default_world', { alvo: 'tests_world::create_default_world (src/world.rs:146)' }, '`create_default_world` é a função do próprio módulo `tests_world`; a outra está no módulo de testes de src/camera.rs.'],
  ['RT-69', 'escopo_rs', 'src/camera.rs:313', 'create_default_world', { alvo: 'tests_camera::create_default_world (src/camera.rs:291)' }, '`create_default_world` é a função do próprio módulo `tests_camera`.'],
  ['RT-70', 'escopo_rs', 'src/camera.rs:359', 'render_benchmark', { alvo: 'render_benchmark (src/camera.rs:143)' }, '`use super::*` no módulo `tests_camera` traz `render_benchmark` do módulo pai; o `unique` de hoje é confirmado.'],
];
function occurrences(key, rows) {
  const P = load(dumpDir, key);
  const out = {};
  for (const [id, , local, texto] of rows) {
    const [file, line] = [local.slice(0, local.lastIndexOf(':')), Number(local.slice(local.lastIndexOf(':') + 1))];
    const found = [...P.calls.values()].filter(c => c.path === file && c.line === line && c.text === texto && c.kind === 'call_observed');
    if (found.length !== 1) throw new Error(`${id}: ${found.length} ocorrências`);
    const c = found[0];
    out[id] = {
      ocorrencia: `${c.path}@${c.sb}-${c.eb}`, local, texto, dono: P.N[c.owner].label,
      linha_de_base: { resolution: c.res, candidates: c.cands, alvos: c.targets.map(P.show).sort() },
    };
  }
  return out;
}
function add(file, key, rows, selecao) {
  const occ = occurrences(key, rows);
  const f = path.join(FIX, file), a = JSON.parse(fs.readFileSync(f, 'utf8'));
  for (const [id, estrato, , , verdade, justificativa] of rows) {
    if (a.chamadas.some(c => c.id === id)) throw new Error(`${id} já existe`);
    const o = occ[id];
    a.chamadas.push({ id, parte_c: false, estrato, ocorrencia: o.ocorrencia, local: o.local, texto: o.texto, dono: o.dono, linha_de_base: o.linha_de_base, verdade, certeza: 'alta', justificativa, verdade_validada_por: VAL });
  }
  a.selecao += selecao;
  fs.writeFileSync(f, JSON.stringify(a, null, 1) + '\n');
}
add('amostra-hc.json', 'healthchecks-25597e41', hc, '; HC-113 a HC-123: estratos do passo 2.4a (escopo_modulo, escopo_fora, escopo_local, escopo_import), escolhidos à mão na Parada 1 do 2.4');
add('amostra-rt.json', 'renderer-9b78f3a9', rt, '; RT-68 a RT-70: estrato escopo_rs do passo 2.4a (as chamadas de produção do ray tracer não mudam pelo escopo)');

// Níveis: "atual" é a resposta do binário do 2.3c (6196976).
const antes = fs.readFileSync(antesFile, 'utf8').trim().split('\n').map(JSON.parse);
const SEM = 'sem tipo';
const niveis = [
  ['N0-py-modulo', 'py', 'e_modulo', 43, 'esc_modulo', 'esc_modulo (pkg/escopo.py:13)', 'scope', 'def esc_modulo no próprio arquivo, e outra em pkg/escopo_apoio.py'],
  ['N0-py-fora', 'py', 'e_fora', 47, 'join', SEM, 'no_candidate_in_project', 'from os.path import join; o join de pkg/escopo_apoio.py não é o chamado'],
  ['N0-py-local', 'py', 'e_local', 52, 'g', SEM, 'no_candidate_in_project', 'g = fabrica(): uma variável, não o g de pkg/escopo_apoio.py'],
  ['N0-py-overload', 'py', 'e_overload', 56, 'esc_sobre', 'esc_sobre (pkg/escopo.py:25)', 'scope', 'duas @overload e a implementação: vale a última def'],
  ['N0-py-condicional', 'py', 'e_cond', 60, 'esc_cond', SEM, 'homonym', 'def esc_cond em if/else, com corpos diferentes: o escopo não decide'],
  ['N0-py-typechecking', 'py', 'e_typechecking', 64, 'so_tipo', 'so_tipo (pkg/escopo_apoio.py:13)', 'unique', 'so_tipo importado só em TYPE_CHECKING: o escopo não decide, e vale o que valia'],
  ['N0-py-estrela', 'py', 'e_estrela', 5, 'esc_modulo', SEM, 'homonym', 'from pkg.escopo_apoio import *: o escopo não decide'],
  ['N0-py-reexport', 'py', 'e_reexport', 68, 'reexp', 'reexp (pkg/sub/base.py:1)', 'import', 'from pkg.sub import reexp, e pkg/sub/__init__.py reexporta o de pkg/sub/base.py'],
  ['N8-py-tipo', 'py', 'n8_tipo', 72, 'a.salvar(', 'Cliente.salvar (pkg/models.py:5)', 'type', 'Alias = Cliente no módulo; a: Alias'],
  ['N8-py-local', 'py', 'n8_local', 78, 'd.salvar(', 'Fornecedor.salvar (pkg/models.py:10)', 'type', 'D = Fornecedor na função; d = D()'],
  ['N8-py-typealias', 'py', 'n8_typealias', 82, 'a.salvar(', 'Fornecedor.salvar (pkg/models.py:10)', 'type', 'TipoF: TypeAlias = Fornecedor; a: TipoF'],
  ['N8-py-fora', 'py', 'n8_fora', 86, 'r.salvar(', SEM, 'homonym', 'R = Dt, e Dt é datetime (de fora): não decide'],
  ['N0-rs-modulo', 'rs', 'e_modulo', 9, 'esc_livre', 'esc_livre (src/escopo.rs:4)', 'scope', 'fn esc_livre no próprio módulo, e outra em src/escopo_apoio.rs'],
  ['N0-rs-bloco', 'rs', 'e_bloco', 16, 'esc_bloco', 'e_bloco::esc_bloco (src/escopo.rs:13)', 'scope', 'fn esc_bloco no corpo da própria função'],
  ['N0-rs-glob', 'rs', 'tests::e_glob', 30, 'esc_livre', 'esc_livre (src/escopo.rs:4)', 'import', 'use super::* num mod tests'],
  ['N0-rs-glob-fora', 'rs', 'de_fora::e_glob_fora', 38, 'esc_livre', SEM, 'homonym', 'use std::collections::*: um glob de fora do crate pode trazer o nome; não decide'],
  ['N8-rs-tipo', 'rs', 'n8_tipo', 21, 'c.salvar(', 'impl Cliente::salvar (src/models.rs:10)', 'type', 'type Cl = Cliente; let c = Cl::new(..)'],
];
const nf = path.join(FIX, 'niveis.json');
let text = fs.readFileSync(nf, 'utf8');
const js = o => JSON.stringify(o).replace(/,"/g, ', "').replace(/":/g, '": ');
const lines = niveis.map(([id, projeto, dono, linha, texto, alvo, resolution, por_que]) => {
  const a = antes.find(x => x.projeto === projeto && x.dono === dono && x.linha === linha && (x.texto === texto || x.texto.startsWith(texto)));
  if (!a) throw new Error(`sem atual: ${id}`);
  const atual = { resolution: a.resolution, candidates: a.candidates, alvos: a.alvos };
  return `  { "id": ${JSON.stringify(id)}, "projeto": "${projeto}", "dono": ${JSON.stringify(dono)}, "linha": ${linha}, "texto": ${JSON.stringify(texto)},\n    "atual": ${js(atual)},\n    "alvo": ${js({ alvo, resolution, passo: '2.4a', por_que: por_que + '; acrescentado no 2.4a, com o binário do 2.3c como atual' })} }`;
});
const end = text.lastIndexOf('\n ]\n}');
if (end < 0) throw new Error('fim de niveis.json');
text = text.slice(0, end) + ',\n' + lines.join(',\n') + text.slice(end);
// N3c-py-dois-alvos: `duplo` também está no próprio pkg/retorno.py; pelo escopo, vale a dele (-> Pedido).
const before = '"alvo": {"alvo": "sem tipo", "resolution": "homonym", "passo": "2.3c", "por_que": "duplo() em dois arquivos, -> Pedido e -> Nota: os candidatos discordam (caso d); acrescentado no 2.3c, com o binário do 2.3b como atual"}';
if (!text.includes(before)) throw new Error('N3c-py-dois-alvos não achado');
text = text.replace(before, '"alvo": {"alvo": "Pedido.fechar (pkg/retorno.py:7)", "resolution": "type", "passo": "2.4", "por_que": "duplo() em dois arquivos, -> Pedido e -> Nota (caso d da DEC-41: sem tipo no 2.3c); dois_alvos está em pkg/retorno.py, que declara o duplo -> Pedido, e o Python chama o do próprio módulo (o N0-livre, como o N3c-rs-discordam); acrescentado no 2.3c, com o binário do 2.3b como atual; alvo mudado no 2.4a"}');
JSON.parse(text);
fs.writeFileSync(nf, text);
console.log('ok', hc.length + rt.length, 'chamadas', niveis.length, 'níveis');

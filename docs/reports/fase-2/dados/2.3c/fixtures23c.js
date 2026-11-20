// Acrescenta às fixtures do portão os estratos do 2.3c (Q5 da Parada 1, validada em 20/11/2026) e os
// níveis N3c. Roda uma vez.   node fixtures23c.js <ocorrencias.json> <antes.jsonl>
'use strict';
const fs = require('fs'), path = require('path');
const FIX = 'C:/Users/beatl/Documents/Projetos/librarian/tools/precisao/fixtures';
const [occFile, antesFile] = process.argv.slice(2);
const occ = JSON.parse(fs.readFileSync(occFile, 'utf8'));
const VAL = 'usuário (20/11/2026; Q5 da Parada 1 do 2.3c)';
const PJ = 'Project.num_checks_available (hc/accounts/models.py:471)';
const hc = [
  ['HC-98', 'retorno_py', { alvo: 'Response.json (hc/lib/curl.py:37)' }, '`result = curl.post(..)`, e `curl.post` (hc/lib/curl.py:224) retorna `Response`, declarada no outro arquivo.'],
  ['HC-99', 'retorno_py', { alvo: 'Response.json (hc/lib/curl.py:37)' }, 'Igual a HC-98, na integração do Discord.'],
  ['HC-100', 'retorno_py', { alvo: PJ }, '`project = _get_rw_project_for_user(..)`, do mesmo arquivo (hc/front/views.py:187), retorna `Project`.'],
  ['HC-101', 'retorno_py', { alvo: PJ }, '`target_project = _get_rw_project_for_user(..)`, do mesmo arquivo, retorna `Project`.'],
  ['HC-102', 'campo_outro_arquivo', { alvo: 'Profile.projects (hc/accounts/models.py:203)' }, '`request: AuthenticatedHttpRequest`, cujo campo `profile: Profile` está em hc/accounts/http.py.'],
  ['HC-103', 'campo_outro_arquivo', { alvo: 'Profile.projects (hc/accounts/models.py:203)' }, 'Igual a HC-102, na integração do Telegram.'],
  ['HC-104', 'campo_outro_arquivo', { alvo: PJ }, '`request: ApiRequest`, cujo campo `project: Project` está em hc/api/decorators.py.'],
  ['HC-105', 'retorno_confirma', { alvo: 'Check.going_down_after (hc/api/models.py:339)' }, '`check = _get_rw_check_for_user(..)` retorna `Check`; o `unique` de hoje é confirmado.'],
  ['HC-106', 'retorno_confirma', { alvo: 'Ping.formatted_kind_created (hc/api/models.py:828)' }, '`ping = self.last_ping(flip)` retorna `Ping | None`; o `unique` de hoje é confirmado.'],
  ['HC-107', 'retorno_confirma', { alvo: 'Profile.send_instant_login_link (hc/accounts/models.py:144)' }, '`profile = Profile.objects.for_user(..)`, e `ProfileManager.for_user` retorna `Profile`; o `unique` de hoje é confirmado.'],
  ['HC-108', 'liga_nao_fecha', { alvo: 'Check.ping (hc/api/models.py:493)' }, '`check = Check.objects.get(..)`: o `get` é do gerenciador do Django, e os candidatos do projeto são métodos de teste; nenhum tipo é dado.'],
  ['HC-109', 'liga_nao_fecha', { alvo: 'Profile.check_token (hc/accounts/models.py:136)' }, '`profile = profiles.get(..)`, um QuerySet do Django; nenhum tipo é dado.'],
  ['HC-110', 'liga_nao_fecha', { alvo: 'Member.can_accept (hc/accounts/models.py:625)' }, '`tr = project.transfer_request()`, `homonym` com um só candidato, `Project.transfer_request -> Member | None`: pelo caso (b) da Q1, o tipo é `Member`.'],
  ['HC-111', 'cadeia', { externo: 'dict.get (doc é JSONValue)' }, '`doc = result.json()`: `result` só tem tipo pelo retorno, e um segundo nível não é seguido; `Response.json` retorna `JSONValue`.'],
  ['HC-112', 'cadeia', { externo: 'dict.get (doc é JSONValue)' }, 'Igual a HC-111, na integração do Discord.'],
];
const rt = [
  ['RT-66', 'retorno_rs', { alvo: 'impl Bounds::add_point (src/bounds.rs:23)' }, '`let mut result = Self::empty()`, e `Bounds::empty` retorna `Self`.'],
  ['RT-67', 'retorno_rs', { alvo: 'impl World::color_at (src/world.rs:109)' }, '`let w = create_default_world()`: dois candidatos (src/world.rs e o módulo de testes de src/camera.rs), ambos retornam `World` (caso (b) da Q1).'],
];
function add(file, rows, selecao) {
  const f = path.join(FIX, file), a = JSON.parse(fs.readFileSync(f, 'utf8'));
  for (const [id, estrato, verdade, justificativa] of rows) {
    if (a.chamadas.some(c => c.id === id)) throw new Error(`${id} já existe`);
    const o = occ[id];
    if (!o) throw new Error(`${id} sem ocorrência`);
    a.chamadas.push({ id, parte_c: false, estrato, ocorrencia: o.ocorrencia, local: o.local, texto: o.texto, dono: o.dono, linha_de_base: o.linha_de_base, verdade, certeza: 'alta', justificativa, verdade_validada_por: VAL });
  }
  a.selecao += selecao;
  fs.writeFileSync(f, JSON.stringify(a, null, 1) + '\n');
}
add('amostra-hc.json', hc, '; HC-88 a HC-97: estratos do passo 2.3b (decorador, campo_init), escolhidos à mão na Parada 1 do 2.3b; HC-98 a HC-112: estratos do passo 2.3c (retorno_py, campo_outro_arquivo, retorno_confirma, liga_nao_fecha, cadeia), escolhidos à mão na Parada 1 do 2.3c');
add('amostra-rt.json', rt, '; RT-66 e RT-67: estrato retorno_rs do passo 2.3c, de conferência (nenhuma chamada do ray tracer muda pelo retorno)');

// Níveis N3c: "atual" é a resposta do binário do 2.3b (f50b8f8).
const antes = new Map(fs.readFileSync(antesFile, 'utf8').trim().split('\n').map(JSON.parse).map(x => [`${x.projeto}|${x.dono}|${x.linha}|${x.texto}`, x]));
const SEM = 'sem tipo';
const niveis = [
  ['N3c-py-mesmo', 'py', 'mesmo', 59, 'p.fechar(', 'Pedido.fechar (pkg/retorno.py:7)', 'type', 'novo_pedido() -> Pedido, no mesmo arquivo'],
  ['N3c-py-metodo', 'py', 'metodo', 64, 'p.fechar(', 'Pedido.fechar (pkg/retorno.py:7)', 'type', 'p = fab.pedido(), Fabrica.pedido -> Pedido'],
  ['N3c-py-opcional', 'py', 'talvez', 69, 'p.fechar(', 'Pedido.fechar (pkg/retorno.py:7)', 'type', 'opcional() -> Pedido | None'],
  ['N3c-py-campo', 'py', 'campo', 73, 'r.pedido.fechar(', 'Pedido.fechar (pkg/retorno.py:7)', 'type', 'r: Requisicao, campo pedido: Pedido em pkg/requisicao.py'],
  ['N3c-py-fora', 'py', 'fora', 78, 'd.fechar(', SEM, 'homonym', 'agora() -> datetime, de fora do projeto: não decide'],
  ['N3c-py-generico', 'py', 'generico', 83, 'ps.fechar(', SEM, 'homonym', 'pedidos() -> list[Pedido]: genérico, 2.6'],
  ['N3c-py-sem-anotacao', 'py', 'sem_tipo', 88, 'p.fechar(', SEM, 'homonym', 'sem_anotacao() sem retorno anotado: o corpo não é lido'],
  ['N3c-py-dois-alvos', 'py', 'dois_alvos', 93, 'd.fechar(', SEM, 'homonym', 'duplo() em dois arquivos, -> Pedido e -> Nota: os candidatos discordam (caso d)'],
  ['N3c-py-cadeia', 'py', 'cadeia', 99, 'n.fechar(', SEM, 'homonym', 'n = p.nota(), e p só tem tipo pelo retorno: um segundo nível não é seguido'],
  ['N3c-py-laco', 'py', 'laco', 104, 'x.fechar(', SEM, 'homonym', 'x = x.proximo(): a chamada que liga é resolvida sem o retorno, e a consulta termina'],
  ['N3c-rs-outro', 'rs', 'r_outro', 52, 'n.fechar(', 'impl Nota::fechar (src/retorno.rs:37)', 'type', 'Pedido::nota_nova() -> Nota: o retorno, e não o T do caminho'],
  ['N3c-rs-option', 'rs', 'r_option', 57, 'p.take(', SEM, 'homonym', 'Pedido::talvez() -> Option<Self>: invólucro, não decide (roda Option::take)'],
  ['N3c-rs-box', 'rs', 'r_box', 63, 'b.fechar(', SEM, 'homonym', 'Pedido::caixa() -> Box<Self>: invólucro, não decide (Q2; perda declarada)'],
  ['N3c-rs-ref', 'rs', 'r_ref', 69, 'r.fechar(', 'impl Pedido::fechar (src/retorno.rs:25)', 'type', 'referencia(&p) -> &Pedido, em outro arquivo'],
  ['N3c-rs-fora', 'rs', 'r_fora', 74, 's.tamanho(', SEM, 'no_candidate_in_project', 'Pedido::texto() -> String: tipo de fora, derruba o método inerente de Pedido'],
  ['N3c-rs-concordam', 'rs', 'r_mesmo_retorno', 87, 'p.fechar(', 'impl Pedido::fechar (src/retorno.rs:25)', 'type', 'feito() em dois arquivos, ambos -> Pedido (caso b)'],
  ['N3c-rs-discordam', 'rs', 'r_discordam', 92, 'p.fechar(', SEM, 'homonym', 'misto() em dois arquivos, -> Pedido e -> Nota (caso d); o retorno gravado do mesmo arquivo deixa de valer'],
  ['N3c-rs-q', 'rs', 'r_q', 97, 'p.fechar(', SEM, 'homonym', 'let p = Pedido::tentar()?: nenhuma ligação é lida (limite declarado, Q2)'],
];
const nf = path.join(FIX, 'niveis.json');
let text = fs.readFileSync(nf, 'utf8');
const lines = niveis.map(([id, projeto, dono, linha, texto, alvo, resolution, por_que]) => {
  const a = antes.get(`${projeto}|${dono}|${linha}|${texto.replace(/\($/, '()')}`) ?? [...antes.values()].find(x => x.projeto === projeto && x.dono === dono && x.linha === linha && x.texto.startsWith(texto));
  if (!a) throw new Error(`sem atual: ${id}`);
  const atual = { resolution: a.resolution, candidates: a.candidates, alvos: a.alvos };
  return `  { "id": ${JSON.stringify(id)}, "projeto": "${projeto}", "dono": ${JSON.stringify(dono)}, "linha": ${linha}, "texto": ${JSON.stringify(texto)},\n    "atual": ${JSON.stringify(atual).replace(/,"/g, ', "').replace(/":/g, '": ')},\n    "alvo": ${JSON.stringify({ alvo, resolution, passo: '2.3c', por_que: por_que + '; acrescentado no 2.3c, com o binário do 2.3b como atual' }).replace(/,"/g, ', "').replace(/":/g, '": ')} }`;
});
const end = text.lastIndexOf('\n ]\n}');
if (end < 0) throw new Error('fim de niveis.json');
text = text.slice(0, end) + ',\n' + lines.join(',\n') + text.slice(end);
JSON.parse(text);
fs.writeFileSync(nf, text);
console.log('ok', hc.length + rt.length, 'chamadas', niveis.length, 'níveis');

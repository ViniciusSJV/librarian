// Passo 2.4d-0, Parada 2. Acrescenta às fixtures do portão HC-158 a HC-171 e os níveis CN-py-*, com o "atual" do binário do
// 2.4c (ac33664), e muda o alvo do DP-py-classe. Roda uma vez.
//   node fixtures24d0.js <amostra proposta (amostra24d0.js, dump do binário do 2.4c)> <dump do sintético do portão, idem>
'use strict';
const fs = require('fs'), path = require('path');
const { load } = require('../2.4/load.js');
const FIX = path.join(__dirname, '../../../../../tools/precisao/fixtures');
const [prop, sintDir] = process.argv.slice(2);
const rd = f => fs.readFileSync(f, 'utf8').replace(/\r\n/g, '\n');
const VAL = 'proposta pelo agente (25/11/2026, Parada 2 do 2.4d-0); aguarda a validação do usuário na folha';
const EDITOR = {
  'HC-158': 'staticmethod em models.py:1442; decorators.authorize é um decorador de view',
  'HC-162': 'classmethod com o parâmetro chamado self',
  'HC-163': 'import na linha 19 (HC-165 na linha 84)',
  'HC-169': 'from threading import Thread; nenhuma classe Thread no projeto',
};
{
  const f = path.join(FIX, 'amostra-hc.json'), a = JSON.parse(rd(f));
  if (a.chamadas.some(c => c.id === 'HC-158')) throw new Error('já existe');
  for (const c of JSON.parse(rd(prop)).chamadas.filter(c => Number(c.id.slice(3)) >= 158)) {
    c.verdade_validada_por = EDITOR[c.id] ? `conferida no editor pelo usuário (25/11/2026: ${EDITOR[c.id]}); sujeita à confirmação na Parada 2 do 2.4d-0` : VAL;
    c.justificativa = c.justificativa[0].toUpperCase() + c.justificativa.slice(1) + '.';
    c.linha_de_base.alvos.sort();
    a.chamadas.push(c);
  }
  a.selecao += '; HC-158 a HC-171: estratos do passo 2.4d-0 (classe_pelo_nome, classe_pelo_nome_unica, classe_pelo_nome_fora, classe_pelo_nome_atributo), escolhidos à mão na Parada 1 do 2.4d-0';
  fs.writeFileSync(f, JSON.stringify(a, null, 1) + '\n');
}
{
  const G = load(sintDir, 'sint'), f = path.join(FIX, 'niveis.json');
  let s = rd(f);
  const L = s.split('\n'), i = L.findIndex(l => l.includes('"id":"DP-py-classe"'));
  if (i < 0) throw new Error('DP-py-classe');
  const tail = L[i].endsWith(',') ? ',' : '', o = JSON.parse(L[i].trim().replace(/,$/, ''));
  const at = label => { const id = Object.keys(G.N).find(i => G.N[i].category === 'symbol' && G.N[i].label === label && G.declared[i]); if (!id) throw new Error(label); return G.show(id); };
  o.alvo = { alvo: at('Veiculo.andar'), resolution: 'type', passo: '2.4d-0', por_que: 'a resposta mudou no 2.4d-0 (DEC-45): até o 2.4c, o nome da classe não dava tipo e a chamada saía homonym, com os quatro andar; agora a classe é usada, e Veiculo.andar(self) roda o corpo da classe escrita (o Python confirma), nunca o de uma subclasse' };
  L[i] = '  ' + JSON.stringify(o) + tail;
  s = L.join('\n');
  const T = 'type', H = 'homonym', X = 'no_candidate_in_project';
  const rows = [
    ['importada', 'cn_c_importada', 'CnBase.', 'CnBase.somar_cn', T, 'from p import C'],
    ['mesmo-modulo', 'cn_c_mesmo_modulo', 'CnLocal.', 'CnLocal.somar_cn', T, 'classe do mesmo módulo'],
    ['alias-import', 'cn_c_alias_import', 'Alias.', 'CnBase.somar_cn', T, 'import C as D'],
    ['alias-modulo', 'cn_c_alias_modulo', 'Apelido.', 'CnFilha.somar_cn', T, 'D = C no módulo'],
    ['alias-local', 'cn_c_alias_local', 'k.', 'CnFilha.somar_cn', T, 'k = C na função'],
    ['reexportada', 'cn_c_reexportada', 'Reexp.', 'CnFilha.somar_cn', T, 'a reexportação de outro módulo (DEC-42)'],
    ['modulo', 'cn_c_modulo', 'classe_nome.', 'CnBase.somar_cn', T, 'modulo.C.m()'],
    ['modulo-as', 'cn_c_modulo_as', 'modulo.', 'CnFilha.somar_cn', T, 'import a.b as m'],
    ['modulo-pontos', 'cn_c_modulo_pontos', 'pkg.', 'CnFilha.somar_cn', T, 'a.b.C.m()'],
    ['aninhada', 'cn_c_aninhada', 'CnOutra.', 'CnOutra.CnDentro.somar_cn', T, 'classe aninhada no próprio corpo'],
    ['aninhada-modulo', 'cn_c_aninhada_modulo', 'modulo.', 'CnOutra.CnDentro.somar_cn', T, 'modulo.C.D.m()'],
    ['aninhada-herdada', 'cn_c_aninhada_herdada', 'CnHerdeiraDeOutra.', null, H, 'limite: a classe aninhada herdada não é lida'],
    ['aninhada-trocada', 'cn_c_aninhada_trocada', 'CnTrocada.', null, H, 'o corpo liga o nome da aninhada a outra classe: não decide'],
    ['import-local', 'cn_c_import_local', 'N.', 'CnNeta.rotulo_cn', T, 'import dentro da função'],
    ['classmethod', 'cn_r_classmethod', 'CnBase.', 'CnBase.criar_cn', T, 'classmethod'],
    ['staticmethod', 'cn_r_staticmethod', 'CnFilha.', 'CnFilha.somar_cn', T, 'staticmethod'],
    ['instancia', 'cn_r_instancia', 'CnBase.', 'CnBase.rotulo_cn', T, 'objeto explícito; a subclasse sobrescreve, e roda o da classe escrita'],
    ['base-self', 'CnFilha.rotulo_cn', 'CnBase.', 'CnBase.rotulo_cn', T, 'Base.m(self)'],
    ['init', 'CnFilha.__init__', 'CnBase.', 'CnBase.__init__', T, 'Base.__init__(self): chamada, não instancia'],
    ['init-herdado', 'cn_r_init_herdado', 'CnNeta.__init__', 'CnFilha.__init__', T, '__init__ herdado'],
    ['init-ninguem', 'cn_r_init_ninguem', 'CnSem.', null, X, 'ninguém na cadeia declara __init__'],
    ['herdado', 'cn_r_herdado', 'CnNeta.', 'CnBase.herdado_cn', T, 'método de uma base'],
    ['herdado-meio', 'cn_r_herdado_meio', 'CnNeta.', 'CnFilha.somar_cn', T, 'para na primeira que declara'],
    ['solta', 'cn_r_solta', 'CnBalde.', 'CnBalde.autorizar_cn', T, 'função solta com o mesmo nome (DEC-38)'],
    ['sombra-variavel', 'cn_r_sombra_variavel', 'CnBase.', 'CnOutra.rotulo_cn', T, 'variável com o nome da classe: vale o construtor dela'],
    ['sombra-parametro', 'cn_r_sombra_parametro', 'CnBase.', null, H, 'parâmetro com o nome da classe: não decide'],
    ['sombra-for', 'cn_r_sombra_for', 'CnBase.', null, H, 'alvo de for: não decide'],
    ['sombra-compreensao', 'cn_r_sombra_compreensao', 'CnBase.', null, H, 'alvo de compreensão: não decide'],
    ['condicional', 'cn_r_condicional', 'Z.', null, H, 'duas classes possíveis: não decide'],
    ['sombra-condicional', 'cn_r_sombra_condicional', 'Base2.', null, H, 'alias que um if troca: não decide'],
    ['atributo', 'cn_r_atributo', 'CnComAtributo.', null, H, 'o nome é atributo, não método: não decide'],
    ['religada', 'cn_r_religada', 'CnReligada.', null, H, 'def m e depois m = outra: não decide'],
    ['religada-objeto', 'cn_g_religada_chamada', 'x.', 'solta_tarefa_cn', null, 'LINK FALSO CONHECIDO das regras antigas (2.4b): o Python roda solta_tarefa_cn. 0 casos no Healthchecks; se aparecer um, corrigir antes do passo seguinte'],
    ['protocolo', 'cn_r_protocolo', 'CnContrato.', null, H, 'limite: Protocol pelo nome não decide'],
    ['metaclasse', 'cn_r_metaclasse', 'CnComMeta.', null, H, 'limite: método da metaclasse não decide'],
    ['privado', 'CnPrivada.abrir_cn', 'CnPrivada.', 'CnPrivada.__cofre_cn', T, 'nome privado dentro da própria classe'],
    ['privado-fora', 'CnCofre2.abrir_cn', 'CnPrivada.', null, H, 'nome privado escrito em outra classe: não decide'],
    ['ninguem', 'cn_r_ninguem', 'CnSem.', null, X, 'ninguém na cadeia declara: o candidato de outra classe sai'],
    ['so-de-fora', 'cn_f_so_de_fora', 'CnTarefa.run', null, X, 'método da base de fora'],
    ['fora-classmethod', 'cn_f_classmethod_de_fora', 'CnDic.', null, X, 'classmethod da base de fora'],
    ['fora-propria', 'cn_f_propria', 'CnTarefaPropria.', 'CnTarefaPropria.rodar_fora_cn', T, 'a própria classe declara, antes da base de fora'],
    ['fora-antes', 'cn_f_antes', 'CnMisto.', null, H, 'limite: base de fora antes da do projeto (DEC-43)'],
    ['fora-depois', 'cn_f_depois', 'CnMistoDepois.', 'CnBase.rotulo_cn', T, 'base de fora depois'],
    ['retorno', 'cn_r_retorno_herdado', 'x.', 'CnBase.liberar_cn', T, 'o retorno do método pelo nome da classe (DEC-41)'],
    ['type-self', 'cn_g_type_self', 'type(x).', null, H, 'limite: type(x).m() não é o nome da classe'],
  ];
  const lines = rows.map(([id, dono, texto, alvo, res, por]) => {
    const c = [...G.calls.values()].filter(c => G.N[c.owner].label === dono && c.text.startsWith(texto) && c.kind === 'method_call_observed');
    if (c.length !== 1) throw new Error(`${id}: ${c.length}`);
    const al = alvo ? { alvo: at(alvo), ...(res ? { resolution: res } : {}) } : { alvo: 'sem tipo', resolution: res };
    return '  ' + JSON.stringify({ id: `CN-py-${id}`, projeto: 'py', dono, linha: c[0].line, texto: c[0].text.split('(')[0] + '(', atual: { resolution: c[0].res, candidates: c[0].cands ?? 0, alvos: c[0].targets.map(G.show).sort() }, alvo: { ...al, passo: '2.4d-0', por_que: por } });
  });
  for (const l of lines) { const o = JSON.parse(l); if (o.texto.startsWith('type(')) o.texto = 'type(x).rotulo_cn('; }
  const end = s.lastIndexOf('\n ]');
  s = s.slice(0, end) + ',\n' + lines.map(l => l.replace('"texto":"type("', '"texto":"type(x).rotulo_cn("')).join(',\n') + s.slice(end);
  JSON.parse(s);
  fs.writeFileSync(f, s);
  console.log('níveis novos', rows.length);
}

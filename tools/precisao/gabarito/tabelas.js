// Writes the tables of the Parada 2 of step 2.4e (Parada 1, Q12) from medicao.json (medir.js), as Markdown.
//
//   node tools/precisao/gabarito/tabelas.js --saida <dir>      writes <dir>/tabelas.md
'use strict';
const fs = require('fs'), path = require('path');
const L = require('../lib');
const opt = L.parseArgs(process.argv.slice(2));
const M = JSON.parse(fs.readFileSync(path.join(opt.saida, 'medicao.json'), 'utf8'));
const pct = x => x === null || x === undefined || Number.isNaN(x) ? '—' : `${(100 * x).toFixed(1).replace('.', ',')}%`;
const frac = (a, b) => b ? `${a} de ${b} (${pct(a / b)})` : '0 de 0';
const ic = r => r && r[0] !== null ? `${pct(r[0])} a ${pct(r[1])}` : '—';
const LANG = { hc: 'Python (Healthchecks)', rt: 'Rust (ray tracer)' };
const GN = { execucao: 'execução', tipos: 'tipos' };
const out = [];
const row = xs => out.push(`| ${xs.join(' | ')} |`);
const head = xs => { row(xs); row(xs.map(() => '---')); };

for (const [lang, P] of Object.entries(M.projetos)) {
  out.push(`\n## ${LANG[lang]}\n`);
  out.push(`Ocorrências de chamada: ${P.ocorrencias} (fora de testes: ${P.producao}).\n`);

  out.push('### Classificação por gabarito (fora de testes)\n');
  const cats = [...new Set(Object.values(P.por_classificacao_producao).flatMap(o => Object.keys(o)))].sort();
  head(['Categoria', ...Object.keys(P.por_classificacao_producao).map(g => GN[g] ?? g)]);
  for (const c of cats) row([c, ...Object.values(P.por_classificacao_producao).map(o => o[c] ?? 0)]);

  out.push('\n### Populações (Q8)\n');
  head(['População', 'Ocorrências', 'Vistas por algum gabarito', 'Discordâncias (link falso candidato ou polimórfica)', 'Taxa entre as vistas', 'IC 95%']);
  for (const [k, n] of [['universo', 'Universo (todas)'], ['fora_de_testes', 'Fora de testes'], ['lista_js', 'Recorte do lista.js (655/245)'], ['portao', 'Recorte do portão (233/94)']]) {
    const r = P.populacao[k];
    row([n, r.total, r.vistas, r.discordancias, r.vistas ? pct(r.discordancias / r.vistas) : '—', ic(r.ic)]);
  }

  out.push('\n### Tabela 1: alcance de cada workflow\n');
  const gns = Object.keys(P.por_classificacao_producao).filter(g => g !== 'combinado');
  head(['Workflow', 'Entradas', 'Símbolos alcançados', 'Pelos dois', 'Só pelo grafo', 'Só pela execução', 'Chamadas no alcance', ...gns.map(g => `Cobertura ${GN[g]}`), 'Vistas por algum', 'Marcos', lang === 'rt' ? 'Arestas perdidas (rayon)' : 'Perda (falhas de ambiente)']);
  for (const [w, W] of Object.entries(P.workflows)) {
    row([`${w}. ${W.nome}`, W.entradas.join('; ') + (W.entradas_nao_achadas ? ` (${W.entradas_nao_achadas} não achadas)` : ''), W.simbolos, W.pelos_dois, W.so_grafo, W.so_execucao, W.chamadas,
      ...gns.map(g => frac(W.cobertura[g], W.chamadas)), frac(W.vistas_algum, W.chamadas), W.marcos + (W.marcos_fora.length ? ` (fora: ${W.marcos_fora.join(', ')})` : ''),
      lang === 'rt' ? (W.rayon?.length ? `${W.rayon.length}: ${[...new Set(W.rayon)].join(', ')}` : '0') : (W.perda_signal ? `signal: ${W.perda_signal.nao_vistas_execucao} de ${W.perda_signal.chamadas} chamadas sem execução` : '—')]);
  }
  for (const [w, W] of Object.entries(P.workflows)) out.push(`\n- ${w}: só pelo grafo, exemplos: ${W.so_grafo_exemplos.join(', ') || '—'}; só pela execução: ${W.so_execucao_exemplos.join(', ') || '—'}`);

  out.push('\n\n### Tabela 2: lista de lacunas, por família (ordenada pelas chamadas afetadas)\n');
  head(['#', 'Família', 'Chamadas (produção / teste)', 'Tipos de discordância', 'Por gabarito', 'Cai num workflow', 'Link falso candidato em workflow', 'Exemplos']);
  P.familias.forEach((f, i) => row([i + 1, f.id ? `${f.id} ${f.nome}` : `sem família: \`${f.chave.replace(/^novo\|/, '')}\``, `${f.total} (${f.producao} / ${f.teste})`,
    Object.entries(f.tipos).map(([k, v]) => `${k} ${v}`).join(', '), Object.entries(f.por_gabarito).map(([k, v]) => `${GN[k] ?? k} ${v}`).join(', '),
    f.workflows.join(', ') || '—', f.lf_em_workflow, f.exemplos.slice(0, 3).join('<br>')]));

  out.push('\n### Tabela 3: para fixar o X\n');
  head(['Família', 'Chamadas', 'Acumulado', 'Link falso candidato dentro de um workflow?']);
  let acc = 0;
  for (const f of P.familias) { acc += f.total; row([f.id ?? `\`${f.chave.replace(/^novo\|/, '')}\``, f.total, acc, f.lf_em_workflow ? `sim (${f.lf_em_workflow})` : 'não']); }
  const counts = P.familias.map(f => f.total).sort((a, b) => b - a);
  out.push('\n' + [2, 3, 4].map(k => counts.length > k ? `- ${k} famílias acima do X: X entre ${counts[k]} e ${counts[k - 1] - 1}` : `- ${k} famílias: há só ${counts.length}`).join('\n'));

  out.push('\n\n### Tabela 4: para fixar o teto (fora de testes; grupo provisório, Q9)\n');
  head(['Grupo provisório', 'Ligações', 'Vistas por algum gabarito', 'Vistas pela execução', 'Só pelos tipos', 'Discordâncias (candidatas)', 'Gabaritos divergem', 'Taxa entre as vistas', 'IC 95%', 'Piso do UQ']);
  for (const [g, r] of Object.entries(P.grupos)) {
    const icv = r.ic ?? [null, null];
    row([g.replace(/_/g, ' '), r.ligacoes, r.vistas, r.vistas_execucao, r.so_tipos, r.discordancias, r.divergem, r.vistas ? pct(r.discordancias / r.vistas) : '—', ic(icv), g === 'provavel_pelo_nome' ? (lang === 'hc' ? '≥ 21 de 233' : '≥ 11 de 94') : '—']);
  }

  out.push('\n### Tabela 5: cobertura por família e por workflow\n');
  head(['Conjunto', 'Ocorrências', ...gns.map(g => `Vistas ${GN[g]}`), 'Vistas por algum', '3 ÷ vistas (menor teto verificável com 0 discordâncias)']);
  for (const [w, W] of Object.entries(P.workflows)) row([`workflow ${w}`, W.chamadas, ...gns.map(g => W.cobertura[g]), W.vistas_algum, W.vistas_algum ? pct(3 / W.vistas_algum) : '—']);

  out.push('\n### Tabela 6: teto do caminho\n');
  head(['Workflow', 'Profundidade (união)', 'Nós reais (união)', 'Passa de 6 camadas?', 'Passa de 300 nós?', 'Passa de 120 nós (o Studio)?', 'Profundidade só pelo grafo', 'Nós só pelo grafo']);
  for (const [w, W] of Object.entries(P.workflows)) row([w, W.profundidade_uniao, W.simbolos, W.profundidade_uniao > 6 ? 'sim' : 'não', W.simbolos > 300 ? 'sim' : 'não', W.simbolos > 120 ? 'sim' : 'não', W.profundidade_grafo, W.nos_grafo]);

  out.push('\n### Calibração: o gabarito contra a verdade validada (fixtures)\n');
  head(['Gabarito', 'Itens', 'Não viu', 'Concorda com a verdade', 'Discorda da verdade']);
  for (const [g, r] of Object.entries(P.calibracao)) row([GN[g] ?? g, r.itens, r.nao_viu, r.concorda, r.discorda]);
  for (const [g, r] of Object.entries(P.calibracao)) if (r.discordancias.length) out.push(`\n- ${GN[g] ?? g}, discordâncias: ${r.discordancias.join('; ')}`);

  out.push('\n\n### Conhecidos (PLANO, 2.4d; UQ)\n');
  head(['Local', 'Texto', 'Alvo do Librarian', 'Teste?', ...gns.map(g => GN[g])]);
  for (const k of P.conhecidos) row([k.local, `\`${k.texto}\``, k.alvo, k.teste ? 'sim' : 'não', ...gns.map(g => k[g])]);

  out.push('\n### Só do gabarito e extras\n');
  out.push('```\n' + JSON.stringify({ so_do_gabarito: P.so_do_gabarito, fora_da_edition: P.fora_da_edition, extras: P.extras, por_resolucao: P.por_resolucao }, null, 1).slice(0, 6000) + '\n```');
}
fs.writeFileSync(path.join(opt.saida, 'tabelas.md'), out.join('\n') + '\n');
console.log('tabelas.md gravado');

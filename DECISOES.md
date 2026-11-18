# Decisões do Librarian

Registro das decisões já tomadas. Cada linha cita o texto como está escrito e onde ele está. A origem é a tabela D3 do [inventário do Piloto 1b](docs/reports/piloto-1b/03-inventario-e-auditoria.md#d3-decisões-já-tomadas), com as referências atualizadas para o estado de 18/11/2026, mais a decisão A do G3 e as decisões da consolidação documental.

**Como citar:** pelo id (`DEC-11`), com a referência ao lado quando ajudar (ex.: "decisão Homônimos, PLANO-H:1555"). As abreviações dos arquivos e os prefixos estão em [docs/IDS.md](docs/IDS.md). `PLANO-H` é o plano até 18/11, em [docs/historico/PLANO-ate-18-11.md](docs/historico/PLANO-ate-18-11.md), com as mesmas linhas do plano em `aae51d7`.

**Estado:** "vigente" quando nada a reabriu. Uma decisão reaberta continua aqui, com o passo que a reabre.

| Id | Decisão | Trecho | Onde | Estado |
|---|---|---|---|---|
| DEC-01 | Grafo v2 | "Decisão do usuário: opção A, ou seja, formato novo exportável a partir de qualquer edition verificada" | PLANO-H:407 | Vigente |
| DEC-02 | Linhas numeradas | "Decisão do usuário: opção A, ou seja, numeração como opt-in da trilha `prepare`" | PLANO-H:504 | Vigente |
| DEC-03 | Claims | "**A:** formato `claims` opt-in por projeto, com a prosa mantida" | PLANO-H:588 | Vigente |
| DEC-04 | Acervo de fatos | "os fatos formam um **acervo consultável** … **só FACT** é promovido … a revisão é **por item**" | PLANO-H:698 | Vigente |
| DEC-05 | Cadeia | "passam as locations conferidas e os fatos vigentes do acervo; o texto dos claims anteriores não passa" | PLANO-H:759 | Vigente |
| DEC-06 | G6a–c | "o Graph Engine fecha antes de seguir no LLM … teto de 8 homônimos" | PLANO-H:796 | Vigente |
| DEC-07 | G6d-1 | "Decisões do usuário: a expansão é automática (dono, campos lidos …" | PLANO-H:826 | Vigente |
| DEC-08 | Condições | "A linha de condições fica encerrada por enquanto, e o extractor não muda." | PLANO-H:1345 | Vigente |
| DEC-09 | Tempo | "a contagem continua sendo a medida confiável, e o tempo instrumentado é só referência" | PLANO-H:1407 | Vigente |
| DEC-10 | Lacunas | "A investigação de lacunas fica suspensa, sem descartar os resultados." | PLANO-H:1482 | Vigente |
| DEC-11 | Homônimos | "investigação suspensa, sem implementar a resolução simulada e sem teste com o modelo" | PLANO-H:1555 | **Reaberta na Fase 2** (passo 2.1 do [roadmap](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md#roadmap)) |
| DEC-12 | Qwen nas mudanças | "fora da escada de mudanças até o fine-tune; continua nos fatos" | CTX:282 | Vigente |
| DEC-13 | Envelope | "não aumentar o limite nem alterar a composição sem evidência" | CTX:308 | Vigente |
| DEC-14 | Trabalho | "commits em português, sem acentos … documentação validada antes do commit … push só quando pedido" | CTX:320 | Vigente |
| DEC-15 | Pareto | "sem dominância nem seleção Pareto por enquanto" | CTX:420 | Vigente; a comparação de propostas é trilha congelada (DEC-31) |
| DEC-16 | Passo 3 | "Decisão formal do usuário: opção A. O Passo 3 está encerrado como entregue" | CTX:543 | Vigente |
| DEC-17 | Capas | "ficam como elemento da identidade visual, sem atribuição literária" | CTX:696 | Vigente |
| DEC-18 | Escada | "Opção A, aplicada em 18/11 (`fn ladder`): um pedido precisa de `turn` …" | CTX:851 | Vigente |
| DEC-19 | Léxico | "Está mantida como operação explícita e separada da investigação de código (decisão de 18/11)" | CTX:853 | Vigente |
| DEC-20 | Decisões de 18/11 | `verificavel` em inglês com duas traduções; `/api/path-questions` mantida; "Abrir arquivos" mantido; Ponytail; baterias antigas não apagadas | CTX:857–861 | Vigente |
| DEC-21 | Multiagente | "Não adicionar framework de agentes aos crates centrais." | CTX:927 | Vigente |
| DEC-22 | Autoria | "**Revisão por agente não é aprovação humana.**" | PLANO-H:2298 | Vigente; a correção dos fatos é da Fase 3 |
| DEC-23 | Promoção por item | "Fica registrada a decisão futura sobre **promoção por item**" | PLANO-H:2300 | Vigente; implementação na Fase 3 |
| DEC-24 | Formato dos fatos | "O formato atual dos fatos **não muda nesta iniciativa**." | PLANO-H:2302 | Vigente até a Fase 3 (formato 3) |
| DEC-25 | Rust sem piora (G1) | "Uma correção que melhora o Python e piora o Rust sem aviso não é aceitável." | PLANO-H:2284 | Vigente; portão de cada passo da Fase 2 |
| DEC-26 | Frontend | Decisões aprovadas 1 a 8 e a tabela de questões fechadas | ESP:50–59, ESP:1090–1113 | Vigente |
| DEC-27 | Frontend, por etapa | Tabela "Registro de decisões funcionais aprovadas" | EST:1517–1557 | Vigente |
| DEC-28 | Decisão A do G3 | "Decided in the Piloto 1b: it is `no_candidate_in_project` too (no declaration of the project matches the occurrence), and its written text is what tells it apart from `missing_helper(flip)`. The graph contract is not changed to say more." | [crates/librarian-ingest/tests/g3.rs:185-188](crates/librarian-ingest/tests/g3.rs#L185-L188) | Vigente |
| DEC-29 | Princípios permanentes | 14 princípios, de "não escolhe os agentes" até "o extrator deve evoluir sem tornar o núcleo dependente de uma linguagem" | ARQ:154–169 | Vigente |
| DEC-30 | Etapa 7 do Studio | A verificação da Etapa 7 (PLANO-H:2013–2015) vira critério de aceite de toda entrega de interface; não é passo da Fase 3 | Parada 1 do passo 1.2 (18/11/2026); [roadmap](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md#roadmap) | Vigente |
| DEC-31 | Etapa 6 do Studio | "Fatos por fonte" vai para a Fase 3. A comparação de propostas de mudança (HYPOTHESIS, `execution.json`, DEC-15) fica como trilha congelada com destino na Fase 8, base do agente de Performance. A Etapa 6 depende da Etapa 5 | Parada 1 do passo 1.2 (18/11/2026); [roadmap](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md#roadmap) | Vigente |
| DEC-32 | Histórico do plano | O plano até 18/11 foi copiado inteiro para `docs/historico/`, com as mesmas linhas; só os destinos de 22 links relativos mudaram. O plano vivo guarda só o roadmap | Parada 1 do passo 1.2 (18/11/2026); [docs/historico/README.md](docs/historico/README.md) | Vigente |

As linhas do CTX são as deste arquivo depois da consolidação de 18/11/2026. O inventário (relatório 03) cita as mesmas decisões pelas linhas de `aae51d7`.

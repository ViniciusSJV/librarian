# Contexto de continuidade do Librarian

**O estado atual não fica mais aqui: comece por [ESTADO-ATUAL.md](ESTADO-ATUAL.md).** Desde 18/11/2026 este arquivo é um diário datado: cada seção descreve o estado na data dela e não é atualizada depois. Onde uma seção diverge do ESTADO-ATUAL, vale o ESTADO-ATUAL. A última seção é a [21](#21-piloto-1b-p1-g3-partes-b-c-e-e-e-q6-18112026).

**Atualização de 18/11/2026, depois da publicação de `d949f91`: o Piloto Python 1 (Healthchecks) foi executado sem mudar código, e a direção para uma infraestrutura de evidências com vários consumidores foi registrada ([seção 19](#19-piloto-python-1-e-direção-multiagente-18112026): comece por ela).** A próxima prioridade é o Piloto 1b, ainda não iniciado.

**Atualização de 18/11/2026: a consolidação (E1 a E5, simplificação do Studio e proteção da escada) foi publicada sobre `27f7bdd` ([seção 18](#18-consolidação-evidence-builder-independente-simplificação-e-proteção-da-escada-17112026-e-18112026): comece por ela).** As seções 1 a 17 descrevem estados anteriores; onde divergem, vale a 18. Em especial, o Evidence Builder agora é um crate (`librarian-evidence`), e a tabela da seção 2 está desatualizada nesse ponto.

**Atualização de 11/11/2026: a Etapa 4 (Evidence Builder) está em andamento e não está encerrada, com todo o trabalho sem commit sobre `60d4d50` ([seção 17](#17-etapa-4-evidence-builder-07112026-a-11112026): comece por ela).** O texto abaixo, até a seção 16, descreve o estado de 06/11/2026.

Conferido no repositório em 06/11/2026, com os Passos 1 e 2 da Etapa 3 da frente do Studio concluídos, o Passo 3 encerrado no backend ([seção 12](#12-passo-3-encerrado-e-sequência-da-etapa-3-06112026)), o Passo 4 concluído, nas entregas 4A e 4B ([seção 14](#14-4b-concluída-e-encerrada-06112026)), e os Passos 6 e 7 feitos, em `94089d3` (**[seção 16](#16-passos-6-e-7-feitos-06112026): comece por ela**). Documento de transferência entre sessões. Não substitui o plano: as tabelas completas estão no plano até 18/11, [docs/historico/PLANO-ate-18-11.md](docs/historico/PLANO-ate-18-11.md), da linha 1116 em diante, e o andamento desta frente em [Evidence Builder no Studio](docs/historico/PLANO-ate-18-11.md#evidence-builder-no-studio-etapas-0-a-7-05112026).

**Como ler as marcas:** **[confirmado]** foi verificado agora no repositório ou nos dados gravados. **[registrado]** vem do plano ou da memória de trabalho, sem nova verificação. **[hipótese]** não foi demonstrado. **[não confirmado]** não pôde ser verificado.

## 1. Visão geral

O Librarian lê um projeto como um bibliotecário lê um acervo: guarda as fontes byte a byte, relaciona o que está escrito e acha onde vale perguntar. O modelo vê só a evidência da pergunta, e nada do que ele diz vira fato sem passar por um portão mecânico e por uma pessoa. É em Rust; analisa Rust e Python hoje. O uso normal é pelo Librarian Studio, uma página web. Projetos de teste: `renderer` (ray tracer em Rust) e `logistica-api` (Python).

Lema do projeto: *condensar fatos a partir de vapores de nuances*. Princípio desta frente: o objetivo não é fazer o modelo pensar mais, e sim entregar informação melhor, suficiente e verificável, para que ele tenha menos espaço para inventar.

## 2. Arquitetura e responsabilidades

### Onde cada parte está no código [confirmado]

A tabela que estava aqui ficou desatualizada em 17/11, quando o Evidence Builder virou o crate `librarian-evidence`. O mapa atual está em [ESTADO-ATUAL.md](ESTADO-ATUAL.md#onde-cada-parte-está); a tabela de 06/11 continua em `git show aae51d7:CONTEXTO-CONTINUIDADE.md`, linhas 25 a 33.

Ao ler "Graph Engine" em pedidos e documentos, confirme se é o grafo de `librarian-ingest` ou o crate de dossiês.

### Responsabilidades

- **Grafo:** relações estruturais pelo nome escrito. Nunca é resolução por compilador, nem afirmação semântica. Uma ligação com homônimos é candidata, não fato.
- **Evidence Builder:** decide o que vai ao modelo, por regras e sem modelo. Papéis, nesta ordem: alvo, origem do valor, custo, caminho, apoio, restrição. Registra o que ficou de fora e o que o grafo não resolveu.
- **LLM Engine:** prepara a query e faz a verificação mecânica de categorias e citações. Não aprova o sentido de nada.
- **Portão:** execução real da mudança numa cópia, com a medição do projeto e os testes. Mostra efeitos; não julga sentido.
- **Pessoa:** decide. O que passou continua HYPOTHESIS até alguém aprovar como fato.

### Quatro responsabilidades que não se misturam

**Seleção:** quais informações são necessárias. **Investigação:** quais relações existem entre elas. **Medição:** o que aconteceu na execução. **Decisão:** se a mudança é aceitável diante dos resultados.

### O envelope e seus limites [confirmado]

`envelope.rs:15`: `BUDGET_BYTES = 6 * 1024` (6.144 B de código), com no máximo **16 declarações**. Alvos vão sempre. Testes ficam de fora porque o portão os roda. O que a medição contou e não rodou sai, salvo se for origem de um valor. Dentro de custo e caminho, ordena-se por contagem (`envelope.rs:427`). Observado: quem limita costuma ser o número de itens, não os bytes.

### Categorias que não podem ser confundidas

| Categoria | O que é | Quem produz |
|---|---|---|
| Relação estrutural | Algo escrito no código: chama, escreve, tem o tipo | Grafo |
| Evidência | Trecho verificável enviado ao modelo | Evidence Builder |
| FACT, INFERENCE, LACUNA | Categorias de uma resposta, conferidas mecanicamente | Modelo e LLM Engine |
| Hipótese | Proposta de mudança. Passar no portão não a torna fato | Modelo e portão |
| Fato validado | Aprovado por uma pessoa | Pessoa |

### Regra de método

Não confundir **o que o modelo solicita**, **o que o sistema consegue encontrar** e **o que efetivamente melhora o resultado**. Exemplo: `inverse` é citada em 25 lacunas de `r5` e tê-la no envelope não muda a taxa de lacuna (28% e 28%). Também não confundir: o grafo encontra, o envelope admite, o orçamento decide o que fica, e o gabarito verifica. Uma resolução tecnicamente correta pode piorar o resultado se tirar evidência mais útil.

### Medição e portão [confirmado no código e no STUDIO]

- **Contagem é exata.** O tempo de um comando que conta não é evidência de melhora, porque inclui os contadores.
- **`measures` no perfil é um contrato de avaliação:** o que cada número significa e para que lado é melhor. O Librarian conhece isso e não sabe se memória vale mais que velocidade.
- **Uma piora não recusa a mudança:** aparece ao lado da decisão, porque um cache pode gastar memória de propósito. A troca é decisão humana.
- **O que é medido não pode ser o que muda:** contador apagado é recusado, e perfil e caminhos protegidos não podem ser tocados.
- **A escada** testa todas as propostas de uma resposta e marca a de maior % de ganho como `passed` e as outras como `passed_smaller`. Isso é o que o Pareto questiona (seção 9).

Resultados são hipóteses, não regras. O mesmo envelope já deu 4/6 e 8/8 em medições diferentes.

## 3. Estado atual do repositório [confirmado]

**Branch:** `main`. Sobre `7868912`, o último commit em `origin/main` antes desta publicação, há 15 commits: os 14 da tabela abaixo e o que traz este texto. Sem stash. **Publicados em `origin/main`, em `f6f3958`,** conferido em 06/11/2026: veja a [seção 12](#12-passo-3-encerrado-e-sequência-da-etapa-3-06112026). Sobre `f6f3958`, também publicados: a 4A (`1908658`, [seção 13](#13-4a-concluída-e-encerrada-06112026)), a 4B (`3b64f66`, [seção 14](#14-4b-concluída-e-encerrada-06112026)) a apresentação pública (`5136dbe`) e duas revisões documentais: veja a [seção 15](#15-publicação-da-4a-da-4b-e-da-apresentação-06112026). Os Passos 6 e 7 estão em `94089d3`: veja a [seção 16](#16-passos-6-e-7-feitos-06112026).

**Commits sobre `7868912` (do mais novo ao mais antigo):**

| Commit | Conteúdo |
|---|---|
| O mais recente | Docs: este registro de continuidade. O hash não cabe aqui: confira no `git log` |
| `9659de5` | Test: baterias de tela preservadas em `crates/librarian-studio/tests/tela` |
| `47337e9` | **Código:** Passos 1 e 2 da Etapa 3, com cobertura, contagem e resolução do alvo |
| `746fa31` | Docs: estado depois do commit da documentação e do arquivo da Etapa 2 |
| `a530017` | Docs: contrato de `origin.selection`, limitações e etapas do Evidence Builder |
| `99f69f3` | **Código:** Etapa 2 da frente do Studio, com decisões e proveniência |
| `8b575ee` | Docs: este arquivo |
| `94e2238` | Docs: plano com homônimos suspensos e a próxima frente |
| `df43138` | Docs: defeitos da lacuna corrigidos, rodada sem espaço e linha de base dos homônimos |
| `abb682a` | **Código:** método de dois traits não é adivinhado, trait pedido não é filtrado, rodada sem espaço no registro |
| `e07c038` | Docs: `Object` e o trait em `r5` sem conclusão, lacunas suspensas |
| `bab430a` | Docs: o que ficou de fora do envelope, classes de ausência, defeitos |
| `02d7ae6` | Docs: tempo na seleção, contagem como medida confiável, correção de "Onde pesa" para o front |
| `9fcb05e` | Docs: notas geradas e condição da função perguntada, condições encerradas por enquanto |
| `7b13e13` | Docs: portão com o modelo, condições e invariantes, notas geradas como próximo experimento |

Último commit em `origin/main` antes desta publicação: `7868912`.

**Passos 6 e 7:** em `94089d3`, na [seção 16](#16-passos-6-e-7-feitos-06112026). O contrato e a documentação da Etapa 2 estão em `a530017`.

**Testes, rodados em `9659de5`:** `cargo test --workspace --locked`: **240 passaram, 0 falharam, 2 ignorados.** Os ignorados são o gabarito (`crates/librarian-ingest/tests/gabarito.rs`), que depende de dados locais, e um teste em `crates/librarian-llm-engine/src/cli/send_ollama.rs`.

**Outros projetos:**

| Projeto | Estado |
|---|---|
| `renderer` | `master` em `e38254b`, limpo. 3 pastas de trabalho (`renderer`, `renderer-claude`, `renderer-librarian`). 12 refs locais em `refs/librarian-arquivo/*` |
| `renderer-librarian` | `diff-librarian` em `35cb76e`, limpo |
| `logistica-api` | `main` em `127dbd7`. `backend/librarian.json` com `measures` **sem commit naquele projeto**, por decisão; `LOGISTICS.md` não rastreado, e sua origem não foi verificada. O experimento de Python usou `cd15515`, ancestral de `127dbd7` |

**Ambiente:** nada escutando em 8765 nem em 8830–8839 **[registrado]**; os servidores de teste em 8841 e 8842 foram encerrados **[confirmado]**. Nenhum contêiner em execução; o volume de `logistica-api-db-1` foi deixado intacto **[registrado]**. `Temp/lt-ev` (4,3 GB), diretório de build de um experimento antigo: **decisão do usuário, não limpar ainda.** A máquina não tem `python`: use `uv run --no-project python`. Heredocs do Bash perderam `\` em scripts: escreva-os com a ferramenta Write.

**Dados que o Git não guarda (`.studio/` é ignorado):**

- `.studio/gabarito`: 6 casos com edition, medição e declarações editadas.
- `.studio/arquivo-copias/`: `condicoes-`, `notas-geradas-`, `foco-`, `tempo-`, `lacunas-`, `trait-` e `homonimos-20261104`, com resultados, scripts e diffs.
- `.studio/arquivo-copias/rodadas-20261104` (516 MB): as editions, execuções e turnos das 6 rodadas (`root`, com `r5`, que não faz parte do gabarito, e `rootpy`), além de resultados e scripts. Tem `LEIA-ME.md` e `MANIFESTO.sha256`. Conferido: 29.174 hashes iguais aos da origem, e `condres.js`, `traitres.js` e `lac3.js` reproduzem a mesma saída sobre a cópia.
- **Origem preservada:** o scratchpad em `AppData/Local/Temp/claude/.../d9e4a878-dc19-44c2-b141-f69816ea3920/scratchpad` não foi alterado, mas é temporário. **Não remova, sobrescreva nem reorganize esses dados sem aprovação do usuário.**
- **`.studio/arquivo-copias/evidence-builder-etapa2-20261105` (939 MB), material da Etapa 2 do Studio:** baterias da tela (`etapa2.js`, `etapa2b.js`, `etapa2c.js`), modelo externo (`etapa2-modelo.js`), mutações (`mutacao3.js`, `final.sh`), raízes de teste (`iso/`), logs, telas, auditorias, o executável usado nos testes, o documento de contrato aprovado e, em `conversa/`, o diagnóstico integral, os pedidos do usuário e as respostas. Comece pelo `LEIA-ME.md`. `ORIGEM.tsv` diz de onde veio cada arquivo, e `EXCLUIDOS.tsv`, o que ficou de fora. Conferir: `sha256sum -c MANIFESTO.sha256 --quiet`, nessa pasta; SHA-256 do manifesto: `81d7739a576ed225ea714e67a313db806b919de34342795b606e56126dc9a601`. Fora do Git, sem cópia fora desta máquina.
- **Limites desse arquivo:** a restauração **não foi executada**: está provado que as cópias são iguais à origem, não que a restauração funcione de ponta a ponta. Ela precisa ser no caminho original, porque a chave do projeto Python é o hash do caminho. `iso/mix/project` não tem `.git`, que trazia nome e e-mail: esse projeto precisa de `git init` e de um grafo novo, e nenhuma das quatro baterias o usa. `conversa/` traz extratos, porque a transcrição crua ficou de fora. `tasks/begpgaj0k.output` sumiu da origem depois da cópia, e a cópia foi conferida no momento em que foi feita.
- **Origem preservada:** o scratchpad da sessão `465f4d1b-0d17-4dff-903f-d3077bb3c8f4` não foi movido nem apagado, mas é temporário.
- **`.studio/arquivo-copias/baterias-passos-6-e-7-20261106`, baterias dos Passos 6 e 7:** `etapa2.js`, `etapa2b.js`, `etapa2c.js` e `etapa2-modelo.js` nas versões da validação final, com as três asserções novas, mais os registros da auditoria. O arquivo da Etapa 2 não mudou. Comece pelo `LEIA-ME.md`. Conferir: `sha256sum -c MANIFESTO.sha256 --quiet`, nessa pasta; SHA-256 do manifesto: `2d1547d3b213285ff429caf9f7cecd8041c5548c628ae476f422556eaa695f95`. Fora do Git, sem cópia fora desta máquina, e a restauração não foi executada.
- **Limite do arquivo:** ele serve para reanalisar. Reenviar exige o studio e o projeto na base da rodada. `hom-casos.json` e `tempo-casos.json` têm caminhos absolutos da origem e precisam ser gerados de novo se ela sumir. O arquivo não está no Git, e não há cópia fora desta máquina.

**Alterações temporárias já removidas [confirmado]:** `crates/librarian-ingest/tests/tempo_tmp.rs`, `crates/librarian-ingest/tests/homonimos_tmp.rs` e o teste injetado em `ladder.rs`. As cópias dos experimentos (`lc-*`) e os builds `lt-gate`, `lt-gen`, `lt-tempo`, `lt-fix` e `lt-handoff` também foram removidos.

## 4. Histórico técnico relevante

**Código desta frente [confirmado no `git log`]:**

| Commit | O que fez |
|---|---|
| `a979f60` | Seleção por papéis e orçamento em bytes, com o que ficou de fora, lacunas e comportamento |
| `75d8a1a` | Escada testa todas as propostas e fica com a de maior ganho |
| `d24af75` | Envelope novo, desligado: notas ao modelo e query sem o excerpt repetido |
| `9c5c761` | Escada de mudanças vai direto ao Haiku; o qwen continua nos fatos |
| `fa10448` | Query compacta ligada nas perguntas novas |
| `fd45483` | `measures` no perfil, com o que piorou ao lado da decisão |
| `c265776` | Cópia recriada aplica a mudança: um `.diff` que sobrou fazia a linha de base passar por mudança (100 em vez de 20) |
| `b5eefca` | Registro da escada: piora em destaque, também no título da mudança escolhida |
| `abb682a` | Correções do caminho da lacuna (abaixo) |

**`abb682a` em detalhe:**

- `ladder.rs`, `resolve`: quando **mais de um trait** declara o método, nada é trazido, em vez do primeiro (`Incuse::transform`). Escrito com o trait, continua vindo. Não escolhe qual trait é o certo, porque isso seria uma heurística nova.
- `ladder.rs`, `ran_type`: um trait pedido por nome vem quando um método dele, ou um `impl` dele, rodou.
- `ladder.rs`, `lacuna_room`; `main.rs`, `no_room`; `index.html`, `ladder.noRoom`: o registro diz que não houve espaço. Nada sai do envelope.
- Testes em `store.rs`: `a_method_two_traits_declare_is_not_guessed` e `a_trait_asked_by_name_comes_when_it_ran`, os dois vistos falhando antes da correção. Em `ladder.rs`: `a_full_question_has_no_room_for_a_lacuna`.

**Efeito de `abb682a`:** nenhum na seleção nem nas aprovações. A seleção não usa esse caminho, e a rodada não acrescenta nada com 16 declarações. As 41 lacunas gravadas têm espaço 0.

**Interface nos temas [registrado, com limite]:** o aviso de piora e a linha `no_room` foram vistos nos temas claro e escuro e em tela estreita, sem erro de página, com a escada **simulada**, via `colorScheme` do navegador. **Não foram testados:** a troca manual por `data-theme`, a interface inteira, e o caminho com a escada real.

## 5. Experimentos e resultados

Modelo: Haiku via `claude-cli`, salvo indicação. p é o teste exato de Fisher, duas caudas, sem correção por múltiplas comparações. "Grande" é cortar ao menos 50% das chamadas de `inverse`, salvo indicação. São um projeto Rust e 3 casos Python: nada aqui é regra.

Soma do custo informado das cinco rodadas com LLM mais recentes (E4 a E7 e o teste da E9): cerca de US$ 2,24. A frente inteira custou mais; o total exato **[não confirmado]**.

Casos: `r1` e `r2` (`inverse`), `r3` (`Groups::intersect`), `r5` (`Intersectable::intersect`, alvo em método de trait, fora do gabarito), `py-memoria`, `py-n-mais-1`, `py-async`.

### E1. Dimensionamento e papéis do envelope

- **Hipótese:** o problema é como as fontes são escolhidas, não falta de contexto.
- **Método:** retirar fontes e reenviar: 63 envios, US$ 0,51, 5 variantes.
- **Resultado:** gravado 15/18, nova 15/18, sem custo 9/9, sem origem 5/9, só o alvo 5/9. Sem origem, a mudança grande nunca aparece.
- **Conclusão permitida:** a seleção nova empatou com a gravada, com custo por envio cerca de 8% menor. A origem do valor importa.
- **Não concluído:** que a seleção nova aprove mais; até aqui, em nenhum caso.
- **Decisão:** seleção por papéis fica, e o papel custo fica: numa medição seguinte, tirá-lo melhora `r1` (6/8 × 2/7) e piora `r3` (4/8 × 8/8).

### E2. Escada e query compacta

- **Hipótese:** parar na primeira proposta aprovada perde a maior; o texto repetido na query não ajuda.
- **Método:** 48 envios (US$ 0,39) para cada medição. As notas ao modelo foram medidas juntas.
- **Resultado:** com as mesmas respostas, em 11 de 43 aprovações a escada escolheu proposta melhor. A compacta reduz a query em cerca de 13%, com ganho igual. As notas pioraram: em `r2`, grande 6/8 → 2/8 e ganho 40% → 15%.
- **Conclusão permitida:** esses são os efeitos mais claros da frente; o da escada não depende de comparar envios diferentes.
- **Não concluído:** qual parte das notas atrapalha.
- **Decisão:** escada corrigida e compacta ligadas; notas desligadas.

### E3. `r5` com variantes de envelope

- **Método:** 16 envios por variante (8 na última): gravado 10/16, nova 8/16, trocando `Bounds::intersects` por `inverse` 6/16, com `impl Intersectable for Object` (9,6 KB) 5/8 com 0 sem proposta.
- **Conclusão permitida:** empate. 2/8 × 6/8 nos primeiros envios foi ruído.
- **Decisão:** nenhuma regra muda por menos que uma diferença repetida. Não deixar a origem passar do limite.

### E4. Portão com medidas declaradas

- **Hipótese:** o portão não via memória, e por isso recusava mudanças que a reduziam.
- **Método:** reprodução sem LLM em 17 propostas; depois 18 envios no `logistica-api` em `cd15515`, US$ 0,084, 59 min.
- **Resultado:** memória de 0/3 para 2/3 (gravada) e de 1/3 para 3/3 (nova); N+1 e `async` 3/3, com memória subindo em boa parte e a piora visível.
- **Conclusão permitida:** o portão vê memória, e cada piora aparece ao lado da decisão.
- **Não concluído:** como escolher entre propostas com trocas.
- **Decisão:** mantido. Problema da escolha pela maior %:

| Proposta, envio `py-memoria` nova k=2 | `memory_end_mb` | `routing_cpu_p50_ms` | `plan_p95_ms` | `plans_per_s` | Resultado |
|---|---|---|---|---|---|
| A | **−77%** | +166% | +267% | −57% | escolhida |
| B | +11%, faixas sobrepostas | **−33,5%** | −4%, sem contar | +4%, sem contar | ganho menor |

A resolve o que foi perguntado, com custo alto. B não tem piora registrada e não resolve. **Nenhuma domina a outra.** Foi observado em um envio.

### E5. Condições e invariantes escritas à mão

- **Hipótese:** dizer o que guarda uma chamada e quem escreve um campo melhora a proposta.
- **Método:** 88 envios, US$ 0,69, duas rodadas, mesmo envelope, notas à mão.
- **Resultado:** `r1`: 3/16 sem fatos × 13/16 com condição e invariante (p = 0,001; repetiu nas duas rodadas). Só condição 5/8, só invariante 2/8, condição com todos os campos 4/8. `r5`: 7/16 × 11/16 (p = 0,29).
- **Conclusão permitida:** em `r1`, esses fatos escritos à mão mudaram o resultado de forma repetível.
- **Não concluído:** generalização. Escrevi as notas sabendo a resposta.
- **Decisão:** testar notas geradas antes de mexer no extractor (E6).

### E6. Notas geradas por script

- **Hipótese:** o Librarian produz sozinho o que foi útil à mão.
- **Método:** 120 envios, US$ 0,86, critérios e notas congelados com hash. O gerador lê código, Medir tudo e envelope.
- **Resultado:** `r1` grande: 1/8 sem notas, 1/8 condições, 0/8 campos, 0/8 invariantes, 1/8 tudo. `r5` aprovadas: 4/8 × 6/32 somando as variantes com notas (p = 0,09). O critério de `r3` foi ruim: `Groups::intersect` não cai, e a base já guarda a inversa em 6/8.
- **Conclusão permitida:** não repetiu o efeito, embora a nota gerada de `r1` traga o mesmo fato, como a quarta de cinco notas.
- **Não concluído:** por que a manual funcionou; se as notas atrapalham `r5`; nada sobre `r3`.
- **Decisão:** extractor não muda. Regra, antes de enviar: o "top 5 do projeto" só traz funções internas de `matrix.rs`.

### E7. Condição só da função perguntada

- **Hipótese:** o fato específico e isolado ajuda onde o acumulado não ajudou.
- **Método:** 48 envios, US$ 0,37. Critério fixado: `r1` ≥ 5/8 com base ≤ 2/8, sem `r5` piorar.
- **Resultado:** `r1` grande 2/8 × 4/8; `r3` 7/8 × 7/8; `r5` grande 4/8 × 6/8, aprovadas 4/8 × 7/8.
- **Conclusão permitida:** **não recuperou** pelo critério.
- **Não concluído:** que a nota isolada não ajude. Pista, entre rodadas e depois de ver os dados: em `r5`, 7/8 contra 6/32 das acumuladas.
- **Decisão:** linha de condições encerrada por enquanto.

### E8. Tempo na seleção (sem LLM, sem custo)

- **Hipótese:** ordenar por tempo melhora a seleção.
- **Método:** a seleção real, trocando só o número de ordenação: contagem, tempo inclusivo, tempo por chamada e camadas de Pareto. Mapeamento conferido com o gabarito.
- **Resultado:** `r1`, `r2` e `r3` escolhem o mesmo conjunto. Só `r5` muda, trazendo `inverse` e `World::color_at`, e a troca por `inverse` já deu 6/16 × 8/16. Alvo, origem e apoio ocupam de 12 a 15 das 16 vagas.
- **Achado:** o tempo do Medir tudo **muda a ordem entre as funções**. Em `r1`, de 8 contadores para todos: `Bounds::intersects` 81 → 78 ms; `Groups::intersect` 4.631 → 482.541 ms (104×); `inverse` 809 → 492.456 ms (609×). `inverse` aparece com 17% ou 99% de `color_at`, enquanto guardar a inversa rende cerca de 44% no portão.
- **Conclusão permitida:** papéis e orçamento decidem a seleção; o tempo instrumentado não serve para ordenar. "A ordem é o que vale" não se sustenta.
- **Não concluído:** o custo real de cada função; quanto do tempo total os selecionados representam (o tempo é inclusivo e não soma); benchmarks em Python (só há números do projeto inteiro).
- **Decisão:** contagem é a medida confiável, tempo instrumentado é referência. Linha suspensa até um profiler por amostragem.

### E9. Investigação de lacunas

**Análise sem LLM, 516 envios gravados:**

- **Comparar só as aprovadas não mostra lacuna, por construção:** o validador recusa edição fora das linhas enviadas (0 em 394 mudanças fora do envelope).
- **41 lacunas, 40 em `r5` e 1 em `r1`.** Nenhuma em `r2`, `r3` ou Python.
- **Pedidos em `r5` (144 envios):** `transform` 40 (nunca candidato); `inverse` 25 (corte por orçamento; lacuna 9/32 × 31/112, 28% e 28%); `Intersectable` 22 e `Object` 21 (nunca candidatos); `local_intersect` 21 (limite de itens; 4/16 × 36/128). `impl Intersectable for Object`, 3.948 B: 0/8 × 40/136 (p = 0,11), variante de outra rodada.
- **Classes decididas:** irrelevante até aqui (`inverse`, `local_intersect` por ora, o que fica de fora em `r1`–`r3`); potencialmente importante (`impl Intersectable for Object`, `Object`, o trait); comprovadamente relevante: **nenhuma**.

**Teste de `Object` e do trait em `r5` (32 envios, US$ 0,24):**

- **Hipótese:** dar a estrutura e o contrato do objeto reduz as lacunas.
- **Método:** 16 envios por variante, mesmo limite: saem `Bounds::intersects` (1.174 B) e `Matrix` (109 B), entram `Object` (185 B) e `Intersectable` (1.082 B). Critério: lacuna menor com p < 0,05, sem aprovadas caírem mais de 2.
- **Resultado:** lacuna 8/16 × 3/16 (p = 0,14); aprovadas 7/16 × 8/16; grande 7/16 × 7/16; sem proposta 8/16 × 5/16. A base veio alta contra o histórico de 28% (p = 0,09), e 3/16 não se separa do histórico (p = 0,56). Os envios que deixaram de ser lacuna viraram recusas. As 3 lacunas restantes pedem as implementações do trait.
- **Conclusão permitida:** sem conclusão. Menos lacuna não virou mais mudança neste teste.
- **Não concluído:** que `Object` e o trait sejam inúteis, ou úteis. O teste mede a troca inteira. O 0/8 de `impl` segue sem repetição.
- **Decisão:** lacunas suspensas, sem descartar os resultados. Isso **não** considera o mecanismo resolvido.

### E10. Homônimos (sem LLM, sem custo)

**Linha de base:**

- **Resultado:** 35 chamadas sem dono único nos alvos dos 7 casos. As 14 declarações editadas no gabarito já estão no envelope, nenhuma por homônimo. A medição desempatou 0. Classes, atribuídas por mim lendo o texto, sem compilador: tipo escrito 8; `self` no próprio trait 3; parâmetro com tipo declarado 9; variável local, de laço ou de campo 14; módulo importado em Python 1.
- **Limite:** Python só aparece com 1, mas em 2 dos 3 casos nenhuma chamada dos alvos foi seguida, então pouco se pode dizer sobre Python.

**Simulação das 11 chamadas sem inferência de tipos, limite de 16 mantido:**

- **Hipótese:** resolver homônimos que já estão escritos melhora o envelope.
- **Método:** a chamada resolvida entra como `called`, e a seleção real decide. Com e sem o tipo dono.
- **Resultado:** 11 de 11 desambiguadas; as outras 24 seguem ambíguas.

| Caso | Entra | Sai | Bytes |
|---|---|---|---|
| `r1` | `Matrix::from`, `Intersections::new` | `is_invertible`, `Bounds::intersects` | 5.373 → 4.369 |
| `r2` | `Intersections::new` | `Bounds::intersects` | 5.512 → 4.530 |
| `r3` | `Intersections::new`, `World::is_shadowed`, `World::color_at` | `Bounds::intersects`, `Matrix` | 6.082 → 5.899 |
| `r5` | `Intersections::new` | `Matrix` | 5.896 → 5.979 |
| Python | nada | nada | iguais |

- **Efeito das substituições:** nenhuma declaração editada se perde. Do envelope gravado saem 1 em `r1` e 2 em `r3`. A contagem decide a troca: `Intersections::new` (4,8 milhões de chamadas) tira `Bounds::intersects` (2,2 milhões). Trazer o dono não muda nenhum envelope. `Intersectable::transform`, o mais pedido, é resolvido e fica de fora por limite de itens. Em 412 mudanças guardadas, o que sai nunca foi editado; o que entra não tinha como ser, porque o validador só permite editar o que foi enviado.
- **Conclusão permitida:** resolver é correto como fato sobre o código; nada indica melhora no envelope.
- **Não concluído:** o efeito no modelo, nunca enviado. O valor de `Bounds::intersects` como contexto: é o teste de saída antecipada de `Groups::intersect`. O tipo de retorno não foi simulado.
- **Decisão:** suspenso, sem implementar.

### E11. Qwen nas mudanças [registrado]

- **Resultado:** 1 de 24 propostas sem problema de forma, e 10 min 21 s gastos. Com pedido menor, 0 de 24 no portão.
- **Decisão:** fora da escada de mudanças até o fine-tune; continua nos fatos, onde passou 9 de 10. Critério de volta na seção "O qwen nas mudanças" do plano.

## 6. Decisões tomadas

### Aprovadas pelo usuário

**Direção atual:**

- Consolidar lacunas e homônimos e avançar para o frontend.
- Próxima frente: evolução do Librarian Studio com Evidence Builder, incluindo as visualizações "Onde pesa" e Pareto.
- Manter o agnosticismo de linguagem e preservar a arquitetura existente.
- Publicação só com pedido do usuário: veja a [seção 11](#11-publicação-dos-passos-1-e-2-da-etapa-3-06112026). `measures` no `logistica-api` aguarda decisão.
- O frontend deve tornar o trabalho do Librarian visível, não mudar o que o Librarian considera evidência.
- Dados experimentais preservados antes de qualquer limpeza, fora do Git.

**Durante o diagnóstico do frontend, não:** alterar contratos de API; implementar cálculos de Pareto; alterar a seleção do envelope; modificar a instrumentação de desempenho; alterar os critérios do portão. Identificar contratos, limitações e mudanças necessárias, apresentar as propostas separadamente e aguardar aprovação.

**Investigações:**

- Lacunas suspensas, sem comprovação de benefício; `Object` e o trait seguem como potencialmente importantes.
- Homônimos suspensos, sem implementar a resolução simulada.
- Correções técnicas de `abb682a` preservadas.
- Condições encerradas por enquanto, extractor sem mudança.
- Tempo: contagem é a medida confiável; linha suspensa até medição independente.
- Cascata qwen → Haiku: não implementar sem evidência de necessidade.

**Envelope:** não aumentar o limite nem alterar a composição sem evidência. Reconhecer uma necessidade potencial não autoriza retirar outra evidência. Sem pesos entre memória, tempo e vazão.

**Método:**

- Nenhuma mudança de arquitetura baseada só em hipótese não validada.
- Corrigir defeito não é oportunidade de mudar comportamento sem evidência.
- Critérios fixados antes de cada experimento e não mudados depois.
- Resultado inconclusivo não vira conclusão.
- A troca entre medidas é decisão humana.

**Frente do Studio:** as decisões das etapas 0 a 7 estão no plano, em [Evidence Builder no Studio](docs/historico/PLANO-ate-18-11.md#evidence-builder-no-studio-etapas-0-a-7-05112026). Aquela seção prevalece sobre a seção 9 deste arquivo.

**Trabalho:** conversa e documentação em português; commits em português, sem acentos (o `99f69f3` tem acentos e prefixo porque o usuário escreveu assim a mensagem); documentação validada antes do commit; código aprovado segue até commit local; push só quando pedido.

### Sugeridas por mim, ainda não aprovadas como desenho

- Ordem interna da tela "Onde pesa" além da especificação da seção 9.
- Reabrir lacunas e homônimos por um caminho específico.
- Guardar uma cópia do arquivo de rodadas fora desta máquina.

## 7. Investigações suspensas

**Não retomar sem evidência nova:**

| Linha | Por que parou | O que reabre |
|---|---|---|
| Condições e invariantes no extractor | Notas geradas e foco não recuperaram o efeito | Regra que escolha o fato certo sem saber a resposta, ou caso novo |
| Tempo na seleção | Quase não muda a seleção, e o tempo é distorcido | Profiler por amostragem, e caso em que a contagem exclua função necessária |
| Busca por lacunas | Sem ganho demonstrado em aprovações | Evidência de que a informação trazida melhora o resultado |
| Resolução de homônimos | Sem sinal de melhora no envelope | Caso em que a identidade resolvida mude o resultado |
| Abrir vaga na rodada de lacuna | Exige decidir o que sai, sem evidência | A mesma evidência das lacunas |
| Cascata qwen → Haiku | O gabarito não mostrou fatos faltando | O gabarito mostrar que faltam fatos |
| Qwen nas mudanças | 0/24 | Critério do plano, depois do fine-tune |
| Notas de papel, lacunas e comportamento ao modelo | Pioraram quando medidas juntas | Teste em partes e menor |

**Hipóteses não comprovadas:** o fato isolado funciona melhor que o acumulado; `Object` e o trait ajudam em método de trait; `impl Intersectable for Object` elimina lacunas.

**Mal-entendido a evitar:** o gabarito só tem mudanças que o envelope já alcançava, e o validador impede editar fora dele. "Nenhuma editada se perde" é sinal fraco, e "nunca foi editado" não prova inutilidade.

**Outras prioridades sem evidência (decisão anterior):** validar a configuração, instrumentar outras linguagens, reconhecer melhor as intenções, tornar configuráveis os critérios do alvo. Só com caso concreto: alvo por pergunta, limite de piora no portão, notas em partes.

## 8. Pendências conhecidas

**Documentação:**

- Corrigir "a ordem é o que vale" em `src/count.rs:9` do `renderer-librarian`, que é **de outro repositório**. No `STUDIO.md` e em `instrument.rs`, já foi corrigido.
- `STUDIO.md` afirma que uma chamada com homônimos entra quando o Medir tudo viu um deles rodar. É verdade, mas nos casos medidos isso desempatou 0 vezes.

**Limitações técnicas conhecidas, sem correção:**

- `main.rs`: a rodada de lacuna não acrescenta nada com 16 declarações. Hoje isso fica registrado. A tela passou a enviar `lacuna_rounds: 0`, como os experimentos, então **o efeito no fluxo real com rodadas não foi medido**.
- O dono de um método de trait não é candidato (alvo `Groups::intersect` traz `Groups`; `Intersectable::intersect` não traz o trait). Causa no código **[não confirmada]**.
- `lacuna_names` extrai palavras em maiúscula do português (`Sem`, `Falta`, `Por`). Inofensivo enquanto não houver símbolo com esse nome.
- Chamadas dentro de macros não são vistas; condições e caminhos alternativos não são extraídos.
- Em Python, não há contagem por função no Medir tudo.
- Uma falha ao ler a medição aparece como "Sem medição" no painel da sugestão: veja a [seção 14](#14-4b-concluída-e-encerrada-06112026).

**Decisões do usuário em aberto:**

- Implementação da evolução da cidade e do fluxograma: depende de autorização própria (seção 12).
- Cópia do arquivo de preservação fora desta máquina: não decidida.
- Commit do `measures` no `logistica-api`: aguardando.
- Limpeza de `Temp/lt-ev`: não limpar ainda.
- Vídeo 13: roteiro em espera, depois do vídeo 12 reprovado **[registrado]**.

## 9. Frontend: de onde veio e o que falta

### A especificação

**Atualização de 06/11/2026: os Passos 1 e 2 da Etapa 3 estão concluídos, e o Passo 3 está encerrado no backend, pela opção A ([seção 12](#12-passo-3-encerrado-e-sequência-da-etapa-3-06112026)); as entregas 4A e 4B do Passo 4 estão concluídas e encerradas ([seção 13](#13-4a-concluída-e-encerrada-06112026) e [seção 14](#14-4b-concluída-e-encerrada-06112026)), e os Passos 6 e 7 foram feitos ([seção 16](#16-passos-6-e-7-feitos-06112026)).** Já estão implementados: cobertura por função; ordem por contagem observada, com o tempo como referência; sem `FREQUENT` nem alvo automático; `count_kind` por origem; `counted_no_passes` e `zero_evidence`; resolução do alvo antes da medição, por símbolo; e, no cartão, a origem da resposta decidida por `execution`. O contrato está no [STUDIO.md](STUDIO.md#medição-capacidade-não-é-cobertura), e as baterias de tela desta etapa estão em [crates/librarian-studio/tests/tela](crates/librarian-studio/tests/tela/README.md), junto com `etapa2c.js`: fora do `cargo test`, porque precisam de um ambiente e de dados que não estão no Git. O texto abaixo guarda o contexto de antes disso.

**Etapas 0, 1 e 2 feitas; a Etapa 3, "Onde pesa", está em andamento, com os Passos 1 a 4 concluídos, os Passos 6 e 7 feitos, e o Passo 5 adiado para a etapa própria.** As decisões estão no plano, em [Evidence Builder no Studio](docs/historico/PLANO-ate-18-11.md#evidence-builder-no-studio-etapas-0-a-7-05112026), e prevalecem sobre esta seção, que guarda o contexto anterior a elas. O contrato do registro e as limitações estão no [STUDIO.md](STUDIO.md#registro-da-seleção-originselection-formato-2). **A Etapa 4 está em andamento desde 07/11/2026: veja a [seção 17](#17-etapa-4-evidence-builder-07112026-a-11112026).**

### Por que esta evolução

As investigações mostraram que os dados de seleção e os trade-offs da decisão precisam ficar visíveis e honestos, não que falte mais conteúdo no envelope. A seleção já grava o orçamento, o que ficou de fora e as lacunas, e a escada já mede ganhos e pioras, mas a tela ainda sugere conclusões que os dados não sustentam: uma ordem por tempo distorcido e uma "vencedora" pela maior %.

### Como se conecta à arquitetura

A interface **mostra** decisões de seleção e medição; não as toma. Seleção, investigação, medição e decisão continuam separadas. A interface é uma camada sobre o que `/api/envelope/suggest`, `/api/ladder`, `/api/hotspots` e `/api/executions` oferecem.

### Ordem aprovada

Uma etapa por vez, com testes, diff e resultados apresentados antes do commit, que depende de autorização separada. O diagnóstico já foi feito e aprovado.

### "Onde pesa": especificação aprovada

Detalhada depois na Etapa 3 do plano:

| Informação | Apresentação |
|---|---|
| Quantidade de chamadas | Ordenação principal |
| Tempo instrumentado | Referência experimental |
| Confiabilidade | Aviso explícito: inflado pela contagem, pode mudar a ordem entre funções |

**O que existe hoje [confirmado]:** sem medição, `GET /api/hotspots` responde 404; com ela, devolve `rows` com `at`, `calls`, `cpu_ms`, `symbol`, `name`, `path`, `start_line`, `end_line`, `callers`, `calls_out`, além de `limits` e `measures`. As linhas vinham ordenadas por `cpu_ms` em `instrument.rs`; desde o Passo 2 vêm pela contagem observada, depois o tempo e o lugar, como desempate.

**Usos de `cpu_ms` em `index.html`, a revisar.** As linhas abaixo são de antes da Etapa 2 e mudaram: procure por `cpu_ms`.

- Linha 3040: etiqueta no ranking.
- Linhas 3374 e 3380, e 3814 e 3934: calor nos mapas.
- Linha 3423: top 8.
- Linhas 1686 e 3933: textos de dica.

Antes do Passo 2, `hot.ranking` dizia "Mais tempo, contado pelo projeto", e a "mais pesada" (`hot.hottest`) também alimentava **Separar para o qwen**. Hoje o título é "Maior contagem observada, com o tempo instrumentado como referência", e a pessoa escolhe o alvo de Separar numa lista.

**Decisão do usuário:** isso não é só uma mudança de ordenação. O diagnóstico precisa mapear todos os consumidores da medição (ordenação, mapas de calor, top 8, Separar para o qwen), porque alterar só um pode produzir uma visualização contraditória.

### Pareto: não implementar antes de medir

**Direção do usuário:** quando há trocas entre medidas, não eleger vencedora; mostrar lado a lado as alternativas não dominadas, com ganhos e pioras de cada uma; identificar as dominadas; sem pesos; escolha final do usuário.

**Decisão do usuário:** sem dominância nem seleção Pareto por enquanto; a frequência do conflito já foi medida (tabela na seção "Etapas 0 e 1" do plano). Propostas lado a lado na Etapa 6. Separar **dados de medição, cálculo de dominância, critério de aprovação e visualização**: uma escolha já embutida no backend não deve virar uma suposta verdade matemática sobre dominância.

**Dados que existem [confirmado]:**

- Cada passo da escada traz `outcome` (`passed`, `passed_smaller`, `refused_by_gate`…), `detail.gain_pct`, `detail.worse` (lista de **textos**), `detail.executions` (ids) e `detail.why`.
- Os números por medida estão em cada `execution.json`: `comparison.stats` com `change_pct` e `ranges_overlap`, além de `measures`.

**Não existe, e não deve ser presumido:**

- Contrato de API com as medidas lado a lado por proposta, ou cálculo de dominância.
- Não verifiquei se `GET /api/executions` já devolve `comparison.stats` completo. Confira antes de desenhar.
- `passed` e `passed_smaller` já embutem a escolha pela maior %. Mudar isso muda o backend e o significado do registro: combine antes com o usuário.

**Cuidados de dados:**

- Calcular dominância exige decidir o que conta como diferença. Regra do portão hoje: ao menos 10%, com faixas separadas; por ela, os +11% de memória de B na E4 não contam como piora.
- Medidas sem declaração de direção só têm sentido para tempo e contagem. O `renderer` não declara memória: lá o Pareto é pobre.
- A frequência do conflito foi medida nas rodadas arquivadas: 1 caso sem dominância em 141 envios com 2 ou mais aprovadas. Isso **não prova que o conflito seja raro**, porque só 1 desses envios é em Python.
- O que mudou de propósito não pode ser julgado pela própria medida (contadores, tempo em comando que conta).

### O que influencia o frontend

- A piora não recusa; a decisão é da pessoa.
- A distinção entre evidência, hipótese e fato precisa aparecer na tela: aprovado no portão não é fato.
- A rodada sem espaço já aparece (`ladder.noRoom`).
- Sem pesos arbitrários, sem pontuação composta e sem nova heurística de seleção embutida na interface.

### Cuidados de implementação

- `index.html` é um arquivo único, com textos em pares pt/en (`t('chave')`) e tokens de cor. Temas por `prefers-color-scheme` e `data-theme`: verifique os dois temas e a tela estreita.
- **Não testar em raiz em uso:** `askLadder` salva settings, e já desligou `allow_hypothesis` numa cópia. Use `LIBRARIAN_ROOT` e `LIBRARIAN_COPIES` isolados, porta própria e projeto descartável.
- **O Studio serve a página de `LIBRARIAN_ROOT/crates/librarian-studio/web/index.html`.** Numa raiz isolada, copie o arquivo atual antes de testar, ou verá uma versão antiga.
- Teste de tela: Playwright, com o caminho em `LIBRARIAN_TESTE_PLAYWRIGHT`, e o padrão de escada simulada em `ui4.js` e `ui5.js`, no arquivo de rodadas. Projeto novo precisa de `POST /api/project` e `POST /api/generate` antes de abrir.
- `cargo` sempre com `CARGO_TARGET_DIR` fora do projeto se houver studio ou experimento rodando.
- Manter o agnosticismo: o texto da tela não pode supor Rust, `matrix.rs` ou `renderer`.

### Não repetir sem justificativa

As investigações da seção 7, e também: ordenar a seleção por tempo; trocar `Bounds::intersects` por `inverse`; "top N do projeto" como seleção; comparar só propostas aprovadas para achar lacunas; notas acumuladas ao modelo.

## 10. Instruções para a nova sessão

1. **Leia este arquivo inteiro.** Depois, no plano até 18/11 ([docs/historico/PLANO-ate-18-11.md](docs/historico/PLANO-ate-18-11.md)), da linha 1116 em diante, quando precisar de tabelas completas.
2. **Confirme o estado, sem confiar neste texto:**
   - `git status -sb`
   - `git log --oneline origin/main..HEAD`, que lista o que ainda não foi publicado. Com a publicação da [seção 15](#15-publicação-da-4a-da-4b-e-da-apresentação-06112026), ele vem vazio, salvo trabalho posterior, como o da [seção 16](#16-passos-6-e-7-feitos-06112026).
   - Antes de qualquer limpeza de temporários, verifique os dados: `sha256sum -c MANIFESTO.sha256 --quiet` em `.studio/arquivo-copias/rodadas-20261104` e a existência de `root` e `rootpy` no scratchpad. Não remova, sobrescreva nem reorganize nada sem aprovação.
3. **Leia a seção [Evidence Builder no Studio](docs/historico/PLANO-ate-18-11.md#evidence-builder-no-studio-etapas-0-a-7-05112026) do plano** e o [contrato no STUDIO.md](STUDIO.md#registro-da-seleção-originselection-formato-2). Antes de usar as baterias de tela, confira se o scratchpad citado na seção 3 ainda existe. Se não existir, restaure pelo `LEIA-ME.md` do arquivo de preservação, sabendo que esse caminho ainda não foi testado.
4. **A Etapa 3 segue como na [seção 16](#16-passos-6-e-7-feitos-06112026), e a Etapa 4 está em andamento, sem commit ([seção 17](#17-etapa-4-evidence-builder-07112026-a-11112026)). Nada além do que a seção 17 lista como próximo está autorizado. Não inicie a evolução da cidade, o resumo "N de M", o fallback único nem a auditoria funcional do frontend sem autorização do usuário.** Apresente testes, diff e resultados antes do commit de cada etapa, e aguarde.
5. **Não faça push** sem pedido.
6. **Não retome investigações suspensas** sem evidência nova e decisão do usuário. Experimento novo exige hipótese e critérios fixados antes, e o usuário costuma decidir etapa por etapa.
7. **Relate os resultados com honestidade:** o que falhou, o que não foi testado e o que é só hipótese. Corrija números errados, inclusive os seus e os do usuário: este trabalho já teve correções assim (B com "+34%" de memória, orçamento de "16 KB").

### Informações não confirmadas

- O custo total da frente inteira.
- A origem de `LOGISTICS.md` no `logistica-api`.
- Se `GET /api/executions` devolve as comparações completas.
- A causa no código da assimetria do dono de método de trait.
- O efeito das rodadas de lacuna num fluxo real, nunca medido.
- A troca manual de tema por `data-theme`, não testada nestes ajustes.
- Se a restauração do arquivo de preservação reproduz as baterias: nunca executada.

## 11. Publicação dos Passos 1 e 2 da Etapa 3 (06/11/2026)

**Registro de antes da publicação; o estado atual está na [seção 12](#12-passo-3-encerrado-e-sequência-da-etapa-3-06112026).** Quando este texto foi escrito, os Passos 1 e 2 estavam concluídos, e a publicação em `origin/main` estava planejada, porque acontece depois do commit que traz este registro, cujo hash não cabia aqui.

Naquele momento, o Passo 3 não tinha sido iniciado.

**Os 14 commits, do mais antigo ao mais novo, sobre `7868912`:**

| Hash | Mensagem |
|---|---|
| `7b13e133ad2a52560fa3fee27a17e9291efe2bcd` | Docs: portao com o modelo, condicoes e invariantes medidas, e notas geradas como proximo experimento |
| `9fcb05e0a1efd63ee1983761679e48fdbc6b1413` | Docs: notas geradas por script e condicao da funcao perguntada, linha de condicoes encerrada por enquanto |
| `02d7ae6cae6a536af9481dcf1dbfac5e7109fbfa` | Docs: tempo na selecao, contagem como medida confiavel e correcao da tela Onde pesa para o front |
| `bab430adf06973f5b712369ce498d06797d06114` | Docs: o que ficou de fora do envelope, classes de ausencia e defeitos no caminho da lacuna |
| `e07c038b98cfe59778a64730ea5a57d6945ae6bc` | Docs: Object e o trait em r5 sem conclusao, lacunas suspensas e decisoes sobre os defeitos |
| `abb682acc799af4e6dcbe37bf67b84cb9718a228` | Lacuna: metodo de dois traits nao e adivinhado, trait pedido nao e filtrado e a rodada sem espaco aparece no registro |
| `df43138ef2b79e3c12f6fa89ce1afa587a7dc325` | Docs: defeitos da lacuna corrigidos, rodada sem espaco e linha de base dos homonimos |
| `94e22383ce8bdf39e98314e1a3b3ab1ff8b9b292` | Docs: homonimos suspensos, sem implementar, e diagnostico antes da proxima frente do Studio |
| `8b575eebee6c3a029f36c4ab90b247482d1541d8` | Docs: contexto de continuidade para a proxima sessao |
| `99f69f3deff704ff6be8b33812600b6babc301ab` | feat(studio): finalize etapa 2 de decisões e proveniência |
| `a530017643e5c6d8f25e1a2dbec11c4dc0f943f8` | Docs: contrato de selecao, limitacoes e etapas do Evidence Builder |
| `746fa31e9b2afa5cc99b956686f83f34d40f7e00` | Docs: estado apos o commit da documentacao e o arquivo da Etapa 2 |
| `47337e9d66db43d3554bacebedeb53c625ea68b7` | feat(studio): passos 1 e 2 da etapa 3, cobertura, contagem e resolucao do alvo |
| `9659de5fc8ae479249f26a5974f6cf75ea56b92a` | Test: preserva regressoes dos passos 1 e 2 |

O histórico local foi reescrito antes da primeira publicação, só em documentos, e os seis últimos commits mudaram de hash. O mapa do antigo para o novo está em `.studio/arquivo-copias/reescrita-hashes-20261106`, fora do Git, com os originais dos 15 arquivos do arquivo de preservação em que as citações foram trocadas.

**Testes em `9659de5`:**

| Verificação | Resultado |
|---|---|
| `cargo test --workspace --locked` | 240 aprovados, 0 falhas, 2 ignorados |
| `tela-cartao.js` | 37 de 37 |
| `tela-zero.js` | 29 de 29 |
| `tela-origem.js` | 32 de 32 |
| `tela-nomes.js` | 12 de 12 |
| `tela-passo2.js` | 9 de 9 |
| `etapa2c.js` | 45 de 45 |
| Gabarito, 2 execuções | Mesma saída da base |

`etapa2.js` (66 de 66) e `etapa2b.js` (75 de 75) rodaram com o mesmo código antes da reescrita, e as mutações, antes disso; não foram repetidas depois dela. O código e as baterias são idênticos, byte a byte, aos testados antes da reescrita.

**Limitações conhecidas e aceitas:**

- Uma pergunta sem intenção reconhecida ("Bom dia"), com medição, traz `execution` na API, sem cartão na tela.
- Expressões como `v1.2` e `e.g.` podem ser lidas como nomes qualificados inexistentes (veja o [STUDIO.md](STUDIO.md#onde-pesa-medir-tudo)).
- Em `otimize Tuple::dot e cross`, só o nome qualificado é reconhecido.
- As baterias de tela ficam fora do `cargo test` e do CI, dependem de cerca de 1,2 GB de dados fora do Git e nunca rodaram em outra máquina. `etapa2.js` e `etapa2b.js` continuam fora do repositório.
- `cargo fmt --check` já falhava antes destes passos; o projeto não usa formatação global.
- O histórico de `8b575ee` a `9659de5` ainda contém um caminho local nesta linha de contexto; o nome de usuário já era público em `origin/main`, e o texto atual não o traz.

**Para reconstruir o contexto na próxima sessão:**

1. Leia este arquivo, a [seção do plano](docs/historico/PLANO-ate-18-11.md#evidence-builder-no-studio-etapas-0-a-7-05112026) e a memória de trabalho, que guarda as preferências de commit do usuário.
2. Rode `git fetch origin`, `git status -sb` e `git log --oneline origin/main..HEAD`. Com a publicação feita, a lista vem vazia e `git rev-parse HEAD origin/main` dá o mesmo hash, salvo trabalho posterior. Sem ela, a lista traz estes 14 commits e o que traz este registro.
3. Nunca use force push, e não publique sem pedido do usuário.
4. O que está autorizado depois deste registro está na seção 12.

## 12. Passo 3 encerrado e sequência da Etapa 3 (06/11/2026)

**Decisão formal do usuário: opção A. O Passo 3 está encerrado como entregue, concluído no backend, e não será ampliado com a apresentação visual dos estados.** A decisão não mudou código. O registro completo está no [plano](docs/historico/PLANO-ate-18-11.md#passo-3-encerrado-pela-opção-a-06112026).

**Por que não a opção B:** ela levaria o estado por função à tela da sugestão. Isso misturaria a responsabilidade do backend, que fornece os estados, com a da interface, que os apresenta, e essa apresentação é do Passo 4. A falta na tela é uma pendência do Passo 4, não um motivo para reabrir o Passo 3. A entrega 4B do Passo 4 planeja esse painel para o futuro: ela não reabre o Passo 3 nem implementa a opção B retroativamente.

### O que cada passo entregou

| Passo | O que resolveu | Onde |
|---|---|---|
| 1, cobertura | Ausência de cobertura deixou de ser confundida com ausência de execução: estados e motivos de exclusão, com `not_recorded`, `not_classified` e nulo distintos, e `bodiless` só em Rust | `47337e9` |
| 2, interpretação | Frequência deixou de se confundir com custo: ordem por contagem observada, sem `FREQUENT` nem alvo automático, com `count_kind`, `counted_no_passes`, `zero_evidence` e o alvo resolvido antes da medição | `47337e9` |
| 3, estado por função | A API fornece o que distingue ausência de cobertura de ausência de contagem: `coverage_state`, `exclusion_reason`, `zero_evidence` e o resumo `coverage`, ao lado de `calls: null`. Foi entregue junto com o Passo 1 | `instrument.rs:457`, teste em `store.rs:2411` |

### Verificações

| Verificação | Resultado | Situação |
|---|---|---|
| `git ls-remote`, `HEAD` e `origin/main` | Todos em `f6f3958`, sem divergência | Executada em 06/11/2026, nesta decisão |
| `cargo test --workspace --locked`, em `f6f3958` | 240 aprovados, 0 falhas, 2 ignorados | Executada em 06/11/2026, nesta decisão |
| `a_suggestion_tells_counted_without_running_from_no_coverage_and_selects_the_same`, Python alvo × Rust contado sem passagens, com a mesma seleção | Aprovado | Executada em 06/11/2026, nesta decisão |
| Gabarito, 1 execução, em `f6f3958` | Saída sem a linha de resultado com SHA-256 `2714b840…`, igual à linha de base | Executada em 06/11/2026, nesta decisão |
| Baterias de tela, 164 de 164, em `9659de5` | Veja a seção 11 | Resultado histórico, não repetido |
| `etapa2.js` e `etapa2b.js` | Veja a seção 11 | Resultado histórico, não repetido |

As limitações da seção 11 continuam valendo.

### Sequência e autorizações

A sequência vem da tabela do §11 do diagnóstico da Etapa 3, aprovado na sessão `661d7681`, cuja transcrição fica fora do repositório. O plano guarda a [tabela completa](docs/historico/PLANO-ate-18-11.md#sequência-dos-passos). Ela é a sequência aprovada, e **não autoriza a implementação de nenhum passo**.

| Trabalho | Estado | O que falta |
|---|---|---|
| Passo 4: 4A, estados na tela Onde pesa, e 4B, estados no painel da sugestão | 4A e 4B concluídas e encerradas ([escopo](docs/historico/PLANO-ate-18-11.md#escopo-registrado-do-passo-4), [resultado da 4A](docs/historico/PLANO-ate-18-11.md#resultado-da-4a-06112026) e [resultado da 4B](docs/historico/PLANO-ate-18-11.md#resultado-da-4b-06112026)) | Nada; as pendências estão na [seção 14](#14-4b-concluída-e-encerrada-06112026) |
| Resumo "N de M" | Fora do escopo definido do Passo 4 | Decisão específica; não está autorizado |
| Fallback único da tabela do §11 | Fora de 4A e de 4B; pendência sem etapa atribuída | Não implementar nem atribuir a outra etapa |
| Passo 5: cidade e fluxograma | Adiado para uma [etapa própria de evolução visual](docs/historico/PLANO-ate-18-11.md#evolução-visual-cidade-e-fluxograma-planejada-em-06112026) | Autorização e os pontos em aberto listados lá |
| Passo 6: conversa e Python | Feito, em `94089d3` ([seção 16](#16-passos-6-e-7-feitos-06112026)) | Nada |

**Encerramento e autorização de 06/11/2026:** a revisão documental do Passo 3 está encerrada, e o Passo 4 da Etapa 3 está autorizado para execução numa sessão posterior, com as entregas 4A e 4B separadas. O nome oficial é Passo 4 da Etapa 3, para não confundir com a Etapa 4 (Evidence Builder) do plano. **A 4A foi revisada e encerrada; a 4B continua autorizada e é a próxima atividade.** A autorização não vale para o Passo 6, a evolução da cidade, o mapa, o fluxograma, o resumo "N de M", a instrumentação, a arquitetura de evidências, uma Fase 5 nem refatorações fora do escopo. A evolução da cidade continua sendo uma iniciativa futura, sem etapa atribuída. **Atualização: a 4B também foi encerrada ([seção 14](#14-4b-concluída-e-encerrada-06112026)), e o Passo 4 está concluído.** Esta autorização não alcança nada além dele.

**Decisão de planejamento não é autorização.** A direção visual e semântica da cidade está aprovada para planejamento. Implementá-la depende de autorização expressa, e ela não deve ser antecipada no Passo 4. Até lá, a cidade, o fluxograma, os dados da API e a instrumentação ficam como estão.

**Semântica a preservar, decidida pelo usuário:**

- **Três estados:** passagens observadas, sem passagens observadas e ausência de cobertura. O nome é "função contada sem passagens observadas", porque ausência de contagem observada não é prova de ausência de execução. Os casos são diferentes: um zero pode vir de um contador zerado depois de uma execução anterior, e em `no_line` nenhuma linha foi impressa.
- **Top 8 por `cpu_ms` observado.** Não são as oito de maior custo próprio: `cpu_ms` é inclusivo e influenciado pela contagem.
- **Altura:** só representa número de chamadas quando `count_kind` for `entry_calls`. A legenda respeita a origem e a semântica de cada métrica.
- **Prédio como arquivo:** a navegação planejada fica, e a semântica continua em aberto. Nenhuma agregação por arquivo está definida, e `cpu_ms` não se soma, porque a soma pode duplicar valores.

**Efeito aceito do adiamento:** a seção 9 registra que alterar só um consumidor da medição pode produzir uma visualização contraditória. Com a cidade adiada, o ranking segue a contagem, enquanto a cor e o top 8 da cidade seguem `cpu_ms`. A legenda diz que a cor é o tempo.

## 13. 4A concluída e encerrada (06/11/2026)

**Registro do encerramento da 4A. Depois dele, a 4B também foi encerrada: o estado atual está na [seção 14](#14-4b-concluída-e-encerrada-06112026), e o que aqui fala da 4B como futura é histórico.**

**A entrega 4A do Passo 4 da Etapa 3 está concluída, revisada e encerrada. A próxima atividade é a 4B, autorizada e não iniciada.** O registro completo está no [plano](docs/historico/PLANO-ate-18-11.md#resultado-da-4a-06112026).

**Git:** commit `1908658`, sobre `f6f3958`. Era local no encerramento e foi publicado depois: veja a [seção 15](#15-publicação-da-4a-da-4b-e-da-apresentação-06112026). Mensagem: `feat(studio): conclui visualizacao de cobertura da 4A`.

### O que a tela faz

Só `web/index.html` mudou. Backend, API, contrato de cobertura, instrumentação, envelope, seleção, portão, `origin.selection` e gabarito não mudaram.

- **Ranking:** as mesmas linhas, com o título "Top 20 · Maior contagem observada, com o tempo instrumentado como referência" e uma nota dizendo que não são todas as funções com passagens observadas.
- **Duas listas recolhidas, sem quantidades:** "Contadas sem passagens observadas" (`counted_no_passes`, com o motivo de `zero_evidence`) e "Sem cobertura" (`no_coverage`, com o motivo de `exclusion_reason`). As duas vêm de `coverage.functions`, em `/api/hotspots`, e mantêm o estado aberto ou fechado quando a pessoa escolhe uma função em Separar.
- **Correção semântica depois da revisão:** a nota de `counted_no_passes` falava em contador zerado, o que não explica `no_line`. O texto final é "A medição não registrou passagens por estas funções. Isso não prova que elas não executaram: ausência de contagem observada não é prova de ausência de execução." Em inglês: "The measurement recorded no passes through these functions. This does not prove they did not execute: absence of observed counts is not proof of absence of execution."
- **"236 funções contadas pelo Librarian" não mudou:** 236 é quem recebeu a instrução, e `coverage.counted` (244) soma quem imprimiu contagem própria sem recebê-la. Veja o [plano](docs/historico/PLANO-ate-18-11.md#resultado-da-4a-06112026).

### Testes

| Verificação | Resultado | Situação |
|---|---|---|
| `tela-4a.js` | 30 de 30 | Executada no encerramento, no código do commit |
| `tela-cartao.js`, `tela-zero.js`, `tela-origem.js`, `tela-nomes.js`, `tela-passo2.js`, `etapa2c.js` | 37, 29, 32, 12, 9 e 45, todas sem falha | Executadas no encerramento, no código do commit |
| Erros de página e envios a LLM | Nenhum, pelas próprias baterias; 844 `send.json` antes e depois, nas cópias de teste | Executada no encerramento |
| 6 mutações contra `tela-4a.js` | 6 detectadas | Antes da correção semântica, que só trocou um texto; não repetidas |
| `cargo test --workspace --locked` | 240 aprovados, 0 falhas, 2 ignorados | Durante a 4A; não repetido, porque nada em Rust mudou |
| Gabarito | SHA-256 `2714b840…`, igual à linha de base | Durante a 4A; não repetido, porque ele não mudou |
| `etapa2.js`, `etapa2b.js` e `etapa2-modelo.js` | Não executados na 4A | Veja a seção 11 |

### Limitações e pendências

As limitações estão no [plano](docs/historico/PLANO-ate-18-11.md#resultado-da-4a-06112026). Seguem pendentes, cada uma com decisão ou autorização própria: o resumo "N de M", o fallback único, o filtro de 5%, a evolução da cidade e do fluxograma, e o Passo 6. A limitação de `calls: null` em `origin.selection` continua sem passo atribuído.

### Como retomar

1. Faça a conferência da [seção 10](#10-instruções-para-a-nova-sessão) e leia, no plano, o [escopo do Passo 4](docs/historico/PLANO-ate-18-11.md#escopo-registrado-do-passo-4) e o [resultado da 4A](docs/historico/PLANO-ate-18-11.md#resultado-da-4a-06112026).
2. A 4B é o painel da sugestão e usa o `coverage_state` que cada item de `/api/envelope/suggest` já traz (`instrument.rs:467`). Essa rota não traz `coverage.functions` (`instrument.rs:474`), e o painel não deve depender de `/api/hotspots`. O painel está em `index.html`: procure `envelope.out.`.
3. **Cuidado para a 4B:** sem medição, o item vem com `no_coverage` e motivo nulo. O texto não pode dizer que a medição não cobriu a função quando não houve medição.
4. **Baterias de tela:** veja o [README](crates/librarian-studio/tests/tela/README.md), rodam em mais de 10 minutos e gravam na raiz, por isso use cópias. Os dados são temporários:
   - raízes: `base/rs` e `base/py`, no scratchpad da sessão `661d7681-430a-4afe-b472-97d59768c138`;
   - projeto Python: `iso/pyproj/backend`, no scratchpad da sessão `465f4d1b-0d17-4dff-903f-d3077bb3c8f4`;
   - Playwright: `librarian-ferramentas/pw/node_modules/playwright-core`, na pasta de vídeos do usuário;
   - antes de rodar, copie o `web/index.html` atual para as raízes de teste.

   Se esses dados tiverem sumido, avise o usuário antes de reconstruir: a restauração pelo arquivo de preservação nunca foi executada.
5. **Não faça push** sem pedido, e apresente testes, diff e resultados antes de qualquer commit da 4B.

## 14. 4B concluída e encerrada (06/11/2026)

**A entrega 4B do Passo 4 da Etapa 3 está concluída, revisada e encerrada. Com ela, o Passo 4 está concluído.** O registro completo está no [plano](docs/historico/PLANO-ate-18-11.md#resultado-da-4b-06112026). **Nenhuma atividade seguinte está autorizada.** Na [sequência aprovada](docs/historico/PLANO-ate-18-11.md#sequência-dos-passos), o próximo é o Passo 6, não iniciado, que aguarda revisão e autorização; o Passo 5 segue adiado para a etapa própria de evolução visual, e o Passo 7 não tinha sido iniciado. **Atualização: os Passos 6 e 7 foram feitos depois, na [seção 16](#16-passos-6-e-7-feitos-06112026).**

**Git:** commit `3b64f66`, sobre `1908658`. Era local no encerramento e foi publicado depois: veja a [seção 15](#15-publicação-da-4a-da-4b-e-da-apresentação-06112026). Mensagem: `feat(studio): conclui estados de cobertura no painel da sugestao`.

### O que a tela faz

Só `web/index.html` mudou no produto. Backend, API, contrato de cobertura, instrumentação, envelope, seleção, portão, `origin.selection` e gabarito não mudaram.

- **Cada item** da sugestão e de "Ficaram de fora" mostra o `coverage_state` de `/api/envelope/suggest`: "contada com passagens observadas", "contada sem passagens observadas", com o motivo de `zero_evidence`, ou "sem cobertura", com o motivo de `exclusion_reason`. O painel não pede `/api/hotspots` para isso.
- **Sem medição:** nenhuma etiqueta por item, só o aviso geral que já existia. O servidor diz `no_coverage` com motivo nulo nesse caso, e a tela não repete isso.
- **Campo ausente, nulo ou estado desconhecido:** nenhuma etiqueta. Motivo desconhecido ou nulo: só o estado.
- **Nota aprovada:** "Passagens observadas não dizem que a função pesa. Ausência de passagens observadas não prova que ela não executou, e ausência de cobertura não diz se ela executou." Em inglês: "Observed passes do not say the function is heavy. Absence of observed passes does not prove it did not execute, and absence of coverage does not say whether it executed."
- **Correção da revisão final:** a seleção lê a mediana da linha contada, e o estado, qualquer repetição. Com `did_not_run` e outro estado (mediana zero com máximo positivo, ou `shared_line`), a linha dizia "sem passagens na medição" ao lado de um estado que a contradizia. Agora diz "a seleção não encontrou contagem acima de zero", em inglês "the selection found no count above zero". Só o texto da tela muda: o registro continua com `did_not_run`.

### Testes, na página final

| Verificação | Resultado | Situação |
|---|---|---|
| `rodar.js` | Saída 0, 249 de 249: `tela-cartao.js` 37, `tela-zero.js` 29, `tela-origem.js` 32, `tela-nomes.js` 12, `tela-passo2.js` 9, `tela-4a.js` 30, `tela-4b.js` 55, `etapa2c.js` 45 | Executada na revisão final |
| `etapa2.js`, `etapa2b.js` | 66 de 66 e 75 de 75 | Executadas na revisão final, antes de a bateria nova ser corrigida, com a mesma página |
| `etapa2-modelo.js` | 5 sequências, 350 passos, 0 ocorrências | Idem |
| 16 mutações contra `tela-4b.js` | 16 detectadas | Executadas na revisão final |
| Projeto misto real | Medição nova numa cópia: Python com `language_not_supported`, Rust com os outros dois estados | Sonda fora do repositório, na revisão final |
| `cargo test --workspace --locked` | 240 aprovados, 0 falhas, 2 ignorados | Executada na revisão final |
| Gabarito | SHA-256 `2714b840…`, igual à linha de base | Executada na revisão final |

### Limitações que permanecem

- **Falha ao ler a medição aparece como "Sem medição" no painel.** `suggest_envelope` descarta o erro de `hotspots` com `.ok()`, então a sugestão chega com `measured: false` e `coverage: null`, enquanto Onde pesa diz que não conseguiu ler a medição. Decisão do usuário: não corrigir nesta etapa. A solução adequada exige distinguir os estados no backend. Sem etapa atribuída.
- **Mediana zero com máximo positivo:** a linha diz "com passagens observadas" sem mostrar número, porque o número exibido é a mediana.
- **Dados:** mediana zero e `shared_line` foram exercitados com registros editados, porque não ocorrem nas duas rodadas reais do `renderer`. Python puro com medição não foi testado: o Medir tudo exige fonte Rust.
- **Tela estreita:** o painel transborda 96 px em 420 px de largura, com ou sem a 4B. É anterior a ela, e a bateria só garante que não piorou.
- **`tela-4b.js`** depende do ambiente e dos dados locais, e não faz parte do `cargo test` nem do CI.

Seguem pendentes, cada um com decisão ou autorização própria: o resumo "N de M", o fallback único, o filtro de 5%, a evolução da cidade e do fluxograma, o Passo 6 e a limitação de `calls: null` em `origin.selection`.

### Como retomar

1. Faça a conferência da [seção 10](#10-instruções-para-a-nova-sessão) e aguarde a instrução do usuário: nada depois do Passo 4 está autorizado.
2. O ambiente das baterias está na [seção 13](#13-4a-concluída-e-encerrada-06112026), item 4 de "Como retomar", e os dados continuam temporários.
3. **Não faça push** sem pedido.

## 15. Publicação da 4A, da 4B e da apresentação (06/11/2026)

**A 4A, a 4B e a apresentação pública estão publicadas em `origin/main`.** O push foi normal, de `f6f3958` a `5136dbe`, sem reescrita de histórico, com autorização do usuário. Nessa publicação, nada depois do Passo 4 estava autorizado; depois disso, veja a [seção 16](#16-passos-6-e-7-feitos-06112026). Continua valendo a [seção 14](#14-4b-concluída-e-encerrada-06112026).

| Commit | Conteúdo |
|---|---|
| `1908658` | 4A: estados de cobertura em Onde pesa |
| `3b64f66` | 4B: estados de cobertura no painel da sugestão |
| `5136dbe` | READMEs públicos em português e inglês e capas em `docs/img/librarian-cover-*.jpg`. CI concluído com sucesso ([execução](https://github.com/ViniciusSJV/librarian/actions/runs/42291412877)) |

Um commit posterior traz esta revisão documental, cujo hash não cabe aqui: confira no `git log`.

**Frase das capas, decisão do usuário:** "Knowledge is the most powerful of goods" e "O conhecimento é o mais poderoso dos bens" ficam como elemento da identidade visual, sem atribuição literária, porque a origem em *Snow Crash* não foi confirmada. A linha "— Snow Crash" foi retirada das duas capas, e o resto das imagens foi preservado.

**Capas antigas:** `docs/img/cover-en.png`, `docs/img/cover-pt-BR.png`, `docs/img/capa.png` e `docs/img/cover/` não têm referência fora da própria pasta: a única é `cover.html` apontando para `base.png`. Elas foram mantidas, e a exclusão depende de uma revisão separada.

## 16. Passos 6 e 7 feitos (06/11/2026)

**Comece por aqui. Os Passos 6 e 7 da Etapa 3 estão feitos e verificados: o código e os testes estão em `94089d3`, sobre `311f5dd`, e estes documentos, no commit seguinte, cujo hash não cabe aqui.** Para saber o que já foi publicado, rode `git log --oneline origin/main..HEAD`, sem confiar neste texto. O registro completo está no [plano](docs/historico/PLANO-ate-18-11.md#resultado-dos-passos-6-e-7-06112026).

### Como retomar

1. `git status -sb`: não deve mostrar arquivos sem commit. Se houver, pare e pergunte.
2. **Aguarde a instrução do usuário.** Não há etapa técnica autorizada.
3. A escolha do que vem é do usuário: [Etapa 4, Evidence Builder](docs/historico/PLANO-ate-18-11.md#etapa-4-evidence-builder), ou a [evolução visual](docs/historico/PLANO-ate-18-11.md#evolução-visual-cidade-e-fluxograma-planejada-em-06112026). **Nenhuma das duas está autorizada.**

### O que mudou

- **D6:** `/api/commands` traz `manual_counts`, ao lado de `measure_all`: uma entrada por comando, com `command`, `execution` e `lines`, lida da execução que Investigar mostra para ele: a última sem mudança, com qualquer resultado. Se ela falhou ou é um Medir tudo, o comando não entra. `null` se os registros não puderem ser lidos. A conversa, Onde pesa e o painel da sugestão dizem que as contagens existem e apontam Investigar, em vez de dizer só que não há medição.
- **D7:** com uma edition legível e sem fonte Rust, a falta de `instrument` agora responde `no_source`. Sem edition ou com edition ilegível, continua `no_instrument`. Para isso, `/api/commands` passou a carregar o catálogo em projetos sem `instrument`: veja a auditoria, abaixo.
- **D8:** nada no produto; a correção era do Passo 2, e agora há um teste que a fixa.

**Invariantes conferidos:** `/api/hotspots` continua 404 sem Medir tudo; contagem manual fora de `rows` e de `measured`; sem instrumentação de Python; seleção, envelope, portão, `origin.selection`, 4A e 4B sem mudança; gabarito com SHA-256 `2714b840…`.

### Testes, no código de `94089d3`

| Verificação | Resultado |
|---|---|
| `cargo test --workspace --locked` | 245 aprovados, 0 falhas, 2 ignorados |
| Gabarito | SHA-256 `2714b840…`, igual à linha de base |
| `rodar.js` | Saída 0, 285 de 285: `tela-cartao.js` 37, `tela-zero.js` 29, `tela-origem.js` 32, `tela-nomes.js` 12, `tela-passo2.js` 9, `tela-4a.js` 30, `tela-4b.js` 55, `tela-passo6.js` 36, `etapa2c.js` 45 |
| `etapa2.js` | 66 de 66 |
| `etapa2b.js` | 75 de 75 |
| `etapa2-modelo.js` | 5 sequências, 350 passos, 0 ocorrências |
| Mutações | 6 de página detectadas por `tela-passo6.js`; 8 de backend detectadas, e 1 equivalente. Feitas antes da correção da auditoria e não repetidas |
| Envios a LLM | Nenhum: 1.688 `send.json` antes e depois, nas raízes de teste |

### Auditoria dos Passos 6 e 7

Os testes acima são da validação final, em build novo, depois da correção de R1, com as baterias em sequência. O registro completo está no [plano](docs/historico/PLANO-ate-18-11.md#auditoria-dos-passos-6-e-7-06112026).

| Ressalva | Tratamento |
|---|---|
| R1: a nota apontava Investigar depois de uma execução que falhou | Corrigida, pela opção A: `manual_counts` lê a mesma execução que Investigar, que não mudou |
| R2: os documentos diziam "sem commit" | Revistos depois do commit do código |
| R3: custo de D7 | Registrado: cerca de 0,46 s contra 0,002 s por chamada de `/api/commands`, no TheAlgorithms-Python. A leitura só de `sources.jsonl` ficou fora do escopo, por decisão do usuário |
| R4: um timeout do `etapa2c.js` | Aceito pelo usuário, e registrado: isolado, passou na execução independente e na validação final |
| R5: escopo de contagem manual | Documentado: a nota só considera `calls` com `at=` |

### Testes fora do repositório

`etapa2.js` e `etapa2b.js` ficam no scratchpad da sessão `465f4d1b-0d17-4dff-903f-d3077bb3c8f4`. Três asserções mudaram ali, por causa de D6 e D7: `etapa2b.js:367` e `etapa2.js:141` esperam `no_source`, e `etapa2.js:133` espera o texto novo com contagens manuais. Os originais estão ao lado, em `*.antes-passo6`. **A cópia desses arquivos no arquivo de preservação não mudou**, e continua com as asserções antigas: restaurada de lá, cada uma dessas três asserções falha até receber a mudança. As versões novas também estão em `.studio/arquivo-copias/baterias-passos-6-e-7-20261106`, fora do Git.

### Publicação

O usuário autorizou dois commits e o push normal para `origin/main` logo depois deles: `94089d3`, com o código e os testes, e o commit que traz este texto, com a documentação, cujo hash não cabe aqui. Este texto foi escrito antes do push e não diz como ele terminou. Para conferir: `git log --oneline origin/main..HEAD`, que vem vazio depois do push, e o [CI](https://github.com/ViniciusSJV/librarian/actions).

### O ambiente das baterias

Raízes e dados estão descritos no [README](crates/librarian-studio/tests/tela/README.md) e na [seção 13](#13-4a-concluída-e-encerrada-06112026). O que foi usado nesta verificação está em `e06f24c4-4b85-4d83-be56-34e0de4b2ccd/scratchpad/tela` (raízes de `rodar.js` e `env.sh`) e em `661d7681-430a-4afe-b472-97d59768c138/scratchpad/base` (raízes de `etapa2*.js`), dentro da pasta temporária de scratchpads. São temporários.

### O que continua pendente

As limitações aceitas estão no [resultado](docs/historico/PLANO-ate-18-11.md#resultado-dos-passos-6-e-7-06112026).

| Pendência | Situação |
|---|---|
| Etapa 4 e evolução visual | Não autorizadas em 06/11/2026. A Etapa 4 foi autorizada em 07/11/2026: veja a [seção 17](#17-etapa-4-evidence-builder-07112026-a-11112026). A evolução visual continua não autorizada |
| "N de M", fallback único, filtro de 5% | Decisão própria, sem etapa |
| Falha de leitura como "Sem medição" na sugestão | Limitação aceita; a solução exige distinguir no backend |
| Custo de `/api/commands` sem `instrument` | Registrado, sem correção; a leitura só de `sources.jsonl` ficou fora do escopo, por decisão do usuário |
| Nota com mais de um comando: soma em dobro e verbo no singular | Registrado, sem correção |
| Perfil v8 no plano, capas antigas, rotas no README do Studio, comentário perto da linha 4139 de `index.html` | Documentais ou cosméticas, para revisão separada |

## 17. Etapa 4: Evidence Builder (07/11/2026 a 11/11/2026)

**[Estado corrigido em 18/11/2026: a Etapa 4 está encerrada e publicada, em `09e7e53` e `9bc92b0`. O texto desta seção descreve 11/11.]** **Comece por aqui. A Etapa 4 está em andamento e não está encerrada. Todo o trabalho dela está na árvore de trabalho, sem commit, sobre `60d4d50`, que é também `origin/main`.** O contrato em vigor está no [STUDIO.md](STUDIO.md#pacote-de-evidências-etapa-4); o andamento passo a passo, as decisões e a errata do Passo 14, no [plano](docs/historico/PLANO-ate-18-11.md#andamento-da-etapa-4-07112026-a-11112026).

### Como retomar

1. `git status -sb`: deve mostrar o trabalho da Etapa 4 sem commit, e nada no índice. Os arquivos não rastreados esperados são `crates/librarian-ingest/src/evidence.rs`, `crates/librarian-ingest/tests/evidence-fixtures/`, `crates/librarian-studio/build.rs`, `crates/librarian-studio/src/builder.rs`, `crates/librarian-studio/tests/evidence-v1/` e `crates/librarian-studio/tests/tela/tela-evidencia.js`. **Não descarte, reverta nem reorganize nada.** Se o estado for outro, pare e pergunte.
2. Aguarde a instrução do usuário. A P1, a P2 e a P3 aguardam avaliação; nada depois delas está autorizado.

### O que a Etapa 4 entregou [registrado]

- **Pacote de evidências** (`turns/<NN>/evidence.json`) em toda pergunta nova que chega à preparação, com a marca `evidence: { format_version, sha256 }` em `question.json`. Perguntas anteriores às marcas seguem a conferência antiga, sem migração.
- **Seleção no servidor:** `origin.selection` no formato 2 é conferido contra os registros de sugestão (`runs/<run>/suggestions/<sha256>.json`) e a edition antes de criar a pergunta, e vira `turns/<NN>/selection.json`. `notes` são recusadas em qualquer parte do corpo dessas rotas.
- **Escada:** o que uma rodada de lacuna acrescenta fica em `server_additions`, com fichas `declared_by: "server_lacuna"`.
- **Conferências:** H1 (integridade), H2 (fonte, pela edition guardada) e H3 (rederivação pelo código atual, no formato gravado) rodam antes de enviar, colar, continuar ou aprovar; o `send.json` grava o `evidence_sha256`. H4 refaz a sugestão e **só informa**.
- **Envio por propósito:** pergunta (`send_question`, só com a conferência feita) e léxico (`send_lexicon`, sem pacote).
- **Identidade do código:** `build.rs` grava o commit e o `git_dirty` do Librarian, ou `unknown`.
- **Medição:** a medição mais recente e a linha de base do portão são escolhidas por ordem cronológica, e não pela ordem dos nomes das pastas.
- **Consulta:** `GET /api/evidence?key&run&turn`, só leitura, separa o que foi gravado do que foi conferido agora; a conversa mostra o mesmo em "Pacote de evidências e conferências", em português e inglês.

### Formatos do pacote

- **Formato 1:** gravado até 11/11/2026, antes da P1. Tem `measure_diverges`, sempre `false` e nunca calculado. Continua sendo lido, conferido e refeito pela regra antiga, e aparece com o aviso `measure_diverges_not_computed`. **Esse `false` não prova que as medições sejam iguais.**
- **Formato 2:** toda pergunta nova. Tem `measure_divergence`, com `same`, `diverges`, `not_comparable` (com o lado e o motivo) ou `not_applicable`. A divergência não bloqueia o envio.
- A marca e o pacote precisam declarar o mesmo formato.
- Uma fixture congelada de formato 1, gravada pelo código anterior à P1, fica em `crates/librarian-studio/tests/evidence-v1/`, com os hashes fixados nos testes.

### Situação das pendências da auditoria

| Item | Situação em 11/11/2026 |
|---|---|
| Passos 0 a 15 | Feitos; o 15 foi a auditoria final, que gerou P1 a P4 |
| P1, formato 2 | Implementada e validada no Windows; aguarda avaliação |
| P2, documentação | Feita nesta atualização; aguarda avaliação |
| P3, errata do Passo 14 | Registrada no plano; aguarda avaliação |
| P4, Ubuntu | Pendente: o DNS da máquina estava fora |
| Commit, push e CI | Pendentes, sem autorização |

**Sequência prevista, cada item com autorização própria:** avaliação da P1 a P3 → P4 → commit e push → CI verde no Ubuntu e no Windows → encerramento da Etapa 4 → auditoria funcional do frontend → reformulação do fluxo e evolução visual da cidade.

### Limites que continuam [registrado]

- Os hashes provam consistência, não autoria nem verdade. H4 não comprova afirmações.
- `at_question` não é relido na conferência (desde o Passo 9).
- Formato 1 mantém a limitação de `measure_diverges`.
- A fixture de formato 1 tem caminhos de até 178 caracteres: no Windows, sem `core.longpaths`, um clone numa pasta funda pode falhar. Avaliado na P2; nenhuma mudança sem aprovação.
- Os itens herdados da Etapa 2 (remover `investigationView`, motivos no inventário, D15) não foram feitos.

### Onde estão os dados [registrado]

- **Linha de base imutável:** `.studio/arquivo-copias/linha-base-etapa4-20261107` (com `MANIFESTO.sha256`), fora do Git. Não altere.
- **Baterias da Etapa 2 atualizadas para o formato 2:** `.studio/arquivo-copias/baterias-passo10-20261110`. As cópias anteriores continuam intactas.
- **Relatórios, checkpoints, diffs e logs de cada passo:** no scratchpad da sessão `df77c0d0-d1b8-4cd1-97d0-ad87f1add956`, que é temporário. O checkpoint mais recente do código é `checkpoint-p1`.

### Cuidados que esta etapa ensinou

- **CRLF:** o CI do Windows faz checkout com `core.autocrlf=true`, e o `.gitattributes` não tem regra para `*.rs`. Valide também numa cópia com checkout CRLF (commit em LF, clone com `core.autocrlf=true`). No Git Bash, `grep` só vê o CR com `-U`.
- **Mutações:** só contam as detectadas por asserção, numa cópia descartável, cada uma com o seu próprio `CARGO_TARGET_DIR`.
- **Baterias de tela:** gravam perguntas nas raízes; rode em cópias, nunca nas raízes que o H3 histórico e o manifesto leem.
- **Validação em paralelo:** nunca rode duas cadeias de validação ao mesmo tempo sobre as mesmas pastas. Em 11/11/2026 isso invalidou uma rodada inteira, que foi descartada e refeita.

## 18. Consolidação: Evidence Builder independente, simplificação e proteção da escada (17/11/2026 e 18/11/2026)

**Estado [confirmado em 18/11]:** branch `main`. A consolidação (E1 a E5, simplificação e proteção da escada) foi revista e publicada em 18/11, num commit sobre `27f7bdd` (o hash está no `git log`; a identidade do Builder conferida depois do commit está abaixo). O piloto Python começa depois da publicação, fora deste repositório.

### Arquitetura atual [confirmado]

| Parte | Onde | Responsabilidade |
|---|---|---|
| Acervo, edition, grafo `v2`, tipos do pacote | `crates/librarian-ingest` (`src/evidence.rs` tem os tipos e as conferências puras) | O que o projeto é, byte a byte |
| **Evidence Builder** | `crates/librarian-evidence` (`package`, `check`, `selection`, `suggestion`, `lacuna`, `keeper`, `identity`) | Monta, confere (H1 a H4) e refaz o pacote a partir dos dados que recebe. Não lê arquivos, não roda comandos, não chama modelos, não conhece o Studio (`tests/independence.rs`) |
| **LLM Engine** | `crates/librarian-llm-engine` (`models.rs`, `transport.rs`, `CLAIMS.md`) | Monta o Modelfile de tarefa × perfil × idioma a partir de `modelfiles/catalog.json` e `system/`; envia por Ollama ou Claude CLI; contrato e conferência mecânica de claims |
| **Studio** | `crates/librarian-studio` | Lê e grava `.studio`, entrega os dados ao Builder e guarda as conferências; escada e portão (`ladder.rs`, `main.rs`, `execution.rs`); a página (`web/index.html`) |

Dependências: `librarian-core ← librarian-graph-engine ← librarian-ingest ← librarian-evidence ← librarian-studio`; `librarian-llm-engine ← librarian-studio`.

Na tela: **Seleção** (antes "Evidence Builder") monta a lista; **Pacote** (antes "Revisão") mostra o pacote, H1 a H4, a tarefa e o idioma, e é **o único lugar de onde sai um envio**, inclusive "Enviar pela escada".

### Princípio e proteção da escada [confirmado]

**Nenhuma pergunta chega ao modelo sem passar por um pacote de evidências revisado pelo usuário.**

- **Falha encontrada na auditoria de 17/11:** a página mandava `retries: 0` e `lacuna_rounds: 0`, mas a API tinha 1 e 1 como padrão e aceitava mais; cada nova tentativa e cada rodada de LACUNA criava uma pergunta e a enviava sem revisão. Também aceitava a escada sem `turn` (criava a pergunta e enviava) e com vários modelos (o seguinte respondia depois de uma resposta).
- **Opção A, aplicada em 18/11 (`fn ladder`):** um pedido precisa de `turn` de uma pergunta reunida que passe em H1 a H3; faz um envio, a um modelo, e termina. É recusado antes de qualquer execução, pergunta ou envio, com mensagem terminada em "nada foi criado, executado nem enviado", quando traz `question`, `nodes`, `with_facts` ou `origin`, `retries` ou `lacuna_rounds` diferentes de 0 (inclusive texto, negativo, fracionário e `null`), `models` com outro número de itens que não 1 ou portão vazio. A LACUNA fica registrada com `suggested` (o que o catálogo acharia), sem acrescentar nada.
- **Testes:** em `store.rs`, `the_ladder_refuses_what_would_ask_on_its_own_before_anything_runs`, `the_ladder_refuses_a_package_that_no_longer_checks_before_anything_runs`, `an_answer_of_the_ladder_starts_no_other_call` (recusa mecânica e pelo portão sem nova chamada) e `the_ladder_climbs_on_the_reviewed_package_without_making_another` (um envio só, sem os dois campos). Os testes que usavam rodadas pela API (`recorded_lacuna_rounds_keep_what_the_server_added_and_only_that` e outros três) agora recriam as perguntas de rodada pela store (`recorded_rounds`), com as mesmas asserções de formato, H1 a H4 e exclusão, porque perguntas gravadas assim antes de 18/11 continuam sendo lidas. Na página, `tela-simplificacao.js` confere o corpo do clique.
- **Fora da escada:** `/api/lexicon/suggest` com um perfil manda ao modelo nomes do catálogo, por um clique, sem pacote. Não é uma pergunta sobre o código nem é feito automaticamente. Está mantida como operação explícita e separada da investigação de código (decisão de 18/11): grava `query.json`, o envio com perfil, tarefa e idioma e a proposta conferida (`proposal.json`); só muda o léxico das configurações quando a pessoa aplica, e nunca vira fato no acervo.

### Decisões de 18/11 [confirmado]

- `verificavel` em inglês: as duas traduções ficam, uma por perfil, como exceção temporária, sem mudar textos nem hashes.
- `/api/path-questions`: mantida (prepara, não envia; testes próprios).
- "Abrir arquivos": mantido (visualizador sem servidor).
- Ponytail: em `skills/`, sem tarefa que a aceite.
- Baterias de tela antigas: nenhuma apagada; a classificação está em `crates/librarian-studio/tests/tela/README.md`.

### Identidade do Builder [confirmado]

`build.rs` grava `unknown` enquanto o Git não rastrear `crates/librarian-evidence/Cargo.toml` (`git ls-files --error-unmatch` falha), como o contrato manda. Com uma cópia do índice com o crate adicionado (`GIT_INDEX_FILE`, sem tocar no índice real, cujo hash foi conferido antes e depois), o mesmo teste deu `recorded`, `27f7bdd63cdf2b9b6406c833b74f2c4554961209`, `git_dirty: true`. **Para validar depois do `git add` e do commit:** `cargo test -p librarian-evidence this_binary_has -- --nocapture` e conferir a linha `BUILDER|`: o commit novo, e `git_dirty: false` se nada sob `crates`, `Cargo.toml` e `Cargo.lock` estiver alterado. Um build que já rodou com o crate fora do índice volta a rodar o `build.rs` quando o índice muda (ele observa `.git/index`).

### Validação de 18/11 [confirmado]

| Verificação | Resultado | Referência de 17/11 |
|---|---|---|
| `cargo test --workspace --locked`, Windows | 446 aprovados, 0 falhas, 2 ignorados | 443, 0, 2 (os 3 a mais são os testes novos da escada) |
| `cargo test -p librarian-studio` | 177 aprovados | 174 |
| Ubuntu 24.04 (Docker, `ubuntu:24.04` `fe815609`, Rust 1.100.0, cópia do repositório) | 446 aprovados, 0 falhas, 2 ignorados | 443, 0, 2 |
| Clippy (`--workspace --all-targets`) | Nenhum aviso novo; o binário do Studio caiu de 3 para 2 avisos; o erro antigo de `send_ollama.rs:384` continua | O mesmo erro |
| `tela-simplificacao.js`, Edge | 47 de 47 | 40 de 40 |
| `tela-estado.js`, parte A, Chrome | 46 de 46; parte B pulada (sem a raiz de 04/11) | — |

Nenhuma bateria foi dada como aprovada por remoção: os testes de rodada não foram apagados, e as dez baterias antigas não foram executadas nem contadas.

### Pendências que dependem do usuário

1. ~~Revisão final~~: feita em 18/11, antes da publicação.
2. ~~Commit e push~~: feitos em 18/11, com os arquivos novos adicionados um a um ao índice; identidade do Builder conferida depois do commit. Falta conferir o CI.
3. ~~O léxico fora do pacote~~: decidido em 18/11 (mantido, acima).
4. Adiados conscientemente: opção B da escada, unificação do `verificavel` em inglês, recriação das baterias antigas com uma raiz nova.
5. Limpeza opcional: `%TEMP%\lw` (cerca de 3 GB) e os scratchpads. O Docker Desktop foi aberto para o teste no Ubuntu.

### Onde estão os dados

- `.studio/` deste repositório só tem `projects/renderer-9b78f3a9` com a run `20261118-020654`. **[Estado corrigido em 18/11/2026: depois desta seção, o piloto acrescentou `projects/healthchecks-25597e41`.]** As raízes de 04/11 e `.studio/arquivo-copias` citadas nas seções anteriores **não estão mais nesta máquina**.
- Cópias de segurança feitas antes de cada mudança: `scratchpad\backup-e1e5` (sessão `a7716d82`, 17/11) e `scratchpad\backup-inicio-18-11` (sessão `02fdc43f`, 18/11: patch da árvore, patch do índice, os não rastreados e as fontes do Studio).

## 19. Piloto Python 1 e direção multiagente (18/11/2026)

**Estado [confirmado em 18/11]:** o piloto rodou sobre `d949f91`, e o código dos crates não mudou. O resultado e a direção foram registrados só em documentos.

### Onde ler

- **Resultado completo do piloto:** [docs/reports/PILOTO-1-HEALTHCHECKS.md](docs/reports/PILOTO-1-HEALTHCHECKS.md). Traz os números, os quatro workflows com a evidência de cada elo, as 14 perguntas, o acervo, a reutilização e as limitações G1 a G8, P1 a P4 e as do Ingest, da verificação e do LLM.
- **Direção arquitetural:** [docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md](docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md). Traz os papéis, os pacotes pela necessidade do consumidor, o ciclo de complementação, o contrato público, os orquestradores externos, os extratores, o papel do Studio, a rastreabilidade e os 14 princípios.
- **Roadmap das fases 1 a 8 e Piloto 1b com critérios de aceite:** [plano, última seção](docs/historico/PLANO-ate-18-11.md#piloto-python-1-e-direção-multiagente-18112026).

### O essencial [confirmado]

- **Healthchecks** em `6c05a7adf266efb70f9517727bef9ea8b2bbf52f`, clone em `Projetos/pilotos/healthchecks`, sem alterações. No `.studio` local, projeto `healthchecks-25597e41`, run `20261118-061046`, turns 01 a 14 e acervo F1 a F26. Esses dados não estão no Git.
- **Números:**
  - ingestão: 657 arquivos, 4.718 símbolos, 61.506 nós, 116.928 arestas, nenhum diagnóstico;
  - envios: 14 perguntas e 16 envios ao Qwen3-Coder local (2 falharam pela queda do `llama-server`);
  - contexto: cerca de 318 KB e 83 mil tokens de prompt;
  - itens: 65 candidatos, 44 aprovados mecanicamente, 26 promovidos e 18 rejeitados;
  - reutilização: 8 de 12 fatos recuperados;
  - conferências: H1 a H3 em todos os pacotes, H4 não exercitado.
- **Os 26 fatos foram revisados pelo agente, não aprovados pelo usuário.** O contrato grava "revisão humana"; cada nota declara isso.
- **P1, bug reproduzido:** `target_of` (`store.rs`) usa `chunk.start_line`, que em Python inclui os decoradores. Por isso fatos de funções decoradas não voltam nem com `with_facts` nem na sugestão de envelope.
- **G3:** `/api/calls` omite sem avisar as chamadas acima do teto de homônimos.
- **Ligação pelo banco:** `Check.ping` → `Flip(processed=None)` → `process_one_flip` não é uma chamada. Foi reconstruída lendo o código.

### Como retomar

1. **Fase 1, Piloto 1b**, só com autorização:
   - corrigir P1, com teste de regressão Python e verificação separada do Rust;
   - mostrar as omissões de G3;
   - gerar uma nova edition e repetir Q6;
   - comparar o resultado.
2. **G1 fica para a Fase 2**, depois de medir o impacto no corpus Rust.
3. **A autoria dos fatos e a promoção por item são da Fase 3.** Não mudar o formato dos fatos sem especificação.
4. **Não adicionar framework de agentes aos crates centrais.** Não implementar a interface pública nem o extrator Java antes das fases deles.

## 20. Encerramento do Piloto 1 e retomada pelo Piloto 1b (18/11/2026)

### Estado publicado [confirmado em 18/11]

- **Branch e sincronização:** `main`, sincronizada com `origin/main`, árvore limpa.
- **Código-base do piloto:** `d949f91`. **Último commit de código:** `d949f91`, que é o mesmo.
- **Registro documental do piloto:** `cf33147`. Feito **depois** do piloto, não é implementação: entre `d949f91` e `cf33147` só mudaram documentos (o único arquivo sob `crates/` é o README do `librarian-evidence`).
- **Piloto 1 (Healthchecks):** concluído.
  - Relatório permanente: [docs/reports/PILOTO-1-HEALTHCHECKS.md](docs/reports/PILOTO-1-HEALTHCHECKS.md).
  - Direção arquitetural: [docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md](docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md).
- **Próxima prioridade:** Piloto 1b (Fase 1 do plano). **Nenhuma parte dele foi iniciada.** **[Estado corrigido: P1 e G3 foram commitados depois desta seção, e a Q6 foi refeita; veja a seção 21.]**
- **Nenhum agente nem orquestrador implementado.**

### Arquitetura aprovada [registrado]

- O Librarian continua em Rust, independente de linguagem, modelo, agente e orquestrador.
- O consumidor declara as evidências de que precisa; o Librarian constrói e fornece pacotes rastreáveis.
- A coordenação dos agentes fica fora do Librarian.

### Ler antes de começar

1. Este documento: as seções 19 e 20.
2. O [plano, seção "Piloto Python 1 e direção multiagente"](docs/historico/PLANO-ate-18-11.md#piloto-python-1-e-direção-multiagente-18112026), com o roadmap e os critérios de aceite do 1b.
3. [docs/reports/PILOTO-1-HEALTHCHECKS.md](docs/reports/PILOTO-1-HEALTHCHECKS.md), sobretudo a seção 6 (limitações) e a reutilização em Q6.
4. [docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md](docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md).
5. A documentação dos componentes:
   - [Ingest](crates/librarian-ingest/README.md): inclui o grafo `v2` que o Studio usa, em `crates/librarian-ingest/src/graph/`.
   - [Graph Engine](crates/librarian-graph-engine/WORKFLOW-V0.1.md): dossiês e query preparada.
   - [Evidence Builder](crates/librarian-evidence/README.md).
6. O histórico Git recente (`git log --oneline -6`).

### Piloto 1b: correções de confiabilidade

Fazer nesta ordem.

**1. P1: recuperação de fatos decorados.**

- Ponto de partida: `target_of` em `crates/librarian-studio/src/store.rs` (perto da linha 1360). Hoje usa `chunk.start_line`; em Python, o trecho começa no decorador. Quem consome: `reviewed_about` (perto da linha 789) e `suggest_with`.
- Corrigir para a linha real de declaração do símbolo, a mesma que o envelope informa em `declaration_line`.
- Teste de regressão em Python: função decorada com fato promovido, que precisa ser recuperado pela pergunta com `with_facts` e pela sugestão de envelope.
- Verificar à parte o comportamento com atributos Rust (`#[...]`), sem supor que seja igual.
- Preservar a compatibilidade com o ray tracer: os fatos e os testes existentes continuam iguais.

**2. G3: omissões de chamadas ambíguas.**

- Pontos de partida:
  - `/api/calls` chama `Graph::calls_from` em `crates/librarian-ingest/src/graph/v2.rs`. Uma ocorrência sem aresta `name_candidate` não gera link.
  - O teto está em `crates/librarian-ingest/src/graph.rs`: acima de `max_candidates`, o nó guarda só `ambiguous_candidates` e nenhuma aresta é desenhada.
- Tornar as omissões explícitas na resposta, com o motivo e a contagem de candidatos.
- Preservar a incerteza e não escolher relação arbitrariamente.

**3. Q6: repetição da investigação.**

- Gerar uma nova edition do Healthchecks (`6c05a7a`, clone em `Projetos/pilotos/healthchecks`, projeto `healthchecks-25597e41` no `.studio` local) depois das correções.
- Repetir a pergunta Q6 com as mesmas sete declarações e `with_facts`.
- Comparar se F2, F4, F5 e F6 voltam e se o F6 deixa de ser repetido.
- Medir o tamanho do pacote e os tokens.
- Comparar com a linha de base: 46,2 KB, 11.667 tokens, 8 de 12 fatos e 1 fato novo. O texto da pergunta está no turn 14 da run `20261118-061046`.

### Restrições da próxima sessão

- **Não antecipar G1.**
- **Não implementar:** Evidence Request, formato 3, API pública HTTP/JSON, extrator Java, agentes especializados, orquestrador nem integração com LangGraph. Esses itens pertencem às fases 2 a 8 do plano.
- **Mudar o escopo do 1b só com justificativa e decisão do usuário.**

### Primeira atividade técnica recomendada

Escrever o teste de regressão do P1 **antes** da correção: uma função Python decorada, com fato promovido, que hoje não é recuperada. O teste deve falhar com o código atual, pela diferença entre `chunk.start_line` e a linha do `def`. Só então mudar `target_of`, e rodar a suíte inteira com o corpus Rust.

## 21. Piloto 1b: P1, G3, Partes B, C e E, e Q6 (18/11/2026)

**Estado nesta data.** Os relatórios completos estão em [docs/reports/piloto-1b/](docs/reports/piloto-1b/), com os dados em `dados/`. O estado vigente fica no [ESTADO-ATUAL.md](ESTADO-ATUAL.md).

- **P1 (`987d7a8`, `fix(studio)`):** `target_of` usa a linha da declaração, e não o início do trecho com os decoradores. Fatos de funções Python decoradas voltam com `with_facts` e na sugestão de envelope: `ping` traz F2, F4, F5 e F6; `ping_by_slug` traz F1, F3 e F7. Com isso, a pendência 65 do EST está resolvida. Relatório: [01-g3.md](docs/reports/piloto-1b/01-g3.md) e [02](docs/reports/piloto-1b/02-fechamento-g3-e-diagnostico.md), Parte A.
- **G3 (`aae51d7`, `feat(graph)`):** `/api/calls` devolve `unresolved` com três motivos (`above_candidate_limit`, `no_candidate_in_project`, `node_limit`), e a vista "o que chama?" tem a seção "Chamadas não apresentadas". Decisão A: chamada sem nome (`handlers[0]`) também é `no_candidate_in_project` (DEC-28 no [DECISOES.md](DECISOES.md)). Suíte: **456 aprovados, 0 falhas, 2 ignorados** (eram 446 na seção 18). O `node_limit` só foi visto num projeto sintético. Relatório: [01-g3.md](docs/reports/piloto-1b/01-g3.md).
- **Parte B (tipos):** há duas regras de resolução que não concordam. "O que chama?" não usa tipo; o inventário usa `receiver_type`, só em Rust. Python não liga variável a tipo nos níveis N1–N8. Relatório: [02](docs/reports/piloto-1b/02-fechamento-g3-e-diagnostico.md), Parte B.
- **Parte C (precisão, sem correção):** Healthchecks, 4 de 8 links únicos da amostra errados (método ligado a função solta); dos 442 links únicos de método, 197 vão para função solta. Ray tracer, nenhum erro. Relatório: [02](docs/reports/piloto-1b/02-fechamento-g3-e-diagnostico.md), Parte C.
- **Parte D (inventário dos MDs) e Parte E (acervo × grafo):** 3 fatos afirmam chamadas que o grafo liga errado (F1, F14, F17) e 2 que ele liga de forma ambígua (F6, F7). `ping_by_slug → ping` é homônimo de 2, não [grafo]. Relatório: [03-inventario-e-auditoria.md](docs/reports/piloto-1b/03-inventario-e-auditoria.md).
- **Q6 (run `20261118-205150`), avaliada pelo usuário em 18/11:** entrega sim (13 fatos, +3,9% de tokens); reutilização não (F20, F22 e F26 entregues e repetidos; só a repetição do F6 sumiu); resposta reprovada na C3 (`location_operation_includes_declaration`); C2 semanticamente incorreta com citação válida; nenhuma LACUNA sobre o elo pelo banco. Relatório: [04-q6.md](docs/reports/piloto-1b/04-q6.md).
- **Depois:** consolidação documental (passo 1.2): [ESTADO-ATUAL.md](ESTADO-ATUAL.md), [DECISOES.md](DECISOES.md), plano reduzido ao roadmap e [docs/historico/](docs/historico/).

## 22. Passo 2.0: portão de precisão da Fase 2 (18/11/2026)

**Estado nesta data.** O estado vigente fica no [ESTADO-ATUAL.md](ESTADO-ATUAL.md); como rodar, em [tools/precisao/README.md](tools/precisao/README.md).

- **Ferramenta:** `tools/precisao/`, em Node, sobre o binário real. O comando copia o `.studio` para uma pasta temporária, gera um run novo com o binário em teste e mede nesse run. Também roda `verify-graph` nos runs antigos e confere o hash do `.studio` original. Os scripts de `docs/reports/piloto-1b/dados/` ficaram como evidência.
- **Linha de base (binário de `aae51d7`):** reproduz a Parte C (20 Python: 4/4/0/12; 10 Rust: 2/0/2/6), a população (1956 e 515) e os 197 links de método → função solta. Reproduz também a Parte E (15 não dependem, 6 corretos, 2 suspeitos, 3 errados). O run gerado de novo dá o mesmo grafo, byte a byte.
- **Amostra ampliada:** 60 Python e 30 Rust, estratificadas por `resolution`. A verdade foi validada pelo usuário: 12 revisadas e 78 aceitas por amostragem.
  - Python: 9 corretas, 17 `link_falso`, 10 `candidatos_falsos`, 10 ambíguas e 14 sem candidato.
  - Rust: 5 corretas, 7 `link_falso`, 0 `candidatos_falsos`, 11 ambíguas e 7 sem candidato.
- **Achado:** a regra `receiver` erra nas 17 chamadas da população em que aparece. Ela toma `self.x.m()` por `self.m()`, e o inventário usa a mesma regra. Fica para o passo 2.1.
- **Decisões:** DEC-33 (dublê de teste não é alvo). "Errada" foi dividida em `link_falso`, que reprova se subir, e `candidatos_falsos`, que é só registrado. A questão do dono no HC-19 (decorador) foi para o passo 2.3.

## 23. Passo 2.1: regra única de resolução (19/11/2026)

**Estado nesta data.** O estado vigente fica no [ESTADO-ATUAL.md](ESTADO-ATUAL.md). O contrato de "o que chama?" está no [SEARCH.md](crates/librarian-ingest/SEARCH.md#o-que-chama-calls_from-post-apicalls).

- **Regra única:** `crates/librarian-ingest/src/resolve.rs`. É usada por `calls_from` (`/api/calls`) e pelo inventário. O envelope e o contexto do dono aceitam o que ela decide com um alvo só (`SETTLED`: unique, receiver, type, trait_default).
  - Precedência: chosen > unique > receiver > type > trait_default > trait > homonym. `unique` e `receiver` vêm antes de `type` para não mudar rótulos corretos; o tipo derrubar um `unique` errado é o 2.2.
  - Mudança em relação ao desenho da Parada 1: o arquivo ficou na raiz do crate, e não em `graph/`, porque conhece `impl`, `trait` e `Self`, e o teste `agnostic.rs` exige que o grafo não nomeie linguagens.
- **receiver:** só `self.m()` (o receptor da linguagem e nada mais antes do nome). Um método de trait não é alvo do receiver, como o inventário já fazia.
  - Achado: o `calls_from` antigo comparava o bloco pelo nome, e o inventário pelo id. Com a regra única, 5 `self.pause()` do Healthchecks (várias classes `Command`) passaram de homônimo para `receiver`, corretamente. Nenhum `receiver` antigo apontava para outro bloco.
- **type e trait_default:** camada sobre o grafo, sem aresta nova. Um link acima do teto não tem aresta candidata: `candidate_edge: null`. Todo link ganhou `basis` (`graph_edge` ou `type`). Vários alvos (`trait`, ou um tipo genérico com vários impls) aparecem como `homonym` em `/api/calls`; o inventário segue todos.
- **Portão** (aprovado, igual à previsão da Parada 1, número por número):
  - Python: `link_falso` 17 → 13, `candidatos_falsos` 10 → 12 e ambíguas 10 → 12; as corretas ficam em 9.
  - Rust: corretas 5 → 11, `link_falso` 7 → 2, `candidatos_falsos` 0 → 1 e ambíguas 11 → 9.
  - População: `receiver` 17 → 0. No Rust, `type` 53, `trait_default` 7, `homonym` 71 → 35 e acima do teto 14 → 3.
  - Níveis: N1, N2, N3b, N4 e N5 do Rust estão no alvo. Python e fatos F1–F26 sem mudança.
- **Amostra ampliada:** RT-31 a RT-50, nos estratos `type`, `type_caminho` (acima do teto), `type_fora` e `trait_default`. A verdade foi proposta pelo agente e aguarda o usuário (DEC-22): 20 de 20 corretas. O portão passou a achar chamadas fora da população e compara só as entradas que a linha de base tem.
- **/api/calls, antigo × novo, todo símbolo dos dois runs:**
  - Healthchecks: 4403 de 4651 consultas não envolvem `receiver` nem `type`, e todas ficaram iguais nos campos antigos.
  - Ray tracer: o mesmo vale para 479 de 729 consultas.
  - Os 17845 links que já existiam ficaram iguais (16519 + 1326); só ganharam `basis`.
  - As mudanças são as previstas: 513 acima do teto → `type`, 98 homonym → `type`, 40 homonym → `trait_default`, 48 + 30 `receiver` desfeitos, e os 5 `self.pause()`.
- **Validação:** `cargo test --workspace --locked` com 468 aprovados, 0 falhas e 2 ignorados. G3 e P1 passam. As perguntas 01–14 do Healthchecks passam em H1–H3.
- **Parada 3 (19/11), depois da avaliação do usuário:** relatório em [docs/reports/fase-2/2.1-parada-3.md](docs/reports/fase-2/2.1-parada-3.md).
  - Portão: a regra antiga com o binário novo e a regra nova com o binário antigo dão o mesmo veredito sobre as 30 + 60 chamadas originais. Registro proposto: DEC-37.
  - Amostra: RT-51 a RT-55 (chamadas por variável acima do teto, uma por origem do tipo), 5 de 5 corretas. Linha de base proposta em `tools/precisao/linha-de-base-proposta.json` (Rust 55: 36 corretas, 2 `link_falso`).
  - Os 513 acima do teto que viraram `type`: 475 por caminho e 38 por variável. Destas, 32 vêm de `let x = T::f(..)`, que o extrator lê como `T` sem olhar o retorno (heurística, para o 2.3).
  - `trait_default`: os 12 casos conferidos com arquivo e linha. O ray tracer não tem `dyn`.
  - Verificação de tela (DEC-30): 51 de 51.
  - O status da DEC-11 foi corrigido: o 2.1 reabre a decisão e resolve pelo tipo, mas os homônimos continuam.

## 24. Passo 2.2: forma da chamada, `import` e o tipo que derruba o `unique` (19/11/2026)

**Estado nesta data.** O estado vigente fica no [ESTADO-ATUAL.md](ESTADO-ATUAL.md). O contrato está no [SEARCH.md](crates/librarian-ingest/SEARCH.md#o-que-chama-calls_from-post-apicalls). Relatórios: [Parada 1](docs/reports/fase-2/2.2-parada-1.md), [Parada 2](docs/reports/fase-2/2.2-parada-2.md), [folha de revisão](docs/reports/fase-2/2.2-revisao-amostra.md).

- **Regra (em `crates/librarian-ingest/src/resolve.rs`):** a forma da chamada admite só alguns candidatos; as arestas do grafo não mudam.
  - `x.m()` não alcança função solta.
  - No Python, um receptor ligado a módulo do projeto alcança só as funções desse módulo (resolução nova `import`, `basis: "import"`).
  - Um receptor ligado a import de fora do projeto não alcança nada.
  - `f()` sem receptor não alcança método, salvo, no escopo de uma classe, um método dela.
- **Tipo:** o tipo escrito do receptor derruba um `unique` que não serve para ele. Com tipo de fora do projeto, só cai método inerente de tipo do projeto, e não cai com referência, `Box`, `Rc`, `Arc` ou com um trait do projeto implementado para o tipo.
- **Receptor:** um `self.m()` acima do teto sai `receiver` (RT-55).
- **De onde vêm os dados:** a tabela de imports, as ligações locais e o tipo escrito que não é do projeto são lidos dos snapshots pelo extrator (`Extractor::call_facts`), sob demanda, no arquivo de quem chama. O grafo, os perfis e as editions não mudam.
- **Decisões do usuário na Parada 1:**
  - Q1, regra espelho: incluída.
  - Q2, import de fora: incluído, e a ligação local vence o import.
  - Q3, homônimo que fica com um só candidato: continua `homonym`. O HC-F6 fica para o 2.3.
  - Q4, tipo de fora: incluído, de forma conservadora.
  - Q5, N0-livre: passo "a definir".
  - Q6, `modulo.f()` que já era `unique`: continua `unique`.
  - Registro: DEC-38.
- **Contagens pedidas (Healthchecks):**
  - Q1 (a), chamadas simples no escopo de uma classe que alcançam método dela: 0 de 2900.
  - Q1 (b), funções aninhadas: 17, chamadas 21 vezes, todas `function`.
  - Q2, sombreamento: 0 de 2244 receptores ligados a import.
  - O `logistica-api` não está no disco (`Projetos\logistica-api` não existe), e a contagem dele ficou pendente.
- **Portão** (aprovado, igual à previsão da Parada 1, número por número):
  - Python: corretas 9 → 13, `link_falso` 13 → 1, `candidatos_falsos` 12 → 11, ambíguas 12 → 8 e sem candidato 14 → 27.
  - Rust: `link_falso` 2 → 0 e sem candidato 7 → 9; as corretas ficam em 36.
  - População: método → função solta 197 → 30 (todos `modulo.f()` certos) no Python e 2 → 0 no Rust; `import` 14.
  - Fatos: 15/6/2/3 → 16/8/1/1 (HC-F7 e HC-F17 corretos, HC-F14 não depende; HC-F6 suspeito e HC-F1 errado).
- **Simulador × binário:** chamada por chamada, 0 divergências nas 16239 chamadas do Healthchecks e nas 2545 do ray tracer.
  - Uma rodada intermediária achou 11 links falsos do binário (`TimestampSigner().sign`, o primeiro nome do receptor tomado com os parênteses), corrigidos com teste antes da medição final.
- **/api/calls, antes × depois:**
  - Mudam 3406 chamadas no Healthchecks e 16 no ray tracer, todas previstas.
  - Das que não mudam, 12670 e 2529 ficam iguais em todos os campos.
  - 163 links `receiver` do Healthchecks só têm `candidates` menor, porque a função solta saiu da contagem.
  - O grafo do run novo é igual ao antigo, byte a byte.
- **Validação:**
  - `cargo test --workspace --locked`: 481 aprovados, 0 falhas e 2 ignorados (13 testes novos em `tests/resolve_forma.rs`).
  - Perguntas 01–14: H1–H3 aprovados.
  - DEC-30: 56 de 56.
- **Amostra:** estratos novos HC-61 a HC-75 e RT-56 a RT-62. A verdade, proposta pelo agente, foi validada pelo usuário em 19/11/2026 (HC-66, HC-69 e RT-62 conferidos no editor). A linha de base proposta passou a ser `tools/precisao/linha-de-base.json`: Python (75) 21/1/12/8/33 e Rust (62) 36/0/1/10/15.

## 25. Passo 2.3: Parada 1 e subpasso 2.3a-0 (19/11/2026)

**Estado nesta data.** O estado vigente fica no [ESTADO-ATUAL.md](ESTADO-ATUAL.md). Relatório da Parada 1: [docs/reports/fase-2/2.3-parada-1.md](docs/reports/fase-2/2.3-parada-1.md); dados em [docs/reports/fase-2/dados/2.3/](docs/reports/fase-2/dados/2.3/).

- **Portão antes de começar:** aprovado contra a linha de base do 2.2, sem mudança (Python 21/1/12/8/33, Rust 36/0/1/10/15, fatos 16/8/1/1).
- **Parada 1 (diagnóstico, sem código de produção):**
  - Método: o dump de `/api/calls` de todos os símbolos, um leitor de tipos do Python sobre a edition (`tipos23.rs`, com o parser do extrator) e um simulador do 2.3a que parte do dump (`sim23.js`, `eval23.js`).
  - Previsão do 2.3a: Python 21/1/12/8/33 → 27/1/12/2/33 (HC-51 a HC-56 ambíguas → corretas), Rust sem mudança, HC-F6 suspeito → correto.
  - Rust: as 204 funções `f` do projeto em `let x = T::f(..)` retornam `Self`; ler o retorno não muda nenhum resultado no ray tracer.
  - Dono: 7 chamadas presas a campo do `__init__` e 783 em decoradores no Healthchecks (103 em produção).
  - Achado: a regra `receiver` acima do teto do 2.2 nunca valia para o Python, porque os métodos eram indexados pelo último trecho depois de `::`, e o rótulo Python é `Classe.metodo`.
- **Decisões do usuário na Parada 1:** Q1–Q5, Q7, Q8 e Q12 aprovadas; Q6 (`Protocol`) sem link único, verdade "ambígua"; Q9 adiada para a Parada 1 do 2.3b; Q10 como subpasso próprio (2.3a-0); DEC-39 aprovada com a exceção do `__getattr__`, para entrar no DECISOES.md no commit do 2.3a.
- **Subpasso 2.3a-0** (`8c689d4`):
  - `resolve.rs`: a chave de `methods_by_name` passa a ser o último trecho do rótulo, depois de `::` ou de `.`.
  - Teste `self_dot_m_above_the_limit_is_receiver_in_python` (com o controle da função aninhada).
  - `cargo test --workspace --locked`: 482 aprovados, 0 falhas, 2 ignorados.
  - Portão: aprovado, relatório igual ao da linha de base, salvo a data.
  - `/api/calls` antes × depois: 109 chamadas mudam no Healthchecks, todas previstas e todas de teste (`self._setup_data(..)`, `above_candidate_limit (9)` → `receiver (9)`); 0 no ray tracer; o grafo do run novo é igual ao antigo.

## 26. Passo 2.3a: tipos do Python no arquivo e instanciação (20/11/2026)

**Estado nesta data.** O estado vigente fica no [ESTADO-ATUAL.md](ESTADO-ATUAL.md). Relatório: [Parada 2](docs/reports/fase-2/2.3a-parada-2.md); folha: [2.3-revisao-amostra.md](docs/reports/fase-2/2.3-revisao-amostra.md); dados em [docs/reports/fase-2/dados/2.3/](docs/reports/fase-2/dados/2.3/). Código, testes e interface em `5589912`.

- **Commits e push do 2.3a-0:** `8c689d4` (código e teste) e `1152875` (documentação), com push em 19/11/2026; CI verde (run `44621157811`).
- **Uma interrupção:** a sessão parou no meio da implementação, por falta de energia prevista; o estado foi registrado numa nota de retomada, apagada depois, e o trabalho foi retomado.
- **Previsão antes da implementação:** o simulador foi estendido para `modulo.Classe(..)`, `T::V(..)`, `Protocol` e `__getattr__`, como pedido na aprovação da Parada 1.
- **Perfil V9** (`librarian-ingest/0.9.0;...;rust-structural-v7;python-structural-v5`, grafo `rust-python-syntax-graph-v6`):
  - o nó `receiver_type` ganha `type_rule`, nos dois extratores;
  - o Python grava `receiver_type` (parâmetro, anotação, construtor, `self`, campo do mesmo arquivo) e tem `class` entre os candidatos de chamada de método;
  - qual classe o nome nomeia é lido na consulta, pelo arquivo e pelos imports do 2.2.
- **Regra:** `type` para o método que a classe declara; derrubada sem base, salvo `__getattr__` e `Protocol`; `T::V(..)` pela variante; `relation: instantiates` com `runs` (o `__init__` próprio), seguido no lugar da classe (DEC-39).
- **Divergência corrigida:** com 17 classes `Command`, acima do teto, o tipo ficava sem aresta (HC-81); a classe passou a ser procurada pelo nome escrito, com teste.
- **Portão:** aprovado, igual à previsão. Python 21/1/12/8/33 → 27/1/12/2/33 (HC-51 a HC-56); Rust igual; fatos 16/8/1/1 → 16/9/0/1; N1-py, N1-py-construtor, N2-py e N4-py no alvo.
- **/api/calls antes × depois:** 171 mudanças no Healthchecks e 11 no ray tracer, todas previstas, nenhuma fora da previsão; "instancia" 655 e 13, todas previstas (574 e 2 só ganharam `relation`; 81 e 11 são links novos, contados entre as que mudam).
- **B5:** a pergunta congelada passou a gerar a edition no perfil V8, em que foi congelada; não exercita o V9.
- **Validação:** `cargo test --workspace --locked` 493 aprovados (11 testes novos em `tests/tipos_2_3a.rs`); perguntas 01–14 com H1–H3; DEC-30 84 de 84, depois de corrigir o texto do `__init__` que a página procurava entre os nós desenhados.
- **Validação do usuário (20/11/2026):** a verdade de HC-76 a HC-87 e RT-63 a RT-65 foi validada (HC-76, HC-81 e RT-63 conferidos no editor; HC-79 na Q6), e a DEC-39 foi aprovada. A linha de base nova (Python 87 chamadas 38/1/12/3/33, Rust 65 chamadas 39/0/1/10/15; fatos 16/9/0/1) passou a ser `tools/precisao/linha-de-base.json`; o portão contra ela deu APROVADO, sem nenhuma chamada mudando de categoria.

## 27. Passo 2.3b: dono do decorador e do campo (20/11/2026)

**Estado nesta data.** O estado vigente fica no [ESTADO-ATUAL.md](ESTADO-ATUAL.md). Relatórios: [Parada 1](docs/reports/fase-2/2.3b-parada-1.md) e [Parada 2](docs/reports/fase-2/2.3b-parada-2.md); folha: [2.3b-revisao-amostra.md](docs/reports/fase-2/2.3b-revisao-amostra.md); dados em [docs/reports/fase-2/dados/2.3b/](docs/reports/fase-2/dados/2.3b/). Nada commitado nesta data.

- **Parada 1 (Q9):** a opção B (pela regra) contra a A (módulo dono, edition V10) e uma A reduzida. B ganhou porque mantém no grafo a ligação entre a view e a guarda (`require_setting`, `cors`, `authorize`), que a Fase 2B usa. A A reduzida a moveria para o módulo e não resolveria o campo.
- **Respostas do usuário (20/11):**
  - Q1: B com duas correções (o campo pela contenção; o `called` lendo a `relation`), com a condição de cada chamada ser contada uma vez;
  - Q2: as 48 chamadas soltas no topo vão para a Fase 2B do PLANO;
  - Q3: DEC-40 aprovada no conteúdo, proposta até a prova em código;
  - Q4: o texto de `instancia` corrigido junto;
  - Q5: estratos HC-88 a HC-97 e o nível N4-py-dono.
- **778 × 783:** o leitor da Parada 1 não entrava nos argumentos nomeados; o certo é 783. Os 5 que faltavam incluem 4 `CurlError(..)` (classe do projeto) em `@patch(.., side_effect=..)`: 69 links passam a `decorates`, não 65.
- **Regra (sem edition):**
  - os intervalos dos decoradores são lidos do arquivo na consulta;
  - `relation: decorates`, não seguido nem no inventário;
  - o método responde pelos campos que contém, com `via`, decidido pelo campo;
  - uma ocorrência por resposta;
  - `called` e envelope não repetem ocorrência; "Decorador de" e "Instanciada por" no `prepare`; motivo `decorator` no envelope.
- **Portão:**
  - a regra da DEC-40 e a anterior congelada, lado a lado. Com o binário do 2.3a, as 152 chamadas da linha de base dão o mesmo veredito, e o portão aprova;
  - com o do 2.3b, aprovado e igual à linha de base. Estratos novos: decorador 3 corretas e 3 sem candidato; campo 1 correta e 3 sem candidato; os 4 executores mostram a chamada; N4-py-dono no alvo.
- **Antes × depois de todos os símbolos:** 0 mudanças fora da previsão; `used_by` igual em 4.718 + 819 símbolos.
- **Validação:** `cargo test --workspace --locked` 497 aprovados (4 testes novos em `tests/dono_2_3b.rs`); perguntas 01–14 com H1–H3; DEC-30 76 de 76.
- **Tropeços:**
  - o `cargo test` não recompila o `librarian_studio`, e a primeira rodada da tela usou um studio velho. Tudo o que usa o studio foi refeito com o binário final;
  - o nível N4-py-dono tinha a linha errada na fixture (12 em vez de 14).
- **Antes da validação (pedido do usuário, 20/11):**
  - o detalhe de um link que o percurso não segue (`decorates`, `instancia` sem `__init__`) passa a dizer "decidida, sem seguir" no lugar de "seguida". Tela refeita: 102 de 102;
  - o portão e o script de tela compilam o `librarian_studio` antes de rodar, quando usam o `target/release`, e registram o SHA-256 dos dois binários;
  - a DEC-40 foi aprovada com o texto final e passou a vigente.
- **Validação do usuário (20/11/2026):**
  - a verdade de HC-88 a HC-97 foi validada (HC-89 e HC-97 conferidos no editor);
  - a linha de base nova (Python 97 chamadas 42/1/12/3/39, Rust 65 chamadas 39/0/1/10/15; fatos 16/9/0/1) passou a ser `tools/precisao/linha-de-base.json`;
  - o portão contra ela deu APROVADO, sem nenhuma chamada mudando de categoria.
- **Commits:** `6b424a0` (portão, com a prova da DEC-40), `5a26630` (código, testes e interface; `cargo test` na worktree do commit), `944fd0a` (documentação) e o da linha de base.

## 28. Passo 2.3c: tipo pelo retorno e fechamento do 2.3 (20/11/2026)

**Estado nesta data.** O estado vigente fica no [ESTADO-ATUAL.md](ESTADO-ATUAL.md). Relatórios: [Parada 1](docs/reports/fase-2/2.3c-parada-1.md) e [Parada 2](docs/reports/fase-2/2.3c-parada-2.md); folha: [2.3c-revisao-amostra.md](docs/reports/fase-2/2.3c-revisao-amostra.md); dados em [docs/reports/fase-2/dados/2.3c/](docs/reports/fase-2/dados/2.3c/).

- **Antes de tudo:** CI do 2.3b verde (run `44697265818`, `f50b8f8`); portão APROVADO contra a linha de base (Python 42/1/12/3/39, Rust 39/0/1/10/15, fatos 16/9/0/1).
- **Parada 1:** a recontagem da seção 5.3 do 2.3 confirmou 4 chamadas pelo retorno e 3 pelo campo de outro arquivo no Python, e nenhuma no Rust. Achado: lida ao pé da letra, a regra "se não fechar com um alvo, sem tipo" derrubaria RT-44 (`Patterns::from(..)`, acima do teto) e reprovaria o portão.
- **Respostas do usuário (20/11):**
  - Q1: a regra em quatro casos: (a) um alvo, o retorno dele; (b) sem alvo único, mas todos os candidatos com o mesmo retorno, esse retorno; (c) sem candidato no projeto, o que valia; (d) candidatos que discordam, sem tipo. Contar os casos antes de implementar e parar se o (d) reprovasse o portão;
  - Q2: `Box`, `Rc` e `Arc` de `Self` não decidem; `Result<Self>` com `?` perde o tipo (limite registrado);
  - Q3: `type_rule` no `/api/calls`, documentado no SEARCH.md;
  - Q4: `f(..).m()` e `self.x = f(..)` entram;
  - Q5: estratos HC-98 a HC-112, RT-66 e RT-67 e os níveis N3c;
  - Q6: DEC-41 reescrita com a Q1, proposta na Parada 2;
  - o IDS.md corrigido (a amostra ia até HC-97 e não estava registrada).
- **Contagem da Q1 (antes do código):** Rust (a) 193, (b) 49 (`Patterns::from` 22, `create_default_world` 27), (c) 38, (d) 0. Python (a) 103, (b) 46, (c) 546, (d) 51, sem efeito (hoje já sem tipo). O (b) acrescentou uma mudança: `tr.can_accept()` (HC-110), ligado por um `homonym` de um só candidato.
- **Regra (sem perfil novo):** a ligação, os retornos e os campos das classes são lidos dos arquivos na consulta; a chamada que liga é resolvida sem o retorno e sem `chosen`, por uma trava de um nível no resolver.
- **Resultado:** portão APROVADO; 8 chamadas do Python passam de `homonym` a `type`, nenhuma no Rust; o grafo do Healthchecks é igual, byte a byte, ao do binário do 2.3b.
- **Validação:** `cargo test --workspace --locked` 509 aprovados (12 testes novos em `tests/retorno_2_3c.rs`); perguntas 01–14 com H1–H3; a interface não muda (DEC-30 não se aplica).
- **Antes da validação (pedido do usuário, 20/11):**
  - o (b) de candidato único: dos 46 casos (b) do Python, 3 vêm de um `homonym` de um só candidato com receptor sem tipo, todos de `tr = project.transfer_request()`; só `tr.can_accept()` (HC-110) muda. O risco foi escrito na DEC-41;
  - a perda do caso (d) com `misto()` foi ligada ao N0-livre (chamada simples resolvida pelo escopo) e os dois foram para o 2.4, junto do alias (N8).
- **Fechamento do 2.3** no PLANO: o que os quatro subpassos entregaram, os níveis no alvo e o que ficou para o 2.4, o 2.6 e a Fase 2B.
- **Validação do usuário (20/11/2026):**
  - a Q5 da Parada 1 tinha aprovado os estratos; a verdade de HC-98 a HC-112, RT-66 e RT-67 foi validada na Parada 2 (HC-100, HC-110 e RT-67 conferidos no editor);
  - a DEC-41 foi aprovada, com o risco do caso (b) escrito, e passou a vigente;
  - a linha de base nova (Python 112 chamadas 53/1/14/5/39, Rust 67 chamadas 41/0/1/10/15; fatos 16/9/0/1) passou a ser `tools/precisao/linha-de-base.json`;
  - o portão contra ela deu APROVADO, sem nenhuma chamada mudando de categoria.
- **Commits:** código, testes e interface; documentação; e linha de base, fixtures, folha e ESTADO-ATUAL (hashes no ESTADO-ATUAL). `cargo test` na worktree do commit de código.

## 29. Passo 2.4: Parada 1 e o subpasso 2.4a, escopo da chamada simples e alias (20/11/2026)

**Estado nesta data.** O estado vigente fica no [ESTADO-ATUAL.md](ESTADO-ATUAL.md). Relatórios: [Parada 1 do 2.4](docs/reports/fase-2/2.4-parada-1.md) e [Parada 2 do 2.4a](docs/reports/fase-2/2.4a-parada-2.md); folha: [2.4-revisao-amostra.md](docs/reports/fase-2/2.4-revisao-amostra.md); dados em [docs/reports/fase-2/dados/2.4/](docs/reports/fase-2/dados/2.4/) e [dados/2.4a/](docs/reports/fase-2/dados/2.4a/).

- **Antes de tudo:** portão APROVADO contra a linha de base do 2.3c (Python 53/1/14/5/39, Rust 41/0/1/10/15, fatos 16/9/0/1).
- **O hash do `librarian_studio`** mudou entre a Parada 2 e o commit final do 2.3c sem mudança no código do studio. Duas causas, reproduzidas: o studio carrega a identidade do código (`recorded:<HEAD>:<dirty>`, do `build.rs` do `librarian-evidence`), recompilada a cada mudança em `.git` ou em `crates/`; e o link no Windows não é reproduzível (data do PE e GUID do `.pdb`). O `librarian_ingest` não depende do `librarian-evidence` e não mudou.
- **Parada 1:** contagens por frente (escopo, alias, herança), os elos dos 4 workflows por passo, a regra da herança (MRO; base de fora não decide) e a divisão em 2.4a e 2.4b.
- **Respostas do usuário (20/11):** Q1 trava pelo hash das fontes compiladas, num commit de ferramenta antes do 2.4a; Q2 2.4a e depois 2.4b; Q3 `import` para import/`use` e `scope` só para a definição do próprio módulo, com a mudança do portão num commit separado e provado; Q4–Q6 como no relatório (base de fora não decide); Q7 `cls.m()` e `@property` no 2.4b; Q8 estratos HC-113 a HC-123 e RT-68 a RT-70, verdade proposta pelo agente; Q9 DEC-42 completa na Parada 2. Proposta do usuário para depois do 2.4: 2.4 → 2.5 → Fase 2B → 2.6 (registrada no PLANO).
- **Ferramenta (antes do 2.4a):** o portão grava o hash das fontes compiladas (`crates/` sem os `.md`, `Cargo.toml`, `Cargo.lock`), a identidade embutida e, só como registro, o hash dos binários; `--fontes` recusa uma medição de outras fontes; `scope` entra no `LINKED`. Provas: o portão com cada mudança deu o mesmo veredito sobre a linha de base.
- **2.4a (sem perfil novo, grafo igual byte a byte):** a chamada simples resolvida pelo escopo (Python: ligações em ordem, condicionais, `TYPE_CHECKING`, `import *`; Rust: módulos, `use`, globs, itens de bloco, variáveis) e o alias (`as`, `X = C`, `TypeAlias`, `type X = C;`). Portão APROVADO, com os estratos novos como previsto; `/api/calls` de todos os símbolos: 110 + 24 mudanças, todas previstas, nenhuma fora; `used_by` igual; `cargo test` 521 aprovados; perguntas 01–14 com H1–H3; tela (DEC-30) 88 de 88.
- **Fora da previsão da Parada 1, só nos sintéticos:** o N3c-py-dois-alvos (o `duplo` do próprio módulo, como o `misto()` do Rust) e o parâmetro `fabrica` do caso N0-py-local.
- **Validação do usuário (24/11/2026):** os 14 casos da folha ok (HC-113, HC-119 e RT-69 no editor); DEC-42 vigente, com a condição de só escolher uma definição visível de onde a chamada está. Antes de gravar, duas correções: a trava com o fim de linha normalizado (`5b643d5`) e `global`, `nonlocal` e visibilidade (`49fcfb2`, 6 testes). Nenhum símbolo mudou nos dois projetos. Medição final APROVADA com `--fontes 5d9aae37…`; `cargo test` 527 aprovados, 0 falhas, 2 ignorados. A linha de base passou a 123 + 70 chamadas.

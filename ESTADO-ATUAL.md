# Estado atual do Librarian

**Entrada única de qualquer sessão.** Atualizado em 19/11/2026. Quando este arquivo e outro divergirem, vale este; se este estiver errado, corrija-o antes de seguir.

## Onde estamos

- **Fase 1 — Piloto 1b: concluída em 18/11/2026.** Relatório final: [docs/reports/piloto-1b/00-resumo.md](docs/reports/piloto-1b/00-resumo.md).
- **Fase 2 — Precisão das relações: em andamento.** O passo 2.0 (portão) está em `2b05b8c`. O **passo 2.1** (regra única de resolução, `receiver` corrigido, resolução `type`) está commitado desde 19/11/2026 (`c316bf7` portão, `3b4e1c6` resolver e testes, `6525fb4` documentação, `cb8e5ef` DEC-37, `078fd0e` linha de base), com push feito em 19/11/2026. A amostra RT-31 a RT-55 foi validada pelo usuário em 19/11, e a linha de base foi trocada. Detalhe no CTX, seção 23.
- **Passo 2.2** (forma da chamada, `import`, o tipo que derruba o `unique`; DEC-38): commitado em 19/11/2026 (`a34059c` código, testes e interface; `950c4f4` documentação; depois, linha de base, fixtures e folha). A amostra HC-61 a HC-75 e RT-56 a RT-62 foi validada pelo usuário em 19/11, e a linha de base foi trocada. Parada 2 em [docs/reports/fase-2/2.2-parada-2.md](docs/reports/fase-2/2.2-parada-2.md); CTX seção 24.
- **Passo 2.3: em andamento.** Parada 1 aprovada em 19/11/2026 ([docs/reports/fase-2/2.3-parada-1.md](docs/reports/fase-2/2.3-parada-1.md)), com a divisão em 2.3a-0, 2.3a, 2.3b e 2.3c. O **2.3a-0** (a chave dos métodos pelo último trecho do rótulo, que faz o `receiver` acima do teto valer em Python) está commitado em `8c689d4`, com a documentação no commit seguinte; portão aprovado, igual à linha de base; 109 chamadas de teste passam de `above_candidate_limit` a `receiver`. CTX seção 25.
- **Próximo passo: Parada 2 do 2.3a** (tipos do Python no arquivo, "instancia", regra gravada no `receiver_type`, `modulo.Classe(..)` e `T::V(..)`). Cada passo depende de autorização.
- **Portão de cada passo da Fase 2:** `node tools/precisao/portao.js --studio .studio --base tools/precisao/linha-de-base.json` (cerca de 4 minutos). Reprova se `link_falso` subir ou as corretas caírem, em Python ou em Rust, se uma edition antiga mudar ou se o `.studio` original mudar. Exceção só com justificativa caso a caso e aprovação do usuário (DEC-34).
- **Feito no 1b:** P1 (`987d7a8`), G3 (`aae51d7`), Partes A–E do diagnóstico, a Q6 (run `20261118-205150`, avaliada em 18/11), a consolidação documental (`a8e4074`) e o relatório final. Relatórios em [docs/reports/piloto-1b/](docs/reports/piloto-1b/) (`0c4bcab`).
- **Git:** `main`; a referência é `origin/main` (ver `git log`). Último marco: passo 2.3a-0 (`8c689d4`, em 19/11/2026); antes, o passo 2.2 (`a34059c` código, `950c4f4` documentação e o commit da linha de base, em 19/11/2026); antes, o passo 2.1 (`c316bf7` a `078fd0e`) e o passo 2.0 (`2b05b8c`, `aee6915`).
- **Suíte:** `cargo test --workspace --locked`: 482 aprovados, 0 falhas, 2 ignorados (19/11, passo 2.3a-0).
- **Roadmap:** [PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md#roadmap). Nenhum agente nem orquestrador existe.

## Para onde vamos

- **Critério de pronto antes dos agentes:** um agente consegue remontar os 4 workflows do Healthchecks e caminhos do ray tracer com relações corretas, lacunas explícitas e fatos aguardando aprovação humana.
- **Primeiros agentes (Fase 7):** o de workflows e o de documentação (fluxogramas para humanos). A validação final é humana.

## Regras de trabalho

- Commits em português, sem acentos e sem coautoria; documentação validada pelo usuário antes do commit; push só quando o usuário pedir (DEC-14).
- Não alterar código de produção nem o acervo de fatos fora de um passo autorizado. Não promover fatos.
- Não reabrir decisões sem o passo do roadmap que as reabre ([DECISOES.md](DECISOES.md)).
- Não editar documentos históricos: [docs/historico/](docs/historico/) e as seções datadas do CTX. Estado novo vai aqui; história nova vai numa seção datada do CTX.
- Dados de execução ficam fora do Git, em `.studio/` (`renderer-9b78f3a9` e `healthchecks-25597e41`). Conferir que o `.studio` original não mudou ao fim de cada experimento (comando em [dados/p1/original-before.sha256-agregado.txt](docs/reports/piloto-1b/dados/p1/original-before.sha256-agregado.txt)).
- Toda entrega de interface passa pela verificação da antiga Etapa 7 (DEC-30).
- A verdade de amostras e a aprovação de fatos são do usuário: o agente propõe, com evidência, e marca como proposta até a validação (DEC-22).
- Um teste unitário que passa não basta para dar uma correção por suficiente: validar também pela API e com dados reais.

## Onde cada parte está

| Parte | Onde |
|---|---|
| Acervo, edition e grafo `v2` | `crates/librarian-ingest` (grafo em `src/graph/`) |
| Evidence Builder | crate `crates/librarian-evidence` |
| Dossiês e query preparada | crate `crates/librarian-graph-engine` (o Studio usa de forma transitiva) |
| LLM Engine e claims | `crates/librarian-llm-engine` (contrato em `CLAIMS.md`) |
| Studio: servidor, escada e portão | `crates/librarian-studio` (`ladder.rs`, `main.rs`, `execution.rs`) |
| Interface | `crates/librarian-studio/web/index.html` |
| Projetos de teste | `renderer` (ray tracer em Rust), Healthchecks (`Projetos/pilotos/healthchecks`, `6c05a7a`), `logistica-api` (Python; não está disponível nesta máquina desde o passo 2.2) |

## Convenção de ids

Cite sempre com o prefixo. As tabelas de abreviações e de prefixos estão em [docs/IDS.md](docs/IDS.md).

**Referência a linha:** `ARQUIVO:linha` (ex.: `PIL:161`). **Referência a item:** `PREFIXO-id` (ex.: `PIL-G1`).

## Pendências abertas por fase

**Fase 1** (concluída)
- Push feito em 18/11 e CI verde em `955537e`, inclusive no `d949f91`, o que fecha a pendência do CTX:883.

**Fase 2**
- **Linha de base do portão (binário do passo 2.2; amostra validada, trocada em 19/11/2026):**
  - Python, 75 chamadas: 21 corretas, 1 `link_falso`, 12 `candidatos_falsos`, 8 ambíguas e 33 sem candidato.
  - Rust, 62 chamadas: 36 corretas, 0 `link_falso`, 1 `candidatos_falsos`, 10 ambíguas e 15 sem candidato.
  - As anteriores estão no histórico do Git: a do passo 2.1 (binário de `3b4e1c6`: Python 60 chamadas 9/13/12/12/14, Rust 55 chamadas 36/2/1/9/7) e a do passo 2.0 (binário de `aae51d7`: Python 9/17/10/10/14, Rust com 30 chamadas 5/7/0/11/7).
  - Fatos: 16 não dependem, 8 corretos, 1 suspeito e 1 errado.
  - Detalhe em [tools/precisao/linha-de-base.json](tools/precisao/linha-de-base.json).
- **Do 2.1:** a verdade de RT-31 a RT-55 foi validada pelo usuário em 19/11/2026 (folha em [docs/reports/fase-2/2.1-revisao-amostra.md](docs/reports/fase-2/2.1-revisao-amostra.md)). As respostas da Parada 3 e o status da DEC-11 foram aprovados na Parada 1 do 2.2. Os 60 links `type`/`trait_default` da população ([2.1-links-type.md](docs/reports/fase-2/2.1-links-type.md)) ficam como leitura do agente, sem validação do usuário, fora da linha de base.
- **Do 2.2:** a verdade de HC-61 a HC-75 e RT-56 a RT-62 foi validada pelo usuário em 19/11/2026 (HC-66, HC-69 e RT-62 conferidos no editor; folha em [docs/reports/fase-2/2.2-revisao-amostra.md](docs/reports/fase-2/2.2-revisao-amostra.md)), e a DEC-38 foi aprovada. Pendente:
  - as contagens da Q1 e da Q2 no `logistica-api`. Ficam pendentes: o projeto não está disponível nesta máquina (19/11/2026: `Projetos\logistica-api` não existe, a busca em `C:\`, a única unidade, não o achou, e a única distribuição WSL é a do Docker). No Healthchecks, as duas deram 0 caso. Elas são feitas quando o projeto voltar.
- **Do 2.3 (decisões da Parada 1, 19/11/2026):**
  - DEC-39 (instanciação e tipo do receptor em Python) aprovada pelo usuário, com a exceção do `__getattr__`. O texto entra no DECISOES.md no commit do 2.3a.
  - Q6: um tipo `Protocol` não dá link único; a verdade de `smtpd.py:122` é "ambígua", validada pelo usuário, e entra na fixture no 2.3a.
  - Q7 e Q8 (`modulo.Classe(..)` e `T::V(..)`) entram no 2.3a, com o simulador estendido antes da implementação.
  - Q9 (dono do decorador e do campo, opção A ou B) foi adiada para a Parada 1 do 2.3b, que responde se, com a opção B, o `used_by` e o contexto do dono continuam dizendo que a chamada do decorador pertence à função decorada.
- **Para o 2.3a:** o HC-F6 (`check.ping`, `hc/api/views.py:255`): depois do 2.2, fica `homonym` com um só candidato, o certo, sem link; o tipo de `check` decide.
- **Para o 2.3c:** o extrator do Rust toma `T` como o tipo de `let x = T::f(..)` sem ler o retorno de `f` (32 dos 38 links `type` por variável acima do teto; certos aqui, porque as cinco funções retornam `Self`), e o grafo não guarda qual regra deu o tipo do receptor.
- **Para o 2.3b:** a chamada de um decorador roda na carga do módulo, mas é atribuída à função decorada (HC-19). É a mesma questão de dono do `Repo()` preso ao campo.
- **Limites do 2.2:** um receptor ligado a import de fora do projeto, acima do teto de homônimos, continua `above_candidate_limit` (não há aresta para tirar); `Box`, `Rc`, `Arc`, referência e `Deref` não deixam o tipo de fora do projeto derrubar nada, mesmo quando derrubaria certo.
- Achados sem correção: a ordem de `unresolved` não segue arquivo/linha/coluna; o corte do `node_limit` segue a ordem dos ids das arestas; `self.repo = Repo()` deixa `Repo()` preso ao campo; o tipo é comparado sem os genéricos (`Matrix<3>` = `Matrix<4>`), o que deixa como `homonym` em "o que chama?" o resultado com vários impls.
- `R30-7` (tipo do receptor): feito só no Rust; desde o 2.1, também em "o que chama?". Python é o 2.3.
- Links errados ou ambíguos nos fatos, depois do 2.2: `HC-F1` (errado: o manager do Django é externo, Fase 2B) e `HC-F6` (ambíguo, para o 2.3). `HC-F7` e `HC-F17` passaram a corretos; `HC-F14` não depende de link. Pela leitura do agente, sem revisão humana, os fatos estão corretos sobre o código; o risco é seguir o grafo a partir deles.

**Fase 3**
- **Prioridade alta:** todos os fatos `HC-F1`–`HC-F26` carregam `authorship: "revisão humana no Librarian Studio"` e o portão `human_review`, mas foram revisados pelo agente que conduziu o piloto, **sem revisão humana**. O texto ao modelo também diz "revisadas e aprovadas por uma pessoa" (`PIL-P4`). Os fatos não foram alterados; a correção é da Fase 3 (DEC-22, DEC-24).
- Promoção por item (DEC-23): a Q6 perdeu a resposta inteira pela C3.
- Repetição de fatos entregues (`HC-F20`, `HC-F22`, `HC-F26` na Q6).

**Ambíguos do inventário (INV-D6)**
- `E4-P4` (Ubuntu): houve validação no Ubuntu em 18/11, mas da consolidação. Não está claro se fecha a P4.
- O STUDIO.md descreve a página anterior à reformulação (`EST-20`, F13). A ESP já avisa, no topo, que os nomes das vistas mudaram.

**Trilhas congeladas** (até o fim da Fase 2B, salvo indicação): F11–F13, `SE-5`, `GE-G7`; a Evolução visual volta com o agente de documentação; a comparação de propostas de mudança vai para a Fase 8 (DEC-31).

**Do usuário, sem fase:** commit do `measures` no `logistica-api`; cópia do arquivo de preservação fora da máquina; limpeza de `%TEMP%\lw`, `Temp/lt-ev` e scratchpads; vídeo 13 em espera; exclusão das capas antigas; adiados de propósito (opção B da escada, unificar o `verificavel` em inglês, recriar as baterias antigas). Lista completa: 1B-03, INV-D5.

## Links

- [PLANO](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md): roadmap · [DECISOES](DECISOES.md) · [CTX](CONTEXTO-CONTINUIDADE.md): diário, última seção 25 · [Histórico](docs/historico/README.md)
- Piloto 1: [PIL](docs/reports/PILOTO-1-HEALTHCHECKS.md) · Piloto 1b: [00 resumo](docs/reports/piloto-1b/00-resumo.md), [01 G3](docs/reports/piloto-1b/01-g3.md), [02 Partes A–C](docs/reports/piloto-1b/02-fechamento-g3-e-diagnostico.md), [03 inventário e Parte E](docs/reports/piloto-1b/03-inventario-e-auditoria.md), [04 Q6](docs/reports/piloto-1b/04-q6.md)
- Direção: [ARQ](docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md) · Frontend: [ESP](docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md), [EST](docs/ESTADO-IMPLEMENTACAO-STUDIO.md) · Manual: [STUDIO.md](STUDIO.md)
- Componentes: [Ingest](crates/librarian-ingest/README.md) · [Evidence Builder](crates/librarian-evidence/README.md) · [LLM Engine](crates/librarian-llm-engine/CLAIMS.md) · [Graph Engine](crates/librarian-graph-engine/WORKFLOW-V0.1.md)

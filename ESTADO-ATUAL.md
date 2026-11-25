# Estado atual do Librarian

**Entrada única de qualquer sessão.** Atualizado em 25/11/2026. Quando este arquivo e outro divergirem, vale este; se este estiver errado, corrija-o antes de seguir.

## Onde estamos

- **Fase 1 — Piloto 1b: concluída em 18/11/2026.** Relatório final: [docs/reports/piloto-1b/00-resumo.md](docs/reports/piloto-1b/00-resumo.md).
- **Fase 2 — Precisão das relações: em andamento.** O passo 2.0 (portão) está em `2b05b8c`. O **passo 2.1** (regra única de resolução, `receiver` corrigido, resolução `type`) está commitado desde 19/11/2026 (`c316bf7` portão, `3b4e1c6` resolver e testes, `6525fb4` documentação, `cb8e5ef` DEC-37, `078fd0e` linha de base), com push feito em 19/11/2026. A amostra RT-31 a RT-55 foi validada pelo usuário em 19/11, e a linha de base foi trocada. Detalhe no CTX, seção 23.
- **Passo 2.2** (forma da chamada, `import`, o tipo que derruba o `unique`; DEC-38): commitado em 19/11/2026 (`a34059c` código, testes e interface; `950c4f4` documentação; depois, linha de base, fixtures e folha). A amostra HC-61 a HC-75 e RT-56 a RT-62 foi validada pelo usuário em 19/11, e a linha de base foi trocada. Parada 2 em [docs/reports/fase-2/2.2-parada-2.md](docs/reports/fase-2/2.2-parada-2.md); CTX seção 24.
- **Passo 2.3: em andamento.** Parada 1 aprovada em 19/11/2026 ([docs/reports/fase-2/2.3-parada-1.md](docs/reports/fase-2/2.3-parada-1.md)), com a divisão em 2.3a-0, 2.3a, 2.3b e 2.3c. O **2.3a-0** (a chave dos métodos pelo último trecho do rótulo, que faz o `receiver` acima do teto valer em Python) está commitado em `8c689d4`, com a documentação no commit seguinte; portão aprovado, igual à linha de base; 109 chamadas de teste passam de `above_candidate_limit` a `receiver`. CTX seção 25.
- **Passo 2.3a** (tipos do Python no arquivo, relação "instancia", `type_rule` no `receiver_type`, `modulo.Classe(..)` e `T::V(..)`; DEC-39; perfil V9, grafo `rust-python-syntax-graph-v6`): commitado em 20/11/2026 (`5589912` código, testes e interface; `04943c5` documentação; depois, linha de base, fixtures e folha). A amostra HC-76 a HC-87 e RT-63 a RT-65 foi validada pelo usuário em 20/11 (HC-76, HC-81 e RT-63 conferidos no editor; HC-79 na Q6), e a linha de base foi trocada. "Instancia": 655 links no Healthchecks (574 já iam para a classe e só ganharam `relation`; 81 são novos) e 13 no ray tracer (2 e 11). A pergunta congelada B5 (`tests/agnostic.rs`) gera a edition no perfil V8, em que foi congelada, e não exercita o V9. Parada 2 em [docs/reports/fase-2/2.3a-parada-2.md](docs/reports/fase-2/2.3a-parada-2.md); CTX seção 26.
- **Passo 2.3b** (dono do decorador e do campo; opção B, pela regra, sem edition nova; DEC-40, vigente desde 20/11/2026): Parada 1 aprovada em 20/11/2026 ([2.3b-parada-1.md](docs/reports/fase-2/2.3b-parada-1.md)). Parada 2 validada em 20/11/2026 ([2.3b-parada-2.md](docs/reports/fase-2/2.3b-parada-2.md)). Commitado em 20/11/2026: `6b424a0` (portão, com a prova da DEC-40), `5a26630` (código, testes e interface), `944fd0a` (documentação) e, depois, linha de base, fixtures e folha. A amostra HC-88 a HC-97 foi validada pelo usuário (HC-89 e HC-97 conferidos no editor), e a linha de base foi trocada.
  - **Decorador:** uma chamada num decorador sai com `relation: decorates` e não é seguida.
  - **Campo:** o método responde também pelas chamadas dos campos que declara, com `via`.
  - **Resultado:** portão APROVADO, igual à linha de base; 69 links passam a `decorates` e 7 chamadas de campo aparecem no `__init__`, nenhuma mudança fora da previsão; `used_by` igual em todos os símbolos. CTX seção 27.
- **Passo 2.3c** (tipo pelo retorno anotado, no mesmo arquivo e entre arquivos; campo de classe de outro arquivo; retorno de `T::f` no Rust; sem perfil novo; DEC-41, vigente desde 20/11/2026, com o risco do caso b): Parada 1 aprovada em 20/11/2026 ([2.3c-parada-1.md](docs/reports/fase-2/2.3c-parada-1.md)), com a regra da Q1 em quatro casos: (a) a chamada que liga fecha com um alvo; (b) não fecha, mas todos os candidatos declaram o mesmo retorno; (c) não tem candidato no projeto: vale o que valia; (d) os candidatos discordam: sem tipo. Parada 2 validada em 20/11/2026 ([2.3c-parada-2.md](docs/reports/fase-2/2.3c-parada-2.md)). Commitado em 20/11/2026: `efb346a` (código, testes e interface), `4924ae9` (documentação) e, depois, linha de base, fixtures e folha. A verdade de HC-98 a HC-112, RT-66 e RT-67 foi validada pelo usuário na Parada 2 (HC-100, HC-110 e RT-67 conferidos no editor), e a linha de base foi trocada.
  - **Resultado:** portão APROVADO; 8 chamadas do Python passam de `homonym` a `type` (HC-110 pelo caso b), nenhuma no Rust; o grafo não muda. Linha de base: Python (112) 53/1/14/5/39, Rust (67) 41/0/1/10/15, fatos 16/9/0/1. CTX seção 28.
  - **O 2.3 está fechado** (fechamento no PLANO).
- **Passo 2.4: em andamento.** Parada 1 aprovada em 20/11/2026 ([2.4-parada-1.md](docs/reports/fase-2/2.4-parada-1.md)), com a divisão em 2.4a (escopo da chamada simples e alias) e 2.4b (herança, com `cls.m()` e a `@property` de retorno anotado). CTX seção 29.
  - **Antes do 2.4a, só ferramenta:** a trava do portão pelo hash das fontes compiladas (`crates/` sem os `.md`, `Cargo.toml`, `Cargo.lock`), porque o hash do `librarian_studio` muda a cada link no Windows e a cada commit (identidade embutida); e `scope` como resolução de alvo único no portão. As duas com prova: veredito igual sobre a linha de base.
  - **2.4a: fechado em 24/11/2026** (DEC-42, vigente): Parada 2 em [2.4a-parada-2.md](docs/reports/fase-2/2.4a-parada-2.md), código e ferramenta commitados (`926416f` trava, `4ad8102` `scope` no portão, `fd388cd` código, `b1fb45f` documentação, `5b643d5` trava com o fim de linha normalizado, `49fcfb2` `global` e visibilidade; depois, a linha de base). Folha: [2.4-revisao-amostra.md](docs/reports/fase-2/2.4-revisao-amostra.md) (HC-113 a HC-123 e RT-68 a RT-70, verdade validada pelo usuário em 24/11/2026).
  - **Ordem do resto da Fase 2 (proposta do usuário, a decidir depois do 2.4):** 2.4 → 2.5 → Fase 2B → 2.6.
  - **2.4b: fechado em 25/11/2026** ([2.4b-parada-1.md](docs/reports/fase-2/2.4b-parada-1.md), [2.4b-parada-2.md](docs/reports/fase-2/2.4b-parada-2.md), folha [2.4b-revisao-amostra.md](docs/reports/fase-2/2.4b-revisao-amostra.md), DEC-43 vigente). Commits: `6a8d0e3` (portão lê `ambigua`), `6dc0e4a` (código, testes, sintéticos e página), `9ec5abf` (documentação) e, depois, linha de base, fixtures e folha. 198 mudanças, todas previstas. A verdade de HC-124 a HC-143 foi validada pelo usuário em 25/11/2026 (HC-137 e HC-141 no editor). CTX seção 30.
    - **HC-140 e HC-142 são `link_falso` conhecidos:** o usuário corrigiu a verdade para ambígua (o corpo que roda pode ser o da subclasse). **O 2.4c os corrige.** HC-138 e HC-139 conferidos: nenhuma subclasse sobrescreve `request` nem `post`.
- **Próximo passo:** a Parada 1 do 2.4c (despacho polimórfico, no PLANO, antes do 2.5). Cada passo depende de autorização.
- **Portão de cada passo da Fase 2:** `node tools/precisao/portao.js --studio .studio --base tools/precisao/linha-de-base.json` (cerca de 4 minutos). Reprova se `link_falso` subir ou as corretas caírem, em Python ou em Rust, se uma edition antiga mudar ou se o `.studio` original mudar. Exceção só com justificativa caso a caso e aprovação do usuário (DEC-34). Desde o 2.4, o relatório registra o hash das fontes compiladas (a trava), e a medição da linha de base final usa `--fontes <hash da Parada 2>`.
- **Feito no 1b:** P1 (`987d7a8`), G3 (`aae51d7`), Partes A–E do diagnóstico, a Q6 (run `20261118-205150`, avaliada em 18/11), a consolidação documental (`a8e4074`) e o relatório final. Relatórios em [docs/reports/piloto-1b/](docs/reports/piloto-1b/) (`0c4bcab`).
- **Git:** `main`; a referência é `origin/main` (ver `git log`). Último marco: passo 2.4b (`6a8d0e3`, `6dc0e4a`, `9ec5abf`); antes, o passo 2.4a (`fd388cd`, `49fcfb2`); o passo 2.3c (`efb346a` código, `4924ae9` documentação e o commit da linha de base, em 20/11/2026); antes, o passo 2.3b (`6b424a0`, `5a26630`, `944fd0a`, `f50b8f8`), o passo 2.3a (`5589912`, `04943c5`, `a11a582`), o passo 2.3a-0 (`8c689d4`, `1152875`), o passo 2.2 (`a34059c`, `950c4f4`), o passo 2.1 (`c316bf7` a `078fd0e`) e o passo 2.0 (`2b05b8c`, `aee6915`).
- **Suíte:** `cargo test --workspace --locked`: 534 aprovados, 0 falhas, 2 ignorados (25/11, worktree limpa do `9ec5abf`). Antes: 527 (24/11, worktree limpa do `49fcfb2`). Antes: 509 aprovados, 0 falhas, 2 ignorados (20/11, passo 2.3c; 12 testes novos em `tests/retorno_2_3c.rs`; conferido também numa worktree do commit `efb346a`).
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
- **Linha de base do portão (binário do passo 2.4b; amostra validada, trocada em 25/11/2026; `--fontes 160c59f54c3a747472557dccfc374fd5cc40a59600e5a3e78e270e3634328ec4`):**
  - Python, 143 chamadas: 73 corretas, 3 `link_falso` (HC-27, e HC-140 e HC-142, que o 2.4c corrige), 14 `candidatos_falsos`, 5 ambíguas e 48 sem candidato.
  - Rust, 70 chamadas: 44 corretas, 0 `link_falso`, 1 `candidatos_falsos`, 10 ambíguas e 15 sem candidato.
  - As anteriores estão no histórico do Git: a do passo 2.4a (Python 123 chamadas 60/1/14/5/43, Rust 70 chamadas 44/0/1/10/15), a do 2.3c (Python 112 chamadas 53/1/14/5/39, Rust 67 chamadas 41/0/1/10/15), a do 2.3b, a do 2.3a, a do 2.2, a do 2.1 e a do 2.0.
  - Fatos: 15 não dependem, 9 corretos, 1 suspeito (F23, despacho dinâmico) e 1 errado.
  - Detalhe em [tools/precisao/linha-de-base.json](tools/precisao/linha-de-base.json).
- **Do 2.1:** a verdade de RT-31 a RT-55 foi validada pelo usuário em 19/11/2026 (folha em [docs/reports/fase-2/2.1-revisao-amostra.md](docs/reports/fase-2/2.1-revisao-amostra.md)). As respostas da Parada 3 e o status da DEC-11 foram aprovados na Parada 1 do 2.2. Os 60 links `type`/`trait_default` da população ([2.1-links-type.md](docs/reports/fase-2/2.1-links-type.md)) ficam como leitura do agente, sem validação do usuário, fora da linha de base.
- **Do 2.2:** a verdade de HC-61 a HC-75 e RT-56 a RT-62 foi validada pelo usuário em 19/11/2026 (HC-66, HC-69 e RT-62 conferidos no editor; folha em [docs/reports/fase-2/2.2-revisao-amostra.md](docs/reports/fase-2/2.2-revisao-amostra.md)), e a DEC-38 foi aprovada. Pendente:
  - as contagens da Q1 e da Q2 no `logistica-api`. Ficam pendentes: o projeto não está disponível nesta máquina (19/11/2026: `Projetos\logistica-api` não existe, a busca em `C:\`, a única unidade, não o achou, e a única distribuição WSL é a do Docker). No Healthchecks, as duas deram 0 caso. Elas são feitas quando o projeto voltar.
- **Do 2.3a:** a verdade de HC-76 a HC-87 e RT-63 a RT-65 foi validada pelo usuário (folha em [docs/reports/fase-2/2.3-revisao-amostra.md](docs/reports/fase-2/2.3-revisao-amostra.md)), e a DEC-39 foi aprovada. HC-F6 ficou correto (`type` → `Check.ping`, pela anotação). Limites: o `__init__` herdado não é seguido (2.4); o `__init__` que uma instanciação roda aparece no detalhe, mas não como caixa na profundidade 1.
- **Do 2.3b:** a verdade de HC-88 a HC-97 foi validada pelo usuário em 20/11/2026 (HC-89 e HC-97 conferidos no editor; folha em [docs/reports/fase-2/2.3b-revisao-amostra.md](docs/reports/fase-2/2.3b-revisao-amostra.md)), e a DEC-40 foi aprovada.
  - Um link que o percurso não segue (`decorates`, ou `instancia` sem `__init__`) diz "decidida, sem seguir" no detalhe, não "seguida".
  - O portão compila o binário antes de rodar e registra o hash dele.
  - As 48 chamadas soltas no topo dos arquivos Python estão registradas na Fase 2B do PLANO (Q2).
  - Limite: o módulo, que é quem roda um decorador de topo, não é símbolo e não aparece em lugar nenhum (lacuna declarada; Fase 2B).
- **Do 2.3c:** o grafo continua gravando `caminho` em `let x = T::f(..)`; a regra efetiva sai no `type_rule` do `/api/calls` (`retorno`, `campo`). Limites: `let x = T::f(..)?` perde o tipo (Q2); `Box<Self>` não decide; campo de struct de outro arquivo no Rust não é lido; no caso (d), o retorno gravado do mesmo arquivo deixa de valer (nenhum caso nos dados; vai para o 2.4 com o N0-livre). Risco do caso (b), escrito na DEC-41: um `homonym` de um só candidato dá tipo; único efeito, a HC-110.
- **Limites do 2.2:** um receptor ligado a import de fora do projeto, acima do teto de homônimos, continua `above_candidate_limit` (não há aresta para tirar); `Box`, `Rc`, `Arc`, referência e `Deref` não deixam o tipo de fora do projeto derrubar nada, mesmo quando derrubaria certo.
- **Do 2.4b:** o envelope lista como lacuna ("combina com N declarações") uma chamada acima do teto que a regra já resolve (`type`, `receiver`). Vem de antes do 2.4b, que aumenta os casos (`super().setUp()`). Leitura do agente; sem passo. O inventário não foi comparado por inteiro. Limites no SEARCH.md.
- Propostas do usuário a decidir, no PLANO (25/11/2026): o agente aprovador (Fase 3, 3a e 3b; reabre a DEC-22), a Fase 6 (6.1 a 6.5) e o front refeito depois do 6.5.
- Achados sem correção: a ordem de `unresolved` não segue arquivo/linha/coluna; o corte do `node_limit` segue a ordem dos ids das arestas; o tipo é comparado sem os genéricos (`Matrix<3>` = `Matrix<4>`), o que deixa como `homonym` em "o que chama?" o resultado com vários impls.
- `R30-7` (tipo do receptor): feito no Rust desde o 2.1 e no Python com o 2.3a (dentro do arquivo) e o 2.3c (entre arquivos, pelo retorno e pelo campo).
- Links errados ou ambíguos nos fatos, depois do 2.3a: `HC-F1` (errado: o manager do Django é externo, Fase 2B). `HC-F6` passou a correto no 2.3a; `HC-F7` e `HC-F17`, no 2.2; `HC-F14` não depende de link. Pela leitura do agente, sem revisão humana, os fatos estão corretos sobre o código; o risco é seguir o grafo a partir deles.

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

- [PLANO](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md): roadmap · [DECISOES](DECISOES.md) · [CTX](CONTEXTO-CONTINUIDADE.md): diário, última seção 30 · [Histórico](docs/historico/README.md)
- Piloto 1: [PIL](docs/reports/PILOTO-1-HEALTHCHECKS.md) · Piloto 1b: [00 resumo](docs/reports/piloto-1b/00-resumo.md), [01 G3](docs/reports/piloto-1b/01-g3.md), [02 Partes A–C](docs/reports/piloto-1b/02-fechamento-g3-e-diagnostico.md), [03 inventário e Parte E](docs/reports/piloto-1b/03-inventario-e-auditoria.md), [04 Q6](docs/reports/piloto-1b/04-q6.md)
- Direção: [ARQ](docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md) · Frontend: [ESP](docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md), [EST](docs/ESTADO-IMPLEMENTACAO-STUDIO.md) · Manual: [STUDIO.md](STUDIO.md)
- Componentes: [Ingest](crates/librarian-ingest/README.md) · [Evidence Builder](crates/librarian-evidence/README.md) · [LLM Engine](crates/librarian-llm-engine/CLAIMS.md) · [Graph Engine](crates/librarian-graph-engine/WORKFLOW-V0.1.md)

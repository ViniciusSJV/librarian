# Estado atual do Librarian

**Entrada única de qualquer sessão.** Atualizado em 18/11/2026. Quando este arquivo e outro divergirem, vale este; se este estiver errado, corrija-o antes de seguir.

## Onde estamos

- **Fase 1 — Piloto 1b: concluída em 18/11/2026.** Relatório final: [docs/reports/piloto-1b/00-resumo.md](docs/reports/piloto-1b/00-resumo.md).
- **Próximo passo: 2.0**, tornar a amostra da Parte C e os scripts da auditoria (Parte E) executáveis a partir do repositório, sem caminhos do scratchpad. É o portão da Fase 2. Cada passo depende de autorização.
- **Feito no 1b:** P1 (`987d7a8`), G3 (`aae51d7`), Partes A–E do diagnóstico, a Q6 (run `20261118-205150`, avaliada em 18/11), a consolidação documental (`a8e4074`) e o relatório final. Relatórios em [docs/reports/piloto-1b/](docs/reports/piloto-1b/) (`0c4bcab`).
- **Git:** `main`, sincronizada: os commits do 1b foram enviados em 18/11, e `origin/main` está em `955537e`.
- **Suíte:** `cargo test --workspace --locked`: 456 aprovados, 0 falhas, 2 ignorados (18/11, com P1 e G3).
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
| Projetos de teste | `renderer` (ray tracer em Rust), Healthchecks (`Projetos/pilotos/healthchecks`, `6c05a7a`), `logistica-api` (Python) |

## Convenção de ids

Cite sempre com o prefixo. As tabelas de abreviações e de prefixos estão em [docs/IDS.md](docs/IDS.md).

**Referência a linha:** `ARQUIVO:linha` (ex.: `PIL:161`). **Referência a item:** `PREFIXO-id` (ex.: `PIL-G1`).

## Pendências abertas por fase

**Fase 1** (concluída)
- Push feito em 18/11 e CI verde em `955537e`, inclusive no `d949f91`, o que fecha a pendência do CTX:883.

**Fase 2**
- 2.0 é o portão: a amostra da Parte C e os scripts da Parte E ainda usam caminhos do scratchpad ([dados/amostra/](docs/reports/piloto-1b/dados/amostra/), [dados/acervo/](docs/reports/piloto-1b/dados/acervo/)).
- Achados sem correção: a ordem de `unresolved` não segue arquivo/linha/coluna; o corte do `node_limit` segue a ordem dos ids das arestas; `self.repo = Repo()` deixa `Repo()` preso ao campo; "o que chama?" não usa `receiver_type`; 197 dos 442 links únicos de método vão para função solta.
- `R30-7` (tipo do receptor): feito só no Rust e só no inventário.
- Links errados ou ambíguos nos fatos: `HC-F1`, `HC-F14`, `HC-F17` (errado), `HC-F6`, `HC-F7` (ambíguo). Pela leitura do agente, sem revisão humana, os fatos estão corretos sobre o código; o risco é seguir o grafo a partir deles.

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

- [PLANO](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md): roadmap · [DECISOES](DECISOES.md) · [CTX](CONTEXTO-CONTINUIDADE.md): diário, última seção 21 · [Histórico](docs/historico/README.md)
- Piloto 1: [PIL](docs/reports/PILOTO-1-HEALTHCHECKS.md) · Piloto 1b: [00 resumo](docs/reports/piloto-1b/00-resumo.md), [01 G3](docs/reports/piloto-1b/01-g3.md), [02 Partes A–C](docs/reports/piloto-1b/02-fechamento-g3-e-diagnostico.md), [03 inventário e Parte E](docs/reports/piloto-1b/03-inventario-e-auditoria.md), [04 Q6](docs/reports/piloto-1b/04-q6.md)
- Direção: [ARQ](docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md) · Frontend: [ESP](docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md), [EST](docs/ESTADO-IMPLEMENTACAO-STUDIO.md) · Manual: [STUDIO.md](STUDIO.md)
- Componentes: [Ingest](crates/librarian-ingest/README.md) · [Evidence Builder](crates/librarian-evidence/README.md) · [LLM Engine](crates/librarian-llm-engine/CLAIMS.md) · [Graph Engine](crates/librarian-graph-engine/WORKFLOW-V0.1.md)

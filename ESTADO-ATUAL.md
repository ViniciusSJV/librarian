# Estado atual do Librarian

**Entrada única de qualquer sessão.** Atualizado em 18/11/2026. Quando este arquivo e outro divergirem, vale este; se este estiver errado, corrija-o antes de seguir.

## Onde estamos

- **Fase 1 — Piloto 1b**, passo **1.2 (consolidação documental)**, em validação pelo usuário.
- **Próximo passo: 1.3**, relatório final do 1b. Depois, a Fase 2, começando pelo passo 2.0. Cada passo depende de autorização.
- **Feito no 1b:** P1 (`987d7a8`), G3 (`aae51d7`), Partes A–E do diagnóstico e a Q6 (run `20261118-205150`, avaliada em 18/11). Relatórios em [docs/reports/piloto-1b/](docs/reports/piloto-1b/).
- **Git:** `main`. Três commits sem push sobre `origin/main` (`0398829`, `987d7a8`, `aae51d7`), mais os desta consolidação quando forem aprovados.
- **Suíte:** `cargo test --workspace --locked`: 456 aprovados, 0 falhas, 2 ignorados (18/11, com P1 e G3).
- **Roadmap:** [PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md#roadmap). Nenhum agente nem orquestrador existe.

## Regras de trabalho

- Commits em português, sem acentos e sem coautoria; documentação validada pelo usuário antes do commit; push só quando o usuário pedir (DEC-14).
- Não alterar código de produção nem o acervo de fatos fora de um passo autorizado. Não promover fatos.
- Não reabrir decisões sem o passo do roadmap que as reabre ([DECISOES.md](DECISOES.md)).
- Não editar documentos históricos: [docs/historico/](docs/historico/) e as seções datadas do CTX. Estado novo vai aqui; história nova vai numa seção datada do CTX.
- Dados de execução ficam fora do Git, em `.studio/` (`renderer-9b78f3a9` e `healthchecks-25597e41`). Conferir que o `.studio` original não mudou ao fim de cada experimento (comando em [dados/p1/original-before.sha256-agregado.txt](docs/reports/piloto-1b/dados/p1/original-before.sha256-agregado.txt)).
- Toda entrega de interface passa pela verificação da antiga Etapa 7 (DEC-30).
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

Muitos ids se repetem com sentidos diferentes nos documentos antigos. Daqui em diante, cite sempre com o prefixo. Nada foi renomeado nos documentos históricos: lá, o prefixo é implícito pelo documento e pela data.

**Referência a linha:** `ARQUIVO:linha` (ex.: `PIL:161`). **Referência a item:** `PREFIXO-id` (ex.: `PIL-G1`).

| Abreviação | Arquivo |
|---|---|
| ESTADO | este arquivo |
| DEC | [DECISOES.md](DECISOES.md) |
| PLANO | [PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md](PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md), só o roadmap |
| PLANO-H | [docs/historico/PLANO-ate-18-11.md](docs/historico/PLANO-ate-18-11.md), mesmas linhas do plano em `aae51d7` |
| CTX | [CONTEXTO-CONTINUIDADE.md](CONTEXTO-CONTINUIDADE.md), diário datado |
| PIL | [docs/reports/PILOTO-1-HEALTHCHECKS.md](docs/reports/PILOTO-1-HEALTHCHECKS.md) |
| 1B-01 a 1B-04 | os relatórios em [docs/reports/piloto-1b/](docs/reports/piloto-1b/) |
| ARQ, ESP, EST | [arquitetura](docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md), [especificação](docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md) e [estado](docs/ESTADO-IMPLEMENTACAO-STUDIO.md) do frontend |

| Prefixo | Família | Onde nasceu | Não confundir com |
|---|---|---|---|
| `Fase N`, `N.M` | Fases e passos do roadmap atual | PLANO | Etapas e passos antigos |
| `DEC-NN` | Decisões | DEC | D1–D16 |
| `PIL-G1`–`PIL-G8` | Limitações do grafo no Piloto 1 | PIL:265–272 | `GE-G1`–`GE-G7` |
| `PIL-P1`–`PIL-P4` | Limitações de evidências e fatos no Piloto 1 | PIL:283–288 | `E4-P1`–`E4-P4` |
| `PIL-Q1`–`PIL-Q6` | Perguntas do Piloto 1 (turns 01–14); `PIL-Q6/C3` é a claim C3 de uma resposta | PIL:171–186 | `ESP-Q1`–`ESP-Q12` |
| `HC-F1`–`HC-F26` | Fatos do acervo do Healthchecks (citados também só como F1–F26) | PIL:213–246 | `ESP-F0`–`ESP-F13` |
| `1B-A`–`1B-E` | Partes do diagnóstico do Piloto 1b | 1B-02, 1B-03 | — |
| `INV-D1`–`INV-D6` | Seções do inventário dos MDs | 1B-03 | demais D |
| `N0`–`N8` | Níveis de tipos (Python e Rust) | 1B-02, Parte B | — |
| `GE-G1`–`GE-G7` | Graph Engine: roteiro de 19/10 (G1–G5), impacto (G6a–d), IaC (G7) | PLANO-H:385–864 | `PIL-G*` |
| `L1`–`L4` | Fase L, limites do LLM Engine | PLANO-H:498–716 | — |
| `R30-1`–`R30-10` | Etapas do roteiro de 30/10 ("item 7 de 30/10" = `R30-7`) | PLANO-H:865–950 | Etapas do Studio |
| `FR-R1`–`FR-R6` | Fase R (renderer) | PLANO-H:951–1069 | `E3-R1`–`E3-R5` |
| `CTX-E1`–`CTX-E11` | Experimentos | CTX §5 | `CONS-E1`–`CONS-E5` |
| `CONS-E1`–`CONS-E5` | Consolidação do Evidence Builder (17/11) | PLANO-H:2061 | `CTX-E*` |
| `SIMP-D1`–`SIMP-D16` | Decisões da simplificação de 17/11 (fora do repositório) | PLANO-H:2072 | demais D |
| `SE-0`–`SE-7` | Etapas 0–7 da frente do Studio (05/11) | PLANO-H:1602 | `DIAG-ETAPA2`, `R30-*` |
| `E2-D1`–`E2-D15`, `E2-C3` | Defeitos e mudança de texto da Etapa 2 do Studio | PLANO-H:1681–1715, CTX §17 | `ESP-D*` |
| `E3-PassoN`, `E3-D6`–`E3-D8`, `E3-R1`–`E3-R5`, `E3-C4` | Passos, defeitos, ressalvas e condição da Etapa 3 | PLANO-H:1728–1930, CTX §12–16 | `FR-R*` |
| `E4-PassoN`, `E4-P1`–`E4-P4` | Passos 0–15 e pendências da Etapa 4 | PLANO-H:1931–1994 | `PIL-P*` |
| `E5-C1`, `E5-C2` | `/api/ladder` com `turn`; `GET /api/source` | PLANO-H:1997–1998 | claims C1–C5 |
| `DIAG-ETAPA2` | ETAPA 2 (2A e 2B) do diagnóstico de 27/09 | PLANO-H:375 | `SE-2` |
| `ESP-F0`–`ESP-F13`, `ESP-B1`–`ESP-B7` | Etapas e passos de backend do frontend | ESP §10 | `HC-F*` |
| `ESP-D1`–`ESP-D11`, `ESP-I1`–`ESP-I8`, `ESP-Q1`–`ESP-Q12`, `ESP-T1`–`ESP-T26` | Dependências, questões e transições da especificação | ESP | demais D, Q |
| `EST-1`–`EST-76`, `EST-M1`, `EST-M2` | Pendências e defeitos da implementação do frontend | EST:1561–1640 | — |
| `H1`–`H4`, Formato 1–3 | Conferências e formatos do pacote | CTX §17 | — |

## Pendências abertas por fase

**Fase 1**
- 1.2: validação desta consolidação e os dois commits.
- 1.3: relatório final do 1b.
- Conferir o CI do commit de 18/11 (CTX:883) e, depois do push, o dos commits do 1b.

**Fase 2**
- 2.0 é o portão: a amostra da Parte C e os scripts da Parte E ainda usam caminhos do scratchpad ([dados/amostra/](docs/reports/piloto-1b/dados/amostra/), [dados/acervo/](docs/reports/piloto-1b/dados/acervo/)).
- Achados sem correção: a ordem de `unresolved` não segue arquivo/linha/coluna; o corte do `node_limit` segue a ordem dos ids das arestas; `self.repo = Repo()` deixa `Repo()` preso ao campo; "o que chama?" não usa `receiver_type`; 197 dos 442 links únicos de método vão para função solta.
- `R30-7` (tipo do receptor): feito só no Rust e só no inventário.
- Links errados ou ambíguos nos fatos: `HC-F1`, `HC-F14`, `HC-F17` (errado), `HC-F6`, `HC-F7` (ambíguo). Os fatos estão corretos sobre o código; o risco é seguir o grafo a partir deles.

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
- Piloto 1: [PIL](docs/reports/PILOTO-1-HEALTHCHECKS.md) · Piloto 1b: [01 G3](docs/reports/piloto-1b/01-g3.md), [02 Partes A–C](docs/reports/piloto-1b/02-fechamento-g3-e-diagnostico.md), [03 inventário e Parte E](docs/reports/piloto-1b/03-inventario-e-auditoria.md), [04 Q6](docs/reports/piloto-1b/04-q6.md)
- Direção: [ARQ](docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md) · Frontend: [ESP](docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md), [EST](docs/ESTADO-IMPLEMENTACAO-STUDIO.md) · Manual: [STUDIO.md](STUDIO.md)
- Componentes: [Ingest](crates/librarian-ingest/README.md) · [Evidence Builder](crates/librarian-evidence/README.md) · [LLM Engine](crates/librarian-llm-engine/CLAIMS.md) · [Graph Engine](crates/librarian-graph-engine/WORKFLOW-V0.1.md)

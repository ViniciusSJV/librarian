# Convenção de ids

Muitos ids se repetem com sentidos diferentes nos documentos antigos. Daqui em diante, cite sempre com o prefixo. Nada foi renomeado nos documentos históricos: lá, o prefixo é implícito pelo documento e pela data. A regra de citação está no [ESTADO-ATUAL.md](../ESTADO-ATUAL.md#convenção-de-ids).

**Referência a linha:** `ARQUIVO:linha` (ex.: `PIL:161`). **Referência a item:** `PREFIXO-id` (ex.: `PIL-G1`).

## Abreviações dos arquivos

| Abreviação | Arquivo |
|---|---|
| ESTADO | [ESTADO-ATUAL.md](../ESTADO-ATUAL.md) |
| DEC | [DECISOES.md](../DECISOES.md) |
| PLANO | [PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md](../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md), só o roadmap |
| PLANO-H | [docs/historico/PLANO-ate-18-11.md](historico/PLANO-ate-18-11.md), mesmas linhas do plano em `aae51d7` |
| CTX | [CONTEXTO-CONTINUIDADE.md](../CONTEXTO-CONTINUIDADE.md), diário datado |
| PIL | [docs/reports/PILOTO-1-HEALTHCHECKS.md](reports/PILOTO-1-HEALTHCHECKS.md) |
| 1B-00 a 1B-04 | os relatórios em [docs/reports/piloto-1b/](reports/piloto-1b/) |
| ARQ, ESP, EST | [arquitetura](ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md), [especificação](ESPECIFICACAO-FUNCIONAL-STUDIO.md) e [estado](ESTADO-IMPLEMENTACAO-STUDIO.md) do frontend |

## Prefixos

| Prefixo | Família | Onde nasceu | Não confundir com |
|---|---|---|---|
| `Fase N`, `N.M` | Fases e passos do roadmap atual | PLANO | Etapas e passos antigos |
| `DEC-NN` | Decisões | DEC | D1–D16 |
| `PIL-G1`–`PIL-G8` | Limitações do grafo no Piloto 1 | PIL:265–272 | `GE-G1`–`GE-G7` |
| `PIL-P1`–`PIL-P4` | Limitações de evidências e fatos no Piloto 1 | PIL:283–288 | `E4-P1`–`E4-P4` |
| `PIL-Q1`–`PIL-Q6` | Perguntas do Piloto 1 (turns 01–14); `PIL-Q6/C3` é a claim C3 de uma resposta | PIL:171–186 | `ESP-Q1`–`ESP-Q12` |
| `HC-F1`–`HC-F26` | Fatos do acervo do Healthchecks (citados também só como F1–F26) | PIL:213–246 | `ESP-F0`–`ESP-F13`, `HC-1`–`HC-60` |
| `HC-1`–`HC-171`, `RT-1`–`RT-70` | Chamadas da amostra do portão da Fase 2: Healthchecks e ray tracer. RT-31–RT-55 vêm do passo 2.1 (validadas em 19/11); HC-61–HC-75 e RT-56–RT-62 vêm do passo 2.2 (validadas em 19/11); HC-76–HC-87 e RT-63–RT-65 vêm do passo 2.3a (validadas em 19/11 e 20/11); HC-88–HC-97 vêm do passo 2.3b (validadas em 20/11); HC-98–HC-112 e RT-66–RT-67 vêm do passo 2.3c (validadas em 20/11, na Parada 2); HC-113–HC-123 e RT-68–RT-70 vêm do passo 2.4a (validadas em 24/11); HC-124–HC-143 vêm do passo 2.4b (validadas em 25/11; HC-140 e HC-142 com a verdade corrigida pelo usuário); HC-144–HC-157 vêm do passo 2.4c (validadas em 25/11); HC-158–HC-171 vêm do passo 2.4d-0 (validadas em 25/11) | [tools/precisao/fixtures/](../tools/precisao/fixtures/) (`amostra-hc.json`, `amostra-rt.json`) | `HC-F*` (fatos, não chamadas) |
| `UB-1`–`UB-28` | Símbolos da amostra "quem usa B" do portão (passo 2.4d): UB-1 a UB-16 escolhidos, UB-17 a UB-28 sorteados; validados em 25/11 | [tools/precisao/fixtures/amostra-usos.json](../tools/precisao/fixtures/amostra-usos.json) | `HC-*`, `RT-*` (chamadas) |
| `PH-1`–`PH-40` | Pares sorteados do possível por homônimo (passo 2.4d; só registro; validados em 25/11) | [amostra-usos.json](../tools/precisao/fixtures/amostra-usos.json) (`pares`) | — |
| `1B-A`–`1B-E` | Partes do diagnóstico do Piloto 1b | 1B-02, 1B-03 | — |
| `INV-D1`–`INV-D6` | Seções do inventário dos MDs | 1B-03 | demais D |
| `N0`–`N8` | Níveis de tipos (Python e Rust) | 1B-02, Parte B | — |
| `DP-py-*` | Níveis do despacho polimórfico (passo 2.4c) | [tools/precisao/fixtures/niveis.json](../tools/precisao/fixtures/niveis.json) | — |
| `CN-py-*` | Níveis da chamada pelo nome da classe (passo 2.4d-0) | [tools/precisao/fixtures/niveis.json](../tools/precisao/fixtures/niveis.json) | — |
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

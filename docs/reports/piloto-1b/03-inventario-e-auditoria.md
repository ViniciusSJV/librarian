# Piloto 1b — Inventário dos planos (Parte D) e auditoria do acervo (Parte E)

> Relatório entregue ao usuário em 18/11/2026 e aprovado por ele. Texto como entregue. Só mudaram os links, que agora são relativos a esta pasta.
> Os caminhos de scratchpad citados abaixo foram copiados para [dados/](dados/): `scratchpad\acervo\` → `dados/acervo/`.
> Os números de linha (arquivo:linha) são do estado dos MDs no commit `aae51d7`.
> Desde a consolidação de 18/11/2026, o plano da raiz guarda só o roadmap. Toda referência `PLANO:N` deste relatório vale, com o mesmo número, em [docs/historico/PLANO-ate-18-11.md](../../historico/PLANO-ate-18-11.md) (`PLANO-H:N`). As referências a CTX, EST e PIL seguem as linhas de `aae51d7`; os estados corrigidos estão no [ESTADO-ATUAL.md](../../../ESTADO-ATUAL.md).

Escrito para: quem vai continuar o projeto sem abrir os MDs (inventário) e para o usuário avaliar o acervo (Parte E).

## Resumo

- **Parte E:** nenhum dos 26 fatos tem a verdade apoiada no grafo, porque o modelo recebeu só trechos de código reunidos à mão. Mas 3 fatos afirmam chamadas que o grafo liga errado (F1, F14 e F17), e 2 afirmam chamadas que o grafo só liga de forma ambígua (F6 e F7). Na Q6 preparada, só o F6 está nessa situação.
- **Parte D:** o maior problema dos MDs é a repetição de identificadores com sentidos diferentes (P1–P4, G1–G8, E1–E11, D1–D16 e outros). O segundo é que os registros de estado ficaram para trás: F0–F10, a Etapa 4, P1 e G3 já estão commitados, e os documentos não dizem.

Nada foi alterado: nenhum MD foi editado, nenhum fato mudou, o servidor de teste foi encerrado e o `.studio` original continua idêntico (932 arquivos com os mesmos hashes).

---

# Parte E: os fatos do acervo contra os links do grafo

## Como o grafo entrou nos fatos

- **O modelo nunca recebeu links do grafo.** Os 14 pacotes dizem: "declarations the person gathered one by one; nothing was searched". O dossiê traz só os trechos das declarações reunidas. A verdade de cada fato foi conferida contra o código: H1–H3 e a revisão feita pelo agente.
- **O grafo entra de dois jeitos:**
  - **Navegação:** as declarações foram achadas por `/api/calls` e `used-by`. O relatório do piloto marca cada elo como [grafo], [nome] ou [código]. Conferi esses elos: todos estão corretos, menos `ping_by_slug → ping`. O relatório o marca como [grafo], mas o grafo dá homônimo de 2 (`views.ping` e `Check.ping`).
  - **Chamadas afirmadas no enunciado:** são a base da classificação.

**Regra de classificação:** olhei as chamadas que o enunciado afirma, não todas as chamadas que aparecem nas linhas citadas.

| Classificação | Quando se aplica |
|---|---|
| Não depende do grafo | O fato não afirma chamada, ou a chamada não tem link (sem candidato, acima do teto, ou referência de nome em vez de chamada) |
| Depende de link correto | O link é único ou de receptor, e o alvo está certo pela leitura do código |
| Depende de link suspeito | O link é homônimo: o alvo certo está entre os candidatos, mas o grafo não escolhe |
| Depende de link errado | Nenhum alvo do grafo é o certo |

## A tabela

| Fato | Na Q6? | Chamada afirmada (linha) | O grafo responde | Método → função solta? | Classificação | Como a declaração foi alcançada |
|---|---|---|---|---|---|---|
| F1 | | `Check.objects.get(...)` (views:277) | homônimo de 8: `get` de classes de teste e `hc/lib/curl.py get`. Nenhum é o certo (ORM do Django, G2) | 1 dos 8 candidatos é função | **Errado** | ping_by_slug, pela rota [código] |
| F2 | ✔ | (erro 400) `HttpResponseBadRequest` | sem candidato | — | Não depende | ping |
| F3 | | `slug.lower()`, `HttpResponseBadRequest` | sem candidato | — | Não depende | ping_by_slug |
| F4 | ✔ | nenhuma | — | — | Não depende | ping |
| F5 | ✔ | nenhuma | — | — | Não depende | ping |
| F6 | ✔ | `check.ping(...)` (views:255) | homônimo de 2: `views.ping` e `Check.ping` (G7) | não | **Suspeito** (o certo está entre os 2) | ping |
| F7 | | `ping(request, ...)` (views:300) | homônimo de 2: `views.ping` e `Check.ping` | não | **Suspeito** (o certo está entre os 2) | ping_by_slug |
| F8 | | `self.get_grace_start()` | único → `Check.get_grace_start` ✔ | não | Correto | `Check.ping → going_down_after`, único ✔ |
| F9 | | `Project.objects.for_api_key(...)`, `error(...)` | único → `ProjectManager.for_api_key` ✔; único → `error` (decorators:21) ✔ | não (`error()` é chamada de função) | Correto | `@authorize`, por referência de nome ✔ |
| F10 | | `_get_api_version(request)` | único ✔ | não | Correto | idem |
| F11 | | nenhuma (atribuições) | — | — | Não depende | `create_check → _update`, único ✔ |
| F12 | | nenhuma (`setattr`) | — | — | Não depende | idem |
| F13 | | `check.going_down_after()` | único → `Check.going_down_after` ✔ | não | Correto | idem |
| F14 | | `check.save()` e `check.save(update_fields=...)` (views:418, 422) | único → **`save` (github/views.py:92)** ✗ | **sim** | **Errado** | idem |
| F15 | | ORM `filter`, `exclude`, `order_by`, `first` | sem candidato | — | Não depende | `handle → handle_going_down`, único ✔ |
| F16 | | `check.get_status()`, `check.going_down_after()` | únicos ✔ (o `q.update` sai sem candidato) | não | Correto | idem |
| F17 | | `Flip(...)` e `flip.save()` (sendalerts:170, 175) | `Flip` único → classe ✔; `flip.save` único → **github `save`** ✗ | **sim** (no `save`) | **Errado** (em parte: a criação está certa, a gravação não) | idem |
| F18 | ✔ | ORM `filter().first()` | sem candidato | — | Não depende | `handle → process_one_flip`, único ✔ |
| F19 | ✔ | `q.update(...)` | sem candidato | — | Não depende | idem |
| F20 | ✔ | `self.executor.submit(notify, flip)` | sem candidato; `notify` é uma referência de nome → `sendalerts.notify` ✔ (G5) | — | Não depende | idem |
| F21 | ✔ | `flip.select_channels()` | único → `Flip.select_channels` ✔ | não | Correto | `notify`, por `used-by` [código] |
| F22 | ✔ | `ch.notify(flip)` | acima do teto: 33 candidatos | — | Não depende | idem |
| F23 | ✔ | `self.transport.notify(...)` | acima do teto: 33 candidatos | — | Não depende | `Channel.notify`, pela leitura do código [código] |
| F24 | ✔ | nenhuma | — | — | Não depende | idem |
| F25 | ✔ | ORM `filter().update()` | sem candidato | — | Não depende | idem |
| F26 | ✔ | `path("fail", views.ping, ...)` | sem candidato; `views.ping` não tem aresta (G4) | — | Não depende | `include(uuid_urls)`, por referência de nome ✔ |

## Totais

| Classificação | Acervo (26) | Q6 preparada (13: F2, F4, F5, F6, F18–F26) |
|---|---|---|
| Não depende do grafo | 15 | 11 |
| Depende de link correto | 6 (F8, F9, F10, F13, F16, F21) | 1 (F21) |
| Depende de link suspeito | 2 (F6, F7) | 1 (F6) |
| Depende de link errado | 3 (F1, F14, F17) | 0 |

**O que isso quer dizer:**

- Os três fatos com link errado estão **corretos como afirmação sobre o código**: a revisão leu o `save` do Django e o ORM.
- O risco está em quem consumir o fato e seguir o grafo a partir dele. O `save` de F14 e de F17 leva a uma view do GitHub.
- Os dois casos de método ligado a função solta (F14 e F17, os dois pelo `save`) são do padrão G1 que a Parte C encontrou.

Os scripts estavam em `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\c2d74a42-2372-4db9-9c84-8fc3b436eb57\scratchpad\acervo\` e agora estão em [dados/acervo/](dados/acervo/): `facts-calls.js`, `facts-calls.json` e `nav.js`.

---

# Parte D: inventário dos planos

**Abreviações usadas nas referências:**

| Abreviação | Arquivo |
|---|---|
| CTX | [CONTEXTO-CONTINUIDADE.md](../../../CONTEXTO-CONTINUIDADE.md) |
| PLANO | [PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md](../../../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md) |
| PIL | [docs/reports/PILOTO-1-HEALTHCHECKS.md](../PILOTO-1-HEALTHCHECKS.md) |
| ARQ | [docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md](../../ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md) |
| ESP | [docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md](../../ESPECIFICACAO-FUNCIONAL-STUDIO.md) |
| EST | [docs/ESTADO-IMPLEMENTACAO-STUDIO.md](../../ESTADO-IMPLEMENTACAO-STUDIO.md) |

**Escopo da leitura:**

- **Lidos por inteiro ou nas seções de estado e decisão:** CTX, a última seção do PLANO, PIL, as seções 1, 2 e 14 a 19 do EST e as seções 1, 10 e 11 da ESP.
- **Lidos só nos títulos e nas linhas de estado:** o resto do PLANO, a ARQ, o STUDIO.md e os MDs menores.

Onde a leitura foi parcial, o item está marcado como **ambíguo** ou **não conferido**.

## D1. Arquivos

| Arquivo | Para que serve |
|---|---|
| CTX | Transferência entre sessões: estado, experimentos E1–E11, decisões, pendências e seções datadas até a 20 (retomada pelo Piloto 1b) |
| PLANO | Plano histórico e atual: Graph Engine G1–G7, fase L, roteiro de 30/10, fase R, Etapas 0–7 do Studio, consolidação e roadmap das Fases 1–8 |
| PIL | Relatório do Piloto 1: números, workflows, Q1–Q6, acervo F1–F26 e limitações G1–G8 e P1–P4 |
| ARQ | Direção multiagente (não implementada): papéis, contrato público e 14 princípios |
| ESP | Especificação aprovada do novo frontend: plano F0–F13 e B1–B7, D1–D11, questões I e Q, Aberta 1–3 |
| EST | O que já foi implementado do frontend, F0–F10: decisões, pendências 1–76 e testes |
| [STUDIO.md](../../../STUDIO.md) | Manual do Studio e contratos: `origin.selection` formato 2, pacote da Etapa 4, proteção da escada |
| [HISTORICO.md](../../../HISTORICO.md) | Narrativa de como o Librarian foi construído |
| [V0.1.md](../../../V0.1.md) | Critérios de aceitação da versão 0.1 (fontes e afirmações, sem release) |
| [WORKFLOW.md](../../../WORKFLOW.md) | Trilha pela CLI e a fronteira atual |
| [TESTME.md](../../../TESTME.md) | Entrada para quem vai testar |
| [README.md](../../../README.md) e [README.pt-BR.md](../../../README.pt-BR.md) | Apresentação pública |
| [crates/librarian-ingest/README.md](../../../crates/librarian-ingest/README.md), [SEARCH.md](../../../crates/librarian-ingest/SEARCH.md) | Edition, perfis, grafo v2 e busca |
| [crates/librarian-graph-engine/WORKFLOW-V0.1.md](../../../crates/librarian-graph-engine/WORKFLOW-V0.1.md) | Fluxo formal de dossiês 0.1 |
| [crates/librarian-evidence/README.md](../../../crates/librarian-evidence/README.md) | Contrato do Evidence Builder (H1–H4, identidade) |
| [crates/librarian-llm-engine/CLAIMS.md](../../../crates/librarian-llm-engine/CLAIMS.md), [README](../../../crates/librarian-llm-engine/README.md) | Contrato de claims (FACT, INFERENCE, LACUNA, HYPOTHESIS) |
| [crates/librarian-studio/README.md](../../../crates/librarian-studio/README.md) | Servidor, rotas e a simplificação de 17/11 |
| [crates/librarian-studio/tests/tela/README.md](../../../crates/librarian-studio/tests/tela/README.md) | Baterias de tela e o que cada uma protege |
| [modelfiles/README.md](../../../modelfiles/README.md) | Biblioteca de modelos (tarefa × perfil × idioma) |
| `experiments/claims-camera-*`, `examples/maintenance/README.md` | Experimentos e exemplo antigos, com critérios registrados antes |
| `skills/ponytail/*` | A skill Ponytail, mantida sem nenhuma tarefa que a aceite |

## D2. Itens com identificador

> ⚠️ **Identificadores repetidos com sentidos diferentes:** P1–P4, G1–G8, E1–E11, D1–D16, R1–R6, Q1–Q12, "Etapa" e "Passo". Cite sempre o documento junto.

**Fases do roadmap atual** (PLANO:2229–2240)

| Id | Descrição | Status | Onde |
|---|---|---|---|
| Fase 1 | Piloto 1b: P1, G3, repetir Q6 | **Em andamento:** P1 e G3 commitados, Q6 pendente. O MD ainda diz "não iniciada" | PLANO:2233, CTX:968 |
| Fase 2 | Precisão das relações (G1, G5) | Pendente | PLANO:2234 |
| Fase 3 | Contrato de fatos, proveniência, formato 3 | Pendente; precisa de especificação | PLANO:2235 |
| Fase 4 | Evidence Request | Pendente | PLANO:2236 |
| Fase 5 | Generalização dos extratores | Pendente | PLANO:2237 |
| Fase 6 | Interface pública HTTP/JSON | Pendente | PLANO:2238 |
| Fase 7 | Primeiro fluxo multiagente | Pendente | PLANO:2239 |
| Fase 8 | Especialização e escala (inclui o extrator Java) | Pendente | PLANO:2240 |

**Piloto 1** (PIL)

| Id | Descrição | Status | Onde |
|---|---|---|---|
| G1 (piloto) | `unique` por nome não quer dizer resolvido (método × função livre) | Pendente, Fase 2 | PIL:265, PLANO:2277 |
| G2 | ORM e framework fora do projeto | Pendente | PIL:266 |
| G3 (piloto) | Omissão silenciosa em `/api/calls` | **Feito** (`aae51d7`); não registrado nos MDs | PIL:267 |
| G4 | Referências em rotas | Pendente | PIL:268 |
| G5 (piloto) | Funções passadas como valor | Pendente, Fase 2 | PIL:269 |
| G6 (piloto) | Despacho dinâmico | Pendente | PIL:270 |
| G7 (piloto) | Homônimo que inclui o próprio chamador | Pendente | PIL:271 |
| G8 | Testes HTTP sem ligação com as views | Pendente | PIL:272 |
| P1 (piloto) | Fatos de funções decoradas não voltam | **Feito** (`987d7a8`); não registrado | PIL:283 |
| P2 (piloto) | A sugestão de envelope herda G1 | Pendente | PIL:286 |
| P3 (piloto) | Custo fixo de cerca de 10 KB por pacote | Pendente | PIL:287 |
| P4 (piloto) | A instrução ao modelo diz "aprovados por uma pessoa" | Pendente | PIL:288 |
| Q1–Q6 (piloto) | As 14 perguntas do piloto (turns 01–14) | Feitas; **Q6 a repetir** | PIL:171–186 |
| F1–F26 | Fatos promovidos, revisados pelo agente e não pelo usuário | Vigentes | PIL:213–246 |

**Graph Engine, roteiro de 19/10** (não confundir com o G do piloto)

| Id | Descrição | Status | Onde |
|---|---|---|---|
| G1 (19/10) | Política de linguagem sai para o extrator | Feito | PLANO:397 |
| G2 (19/10) | Contrato neutro do grafo v2 | Feito (opção A) | PLANO:405 |
| G3 (19/10) | Dossiês sem `rust_source` | Feito | PLANO:424 |
| G4 (19/10) | Ponte busca → dossiê | Feito | PLANO:443 |
| G5 (19/10) | Python como prova da fronteira | Feito | PLANO:558 |
| G6 | Impacto: a/b/c e d-1 | a–c e d-1 feitos; G6d, o relatório de impacto, pendente | PLANO:764, 794, 824, 888 |
| G7 (21/10) | Extrator de IaC (HCL) | Planejado, não iniciado | PLANO:848 |

**Fase L e roteiro de 22/10**

| Id | Descrição | Status | Onde |
|---|---|---|---|
| L1–L4 | Linhas numeradas, claims, vínculo do bundle, portões até o acervo | Feitos | PLANO:498–716 |
| L2b, Cadeia, Patch | Comparação por conteúdo, cadeia, patch conferido | Feitos | PLANO:631, 716, 728 |
| (item 4 de 22/10) | Modelfile `impacto` | **Pendente** | PLANO:761 |

**Roteiro de 30/10 e fase R**

| Id | Descrição | Status | Onde |
|---|---|---|---|
| Etapas 1–5 (30/10) | Grafo vê mais usos, inventário, laços, Régua por origem, perguntas | 1 a 5 com resultado registrado | PLANO:891–930 |
| Etapa 6 (30/10) | Mudança aprovada vira fato (CHANGE) | Feito (`5b0ebf8`) | PLANO:1053 |
| Etapa 7 (30/10) | Menos ligações só pelo nome (tipo do receptor) | **Ambíguo:** feito para Rust no inventário (PLANO:988); Python não; "o que chama?" não usa | PLANO:884 |
| Etapas 8–10 (30/10) | Impacto, léxico em português, linguagens novas | **Ambíguo / não conferido** | PLANO:888–890 |
| R1–R3 | Mapa, medida e teste por unidade no renderer | Feitos | PLANO:983–1019 |
| R4 | Bancada por unidade | **Ambíguo:** não há seção de resultado | PLANO:962 |
| R5 | Otimização com portão | Rodadas registradas ("segunda rodada do R5") | PLANO:1039 |
| R6 | Exploração (Voronoi) | **Ambíguo:** sem resultado | PLANO:964 |

**Experimentos e consolidação** (E1–E11 e E1–E5 são famílias diferentes)

| Id | Descrição | Status | Onde |
|---|---|---|---|
| E1–E11 (experimentos) | Experimentos com LLM e sem LLM | Concluídos, com decisões; vários suspensos | CTX:166–290 |
| E1–E5 (consolidação) | Evidence Builder virou crate independente | Feito (`d949f91`) | PLANO:2061 |

**Etapas 0–7 da frente do Studio** (05/11; não confundir com as etapas de 30/10 nem com a ETAPA 2 de 19/10)

| Id | Descrição | Status | Onde |
|---|---|---|---|
| Etapas 0–2 | Preservação, diagnóstico, correções | Feitas | PLANO:1608–1610 |
| Etapa 3, Passos 1–4, 6 e 7 | "Onde pesa" | Feitos | PLANO:1611, CTX:555–586 |
| Etapa 3, Passo 5 | Cidade e fluxograma | Adiado para a evolução visual | CTX:585 |
| Etapa 4, Passos 0–15 | Evidence Builder | Feitos; o MD diz "em andamento, sem commit", mas há o commit `09e7e53` "conclui etapa 4" (**divergência**) | PLANO:1957–1972, 1612 |
| P1–P4 (Etapa 4) | P1: formato 2; P2: documentação; P3: errata do Passo 14; P4: Ubuntu | O MD diz "aguardam avaliação" e "P4 pendente". **Ambíguo:** houve commits e validação no Ubuntu em 18/11 | PLANO:1973–1979, CTX:803–812 |
| Etapa 5 | Pacote e revisão: C1 e C2 | O MD diz "não iniciada". **Parcial na prática:** C1 (`/api/ladder` com `turn`) e "todo envio sai do Pacote" foram feitos em 18/11 | PLANO:1613, 1995 |
| Etapas 6 e 7 | Fact Builder e portão; verificação | Não iniciadas | PLANO:1614–1615 |
| Evolução visual (sem número) | Cidade e fluxograma | Planejada, não autorizada | PLANO:2017 |

**Códigos menores**

| Id | Descrição | Status | Onde |
|---|---|---|---|
| C1, C2 | `/api/ladder` com `turn`; `GET /api/source` | C1 feito na consolidação; C2 pendente | PLANO:1997–1998 |
| C3, C4 | Mudança de texto ao modelo; condição dos consumidores da medição | Registrados | PLANO:1715, 1730 |
| D1–D14 (PLANO) | Defeitos corrigidos na Etapa 2 | Feitos | PLANO:1681–1693 |
| D15 | Item herdado da Etapa 2 | Não feito | CTX:822 |
| D6–D8 (Passos 6 e 7) | `manual_counts`, `no_source`, teste fixado | Feitos | CTX:720–722 |
| D1–D11 (ESP) | Dependências de backend | Pendentes | ESP:902 |
| D1–D16 (simplificação) | Decisões da simplificação de 17/11 | **Ambíguo:** estão "no relatório da sessão", fora do repositório | PLANO:2072 |
| R1–R5 (auditoria) | Ressalvas da auditoria dos Passos 6 e 7 | Tratadas | CTX:743–749 |
| H1–H4 | Conferências do pacote: H1–H3 bloqueiam, H4 informa | Implementadas | CTX:790 |
| Formato 1 e 2 | Formatos do pacote | Implementados | CTX:796–801 |
| Formato 3 | Fatos com autoria e proveniência | Pendente, Fase 3 | PLANO:2235, 2302 |

**Frontend novo (ESP e EST)**

| Id | Descrição | Status | Onde |
|---|---|---|---|
| F0–F10 | Reformulação do frontend | Feitos e commitados (`f7d2131`); o EST diz "não commitado" (**divergência**) | EST:86–98 |
| F11 | Ferramentas | Pendente; o EST o chama de "próxima etapa autorizada" | EST:99, 1753 |
| F12, F13 | Comparação temporária; aceitação completa | Pendentes | ESP:1069–1070 |
| B1–B7 | Passos de backend | Pendentes, fora da v1 | ESP:1076–1084 |
| I-1–I-8; Q1–Q12 (ESP) | Questões da especificação | Fechadas (Q9 parcialmente incompatível) | ESP:1090–1113 |
| Aberta 1, 2 e 3 | Quando fazer D10 e B3; remover `librarian.mode` | Abertas | ESP:1114–1116 |
| T1–T26 | Transições da especificação | Referência | ESP, seção 6 (≈773) |
| M1, M2 | Defeitos da auditoria do F9 | Resolvidos | EST:1267, 1274 |
| Pendências 1–76 | Pendências do frontend | Resolvidas e abertas misturadas (abertas em D5) | EST:1561–1640 |

## D3. Decisões já tomadas

Os trechos estão citados como estão escritos.

| Decisão | Trecho | Onde |
|---|---|---|
| Grafo v2 | "Decisão do usuário: opção A, ou seja, formato novo exportável a partir de qualquer edition verificada" | PLANO:407 |
| Linhas numeradas | "Decisão do usuário: opção A, ou seja, numeração como opt-in da trilha `prepare`" | PLANO:504 |
| Claims | "**A:** formato `claims` opt-in por projeto, com a prosa mantida" | PLANO:588 |
| Acervo de fatos | "os fatos formam um **acervo consultável** … **só FACT** é promovido … a revisão é **por item**" | PLANO:698 |
| Cadeia | "passam as locations conferidas e os fatos vigentes do acervo; o texto dos claims anteriores não passa" | PLANO:759 |
| G6a–c | "o Graph Engine fecha antes de seguir no LLM … teto de 8 homônimos" | PLANO:796 |
| G6d-1 | "Decisões do usuário: a expansão é automática (dono, campos lidos …" | PLANO:826 |
| Condições | "A linha de condições fica encerrada por enquanto, e o extractor não muda." | PLANO:1345 |
| Tempo | "a contagem continua sendo a medida confiável, e o tempo instrumentado é só referência" | PLANO:1407 |
| Lacunas | "A investigação de lacunas fica suspensa, sem descartar os resultados." | PLANO:1482 |
| Homônimos | "investigação suspensa, sem implementar a resolução simulada e sem teste com o modelo" | PLANO:1555 |
| Qwen nas mudanças | "fora da escada de mudanças até o fine-tune; continua nos fatos" | CTX:290 |
| Envelope | "não aumentar o limite nem alterar a composição sem evidência" | CTX:316 |
| Trabalho | "commits em português, sem acentos … documentação validada antes do commit … push só quando pedido" | CTX:328 |
| Pareto | "sem dominância nem seleção Pareto por enquanto" | CTX:428 |
| Passo 3 | "Decisão formal do usuário: opção A. O Passo 3 está encerrado como entregue" | CTX:551 |
| Capas | "ficam como elemento da identidade visual, sem atribuição literária" | CTX:704 |
| Escada | "Opção A, aplicada em 18/11 (`fn ladder`): um pedido precisa de `turn` …" | CTX:859 |
| Léxico | "Está mantida como operação explícita e separada da investigação de código (decisão de 18/11)" | CTX:861 |
| Decisões de 18/11 | `verificavel` em inglês com duas traduções; `/api/path-questions` mantida; "Abrir arquivos" mantido; Ponytail; baterias antigas não apagadas | CTX:865–869 |
| Multiagente | "Não adicionar framework de agentes aos crates centrais." | CTX:935 |
| Autoria | "**Revisão por agente não é aprovação humana.**" | PLANO:2298 |
| Promoção por item | "Fica registrada a decisão futura sobre **promoção por item**" | PLANO:2300 |
| Formato dos fatos | "O formato atual dos fatos **não muda nesta iniciativa**." | PLANO:2302 |
| G1 | "Uma correção que melhora o Python e piora o Rust sem aviso não é aceitável." | PLANO:2284 |
| Frontend | Decisões aprovadas 1 a 8 e a tabela de questões fechadas | ESP:50–59, ESP:1090–1113 |
| Frontend, por etapa | Tabela "Registro de decisões funcionais aprovadas" | EST:1517–1557 |
| **Decisão A do G3** | "Decided in the Piloto 1b: it is `no_candidate_in_project` too" | **Não está em nenhum MD**; só em [tests/g3.rs:185-188](../../../crates/librarian-ingest/tests/g3.rs#L185-L188) |
| Princípios permanentes | 14 princípios, de "não escolhe os agentes" até "o extrator deve evoluir sem tornar o núcleo dependente de uma linguagem" | ARQ:154–169 |

## D4. Roadmap como está registrado hoje

**Trilha principal** (PLANO:2229–2240, CTX:968–1004)

1. **Fase 1, Piloto 1b:** P1, G3 e a repetição da Q6 com a mesma pergunta, as 7 declarações, `with_facts`, comparando com 46,2 KB, 11.667 tokens, 8 de 12 fatos e 1 fato novo.
2. **Fase 2:** G1 (método × função livre) com medição no Rust, casos de regressão nas duas linguagens e G5.
3. **Fase 3:** autoria, revisão e aprovação separadas, promoção por item, confirmação por teste e formato 3.
4. **Fase 4:** Evidence Request.
5. **Fase 5:** representação intermediária comum dos extratores e preparação para Java.
6. **Fase 6:** contratos HTTP/JSON públicos.
7. **Fase 7:** primeiro fluxo multiagente.
8. **Fase 8:** agentes especializados, extrator Java, escala.

**Trilhas paralelas registradas sem ordem em relação às fases:**

- **Frontend:** F11 → F12 → F13, e B1–B7 com autorização própria (EST:1753).
- **Etapas 5–7 da frente do Studio** (PLANO:1613–1615).
- **Evolução visual** (PLANO:2017).
- **G7, IaC** (PLANO:848).

**Ordem entre essas trilhas e as fases: ambígua.** O CTX:947 diz que a próxima prioridade é o Piloto 1b. O EST:1756 diz que o F11 é "a próxima etapa autorizada".

## D5. Perguntas em aberto e pendências sem id

**Decisões ainda em aberto**

- Commit do `measures` no `logistica-api`; cópia do arquivo de preservação fora da máquina; limpeza de `Temp/lt-ev`, `%TEMP%\lw` e dos scratchpads (CTX:376–378, 894).
- Vídeo 13 em espera (CTX:379); exclusão das capas antigas (CTX:706).
- "N de M", fallback único, filtro de 5% (CTX:770).
- "Sem medição" quando a leitura falha (CTX:771); custo de `/api/commands` e a nota com mais de um comando (CTX:772–773).
- Adiados de propósito: opção B da escada, unificar o `verificavel` em inglês, recriar as baterias antigas (CTX:893). Falta conferir o CI do commit de 18/11 (CTX:891).

**Fatos não confirmados**

- Lista em CTX:481–489: custo total, origem de `LOGISTICS.md`, `/api/executions`, causa do dono de método de trait, rodadas de lacuna no fluxo real, tema manual, restauração do arquivo.
- O dono de um método de trait não é candidato; a causa não está confirmada (CTX:367).

**Limitações sem id**

- **Do piloto:** relação mediada pelo banco (PIL:161); transportes não aprofundados (PIL:165); imports, dependências externas, `is_test` e templates (PIL:276–279); o portão vale para a resposta inteira e a confirmação por teste só fica na nota (PIL:294–297).
- **Herdados da Etapa 2 e não feitos:** remover `investigationView` e os motivos no inventário (CTX:822).

**Pendências abertas do frontend** (as resolvidas foram omitidas)

- Ainda com decisão humana: 1, 2, 7, 8, 14, 15, 69, 70 e 74.
- De classificação: 43, 44, 45 e 47.
- De etapa: 9, 20, 24, 38, 39, 40, 50, 71, 72 e 75.

**Achados desta sessão, sem nenhum registro em MD**

- A ordem de `unresolved` não segue arquivo/linha/coluna.
- O corte do `node_limit` segue a ordem dos ids das arestas.
- Em Python, `self.repo = Repo()` deixa a chamada `Repo()` presa ao campo, e não ao `__init__`.
- "O que chama?" não usa o `receiver_type` do Rust, que o inventário usa.
- Dos 442 links únicos de método no Healthchecks, 197 vão para uma função solta.
- A amostra de precisão (Parte C) e esta auditoria (Parte E).

## D6. Divergências

| # | O que os MDs dizem | O que existe de fato |
|---|---|---|
| 1 | O EST (2.1 e 19) diz "nada de F0 a F10 foi commitado" | Há o commit `f7d2131` "consolida implementação F0-F10" |
| 2 | O PLANO (1612, 1933) e o CTX (§17) dizem que a Etapa 4 está "em andamento, sem commit" | Há os commits `09e7e53` "conclui etapa 4" e `9bc92b0`; nenhum MD registra o encerramento |
| 3 | O PLANO:1613 diz que a Etapa 5 está "não iniciada" | C1 (`/api/ladder` com `turn`) e "todo envio pelo Pacote" foram feitos em 18/11 (PLANO:2087–2098) |
| 4 | O PLANO:2233 e o CTX:947 dizem que a Fase 1 não foi iniciada | P1 (`987d7a8`) e G3 (`aae51d7`) estão commitados |
| 5 | A pendência 65 do EST diz que o `target_of` não foi corrigido | Corrigido pelo P1 |
| 6 | O PIL:104 marca `ping_by_slug → ping` como [grafo] (aresta única correta) | O grafo dá homônimo de 2; isso também vale para o F7 |
| 7 | A decisão A do G3 não aparece em nenhum MD | Está registrada só no comentário do teste `g3.rs` |
| 8 | A tabela do CTX:30 diz que o Evidence Builder "não é um crate" | É o crate `librarian-evidence`; o próprio CTX:7 avisa que a tabela está desatualizada |
| 9 | O CTX:898 diz que o `.studio` só tem o projeto `renderer` | Também tem `healthchecks-25597e41`; é um registro anterior ao piloto |
| 10 | A ESP usa os nomes "Evidence Builder" e "Revisão" para as vistas | Hoje as vistas se chamam Seleção e Pacote; a própria ESP:7–14 avisa |
| 11 | O STUDIO.md descreve a página anterior à reformulação (pendência 20 do EST) | Continua assim |
| 12 | A P4 da Etapa 4 aparece como "Ubuntu pendente" | O CTX:881 registra validação no Ubuntu em 18/11, mas da consolidação. Não está claro se fecha a P4 (**ambíguo**) |
| 13 | O item 7 do roteiro de 30/10 (tipo do receptor) não tem estado | Feito no Rust, mas só no inventário. Não existe em Python nem em "o que chama?" (Parte B) |
| 14 | O CTX registra 446 testes em 18/11 | Hoje são 456, por causa dos testes do P1 e do G3 |

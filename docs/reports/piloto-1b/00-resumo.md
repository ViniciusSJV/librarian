# Piloto 1b — Relatório final

> Passo 1.3 do roadmap, 18/11/2026. Resume os relatórios [01](01-g3.md), [02](02-fechamento-g3-e-diagnostico.md), [03](03-inventario-e-auditoria.md) e [04](04-q6.md); os detalhes e os números de cada parte estão neles. Ids com prefixo, como em [docs/IDS.md](../../IDS.md).

## Objetivo

Corrigir as limitações comprovadas do [Piloto 1](../PILOTO-1-HEALTHCHECKS.md) que impediam a reutilização de fatos (`PIL-P1`) e escondiam chamadas (`PIL-G3`), sem funcionalidade nova, e repetir a pergunta de reutilização (`PIL-Q6`) para medir o efeito. No caminho, o 1b também mediu a precisão das relações e conferiu o acervo e os documentos.

## O que foi feito

| Parte | Entrega | Relatório |
|---|---|---|
| P1 | `target_of` usa a linha da declaração, não o decorador (`987d7a8`) | [01](01-g3.md), [02](02-fechamento-g3-e-diagnostico.md) Parte A |
| G3 | `/api/calls` devolve `unresolved` com três motivos, e a tela mostra "Chamadas não apresentadas" (`aae51d7`) | [01](01-g3.md) |
| Diagnóstico | Tipos N0–N8 em Python e Rust (Parte B) e linha de base de precisão (Parte C) | [02](02-fechamento-g3-e-diagnostico.md) |
| Auditoria | Acervo F1–F26 contra os links do grafo (Parte E) e inventário dos MDs (Parte D) | [03](03-inventario-e-auditoria.md) |
| Q6 | Repetida com a mesma pergunta e as mesmas 7 declarações, na run `20261118-205150` | [04](04-q6.md) |
| Consolidação | ESTADO-ATUAL, DECISOES, roadmap novo e histórico do plano (passo 1.2, `a8e4074`) | — |

## Resultados

- **P1:** `ping` volta com F2, F4, F5 e F6; `ping_by_slug`, com F1, F3 e F7. A Q6 passou de 8 para 13 fatos entregues: 4 pelo P1 (F2, F4, F5, F6) e o F26, promovido da própria Q6 original.
- **G3:** `ch.notify(flip)` aparece como `above_candidate_limit`, com 33 candidatos. Os campos antigos de `/api/calls` ficaram iguais byte a byte em 15 consultas do Healthchecks e 271 do ray tracer.
- **Suíte:** 456 aprovados, 0 falhas, 2 ignorados (antes da correção do G3: 452 aprovados e 4 falhas, as dos testes novos). Os 5 testes do G3 passam (antes, 1 de 5); os 5 do P1, também.
- **Parte B:** Python não liga variável a tipo em nenhum nível com homônimo. Rust grava `receiver_type`, mas só o inventário o usa; "o que chama?" não.
- **Parte C:** no Healthchecks, 4 dos 8 links únicos da amostra estão errados, e 197 dos 442 links únicos de método vão para função solta. No ray tracer, nenhum erro (2 corretos, 2 ambíguos, 6 sem candidato).
- **Parte E:** dos 26 fatos, 15 não dependem do grafo, 6 dependem de link correto, 2 de link ambíguo (F6, F7) e 3 de link errado (F1, F14, F17). Pela leitura do agente, sem revisão humana, os 26 estão corretos como afirmação sobre o código.
- **Q6** (avaliada pelo usuário em 18/11):
  - Entrega: sim. 13 fatos chegaram ao modelo, com +3,9% de tokens (11.667 → 12.119) e +3,4% de pacote (46.220 → 47.777 B).
  - Reutilização: não. F20, F22 e F26 foram entregues e repetidos; só a repetição do F6 sumiu.
  - A resposta foi reprovada na C3 (`location_operation_includes_declaration`). A C2 é semanticamente incorreta com citação válida. Não houve nenhuma LACUNA sobre o elo pelo banco.

## O que ficou provado

- Fatos de funções decoradas voltam ao pacote (P1), pela API e com dados reais, não só pelo teste unitário.
- Ausência de link em "o que chama?" deixou de ser silenciosa: toda chamada sem link tem motivo (G3).
- Entregar um fato não impede o modelo de repeti-lo: o F26 foi repetido quase palavra por palavra.
- Em Python, link "único" hoje não quer dizer link correto.
- O portão pela resposta inteira perde itens corretos: as C1, C4 e C5 da Q6 estavam certas.

**Limites:** o `node_limit` só foi visto num projeto sintético. A Q6 teve um único envio (temperatura 0 e semente fixa), que não separa o efeito de cada fato. A verdade das amostras C e E foi definida pela leitura do agente.

## O que ficou em aberto, e para onde foi

| Pendência | Destino |
|---|---|
| Amostra da Parte C e scripts da Parte E executáveis a partir do repositório | Fase 2, passo 2.0 (portão da fase) |
| Duas regras de resolução; `type` separado de `unique`; reabre DEC-11 (Homônimos) | Fase 2, passo 2.1 |
| Método ligado a função solta (197 links; F6, F7, F14, F17) | Fase 2, passo 2.2 |
| Tipos em Python (N1–N4), herança e alias (N5, N8), referência passada (`PIL-G5`/N7), genéricos (N6) | Fase 2, passos 2.3–2.6 |
| Ordem de `unresolved` e corte do `node_limit` pela ordem das arestas | Fase 2 (achados registrados, sem correção) |
| Elo pelo banco (`PIL:161`), rotas (`PIL-G4`), ORM (`PIL-G2`), testes ligados às views (`PIL-G8`) | Fase 2B |
| `authorship: "revisão humana"` nos fatos F1–F26, sem revisão humana (`PIL-P4`), **prioridade alta** | Fase 3 |
| Promoção por item (Q6/C3) e detecção de repetição (F20, F22, F26) | Fase 3 |
| LACUNA declarada sobre o elo pelo banco; custo fixo por pacote (`PIL-P3`) | Fase 4 |
| `E4-P4` (Ubuntu), que ficou ambígua | Pendência do ESTADO-ATUAL, sem fase |

**Fase 1 concluída.** O próximo passo é o 2.0.

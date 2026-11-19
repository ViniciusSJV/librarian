# Librarian: plano de evolução

**Desde 18/11/2026 este arquivo guarda só o roadmap ativo.** O estado, as regras de trabalho e as pendências estão no [ESTADO-ATUAL.md](ESTADO-ATUAL.md); as decisões, no [DECISOES.md](DECISOES.md).

**Histórico:** o plano até 18/11/2026 (diagnóstico de 27/09, ETAPA 2, roteiros de 19/10, 22/10 e 30/10, Fase L, fase R, Evidence Builder de 04/11, Etapas 0–7 do Studio, consolidação de 17/11, Piloto 1 e o roadmap anterior) está em [docs/historico/PLANO-ate-18-11.md](docs/historico/PLANO-ate-18-11.md), com as mesmas linhas: `PLANO:N` citado antes de 18/11 é `PLANO-H:N`. Veja [docs/historico/README.md](docs/historico/README.md).

## Princípios preservados

- Evidência não é conclusão.
- Toda aresta deve permitir responder: “Qual evidência autorizou esta ligação?”.
- Hash comprova consistência, não autenticidade.
- Código descreve estrutura/procedimento; não prova execução.
- `#[test]` observado não comprova que um teste foi executado.
- Graph Engine testa/relaciona sem explicar; LLM Engine explica sem extrapolar as evidências.
- Mais contexto não implica mais certeza.
- Referência sintática, candidato por nome e resolução comprovada permanecem distintos.
- O viewer projeta relações existentes; proximidade visual não cria relações.
- Librarian não conhece os domínios de Renderer, xadrez ou outros consumidores.

## Roadmap

Critério de pronto antes dos agentes: um agente consegue remontar os 4
workflows do Healthchecks e caminhos do ray tracer com relações
corretas, lacunas explícitas e fatos aguardando aprovação humana.

### Fase 1 — Piloto 1b
- 1.1 Q6 — feita em 18/11 (run 20261118-205150). Entrega: sim (13
  fatos, +3,9% tokens). Reutilização: não (F20, F22 e F26 entregues e
  repetidos; só a repetição do F6 sumiu). Resposta reprovada na C3
  (location_operation_includes_declaration). C2 semanticamente
  incorreta com citação válida. Nenhuma LACUNA sobre o elo pelo banco.
- 1.2 Consolidação documental.
- 1.3 Relatório final do 1b.

### Fase 2 — Precisão das relações (PIL-G1, PIL-G5, PIL-G7, item 7 de 30/10)
- 2.0 Tornar a amostra da Parte C e os scripts da auditoria (Parte E)
  executáveis a partir do repositório, sem caminhos do scratchpad.
  Eles são o portão da Fase 2: `tools/precisao/` (README lá).
- 2.1 Regra única de resolução para "o que chama?" e inventário;
  resolução própria `type`, separada de `unique`. Reabre a decisão
  "Homônimos: investigação suspensa" (DEC-11, PLANO-H:1555).
  Inclui a regra `receiver`: ela toma `self.x.m()` por `self.m()` e liga
  a chamada ao próprio método que a contém. Erra nas 17 chamadas da
  população (4 no Healthchecks, 13 no ray tracer), e o inventário usa a
  mesma regra antes da regra por tipo (`inventory.rs:635-639`).
  Commitado em 19/11 (`c316bf7`, `3b4e1c6`, `6525fb4`, `cb8e5ef`, `078fd0e`), com push
  em 19/11/2026; CTX seção 23.
- 2.2 Chamada de método não casa com função solta (Rust direto; Python
  com tabela mínima de imports para modulo.f()). Meta: os 197 links;
  F6, F7, F14 e F17. Quando o tipo do receptor é conhecido e não tem o
  método, o tipo derruba um unique (ex.: RT-15/16, `model.lines()` com
  `model: &str`); no 2.1, unique vem antes de type.
  Feito em 19/11 (CTX seção 24, DEC-38; amostra validada): 197 → 30 links
  (todos modulo.f() certos); HC-F7 e HC-F17 corretos, HC-F14 sem link.
  Meta não atingida: HC-F6 (`check.ping`) fica `homonym` com um só
  candidato, o certo, sem link. Promover um homônimo que fica com um
  candidato criaria 711 links falsos em testes (`self.client.login`), e
  o tipo de `check` em Python só vem no 2.3. Fica para o 2.3.
- 2.3 Tipos em Python: N2 → N1 (com relação "instancia", separada de
  "chama") → N4 (corrige Repo() preso ao campo) → N3 entre arquivos
  (também Rust). Na mesma questão de dono: a chamada de um decorador
  roda na carga do módulo, não na função decorada (HC-19 da amostra).
- 2.4 Herança (N5) e alias local (N8).
- 2.5 PIL-G5/N7: relação "passa referência", nunca "chama".
- 2.6 Genéricos (N6), por último (ch.notify: 33 → 1).
- Portão de cada passo: `node tools/precisao/portao.js --studio .studio
  --base tools/precisao/linha-de-base.json`. Reprova se `link_falso`
  subir ou as corretas caírem (Python ou Rust; DEC-25, PLANO-H:2284),
  se uma edition antiga mudar ou se o `.studio` original mudar.

### Fase 2B — Relações de workflow
- Pontos de entrada: rotas (PIL-G4), management commands, jobs.
- Relação mediada pelo banco (PIL:161): "grava modelo X" / "lê modelo X".
- ORM e framework (PIL-G2) como externo conhecido, não "sem candidato".
- Testes ligados às views (PIL-G8).

### Fase 3 — Fatos, proveniência e aprovação humana
(absorve as Etapas 6–7 da frente do Studio: "fatos por fonte" da
Etapa 6 entra na fila de aprovação; a Etapa 7 vira critério de aceite
de toda entrega de interface, DEC-30; a comparação de propostas da
Etapa 6 vai para as trilhas paralelas, DEC-31)
- Formato 3; autoria correta. PIL-P4: o texto ao modelo e o campo
  authorship "revisão humana" em fatos revisados por agente.
  Prioridade alta.
- Promoção por item: o portão vale por claim, não pela resposta (Q6/C3).
- Fila de aprovação humana.
- Detecção mecânica de repetição: claim que repete fato entregue é
  marcada, não conta como nova (Q6: F20, F22, F26).
- Confirmação por teste.

### Fase 4 — Evidence Request
- Primeiro consumidor de referência: o agente de workflows.
- Fatos entregues referenciáveis por id; o consumidor declara o que
  reutilizou e produz só o novo.
- Lacunas conhecidas fazem parte do contrato (ex.: elo pelo banco).
- PIL-P3: custo fixo por pacote.
- Formato de workflow no acervo: um workflow como sequência de etapas,
  cada uma apoiada em fatos aprovados e com as lacunas declaradas. É o
  que o agente de workflows produz e o agente de documentação consome
  (DEC-36).

### Fase 6 — HTTP/JSON público

### Fase 7 — Primeiros agentes
Agente de workflows e agente de documentação (fluxogramas para
humanos). A validação final é humana.

### Fase 5 — Extratores: representação intermediária comum
Executada depois da Fase 7, mantendo o nome (DEC-35): os primeiros
agentes trabalham em Python e Rust, que já existem.

### Fase 8 — Especialização e escala (DDD, performance, segurança)

### Extrator Java
Último item do roadmap, depois da Fase 8 (DEC-35).

### Trilhas paralelas
Congeladas até o fim da Fase 2B: F11–F13, Etapa 5 do Studio, GE-G7
(IaC). A Evolução visual (cidade e fluxograma) volta junto com o
agente de documentação.

Comparação de propostas de mudança (HYPOTHESIS, medidas e pioras de
`execution.json`, "sem dominância nem Pareto", DEC-15, CTX:420):
congelada, com destino na Fase 8, como base do agente de Performance
(DEC-31).

A Etapa 6 do Studio depende da Etapa 5 (revisão do pacote e C2,
`GET /api/source`), que está congelada acima.

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
  Dividido na Parada 1 (19/11): 2.3a-0 (chave dos métodos no Python,
  `8c689d4`, com push), 2.3a (tipos no arquivo, "instancia", regra no
  receiver_type; DEC-39; validado e commitado em 20/11, `5589912`;
  amostra Python 21 → 27 corretas, HC-F6 correto), 2.3b
  (dono do campo e do decorador; opção B, pela regra, sem edition nova;
  DEC-40; validado e commitado em 20/11, `5a26630`) e 2.3c (N3
  entre arquivos, retorno de T::f; DEC-41; validado em 20/11).
  **Fechamento do 2.3 (20/11/2026; CTX seção 28):**
  - Entregou: a chave dos métodos do Python pelo último trecho (2.3a-0);
    o tipo do receptor em Python escrito no arquivo, a relação
    "instancia" e a regra no `receiver_type` (2.3a, perfil V9); o dono
    do decorador (`decorates`) e do campo do `__init__` (`via`), pela
    regra (2.3b); o tipo pelo retorno anotado, no mesmo arquivo e entre
    arquivos, o campo de classe de outro arquivo (Python) e o retorno de
    `T::f` no lugar do `T` do caminho (Rust), com o `type_rule` efetivo
    no `/api/calls` (2.3c). Só o 2.3a pediu perfil novo.
  - Níveis no alvo: N1, N2, N3 (Python e Rust), N4 e N4-py-dono, e os
    N3c do 2.3c. Ficam N5 e N8 (2.4), N6 (2.6), N7 (2.5) e N0-livre
    (2.4).
  - Amostra: Python com 53 corretas e 1 `link_falso` em 112 chamadas,
    Rust com 41 corretas e nenhum `link_falso` em 67 (linha de base
    do 2.3c). HC-F6 correto; HC-F1 continua errado (Fase 2B).
  - Para o 2.4: o `__init__` herdado; o alias (`as`, N8), que também
    tira o tipo de 38 retornos `TestHttpResponse` nos testes.
  - Para o 2.6: os genéricos e os elementos de coleção (`list[Check]`,
    `Vec<..>`, `ch.notify`); `Option`, `Box`, `Rc` e `Arc` de `Self`
    continuam sem decidir.
  - Para a Fase 2B: o gerenciador e o QuerySet do Django
    (`Check.objects.get(..)` não dá tipo), `Model.save`, o módulo como
    dono do decorador de topo.
  - Limites sem passo: `let x = T::f(..)?` (o `Result<Self, E>` perde o
    tipo, Q2 do 2.3c); campo de struct de outro arquivo no Rust
    (`summary.samples`).
  - Risco do caso (b) da DEC-41: um `homonym` de um só candidato dá o
    seu retorno como tipo, embora o 2.2 não o promova a link; único
    efeito nos dados, a HC-110 (certo). O caso (d) com `misto()` foi
    para o 2.4.
- 2.4 Herança (N5) e alias local (N8). Junto: a chamada simples
  resolvida pelo escopo, mesmo módulo e `use`/`import` (N0-livre, "a
  definir" desde o 2.2), e a perda do caso (d) da DEC-41 que ela causa:
  `misto()` declarado em dois módulos com retornos diferentes fica sem
  tipo, embora o Rust chame o do próprio módulo (N3c-rs-discordam;
  nenhum caso nos dados).
  Parada 1 aprovada em 20/11/2026 ([2.4-parada-1.md](docs/reports/fase-2/2.4-parada-1.md)):
  antes do 2.4a, um commit de ferramenta com a trava do portão pelo hash
  das fontes compiladas (Q1) e `scope` como resolução de alvo único (Q3);
  depois, 2.4a (escopo e alias; DEC-42) e 2.4b (herança, com
  `cls.m()` e o método chamado por uma `@property` de retorno anotado,
  Q7). 2.4a: Parada 2 em [2.4a-parada-2.md](docs/reports/fase-2/2.4a-parada-2.md),
  validado em 24/11/2026 (DEC-42 vigente, linha de base trocada).
  2.4b: Parada 1 aprovada em 24/11/2026 ([2.4b-parada-1.md](docs/reports/fase-2/2.4b-parada-1.md)),
  Parada 2 em [2.4b-parada-2.md](docs/reports/fase-2/2.4b-parada-2.md), validada em 25/11/2026 (DEC-43
  vigente, linha de base trocada).
  **Fechamento do 2.4a e do 2.4b:** o 2.4a entregou o
  escopo da chamada simples (Python e Rust), o alias (N8) e a correção do
  `misto()`; o 2.4b, a herança (N5) por `self`, `cls`, `super()`, tipo e
  `@property`, o `__init__` herdado, o método abstrato (ambíguo, com as
  sobrescritas) e a saída do candidato fora da cadeia. Os níveis N0-livre,
  N5 e N8 estão no alvo. Efeito do 2.4b no Healthchecks: 198 chamadas, todas
  previstas (174 `type`, 1 ambígua, 23 sem candidato, das quais 4 links
  falsos); nenhuma no ray tracer; o grafo e o `used_by` não mudam.
  Ficam: para o 2.5, `executor.submit(notify, flip)` (F20); para o 2.6,
  `ch.notify` (F22) e `ch.transport.is_noop` em `Flip.select_channels`; sem
  passo, o despacho por `TRANSPORTS` (Fase 2B); limites em SEARCH.md
  (`:=` em compreensão, `pub`, base de fora que delega, `cls(..)`, campo
  herdado).
  **Ordem do resto da Fase 2 (proposta do usuário, 20/11/2026, a decidir
  depois do 2.4):** 2.4 → 2.5 → Fase 2B → 2.6. Os workflows A, B e C do
  Healthchecks só dependem da 2B e do 2.5 (as rotas passam a view como
  referência); o 2.6 só serve ao workflow D (`ch.notify`, F22).
- 2.4c Despacho polimórfico (registrado em 25/11/2026, antes do 2.5). O
  correto é o corpo que roda (2.1, Q6 do 2.3, V1 do 2.4b):
  - sem sobrescrita numa subclasse do projeto, link único;
  - com sobrescrita, ambíguo, com o método do tipo estático mais as
    sobrescritas;
  - com método abstrato, ele sai da lista (como na DEC-43).
  Corrige HC-140 (`cls.raise_for_response`, 11 sobrescritas) e HC-142
  (`self.transport.is_noop`, 13), links falsos conhecidos da linha de
  base. Pela contagem do 2.4b (simulador), no Healthchecks há mais um
  caso, `self.fix_asterisks` em
  `hc/integrations/slack/transport.py:72` (`Discord` sobrescreve), e
  `form.get_json()` em `hc/integrations/call/views.py:26`, de construtor
  exato, que roda o próprio.
- 2.5 PIL-G5/N7: relação "passa referência", nunca "chama".
- 2.6 Genéricos (N6), por último (ch.notify: 33 → 1).
- Portão de cada passo: `node tools/precisao/portao.js --studio .studio
  --base tools/precisao/linha-de-base.json`. Reprova se `link_falso`
  subir ou as corretas caírem (Python ou Rust; DEC-25, PLANO-H:2284),
  se uma edition antiga mudar ou se o `.studio` original mudar.

### Fase 2B — Relações de workflow
- Pontos de entrada: rotas (PIL-G4), management commands, jobs.
- Chamadas soltas no topo dos arquivos Python (Q2 da Parada 1 do 2.3b,
  20/11): 48 no Healthchecks ficam fora do grafo, porque nenhum símbolo
  as contém (34 em `hc/settings.py`; os processos em `manage.py`,
  `hc/wsgi.py`, `docker/`). Tocam a 2B estas 5:
  `register_converter(..)` em `hc/api/urls.py:30-31` (os conversores
  `quoted` e `sha1` das rotas), o prefixo em `hc/urls.py:12` (2
  chamadas) e `admin.site.unregister(User)` em
  `hc/accounts/admin.py:301`. Um símbolo de módulo dono só do que hoje
  é descartado (sem mover os decoradores) resolve, com edition própria,
  decidido pelas rotas. As listas de rotas (`urlpatterns = [..]`) já
  estão no grafo, observadas pela variável.
- Lacuna sem passo (provavelmente aqui; Parada 1 do 2.4b, 24/11):
  despacho por registro. `Channel.transport` escolhe a classe em
  `TRANSPORTS` (`hc/api/models.py`) e a cria com `import_module` e
  `cls(self)`. É o que separa os dois trechos do workflow D: até
  `Channel.notify`, e do `notify` de cada transporte em diante.
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
- **Proposta do usuário, a decidir (25/11/2026): o agente aprovador.**
  Reabre a DEC-22.
  - Conferência automática primeiro.
  - Aprovador com contexto próprio, que registra o motivo e o que leu.
  - Três níveis: proposto, aprovado por agente e validado por humano.
  - Fila humana curta, por risco, mais uma amostra para medir o
    aprovador; a confiança é ajustada pela concordância.
  - Revogação que se propaga.
  - Um único portão humano: a documentação que vai para pessoas.
  - Divisão em 3a (antes da Fase 4) e 3b (depois).

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
**Proposta do usuário, a decidir (25/11/2026)**, com divisão de 6.1 a 6.5:
- Servidor próprio, separado do Studio.
- Superfície da API: grafo, pedido de evidência, acervo, workflows e
  auditoria.
- Identidade de quem chama, com validação humana só por credencial
  humana.
- Respostas reproduzíveis (edition, perfil, resolver).
- Versão do contrato, com regra de evolução.
- Limites e erros.
- Escrita concorrente com controle de versão.
- Local por padrão.
- Adaptador MCP.
- Testes do contrato e portão pela API.
- Uso no Kubernetes local: volumes persistentes, credenciais em secrets,
  imagem com o hash das fontes.

**Front, proposta do usuário (25/11/2026):** refazer no final, depois do
6.5, como ferramenta de investigação sem agentes nem MCP. A decisão é do
usuário, quando chegar lá.

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

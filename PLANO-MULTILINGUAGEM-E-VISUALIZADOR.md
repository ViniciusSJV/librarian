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
  **Ordem do resto da Fase 2 (decidida pelo usuário em 25/11/2026):** 2.4d-0
  → 2.4d → 2.4e → 2.4f → 2.5 → estratos → Fase 2B → 2.6 (2.4e e 2.4f
  incluídos na validação do 2.4d; os estratos, passo próprio desde o lote de
  25/11/2026, abaixo do 2.5). Os workflows A, B e C do Healthchecks só
  dependem da 2B e do 2.5 (as rotas passam a view como referência); o 2.6 só
  serve ao workflow D (`ch.notify`, F22). **Condição para a Parada 1 da 2B:**
  decidir, com contagem, se o tipo do elemento das coleções do Django
  (QuerySet, gerenciador) entra na 2B ou espera o 2.6.
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
  Parada 1 aprovada em 25/11/2026 ([2.4c-parada-1.md](docs/reports/fase-2/2.4c-parada-1.md));
  Parada 2 em [2.4c-parada-2.md](docs/reports/fase-2/2.4c-parada-2.md), validada em
  25/11/2026 (DEC-44 vigente, linha de base trocada).
  **Fechamento do 2.4 (validado pelo usuário em 25/11/2026):** o 2.4 entregou o escopo e o
  alias (2.4a), a herança (2.4b) e o despacho polimórfico (2.4c): com
  sobrescrita numa subclasse do projeto, a chamada é ambígua, com os corpos
  que podem rodar. Efeito do 2.4c no Healthchecks: 3 chamadas, todas
  previstas (HC-140, HC-142 e `self.fix_asterisks`); nenhuma no ray tracer; o
  grafo e o `used_by` não mudam. Dos `link_falso` da amostra sobra a HC-27
  (tipo de fora, `pycurl`). Ficam: o `used_by` pela regra (2.4d); o que o
  fechamento do 2.4b já deixou para o 2.5, o 2.6 e a Fase 2B; limites em
  SEARCH.md (base de fora antes numa subclasse, subclasse de teste, campo e
  anotação que não são construtor exato); e a lacuna de `Classe.metodo(..)`
  (abaixo).
  **`@abstractmethod`:** o Healthchecks não tem nenhum, nem `ABCMeta` nem
  `ABC` (0 em classe sem `ABCMeta` na cadeia, 0 sobre um concreto). Nenhum
  código mudou; fica o limite, na DEC-44 e no SEARCH.md: sem `ABCMeta`, o
  decorador não impede o corpo de rodar.
  **Lacuna (25/11/2026; passo 2.4d-0): `Classe.metodo(..)` no Python não usa a
  classe.** O receptor que nomeia uma classe não tem tipo estático, e a
  chamada só fecha quando o nome tem um candidato só. No Healthchecks, 71
  chamadas com classe do projeto: 27 `unique` (26 métodos, todos certos, e
  uma classe aninhada construída), 26 sem candidato (método do pydantic, na
  base de fora: certo) e **18 `homonym`, que fechariam todas** se a classe
  fosse usada: `TokenBucket.authorize` (13), `PdConf.load`, `EmailConf.load`,
  `TelegramCallback.load`, `Signal.send` e `Telegram.send` (HC-156). Todas de
  produção, nenhuma com sobrescrita. No Rust não há a lacuna: `T::f(..)` usa o
  tipo desde o 2.1 (1.366 chamadas, 0 `homonym`); as 100 de tipo do projeto
  sem fechar são `Object::from` e `Patterns::from`, com vários `impl From<X>`,
  que só o tipo do argumento decide (limite sem passo, no SEARCH.md). A lacuna
  do Python é do passo 2.4d-0 (abaixo).
  **Propostas do usuário (25/11/2026), a decidir**, escritas onde serão lidas:
  [Propostas de arquitetura](#propostas-de-arquitetura-a-decidir) (papéis das
  peças e agentes), notas nas Fases 4 e 6 e
  [Fronteiras entre sistemas](#fronteiras-entre-sistemas-proposta-a-decidir),
  depois da Fase 7.
- 2.4d-0 `Classe.metodo(..)` usa a classe (proposta aceita pelo usuário em
  25/11/2026; antes do 2.4d). Python. Parada 1 aprovada em 25/11/2026
  ([2.4d0-parada-1.md](docs/reports/fase-2/2.4d0-parada-1.md)); Parada 2 em
  [2.4d0-parada-2.md](docs/reports/fase-2/2.4d0-parada-2.md), validada em
  25/11/2026 (DEC-45 vigente, linha de base trocada).
  - `Classe.m(..)` com `Classe` do projeto (importada, do mesmo módulo ou
    aninhada) resolve pela cadeia da própria classe, para cima, na ordem de
    herança. Nunca desce para subclasses: não há despacho, ao contrário do
    2.4c.
  - Meta: as 18 `homonym` (`TokenBucket.authorize` 13, `PdConf.load`,
    `EmailConf.load`, `TelegramCallback.load`, `Telegram.send`/HC-156 e
    `Signal.send`); medir também o efeito indireto pelo retorno (DEC-41).
  - Motivo da ordem: o `used_by` pelo inverso da regra (2.4d) herdaria as 18.
  - Resultado: as 18 fecham, e nenhuma outra chamada muda; o efeito indireto
    pelo retorno é 0.
  - **Link falso conhecido das regras antigas (2.4b), sem passo:** o método
    religado (`def m` e depois `m = outra` no mesmo corpo), chamado por um
    objeto. 0 casos no Healthchecks. **Gatilho:** se aparecer um caso, corrigir
    antes do passo seguinte.
- 2.4d `used_by` pelo inverso da regra (registrado em 25/11/2026, pedido do
  usuário; depois do 2.4d-0 e antes do 2.5). O `used_by` calculado pelo inverso
  da regra de resolução, incluindo o despacho polimórfico, à parte dos usos
  por nome. Motivo: 705 dos 2.638 links únicos do Python não apareciam no
  `used_by` do alvo pela API, dos quais 376 (14%) pela regra e 329 pelo teto
  de 120 pontos da vista (`Webhook.is_noop` responde 0 usos), e é essa
  pergunta que responde "quem é afetado se eu mudar isto?". Análise na
  seção 6 da [Parada 1 do 2.4c](docs/reports/fase-2/2.4c-parada-1.md).
  Parada 1 aprovada em 25/11/2026 ([2.4d-parada-1.md](docs/reports/fase-2/2.4d-parada-1.md));
  Parada 2 em [2.4d-parada-2.md](docs/reports/fase-2/2.4d-parada-2.md), validada em
  25/11/2026 (DEC-46 vigente, linha de base trocada).
  - **Links falsos conhecidos, sem mudar regra (Q11):** os usos ligados de
    `Ping.get_body` (`hc/api/management/commands/smtpd.py:37` e `:41`,
    `hc/front/views.py:800` e `:805`, que são de `email.message`) e de
    `Groups::len` (`src/obj.rs:90`, `src/camera.rs:129`, `src/count.rs:133`
    e `:151`, que são de `Vec` e `str`), e 10 dos 11 de `Sorteio::next`
    (`src/obj.rs:19`, `:21`, `:22`, `:23`, `:28`, `:63`, `:66`, `:81` e
    `src/count.rs:133`, `:151`, que são de `Iterator`). São 18, `unique` com
    receptor sem tipo.
- 2.4e Caça às lacunas (proposta aceita pelo usuário em 25/11/2026; entre o
  2.4d e o 2.4f; Parada 1 aprovada com ajustes; Parada 2 validada em
  26/11/2026; [2.4e-parada-2.md](docs/reports/fase-2/2.4e-parada-2.md)).
  Só medição, sem mudar regra; fechado como medição.
  - **Pendentes depois da Parada 2 (26/11/2026), decididos pelo usuário depois
    do mapeamento do repositório:**
    - o X (DEC-48, item 4);
    - o teto das "ligadas pelo nome" (DEC-48, item 9; medido: Python 6,8%,
      Rust 8,8%, fora de testes, agrupamento provisório);
    - os valores da cobertura baixa (complemento à DEC-48, item 9);
    - o teto do caminho: o D e o CR-1 passam das 6 camadas, e o D passa dos
      120 nós do Studio.
    Validados pelo usuário em 26/11/2026: a verdade dos 10 candidatos novos
    da FM-2 (link falso nos 10) e o `FM-6` (chamada dentro de macro do
    Rust: lacuna, sem passo; 43 em produção e 272 em teste). A família de 2.166 homônimos para externo
    fica sem id (é a recolhida da DEC-46).
  - **Decidido pelo usuário em 25/11/2026 (DEC-48, itens 1 a 8):**
    - **Workflows:** a sessão do 2.4e propõe as funções de entrada dos 4
      workflows do Healthchecks e dos caminhos do ray tracer, a partir do PIL
      e dos fatos HC-F1 a HC-F26; o usuário valida antes da medição.
    - **Alcance de cada workflow:** a união do que o grafo alcança e do que
      o gabarito de execução percorre; o que só o grafo alcança fica marcado
      "alcançado só pelo grafo". Motivo: medir o workflow só pelo grafo
      usaria as ligações sob suspeita para medir a si mesmas.
    - **Entrega:** a lista de lacunas, ordenada pelo número de chamadas
      afetadas, com a coluna "cai num workflow".
    - **Regra de parada** (no lugar de "o usuário escolhe quais viram
      passo"): a unidade é a família de erro. Uma família vira passo próprio
      (2.4g, 2.4h...) se tiver ligação falsa dentro de um workflow ou se
      afetar mais de X chamadas no total; X é fixado pelo usuário com os
      números da Parada 1 do 2.4e. Uma ligação que falta fora dos workflows
      ganha selo de lacuna, sem passo. Ligação falsa pesa mais que ligação
      que falta.
    - **Dois gabaritos**, comparados com a resposta do Librarian em todas as
      chamadas: um de execução (a suíte do Healthchecks rodada gravando as
      chamadas reais, quem chamou quem) e um de tipos (no Python, pyright ou
      mypy com django-stubs; no Rust, rust-analyzer no ray tracer). O mypy
      fica como alternativa porque o pyright é fraco no Django dinâmico, que
      é o que pesa na 2B. No ray tracer, o gabarito de execução é proposto
      na Parada 1 do 2.4e.
    - **O gabarito é só régua:** serve para medir e para o portão, nunca é
      evidência e nunca entra no grafo (quem mede não pode ser quem
      responde; "código não prova execução" vale nos dois sentidos).
    - **Porta aberta (adaptador opcional):** se o 2.4e mostrar que o pyright
      liga, de forma barata, a maior parte das ligações "ligado pelo nome",
      pode-se abrir uma decisão própria para um adaptador opcional e
      plugável, fora do núcleo (como o perfil do Django; DEC-21), com selo
      próprio, condicional: "ligado pelo verificador externo", com a
      ferramenta e a versão registradas, só existe se essa decisão for
      aprovada. A decisão própria tem de respeitar ou reabrir a DEC-29,
      princípio 4 ("o Librarian não executa ferramentas externas"; por
      exemplo, lendo uma saída gerada fora do Librarian). Nesse caso, a régua
      do portão passa a ser a execução ou outra ferramenta, nunca a mesma que
      responde.
    - A automação da aprovação de fatos é decidida na Fase 3.
  - **Cobertura do gabarito (usuário, 25/11/2026):** a medição informa, além
    da taxa, a cobertura de cada gabarito em cada família e em cada workflow:
    "o gabarito não discorda" pode ser "o gabarito não viu". A Parada 1 diz
    também como cada gabarito roda (ambiente: virtualenv, banco, cargo) e com
    que versões, registradas. Ela propõe, com os números, o limite de
    "cobertura baixa" da terceira condição da 2B (complemento à DEC-48, item
    9), fixado pelo usuário, como o X e o teto.
  - Serve também para conferir o 2.4d e a Fase 2B.
  - **Prioridade (usuário, 25/11/2026):** separar os links `unique` com
    receptor (cerca de 900: 655 no Python e 245 no Rust; 233 e 94 pela
    definição do portão, no item "Antes dele: feito", abaixo) em ligados pelo
    tipo ou pelo nome. É a família de `Ping.get_body`, `Groups::len`,
    `FakeCurl` e HC-27. As 18 chamadas de `Ping.get_body`, `Groups::len` e
    `Sorteio::next` são links falsos conhecidos, a medir aqui.
  - **Propor como o gabarito entra no portão de cada passo (usuário,
    25/11/2026):** comparado com os gabaritos (pyright ou mypy com
    django-stubs; rust-analyzer) em todas as chamadas, sem entrar no grafo.
    Quando o gabarito entrar no portão (proposta do 2.4e,
    com aprovação do usuário e linha de base nova, DEC-37), as discordâncias
    não podem subir; até lá, ele mede à parte, sem reprovar. Dizer se uma
    discordância é link falso cabe ao usuário (DEC-22; DEC-48, item 6).
  - **Antes dele (usuário, 25/11/2026): feito.** O estrato sorteado de 30
    links `unique` com receptor (UQ-1 a UQ-30; semente
    `estrato-unique-2026-11-25`; verdade validada pelo usuário em 25/11/2026;
    folha [estrato-unique-revisao.md](docs/reports/fase-2/estrato-unique-revisao.md)):
    Python 1 de 15 `link_falso` (6,7%, IC 95% 0,2% a 31,9%), Rust 2 de 15
    (13,3%, 1,7% a 40,5%). Piso contado: Python pelo menos 21 de 233,
    verificado; Rust pelo menos 11 de 94. Decidido com ele (usuário,
    25/11/2026), para este passo:
    - **As duas contagens:** 233 (Python) e 94 (Rust) pela definição do
      portão (fora de testes, sem `self.m()`, alvo método); 655 e 245 com
      testes e `self.m()` (o `lista.js` do 2.4d, todas as chamadas).
    - **Duas famílias a medir:** o dublê de teste ligado por código de
      produção (17 links de `FakeCurl` em `hc/lib/curl.py`, com a HC-27) e o
      método da biblioteca padrão do Rust com nome de método do projeto
      (`len`, `next` e `clamp`, este pela UQ-23, `f64::clamp` →
      `Color::clamp`).
    - **Comparar:** medir a taxa nesta mesma população (233 e 94) e comparar
      com a do estrato.
    - **A conferir (não são links falsos conhecidos):** os dois `pixel.clamp`
      de `src/canvas.rs:50` e `:97`, e os nomes `first`, `last` e `get`
      chamados sobre `Vec` (ex.: `src/camera.rs`, em `benchmark_render`), se
      algum tiver homônimo no projeto.
    - **Portão:** a fixture `tools/precisao/fixtures/estrato-unique.json`
      existe, e o portão não a lê. Ligá-la (só medir, tabela à parte, com
      prova de veredito igual) é da Parada 1 deste passo ou do 2.4f.
  - Duas lacunas do Rust, achadas na amostra do 2.4d: o tipo do elemento de um
    array (PH-27: `self.data[1][1].equivalent(..)` sai possível, mas sempre
    roda o de `f64`) e o despacho pelo método padrão de uma trait (PH-34: o
    2.4c só existe no Python).
  - Medir também a lacuna de `T::from` no Rust, quase toda impossível (115 das
    132 de `Patterns::from`, medido).
- 2.4f Correção dos links `unique` ligados pelo nome (registrado pelo usuário
  em 25/11/2026; entre o 2.4e e o 2.5; sem passo aberto). É a família de
  `Ping.get_body`, `Groups::len`, `Sorteio::next`, `FakeCurl` e HC-27, com os
  18 links falsos conhecidos. A Parada 1 escolhe, com os números do 2.4e:
  - (a) separar "ligado pelo tipo" de "ligado pelo nome": não tira nenhum
    link, e entra de qualquer forma;
  - (b) rebaixar para possível quando o nome existe numa biblioteca conhecida
    (`next`, `len`, `get_body`), o que antecipa parte do perfil de externos
    conhecidos;
  - (c) rebaixar sempre que o tipo é desconhecido, o que derruba corretas e
    exigiria exceção da DEC-34.
  - Registrado sem decidir (usuário, 25/11/2026, estrato UQ): uma opção (d),
    não ligar uma chamada de produção a um método declarado em arquivo de
    teste (fecharia os 17 de `FakeCurl`), que só pode ser escolhida com a
    contagem de quantas corretas ela derruba; e `clamp` na lista de nomes da
    opção (b).
  - **Direção (decidida pelo usuário em 25/11/2026, sem DEC):** (a)+(b), a
    confirmar com os números do 2.4e; a (a) entra de qualquer forma; a (c)
    fica descartada. A (d) e o `clamp` na (b) continuam registrados sem
    decidir.
  - **Selos, onde entram e composição do envelope: DEC-49.** Quatro selos
    ("ligado pelo tipo", "ligado pelo nome", "possível" e "lacuna"), com o
    motivo e, quando houver, a família conhecida; o mesmo selo no "o que
    chama" e no "quem usa" pela regra. Na tela, no "o que chama" e no "Onde é
    usado?"; no envelope, no "o que chama", e a nota ao modelo passa a dizer
    "pelo nome" em cada chamador do "quem usa" (fecha a pendência do
    "caminho: chama um alvo"). Se a mudança da nota pedir versão nova do
    envelope, ou se a (b) tirar um `unique` rebaixado a "possível" do filtro
    SETTLED, a Parada 1 decide se isso pede `envelope-2`, com a evidência do
    2.4e exigida pela DEC-13.
  - **Pendências da DEC-49 (registradas sem decidir):** a classificação de
    cada resolução (`unique`, `type`, `trait_default`, `import`, `scope`,
    `receiver`, `chosen`) num dos selos é proposta na Parada 1 do 2.4f, com
    contagem, e validada pelo usuário; um tipo que nasce de um candidato só
    pelo nome (DEC-41, caso b; HC-110) não vira "ligado pelo tipo" sem
    decisão; a Parada 1 do 2.4f propõe.
- 2.5 PIL-G5/N7: relação "passa referência", nunca "chama".
- Estratos (passo próprio desde o lote do usuário de 25/11/2026; entre o 2.5
  e a Fase 2B; sem passo aberto): refazer os estratos da estimativa
  ponderada, com o estrato `type` e a população do UQ, com a verdade validada
  pelo usuário. Já decidido em 25/11/2026 (pendência da estimativa ponderada,
  no ESTADO e no [README do portão](tools/precisao/README.md)); agora com
  lugar na ordem.
- 2.6 Genéricos (N6), por último (ch.notify: 33 → 1).
- Portão de cada passo: `node tools/precisao/portao.js --studio .studio
  --base tools/precisao/linha-de-base.json`. Reprova se `link_falso`
  subir ou as corretas caírem (Python ou Rust; DEC-25, PLANO-H:2284),
  se uma edition antiga mudar ou se o `.studio` original mudar.

### Fase 2B — Relações de workflow
**Ordem do resto da Fase 2 (decidida pelo usuário em 25/11/2026):** 2.4d-0 →
2.4d → 2.4e → 2.4f → 2.5 → estratos → Fase 2B → 2.6 (2.4e e 2.4f incluídos na
validação do 2.4d; os estratos, passo próprio desde o lote de 25/11/2026).
Os workflows A, B e C do Healthchecks só dependem
da 2B e do 2.5; o 2.6 só serve ao workflow D (`ch.notify`, F22).

**Condição para a Parada 1 da 2B (usuário, 25/11/2026):** decidir, com
contagem, se o tipo do elemento das coleções do Django (QuerySet,
gerenciador) entra na 2B ou espera o 2.6.

**Condição para a Parada 1 da 2B (usuário, 25/11/2026):** o conhecimento de
framework fica isolado como um perfil plugável (o Django é o primeiro),
separado das regras da linguagem. O perfil diz onde os fluxos começam (rotas,
comandos, jobs), como o banco é lido e gravado (ORM), quais bibliotecas do
framework são externas conhecidas e como ele escolhe implementações
(registro, injeção). Flask, FastAPI e, depois do Java, Spring devem entrar
como perfis novos, sem mudar o núcleo nem as regras do Python. A Parada 1 da
2B propõe a forma desse perfil e diz o que do Django não cabe nela.

**Condição para a Parada 1 da 2B (usuário, 25/11/2026; reescrita pela DEC-48,
item 9):** dois limites por linguagem, medidos pelo gabarito do 2.4e em todas
as chamadas, não na amostra: nas "ligadas pelo tipo", nenhuma ligação falsa sem
explicação (uma só bloqueia); nas "ligadas pelo nome", um teto fixado pelo
usuário depois do 2.4e; e nenhum fluxo da 2B usa uma ligação "ligado pelo nome"
sem mostrar o selo. Referência: o piso do estrato UQ (Python pelo menos 21 de
233; Rust pelo menos 11 de 94); o estrato dá só a ordem de grandeza.
**Complemento (usuário, 25/11/2026; registrado no estado da DEC-48, sem mudar
o texto do item 9):** um workflow com cobertura baixa do gabarito não passa
esta condição por ausência de discordância; precisa de um estrato sorteado com
a verdade validada pelo usuário, como o UQ. O limite de "cobertura baixa" é
proposto na Parada 1 do 2.4e com os números e fixado pelo usuário, como o X e
o teto.

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

### Propostas de arquitetura (a decidir)
**Proposta do usuário (25/11/2026), a decidir** (não abre passo nem decisão).
- *Papéis das peças:* a busca lexical é a porta de entrada (continua
  lexical, determinística e auditável; vira rota na Fase 6); o grafo é o
  caminho; o acervo é a memória; o LLM Engine se divide: o formato de claims
  e as conferências ficam no núcleo, como a conferência automática da Fase
  3, e a geração do texto passa para os agentes. Dependência: as perguntas
  01–14 (H1 a H3) e o Studio usam hoje a geração; a divisão não pode quebrar
  essa conferência. O usuário tem uma ideia de melhoria da busca lexical
  para depois.
- *Agentes:* LangGraph como orquestrador do agente aprovador (Fase 3b) e
  dos agentes da Fase 7 (pausa para decisão humana e estado guardado);
  LangChain opcional, como camada fina (troca de modelo e ferramentas pelo
  MCP da Fase 6); Langfuse auto-hospedado no Kubernetes, para
  observabilidade, custo e avaliação (a concordância entre o aprovador e o
  usuário; as folhas de revisão como datasets). O registro de auditoria
  oficial continua no acervo do Librarian; o Langfuse é a visão operacional
  e, se divergir, vale o acervo. Nada disso entra no núcleo (DEC-29).

**Contrato de evidência entre o Librarian e o agente** (proposta aceita como
direção pelo usuário em 25/11/2026; não abre passo nem decisão; vira DEC na
Fase 3, junto com o agente aprovador):
1. *O pedido:* o usuário pergunta em linguagem normal; o agente traduz num
   pedido ao Librarian (ex.: "o caminho a partir do recebimento de e-mail").
2. *O pacote:* o Librarian entrega trechos, linhas, ligações com selo e
   lacunas com motivo; não entrega o grafo para o agente passear.
3. *O rascunho:* o agente monta o rascunho; um caminho vale pelo elo mais
   fraco (com um elo incerto, o caminho inteiro é incerto).
4. *A pergunta-chave:* sem elo incerto, vai para a entrega; com elo incerto,
   o agente não chuta: pede evidência, primeiro nos piores elos (família de
   taxa mais alta, elo dentro de workflow).
5. *O pedido de evidência* é específico sobre o elo fraco (ex.: "de onde vem
   o message da linha 37?") e é respondido só com o que o código mostra: onde
   a variável nasce, o tipo anotado, o retorno, a herança e as sobrescritas.
   O gabarito nunca é evidência. O selo é do Librarian: o agente não o muda;
   anota a evidência e propõe ("a ligação diz X, a evidência aponta Y; humano
   confirmar"). Até 3 pedidos por elo; depois, declara a lacuna, sem ficar
   em círculo. O 3 é valor inicial, revisto nos primeiros usos (Fase 7).
6. *A entrega:* seta sólida; seta tracejada (hipótese, com o motivo e a
   evidência); buraco marcado (lacuna, com o que faltaria para fechar); uma
   lista curta de perguntas ao usuário; o registro do que o agente
   consultou.
7. *A validação:* o humano aprova o sólido, decide o tracejado e responde as
   perguntas; só então algo vira fato, com autoria humana. Se o agente
   aprovador da Fase 3 vier, ele faz esta etapa e o humano audita no fim; o
   fluxo não muda. Isso depende da reabertura da DEC-22 na Fase 3 (proposta
   do agente aprovador); enquanto ela vigorar, só o humano promove.
8. *O acervo:* o fato aprovado fica guardado e volta na próxima pergunta que
   passar pelo trecho.

Três regras: o Librarian nunca finge certeza (todo elo tem selo); o agente
nunca muda um selo nem promove um elo sozinho; só o humano transforma
hipótese em fato (DEC-22, vigente). Em aberto, para a Fase 3: onde entra
"possível" (tracejado ou lacuna); o nome de "sólido", que não pode sugerir
"verificado" (DEC-46, DEC-49); e se o que a evidência sustentou depois da
validação entra como sólido. O que é sólido foi resolvido pela família
calibrada (abaixo, item b).

**Família calibrada** (direção do usuário para a Fase 3, 25/11/2026; nenhuma
DEC muda; vira DEC na Fase 3, junto com este contrato; CTX seção 36):
- a) *Objetivo:* o Librarian nunca apresenta como certo o que não sabe. Toda
  ligação tem selo; uma ligação falsa só é aceitável dentro de um selo que já
  diz "incerto". O pacote entregue ao agente não chama ligação de fato; fato é
  só o que o humano aprovou e está no acervo.
- b) *Seta sólida:* sólido = "ligado pelo tipo", numa família com taxa de
  ligação falsa medida perto de zero no projeto. "Ligado pelo nome" é sempre
  tracejado (hipótese, com o motivo).
- c) *Validação em bloco:* o humano pode validar o sólido em bloco, por
  família, e gastar a atenção no tracejado. A seta sólida continua sendo
  ligação feita pela regra, não fato provado. É validação humana; não reabre a
  DEC-22. Risco aceito: "perto de zero" não é zero (atenuado pela validade do
  fato, pela revogação que se propaga e pela amostra sorteada); com tolerância
  zero, a validação é um a um.
- d) *O gabarito calibra famílias, nunca decide um fato:* mede a taxa de cada
  família e, onde houver, mede o agente aprovador; nunca aprova nem sustenta
  um fato específico. Complementa a DEC-48, item 6, sem mudá-la.
- e) *Aprovação por agente:* na Fase 3, abaixo ("Proposta aceita pelo
  usuário (25/11/2026), ao lado do agente aprovador").
- f) *Família sem taxa medida no projeto* conta como risco alto (tracejado)
  até ter um estrato sorteado com a verdade validada pelo usuário, como o UQ.
  Vale para projetos novos, Java e IaC.
- g) *Riscos registrados:* a taxa não viaja entre projetos (`FakeCurl` é do
  Healthchecks); uma família desconhecida é invisível quando a regra e o
  gabarito erram juntos (mais provável no gabarito de tipos); o custo de
  calibração por linguagem nova.

### Antes da Fase 3: o envelope e o "Medir tudo" pela regra
Registrado em 25/11/2026 (Q9 do 2.4d). Parada 1 não aberta.
- **A nota ao modelo.** Ela diz "caminho: chama um alvo", sem "pelo nome", e é
  lida pelos agentes. O `envelope-1` declara que os chamadores são por nome no
  campo `limits` da resposta e na ajuda da tela; a nota, não. O texto da tela
  ("chama X pelo nome escrito") foi corrigido no 2.4d. No 2.4f, a nota passa a
  dizer "pelo nome" em cada chamador (DEC-49). Fica neste item a leitura pela
  regra, com os selos, no "quem usa" do envelope e no "Medir tudo".
- Os chamadores do envelope e do "Medir tudo" usam o `used_by` por nome, com um
  candidato só. A regra nega 1.101 dos 2.316 no Python e 15 dos 1.041 no Rust, e
  liga 727 e 697 que não aparecem lá.
- Mudar isso muda o `envelope-1`: pede `envelope-2` e diz o que acontece com as
  sugestões gravadas.

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
  - Ver também, a decidir: o orquestrador do aprovador e os papéis das peças,
    em [Propostas de arquitetura](#propostas-de-arquitetura-a-decidir).
- **Proposta aceita pelo usuário (25/11/2026), ao lado do agente aprovador:**
  decidir junto com o contrato de evidência (em
  [Propostas de arquitetura](#propostas-de-arquitetura-a-decidir)).
  - O humano aprova fatos, não ligações: as ligações têm selo e o portão as
    vigia.
  - Fatos mecânicos cujos elos são todos "ligado pelo tipo", em famílias com
    taxa medida perto de zero no projeto, podem ser aprovados por agente, que
    registra a evidência do código que leu, com auditoria do usuário por
    amostra sorteada; um erro na amostra devolve o lote. Depende da
    reabertura da DEC-22 na Fase 3 (proposta do agente aprovador); enquanto
    ela vigorar, só o humano promove. (Texto trocado pelo usuário em
    25/11/2026, CTX seção 36: o anterior, "com concordância de fontes
    independentes (regra, tipo e execução)", usava os gabaritos de tipo e de
    execução como evidência para aprovar fato, contra a DEC-48, item 6, e o
    item 5 do contrato de evidência.)
  - A família calibrada (objetivo, seta sólida, validação em bloco, o
    gabarito que calibra famílias e nunca decide fato, família sem taxa
    medida e riscos) está junto do
    [contrato de evidência](#propostas-de-arquitetura-a-decidir) e vira DEC
    nesta fase, com ele.
  - Fatos interpretativos ("este workflow serve para X", "isto é seguro de
    mudar") continuam humanos.
  - Todo fato tem validade: quando o código por trás muda, volta para
    revisão.

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
- **Item de trabalho (usuário, 25/11/2026):** o pedido do agente "de onde vem
  esta variável?" não existe; construir antes dos agentes, junto com a lista
  do que o agente pode perguntar ao Librarian.
- **Proposta do usuário (25/11/2026), a decidir:** restrição proposta para as
  Fases 4 e 6: ids únicos entre projetos (fatos, workflows, símbolos) e fatos
  que podem citar mais de um projeto. Nota: a citação de fatos sem prefixo
  (F23, F20) deixa de ser segura com mais de um projeto.

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

- **Proposta do usuário (25/11/2026), a decidir:** restrição proposta para as
  Fases 4 e 6: ids únicos entre projetos (fatos, workflows, símbolos) e fatos
  que podem citar mais de um projeto. Nota: a citação de fatos sem prefixo
  (F23, F20) deixa de ser segura com mais de um projeto.

**Front, proposta do usuário (25/11/2026):** refazer no final, depois do
6.5, como ferramenta de investigação sem agentes nem MCP. A decisão é do
usuário, quando chegar lá.

### Fase 7 — Primeiros agentes
Agente de workflows e agente de documentação (fluxogramas para
humanos). A validação final é humana.

Ver também, a decidir: o orquestrador, a observabilidade e os papéis das
peças, em [Propostas de arquitetura](#propostas-de-arquitetura-a-decidir).

### Fronteiras entre sistemas (proposta, a decidir)
**Proposta do usuário (25/11/2026), a decidir** (não abre passo nem decisão).
- *Trilha "Fronteiras entre sistemas", depois da Fase 7:* contratos que
  atravessam o sistema (OpenAPI, GraphQL, webhooks, mensageria, IaC); a
  relação entre projetos no acervo; e a mudança de significado de um campo
  pega pela revogação de fatos (Fase 3), propagada aos fatos de outros
  projetos que dependem dele.

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

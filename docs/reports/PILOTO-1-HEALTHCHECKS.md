# Piloto Python 1: Healthchecks

**Executado em 18/11/2026.** É o primeiro uso do Librarian num projeto Python independente, que não foi escrito para ele. A pergunta do piloto foi esta: com os mecanismos que já existem, o Librarian consegue receber um projeto desconhecido, reconstruir seus workflows com evidência, transformar descobertas em fatos verificados e reutilizá-los?

**Condições:**

- O Librarian estava em `d949f91`, e o código dele **não foi alterado**.
- O Healthchecks foi fixado em `6c05a7adf266efb70f9517727bef9ea8b2bbf52f`, e o código dele **não foi alterado**. Nenhum comando foi executado no projeto.
- Tudo passou pelos mecanismos existentes do Studio, via HTTP: inventário, `/api/calls`, `/api/used-by`, `/api/gathered-question`, `/api/evidence`, `/api/send`, `/api/review`, `/api/facts` e `/api/envelope/suggest`. **Não houve implementação específica para o Healthchecks.**
- O modelo foi só o Qwen3-Coder 30B local, pelo Ollama. Nada foi enviado a serviço externo.
- **A condução e a revisão semântica foram feitas pelo agente que conduziu o piloto, e não pelo usuário.** Isso vale para os 26 fatos promovidos (ver [Autoria](#autoria-dos-fatos-do-piloto)).
- Os testes do Healthchecks foram **lidos, não executados**. "Confirmado por teste" quer dizer "um teste afirma esse comportamento", não "o teste passou neste commit".

Os dados da investigação ficaram na máquina local, em `.studio/projects/healthchecks-25597e41/` (run `20261118-061046`, turns 01 a 14, envios, `reviews.jsonl` e `facts.jsonl`), e não fazem parte do repositório.

---

## 1. Projeto

| | |
|---|---|
| Repositório | https://github.com/healthchecks/healthchecks |
| Commit | `6c05a7adf266efb70f9517727bef9ea8b2bbf52f`; `git status` limpo antes e depois |
| Linguagem e framework | Python, Django 6.1.3 (licença BSD-3) |
| Dependências (`requirements.txt`) | cronsim, Django[argon2], django-compressor, django-stubs-ext, fido2, minismtpd, oncalendar, psycopg, pycurl, pydantic, PyJWT, pyotp, segno, statsd, whitenoise |
| Arquivos versionados | 1.275: `hc/` 826, `templates/` 217, `static/` 134, `stuff/` 74, `docker/` 7 |
| Python ingerido | 657 arquivos: 420 de produção e 237 de teste |
| Apps | `hc/api` (144 .py), `hc/integrations` (140), `hc/accounts` (71), `hc/front` (19), `hc/payments` (15), `hc/lib` (14), `hc/logs` (9) |
| Entradas da aplicação | `hc/urls.py` → `hc/api/urls.py` (rotas de ping e API v1 a v3); `manage.py`, com 17 comandos de gestão, entre eles `sendalerts` (laço de alertas) e `smtpd` (ping por e-mail); `hc/wsgi.py` |
| Testes | 202 arquivos `test_*.py`. Os do workflow investigado: `test_ping`, `test_ping_by_slug`, `test_create_check`, `test_auth`, `test_check_model`, `test_check_going_down_after`, `test_sendalerts`, `test_flip_model`, `test_notify` |

## 2. Ingestão

Edition `20261118-061046`: **657 arquivos Python, 4.718 símbolos, 61.506 nós, 116.928 arestas e nenhum diagnóstico.** O `git.head` é o commit fixado, com `dirty: false`.

**Símbolos.** Produção: 648 classes, 636 métodos, 354 funções, 274 variáveis de módulo e 1.017 campos. Testes: 1.787 métodos.

**Arestas.** Há quatro tipos:

- `declares` e `contains`: estruturais;
- `observes`: ocorrência sintática;
- `name_candidate`: casamento por nome.

Não existe aresta de import, de chamada resolvida, de rota nem de decorador avaliado. O extrator Python declara essa limitação: *"No import resolution, dynamic dispatch, decorator evaluation or runtime behavior"*.

**Resolução das chamadas de produção (5.243 ocorrências):**

| | `name(...)` | `obj.name(...)` |
|---|---|---|
| Um candidato (`unique`) | 626 | 619 |
| Homônimos | 138 | 307 |
| Acima do teto, sem aresta | 0 | 90 |
| Nenhum candidato (biblioteca, ORM, builtins) | 1.356 | 2.107 |

### O que foi identificado corretamente

- **Todas as declarações do workflow.** Isso inclui funções aninhadas de decoradores (`authorize.wrapper`, `cors.decorator.wrapper`), as variáveis de rotas (`urlpatterns`, `uuid_urls`, `slug_urls`, `api_urls`) e trechos com os decoradores incluídos.
- **Chamadas `self.metodo()` e chamadas a funções do projeto com nome único.** Exemplos: `checks → create_check/get_checks`, `create_check → _lookup/_update`, `Check.ping → create_flip/going_down_after/prune/put_object`, `handle → handle_going_down/process_one_flip`, `handle_going_down → get_status/going_down_after` e `notify → select_channels/update_next_nag_dates`.
- **`include(uuid_urls)`**, resolvido para a variável.
- **Decorador sem parênteses (`@authorize`)**, como `name_reference`. O "onde é usado?" de `authorize` lista as oito views que ele protege.
- **Testes que chamam métodos de modelo.** O `used-by` de `going_down_after` lista seis testes; o de `select_channels`, outros seis.

**Suficiente para começar?** Sim. As declarações e as consultas "o que chama" e "onde é usado" deram o esqueleto do caminho em minutos. As ausências (G3 a G6, na seção 6) são justamente os elos que atravessam o framework. A leitura do código cobriu esses elos.

## 3. Workflows reconstruídos

Cada elo leva a evidência que o sustenta:

| Marca | Significado |
|---|---|
| **[grafo]** | Relação identificada pelo grafo (aresta `unique` correta) |
| **[nome]** | Relação ambígua identificada por nome (homônimo; o elo certo está entre os candidatos) |
| **[código]** | Relação reconstruída pela leitura do código; o grafo não tem aresta |
| **[banco]** | Relação inferida pela persistência no banco; não há chamada |
| **[teste]** | Comportamento afirmado por um teste (lido, não executado) |
| **Fn** | Fato promovido ao acervo (seção 5) |

### A. Cadastro e configuração de Check (API)

1. **Rota.** `POST /api/v{1,2,3}/checks/`: `urlpatterns` → `include(api_urls)` [grafo] → `path("checks/", views.checks)` [código]. `test_create_check` e `test_auth` usam `/api/v1/checks/` [teste].
2. **`checks`** (`views.py:481`), com `@csrf_exempt` e `@cors("GET","POST")`. Quando o método é POST, chama `create_check` [grafo].
3. **`@authorize`** [grafo, `name_reference`] → `wrapper`:
   - chave sem 32 caracteres → "missing api key", 401 [teste];
   - projeto inexistente → "wrong api key", 401 (**F9**) [teste];
   - grava `request.project`, `readonly = False` e `v` (**F10**).
4. **`create_check`.** `Spec.model_validate` (pydantic, fora do grafo) → `_lookup` [grafo]. Se nada for achado e `num_checks_available() <= 0`, devolve 403 [nome] [teste `test_it_obeys_check_limit`]. Senão, cria `Check(project=…)`.
5. **`_update`** [grafo]:
   - tipo `simple` → `kind` e `timeout` (**F11**) [teste];
   - `grace` pelo laço `setattr`, quando não é `None` (**F12**) [teste];
   - `alert_after = going_down_after()` (**F13**) → `get_grace_start` [grafo], somado ao `grace` (**F8**) [teste];
   - `save()` inteiro na criação e `update_fields` na atualização (**F14**).
6. **Término.** `JsonResponse` com 201 (criado) ou 200 (reutilizado por `unique`) [teste `test_it_supports_unique_*`]. O item do modelo foi rejeitado; o comportamento está no código, linha 476.

### B. Recebimento de Ping

1. **Rotas.**
   - `ping/<uuid:code>` → `views.ping` [código].
   - `ping/<uuid:code>/` → `include(uuid_urls)` [grafo] → `fail` com `{"action": "fail"}` (**F26**) [teste `test_fail_endpoint_works`]; também `start`, `log` e `<int:exitstatus>`.
   - As rotas por slug vão para `ping_by_slug` [código].
2. **`ping_by_slug`.**
   - slug com maiúscula → 400 (**F3**) [teste];
   - busca por `slug` e `project__ping_key` (**F1**) [teste];
   - com `?create=1`, cria o Check (auto-provisionamento, código 278–293) [teste `test_it_auto_provisions_missing_check`];
   - delega a `ping(request, check.code, check, …)` (**F7**) [nome] [teste]. **[Corrigido em 18/11/2026: o grafo não dá aresta única aqui; dá homônimo de 2, `views.ping` e `Check.ping`. Por isso o elo é [nome], e a chamada que o F7 afirma depende de um link ambíguo (relatório 03 do Piloto 1b, Parte E).]**
3. **`ping`.**
   - busca por `code` quando `check is None` [nome, ORM]; 404 se o Check não existir [teste];
   - `exitstatus > 255` → 400 (**F2**) [teste];
   - `exitstatus > 0` → ação `fail` (**F4**) [teste];
   - `methods == "POST"` com outro método → ação `ign` (**F5**) [teste];
   - filtro de palavras-chave no corpo (código 236–246) [teste, 5 testes];
   - `rid` inválido → 400 [teste].
4. **Chamada.** `check.ping(remote_addr, scheme, method, ua, body, action, rid, exitstatus)` (**F6**) [nome: `views.ping` e `Check.ping`].
5. **`Check.ping`** (`models.py:493`), dentro de `transaction.atomic` e `select_for_update`:
   - Check pausado com `manual_resume` → `ign`;
   - `start` → `last_start` e `last_start_rid`;
   - `success` e `fail` → `last_ping`, `last_duration` e limpeza de `last_start` conforme o `rid`. Se o status muda, `create_flip(new_status, reason)` [grafo], com `reason` "fail" ou vazio;
   - `alert_after = going_down_after()` [grafo]; `n_pings` incrementado; `save` (o grafo dá um falso `unique`, G1);
   - `Ping(...)` [grafo] e `ping.save`;
   - fora da transação: `put_object` (S3) [grafo]; a cada 100 pings, `prune` [grafo].

   Os 50 testes de `test_ping` vão por HTTP até o banco [teste]. **Nenhum fato foi promovido sobre `Check.ping`**: os quatro itens do modelo foram rejeitados na revisão (seção 5).
6. **`create_flip`** grava um `Flip` com `processed = None` (código 665–672).

### C. Detecção de atraso pelo `sendalerts`

1. **Entrada.** `Command.handle`, achado pela convenção de comandos do Django [código]. O laço chama `handle_going_down` enquanto houver trabalho [grafo], depois `process_one_flip` [grafo], e dorme 2 s. O item do modelo foi rejeitado por misturar interpretação; o código está nas linhas 202–213.
2. **`handle_going_down`** escolhe o Check com `alert_after < now()` e status diferente de `down`, ordenado por `alert_after` (**F15**) [teste].
3. **`get_status`** [grafo]:
   - `last_start + grace` vencido → `down`;
   - status `new`, `paused` ou `down` → devolve o próprio status;
   - nos outros casos, compara o momento atual com `grace_start` e `grace_end` → `down`, `grace` ou `up` (código 355–380) [teste, 16 testes em `test_check_model`].
4. **Ainda não está `down`** → atualiza `alert_after` com `going_down_after()` (**F16**) [teste `test_it_updates_alert_after` e `test_it_handles_grace_period`].
5. **Está `down`** → `update(alert_after=None, status="down")` atômico. Se uma linha foi atualizada, cria `Flip(reason="timeout", created=flip_time)` (**F17**) [teste `test_it_creates_a_flip_when_check_goes_down`].

### D. Processamento de Flip e envio de notificações

1. **`process_one_flip`.**
   - pega `Flip.objects.filter(processed=None).first()` (**F18**) [teste];
   - marca `processed`, com guarda contra corrida (**F19**) [teste];
   - chama `executor.submit(notify, flip)` (**F20**) [código: não aparece em `/api/calls`, mas aparece em `used-by`] [teste, com `notify` substituído por mock].
2. **`notify`.**
   - chama `update_next_nag_dates` [grafo];
   - obtém os canais por `flip.select_channels()` (**F21**) [grafo] [teste];
   - para cada canal, chama `ch.notify(flip)` (**F22**) [código: 33 homônimos, sem aresta].
3. **`Flip.select_channels`.** Não devolve canais em `new→up` e `paused→up`. Exclui os desativados e os que têm `transport.is_noop`, e ordena por `last_notify_duration` (código 1391–1409) [teste, 6 testes]. Sem fato promovido: a resposta foi reprovada na conferência mecânica.
4. **`Channel.notify`.**
   - com `is_noop`, devolve "no-op";
   - cria e salva uma `Notification`;
   - chama `self.transport.notify(flip, notification=n)` (**F23**) [código, despacho dinâmico];
   - com `TransportError`, marca `disabled` se o erro é permanente e grava `error = e.message` (**F24**);
   - por fim, atualiza `Notification.error` e, no `Channel`, `last_notify*`, `last_error` e `disabled` (**F25**) [teste por transporte, por exemplo `discord/tests/test_notify.py::test_it_disables_channel_on_404`].

### A relação mediada pelo banco

**A ligação entre `Check.ping`, a criação do `Flip` e o processamento posterior pelo `sendalerts` não é uma chamada direta.** É mediada pelo banco:

```text
Check.ping → create_flip → grava Flip(processed=None) → process_one_flip consome o registro
```

`handle_going_down` também grava `Flip`, pelo mesmo caminho. Essa relação foi **reconstruída pela leitura do código** [banco]. O grafo não a declara, porque não representa relações por tabela. O modelo não a mencionou, nem como fato nem como lacuna. Nenhum teste de ponta a ponta a confirma (seção 7).

### Onde a investigação parou

A investigação terminou **na fronteira das classes de transporte**. Qual classe responde é decidido por `TRANSPORTS[kind]` e `import_module` (`Channel.transport`, `models.py:1138-1150`). As classes de transporte (`hc/api/transports.py` e as integrações) ainda não foram aprofundadas.

## 4. Investigação com o LLM Engine

Perfil `qwen3coder` (Qwen3-Coder 30B local, `num_ctx` 32768), tarefa `verificavel`, formato claims. Cada pergunta foi um pacote de declarações juntadas à mão (`/api/gathered-question`), conferido antes do envio.

| Pergunta | turn | Declarações | Query | Tokens de prompt | Tempo | Conferência mecânica | Itens | Promovidos |
|---|---|---|---|---|---|---|---|---|
| Q1 rotas e views de ping | 01 | 5 | 26,9 KB | 7.085 | 77 s | aprovada | 9 | 7 |
| Q2 `Check.ping` e estado | 02 | 4 | 28,5 KB | 7.232 | 63 s | aprovada | 6 | 1 |
| Q3 cadastro (pergunta ampla) | 03 | 5 | 31,6 KB | 8.037 | 66 s | **reprovada** | 4 | — |
| Q3b despacho e chave de API | 04 | 4 | 20,7 KB | 5.438 | 38 s | **reprovada** | 5 | — |
| Q3c `_update` | 05 | 1 | 20,1 KB | 5.267 | 22 s | **reprovada** | 3 | — |
| Q3d `authorize` | 06 | 1 | 13,8 KB | 3.743 | 24 s | aprovada | 3 | 2 |
| Q3e `create_check` e `_lookup` | 07 | 2 | 15,0 KB | 4.074 | 16 s | aprovada | 2 | 0 |
| Q3f `_update` (pergunta estreita) | 08 | 1 | 20,3 KB | 5.302 | 31 s | aprovada | 4 | 4 |
| Q4 detecção de atraso | 09 | 3 | 23,3 KB | 6.028 | 49 s | aprovada | 5 | 3 |
| Q5 alertas (pergunta ampla) | 10 | 4 | 24,5 KB | 6.376 | 57 s | **reprovada** | 4 | — |
| Q5b `process_one_flip` e `notify` | 11 | 2 | 17,3 KB | 4.604 | 40 s | aprovada | 6 | 5 |
| Q5c `select_channels` e `Channel.notify` | 12 | 2 | 16,4 KB | 4.440 | 34 s | **reprovada** | 5 (1 LACUNA) | — |
| Q5d `Channel.notify` | 13 | 1 | 13,6 KB | 3.720 | 30 s | aprovada | 4 | 3 |
| Q6 reutilização: ping de falha | 14 | 7 + 8 fatos | 46,2 KB | 11.667 | 31 s | aprovada | 5 | 1 |

**Volume e falhas de envio:**

- **14 perguntas e 16 envios** ao Qwen3-Coder local.
- **Duas falhas de envio**, ambas em Q6, causadas pela queda do `llama-server` (erro CUDA) ao recarregar o modelo com a GPU de 12 GB quase cheia. O Studio registrou as duas como `provider_failed`, com a resposta HTTP 500 preservada. O terceiro envio, com o modelo já carregado, foi concluído.
- **Cerca de 318 KB de queries e 83 mil tokens de prompt** no total.

**Conferências e custo do pacote:**

- **Os 14 pacotes passaram por H1 a H3. O H4 não foi exercitado**: as declarações foram juntadas à mão, sem seleção sugerida, e o H4 deu "não se aplica" em todos.
- **Custo fixo de cerca de 10 KB por pacote** (cerca de 2,7 mil tokens) de esquema e instruções. Uma função de 36 linhas (Q3d) já gera 13,8 KB.

**Comportamento do modelo:**

- **65 itens candidatos**: 64 FACT e 1 LACUNA.
- **A única LACUNA era falsa.** Q5c/C5 diz que o código não mostra o que acontece com `disabled` e `error` em `TransportError`, mas as linhas 1176–1177 do mesmo pacote mostram.
- **O modelo não declarou nenhuma lacuna real**: nem os transportes, nem a relação pelo banco, nem o auto-provisionamento.
- **Perguntas estreitas, com uma ou duas declarações e "um item por comportamento", tiveram resultado muito melhor.** Com perguntas amplas, o modelo junta comportamentos num item e cita a função inteira.
- Às vezes ignora a instrução "não escreva números de linha".

**Erros barrados pela conferência mecânica (resposta inteira reprovada):**

- Q3/C4 afirma `check.grace = …`. Essa atribuição não existe; `grace` é gravado por `setattr`.
- Q3 e Q3b: `declaration_line` 479 (linha do decorador) em vez de 481 (`def checks`), nas três tentativas. Com `ping` (196/198), o modelo acertou.
- Q5: faixa `91-91` com a docstring inteira no `quote`, e uma linha vazia reescrita com espaços.

## 5. Acervo de fatos

| Estado | Como aparece no contrato atual | Quantidade |
|---|---|---|
| Candidato | Item de uma resposta | **65** |
| Citações aprovadas mecanicamente | `mechanical_status: passed` da resposta | **44** itens, em 9 respostas |
| Sustentado por código | Revisão semântica do agente | 26 |
| Confirmado por teste | **Só no texto da nota** (o contrato não tem campo) | 22 dos 26; 1 indireto (F22); 3 sem teste (F10, F13, F14) |
| Promovido | `facts.jsonl`, `status: current` | **26 (F1 a F26)** |
| Rejeitado na revisão semântica | `reviews.jsonl`, `rejected`, com nota | **18** |
| Desatualizado | `stale` na reconferência por texto | 0 (mesma edition) |

**Motivos das 18 rejeições:**

- 6: a afirmação ultrapassa as linhas citadas. Exemplo: Q2/C3 é verdadeiro, mas o `n_pings` está na linha 542, que não foi citada.
- 4: não atômicos ou vagos.
- 3: omitem uma condição que torna o item falso. Exemplo: Q2/C1 diz que o status vira `up` "caso contrário", mas com `start` ele não muda.
- 2: repetições na reutilização.
- 1: motivo errado (Q6/C3).
- 2: um composto e parcialmente errado (Q1/C1, rota `include(slug_urls)`), outro com a citação apontando para `try:` (Q1/C2).

**Proveniência de cada fato.** O fato guarda:

- id, enunciado, `path`, `symbol_name`, `declaration_line`, a faixa e as citações com `quote`;
- a edition (hash do manifesto, que registra `git.head = 6c05a7a…`);
- run, turn, send e claim;
- os hashes do claim, da resposta e da query, e o vínculo do pacote (H1 a H3);
- a nota e a data.

O commit não é campo do fato; chega-se a ele pela edition.

### Autoria dos fatos do piloto

**Os 26 fatos não foram aprovados pelo usuário.** A revisão semântica foi feita pelo agente que conduziu o piloto. O contrato atual grava `authorship: "revisão humana no Librarian Studio"` e o portão `human_review` em todo fato, e não tem campo para outro revisor. Por isso cada nota começa declarando que a revisão foi do agente, e não humana. Qualquer um desses fatos pode ser revogado por `/api/review` com `rejected`.

### Reutilização (Q6)

A pergunta Q6 atravessou o workflow de ponta a ponta para um ping de falha. Foi feita com `with_facts`, reaproveitando conhecimento anterior e exigindo informação nova.

- **Fatos recuperados: 8 de 12 aplicáveis** (F18 a F25), entregues em `evidence.reviewed_facts`. **F2, F4, F5 e F6 (de `views.ping`) não vieram** (P1, seção 6).
- **Evidências acessíveis:** as citações são reconferidas por texto contra a edition (`current`).
- **Repetição:** o modelo **não repetiu 7 dos 8** fatos entregues, mas repetiu o F22 (C5). Repetiu também o F6 (C2), que não lhe foi entregue.
- **Complemento:** **1 fato novo correto** (F26, a rota `/fail`). Outros 2 itens novos não se sustentavam nas linhas citadas e foram rejeitados.
- **Insuficiência:** nenhuma lacuna declarada; a relação pelo banco ficou sem menção.
- **Contexto:** reutilizar **não reduziu** o pacote. O código das declarações vai inteiro, e os fatos somam 1,9 KB como orientação não citável.

## 6. Limitações encontradas, por componente

### Graph Engine

| # | Limitação | Exemplo do piloto | Consequência |
|---|---|---|---|
| G1 | **Resolução falsa.** `unique` quer dizer "um só nome igual no projeto", não "resolvido" | 122 chamadas `.save()` de produção apontam para `hc/integrations/github/views.py:92 save`; `body.decode()` → `hc_extras.decode` (filtro de template); `"\n".join` → `matrix/client.py join`; `time.time()` → `ChannelsAdmin.time` | `/api/calls` segue esses elos como certos, e a sugestão de envelope os herda (P2) |
| G2 | **ORM e framework fora do projeto** | `Check.objects.get(code=code)` → homônimo de 8 métodos `get` de classes de teste | Ruído; a consulta real não aparece |
| G3 | **Omissão silenciosa.** Acima do teto de homônimos, o grafo guarda só a contagem (`ambiguous_candidates`), e `/api/calls` não informa o que deixou de fora | `ch.notify(flip)` (33 candidatos), `ch.transport.is_noop` (14), `super()` (140) | O elo `notify → Channel.notify` some; um consumidor pode ler ausência de resultado como ausência de relação |
| G4 | **Referências em rotas.** Acesso a atributo só casa com campos | `path("ping/<uuid:code>", views.ping)`: a ocorrência existe, mas sem aresta | Rota → view não aparece no "onde é usado?" |
| G5 | **Funções passadas como valor** | `self.executor.submit(notify, flip)` | `/api/calls(process_one_flip)` volta vazio; o `used-by(notify)` encontra a ligação |
| G6 | **Despacho dinâmico** | `Channel.transport` via `TRANSPORTS[kind]` e `import_module`; comando de gestão achado pelo nome do arquivo; decorador que envolve a view | A fronteira dos transportes e as entradas convencionais ficam fora do grafo |
| G7 | **Homônimos que incluem o próprio chamador** | `check.ping(...)` dentro de `views.ping` → {`views.ping`, `Check.ping`} | Sugere uma recursão inexistente; o elo real fica ambíguo |
| G8 | **Testes HTTP** | `self.client.get("/ping/<code>")` | O `used-by(views.ping)` não lista nenhum teste de `test_ping.py` |

### Ingest

- Imports não são representados como relações; não há relação entre módulos.
- Dependências externas não são inventariadas: aparecem só como 3.463 chamadas sem candidato.
- `is_test` marca métodos de teste, mas as classes de teste contam como produção.
- `templates/` e `static/` estavam nas entradas, mas só arquivos `.py` são ingeridos no fluxo testado.

### Evidências, fatos e reutilização

- **P1. Recuperação de fatos em funções decoradas (bug reproduzido).** `target_of` (`crates/librarian-studio/src/store.rs`) usa `chunk.start_line` como linha de declaração. Em Python, o trecho começa no primeiro decorador, enquanto o fato guarda a linha do `def` informada pelo envelope. A recuperação exige igualdade, então nem `reviewed_about` (pergunta reunida com `with_facts`, passo de cadeia) nem `suggest_envelope` acham fatos de funções decoradas. Reproduzido duas vezes:
  - em Q6, sem F2, F4, F5 e F6;
  - em `/api/envelope/suggest` para `ping` e `ping_by_slug`, que voltaram com `facts: []` embora F1 a F7 existam.
- **P2. A sugestão de envelope herda G1.** O suporte sugerido para `views.ping` inclui `hc_extras.decode`, um filtro de template. Deixa de fora `Check.ping`, que vira lacuna por homonímia.
- **P3. Custo fixo.** São cerca de 10 KB de estrutura e instruções por pacote. Fatos anteriores não reduzem o código enviado.
- **P4. Atribuição indevida.** A instrução enviada ao modelo diz que os `reviewed_facts` foram "aprovados por uma pessoa". No piloto, quem os revisou foi o agente.

### Verificação e LLM

- O modelo não declarou lacunas reais e produziu uma LACUNA falsa.
- Perguntas estreitas, com uma ou duas declarações, tiveram resultado melhor.
- **O portão vale para a resposta inteira.** Um item reprovado impede a promoção dos corretos da mesma resposta. Em Q3, C2 e C3 estavam corretos e foram perdidos; foram precisas novas perguntas.
- **Conferir citações não garante, sozinho, a correção semântica.** O `strict_text` pega código citado no texto, mas não afirmações em prosa fora das linhas citadas. 18 dos 44 itens aprovados mecanicamente foram rejeitados na revisão semântica.
- **Linha de declaração com decorador.** O modelo confunde o início do item com a linha de declaração quando há decorador. O verificador resolve faixas de citação (`citation_range_from_quote`), mas não esse caso.
- **Confirmação por teste** só aparece na nota, não num campo do contrato.

**O resultado do Qwen não mostra que o modelo conduz sozinho uma investigação.** A revisão semântica continua necessária.

## 7. Testes como evidência: código, LLM e testes

- **Os testes de `test_ping` (cliente HTTP) cobrem rota → view → `Check.ping` → banco.** Por exemplo, `test_it_works` confere `alert_after = last_ping + 1 dia + 1 hora`, que é `timeout + grace`. Isso confirma a composição F6 → `Check.ping` → F8 sem citar nenhuma função.
- **Nenhum teste cobre o workflow inteiro** ping → `Flip` → `sendalerts` → notificação. Cada etapa é coberta separadamente:
  - `handle_going_down` é chamado diretamente;
  - `test_it_processes_flip` substitui `notify` por um mock: confirma F18 a F20, mas não a notificação;
  - `notify(flip)` é testado só pelas datas de lembrete;
  - `Channel.notify` é testado por transporte.
- **Onde houve divergência, os testes concordaram com o código.** O modelo divergiu do código em quatro itens:
  - `check.grace` inventado;
  - motivo "fail" incondicional;
  - status `up` "caso contrário";
  - a LACUNA falsa.

  Três deles seriam contrariados por testes existentes: `test_it_works` do cadastro com grace 60, `test_start_endpoint_works` e os testes de transporte com erro.

## 8. Critérios de sucesso

| # | Critério | Resultado |
|---|---|---|
| 1 | Ingestão de projeto Python independente | Sim |
| 2 | Estruturas e relações | Estruturas sim; relações parciais (G1 a G8) |
| 3 | Workflow completo reconstruído | Sim, de A a D até a fronteira dos transportes, com cada elo classificado pela evidência |
| 4 | Seleção progressiva | Sim, 14 pacotes de 1 a 7 declarações |
| 5 | Pacotes construídos e validados | Sim, H1 a H3 em todos; H4 não exercitado |
| 6 | LLM local | Sim, Qwen3-Coder, 16 envios |
| 7 | Fatos atômicos | Sim, com orientação explícita na pergunta |
| 8 | Verificação contra código e testes | Sim; testes lidos, não executados |
| 9 | Promoção ao acervo | Sim, 26 fatos, revisados pelo agente e não pelo usuário |
| 10 | Reutilização | **Parcial**: 8 de 12 fatos recuperados, 1 fato novo, 1 repetição de fato entregue; P1 impede o resto |

## 9. Próximos passos

A sequência está no [plano, roadmap](../../PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md#roadmap). A prioridade imediata é o **Piloto 1b**:

1. Corrigir P1, com teste de regressão para uma função Python decorada e verificação separada dos atributos Rust (`#[...]`).
2. Fazer `/api/calls` mostrar o que omitiu (G3).
3. Gerar uma nova edition e repetir Q6, comparando com este resultado.

G1 vem depois do 1b, só com medição do impacto no corpus Rust. A evolução de autoria e proveniência (separar revisão por agente de aprovação humana e permitir promoção por item) depende de especificação própria. A direção de longo prazo está em [Arquitetura de evidências para múltiplos consumidores](../ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md).

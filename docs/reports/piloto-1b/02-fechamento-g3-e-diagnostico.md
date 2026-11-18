# Piloto 1b — Fechamento do P1 e do G3, e diagnóstico de tipos e precisão (Partes A, B e C)

> Relatório entregue ao usuário em 18/11/2026. Texto como entregue. Só mudaram os links, que agora são relativos a esta pasta.
> Os caminhos de scratchpad citados abaixo foram copiados para [dados/](dados/):
>
> - `scratchpad\tipos\` → `dados/tipos/`
> - `scratchpad\amostra\` → `dados/amostra/`
> - `scratchpad\g3\depth.js` e `dup.js` → `dados/g3/`

## Resumo

- **Parte A:** P1 e G3 commitados em separado (`987d7a8` e `aae51d7`), sem push. `unresolved` não repete ocorrências, mas não sai em ordem de arquivo/linha/coluna.
- **Parte B:**
  - Python não sabe tipos de variáveis: em todos os níveis com homônimo, a chamada vai para todos os homônimos do nome escrito.
  - Rust grava o tipo quando ele está escrito, e o inventário usa isso para escolher o método certo. A vista "o que chama?" ignora essa informação.
- **Parte C:**
  - **Healthchecks:** nenhum link errado nas chamadas sem candidato. Mas 4 dos 8 links únicos estão errados, todos pelo mesmo motivo.
  - **Ray tracer:** nenhum erro.

Não mudei código de produção nas partes B e C, não promovi fatos e o `.studio` original está idêntico (932 arquivos, mesmos hashes). O acervo do Healthchecks continua sem alteração no git.

**A Parte D não foi feita:** ela chegou como bloco colado e não está na lista do que entregar. Se é para fazer, me diga e eu faço antes da Q6. *(Nota posterior: a Parte D foi confirmada e entregue; veja [03-inventario-e-auditoria.md](03-inventario-e-auditoria.md).)*

## Parte A

### A1. Commits

Nenhum arquivo misturava P1 e G3. Os commits estão sem coautoria, como está na sua preferência.

- `987d7a8` **fix(studio)** (P1): `claims.rs` e `store.rs`, com os 5 testes do P1.
- `aae51d7` **feat(graph)** (G3): `v2.rs`, `index.html` e `tests/g3.rs`.

### A2 e A3. Contagens de `unresolved`, repetição e ordem

| Semente | Prof. | Total | Por motivo | Mesma posição de início | Mesma ocorrência repetida |
|---|---|---|---|---|---|
| views.ping | 3 | 16 | 16 sem candidato | 0 | 0 |
| views.ping | 6 | 16 | 16 sem candidato | 0 | 0 |
| sendalerts.notify | 3 | 30 | 28 sem candidato · 2 acima do teto | 5 | 0 |
| sendalerts.notify | 6 | 58 | 55 sem candidato · 3 acima do teto | 7 | 0 |

- **Posições repetidas:** são todas chamadas encadeadas. Por exemplo, `User.objects.filter(...)` e `User.objects.filter(...).values_list("id")` começam no mesmo byte, mas terminam em lugares diferentes e são ocorrências distintas. Nenhuma ocorrência aparece duas vezes.
- **A ordem é estável, mas não é arquivo/linha/coluna.** A lista segue o percurso: camada por camada, chamador por chamador e, dentro de cada um, a ordem dos ids das arestas.
  - Em views.py, por exemplo, a linha 259 vem antes da 216, porque `call_observed` vem antes de `method_call_observed` na ordem alfabética.
  - Duas consultas iguais devolvem a mesma ordem.
  - Fica registrado e sem correção, como você pediu.

### A4. Ordem do corte no `node_limit`

Fica registrada como pendência, sem mudança.

## Parte B

### B1. Onde a resolução acontece

Há duas regras de resolução, e elas não concordam.

**1. Criação das arestas de candidato** (`graph.rs`, `Graph::occurrence`)

- Cada ocorrência gera uma aresta `name_candidate` para cada declaração com o mesmo nome escrito, na mesma linguagem, entre os tipos de declaração permitidos.
- Acima de 8 homônimos, não cria aresta e só guarda a contagem.
- Não há resolução de import nem de tipo; o próprio extrator Python diz isso: "No import resolution".
- Numa chamada de método, os candidatos válidos são, em Python, `method` e `function`; em Rust, `function`, `method` e `trait_method`.

**2a. "O que chama?" no Studio** (`v2.rs`, `calls_from`)

| Situação | Resolução |
|---|---|
| Escolha feita pela pessoa | `chosen` |
| 1 candidato | `unique` |
| `self.x()` com um único candidato na mesma classe ou impl | `receiver` |
| Qualquer outro caso | `homonym`: todos os candidatos, e o percurso não segue por eles |

Esta regra é igual para Python e Rust e **não usa tipo nenhum**.

**2b. Inventário de operações** (`inventory.rs`, `follow`)

- Aplica a mesma regra, mais três resoluções exclusivas do Rust:
  - `type`: o método do tipo escrito do receptor;
  - `trait_default`: o método padrão de um trait que esse tipo implementa;
  - `trait`: todas as implementações, quando todos os homônimos são do mesmo trait.
- O tipo vem da ocorrência `receiver_type`, que o extrator Rust grava quando o tipo está escrito.
  - Casos cobertos: `self`, parâmetro ou `let` anotado, `T::f(..)`, `T {..}`, campo declarado no mesmo arquivo, função livre do mesmo arquivo pelo retorno.
  - O extrator Python não grava `receiver_type`.

### B2. Relações que o grafo v2 registra

- **Arestas:** quatro tipos.
  - `declares` e `contains` são estruturais.
  - `observes` liga a declaração à ocorrência escrita.
  - `name_candidate` liga a ocorrência à declaração de mesmo nome.
  - Não existe aresta de "chama", "instancia" ou "tem tipo".
- **Tipo da ocorrência:** o que diferencia uma relação de outra.
  - Chamadas: `call_observed`, `method_call_observed`.
  - Tipos: `type_reference`, `receiver_type`.
  - Nomes e herança: `name_reference`, `base_class`.
  - Campos e structs: `attribute_access`, `field_assign`, `field_init`, `struct_literal`, `impl_self_type`.
  - Também há `declared_return`, `local_binding`, `parameter`, `loop` e `loop_iterable`, entre outras.

### B3. Os níveis N0–N8 com homônimos de propósito

Nos dois projetos coloquei homônimos de propósito:

- `salvar` em 4 classes no Python (3 tipos no Rust);
- `notify` em 4 classes, mais uma função solta;
- `livre` em dois arquivos;
- `herdado` e `padrao` também em outra classe.

As colunas significam:

- **"O que chama?"** é o que o Studio responde (`calls_from`).
- **Inventário** é o que a cadeia segue. Medi pondo `sqrt` em cada homônimo e um laço em cada caso.

Os projetos estão em [dados/tipos/](dados/tipos/): `py`, `rs`, `py-inv` e `rs-inv`, com os scripts `run.js`, `inv.js` e `real.js`.

**Python**

| Nível | Aresta / alvo em "o que chama?" | Inventário | Correto? | Relação de uso ou instância |
|---|---|---|---|---|
| N0 `livre(1)` / `unica()` | `livre`: homonym (2, os dois arquivos; o import é ignorado). `unica`: unique | só segue `unica` | `unica` sim; `livre` ambíguo | — |
| N1 `A = Cliente(); A.salvar()` | `Cliente()`: unique para a **classe**. `A.salvar()`: homonym (4) | não segue | Não | O construtor aparece como "chama a classe Cliente", misturado às chamadas. `A` não fica ligado a Cliente |
| N2 `def abc(a: Cliente)` | `a.salvar()`: homonym (4) | não segue | Não | `type_reference Cliente` existe, mas não está ligado ao parâmetro `a` |
| N3 `c = fabrica()` com `-> Cliente` | `fabrica`: unique. `c.salvar()`: homonym (4) | não segue | Não | `declared_return` existe, mas não é usado |
| N4 `self.repo = Repo()` | `self.repo.salvar(1)`: homonym (4) | não segue | Não | **Achado:** `Repo()` fica preso ao campo `Servico.repo`, não ao `__init__`. "O que chama?" do `__init__` não mostra a chamada |
| N5 herdado | `f.herdado()` e `self.herdado()`: homonym (Base e Outro) | não segue | Não | Existe `base_class Filho → Base`, mas não é usado |
| N6 `for ch in canais: list[Canal]` | `ch.notify(1)`: homonym (5, inclusive a função solta `notify`) | não segue | Não | — |
| N7 `executor.submit(notify, x)` | `submit`: sem candidato. **Nenhum link para `notify`** | — | Parcial | `name_reference notify → notify` existe no grafo, mas não vira link em "o que chama?" |
| N8 `from m import Cliente as C` | `C(...)`: sem candidato. `c.salvar()`: homonym (4) | não segue | Não | O alias não é resolvido |

**Rust**

| Nível | "O que chama?" | Inventário | `receiver_type` gravado? |
|---|---|---|---|
| N0 | `livre`: homonym (2); `unica`: unique | igual | — |
| N1 `let a = Cliente::new()` | `Cliente::new`: homonym (3 `new`); `a.salvar()`: homonym (3) | **type → Cliente::new e Cliente::salvar ✔** | `a → Cliente` |
| N2 `a: &Cliente` | homonym (3) | **type → Cliente::salvar ✔** | `a → Cliente` |
| N3 `fabrica()` em outro arquivo | homonym (3) | não segue | **Não** (só para funções do mesmo arquivo) |
| N3b `fabrica_local()` no mesmo arquivo | homonym (3) | **type ✔** | `c → Cliente` |
| N4 campo `repo: Repo` | homonym (3) | **type → Repo::salvar ✔** | `self.repo → Repo` |
| N5 `impl Notifica for Email` | `notify`: homonym (4); `padrao`: homonym (2) | **type → Email::notify ✔; trait_default → Notifica::padrao ✔** | `e → Email` |
| N6 `for ch in canais: &[Email]` | homonym (4) | não segue | Não (só para campos de `self`) |
| N6b `&[Box<dyn Notifica>]` | homonym (4) | não segue (a `Fila` não é do trait) | Não |
| N7 `.map(notify_one)` | nenhum link para `notify_one` | — | `name_reference → notify_one` existe |
| N8 `use ... Cliente as C` | `C::new`: homonym (3); `c.salvar()`: homonym (3) | não segue | `c → "C"`, que não casa com nenhuma declaração |

**Resposta direta à sua dúvida**

- **Python:** o grafo **não sabe** que A e B são Cliente. O construtor `Cliente()` aparece como "chama a classe Cliente" dentro de "o que chama?", e não como uma relação separada de instância. `A.salvar()` vai para todos os `salvar` homônimos.
- **Rust:** o grafo sabe `a → Cliente`, quando o tipo está escrito. Só o inventário usa isso; "o que chama?" continua mostrando os homônimos.

### B4. Por que `ch.notify(flip)` tem 33 candidatos

- **De onde vêm os 33:** são todas as declarações chamadas `notify` no projeto, de método ou de função:
  - `Channel.notify` e `Transport.notify`;
  - 30 `notify` de transportes em `hc/integrations/*` (dois no Slack);
  - a própria função solta `notify` de sendalerts.py.
- **O tipo real de `ch` é `Channel`, e o código escreve esse tipo.** A cadeia é:
  1. `def notify(flip: Flip)`: parâmetro anotado (N2);
  2. `channels = flip.select_channels()`, e `select_channels(self) -> list[Channel]` (models.py:1391): retorno anotado de um método, achado pelo tipo do receptor (N3 sobre N2);
  3. `for ch in channels`: elemento de `list[Channel]` (N6).
- **O que já existe no grafo:**
  - `type_reference Flip` na assinatura;
  - `declared_return list[Channel]` e `type_reference Channel` em `select_channels`.
- **O que falta:**
  - ligar o parâmetro `flip` ao tipo Flip;
  - achar `Flip.select_channels` pelo tipo do receptor;
  - ler o elemento do genérico `list[...]`;
  - passar esse tipo para a variável do laço.

  São N2, N3 e N6 juntos, e o N3 entre arquivos.

### B5. `self.executor.submit(notify, flip)` em `process_one_flip`

Confirmado: **o elo `process_one_flip → notify` não existe** em "o que chama?".

- A chamada `submit` sai como `no_candidate_in_project`.
- No grafo existe `name_reference "notify" → sendalerts.notify (function)`, pela regra do nome do próprio arquivo. A informação está lá, mas é uma referência de nome, não uma chamada.

## Parte C: linha de base de precisão

Sem nenhuma correção. A amostra é determinística (ordem por hash do id da ocorrência), com uma chamada por arquivo e sementes variadas. Exclui testes e chamadas `self.m()`.

Os dados e scripts estão em [dados/amostra/](dados/amostra/): `hc.json`, `rt.json`, `sample.js` e `pool.js`.

### Healthchecks: 20 chamadas

| Resultado | Qtde | Casos |
|---|---|---|
| Correta (link único certo) | 4 | `profile.choose_next_report_date()`, `check.cloaked_url()`, `Profile.objects.for_user()` → `ProfileManager.for_user`, `TokenBucket.authorize_pushover()` |
| **Errada** | **4** | `check.save()` e `ch.save()` → função de view `save` (github/views.py:92); `logger.error()` → função `error` (decorators.py:21); `result.replace()` → função `replace` (lib/string.py:10) |
| Ambígua | 0 | por sorteio, nenhuma caiu aqui |
| Sem candidato | 12 | Django, stdlib e pyotp; todas corretamente sem link |

### Ray tracer: 10 chamadas

| Resultado | Qtde | Casos |
|---|---|---|
| Correta | 2 | `self.e2.dot()` → `Tuple::dot`, `direction.is_vector()` → `Tuple::is_vector` |
| Errada | 0 | — |
| Ambígua | 2 | `checkers.color_at()` (7 candidatos) e `ray.direction.y.equivalent(0.)` (8 candidatos); nos dois, o alvo certo está entre os candidatos |
| Sem candidato | 6 | `f64`, `Vec` e `Cell` da biblioteca padrão; corretas |

### Distribuição em toda a população de chamadas `obj.metodo()`

| | Total | Sem candidato | Única | Homônimo | Acima do teto | Receptor |
|---|---|---|---|---|---|---|
| Healthchecks | 1956 | 1186 | 442 | 246 | 78 | 4 |
| Ray tracer | 515 | 313 | 104 | 71 | 14 | 13 |

- **Padrão dos erros:** em Python, uma chamada de método pode ter função solta como candidata. Dos 442 links "únicos" do Healthchecks, **197 apontam para uma função solta**. Nem todos estão errados: `modulo.func()` é legítimo. Mas é a origem de todos os erros da amostra.
  - No ray tracer, só 2 caem nesse padrão. Como em Rust `x.f()` nunca chama uma função livre, esses 2 estão errados por construção.
- **Na prática, "único" hoje não quer dizer "correto" em Python.**
- Na Parte B apareceram mais dois links errados do mesmo tipo: `time.time()` → `ChannelsAdmin.time` e `"\n".join()` → uma função `join`.

## Viabilidade dos níveis N1–N8

| Nível | Viável? | Esforço | Risco | Observação |
|---|---|---|---|---|
| **0. Usar a regra por tipo do inventário em "o que chama?" (Rust)** | Sim | Baixo | Baixo | Não é um nível, mas é o primeiro passo natural. Rust N1, N2, N3b, N4 e N5 já estão resolvidos no inventário. Precisa extrair uma função comum, sem duplicar a regra, como você exigiu |
| **Precisão: chamada de método não casa com função solta** | Sim | Baixo em Rust, médio em Python | Médio | Em Rust é correção direta. Em Python precisa distinguir `modulo.f()`, o que exige saber os imports do arquivo |
| N2 parâmetro anotado (Python) | Sim | Baixo-médio | Baixo | A anotação já é gravada; falta ligá-la ao nome do parâmetro |
| N1 construtor local (Python) | Sim | Médio | Médio | `x = Cls(...)` quando `Cls` é uma classe do projeto; homônimos de classe e reatribuição exigem cuidado |
| N4 atributo `self.x = Cls()` (Python) | Sim | Médio | Médio | Inclui decidir de quem é a chamada do `__init__` (o achado do N4) |
| N5 herança (Python) | Sim | Médio | Médio | `base_class` já existe; herança múltipla e bases homônimas são o risco |
| N3 retorno anotado | Sim | Médio | Médio | Em Rust, estender para outros arquivos; em Python, achar a função e depois o retorno |
| N8 alias de import | Parcial | Médio | Baixo-médio | O alias local (`as C`) é simples; resolver o módulo de origem é import resolution, que não existe hoje |
| N7 função passada como argumento | Sim | Baixo | **Médio (semântico)** | A referência já existe no grafo. Só pode aparecer como relação própria ("passa `notify`"), nunca como "chama", porque quem executa é o `submit` |
| N6 elemento de coleção | Sim, mas é o último | Alto | Alto | Exige N2 e N3 antes, mais a leitura de genéricos (`list[X]`, `&[X]`). É o que destrava `ch.notify(flip)`: de 33 candidatos para `Channel.notify` |

**Riscos comuns:**

- Toda mudança de extrator cria um novo perfil de edição e precisa manter as edições antigas iguais (`verify`).
- O grafo se declara "never compiler resolution". Um link por tipo precisa de resolução própria (por exemplo `type`, como no inventário), separada de `unique`.

## Arquivos

Originalmente em `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\c2d74a42-2372-4db9-9c84-8fc3b436eb57\scratchpad\`; agora copiados para [dados/](dados/):

- **Parte B:** `tipos\` (projetos `py`, `rs`, `py-inv` e `rs-inv`; scripts `run.js`, `inv.js` e `real.js`).
- **Parte C:** `amostra\` (`hc.json`, `rt.json`, `sample.js` e `pool.js`).
- **Parte A:** `g3\depth.js` e `g3\dup.js`.

O servidor de teste foi encerrado.

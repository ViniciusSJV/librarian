# Mapeamento do repositório — parte 3: como uma chamada vira ligação

**Estado:** Pendente de validação do usuário: Paradas 1, 2 e 2-bis (A a H). O gabarito do 2.4e (a antiga parte I) passou para a parte 4, [04-gabarito-2.4e.md](04-gabarito-2.4e.md), na Parada 3. Em aberto: MAP-Q9, MAP-Q10, MAP-Q11.

**Data:** 26/11/2026. **Base:** `main` em `6ac1865` (Parada 1), `be7d546` (Parada 2) e `ecae7a4` (Parada 2-bis). Worktree: só os relatórios do mapeamento. **Só leitura**: nenhum código, documento, `.studio`, Healthchecks ou ray tracer foi alterado. Nas Paradas 1 e 2 a `.studio/` estava vazia (MAP-O1) e o portão não foi rodado. Na Parada 2-bis ela já estava restaurada (MAP-O1, parte 1); o portão e uma sonda rodaram sobre **cópias** dela, e o original não mudou (seção E, "Parada 2-bis").

**Escopo.** Da chamada crua que o extrator registra até a ligação que `/api/calls` e `/api/used-by` devolvem. O léxico, a pergunta, o envelope, o evidence e o resto do studio ficam fora; deles, só se cita onde leem as ligações. Ids novos continuam a numeração das partes [1](01-mapa-geral.md) e [2](02-core-graph-engine.md): `MAP-O16` em diante e `MAP-Q9` em diante.

**Fontes.** Só `.rs`, `.py`, `.js`, `Cargo.toml`, `Cargo.lock`, `.github/` e JSON de configuração e de fixtures. Nenhum `.md` foi aberto além dos relatórios 01 e 02. Comentários no código, mensagens de commit e textos da página contam como **"diz ser"**.

**Regras de evidência**, as mesmas das partes 1 e 2:
- Cada afirmação traz `ARQUIVO:linha` ou o comando.
- **Verificado** quer dizer: por execução, ou pela leitura do código que faz a coisa.
- **Leitura minha** marca interpretação.

Caminhos abreviados nesta parte: `resolve.rs`, `graph.rs`, `v2.rs` (`graph/v2.rs`), `uses.rs` (`graph/uses.rs`), `rust.rs`, `python.rs`, `mod.rs` (`extractors/`) são de `crates/librarian-ingest/src/`. `main.rs` e `index.html` são do `crates/librarian-studio/`.

---

## Parada 1

### A. A chamada crua

**O que o extrator grava** (verificado, por leitura). Uma chamada é uma `Occurrence` (`mod.rs:17-29`) com seis campos: `kind`, `start`, `end` (bytes), `candidate` (o nome escrito), `label_end` e `rule`. Nenhum campo guarda o receptor, os argumentos ou um tipo da própria chamada.

| O quê | Rust | Python |
|---|---|---|
| Tipos de chamada (`call_kinds`) | `call_observed` e `method_call_observed` (`rust.rs:187-189`) | os mesmos (`python.rs:117-119`) |
| `call_observed` | `f()`, `T::f()`, `m::f()`. O trecho cobre **só o caminho chamado**, sem argumentos (`value.func.span()`, `rust.rs:957`) | um callee que não é atributo: `f()`, `x[0]()`. O trecho cobre só o callee (`func.range()`, `python.rs:1140`) |
| `method_call_observed` | `x.m(..)`. O trecho cobre a expressão inteira, com receptor e argumentos (`value.span()`, `rust.rs:1071`) | todo callee atributo: `x.m(..)`, `modulo.f(..)`, `Classe.m(..)`. O trecho é a chamada inteira (`node.range`, `python.rs:1132`) |
| Nome escrito (`candidate`) | o último segmento do caminho (`Ray::new` → `new`, `rust.rs:951-955`) ou o nome do método (`:1070`) | o atributo (`python.rs:1132`) ou o `Name` chamado (`:1136-1139`). Outro callee: nenhum nome |
| Tipo do receptor | outra ocorrência, `receiver_type`, com o nome do tipo como `candidate` e a regra que o deu em `rule` (`rust.rs:858-862`) | a mesma ocorrência, desde o perfil V9 (`python.rs:75`) |

**O que o grafo faz com ela** (`graph.rs:267-350`, verificado):
- **Nó:** id `{fonte}:occurrence:{kind}:{start}:{end}`. O rótulo é o texto dos bytes até `label_end` (`:321-325`), e o `type_rule` vem de `rule` (`:326`).
- **Escopo (quem chama):** o símbolo de menor trecho que contém a ocorrência (`:308-318`), ligado a ela por uma aresta com o `kind` da ocorrência e `resolution: "syntax_only"` (`:328-334`). No grafo `v2`, essa aresta vira `observes` (`v2.rs:97-103`).
- **Linha:** a `Evidence` da ocorrência: arquivo, hash da fonte, bytes, linhas e hash do trecho (`graph.rs:24-33`, `:242-256`).
- **Candidatos:** uma aresta `name_candidate` para **cada** símbolo com o mesmo nome, dos tipos que `candidate_kinds_for` admite, na mesma linguagem (`:290-298`, `:341-349`).
  - Tipos admitidos para chamadas: em Rust, `function`, `method`, `trait_method`, `variant` e `struct` (`rust.rs:164`); em Python, `function`, `method` e `class`, e `method_call_observed` também admite `class` (`python.rs:87`, `:100`).
  - Acima do teto (`max_candidates: Some(8)` no perfil atual, `mod.rs:463`, `:468-470`), nenhuma aresta é desenhada. O nó guarda só a contagem, em `ambiguous_candidates` (`graph.rs:335-339`).
  - As regras de nome por arquivo (`matched_only` e `file_scoped`) não valem para os tipos de chamada: as listas não os contêm (`rust.rs:191-197`; `python.rs:121-136`).

**O receptor não é um campo.** Ele é lido de novo do rótulo a cada pergunta: `call_parts` (`resolve.rs:2211-2224`) devolve o nome e o nome escrito antes dele (`""` sem receptor, `"()"` quando o receptor é resultado de chamada ou índice), e `written_receiver` (`:2226-2231`) devolve o receptor inteiro.

**O que a regra lê além do grafo, sem gravar** (verificado). `CallFacts` (`resolve.rs:42-79`) é relido dos bytes preservados da edition por arquivo, na hora da pergunta (`Catalog::call_syntax`, `lib.rs:540-560`; `call_facts` do Rust em `rust.rs:109-150` e do Python em `python.rs:139-…`). Ele traz:
- os escopos de nomes, com imports, nomes locais e todas as ligações de cada nome (2.2 e 2.4a);
- o tipo escrito de cada receptor;
- as classes (2.3a e 2.4b);
- os decoradores (2.3b);
- as chamadas que ligam um receptor e os retornos escritos (2.3c);
- os campos de outro arquivo (2.3c);
- os `cls`, os métodos (2.4b);
- os módulos, blocos e variáveis do Rust (2.4a).

Também vem do perfil o receptor próprio de cada linguagem: `self` nas duas (`rust.rs:206`, `python.rs:652`; lido em `lib.rs:544`).

### B. A regra, na ordem em que decide

**Pontos de entrada** (verificado; `git grep -nE "Resolver::new|\.resolve\(|answer\("`):
- `Resolver::new` monta os índices a partir das arestas e dos nós (`resolve.rs:458-571`): candidatos, `contains`, `declares`, `observes`, métodos, classes e funções por nome, `impl Trait for Type`, os itens Rust por escopo.
- `Resolver::resolve(caller, occurrence, chosen)` (`resolve.rs:2038-2150`) é **a regra**. Devolve uma `Resolution` (`:273-289`).
- `v2::answer` (`v2.rs:408-445`) chama `admitted` e `resolve` e traduz a `Resolution` para os campos da ligação. É a única tradução, e a usam os dois sentidos:
  - `Graph::calls_from` (`v2.rs:449-604`), por trás de `/api/calls` (`main.rs:137-146`);
  - `UseIndex::build` (`uses.rs:134-236`), por trás de `/api/used-by` (`main.rs:130-136`, `:297-308`).
- Leitores da regra fora das duas rotas, só citados:
  - `inventory.rs:506-529` chama `resolve` direto;
  - `envelope.rs:225-226` e `owner.rs:403-405` leem `calls_from` filtrado por `SETTLED`;
  - `instrument.rs:292` e `:304` (o Medir tudo do studio) leem `used_by` por nome e `calls_from`;
  - `resolve.rs:947` usa a própria regra na regra do retorno (2.3c).

**A ordem dentro de `resolve`** (verificado, por leitura; o passo de cada ramo vem do comentário "Step …" junto do código, que conta como "diz ser", e do nome do arquivo de teste que o fixa):

| # | Linhas | O que olha | O que decide | O que marca na ligação | Passo |
|---|---|---|---|---|---|
| 1 | `:2039` | arestas `name_candidate` da ocorrência (`all`) | — | — | G6 / 2.1 |
| 2 | `:2040`, `admit` `:1128-1132`, `form` `:1135-1150`, `form_reaches` `:1152-1162` | a forma escrita. `x.m()` cujo receptor é um módulo do projeto → só funções e classes desse arquivo. Receptor ligado a um import de fora → nada. `x.m()` → nunca uma função livre. `f()` → nunca um método, a não ser da classe em volta. Outras → tudo | quais candidatos ficam (`edges`) | nada direto: muda o conjunto e o `candidates` | 2.2 |
| 3 | `:2042-2048`, `chain` `:1479-1505` | só Python, só `method_call_observed`. A classe estática do receptor, por `python_receiver` (`:1241-1258`), nesta ordem: `super()`, `cls`, o tipo escrito (2.3a, mais retorno, campo e `@property` em `python_class`, `:801-815`) e o nome da classe (`Classe.m()`, 2.4d-0). Procura o nome na linearização C3 (`search`, `:1384-1413`) | se uma subclasse do projeto sobrescreve o método (`dispatch`, `:1536-1575`), a regra devolve `Overrides` (os corpos), ou `Chosen` se a pessoa escolheu um deles | `homonym`, com `basis: "type"` e `abstract_method` (`v2.rs:425-427`, `:438-443`) | 2.4b, 2.4c |
| 4 | `:2049-2052`, `in_chain` `:586-592`, `left_by_chain` `:1607-1625` | métodos candidatos fora da cadeia de bases | os candidatos fora saem. Sem nenhum e com nome escrito, devolve `OutOfChain` | sem ligação: `no_candidate_in_project` (`v2.rs:423`, `:431-434`) | 2.4b |
| 5 | `:2053-2055` | o `chosen` da pessoa, se está entre os candidatos que ficaram | `Linked(Chosen)` | `chosen`, `basis: "graph_edge"` | 2.1 |
| 6 | `:2058-2068`, `scoped` `:1634-1647` | só `call_observed`. O nome simples (Python, `python_name` `:1655-…`) ou o caminho (Rust, `rust_scoped` `:1787-…`), como o escopo de quem chama o liga: def do módulo, import, alias, `global`/`nonlocal`, bloco, `use`, glob | `Nothing` (import de fora, variável, builtin) → `NoCandidate`. `Target` → `unique`, se confirma a única candidata, ou então `import` ou `scope`. `Undecided` segue adiante | `unique`, `import` (`basis: "import"`) ou `scope` (`basis: "scope"`) (`How::basis`, `:262-269`) | 2.4a (e a correção de 24/11, `escopo_visivel_2_4a.rs`) |
| 7 | `:2070-2086` | uma só candidata admitida | receptor de módulo com outras de mesmo nome → `import`. Se o nome é único no projeto, a chamada é simples ou o receptor é módulo: o tipo escrito pode descartá-la (`type_rules_out`, `:699-733`) → `NoCandidate`. Se a cadeia achou outro método → `type`. Senão → `unique` | `import`; `unique` (`basis: "graph_edge"`); `type`; ou sem ligação | 2.1; descarte 2.2, ampliado em 2.3a e 2.3c; a cadeia, 2.4b |
| 8 | `:2088-2093` | `self.m()` (o receptor é exatamente `self`) e uma só candidata admitida no mesmo contêiner (impl, classe), que não seja `trait_method` | `Linked(Receiver)` | `receiver`, `basis: "graph_edge"` | 2.1 |
| 9 | `:2094-2103` | `self.m()` acima do teto: os métodos do nome no contêiner de quem chama | `Linked(Receiver)` se há só um | `receiver`, `candidate_edge: null` | 2.2 (em Python, 2.3a-0: só a mensagem do commit `8c689d4`, que é "diz ser") |
| 10 | `:2108-2110` | a cadeia achou os métodos (`Chain::Methods`) | `Linked(Type, achados)` | `type` | 2.3a, 2.4b |
| 11 | `:2111-2138`, `receiver_types` `:641-681` | o tipo escrito: `Self::f`, `T::f`, o `receiver_type` gravado, o retorno da chamada que liga o receptor (2.3c) e o alias (2.4a) | variante `T::V(..)` → `type` (2.3a). Métodos do impl do tipo → `type`. Padrão de um trait que o tipo implementa → `trait_default` | `type` ou `trait_default`, `basis: "type"`, e `type_rule` | 2.1; variante 2.3a; retorno 2.3c; alias 2.4a |
| 12 | `:2139-2142` | várias candidatas, todas de um mesmo trait | `Linked(Trait, todas)` | vira `homonym` em `calls_from` (linha 14 abaixo) | 2.1 |
| 13 | `:2143-2148` | nenhuma candidata admitida | acima do teto e sem candidata → `AboveLimit`; senão → `NoCandidate` | sem ligação | G6 / 2.1 |
| 14 | `:2149` | o que sobrou | `Homonym(admitidas)` | `homonym`, `basis: "graph_edge"` | 2.1 |

**Depois da regra, em `answer` e em `calls_from`** (verificado):
- **Um alvo só** (`v2.rs:419-421`): `resolution` = o `How`, `basis` = `How::basis`. A `candidate_edge` é a aresta admitida com aquele alvo, se existe. Não existe para `scope` fora das arestas, para `type`/`receiver` acima do teto e para um método herdado (`v2.rs:345-347`, diz ser).
- **Vários alvos** (`Trait`, `Type` com vários impls ou métodos, `TraitDefault` com vários) caem no ramo `_` (`v2.rs:428`). Saem como `homonym` sobre as **arestas admitidas**, e não sobre os alvos que a regra deu. Ver MAP-O17.
- **`relation`** (`Answer::relation`, `v2.rs:393-402`):
  - `decorates`, se a chamada está dentro de um decorador (`resolve.rs:613-617`; 2.3b);
  - `instantiates`, se o alvo único é `class`, `struct` ou `variant` (`resolve.rs:34`; 2.3a). Nesse caso `runs` traz o `__init__`, próprio ou herdado (`resolve.rs:1083-1104`; 2.3a e 2.4b);
  - `calls` nos outros casos.
- **`via`:** a chamada escrita no valor de um campo responde pelo método que declara o campo (`v2.rs:470-476`, `:506-513`; 2.3b).
- **`type_rule`:** só com `basis: "type"` (`v2.rs:404-406`, `resolve.rs:1010-1044`; 2.3c, 2.4b, 2.4d-0).
- **Seguir ou parar** (`v2.rs:567`): o caminho segue `runs` numa instanciação, nunca segue `decorates`, e segue o alvo de toda `resolution` diferente de `homonym`.
- **2.4d:** `UseIndex` inverte o mesmo `answer` (`uses.rs:181`), e não uma regra própria.

**O 2.4e não mexeu na regra** (verificado): `git show --stat aead3eb` só toca `tools/precisao/gabarito/` (11 arquivos), e o último commit em `resolve.rs` é o `665ff5b` do 2.4d (`git log -- crates/librarian-ingest/src/resolve.rs`). O gabarito do 2.4e fica para a parte 4 (Parada 3).

**O que cada passo acrescentou, pelos testes.** Primeira linha de cada arquivo (diz ser) e nomes dos testes, por `grep -nE '^\s*fn [a-z0-9_]+\(\)'`:

| Passo | Arquivo de teste | O que os nomes fixam |
|---|---|---|
| P1b | `tests/calls.rs` (2 testes; *correção da Parada 2:* dizia 3, mas o `#[test]` de `calls.rs:50` está dentro do texto de uma fixture, e a execução da seção H roda 2) | seguir só quando o nome aponta para uma declaração; busca sem testes |
| 2.1 | `tests/resolve.rs` (11) | uma resolução por teste: `unique`, `receiver` só em `self.m()`, `type`, `type` acima do teto, `trait_default`, `trait`, `homonym`, `chosen` antes de tudo. Também: `calls_from` reporta um alvo e mantém vários como homônimos; `type` é seguido e `homonym` não; o dono só usa as ligações decididas |
| 2.2 | `tests/resolve_forma.rs` (14) | método nunca chega a função livre; receptor de módulo do projeto; import de fora não chega a nada; nome local vence import; chamada simples nunca chega a método; o tipo escrito derruba o `unique`, mas não por `&`, `Box`, `Rc`, `Arc`, nem por um trait que o projeto implementa; `self.m()` acima do teto em Rust e em Python; `calls_from` reporta as admitidas |
| 2.3a | `tests/tipos_2_3a.rs` (11) | anotação, construtor, campo do `__init__` e `self` dão o tipo; classe pelos imports; classe sem base derruba a candidata; classe acima do teto pelo import; `Protocol` não dá ligação; instanciação como relação própria, que roda o `__init__`; struct de tupla e variante; a regra de cada `receiver_type`; perfis antigos sem nada disso |
| 2.3b | `tests/dono_2_3b.rs` (4) | chamada em decorador é `decorates` e não é seguida; o método responde pelos campos que declara; cada chamada contada uma vez; o texto do passo da cadeia |
| 2.3c | `tests/retorno_2_3c.rs` (12) | o retorno tipa o receptor no mesmo arquivo e entre arquivos; `Optional` e `self`; campo de outro arquivo; nada por tipo de fora, genérico ou sem anotação; candidatas que concordam ou discordam; um nível só; a escolha da pessoa não chega à chamada que liga; `let x = T::f(..)` pelo retorno de `f`; invólucros não decidem |
| 2.4a | `tests/escopo_2_4a.rs` (12; *correção da Parada 2:* dizia 14, mas `t_proprio` e `t_glob`, `:253-260`, estão dentro do texto de uma fixture) e `tests/escopo_visivel_2_4a.rs` (6) | def do módulo decide entre homônimos; o último def depois dos `@overload`; import e reexport; import de fora, variável e builtin descartam; o que o escopo não diz fica como antes; alias de import instancia e tipa; `scope` com `basis` próprio; item do módulo ou do bloco, `use`, glob e caminho de módulo; alias por `use as` e `type`. Depois de 24/11: `global`, `nonlocal` e só definição visível |
| 2.4b | `tests/heranca_2_4b.rs` (7) | método herdado pela ordem C3; o que a cadeia não diz fica indeciso; classe fora da cadeia nunca é alvo; `super()` nunca chega à própria classe; método abstrato dá as sobrescritas; instanciação roda o `__init__` herdado; sobrescritas acima do teto podem ser escolhidas |
| 2.4c | `tests/despacho_2_4c.rs` (9) | método sobrescrito dá todos os corpos que podem rodar; sem sobrescrita, a ligação fica; `__new__` que constrói outra classe; nome privado; subclasse que liga o nome a outra coisa; sobrescrita vazia; a ligação nomeia o abstrato que saiu; os corpos podem ser escolhidos |
| 2.4d-0 | `tests/classe_nome_2_4d0.rs` (7) | `Classe.m()` pelo escopo; sobe a cadeia e nunca desce; variável ou duas classes possíveis não são a classe; nome que não é só método; base de fora para a busca; o retorno do método tipa; a regra também acima do teto |
| 2.4d | `tests/usos_2_4d.rs` (7) | toda ligação é um uso e todo uso é uma ligação, nos sintéticos Python e Rust; os tipos de uso ficam à parte; instanciação usa a classe e o `__init__`; o caminho sobe só pelos ligados e nunca por decorador; o teto corta o desenho; acima do teto é lacuna filtrada pela forma e pelo tipo |

### C. Os estados de uma ligação

**Onde cada valor é produzido e lido** (verificado):

| Camada | Campo | Valores | Produzido em | Lido em |
|---|---|---|---|---|
| Aresta do grafo | `resolution` (texto) | `structural` (`declares`, `contains`), `syntax_only` (quem chama → ocorrência), `name_candidate` | `graph.rs:90`, `:110`, `:332`, `:346` | `Resolver::new` lê as arestas por `kind`, não por esse campo (`resolve.rs:488-525`) |
| Nó da ocorrência | `ambiguous_candidates` | contagem acima do teto; ausente sem teto | `graph.rs:338` | `resolve.rs:2050`, `:2098`, `:2144`; `v2.rs:432-437`; `uses.rs:210` (pelo `unresolved`) |
| Nó `receiver_type` | `type_rule` | `parametro`, `anotacao`, `self`, `construtor`, `caminho`, `retorno`, `campo`, `item`, `variante` (`mod.rs:26-28`, diz ser; produtores em `rust.rs:724`, `:847-854`, `:1006`, `:1029`, `:1118-1123`, `python_types.rs:264`) | `graph.rs:326` | `resolve.rs:1014`, `:1043`, `:1262` |
| Regra | `Resolution` (enum) | `Linked(How, alvos)`, `Homonym`, `AboveLimit`, `NoCandidate`, `Overrides`, `OutOfChain` | `resolve.rs:2038-2150` | `v2.rs:418-429`; `inventory.rs:514-528` |
| Regra | `How` (enum) → texto | `chosen`, `unique`, `import`, `scope`, `receiver`, `type`, `trait_default`, `trait` | `resolve.rs:223-260` | `v2.rs:420` |
| Regra | `How::basis` | `graph_edge`, `type`, `import`, `scope` | `resolve.rs:262-269` | `v2.rs:420` |
| Regra | `SETTLED` | `unique`, `import`, `scope`, `receiver`, `type`, `trait_default` (sem `chosen` nem `trait`) | `resolve.rs:30` | `envelope.rs:226`, `owner.rs:405` |
| `/api/calls`, `CallLink` | `resolution` | `chosen`, `unique`, `import`, `scope`, `receiver`, `type`, `trait_default`, `homonym`. **`trait` nunca sai**: o `Trait` sempre tem mais de um alvo (`resolve.rs:2140`) e cai em `homonym` (`v2.rs:428`) | `v2.rs:418-429`, `:588` | `index.html:7527` (`dashed`), `:7554`, `:7604`, `:7665`, `:7887`, `:7929` |
| `/api/calls` | `basis` | `graph_edge`, `type`, `import`, `scope` | `v2.rs:420-428` | `index.html:7703`, `:7929` |
| `/api/calls` | `relation` | `calls`, `instantiates`, `decorates` | `v2.rs:393-402` | `v2.rs:566-567`; `owner.rs:410` |
| `/api/calls` | `type_rule` | os 9 acima e mais `propriedade`, `super`, `cls`, `classe` | `resolve.rs:1010-1044` | — (vai ao cliente) |
| `/api/calls` | `candidate_edge`, `candidates`, `runs`, `via`, `abstract_method` | id da aresta ou `null`; contagem; id | `v2.rs:581-597` | — |
| `/api/calls`, `UnresolvedCall` | `reason` | `above_candidate_limit`, `no_candidate_in_project`, `node_limit` | `v2.rs:431-434`, `:549` | `index.html:7703` (`node_limit`) |
| `/api/used-by`, `Use` | `use` | `linked` (a `resolution` não é `homonym`), `possible` (é `homonym`), `name` (aresta de nome de uma ocorrência que não é chamada) | `uses.rs:252`, `:273` | `index.html:7527-7530`, `:7683`, `:7767-7768`, `:7887` |
| `/api/used-by`, `Uses` | `counts` e listas à parte | `linked`, `possible_type`, `possible_homonym`, `name`; à parte `above_limit`, `abstract_calls`, `ruled_out` | `uses.rs:371-383`, `:205-232` | página |
| Página, só no cliente | `resolution` reescrita | `measured`: um `chosen` que veio do Medir tudo, e não da pessoa | `index.html:7604` | `index.html:7909-7910` |

**Como a página desenha hoje** (só citado, verificado na leitura do JS):
- `dashed = l => callsMode() ? l.resolution === 'homonym' : l.use === 'possible'` (`index.html:7527`); `named` = `use === 'name'`, pontilhada (`:7528`, CSS `:193-194`).
- Na vista dos usos, os `possible` por homônimo ficam fora do desenho (`:7530`, `:7683`).

**Tabela: cada valor no modelo do usuário.** Os quatro estados são os do pedido: **cheia** (afirmado, como um compilador, por tipo), **tracejada** ("pode ser A ou B"), **lacuna** (não se sabe) ou **não cabe**. A coluna "Hoje" diz como o código e a página tratam o valor (verificado). A coluna "No modelo" é **leitura minha**, com o motivo tirado do código.

| Valor | Hoje | No modelo | Motivo pelo código |
|---|---|---|---|
| `unique` | cheia; seguida; em `SETTLED` | **não cabe** | Afirma por nome, não por tipo: "o nome escrito só combina com esta declaração, entre as que a forma alcança" (`resolve.rs:226-227`). O tipo só entra para descartar (`:2076`). Um alvo de fora do projeto não é candidato, porque só há arestas para símbolos do projeto (`graph.rs:290-298`). Na verdade, é "esta ou algo de fora" |
| `import` | cheia; seguida | **cheia** (por nome ligado, não por tipo) | O nome é o que o import liga, lido do arquivo (`resolve.rs:229-232`, `:1720-1733`, `:1866-1876`), como o compilador ou o interpretador resolve nomes |
| `scope` | cheia; seguida | **cheia** (por nome ligado) | A definição que o escopo de quem chama liga ao nome, só a visível (`resolve.rs:233-235`, `:1655-1717`, `:1787-1836`) |
| `receiver` | cheia; seguida | **cheia**, com ressalva | `self.m()` e a única candidata no mesmo contêiner (`resolve.rs:2088-2093`). É por contêiner, não por tipo: em Rust, um método do mesmo nome em outro `impl` do mesmo tipo não entra nessa conta. Em Python, a cadeia (passos 3 e 10) decide antes, quando lê a classe de `self`. Leitura minha |
| `type` (um alvo) | cheia; seguida | **cheia** | Método do tipo escrito do receptor ou achado na cadeia de bases (`resolve.rs:2080-2083`, `:2108-2110`, `:2124-2128`) |
| `trait_default` (um alvo) | cheia; seguida | **cheia** | O tipo não define o método; o padrão do trait que ele implementa (`resolve.rs:2129-2137`) |
| `chosen` | cheia; seguida; **fora** de `SETTLED` | **não cabe** | Não é certeza da regra: é a escolha da pessoa entre as candidatas (`resolve.rs:2044-2046`, `:2053-2055`). Ver D |
| `measured` (só na página) | cheia | **não cabe** | É um `chosen` montado pela página a partir das contagens do Medir tudo (`index.html:7551-7563`, `:7590`, `:7604`): origem na execução, não na regra |
| `homonym` com `basis: "type"` (corpos do despacho, `Overrides`) | tracejada; não seguida; na vista dos usos, `possible` pelo tipo, tracejada | **tracejada** | "Pode ser A ou B" por tipo: os corpos que podem rodar para o tipo estático e as subclasses do projeto (`resolve.rs:1536-1575`, `v2.rs:425-427`) |
| `homonym` vindo de `Trait`, de `Type` ou de `TraitDefault` com vários alvos | tracejada; não seguida | **tracejada**, com ressalva | A regra deu um conjunto. Mas a ligação lista as **admitidas**, que podem ser mais que esse conjunto (`v2.rs:428`), e marca `basis: "graph_edge"`. Ver MAP-O17 |
| `homonym` com `basis: "graph_edge"` (`Resolution::Homonym`) | tracejada na vista das chamadas; na vista dos usos, fora do desenho e só na lista | **não cabe**: é lacuna desenhada como tracejada | "Several candidate edges and nothing settles them" (`resolve.rs:275-276`). As candidatas são as de mesmo nome. Nada diz que o alvo certo está entre elas, nem que não é de fora do projeto. A página diz que "quase nunca são este ponto" (`index.html:1075`, diz ser) |
| `homonym` com uma só candidata | tracejada (texto `homonymOne`, `index.html:1259`) | **lacuna** | Restou uma admitida, mas a regra não a afirma (`resolve.rs:2075`: o nome não é único no projeto, a chamada não é simples e o receptor não é módulo) |
| sem ligação, `above_candidate_limit` (`AboveLimit`) | sem linha; listada em `unresolved`; na vista dos usos, à parte em `above_limit`, filtrada por `may_reach` | **lacuna** | Mais homônimos que o teto e nenhum tipo decide. O grafo só guarda a contagem (`graph.rs:335-339`, `resolve.rs:2143-2146`) |
| sem ligação, `no_candidate_in_project` por nenhum nome igual | sem linha; listada | **lacuna** | "Does not say the call is external" (`v2.rs:308-310`, diz ser) |
| sem ligação, `no_candidate_in_project` por descarte (`NoCandidate` do escopo ou do tipo, `OutOfChain`) | sem linha; listada com o **mesmo texto** do caso acima; na vista dos usos, em `ruled_out` | **não cabe**: é uma afirmação negativa ("não é nada do projeto") | O escopo (`resolve.rs:2061`), o tipo (`:2077`) ou a cadeia (`:2051`) descartam as candidatas. A ligação não distingue isso de "nenhum nome igual" (`v2.rs:431-434`). Ver MAP-O18 |
| `NoCandidate` numa chamada acima do teto | sem linha; listada como `above_candidate_limit` | **não cabe** (o mesmo descarte, com o nome errado) | `v2.rs:432`: com `ambiguous_candidates` e sem `OutOfChain`, o motivo é o teto, mesmo quando a regra descartou. Ver MAP-O18 |
| `node_limit` | sem linha; listada com a `resolution` que teria | **não cabe** | É corte do desenho, não estado da ligação (`v2.rs:541-557`) |
| `relation` `decorates` e `instantiates` | qualquer linha; `decorates` nunca seguida | **não cabe** (outro eixo) | Dizem o que a chamada faz, não quanto se sabe dela (`v2.rs:393-402`, `:567`) |
| uso `name` (vista dos usos) | pontilhada | **não cabe** | Não é chamada: o nome está escrito onde nada é chamado (`uses.rs:226-232`, `:264-282`) |
| `abstract_calls` (vista dos usos) | à parte, sem linha | **não cabe** | Chamada escrita contra um método abstrato; roda o corpo de uma subclasse (`uses.rs:206-208`) |
| `name_candidate`, `structural`, `syntax_only` (aresta do grafo) | — | **não cabe** (camada de baixo) | São arestas de navegação e de estrutura, "never compiler resolution" (`graph.rs:40`, diz ser); a ligação é derivada delas |

**As ligações pelo nome e por homônimo, hoje** (verificado):
- **Pelo nome** é `unique`, com `basis: "graph_edge"`. Hoje cai em **linha cheia**, seguida e em `SETTLED`, sem distinção de `type` ou `scope` na linha (`index.html:7527` só olha `homonym`). Também é pelo nome o uso `name`, pontilhado, e toda candidata de `Homonym`.
- **Por homônimo** é `homonym` com `basis: "graph_edge"`. Hoje cai em **linha tracejada** na vista das chamadas, e o caminho para ali até a pessoa escolher. Na vista dos usos fica **fora do desenho** (`index.html:7683`), contado em `possible_homonym` (`uses.rs:378`).
- O `homonym` com `basis: "type"` (o despacho) usa a mesma linha tracejada na vista das chamadas. A página o separa só no texto (`index.html:7929`).

### D. Origem, certeza, evidência e validação humana

**Campos de cada ligação de `/api/calls`** (`CallLink`, `v2.rs:330-374`; *correção da Parada 2:* dizia `:330-372`; verificado) e o que cada um diz:

| Pergunta | Campo separado? | O que o código guarda |
|---|---|---|
| **Como foi descoberta** (origem) | em parte | `resolution` diz a regra que decidiu (`unique`, `scope`…). `basis` diz de onde veio: só o grafo, o tipo escrito, o import ou o escopo (`resolve.rs:262-269`). `type_rule` diz qual regra deu o tipo (`resolve.rs:1010-1044`). Mas `resolution` também carrega a certeza (abaixo), e com `chosen` perde a regra (abaixo) |
| **Quanto se pode afirmar** (certeza) | **não** em `/api/calls` | Não há campo próprio. A certeza é deduzida do valor de `resolution`: pertence a `SETTLED` (`resolve.rs:30`) ou é `homonym`. E, junto, de `candidates`. Em `/api/used-by` existe o campo `use` (`linked`/`possible`/`name`, `uses.rs:63-66`), o mais perto de um campo de certeza, mas ele é calculado só a partir de `resolution == "homonym"` (`uses.rs:263`) |
| **O que a sustenta** (evidência) | sim, mas só o lugar da chamada | `evidence` é sempre o trecho onde a chamada está escrita. As arestas `observes` e `name_candidate` levam os mesmos bytes (`graph.rs:333`, `:347`), e `calls_from` usa uma ou outra (`v2.rs:539`). `candidate_edge` aponta para a aresta de nome. **Nenhum campo guarda o que sustenta a decisão**: a linha do import, a definição do escopo, a anotação de tipo, as bases da classe. O nó `receiver_type` tem a própria evidência no grafo, mas a ligação só leva o texto `type_rule` |
| **Uma pessoa a validou?** | **não** | Nenhum campo. O que mais se aproxima, sem ser validação, é o `chosen` (abaixo) |

**`chosen`** (verificado):
- É entrada de cada pedido, e não estado guardado. A página o mantém em memória (`pathChosen`, `index.html:7482`, `:7921`) e o envia em `/api/calls` (`main.rs:142`, `index.html:7590`).
- Só é gravado quando a pessoa faz as perguntas do caminho: `path.chosen` da pergunta (`store.rs:489`).
- Quando vale, `resolve` devolve `Chosen` antes do escopo, do tipo e do resto (`resolve.rs:2053-2055`). A ligação diz `chosen`, e **o que a regra diria sem a escolha não fica registrado**.
- A página também envia como `chosen` as escolhas do Medir tudo (`index.html:7590`) e as rebatiza de `measured` no cliente (`:7604`).
- `chosen` fica fora de `SETTLED`, por isso o envelope e o dono não o usam (`envelope.rs:226`, `owner.rs:405`).

**A validação humana (acervo, fatos) toca uma ligação?** (verificado, com o que o método não pega)
- **A edition (o acervo imutável):** não guarda ligação nenhuma. O studio grava o grafo `v2` em `graph-v2.json` (`store.rs:349-351`), que tem só nós e arestas `declares`/`contains`/`observes`/`name_candidate` (`v2.rs:49-57`). As ligações são **recalculadas a cada pedido**: `calls_from` cria um `Resolver` por chamada (`v2.rs:454`). O `UseIndex` fica só em memória no studio, um por run (`main.rs:297-308`). Ver MAP-O16.
- **Os fatos (L4):** são afirmações de uma resposta do modelo sobre linhas de texto, aprovadas por uma pessoa com nota (`facts.rs:1-6`, diz ser; `AUTHORSHIP` "revisão humana", `:21`).
  - `grep -nE "resolution|calls_from|used_by|candidate_edge|occurrence" crates/librarian-studio/src/facts.rs` só acha as `resolutions` das **claims** (`facts.rs:34`, `:68-69`, `:236-245`), que são outro tipo: `librarian_llm_engine::claims::Resolution` (`claims/resolve.rs:8-16`: citação, intervalo). Nenhum campo de ligação.
  - O único contato é indireto, e só "diz ser": os fatos vão para `evidence.reviewed_facts` num passo da cadeia (`facts.rs:5-6`), cujos alvos vêm de ligações em `SETTLED` (`owner.rs:403-405`). O fato não muda a ligação.
- **A verdade validada das fixtures** (`tools/precisao/fixtures/*.json`): nenhum crate a lê. `git grep -nE "precisao/fixtures|amostra-(hc|rt|usos)|estrato-unique|fatos\.json" -- crates ':!*.md'` está vazio. Quem a lê é `tools/precisao/portao.js` e `tools/precisao/gabarito/medir.js`, além dos scripts de relatório em `docs/reports/fase-2/dados/` (`git grep -l`). A verdade humana mede a regra **de fora**, sem voltar a nenhum campo da ligação. O que o método não pega: uma leitura por caminho montado em tempo de execução, num crate. Não achei `fixtures` em nenhum `fs::read` do ingest nem do studio por esse grep.

---

## Observações novas (Parada 1)

- **MAP-O16.** As ligações **não são guardadas**. O grafo gravado (`graph-v2.json`, `store.rs:349-351`) só tem as arestas de nome (`name_candidate`). Cada ligação é decidida de novo a cada `/api/calls` (`v2.rs:454`; *precisão da Parada 2:* um `Resolver` por pedido, dentro de `calls_from`, e não um por chamada escrita) e na primeira `/api/used-by` de um run (cache em memória, `main.rs:297-308`). A regra também lê, a cada vez, os bytes da edition (`lib.rs:540-560`). Por isso a "ligação guardada no grafo" do ponto final da parte 3 é, no código, uma resposta calculada sobre o grafo.
- **MAP-O17.** Uma `Resolution::Linked` com **vários alvos** (`Trait`, `Type` com vários métodos ou impls, `TraitDefault` com vários) sai em `/api/calls` como `homonym`, com `basis: "graph_edge"`, sobre as **arestas admitidas** e não sobre os alvos da regra (`v2.rs:419-428`). Acima do teto, não há arestas: a ligação some e vira `above_candidate_limit` (`v2.rs:431-433`). O inventário trata o mesmo caso de outro jeito: segue todos os alvos (`inventory.rs:514-523`). *Leitura minha:* dois leitores da mesma regra dão respostas diferentes para o mesmo `Linked`.
- **MAP-O18.** Três casos diferentes caem no mesmo motivo `no_candidate_in_project` (`v2.rs:423`, `:431-434`):
  - nenhum nome igual no projeto;
  - descarte pelo escopo ou pelo tipo (`NoCandidate`);
  - fora da cadeia (`OutOfChain`).

  E um `NoCandidate` numa chamada acima do teto sai como `above_candidate_limit` (`v2.rs:432`). Na vista dos usos, esse caso entra em `above_limit`, filtrado por `may_reach` (`uses.rs:210-218`). `may_reach` olha a forma e o tipo, mas não o escopo (`resolve.rs:1168-1170`). *Leitura minha, a conferir na Parada 2:* uma chamada que o escopo diz ser de fora (import externo) e que tem mais de 8 homônimos pode aparecer como "pode usar" na vista dos usos.
- **MAP-O19.** Origem e certeza vêm misturadas no mesmo campo `resolution`. A certeza não tem campo próprio em `/api/calls` (seção D). A `evidence` é sempre o lugar da chamada, nunca o que sustenta a decisão.
- **MAP-O20.** `chosen` substitui o estado da regra sem guardar o que ela diria (`resolve.rs:2053-2055`). A página manda as contagens do Medir tudo pelo mesmo campo e as rebatiza de `measured` só no cliente (`index.html:7590`, `:7604`).
- **MAP-O21.** Homônimos de nome, com três sentidos:
  - `Resolution`: a da regra (`resolve.rs:273`) e a das claims (`librarian-llm-engine/src/claims/resolve.rs:8`, importada em `facts.rs:11`);
  - o campo de texto `resolution` das arestas do grafo (`structural`/`syntax_only`/`name_candidate`, `graph.rs:40-41`);
  - o campo `resolution` das ligações (`v2.rs:344`).
- **MAP-O22.** Comentários que ficaram para trás do código (diz ser × faz):
  - a precedência do cabeçalho, "chosen > unique > import > receiver > type > trait_default > trait > homonym" (`resolve.rs:18-21`), é do 2.2. Não fala da cadeia (2.4b, decide antes de `chosen`, `:2042-2052`) nem do escopo (2.4a, decide antes do `unique` de uma aresta só, `:2058-2068`);
  - o comentário de `CallLink.resolution` lista "unique, receiver, type, trait_default, chosen or homonym" (`v2.rs:342-343`), sem `import` nem `scope`;
  - o de `basis`, "`graph_edge` … or `type`" (`v2.rs:349-350`), sem `import` nem `scope`.
- **MAP-O23.** O 2.4e não mudou a regra: `aead3eb` só toca `tools/precisao/gabarito/`, e o último commit em `resolve.rs` é o do 2.4d (`665ff5b`).

## Perguntas (Parada 1)

- **MAP-Q9.** No modelo de comparação, "linha cheia: por tipo quando dá para afirmar" inclui as afirmações **por nome ligado** (`import`, `scope`), que o compilador também faz sem tipo? E o `unique`, que afirma por ser o único nome igual **dentro do projeto**, sem olhar o que há fora: ele entra como cheia, tracejada ("este ou algo de fora") ou lacuna? A tabela C marca `import` e `scope` como cheia e `unique` como "não cabe", à espera dessa resposta.
- **MAP-Q10.** "Acervo" na pergunta D: é a edition (o "acervo imutável" do ingest) ou a coleção de fatos do studio (`facts.jsonl`, "project's collection", `facts.rs:4`)? A seção D responde pelos dois.

---

## Parada 2

**Base e escrita.** `main` em `be7d546`, com a worktree limpa no início (`git status --porcelain -uall` vazio). A `.studio/` continua vazia (`ls -la .studio` lista só `.` e `..`) e não foi tocada. Não rodei o portão, o gabarito nem binários que gravam. Os únicos comandos que executaram código do repositório foram os testes da seção H e `sourceHash` de `tools/precisao/lib.js:173` (só lê arquivos e o `git ls-files`). Os scripts e as saídas desta parada estão no scratchpad da sessão, fora do repositório.

**Fontes a mais, além das da Parada 1.** As quatro fixtures validadas (`tools/precisao/fixtures/amostra-hc.json`, `amostra-rt.json`, `estrato-unique.json`, `amostra-usos.json`) e a linha de base do portão, `tools/precisao/linha-de-base.json` (o JSON que o portão grava e compara, `portao.js:5-6`; tratado aqui como JSON de configuração). A pasta `docs/reports/fase-2/dados/2.4e/` só tem `tabelas.md` (`ls -la`), que não foi aberto.

### E. Links falsos pela verdade validada

**Que estado é "hoje"** (verificado, com o que não foi conferido):
- **Amostras HC e RT:** o campo `resposta` de cada id em `linha-de-base.json` (`projetos.<hc|rt>.amostra`). São os runs `20261125-210527` (HC) e `20261125-210849` (RT), gravados pelo portão do 2.4d no commit `6254641`.
  - O código da regra não mudou depois: o último commit em `resolve.rs` é o `665ff5b`, ancestral do `6254641` (`git merge-base --is-ancestor 665ff5b 6254641`).
  - Nada que é compilado mudou depois: `git diff --stat 6254641 HEAD -- crates Cargo.toml Cargo.lock ':!*.md'` está vazio.
  - **Verificado por execução na Parada 2-bis** (subseção no fim desta seção E): os runs gerados de novo dão as mesmas 241 respostas. *Texto da Parada 2, mantido como registro:* "não conferido por execução: não regenerei as respostas (`.studio` vazia, MAP-O1). Leitura minha: com o mesmo código e os mesmos projetos nos mesmos commits, a resposta é a mesma."
- **Estrato `unique` (UQ):** o `linha_de_base` gravado na própria fixture, com `fontes: 3aeb2fa7…` e os runs `20261125-234413`/`-234657`. O hash das fontes de hoje dá o mesmo: `node -e 'console.log(require("./tools/precisao/lib").sourceHash())'` → `3aeb2fa77adc9ec4…`, 250 arquivos.
- **Amostra de usos:** o resultado gravado em `linha-de-base.json`, `projetos.<p>.usos.amostra`, montado por `usesSample` (`portao.js:185-211`).

**Método.** O script `e-verdade.js` (scratchpad) roda com `node e-verdade.js <raiz do repositório>` e só lê os JSON acima. A regra de julgamento é minha, escrita no script:
- **linha cheia** (os estados de `LINKED`, `portao.js:20`): é **verdadeira** quando o único alvo é o `verdade.alvo` validado; é **falsa** quando a verdade é de fora do projeto (`externo`), quando é `ambigua`, ou quando o alvo é outro;
- **tracejada** (`homonym`): o alvo validado está entre as candidatas; ou todas as candidatas são falsas; ou a verdade é ambígua;
- **sem linha**: é lacuna certa quando a verdade é de fora; é lacuna com o alvo no projeto quando ele estava lá.

**Conferência cruzada:** os totais batem com a classificação que o próprio portão gravou na linha de base (`amostra_contagem`, função `classify`, `portao.js:34-57`):
- HC: 96 corretas, 1 `link_falso`, 15 `candidatos_falsos`, 8 ambíguas, 51 sem candidato;
- RT: 44 corretas, 0 `link_falso`, 1 `candidatos_falsos`, 10 ambíguas, 15 sem candidato.

**Contagem por estado de hoje** (verificado, pela saída do script):

| Estado (tabela C) | Python, HC (171) | Rust, RT (70) | Python, UQ (15) | Rust, UQ (15) |
|---|---|---|---|---|
| `unique` | 38 verdadeiras, **1 falsa** | 8 verdadeiras | 14 verdadeiras, **1 falsa** | 13 verdadeiras, **2 falsas** |
| `import` | 9 verdadeiras | — | — | — |
| `scope` | 5 verdadeiras | 2 verdadeiras | — | — |
| `receiver` | 6 verdadeiras | 1 verdadeira | — | — |
| `type` (um alvo) | 38 verdadeiras | 28 verdadeiras | — | — |
| `trait_default` (um alvo) | — | 5 verdadeiras | — | — |
| `chosen` | não medido: o portão pede sempre `chosen: {}` (`lib.js:112`, `portao.js:95`) | idem | — | — |
| `homonym`, 2 ou mais candidatas | 1 com o alvo entre elas; **13 com todas falsas**; 5 com verdade ambígua | 9 com o alvo entre elas; **1 com todas falsas** | — | — |
| `homonym`, 1 candidata | 2 em que a única candidata é a certa; **1 falsa** | — | — | — |
| `above_candidate_limit` | 1 lacuna de fora (certa) | 1 lacuna com o alvo no projeto | — | — |
| `no_candidate_in_project` | 51 lacunas, todas de fora (certas) | 15 lacunas, todas de fora (certas) | — | — |

Notas da tabela:
- **Os falsos de linha cheia, hoje, são 4, todos `unique` e todos com a verdade de fora do projeto:**
  - HC-27: `c.getinfo(..)` → `FakeCurl.getinfo`, que é o dublê dos testes; o certo é o `pycurl`;
  - UQ-4: `c.setopt(..)` → `FakeCurl.setopt`, o mesmo caso;
  - UQ-19: `ordered.len()` → `impl Groups::len`; o certo é `Vec::len`;
  - UQ-23: `k.min(4. - k).clamp(0., 1.)` → `impl Color::clamp`; o certo é `f64::clamp`.

  Nenhum `import`, `scope`, `receiver`, `type` ou `trait_default` das amostras é falso.
- **`homonym`, por `basis`:** a `resposta` gravada não traz o `basis` (formato em `portao.js:252`). Pelos dados gravados, três delas eram com certeza `basis: "type"` (corpos do despacho): HC-140 (12 candidatas), HC-141 (30) e HC-142 (14), todas acima do teto de 8. *A Parada 2-bis separou todas por execução:* são 4 com `type` e 28 com `graph_edge` (abaixo).
  - Por quê, pelo código: acima do teto não há arestas (`graph.rs:335-339`); o ramo `_` só lista arestas (`v2.rs:428`); e só `Overrides` conta os alvos em vez das arestas (`v2.rs:425-427`, `:437`).
  - As outras 29 não dava para separar só pelos dados gravados. Separadas na Parada 2-bis.
- **`homonym` de 1 candidata:** HC-108 e HC-109 (a única candidata é a certa); HC-66, `self.client.login(..)` → `django.test.Client.login`, de fora.
- **Os tracejados com todas as candidatas falsas** (HC 14, contando a de 1 candidata, e RT 1) têm sempre a verdade de fora (`dict.get`, `QuerySet.get`, `OutputWrapper.write`, `Vec::into_iter`…). Ex.: HC-40 a HC-50 e RT-28.

**A vista dos usos** (`amostra-usos.json` e o resultado gravado; verificado pela saída do mesmo comando, no scratchpad):

| | HC (18 símbolos) | RT (10 símbolos) |
|---|---|---|
| uso ligado certo | 43 | 39 |
| **uso ligado falso** | **4** (UB-10 `Ping.get_body`) | **14** (UB-15 `impl Groups::len`, 4; UB-23 `impl Sorteio::next`, 10) |
| uso possível certo | 5 | 1 |
| **uso possível falso** | **2** (UB-6, UB-7) | **9** (UB-13, 1; UB-25, 8) |
| na lacuna (`above_limit`) e na verdade | 1 (UB-1) | 0 |
| uso da verdade que não aparece | 0 | 2 (UB-15) |
| pares homônimo × candidata (PH): pode rodar | 0 de 20 | 4 de 20 |

O resultado gravado não diz de qual estado vêm os usos ligados falsos: `usesSample` só guarda o lugar da chamada (`portao.js:198-199`). *Leitura da Parada 2:* `Groups::len` e `Sorteio::next` têm o mesmo padrão da UQ-19, um `unique` sem o tipo escrito do receptor. **Confirmado por execução na Parada 2-bis:** os 18 são `unique`, com `basis: "graph_edge"` e 1 candidata (abaixo).

**A fixture guarda o estado da época?** (verificado)
- Sim, e só o da época. O `linha_de_base` de cada chamada (`resolution`, `candidates`, `alvos`) é escrito quando a amostra é sorteada (`tools/precisao/amostra.js:54`). Nos estratos escolhidos à mão, é o estado do passo em que entraram (diz ser: campo `selecao` de `amostra-hc.json` e de `amostra-rt.json`).
- O portão não o lê: `git grep -n linha_de_base -- tools/precisao/*.js tools/precisao/gabarito/*.js` só acha `amostra.js:54`. O portão compara a resposta do run novo com a `verdade` (`portao.js:251`).
- **Mudou no código atual:** em **143 das 241** chamadas de HC e RT, a resposta de hoje é diferente do `linha_de_base`.
- Pela mesma regra de julgamento, a época tinha estes links falsos de linha cheia:
  - HC: 30 (25 `unique` de fora; 2 `receiver` de fora; 2 `receiver` com outro alvo; 1 `receiver` com verdade ambígua);
  - RT: 13 (8 `unique` de fora; 4 `receiver` com outro alvo; 1 `receiver` de fora).

  Hoje são 1 e 0. Comando: o `node -e` que lê `amostra-<p>.json`, no scratchpad.
- **A verdade** (`alvo`, `externo`, `ambigua`, `relacao`, `executor`) fala do projeto no commit fixo (`fonte`: Healthchecks `6c05a7a`, renderer `e38254b`) e de convenções da medição, por exemplo "dublê de teste não conta como alvo de código de produção (DEC-33)" na justificativa da HC-27. Ela não depende do código do Librarian. Só muda se mudar o projeto ou a convenção.
- Datas em `verdade_validada_por`, na HC: 18/11 (60), 19/11 (16), 20/11 (36), 24/11 (11), 25/11 (48).

**MAP-O18 conferida com os dados, em três partes:**
1. **O mesmo motivo para casos diferentes:** confirmado.
   - Das 51 chamadas da HC que hoje são `no_candidate_in_project`, 28 tinham candidata na época (24 `unique`, 3 `homonym`, 1 `above_candidate_limit`), e 23 não tinham. No RT, são 8 e 7.
   - Todas saem hoje com a mesma resposta, `no_candidate_in_project (0)`: o motivo e a contagem 0 são fixos (`v2.rs:433`), mesmo quando o grafo tem a aresta de nome que a regra descartou.
   - Nas 66, a verdade é de fora: nas amostras, a afirmação negativa está sempre certa.
2. **Descarte acima do teto contado como teto:** sem caso nos dados.
   - O único `above_candidate_limit` de verdade de fora é a HC-169, `Thread.__init__(self)`, que hoje é `above_candidate_limit (30)`.
   - Pelo código, ela é `AboveLimit` (`resolve.rs:2145`), e não `NoCandidate`: é método, e o passo do escopo só vale para `call_observed` (`scoped`, `resolve.rs:1635-1637`).
   - *Leitura minha:* a forma sabe que o receptor é um import de fora (`Module::External`, `resolve.rs:769`; `Form::Outside`, `:1140`). Mesmo assim, a ligação diz "acima do teto", e não "de fora".
3. **O falso "pode usar":** não aparece nos dados. *Na Parada 2-bis, também não nos runs inteiros* (abaixo).
   - Para a HC-169, `may_reach` dá falso, porque `Form::Outside` não alcança nada (`resolve.rs:1157`, `:1168-1170`). Ela não entra em `above_limit` (`uses.rs:210-215`).
   - O caso que sobra, só pelo código: uma chamada simples `f()`. O escopo diz "de fora" (`Scoped::Nothing` → `NoCandidate`, `resolve.rs:2061`); ela tem mais de 8 homônimos (`v2.rs:432` → `above_candidate_limit`); e `may_reach` usa só a forma (`Form::Plain`) e o tipo, nunca o escopo. Por isso ela entra na lacuna, que a página apresenta como "podem usar este ponto" (`index.html:1118`, diz ser).
   - As amostras não têm esse caso: a população do portão só tem `method_call_observed` (`lib.js:100`), e o único `above_limit` da amostra de usos é o da UB-1, que está na verdade.
   - Nenhum teste fixa esse caso (`git grep -n above_limit -- crates/librarian-ingest/tests` só acha `usos_2_4d.rs`, onde os casos de lacuna são `x.abrir()` e `Alvo::from`/`f64::from`).

### E, Parada 2-bis: o mesmo por execução

**Base e escrita.** `main` em `ecae7a4`, worktree limpa no início, fora o 01 já editado na Parada 0. A `.studio` foi conferida antes (MAP-O1, parte 1): `healthchecks-25597e41` (932 arquivos, `61cd25c1…`) e `renderer-9b78f3a9` (42, `612cdbf6…`), com o Healthchecks em `6c05a7a` e o ray tracer em `e38254b`, sem mudança local. Tudo o que subiu o Studio rodou sobre uma cópia temporária, por `withStudio` (`tools/precisao/lib.js:127-139`), que copia (`copyStudio`, `:73-77`) e recusa se o original mudar (`:135-136`). Saídas no scratchpad da sessão.

**1. O portão, só para medir** (verificado, por execução):

```
node tools/precisao/portao.js --studio .studio --base tools/precisao/linha-de-base.json --saida <scratchpad>/portao
```

- Ele compilou antes (`cargo build --release --locked -p librarian-studio -p librarian-ingest`, `lib.js:145-150`; só `librarian-evidence` e `librarian-studio` recompilaram, em 14 s).
- **Hash das fontes:** `3aeb2fa77adc9ec44716bc113822faa14395b71cb7f7217deb46b4e2b87c4726`, 250 arquivos. É o mesmo `3aeb2fa7…` da Parada 2 e do `fontes` da fixture UQ. Identidade gravada no binário: `recorded:ecae7a4…:false`.
- Runs novos: HC `20261126-074952`, RT `20261126-075336`. Exit 0, **APROVADO** contra a linha de base. `--gravar-base` não foi usado.
- `grafo_novo_igual_ao_de_referencia` é `false` nos dois projetos, como na linha de base: o grafo do run novo difere do run de 18/11 nos dois casos. A comparação vale contra a linha de base do 2.4d, e não contra o run de 18/11.

**2. A resposta de cada chamada contra a linha de base** (verificado, por um `node -e` que compara `resultado.json` com `linha-de-base.json`, id a id):

| O quê | HC | RT |
|---|---|---|
| `resposta` e `categoria` da amostra | **171 de 171 iguais** | **70 de 70 iguais** |
| amostra de usos, símbolo a símbolo (o objeto inteiro de `usesSample`) | **18 de 18 iguais** | **10 de 10 iguais** |
| invariante do `used_by` | igual (0 exceções) | igual (0 exceções) |
| população (`populacao`) | igual | igual |

Fatos F1–F26 e níveis N0–N8: iguais à linha de base.

**3. A sonda** (`sonda.js`, no scratchpad; `node sonda.js <repo> <saida>`). Ela usa o mesmo `withStudio` sobre outra cópia, gera runs novos (HC `20261126-075402`, RT `20261126-075517`) e pede `/api/calls` com `depth: 1` e `chosen: {}` ao dono de cada chamada. O dono é achado pela aresta `observes` da ocorrência, como o portão faz. A sonda guarda o que a resposta gravada não traz: `basis`, `type_rule` e os alvos de um `homonym`. Ela também confere o hash das fontes (`3aeb2fa7…`, 250) e o da `.studio` antes e depois (iguais).

**Como repetir a medição das 30 chamadas UQ** (o portão não as mede; só a sonda):
- **Onde está:** `C:\Users\beatl\AppData\Local\Temp\claude\09139e964f85837f\994f9711-cddd-4683-b9ea-bc7761f48d58\scratchpad\sonda.js`, com a saída em `sonda.json` na mesma pasta. O scratchpad é da sessão e pode ser apagado. O script não está no repositório.
- **Comando:** `node sonda.js <raiz do repositório> <saida.json>`, depois do portão, porque usa o `target/release` que ele compilou. A sonda não compila nada.
- **Sobre qual `.studio`:** a `<raiz>/.studio` é só lida. `withStudio` (`tools/precisao/lib.js:127-139`) copia os dois projetos (`healthchecks-25597e41`, `renderer-9b78f3a9`) para uma pasta nova em `os.tmpdir()` (`copyStudio`, `:73-77`). Sobe o `librarian_studio` com `LIBRARIAN_ROOT` nessa cópia (`startStudio`, `:47-70`) e confere o hash do original antes e depois (`:128`, `:135-136`).
- **O que ela chama, por projeto:**
  1. `POST /api/generate {key}`: um run novo, na cópia;
  2. lê o `graph-v2.json` desse run (`loadGraph`, `lib.js:80-89`) e indexa cada ocorrência de chamada pela chave `path@start_byte-end_byte` (`occKey`), com o dono vindo da aresta `observes`;
  3. para cada chamada de `tools/precisao/fixtures/estrato-unique.json` do projeto, acha a ocorrência pela `ocorrencia` da fixture e pede `POST /api/calls {key, run, node: <dono>, depth: 1, chosen: {}}`, uma vez por dono;
  4. dos `links` com aquela `occurrence`, junta os que saíram por `node_limit` em `unresolved`. Guarda `resolution`, `candidates`, `basis`, `type_rule` e os alvos como `rótulo (caminho:linha)`. Para os não ligados, guarda o `reason` de `unresolved`.
- **A comparação:** `resolution`, `candidates` e os alvos ordenados, contra o `linha_de_base` da fixture (gravado pelo `estrato-unique.js` em 25/11, fontes `3aeb2fa7…`).

- **Estrato `unique` (UQ):** as 30 chamadas dão hoje **o mesmo** `resolution`, `candidates` e alvos do `linha_de_base` da fixture. São todas `unique (1)`, com `basis: "graph_edge"`. Os falsos continuam os mesmos: UQ-4 (`FakeCurl.setopt`), UQ-19 (`impl Groups::len`) e UQ-23 (`impl Color::clamp`).
- **Os 4 falsos de linha cheia** (HC-27, UQ-4, UQ-19, UQ-23): `unique`, `basis: "graph_edge"`, 1 candidata, sem `type_rule`.
- **`homonym` da amostra, por `basis`** (a contagem da tabela de cima não muda, só se divide):

  | | `basis: "type"` (despacho, `Overrides`) | `basis: "graph_edge"` |
  |---|---|---|
  | HC, 2 ou mais, verdade ambígua | 4: HC-140 (12, `type_rule` `cls`), HC-141 (30, `propriedade`), HC-142 (14, `propriedade`), **HC-144** (2, `self`) | 1: HC-79 |
  | HC, 2 ou mais, alvo entre elas | — | 1: HC-59 |
  | HC, 2 ou mais, todas falsas | — | 13 (HC-37, 38, 40, 41, 44 a 47, 50, 57, 60, 111 e 112; verdade sempre de fora) |
  | HC, 1 candidata | — | 3: HC-108 e HC-109 (a certa), HC-66 (falsa, de fora) |
  | RT, 2 ou mais, alvo entre elas | — | 9: RT-10, 17 a 21, 26, 29 e 60 |
  | RT, 2 ou mais, todas falsas | — | 1: RT-28 (de fora) |

  Ou seja: os 4 de `type` têm todos verdade ambígua, como deve ser num despacho. **Todo tracejado com as candidatas todas falsas é `graph_edge`** (14 na HC, 1 no RT). A HC-144 (`homonym (2)`, abaixo do teto) é o quarto `type`, que os dados gravados não mostravam.

**4. Os 18 usos ligados falsos da vista dos usos** (item 7; verificado por execução). A sonda pediu `/api/used-by` dos três símbolos e, para cada uso `linked`, `/api/calls` no dono da chamada:

| Símbolo | Usos `linked` hoje | Falsos (os de `estrato-unique.js:22-28`) | Estado no `/api/used-by` | Estado no `/api/calls` |
|---|---|---|---|---|
| UB-10 `Ping.get_body` | 4 | 4: `message.get_body(..)` em `smtpd.py:37` e `:41`; `parsed.get_body(..)` em `views.py:800` e `:805` | `unique`, `graph_edge`, 1 candidata | o mesmo, alvo `Ping.get_body (hc/api/models.py:772)` |
| UB-15 `impl Groups::len` | 4 | 4: `face.len()` (`obj.rs:90`), `ordered.len()` (`camera.rs:129`), `"librarian-measure cpu_ms=".len()` (`count.rs:133` e `:151`) | `unique`, `graph_edge`, 1 | o mesmo, alvo `impl Groups::len (src/groups.rs:60)` |
| UB-23 `impl Sorteio::next` | 11 | 10: `fields.next()`, `f.next()`, `parts.next()`, `field.split('/').next()` (`obj.rs:19-81`) e `….split(' ').next()` (`count.rs:133`, `:151`). O 11º, `self.next()` em `src/bin/cena_video.rs:34`, é o certo | `unique`, `graph_edge`, 1 | o mesmo, alvo `impl Sorteio::next (src/bin/cena_video.rs:29)` |

Contagens de `/api/used-by`: `Ping.get_body` `linked 4`; `Groups::len` `linked 4, ruled_out 8`; `Sorteio::next` `linked 11, ruled_out 3`. Nenhum `above_limit` nem `possible`.

→ **A leitura da Parada 2 fica confirmada:** os 18 são `unique` pelo nome (`basis: "graph_edge"`, sem `type_rule`), com a verdade de fora (`Message.get_body` do Python, `Vec::len`/`str::len`, `Iterator::next`). O `basis` da `Use` é o mesmo da `CallLink`, o que mostra por execução que o `UseIndex` inverte o mesmo `answer` (seção B, 2.4d).

**5. MAP-O18, o falso "pode usar" por escopo acima do teto** (item 8). A sonda listou toda ocorrência `call_observed` com `ambiguous_candidates` nos runs novos. Para cada uma, guardou o que `/api/calls` responde e se ela está no `above_limit` de algum símbolo de mesmo nome curto.

| | Chamadas simples acima do teto | `/api/calls` responde | Na lacuna de algum símbolo |
|---|---:|---|---:|
| HC | 48 (`Command(..)` 42, `notify(..)` 6; todas de um segmento) | **todas `import`** | 0 |
| RT | 640 (todas com caminho `T::f`; nenhuma de um segmento) | `type` 475, `above_candidate_limit` 165 | 132 |

- **Os 132 do RT que entram na lacuna:** `Object::from` 82, `f64::from` 32, `Patterns::from` 17 e `crate::object::Object::from` 1. **Nenhum é o caso da MAP-O18.** Pelo código, o escopo do Rust só devolve `Scoped::Nothing` para um nome de **um segmento** (`resolve.rs:1793-1817`, `Nothing` em `:1817`). Um caminho com cabeça maiúscula (`Object::from`) vai para a regra do tipo (`:1821-1823` → `Undecided`). Um caminho de cabeça minúscula que não é módulo do projeto (`f64::from`) também fica `Undecided` (`:1824-1835`).
  - `f64::from` na lacuna dos `from` do projeto é o limite do primitivo já registrado na seção F (`resolve.rs:1166-1167`; `usos_2_4d.rs:300-305`).
  - `Object::from` e `Patterns::from` saem `above_candidate_limit` com um tipo escrito. *Leitura minha:* é o caso acima do teto da MAP-O17 (`type` com vários corpos, sem arestas → `above_candidate_limit`), agora num run real.
- **Os 165 `above_candidate_limit` que não entram na lacuna** são caminhos de fora (`Vec::new`, `String::from`, `Mutex::new`, `Sha256::new`, `Option::from`, `png::Encoder::new`…). `may_reach` os tira pelo tipo.
- **Python:** as 48 chamadas simples acima do teto são decididas pelo import. Nenhuma chega ao `Scoped::Nothing`.

→ **Não há caso real da MAP-O18 nos runs** de hoje, nem no Python nem no Rust. O caso continua só pelo código.

**6. A `.studio` original depois de tudo** (item 9; verificado): `hashProject` dá de novo `61cd25c1…` (932) e `612cdbf6…` (42), nos registros do portão (`studio_original.inalterado: true`), no fim da sonda e numa conferência separada no fim da parada. Ver MAP-O33 sobre as cópias temporárias.

### F. Links falsos conhecidos no código

**Guardas que existem** (verificado, por leitura; o passo vem do comentário junto, que é "diz ser"):

| Caso | Onde o código trata | Teste que fixa |
|---|---|---|
| **Método religado** (a classe liga o nome também a outra coisa: `tarefa_cn = solta_tarefa_cn` depois do `def`, `sinteticos/py/pkg/classe_nome.py:106-110`) | `ClassFacts.names` (`resolve.rs:112-113`). Na busca pela cadeia, o nome ligado a outra coisa deixa indeciso (`:1408`). Por `Classe.m()`, o achado numa classe que religa o nome volta ao que valia antes (`:1488`, 2.4d-0). No despacho, uma subclasse que liga o nome esconde os corpos (`:1559`) | `classe_nome_2_4d0.rs:155` (`cn_r_religada`, indeciso com 2 candidatas); `despacho_2_4c.rs:150` (subclasse que liga o nome, sem ligação única) |
| Atributo gravado por `self`/`cls`, `__getattr__`, `Protocol`, metaclasse ou decorador de classe | `ClassFacts` (`resolve.rs:92-116`); busca (`:1399-1401`, `:1405-1410`); `opens` (`:1471-1475`); `type_rules_out` não descarta por classe com base, `__getattr__` ou `Protocol` (`:703-708`) | `tipos_2_3a.rs:418` (o `Protocol` fica `homonym`, 2 candidatas); `tipos_2_3a.rs:393`; `heranca_2_4b.rs:115` |
| Nome privado `__m` | não é sobrescrito (`resolve.rs:1538-1540`) | `despacho_2_4c.rs:144` |
| `super()` não chega à própria classe | busca depois da classe (`resolve.rs:1384-1413`, `after`) | `heranca_2_4b.rs:152` |
| Receptor ligado a um import de fora | a forma não alcança nada (`resolve.rs:1140`, `:1157`) | `resolve_forma.rs:366` (`time.time()`, `auth.login(1)`, `TimestampSigner().sign(1)`) |
| Chamada simples a um import de fora, variável ou builtin | `Scoped::Nothing` → sem ligação (`resolve.rs:2061`) | `escopo_2_4a.rs:469` (`join` de `os.path`, `len`) |
| Referência, `Box`, `Rc`, `Arc`, ou um trait que o projeto implementa para o tipo | **não** descartam: a guarda é contra o descarte errado, não contra o link falso (`WRAPPERS`, `resolve.rs:38`; `:736-738`) | `resolve_forma.rs:421-445` |

**Casos conhecidos sem guarda** (verificado, por leitura e busca):
- **O dublê `FakeCurl`** (HC-27, UQ-4). A regra não conhece código de teste: `git grep -n is_test -- crates/librarian-ingest/src/resolve.rs crates/librarian-ingest/src/graph.rs crates/librarian-ingest/src/graph/` está vazio. O `is_test` do extrator (`python.rs:794`, `rust.rs:245`) só é lido pela busca e pelo envelope (`search.rs:191`, `:249`; `envelope.rs:192`, `:286`, `:414`).
  - Em Python, o retorno de uma chamada de fora (`c = pycurl.Curl()`) não entra no descarte: `type_rules_out` só olha o retorno fora do Python (`resolve.rs:713-722`).
  - O teste mais perto fixa o efeito com duas candidatas: `retorno_2_3c.rs:443` (`d.fechar()` sem tipo de fora fica `homonym`). Com uma candidata só, o resultado é `unique`. Nenhum teste fixa o caso de uma só.
- **`f64::clamp` → `Color::clamp`** (UQ-23) e **`Vec::len` → `Groups::len`** (UQ-19). Sem tipo escrito do receptor, nada descarta a única candidata. O tipo de fora só descarta um método inerente de um tipo do projeto, e só quando o tipo é conhecido (`outside_rules_out`, `resolve.rs:735-740`).
  - Comentário que diz o limite: `may_reach` "rules out neither by a primitive (`f64::from`) nor by a type that implements the same trait" (`resolve.rs:1166-1167`).
  - Testes que fixam o resultado como **esperado**:
    - `usos_2_4d.rs:300-305`: `f64::from(1u8)` fica na lacuna "pode usar" de `Alvo::from` e de `Outro::from` ("known limit", `:302`);
    - `retorno_2_3c.rs:512-520`: `p.take()` com `Option<Self>` fica `homonym` entre `Nota::take` e `Pedido::take`, e o comentário diz que roda o `Option::take` (`:514`). Todas as candidatas tracejadas são falsas.
- **Argumentos genéricos não são comparados**: `m.det()` com `m: &M<3>` dá `type` com `M<2>::det` e `M<3>::det` (`tests/resolve.rs:323-324`). O teste fixa `M<2>::det` como alvo.
- Comentários que avisam do não saber: `UnresolvedCall.reason` "does not say the call is external" (`v2.rs:308-310`); o `CallLink.candidate_edge` nulo (`v2.rs:345-347`).

**O que o método não pega:** a busca (`git grep -nIiE "FakeCurl|clamp|religad|rebound|rebind|false link|link_falso|dubl|test double|known false|Vec::len|primitive|f64::from" -- crates tools/precisao/sinteticos ':!*.md' ':!*.json' ':!*.jsonl' ':!*.html'`) acha só palavras escritas. Uma guarda sem comentário com essas palavras só foi achada pela leitura de `resolve.rs`, nos trechos citados.

### G. Como a ligação chega ao Studio

**`/api/calls`** (`POST`, `main.rs:137-146`, verificado):
- **Entrada:** `key`, `run`, `node`, `depth` e `chosen`, um mapa de ocorrência para homônimo escolhido (`:139-142`).
- **O que roda a cada pedido:**
  - a edition é aberta e relida (`Catalog::load(..).call_syntax()`, `:143`);
  - o grafo vem de um cache de **um** par (key, run) (`graph`, `main.rs:283-294`, que lê o `graph-v2.json` gravado);
  - `calls_from` cria **um** `Resolver` por pedido (`v2.rs:454`) e decide cada chamada ali (`v2.rs:520`);
  - o teto de nós é `USED_BY_NODES = 120` (`main.rs:280`).
- **Resposta, `Calls`** (`v2.rs:284-295`): `seed`, `depth`, `max_nodes`, `truncated`, `nodes`, `links` e `unresolved`. Cada nó (`UsedNode`, `v2.rs:154-165`) traz `id`, `label`, `kind`, `language`, `layer`, `container`, `path` e `line`.
- **Por ligação, `CallLink`** (`v2.rs:330-374`): `from`, `to`, `occurrence`, `occurrence_kind`, `text`, `candidates`, `resolution`, `candidate_edge`, `basis`, `relation` e `evidence`. Opcionais: `runs`, `via`, `type_rule` e `abstract_method`.
  - Um `homonym` de *n* alvos são *n* `CallLink` com a mesma `occurrence` (`v2.rs:538-…`).
  - A `evidence` (`graph.rs:24-33`) traz `source_id`, `path`, `source_sha256`, `start_byte`, `end_byte`, `start_line`, `end_line` e `excerpt_sha256`.
- **Por chamada sem ligação, `UnresolvedCall`** (`v2.rs:299-327`): `from`, `occurrence`, `occurrence_kind`, `text`, `reason`, `candidates` e `evidence`, mais `via`. Só com `node_limit`: `resolution`, `to` e `candidate_edge`.

**`/api/used-by`** (`GET`, `main.rs:130-136`, verificado):
- **Entrada:** `key`, `run`, `node` e `depth`.
- **O que roda:** o `UseIndex` é montado na primeira pergunta de um run (`UseIndex::build`, um `Resolver`, `uses.rs:134-135`). Ele fica em memória, **um run por vez**: trocar de run monta de novo (`main.rs:296-308`).
- **Resposta, `Uses`** (`uses.rs:113-128`): `seed`, `depth`, `max_nodes`, `truncated`, `nodes`, `links`, `counts` (`linked`, `possible_type`, `possible_homonym`, `name`, `above_limit`, `abstract_calls`, `ruled_out`; `uses.rs:102-110`), `above_limit`, `abstract_calls` e `ruled_out`.
- **Por uso, `Use`** (`uses.rs:58-87`): `from`, `to`, `occurrence`, `occurrence_kind`, `text`, `use` (`linked`, `possible`, `name`), `candidates`, `candidate_edge` e `evidence`. Opcionais: `resolution`, `basis`, `relation`, `instance_of`, `via` e `type_rule`. `use` é `possible` só quando `resolution == "homonym"` (`uses.rs:252`).
- **Por chamada à parte, `Aside`** (`uses.rs:91-98`): só `from`, `occurrence`, `occurrence_kind`, `text`, `candidates` e `evidence`. Não traz `resolution` nem o motivo: a lista em que está é o motivo.

**O que não chega por nenhuma das duas rotas** (o mesmo da seção D): um campo de certeza separado da origem; quem validou; e o que sustenta a decisão (import, definição, anotação, bases). A `evidence` é sempre o lugar da chamada.

**Diz ser × faz:** o comentário de `Use.basis` diz "For `possible`" (`uses.rs:74`), mas o código preenche o `basis` em todo uso de chamada, também nos `linked` (`uses.rs:256`).

**MAP-O16 conferida:** nenhuma das duas rotas lê uma ligação gravada. As duas leem o grafo (nós e arestas de nome) e a edition, e decidem com um `Resolver` novo: um por pedido em `/api/calls` e um por run em `/api/used-by` (citações acima).

**Por execução (Parada 2-bis, seção E):** sobre runs gerados de novo, `/api/calls` e `/api/used-by` deram as mesmas respostas da linha de base do 2.4d (241 chamadas, 28 símbolos). Nos 18 usos ligados falsos, o `Use` de `/api/used-by` traz o mesmo `resolution` e `basis` da `CallLink` de `/api/calls`.

**MAP-O17 confirmada com um caso concreto** (sintético de `tests/resolve.rs`, verificado pela execução da seção H):
- **Abaixo do teto.** `n_generic(m: &M<3>)` chama `m.det()` (`tests/resolve.rs:143-144`). O projeto tem três `det`: `impl M<2>`, `impl M<3>` e `impl Z` (`:89-104`).
  - A regra diz `type` com **dois** alvos, `M<2>::det` e `M<3>::det` (`:324`).
  - `/api/calls` devolve **três** `homonym`, um para cada aresta admitida, `Z::det` incluído, que o tipo exclui (`:386-387`).
- **Acima do teto.** No sintético de `usos_2_4d.rs:300`, com 9 `impl From<Ai> for Alvo` e mais `impl Outro::from`, a chamada `Alvo::from(A0)` tem 10 homônimos.
  - A regra dá `type` com os 9 corpos de `Alvo` (*leitura minha* do código: `resolve.rs:2124-2128`).
  - A ligação sai sem alvo, como `above_candidate_limit`. Isso o teste confere pelo inverso: a chamada está na lacuna de `impl From<A3> for Alvo::from` (`:304`), e `uses.rs:210` só põe na lacuna o que saiu como `above_candidate_limit`.
  - E ela não está na lacuna de `Outro::from` (`:305`), porque o tipo a exclui ali (`may_reach`).
- **Num run real** (Parada 2-bis, seção E, item 5): no ray tracer, `Object::from(..)` (82 chamadas) e `Patterns::from(..)` (17) saem `above_candidate_limit` com 14 homônimos e entram na lacuna dos `from` do projeto. *Leitura minha:* é o mesmo caso acima do teto.

### H. Testes que fixam a regra

São os arquivos da tabela B mais `g3.rs` e `g6.rs`, que fixam as candidatas e as chamadas sem ligação. Fora desses, os arquivos de `tests/` que citam `resolution` falam do envelope, do inventário, das editions e dos perfis antigos (busca `grep -cE "calls_from|Resolver::new|\.resolve\(|UseIndex|used_by\(|name_candidate|ambiguous_candidates"` por arquivo).

Os sintéticos vêm de dois lugares:
- **embutidos de `tools/precisao/sinteticos/`:** `heranca_2_4b.rs:20-22`, `despacho_2_4c.rs:19` e `classe_nome_2_4d0.rs:19-21`, por `include_str!`; e `usos_2_4d.rs:62-65`, que lê as pastas `py` e `rs` inteiras em execução (`:145`, `:153`);
- **escritos pelo próprio teste:** os outros arquivos montam projetos pequenos com o texto que o teste guarda em constantes e os gravam numa pasta temporária do sistema, apagada no fim (ex.: `tests/resolve.rs:195-221`).

| Arquivo | Testes | O que verifica | Sintéticos |
|---|---:|---|---|
| `calls.rs` | 2 | seguir só onde o nome aponta para uma declaração; busca sem testes | no próprio arquivo |
| `resolve.rs` | 11 | uma resolução por teste (`unique`, `receiver` só em `self.m()`, `type`, `type` acima do teto, `trait_default`, `trait`, `homonym`, `chosen` antes de tudo); o que `/api/calls` reporta, vários alvos como homônimos; `type` seguido e `homonym` não; o dono só usa as ligações decididas | no próprio arquivo (Rust e Python) |
| `resolve_forma.rs` | 14 | a forma: método nunca chega a função livre; receptor de módulo; import de fora não chega a nada; nome local vence import; chamada simples nunca chega a método; o tipo escrito derruba o `unique`, mas não por `&`, `Box`, `Rc`, `Arc` nem por trait implementado; `self.m()` acima do teto; as admitidas na API | no próprio arquivo |
| `tipos_2_3a.rs` | 11 | o tipo Python do receptor (anotação, construtor, campo, `self`); classe pelos imports; classe sem base descarta; acima do teto pelo import; `Protocol` sem ligação; instanciação; struct de tupla e variante; a regra de cada `receiver_type`; perfis antigos | no próprio arquivo |
| `dono_2_3b.rs` | 4 | chamada em decorador é `decorates` e não é seguida; o método responde pelos campos; uma contagem por chamada; o texto da cadeia | no próprio arquivo |
| `retorno_2_3c.rs` | 12 | o retorno escrito tipa o receptor (mesmo arquivo, entre arquivos, `Optional`, `self`, campo de outro arquivo); nada por tipo de fora, genérico ou sem anotação; candidatas que concordam ou não; um nível só; a escolha não chega à chamada que liga; `let x = T::f(..)`; invólucros não decidem; retorno de fora | no próprio arquivo |
| `escopo_2_4a.rs` | 12 | o escopo de quem chama: def do módulo, o último depois de `@overload`, import e reexport, import de fora/variável/builtin descartam, o que o escopo não diz, alias, `scope` com `basis` próprio; em Rust, item do módulo ou bloco, `use`, glob, caminho, alias | no próprio arquivo |
| `escopo_visivel_2_4a.rs` | 6 | `global`, `nonlocal` e só a definição visível, em Python e Rust | no próprio arquivo |
| `heranca_2_4b.rs` | 7 | a cadeia C3; o que ela não diz fica indeciso; fora da cadeia nunca é alvo; `super()`; abstrato dá as sobrescritas; `__init__` herdado; sobrescritas acima do teto | `sinteticos/py/pkg/heranca_apoio.py`, `heranca.py`, `heranca_regras.py` |
| `despacho_2_4c.rs` | 9 | todos os corpos que podem rodar; sem sobrescrita a ligação fica; `__new__`; nome privado; subclasse que liga o nome; sobrescrita vazia; o abstrato que saiu; os corpos podem ser escolhidos | `sinteticos/py/pkg/despacho.py` |
| `classe_nome_2_4d0.rs` | 7 | `Classe.m()` pelo escopo; sobe a cadeia e nunca desce; variável ou duas classes não são a classe; nome que não é só método (o religado); base de fora; o retorno tipa; a regra acima do teto | `sinteticos/py/pkg/classe_nome.py`, `classe_nome_fora.py`, `classe_nome_usos.py` |
| `usos_2_4d.rs` | 7 | toda ligação é um uso e todo uso é uma ligação; os tipos de uso à parte; instanciação usa a classe e o `__init__`; o caminho sobe só pelos ligados; o teto corta o desenho; acima do teto é lacuna filtrada pela forma e pelo tipo, com o limite do primitivo | `sinteticos/py` e `sinteticos/rs` inteiros, mais textos no arquivo |
| `g3.rs` | 5 | chamada acima do teto com a contagem e sem candidata; sem nome igual no projeto; `node_limit` à parte; chamada sem nome escrito não some; campos antigos ficam | no próprio arquivo |
| `g6.rs` | 4 | campos são símbolos; acessos, chamadas e bases são candidatas, nunca resoluções; homônimos demais guardam só a contagem; usos pelo nome em camadas | no próprio arquivo |

**Execução** (verificado):

```
cargo test -p librarian-ingest --locked --test calls --test resolve --test resolve_forma \
  --test tipos_2_3a --test dono_2_3b --test retorno_2_3c --test escopo_2_4a \
  --test escopo_visivel_2_4a --test heranca_2_4b --test despacho_2_4c \
  --test classe_nome_2_4d0 --test usos_2_4d --test g3 --test g6
```

O exit foi 0, com **111 testes aprovados**, 0 falhas e 0 ignorados (log no scratchpad). Por binário:

| `calls` | `classe_nome_2_4d0` | `despacho_2_4c` | `dono_2_3b` | `escopo_2_4a` | `escopo_visivel_2_4a` | `g3` | `g6` | `heranca_2_4b` | `resolve` | `resolve_forma` | `retorno_2_3c` | `tipos_2_3a` | `usos_2_4d` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2 | 7 | 9 | 4 | 12 | 6 | 5 | 4 | 7 | 11 | 14 | 12 | 11 | 7 |

Depois da execução, `git status --porcelain -uall` continuou vazio, fora este relatório.

---

## Em palavras simples

*À parte das seções acima, sem números nem nomes de arquivo. Foi escrita com a skill "elementary school teacher" (nível newcomer).*

Quando o código de um projeto chama alguma coisa pelo nome, o Librarian tenta dizer **qual** coisa é. Ele desenha essa resposta de três jeitos.

**Linha cheia: "é esta".** Ela aparece quando o Librarian tem um motivo para afirmar:
- o nome foi trazido de outro arquivo por um import;
- o nome foi definido ali mesmo, onde a chamada está;
- o tipo do objeto está escrito e só uma declaração serve a ele;
- ou o nome só combina com uma coisa dentro do projeto.

Nas chamadas que uma pessoa já conferiu, quase todas as linhas cheias estão certas. **Mas não todas.** As erradas que sobraram são todas do último caso, o "só tem uma com esse nome". Em algumas chamadas, a coisa que roda de verdade vem de fora do projeto: da linguagem ou de uma biblioteca. O Librarian não vê o que está fora. Então ele liga a chamada à única coisa do projeto que tem o mesmo nome.

Exemplos:
- um "dublê" que só os testes usam, no lugar da biblioteca de rede verdadeira;
- o "tamanho de uma lista" da linguagem, ligado a um método do projeto com o mesmo nome;
- o "limitar um número" da linguagem, ligado ao "limitar uma cor" do projeto.

Na pergunta "quem usa isto?", aparecem erros do mesmo tipo.

**Linha tracejada: "pode ser esta ou aquela".** Ela aparece quando há várias coisas com o mesmo nome e nada decide entre elas. Às vezes a dúvida é honesta: o objeto pode ser de vários tipos, e qualquer um dos corpos pode rodar. Mas, na maioria das vezes que uma pessoa conferiu, **nenhuma** das opções tracejadas era a certa: a coisa que roda vinha de fora do projeto. A linha tracejada parece dizer "uma destas", mas muitas vezes quer dizer "talvez nenhuma destas".

**Lacuna: "não sei".** A chamada não ganha linha. Isso acontece quando o nome não existe no projeto, ou quando há tantas coisas com o mesmo nome que o Librarian nem as lista. Nas chamadas conferidas, quase todas as lacunas eram mesmo coisas de fora, e não esquecimentos.

Mas a mesma frase "não há nada no projeto" serve para duas situações diferentes. Numa, nenhum nome igual existia. Na outra, o Librarian **descartou** os nomes iguais, porque sabia que a chamada ia para fora. Quem lê a resposta não sabe qual foi. E há um caso em que o descarte se perde: se a chamada tem nomes iguais demais, ela aparece como "nomes demais", e na pergunta "quem usa isto?" pode aparecer como "talvez use". Isso foi lido no código, mas não aconteceu em nenhuma chamada conferida.

**Duas coisas a lembrar:**
- Essas respostas não ficam guardadas: o Librarian as calcula de novo cada vez que alguém pergunta.
- Quando a regra diz "é uma destas duas", a página às vezes mostra mais opções do que a regra disse, ou nenhuma.

---

## Observações novas (Parada 2)

- **MAP-O24.** Nas amostras validadas, **todo link falso de linha cheia de hoje é `unique`** (4 de 4: HC-27, UQ-4, UQ-19, UQ-23), sempre com a verdade de fora do projeto (seção E). Não há falso em `import`, `scope`, `receiver`, `type` ou `trait_default`. Isso bate com o motivo da tabela C para o `unique`: "esta ou algo de fora". Na vista dos usos, há 18 usos ligados falsos em três símbolos (HC 4, RT 14), cujo estado o resultado gravado não guarda. *Parada 2-bis:* por execução, os 18 também são `unique` (`graph_edge`, 1 candidata).
- **MAP-O25.** O `linha_de_base` das fixtures é o estado da época da seleção (`amostra.js:54`). Ninguém o atualiza, e o portão não o lê: 143 das 241 chamadas de HC e RT respondem diferente hoje.
  - O estado de hoje está em `linha-de-base.json` (`resposta`), que **não grava o hash das fontes** (as chaves são `projetos`, `editions`, `fatos`, `niveis` e `studio_original`). Só o estrato `unique` grava (`fontes`).
  - Ligar uma resposta ao código depende do Git (seção E).
- **MAP-O26.** O tracejado por homônimo é falso na maior parte das vezes conferidas:
  - das 32 chamadas `homonym` de hoje nas amostras (HC 22, RT 10), 15 têm todas as candidatas falsas;
  - dos 40 pares PH, em 36 a candidata não pode rodar.
- **MAP-O27.** MAP-O17 confirmada com um sintético (seção G): a regra diz 2 alvos, e a API mostra 3 homônimos; acima do teto, a regra decide por tipo, e a API diz `above_candidate_limit`.
- **MAP-O28.** MAP-O18 conferida em parte (seção E):
  - o mesmo motivo para "sem nome igual" e para "descartada" está nos dados, com contagem 0 nos dois;
  - o falso "pode usar" por escopo é possível pelo código, mas não aparece nos dados nem nos testes. *Parada 2-bis:* também não aparece nos runs inteiros. No HC, as 48 chamadas simples acima do teto são `import`. No RT não há nenhuma de um segmento, e só nelas o escopo do Rust dá `Nothing` (`resolve.rs:1817`);
  - a HC-169 mostra outro caso vizinho: a forma sabe que o receptor é de fora, e a ligação diz "acima do teto".
- **MAP-O29.** As guardas contra link falso que existem são do Python: nome religado, atributo, `__getattr__`, `Protocol`, metaclasse, nome privado e `super()`. Não há guarda contra:
  - um dublê de teste (a regra não lê `is_test`);
  - um tipo da linguagem ou de fora que não está escrito.

  Três testes fixam como **esperado** um resultado que a verdade diria falso (seção F): `tests/resolve.rs:324` (`M<2>::det`), `usos_2_4d.rs:304-305` (`f64::from` como "pode usar") e `retorno_2_3c.rs:514` (`take` tracejado entre dois métodos do projeto, quando roda o de `Option`).
- **MAP-O30.** Diz ser × faz: `Use.basis` "For `possible`" (`uses.rs:74`), mas o código o preenche em todo uso de chamada (`uses.rs:256`).
- **MAP-O31.** Correções da Parada 1, feitas no lugar e marcadas:
  - `calls.rs` tem 2 testes, e não 3;
  - `escopo_2_4a.rs` tem 12, e não 14;
  - `CallLink` vai até `v2.rs:374`;
  - em MAP-O16, é um `Resolver` por pedido.

## Observações novas (Parada 2-bis)

- **MAP-O32.** A Parada 2 fica conferida por execução: o portão (fontes `3aeb2fa7…`, aprovado) e a sonda, sobre runs novos, dão as mesmas respostas da linha de base em HC, RT, UQ e usos. Nenhuma contagem da seção E muda. O que a execução acrescentou:
  - os `homonym` da amostra são 4 com `basis: "type"` (HC-140, 141, 142, 144) e 28 com `graph_edge`;
  - os tracejados com todas as candidatas falsas são todos `graph_edge`;
  - os 18 usos ligados falsos são `unique` pelo nome.
- **MAP-O33.** `withStudio` tenta apagar a cópia temporária, mas ignora a falha no Windows ("the studio may still hold a file", `lib.js:137`). Na pasta temporária do sistema há **121** pastas `librarian-precisao-*`, de 18/11 a 25/11 (3, 23, 7, 2 e 86 por dia; `ls -ld --time-style=+%F "$LOCALAPPDATA"/Temp/librarian-precisao-*`). Nenhuma é de 26/11: as cópias desta parada foram apagadas. As antigas são cópias da `.studio` deixadas por medições anteriores, fora do repositório. Não as apaguei. Só o fato.
- **MAP-O34.** No ray tracer, 640 chamadas `T::f(..)` passam do teto. 475 são decididas pelo tipo. 165 saem `above_candidate_limit`, e 132 delas (`Object::from`, `Patterns::from`, `f64::from`) entram na lacuna "pode usar" dos `from` do projeto (seção E, item 5). Das 132, 32 são `f64::from`, cuja verdade é de fora (o limite do primitivo, seção F).

## Perguntas (Parada 2)

- **MAP-Q11.** A convenção "dublê de teste não conta como alvo de código de produção" (DEC-33, citada na justificativa da HC-27 e da UQ-4) vive só na verdade das fixtures. A regra não conhece código de teste (seção F). No mapa, ela é **regra do sistema** que falta, ou **convenção da medição**? A resposta muda se a HC-27 e a UQ-4 contam como link falso "do Librarian" ou como diferença de critério.

MAP-Q9 e MAP-Q10 continuam em aberto. A tabela C mantém o `unique` como "não cabe", e a seção D responde pelos dois sentidos de "acervo".

---

*Parada 3 (o gabarito do 2.4e, no relatório 04): aguardando liberação.*

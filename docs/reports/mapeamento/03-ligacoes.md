# Mapeamento do repositório — parte 3: como uma chamada vira ligação

**Estado:** pendente de validação do usuário. Feita só a Parada 1 (A a D); a Parada 2 (E a I) não começou. Em aberto: MAP-Q9, MAP-Q10.

**Data:** 26/11/2026. **Base:** `main` em `6ac1865`. Worktree: só os relatórios do mapeamento, não rastreados. **Só leitura**: nenhum código, documento, `.studio`, Healthchecks ou ray tracer foi alterado. A `.studio/` continua vazia (MAP-O1; `ls -la .studio` lista só `.` e `..`) e não foi tocada. O portão não foi rodado, nem nenhum binário que grava.

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

**O 2.4e não mexeu na regra** (verificado): `git show --stat aead3eb` só toca `tools/precisao/gabarito/` (11 arquivos), e o último commit em `resolve.rs` é o `665ff5b` do 2.4d (`git log -- crates/librarian-ingest/src/resolve.rs`). O gabarito do 2.4e é a parte I, na Parada 2.

**O que cada passo acrescentou, pelos testes.** Primeira linha de cada arquivo (diz ser) e nomes dos testes, por `grep -nE '^\s*fn [a-z0-9_]+\(\)'`:

| Passo | Arquivo de teste | O que os nomes fixam |
|---|---|---|
| P1b | `tests/calls.rs` (3 testes) | seguir só quando o nome aponta para uma declaração; busca sem testes |
| 2.1 | `tests/resolve.rs` (11) | uma resolução por teste: `unique`, `receiver` só em `self.m()`, `type`, `type` acima do teto, `trait_default`, `trait`, `homonym`, `chosen` antes de tudo. Também: `calls_from` reporta um alvo e mantém vários como homônimos; `type` é seguido e `homonym` não; o dono só usa as ligações decididas |
| 2.2 | `tests/resolve_forma.rs` (14) | método nunca chega a função livre; receptor de módulo do projeto; import de fora não chega a nada; nome local vence import; chamada simples nunca chega a método; o tipo escrito derruba o `unique`, mas não por `&`, `Box`, `Rc`, `Arc`, nem por um trait que o projeto implementa; `self.m()` acima do teto em Rust e em Python; `calls_from` reporta as admitidas |
| 2.3a | `tests/tipos_2_3a.rs` (11) | anotação, construtor, campo do `__init__` e `self` dão o tipo; classe pelos imports; classe sem base derruba a candidata; classe acima do teto pelo import; `Protocol` não dá ligação; instanciação como relação própria, que roda o `__init__`; struct de tupla e variante; a regra de cada `receiver_type`; perfis antigos sem nada disso |
| 2.3b | `tests/dono_2_3b.rs` (4) | chamada em decorador é `decorates` e não é seguida; o método responde pelos campos que declara; cada chamada contada uma vez; o texto do passo da cadeia |
| 2.3c | `tests/retorno_2_3c.rs` (12) | o retorno tipa o receptor no mesmo arquivo e entre arquivos; `Optional` e `self`; campo de outro arquivo; nada por tipo de fora, genérico ou sem anotação; candidatas que concordam ou discordam; um nível só; a escolha da pessoa não chega à chamada que liga; `let x = T::f(..)` pelo retorno de `f`; invólucros não decidem |
| 2.4a | `tests/escopo_2_4a.rs` (14) e `tests/escopo_visivel_2_4a.rs` (6) | def do módulo decide entre homônimos; o último def depois dos `@overload`; import e reexport; import de fora, variável e builtin descartam; o que o escopo não diz fica como antes; alias de import instancia e tipa; `scope` com `basis` próprio; item do módulo ou do bloco, `use`, glob e caminho de módulo; alias por `use as` e `type`. Depois de 24/11: `global`, `nonlocal` e só definição visível |
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

**Campos de cada ligação de `/api/calls`** (`CallLink`, `v2.rs:330-372`, verificado) e o que cada um diz:

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

- **MAP-O16.** As ligações **não são guardadas**. O grafo gravado (`graph-v2.json`, `store.rs:349-351`) só tem as arestas de nome (`name_candidate`). Cada ligação é decidida de novo a cada `/api/calls` (`v2.rs:454`) e na primeira `/api/used-by` de um run (cache em memória, `main.rs:297-308`). A regra também lê, a cada vez, os bytes da edition (`lib.rs:540-560`). Por isso a "ligação guardada no grafo" do ponto final da parte 3 é, no código, uma resposta calculada sobre o grafo.
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

*Parada 2 (E a I e "Em palavras simples"): aguardando liberação.*

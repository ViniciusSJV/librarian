# Mapeamento do repositório — parte 6: o extrator

**Estado:** Pendente de validação do usuário: Parada 1 (A a D)

**Data:** 26/11/2026. **Base:** `main` em `66efcbf` (worktree limpa no início). **Só leitura**: nenhum código, documento, `.studio`, Healthchecks ou ray tracer foi alterado. Nenhum LLM foi chamado. A execução foi feita só sobre uma **cópia** da `.studio`, no scratchpad da sessão. No fim, o original foi conferido e a cópia, apagada (seção "Execução").

**Escopo.** Do pedido de geração (`/api/generate` no studio e `librarian_ingest generate` na CLI) até a edition gravada. A regra de ligação, a busca e o léxico já estão nas partes [3](03-ligacoes.md) e [5](05-lexico.md); deles, só se cita onde leem o que o extrator produz. Ids novos continuam a numeração do 05: `MAP-O58` em diante e `MAP-Q20` em diante.

**Fontes.** `.rs`, `.py`, `.js`, `Cargo.toml`, os JSON de configuração (`project.json`, `librarian.json` e o `sources_config` do ray tracer), os JSON da cópia da `.studio` e o código-fonte do `syn` fixado, no registro local do cargo. Nenhum `.md` foi aberto além dos relatórios 01 a 05; do ESTADO-ATUAL, só as regras de trabalho. Comentários no código e textos de limite contam como **"diz ser"**.

**Regras de evidência:** as mesmas das partes 1 a 5. Cada afirmação traz `ARQUIVO:linha` ou o comando. **Verificado** quer dizer por execução ou pela leitura do código que faz a coisa. **Leitura minha** marca interpretação.

Caminhos abreviados: `lib.rs`, `cli.rs`, `graph.rs`, `resolve.rs`, `search.rs`, `envelope.rs` e `prepare.rs` são de `crates/librarian-ingest/src/`; `mod.rs`, `rust.rs`, `python.rs` e `python_types.rs` são de `crates/librarian-ingest/src/extractors/`; `store.rs`, `main.rs` e `instrument.rs` são de `crates/librarian-studio/src/`.

**Casos reais usados** (cópia da `.studio`, MAP-O1):
- os runs gravados de 18/11: ray tracer `renderer-9b78f3a9/runs/20261118-020654` e Healthchecks `healthchecks-25597e41/runs/20261118-061046`. **Os dois são do perfil v8** (`librarian-ingest/0.8.0;…;rust-structural-v6;python-structural-v4`, no `manifest.json`), e não do atual, o v9 (MAP-O69);
- dois runs novos, gerados **na cópia** por `/api/generate`, com o perfil v9: ray tracer `20261126-123529` (34 fontes, 819 símbolos, 8.707 nós) e Healthchecks `20261126-123530` (657 fontes, 4.718 símbolos, 66.965 nós). São os que respondem ao D2;
- um projeto sintético de 9 arquivos, montado dentro da pasta da cópia, para o item A (seção "Execução").

---

## Parada 1

### A. As entradas

**O caminho da chamada** (verificado, por leitura):

```
studio                                              CLI
POST /api/generate {key}      main.rs:129           librarian_ingest generate <raiz> <config.json> <edition-nova>
 → Store::generate            store.rs:340           cli.rs:63-71
   lê Settings do project.json   :342                 lê o Config do arquivo  cli.rs:64
   reserve runs/<carimbo>        :343 (store.rs:1537-1549)
   Config {project, inputs}      :345
 → librarian_ingest::generate (perfil atual: Registry::current() = V9)        lib.rs:307-309, mod.rs:469-471
   → generate_with                                                             lib.rs:345-421
       discover (lista de arquivos)                                            :352 → lib.rs:255-265
       git_metadata (rev-parse HEAD, status --porcelain)                       :353 → lib.rs:92-108
       para cada arquivo: lê inteiro, source(), extractor.extract()            :359-370
       grava snapshots/, os 4 registros e o manifest.json por último           :408-419
 → Catalog::load(edition)     store.rs:348            Catalog::load  cli.rs:67
   relê, confere os hashes e EXTRAI DE NOVO cada fonte, comparando             lib.rs:426-528 (:486-502)
 → v2::Graph::build → graph-v2.json → verify          store.rs:349-352   (a CLI não monta o grafo aqui)
 → run.json                                           store.rs:353-367
```

**De onde vem a lista de arquivos** (verificado):

| Etapa | O que decide | Onde |
|---|---|---|
| A lista `inputs` | O `Settings.inputs` do `project.json` (`store.rs:345`). Ao abrir o projeto pela primeira vez: num projeto Cargo, `src`, `tests`, `benches`, `examples`, `Cargo.toml` e `Cargo.lock`, os que existem (`store.rs:1466-1472`); em outro projeto, `suggested_inputs`, que lista o primeiro nível da pasta, sem pastas ocultas, sem as pastas puladas e só os arquivos de extensão conhecida (`store.rs:1473`, `lib.rs:313-331`). O `librarian.json` do projeto pode trocar a lista pelo `sources_config` (`store.rs:1503-1506`). Depois, a pessoa edita em Configurações, e `save_settings` só recusa a lista vazia (`store.rs:331`) | — |
| Conferência da lista | Projeto e entradas não vazios, e cada entrada um caminho relativo canônico: sem `\`, `:`, `.`, `..` nem segmento vazio (`lib.rs:176-191`, `:208-216`) | `config_check` |
| Percurso | Uma entrada que é pasta é percorrida inteira, recursivamente, em ordem alfabética (`lib.rs:227-244`). Uma entrada explícita que é arquivo entra sempre, qualquer que seja a extensão (`lib.rs:246`). Dentro de uma pasta, só entra o arquivo de extensão conhecida (`registry.discovers`, `mod.rs:488-490`) | `walk`, `lib.rs:218-253` |
| Exclusões | Só por **nome de pasta**, em qualquer nível, e nunca numa entrada explícita: as 15 de `skipped_dirs` (`.git`, `.hg`, `.svn`, `.venv`, `venv`, `.tox`, `.nox`, `__pycache__`, `.mypy_cache`, `.pytest_cache`, `.ruff_cache`, `site-packages`, `node_modules`, `target`, `.studio`; `mod.rs:459-462`; aplicadas em `lib.rs:228-231`). Uma pasta oculta fora dessa lista **é percorrida** (verificado: `src/.oculta/h.rs` entrou na edition sintética). O `.gitignore` não é lido (nenhuma leitura dele em `lib.rs`) | — |
| Recusas que param a geração inteira | Link simbólico em qualquer segmento do caminho (`lib.rs:198-200`); caminho que sai da raiz (`:202-204`); nome de arquivo que não é UTF-8 (`:238`); algo que não é arquivo nem pasta (`:249-251`); nenhum arquivo selecionado (`:261-263`); destino dentro de uma pasta de entrada (`:402-407`); destino que já existe (`fs::create_dir`, `:408`) | — |
| Tamanho | **Não há limite.** Cada arquivo é lido inteiro na memória (`lib.rs:360`) e guardado inteiro como snapshot (`:368`, `:410-412`). O único número de tamanho gravado é `byte_len` (`lib.rs:116`, `:273`) | — |

**Casos reais:**
- ray tracer: `inputs` = `src`, `tests`, `Cargo.toml`, `Cargo.lock`, do `sources_config` do `librarian.json` (`ai/acervo/renderer-sources.json`). 34 fontes: 32 `rust` e 2 `auxiliary` (`Cargo.toml`, `Cargo.lock`);
- Healthchecks: sem `librarian.json`, então `suggested_inputs` deu `docker`, `hc`, `manage.py`, `static`, `stuff`, `templates`. 657 fontes, todas `python`. Das pastas `static` e `templates` só entram os `.py`; os `.js`, `.css` e `.html` não são vistos nem guardados.

**Quem decide a linguagem** (verificado): só a **extensão**, por `Registry::language` (`mod.rs:478-485`), chamada em `source()` (`lib.rs:267-277`). No v9 são duas: `rs` → `rust` e `py` → `python` (`mod.rs:427`). Tudo o mais é `auxiliary`. Não se olha o conteúdo, a primeira linha (`#!`) nem a caixa da extensão: um `.pyi` ou um `.PY` é `auxiliary`. O `Catalog::load` recalcula a linguagem e recusa se ela mudou (`lib.rs:480-482`).

**O que acontece em cada caso** (verificado, por leitura e na edition sintética):

| Caso | O que o código faz | Resultado gravado |
|---|---|---|
| Linguagem desconhecida, dentro de uma pasta | não é selecionado (`lib.rs:246`) | nada: não é lido nem guardado (verificado: `src/leia.txt` ficou fora) |
| Linguagem desconhecida, entrada explícita | lido, guardado e com hash, mas sem extrator (`lib.rs:362`; comentário "Unlisted extensions are auxiliary: preserved and hashed, never extracted", `mod.rs:478`, diz ser) | uma fonte `auxiliary`, sem símbolos (verificado: `docs/README.md`) |
| Bytes que não são UTF-8 | `invalid_utf8` (`rust.rs:47-50`, `python.rs:40-43`) | a fonte, o snapshot e **um diagnóstico**; nenhum símbolo do arquivo |
| Erro de sintaxe | `rust_parse_error` ou `python_parse_error`, com a mensagem do parser (`rust.rs:51-54`, `python.rs:44-47`) | idem: o arquivo inteiro fica sem símbolos, mesmo que só uma linha esteja errada. Verificado: `fn quebrado( {` deu "cannot parse string into token stream"; `def f(:` deu "invalid syntax. Got unexpected token ':' at byte offset 6" |
| Arquivo grande | nada o corta. *Medido* (só `generate` e `verify` da CLI, arquivo de funções de uma linha): 56 KB → 181 ms; 230 KB → 895 ms; 474 KB → 3,0 s; 970 KB → 11,9 s. Um arquivo de 3,7 MB passou de 120 s e a geração foi interrompida | o tempo cresce cerca de 4 vezes quando o tamanho dobra. *Leitura minha, não medida por perfil:* `line_at` conta as quebras de linha desde o começo do arquivo para cada trecho (`lib.rs:172-174`, chamada em `rust.rs:253-254` e `python.rs:810-811`) |

**Depois de um diagnóstico** (verificado): a geração não falha. A CLI sai com código 2 (`cli.rs:70`), e o studio grava o run sem recusar (`store.rs:340-369`). Quem recusa é o `prepare`, a não ser que o pedido traga `accept_diagnostics` (`prepare.rs:119-120`). O grafo também não tem nada do arquivo: o `observe` de um arquivo que não compila devolve lista vazia (`rust.rs:74-79`, `python.rs:60-65`). Os dois projetos reais têm **0 diagnósticos** nos quatro runs.

**O extrator roda quatro vezes, e só a primeira grava na edition** (verificado; ver MAP-O58):

| Quando | Função | O que sai | Onde fica |
|---|---|---|---|
| Na geração | `extract` (`lib.rs:362-367`) | símbolos, trechos e diagnósticos | `symbols.jsonl`, `chunks.jsonl`, `diagnostics.json` da edition |
| Em todo `Catalog::load` | `extract` de novo (`lib.rs:486-491`), comparado com o gravado (`:497-502`) | a mesma coisa, para conferir | nada novo; recusa se difere |
| Na construção do grafo | `observe` (`graph.rs:118-124`) | as ocorrências (chamadas, tipos escritos, acessos…) | só no `graph-v2.json` que o studio grava (`store.rs:351`). A edition não guarda ocorrência nenhuma |
| A cada pergunta da regra | `call_facts` (`lib.rs:549-558`) | imports, escopos, tipos escritos, classes, retornos | nada: relido dos snapshots a cada vez (parte 3, MAP-O16) |

O `Catalog::load` é chamado em cada `/api/calls` (parte 3, G) e no `prepare` de cada pergunta. Por isso a extração inteira do projeto é refeita a cada um desses pedidos.

### B. O extrator Rust

**Biblioteca e perfil:** `syn` `=3.0.5` com `full` e `visit`, e `proc-macro2` com `span-locations` (`crates/librarian-ingest/Cargo.toml:11-12`). O perfil v9 usa `RUST_V8`, com todas as opções ligadas (`rust.rs:34`, `mod.rs:428`). `parse` tira o BOM e o `#!` e guarda o deslocamento, para os bytes baterem com o original (`rust.rs:213-218`).

**Símbolos** (`Declarations`, `rust.rs:220-329`; verificado):

| `kind` | De onde | Observação |
|---|---|---|
| `function`, `struct`, `enum`, `union`, `trait`, `type`, `module`, `const`, `static` | os `Item` correspondentes (`rust.rs:269-277`) | também os itens escritos **dentro** do corpo de uma função, com o nome da função no `qualified_name` (o visitor desce nos blocos; caso real: `tests_count::…::counted`, `src/count.rs:122`) |
| `impl` | `impl T` ou `impl Trait for T`, com esse texto como nome (`rust.rs:278-287`) | é navegação, não resposta (`rust.rs:179-181`) |
| `method`, `trait_method` | `fn` dentro de `impl` e de `trait` (`rust.rs:314-328`) | — |
| `field` | campo **com nome** de `struct` (`rust.rs:293-300`) | campo de tupla não tem nome e não entra |
| `variant` | cada variante de `enum` (`rust.rs:301-306`) | — |

Cada símbolo tem um trecho (`Chunk`) com bytes, linhas e hash (`rust.rs:248-256`). O `qualified_name` é o caminho de declarações no mesmo arquivo, separado por `::` (`rust.rs:236-242`): "File-local syntactic scope, not a resolved path" (`lib.rs:126`, diz ser).

**Ocorrências** (`Occurrences`, `rust.rs:332-1152`, só no grafo): `type_reference`, `impl_self_type`, `struct_literal`, `field_init`, `field_assign`, `declared_return`, `call_observed`, `method_call_observed`, `name_reference`, `struct_pattern`, `field_pattern`, `loop`, `loop_iterable`, `attribute_access`, `local_binding` e `receiver_type` (os `push` de `rust.rs:884-1151`). As chamadas são descritas na parte 3, A.

**Fatos da regra** (`call_facts`, `rust.rs:109-152`, nunca gravados): os tipos escritos dos receptores e se são referência; as chamadas que ligam um receptor (`let x = T::f(..)`); os retornos escritos (`Returns`, `rust.rs:421-445`); os módulos, os `use` (com `as` e glob), os aliases `type` e as variáveis de cada função (`Modules`, `rust.rs:527-646`).

**O que o código mostra que descarta ou não consegue** (verificado):

| O quê | Onde | Caso real |
|---|---|---|
| **Conteúdo de macro.** O `syn` não expande macros, e o visitor não entra nos tokens: `visit_macro` chama `visit_token_stream`, que é vazio (`syn-3.0.5/src/gen/visit.rs:2830-2838`, `:768`). Nenhuma chamada, tipo ou nome escrito dentro de `println!`, `assert_eq!` ou `vec!` vira ocorrência | — | o ray tracer tem 568 invocações de macro, contadas por `grep` em `src` e `tests` (`assert_equivalent!` 201, `assert_eq!` 196, `vec!` 80, `assert!` 36…) |
| **Macros do projeto não são símbolos**: `Item::Macro` cai no `_ => None` (`rust.rs:288`) | — | `macro_rules! count` (`src/count.rs:70`), `assert_equivalent` e `not_equivalent` (`src/equivalent.rs:18`, `:34`) não aparecem na edition |
| **`cfg` não é avaliado**: todo item entra, qualquer que seja o `#[cfg(..)]` | "No macro expansion, cfg evaluation…" (`mod.rs:431`, diz ser); o `add` não olha atributos além do `test` (`rust.rs:245`) | o módulo `tests_count`, sob `#[cfg(all(test, feature = "librarian-count"))]` (`src/count.rs:116`), está na edition |
| Itens sem símbolo | `use`, `extern crate`, blocos `extern`, alias de trait e `Verbatim` (o `_ => None` de `rust.rs:288`); `const` e `type` associados de `impl` e de `trait` (só `visit_impl_item_fn` e `visit_trait_item_fn` são sobrescritos, `rust.rs:314-328`) | — |
| Nomes fora do arquivo | escopo só no arquivo; sem resolução entre arquivos (`rust.rs:1-2`, diz ser). Os `use` só são lidos pela regra, na hora da pergunta | — |
| Ocorrência fora de qualquer símbolo | o grafo a descarta: sem dono, `return` (`graph.rs:308-320`) | (em Rust quase tudo está dentro de um item) |

### C. O extrator Python

**Biblioteca e perfil:** `rustpython-parser` e `rustpython-ast` `=0.4.0` (`Cargo.toml:13-14`). O v9 usa `PYTHON_V9` (`python.rs:27`, `mod.rs:428`). `parse` tira o BOM (`python.rs:659-662`).

**Símbolos** (`Walker`, `python.rs:665-1202`; verificado):

| `kind` | De onde | Observação |
|---|---|---|
| `class` | `class` (`python.rs:1050-1051`) | é navegação, não resposta (`python.rs:109-111`) |
| `method` | `def` ou `async def` logo dentro de uma classe (`python.rs:817`) | — |
| `function` | qualquer outro `def`, inclusive o escrito dentro de um método (`python.rs:817`) | — |
| `field` | nome atribuído no corpo da classe, e `self.<nome>` atribuído no `__init__` (`python.rs:827-848`) | um campo por classe, na primeira atribuição (`:843`). `self.x = ..` em outro método não é campo |
| `variable` | nome atribuído no nível do módulo, na primeira atribuição (`python.rs:731-741`) | só alvo `Nome` simples: `a, b = ..` não declara variável. Vale também dentro de um `if` do módulo (caso real: `msg`, `manage.py:15`) |

O trecho de um `def` ou de uma `class` começa no `@` do primeiro decorador (`python.rs:786-789`). O `qualified_name` usa `.` (`python.rs:791-800`).

**Ocorrências** (só no grafo): `declared_return`, `type_reference`, `base_class`, `local_binding`, `name_reference`, `name_write`, `keyword_argument`, `attribute_access`, `call_observed`, `method_call_observed`, `parameter`, `keyword_parameter`, `list_binding`, `membership`, `loop`, `loop_iterable`, `with_block` e, desde o v9, `receiver_type` (`python.rs:73-76`).

**Herança, decoradores, imports e tipos** (verificado):
- **Bases:** cada base escrita como nome ou atributo vira `base_class`, com o último segmento como candidato (`python.rs:1053-1059`). Para a regra, `Fields::read` guarda as bases como escritas (`object` fora), se a classe tem decorador ou `keywords` (`dynamic`), `__getattr__`, `Protocol`, os nomes que a classe liga e os atributos gravados por `self`/`cls` (`python_types.rs:301-331`, `:343`).
- **Decoradores:** não são símbolos. Entram no trecho da declaração (acima). As chamadas escritas neles viram ocorrências normais, e o `call_facts` guarda o trecho de cada decorador (`python.rs:255-260`). O que cada decorador faz não é avaliado (`python.rs:1-2`, diz ser). Só alguns nomes são lidos: `staticmethod`, `classmethod`, `property` e `abstractmethod` (`python_types.rs:518-524`, `:662-691`).
- **Imports:** não são símbolos. Só o `call_facts` os lê, por escopo: `import a.b`, `import a as b`, `from .x import y as z` e `*` (`python.rs:446-485`), mais `global`, `nonlocal`, condicionais e `if TYPE_CHECKING:` (`python.rs:513-526`, `:631-642`).
- **Tipos anotados** (`annotation`, `python_types.rs:80-114`): `T`, `m.T`, `"T"`, `T | None`, `Optional[T]`, `Union[T, None]` e `Annotated[T, ..]`. **Não** dão tipo: a união de dois tipos, `list[T]`, `type[T]` e qualquer outro genérico `Foo[T]` (o ramo `Subscript` só aceita `Optional`, `Annotated` e `Union`, `:96-111`).
- **Tipos "inferidos"**, sem anotação (`settle`, `python_types.rs:260-277`; `type_of`, `:480-505`): o primeiro parâmetro de um método é a classe (`self`, `:524`); um nome atribuído a uma chamada recebe o **chamado** como tipo (`construtor`, `:120`, `:273`), desde que todas as atribuições do nome na função concordem (`None` não conta); `self.x` recebe o tipo de um campo da classe do mesmo arquivo (`campo`, `:493-501`). Uma anotação vence (`:261-266`).

**O que é dinâmico demais para o extrator** (verificado, pelo que o código não lê):
- `getattr`, `setattr` e nomes em texto: "names in macros, getattr or strings are not seen" (`mod.rs:436`, diz ser). Caso real: o dublê do teste do curl é posto no lugar de `pycurl.Curl` por `@patch("hc.lib.curl.pycurl.Curl")` e `mock.return_value = obj = FakeCurl()` (`hc/lib/tests/test_curl.py:47-49`). O caminho está num texto, e a troca é feita em tempo de execução; nada disso chega a uma ocorrência que ligue `pycurl.Curl` a `FakeCurl`;
- o que um decorador, uma metaclasse ou um `__getattr__` fazem: só se marca que existem (`dynamic`, `getattr`, `python_types.rs:315`, `:343`);
- import por `importlib` ou `__import__`, e o conteúdo de um `from x import *` (só fica a marca `star`, `python.rs:473-478`);
- tipos de `*args` e `**kwargs` (`python_types.rs:529-531`), de alvos de `for`, `with` e `match` (`:166-203`) e de qualquer genérico (acima).

**Também descartado:** a ocorrência escrita fora de qualquer símbolo (`graph.rs:320`). Caso real, `manage.py` do Healthchecks: as chamadas das linhas 7 (`os.environ.setdefault(..)`), 13 (`execute_from_command_line(..)`) e 16 (`logger.exception(..)`) não estão no grafo. Só ficaram as ocorrências das duas atribuições que viraram `variable` (`logger`, linha 11; `msg`, linha 15). Verificado por um `node` sobre o `graph-v2.json` do run v9 da cópia.

### D. Duas perguntas ligadas a decisões do usuário

#### D1. A marca de teste

**Como o código marca** (verificado):

| | Rust | Python |
|---|---|---|
| Critério | o símbolo tem o atributo `#[test]`, exatamente esse caminho de um segmento (`a.path().is_ident("test")`, `rust.rs:245`) | o nome começa com `test` ou `Test`, ou alguma classe em volta começa com `Test` (`python.rs:794-795`) |
| Caminho do arquivo | não conta | não conta |
| O que não marca | `#[cfg(test)]` (o módulo e os auxiliares dele), `#[tokio::test]` ou outro atributo com caminho, `#[bench]` | uma classe `…TestCase` cujo nome não começa com `Test`, o `setUp`, os auxiliares e os dublês |
| Texto de limite | "Test markers are observations (Rust #[test]; Python test*/Test* names), not evidence that tests ran" (`mod.rs:446`, diz ser) | idem |

Não funciona igual nas duas linguagens: em Rust a marca é um atributo escrito; em Python é uma convenção de nome (o comentário diz "A naming convention observed (pytest)", `python.rs:793`, diz ser).

**Casos reais** (cópia, edition de 18/11; a marca não muda entre o v8 e o v9, porque `rust.rs:245` e `python.rs:794-795` não dependem do perfil):

| | ray tracer | Healthchecks |
|---|---|---|
| Marcados | 214 (os 214 `#[test]` do código: 210 em `src`, 4 em `tests`) | 1.789, todos em arquivos de pasta ou nome de teste |
| Em código de teste, **sem marca** | os 48 `module` dos `#[cfg(test)]`; 6 funções: 2 auxiliares no nível do módulo (`tests_camera::create_default_world`, `src/camera.rs:291`; `tests_world::create_default_world`, `src/world.rs:146`) e 4 escritas dentro de uma função de teste | 469: 215 classes `…TestCase`, 139 `setUp`, 49 outros métodos, 25 campos, 29 variáveis, 7 funções e 5 classes auxiliares (`NullSink`, `SystemChecksCase`, dois `MockSocket` e o `FakeCurl`) |
| Fora de código de teste, com marca | 0 | 0 |

"Código de teste" aqui é critério meu, só para contar: módulo `tests*` no Rust; caminho com `tests/`, `test_*.py` ou `*_test(s).py` no Python.

**O dublê `FakeCurl`** (verificado, na edition da cópia): ele está em `hc/lib/tests/test_curl.py:13-43`, e **não** em `hc/lib/curl.py`, que é onde estão as chamadas que a regra liga a ele. A classe, os campos `opts` e `ip` e os métodos `__init__`, `setopt`, `perform`, `getinfo` e `close` saem todos com `is_test: false`. O nome não começa com `Test`, e a classe não está dentro de uma classe `Test…`. O nome do arquivo não conta.

**Onde a marca é lida** (`git grep -n is_test -- crates`, fora dos testes; verificado):

| Leitor | Onde | O que faz com ela |
|---|---|---|
| busca, `without_tests` | `search.rs:185-198` (`:191`) | com `exclude_tests`, tira o símbolo marcado **e o que está dentro dele**; os auxiliares de fora ficam (parte 5, F) |
| busca, pontos e ordem | `search.rs:249-250` | desconta 20 pontos; desempata contra o teste |
| busca, texto de limite | `search.rs:292` | "Test symbols marked by the extractor … were left out" |
| envelope | `envelope.rs:192`, `:196` | não conta como "escrita não seguida" a que está dentro de um teste |
| envelope | `envelope.rs:286` | quem chama é `test_caller`, e não `caller` |
| envelope | `envelope.rs:414` | papel `constraint` para o candidato marcado |
| evidence, lacuna | `crates/librarian-evidence/src/lacuna.rs:42` | deixa os marcados fora da resolução de nomes da lacuna |
| studio, Medir tudo | `instrument.rs:134` | não instrumenta o símbolo marcado (`"teste"`) |

**Não leem a marca:** a regra de ligação (`resolve.rs`), o grafo (`graph.rs`, `graph/v2.rs`, `graph/uses.rs`), o inventário (`inventory.rs`), o dono (`owner.rs`), o `prepare` e o vocabulário do léxico (`search.rs:55-68`; parte 5, MAP-O43). Os nós do grafo não levam o campo (`UsedNode`, parte 3, G). A página também não o usa (`grep is_test index.html` vazio).

→ O dublê não é marcado como teste, e a regra não leria a marca mesmo se fosse. Ver MAP-O64.

#### D2. O tipo do receptor

**O que o extrator grava, e onde** (verificado):
- **Na edition: nada.** Ela só tem fontes, símbolos, trechos e diagnósticos (`lib.rs:54-59`).
- **No grafo** (`graph-v2.json`): para cada receptor com tipo escrito, uma ocorrência `receiver_type` sobre o trecho do receptor, com o **texto do receptor** como rótulo (`graph.rs:323-325`) e, desde o v9, o `type_rule` (`graph.rs:326`). **O nome do tipo não é gravado no nó.** Ele só vira arestas `name_candidate` para os símbolos de tipo com esse nome, na mesma linguagem (`graph.rs:290-297`, `:341-349`): em Rust, `struct`, `enum`, `union`, `type` e `trait` (`rust.rs:159-161`); em Python, só `class` (`python.rs:88`). Um tipo que o projeto não declara (`Vec`, `f64`, `pycurl.Curl`) fica num nó **sem aresta**.
- **Nos fatos da regra**, relidos a cada pergunta: o nome escrito e se é referência (`rust.rs:768-774`; em Python, `python.rs:163`). É daí que a regra tira o `Vec` quando o grafo não tem candidata (`resolve.rs:723-729`).
- O `type_rule` do nó só chega à ligação quando ela sai com `basis: "type"` (parte 3, B).

**Quando grava** (verificado, por leitura):

| Regra (`type_rule`) | Rust (`type_of` e `rule_of`, `rust.rs:810-856`) | Python (`type_of`, `python_types.rs:480-505`) |
|---|---|---|
| `self` | `self` num `impl` (`:814`, `:846`) | o primeiro parâmetro de um método sem `@staticmethod` nem `@classmethod` (`:524`) |
| `parametro` | parâmetro anotado da função (`:720-728`) | parâmetro anotado (`:521-522`) |
| `anotacao` | `let x: T = ..` (`:1118`) | `x: T = ..` na função ou no módulo (`:164`) |
| `construtor` | `T { .. }` (`:833-836`, `:853`) | **qualquer** `x = f(..)`, com o chamado como tipo escrito, e `f(..).m()` (`:120`, `:273`, `:502`) |
| `caminho` | `T::f(..)`, com o **dono do caminho** como tipo, se começa com maiúscula (`:827-831`, `:851`) | — |
| `retorno` | `f(..)` de uma função livre do mesmo arquivo, pelo retorno escrito (`:824-826`, `:850`) | — |
| `campo` | `self.f`, com `f` declarado no mesmo arquivo (`:819`, `:848`) | `self.x` ou `a.x` de uma classe do mesmo arquivo (`:493-501`) |
| `item` | variável de um `for` sobre um campo-coleção do mesmo arquivo (`:1019-1031`) | — |
| `variante` | `T::V(x)` de um `enum` do mesmo arquivo (`:1000-1009`) | — |

Em Rust, a última ligação de um nome vale para a função inteira, e blocos não são escopos (`rust.rs:751-765`; texto de limite em `mod.rs:440`, diz ser).

**Quando não grava** (verificado, por leitura; os casos reais abaixo): receptor que é resultado de chamada de método (`a.b().c()`; o `type_of` do Rust não tem ramo para `MethodCall`, `rust.rs:810-838`); índice (`v[i].m()`); literal (`"abc".len()`); expressão (`(a + b).m()`); parâmetro de closure, mesmo anotado (só os parâmetros de `fn` entram, `rust.rs:720-728`); campo de um tipo declarado em outro arquivo (`Shapes` é só do arquivo, `rust.rs:371-373`); e, em Python, alvos de `for`, `with` e `match` e anotações genéricas (seção C).

**Casos reais** (runs v9 da cópia; `node` sobre o `graph-v2.json`; script `d2.js` no scratchpad):

| Chamada | Como o receptor é escrito | O que fica gravado | Como a ligação sai |
|---|---|---|---|
| UQ-19, `ordered.len()`, `src/camera.rs:129` | `let mut ordered = timings.clone();` (`:125`). O `timings` tem tipo escrito, `Vec::with_capacity(samples)` (`:111`) | só o `method_call_observed`, com a aresta de nome para `impl Groups::len`. **Nenhum `receiver_type`.** O `timings` tem: `receiver_type` com `caminho` nas linhas 121 e 125, sem aresta (o `Vec` não é do projeto). O tipo se perde num salto: `.clone()` é chamada de método | `unique`, `graph_edge` (parte 3, E) |
| UQ-23, `k.min(4. - k).clamp(0., 1.)`, `src/bin/cena_video.rs:42` | `let k = (h * 6. + offset) % 6.;` (`:41`), dentro de uma closure `\|offset: f64\|` (`:40`); `h: f64` é parâmetro da função (`:39`) | dois `method_call_observed` (`k.min(..)`, sem candidata, e `.clamp(..)`, com a aresta para `impl Color::clamp`). **Nenhum `receiver_type`**: o receptor do `clamp` é resultado de chamada, e `k` vem de uma expressão | `unique`, `graph_edge` |
| `balls.len()`, `src/bin/cena_video.rs:107` | `let mut balls: Vec<(f64, f64, f64)> = vec![];` (`:105`) | `receiver_type` `balls`, regra `anotacao`, 0 arestas | descartada pelo tipo (`ruled_out` em `/api/used-by` de `Groups::len`) |
| `objects.len()`, `src/obj.rs:124` | parâmetro `mut objects: Vec<Object>` (`:123`) | `receiver_type` `objects`, `parametro`, 0 arestas | descartada |
| `face.len()`, `src/obj.rs:90` | parâmetro de closure `\|face: &Vec<..>\|` | **nada** | `unique` → `Groups::len` (um dos usos falsos da UB-15, parte 3, E) |
| `"librarian-measure cpu_ms=".len()`, `src/count.rs:133` | literal | **nada** | `unique` → `Groups::len` |
| HC-27 e UQ-4, `c.getinfo(..)` e `c.setopt(..)`, `hc/lib/curl.py:203` e `:139` | `c = pycurl.Curl()` (`:138`) | `receiver_type` `c`, regra `construtor`, 0 arestas (o projeto não tem classe `Curl`). O tipo escrito, `pycurl.Curl`, só existe nos fatos relidos da regra | `unique`, `graph_edge`, sem `type_rule` (`/api/calls` da função `request`, na cópia). O `Curl` de fora não descarta: o descarte por tipo de fora só vale para método de `impl` (`owner_of`, `resolve.rs:620-627`; `outside_rules_out`, `:735-740`; parte 3, F) |

As contagens de `/api/used-by` de `impl Groups::len` na cópia: `linked 4` (`face`, os dois literais e `ordered`) e `ruled_out 8` (três `balls.len()`, dois `objects.len()`, `self.children.len()`, `self.inverses.len()` e `self.objects.len()`, todos com `receiver_type` sem aresta).

**Quanto o receptor tem tipo gravado**, por chamada de método (runs v9 da cópia; script `d2b.js`, que casa cada chamada com o `receiver_type` do receptor inteiro, como `receiver_type_node` faz, `resolve.rs:632-637`):

| | ray tracer | Healthchecks |
|---|---:|---:|
| Chamadas de método | 1.044 | 11.806 |
| Com `receiver_type` | 503 | 5.459 |
| … por regra | `caminho` 246, `anotacao` 63, `campo` 50, `self` 49, `parametro` 43, `variante` 27, `retorno` 17, `item` 8 | `self` 4.159, `construtor` 1.093, `parametro` 186, `campo` 16, `anotacao` 5 |
| … com tipo que o projeto não declara (0 arestas) | 138 | 865 (`construtor` 698, `parametro` 147, `campo` 15, `anotacao` 5) |
| **Sem `receiver_type`** | **541** | **6.347** |
| … por forma do receptor | resultado de chamada 244, nome 153, atributo ou campo 84, índice 55, outro 5 | atributo 4.333, nome 1.915, índice 44, outro 36, resultado de chamada 19 |

Os 698 `construtor` do Healthchecks sem classe do projeto mostram o que essa regra grava. O texto escrito antes do `(` vira o "tipo": `super` em 333, `get_object_or_404` em 27, `self.client.get` em 22, `pycurl.Curl` em 18, `self.get` em 17, `set` em 17, `logging.getLogger` em 12, `Check.objects.create` em 11… (script `d2c.js`, que procura a atribuição do receptor no fonte). São funções e métodos, não classes. Ver MAP-O66.

→ **A resposta à pergunta.** Para um receptor de tipo da linguagem ou de biblioteca que **não está escrito** (`ordered`, `k.min(..)`, `face`, um literal), o extrator não grava nada sobre o tipo. Fica só a chamada, com o nome do método como candidato. O grafo não distingue "tipo desconhecido" de "tipo não procurado". Quando o tipo **está escrito** mas é de fora (`balls: Vec<..>`, `c = pycurl.Curl()`), fica um nó `receiver_type` sem aresta e com o `type_rule`; o nome do tipo só existe nos bytes, relidos pela regra.

---

## Execução (Parada 1)

Tudo no Git Bash. `S` é o scratchpad da sessão.

1. **Parada 0:** os quatro relatórios copiados para `S/orig` antes de editar; o diff contra a cópia mudou só a linha 3 de cada um; commit `66efcbf` ("docs: mapeamento, relatorios 02 a 05 validados"), push.
2. **Original antes**, pelo comando de `docs/reports/piloto-1b/dados/p1/original-before.sha256-agregado.txt` (`cd .studio && find projects/<key> -type f -print0 | xargs -0 sha256sum | sed 's/ \*/  /' | LC_ALL=C sort -k2 | sha256sum`): healthchecks `61cd25c1…5ca3d5` (932 arquivos) e renderer `612cdbf6…f3ecc` (42), os da Parada 0 do 03.
3. **Cópia:** `cp -r .studio/projects/* $S/ext/root/.studio/projects/`.
4. **Binários:** `cargo build --release --locked -p librarian-studio -p librarian-ingest` (só `librarian-evidence` e `librarian-studio` recompilaram; o `librarian_ingest.exe` já estava em dia). `git status --porcelain -uall` vazio depois.
5. **Studio sobre a cópia:** `LIBRARIAN_ROOT=$S/ext/root target/release/librarian_studio.exe --port 8793`. O log confirmou "Dados: …\scratchpad\ext\root\.studio". Pedidos: `POST /api/generate` dos dois projetos (os runs v9), `GET /api/used-by` de `impl Groups::len` e `POST /api/calls` da função `request` de `hc/lib/curl.py`. Os projetos só foram lidos (`git status --porcelain` vazio nos dois, nos commits `6c05a7a` e `e38254b`).
6. **Sintético do item A**, montado em `$S/ext/root/sintetico` (dentro da pasta da cópia, fora de qualquer projeto): `src/ok.rs`, `src/quebrado.rs`, `src/quebrado.py`, `src/latin.py` (bytes não UTF-8), `src/.oculta/h.rs`, `src/target/t.rs`, `src/node_modules/n.rs`, `src/leia.txt`, `src/grande.rs`, `src/novo.py`, `src/generico.py` e `docs/README.md`, com `inputs: ["src", "docs/README.md"]`. Rodado por `librarian_ingest generate`, `verify` e `verify <edition> <raiz>`. A edition deu 9 fontes e 3 diagnósticos. `target/t.rs`, `node_modules/n.rs` e `leia.txt` ficaram fora. O `verify` com a raiz apontou `src/grande.rs` como `changed`, porque o arquivo foi reescrito entre as medições de tamanho.
7. **Scripts** (só leem): `scripts/d1.js`, `d2.js`, `d2b.js`, `d2c.js` no scratchpad, com as respostas guardadas `calls-request.json` e `usedby-len.json`.
8. **Fim:** o studio da cópia foi encerrado. O hash do original foi refeito: os mesmos dois valores e contagens do passo 2; `ls .studio/projects/*/runs/` só mostra os runs de 18/11. A cópia, com os runs novos e o sintético, foi apagada (`rm -rf $S/ext`).

---

## Observações novas (Parada 1)

- **MAP-O58.** O extrator roda em quatro momentos: `extract` na geração; `extract` de novo em todo `Catalog::load`, para conferir; `observe` ao montar o grafo; e `call_facts` a cada pergunta da regra. Só o primeiro grava na edition (seção A). As ocorrências só existem no `graph-v2.json` do studio, e os fatos da regra nunca são gravados. Como `/api/calls` e o `prepare` chamam `Catalog::load`, a extração inteira é refeita a cada um desses pedidos.
- **MAP-O59.** A linguagem é decidida só pela extensão (`rs`, `py`; `mod.rs:427`, `:478-485`). Dentro de uma pasta, o resto nem é lido; como entrada explícita, vira `auxiliary`, guardado e sem extração. No Healthchecks, as pastas `static` e `templates` entram, mas só os `.py` delas.
- **MAP-O60.** As exclusões são só os 15 nomes de pasta de `skipped_dirs`, em qualquer nível, e nunca numa entrada explícita (`mod.rs:459-462`, `lib.rs:228-231`). Pasta oculta fora da lista é percorrida (verificado com `src/.oculta`), e o `.gitignore` não é lido. Não há limite de tamanho. No sintético, o tempo de `generate` e `verify` cresceu cerca de 4 vezes a cada vez que o arquivo dobrou: 970 KB levaram 11,9 s, e 3,7 MB passaram de 120 s.
- **MAP-O61.** Um erro de sintaxe ou de UTF-8 deixa o **arquivo inteiro** sem símbolos e sem ocorrências; ficam os bytes e um diagnóstico (seção A). A geração não falha: a CLI sai com 2, o studio grava o run, e só o `prepare` recusa a edition (`prepare.rs:119-120`). Os quatro runs reais têm 0 diagnósticos.
- **MAP-O62.** O grafo descarta a ocorrência escrita fora de qualquer símbolo (`graph.rs:308-320`). Caso real: três chamadas de nível de módulo do `manage.py` do Healthchecks (linhas 7, 13 e 16) não estão no grafo.
- **MAP-O63.** Rust: o conteúdo das macros é opaco (`syn-3.0.5/src/gen/visit.rs:768`, `:2830-2838`), as `macro_rules!` do projeto não são símbolos e o `cfg` não é avaliado. No ray tracer: 568 invocações de macro, 3 macros do projeto sem símbolo e um módulo sob `#[cfg(all(test, feature = ..))]` extraído como os outros.
- **MAP-O64.** A marca de teste é por atributo no Rust (`#[test]`, `rust.rs:245`) e por nome no Python (`test*`/`Test*`, `python.rs:794-795`); o caminho nunca conta. Casos reais: no Healthchecks, 469 símbolos de código de teste sem marca (215 classes `…TestCase`, 139 `setUp`, o `FakeCurl` e outros); no ray tracer, os 48 módulos `#[cfg(test)]` e 6 funções. O `FakeCurl` (`hc/lib/tests/test_curl.py:13-43`) e os seus métodos saem com `is_test: false`. A marca é lida pela busca, pelo envelope, pela lacuna do evidence e pelo Medir tudo; não é lida pela regra, pelo grafo, pelo inventário, pelo dono, pelo `prepare` nem pelo léxico (seção D1).
- **MAP-O65.** O nome do tipo de um receptor não fica no grafo: o nó `receiver_type` tem o texto do receptor e o `type_rule`, e o tipo só aparece como aresta `name_candidate` para um tipo do projeto com esse nome (`graph.rs:290-349`). Um tipo de fora (`Vec`, `f64`, `pycurl.Curl`) fica num nó sem aresta, e a regra relê o nome dos bytes (`resolve.rs:723-729`). Nos runs v9: 138 receptores assim no ray tracer, 865 no Healthchecks.
- **MAP-O66.** Homônimo: o `type_rule` `construtor` quer dizer coisas diferentes nas duas linguagens. Em Rust, é `T { .. }` (`rust.rs:853`). Em Python, é o chamado de **qualquer** chamada atribuída ao receptor (`python_types.rs:120`, `:273`). Caso real: dos 1.093 `construtor` do Healthchecks, 698 não têm classe do projeto, e os mais comuns são `super` (333), `get_object_or_404`, `self.client.get`, `pycurl.Curl`, `set` e `logging.getLogger`. Também o `caminho` do Rust grava o **dono** do caminho (`Vec` em `Vec::with_capacity`), e não o que a função devolve (`rust.rs:827-831`). *Leitura minha:* o nome de regra diz de onde o texto veio, não que ele é o tipo do valor.
- **MAP-O67.** O tipo escrito se perde num salto: `let ordered = timings.clone()` não herda o `Vec` de `timings` (`src/camera.rs:111`, `:125`, `:129`), porque o extrator não lê o tipo de uma chamada de método. Também não lê o tipo de parâmetro de closure, de literal, de índice nem de expressão. Sem `receiver_type`: 541 de 1.044 chamadas de método no ray tracer e 6.347 de 11.806 no Healthchecks. Três dos quatro usos falsos de `Groups::len` (UB-15) e o UQ-19 e o UQ-23 são desses casos.
- **MAP-O68.** Diz ser × faz: o comentário de `Config.inputs` diz "Directories contribute .rs files recursively" (`lib.rs:65`), mas, do v2 em diante, as pastas também trazem os `.py` (`mod.rs:427`).
- **MAP-O69.** Os runs gravados na `.studio` (18/11) são do perfil **v8**, e uma edition é sempre lida com o perfil que a gerou (`lib.rs:431-434`). Neles, o Healthchecks não tem nenhum `receiver_type`, e nenhum nó tem `type_rule` (ray tracer: 1.382 `receiver_type`, 0 com `type_rule`), contados nos dois `graph-v2.json`. Uma pergunta feita na página sobre esses runs usa o v8. O portão e esta parte geram runs novos, com o v9.

## Perguntas (Parada 1)

- **MAP-Q20.** Os runs que a `.studio` guarda são do perfil v8 (MAP-O69), e os que o portão mede são gerados na hora, com o v9. Para o mapa, "o estado de hoje" do extrator é o v9, que o código gera, ou o v8, que está nos runs que a pessoa abre na página? A Parada 2 (edition e fronteira) muda conforme a resposta, porque os campos produzidos não são os mesmos.

---

*Parada 2 (a edition, a fronteira, os testes, as dependências, os arquivos não mapeados e o anexo da busca): aguardando liberação.*

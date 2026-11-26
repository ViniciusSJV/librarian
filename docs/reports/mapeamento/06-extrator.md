# Mapeamento do repositório — parte 6: o extrator

**Estado:** Pendente de validação do usuário: Parada 2 (E a I, ajustes 2 a 4 e anexo); Parada 1 avaliada (MAP-Q20 respondida).

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
  - **Resposta do usuário (26/11/2026):** o estado de hoje para o mapa é o v9, o que o código gera. Os runs v8 da `.studio` entram como observação: a página abre um perfil e o portão mede outro (MAP-O69). Nada é regerado na `.studio` original.

---

## Parada 2

**Base:** `main` em `52292f4`. As mesmas regras da Parada 1. A execução foi feita sobre uma **nova cópia** da `.studio`, no scratchpad, com dois runs v9 novos de cada projeto (ray tracer `20261126-225045` e `-225054`; Healthchecks `20261126-225046` e `-225055`). Rotas usadas no studio da cópia: `/api/generate`, `/api/calls`, `/api/used-by` e `/api/graph`. Também a CLI (`generate`, `verify`, `verify-graph` e `search`) sobre editions da cópia e sobre sintéticos montados dentro dela. Nenhum LLM foi chamado (seção "Execução (Parada 2)").

Caminhos abreviados novos: `v2.rs` é `crates/librarian-ingest/src/graph/v2.rs`; `uses.rs` é `crates/librarian-ingest/src/graph/uses.rs`; `index.html` é `crates/librarian-studio/web/index.html`.

### E. A edition

**Os arquivos no disco** (verificado, `lib.rs:54-59`, `:408-419`): uma pasta nova, com `snapshots/` e cinco arquivos.

| Arquivo | O que tem | Campos (todos com `deny_unknown_fields`) |
|---|---|---|
| `snapshots/<sha256>` | os bytes de cada fonte, como lidos, com o hash do conteúdo como nome (`lib.rs:410-412`) | — |
| `sources.jsonl` | uma fonte por linha | `id`, `path`, `sha256`, `byte_len`, `language` (`lib.rs:110-118`) |
| `symbols.jsonl` | um símbolo por linha | `id`, `source_id`, `name`, `qualified_name`, `kind`, `is_test`, `chunk_id` (`lib.rs:120-131`) |
| `chunks.jsonl` | um trecho por símbolo | `id`, `source_id`, `start_byte`, `end_byte` (exclusivo), `start_line`, `end_line` (inclusivo), `sha256` (`lib.rs:133-144`) |
| `diagnostics.json` | lista JSON | `source_id`, `code`, `message` (`lib.rs:146-152`) |
| `manifest.json` | gravado **por último** (`lib.rs:416-419`; comentário em `:305-306`, diz ser) | `schema_version`, `extractor` (o perfil), `config`, `git` (`head`, `dirty`), `files` (o hash de cada um dos 4 registros), `source_count`, `symbol_count`, `diagnostic_count`, `limits` (`lib.rs:69-82`) |

**Versões** (verificado):
- do formato: `SCHEMA_VERSION = 1` (`lib.rs:24`), conferido no `load` (`lib.rs:433`);
- do extrator: 9 perfis, de `EXTRACTOR` a `EXTRACTOR_V9` (`lib.rs:25-53`), cada um com a sua lista de limites (`mod.rs:126-465`). O perfil novo é sempre o v9 (`mod.rs:469-471`). Uma edition só é lida pelo perfil que a gerou, com os limites iguais (`lib.rs:431-434`): "editions are never reinterpreted" (`mod.rs:104`, diz ser);
- do grafo: cada perfil grava um `graph_version` (no v9, `rust-python-syntax-graph-v6`, `mod.rs:452`). O `graph-v2.json` do studio leva `version: "syntax-graph-v2"` (`v2.rs:9`), o `derived_from` (essa versão do perfil), o `profile` e o `catalog_sha256` (`v2.rs:49-57`).

**Os hashes** (verificado):
- `source.sha256` é o hash do conteúdo do arquivo, e o snapshot tem esse nome (`lib.rs:268-272`);
- `chunk.sha256` é o hash dos bytes do trecho (`rust.rs:255`, `python.rs:812`);
- `manifest.files` guarda o hash de cada registro (`lib.rs:382-386`);
- o `catalog_sha256` do grafo é o hash do `manifest.json` inteiro (`graph.rs:68-71`);
- o id de cada aresta do grafo v2 é o hash dos outros campos dela (`v2.rs:126-130`).

Nada assina o `manifest.json`: "Hashes establish consistency, not authenticity; manifest is not signed" (`mod.rs:450`; `lib.rs:1`, diz ser).

**Como a imutabilidade é garantida** (verificado):
- **Na escrita:** a pasta é criada com `fs::create_dir`, que falha se ela já existe (`lib.rs:408`). Cada arquivo é aberto com `create_new` e `sync_all` (`write_new`, `lib.rs:295-303`). Nenhuma função do crate regrava uma edition.
- **Na leitura:** todo `Catalog::load` confere, nesta ordem: o perfil e os limites (`lib.rs:431-434`); que os registros são exatamente os quatro (`:435-443`); o hash de cada registro (`:445-451`); as contagens (`:456-462`); cada snapshot contra a sua fonte, refazendo o `source()` (`:469-482`); e a **extração inteira de novo**, comparada com os registros (`:486-502`).
- **O que não é garantido** (por leitura): quem reescrever de modo coerente snapshot, registros e `manifest.json` não é percebido, porque o `manifest.json` não tem assinatura. Uma falha de I/O no meio deixa uma pasta incompleta (comentário em `lib.rs:306`, diz ser); como o `manifest.json` vem por último, o `load` dessa pasta falha.

**Por execução, numa cópia de uma edition do ray tracer:**
- um nome trocado no `symbols.jsonl` → `verify` recusa: "Record hash mismatch: symbols.jsonl";
- o mesmo, com o hash do `manifest.json` recalculado → `verify` recusa: "Extraction records do not match the preserved source bytes".

**O grafo não tem a mesma garantia** (verificado). O `graph-v2.json` não faz parte da edition. O studio o grava com `fs::write`, que sobrescreve (`store.rs:351`), e o confere **uma vez**, logo depois (`store.rs:352`). Depois, `Store::graph` e o cache de `main.rs` só leem e desserializam (`store.rs:371-373`, `main.rs:283-294`). Por execução, na cópia: um rótulo trocado no `graph-v2.json` de um run faz a CLI `verify-graph` recusar ("Graph does not match verified snapshots and graph version"). Mas o `/api/graph` do studio devolveu o rótulo trocado. Ver MAP-O70.

**Como os ids são formados** (verificado):

| Id | Forma | Onde |
|---|---|---|
| fonte | `src:{sha256 do caminho}:{sha256 do conteúdo}` | `lib.rs:271` |
| símbolo e trecho (o mesmo texto) | `{id da fonte}:{kind}:{byte inicial}:{byte final}` | `rust.rs:235`, `python.rs:790`; `chunk_id` é o mesmo id |
| ocorrência (só no grafo) | `{id da fonte}:occurrence:{kind}:{início}:{fim}` | `graph.rs:321` |
| aresta (grafo v2) | `edge:{hash dos campos}` | `v2.rs:126-130` |

**Se mudam entre duas gerações do mesmo código** (verificado, por execução na cópia):
- **Mesmo código, mesmo perfil: não mudam.** Duas gerações v9 de cada projeto deram `sources.jsonl`, `symbols.jsonl`, `chunks.jsonl`, `diagnostics.json`, `manifest.json` e snapshots **byte a byte iguais**. O `graph_sha256` também é o mesmo nas duas e no run v9 da Parada 1 (ray tracer `e449a475…`, Healthchecks `239c8e55…`).
- **Mesmo código, perfil v8 (18/11) × v9 (hoje):** os quatro registros e os snapshots são iguais. Só o `manifest.json` muda: o `extractor` e duas linhas de limite novas do v9. Os ids dos símbolos são os mesmos; o grafo não, porque as ocorrências mudam com o perfil (Parada 1, MAP-O69).
- **Um byte mudado num arquivo muda o id da fonte, e com ela o id de todo símbolo do arquivo.** Sintético na cópia: com uma linha de comentário acrescentada no topo de `a.rs`, `a` foi de `src:bdfc5619…:83f8dd0f…:function:0:9` para `src:bdfc5619…:58d6c99d…:function:14:23`, e `b` também mudou. O símbolo `c`, de `b.py`, ficou com o mesmo id. Leitura minha, pelo formato: renomear ou mover o arquivo também muda todos os ids dele (o caminho entra no hash).

### F. A fronteira extrator → resto

**O que o extrator produz:** os campos dos quatro registros (seção E); as ocorrências (`kind`, `start`, `end`, `candidate`, `label_end`, `rule`; `mod.rs:17-29`), que só o grafo consome; e os fatos da regra (`CallFacts`, 12 campos, `resolve.rs:42-77`), que só a regra consome.

**Método.** `git grep` pelo acesso a cada campo (`.campo`) e pelas funções do catálogo (`sources()`, `symbols()`, `chunks()`, `diagnostics()`, `manifest()`, `source_by_path`, `symbols_named`, `chunk`, `excerpt`, `source_bytes`, `call_syntax`, `owner_context` e os campos `catalog.*`) em `crates/*/src`, fora dos módulos `#[cfg(test)]` e de `tests/`. Script `campos.js` no scratchpad; a saída inteira em `campos.txt`. **O que o método não pega:**
- os campos de nome genérico (`id`, `path`, `name`, `kind`, `sha256`, `code`, `source_id`, `rule`), que existem em outros tipos (homônimos). Para eles valem só os acessos pelas funções do catálogo, abaixo;
- a leitura por comparação de struct inteira (`source(..) != s`, `lib.rs:480`; `symbols != expected_symbols`, `:497`), por desserialização e por desestruturação (`graph.rs:276-283`).

**Quem lê, por leitor** (só a lista, `ARQUIVO:linha` da primeira leitura de cada coisa):

| Leitor | O que lê do extrator |
|---|---|
| resolve | não lê o catálogo. Lê o grafo (`Resolver::new`, `resolve.rs:458-571`) e os fatos da regra por `syntax.facts` (`resolve.rs:471`, `file_facts` `:606`): `scopes` `:750`, `decorators` `:616`, `receiver_types` `:635`, `bound_calls` `:918`, `returns` `:969`, `classes` `:1002`, `field_receivers` `:999`, `cls_receivers` `:1215`, `methods` `:1188`, `modules` `:1910`, `blocks` `:2155`, `variables` `:1800`. Todos os 12 campos são lidos |
| graph (`graph.rs`) | `manifest` inteiro, para o hash `:68-71`; `registry` (versão, teto) `:68`; fontes `:75`; símbolos (`kind`, `qualified_name`) `:78`; trechos `:84`; snapshots e `observe` `:119`; `symbols_named` (`name`, `kind`, linguagem) `:292`; as ocorrências inteiras, por desestruturação `:276-283` |
| graph v2 (`v2.rs`) | fontes `:63`; símbolos `:76`; `manifest().extractor` `:111` |
| used_by (`uses.rs`) | não lê o catálogo; só o grafo e o `call_syntax` (pela regra) |
| search | símbolos (`name`) `:57`; fonte e linguagem `:152`; `excerpt` `:161`; `qualified_name` e caminho `:166-167`; trechos `:187-188`; `is_test` `:191`, `:249`; `diagnostics.len()` `:290`; `registry.search_limits` `:291` |
| inventory | `call_syntax` `:373`; `symbols()` `:383`; `manifest().extractor` `:677`; snapshots `:790` |
| envelope | `source_by_path` `:158`; trechos `:163`; símbolos `:173`; `registry` `:190`; `is_test` `:192`; `call_syntax` `:218` |
| owner | `registry` `:137`; símbolos `:145`; fontes `:146`; trechos `:234`; `symbols_named` `:360`; `call_syntax` `:397` |
| evidence (ingest, `evidence.rs`) | não lê o catálogo pelas funções acima (`grep` vazio); lê os JSON gravados do pacote |
| evidence (crate `librarian-evidence`) | `lacuna.rs:42` (símbolos, `is_test`); `selection.rs:62-69` (`source_by_path`, `source_bytes`, símbolos, trechos, fontes); `package.rs:137` (`manifest`), `:176-177`, `:229-231` (`extractor`, `git.head`, `git.dirty`); `check.rs:171` |
| prepare | `diagnostics` `:119`; `manifest.extractor` `:186`; símbolos `:191`; fontes `:192`; trechos `:201`; `git.head` `:230`; `source_by_path` `:251`; `registry.search_limits` `:447`; snapshots `:585` |
| studio | `store.rs:345-360` (no `generate`: `git` e as três contagens do manifest), `:485` (símbolos), `:773-775` (trechos e fontes), `:858` (`owner_context`); `main.rs:143` (`call_syntax`), `:468` (símbolos); `instrument.rs:64-201`; `lexicon.rs:177-182`; `questions.rs:77-78`; `claims.rs:109-130`; `execution.rs:279-299`, `:438`, `:498`; `facts.rs:116`, `:275` |

**Campos produzidos que ninguém lê** fora das conferências do próprio `lib.rs` (verificado pelo método acima, com a busca dos homônimos lida linha a linha):

| Campo | Única leitura | Observação |
|---|---|---|
| `Source.byte_len` | a comparação da fonte inteira no `load` (`lib.rs:480`) | nenhum leitor pelo nome |
| `Diagnostic.code` e `Diagnostic.message` | nenhuma | os 4 acertos de `.code` são de outros tipos (`envelope.rs:327`, `evidence.rs:2249`, `models.rs:164`, `store.rs:1182`); `.message`: 0. Os leitores só contam os diagnósticos ou perguntam se há algum (`cli.rs:17`, `prepare.rs:119`, `:399-401`, `search.rs:290`, `store.rs:360`) |
| `Manifest.schema_version` | `lib.rs:433` | os outros acertos são de outros tipos (`evidence.rs:1716`, `claims.rs:155`) |
| `Manifest.config` | `lib.rs:430` e `:618` | o studio guarda `inputs` no `run.json` pelos `Settings`, não pelo manifest (`store.rs:356`) |
| `Manifest.files` | `lib.rs:436-451` | os acertos de `check.rs:196` e `store.rs:1142` são outro `files` |
| `Manifest.limits` | `lib.rs:433` | nenhum texto de limite do manifest vai à busca, à consulta ou à página (ver o ajuste 3) |

As ocorrências `candidate` e `label_end` dão 0 pelo método, mas são lidas por desestruturação em `graph.rs:280-281`, `:290`, `:322`.

### G. Os testes do extrator e da edition

Os 9 arquivos de `crates/librarian-ingest/tests/` cujo assunto é a extração e a edition (primeira linha de cada um, diz ser, e os nomes dos testes). Os testes da regra, que também geram editions, estão na parte 3, H.

| Arquivo | Testes | O que verifica | Fixtures e sintéticos |
|---|---:|---|---|
| `editions.rs` | 11 | tokens e busca lexical; o grafo deixa chamadas sem resolução e tipos como candidatas; trechos em UTF-8 e corte; trechos exatos com Unicode, CRLF, BOM e `#!` (`:171`); registros e snapshots reprodutíveis, entradas repetidas uma vez só (`:198`); conferência histórica e detecção de adulteração (`:232`); intervalos forjados com hash recalculado (`:255`); falha de sintaxe e de codificação guardadas e contadas (`:273`); caminhos perigosos, edition incompleta e destino dentro da entrada (`:286`); `cfg` observado e macro não expandida: `#[cfg(any())] fn disabled` entra, o `fn generated` de dentro de uma macro não (`:308-321`) | projetos escritos pelo teste numa pasta temporária |
| `python.rs` | 4 | declarações com decoradores e escopos; ocorrências e candidatas só na mesma linguagem; método Python buscado e preparado como em Rust; `vocabulary` | idem |
| `v1_regression.rs` | 7 | a edition v1 congelada ainda é aceita; regerar o projeto da fixture reproduz cada registro e snapshot; descoberta e classificação do v1 congeladas; grafo e buscas iguais aos congelados; cópias adulteradas recusadas | `tests/fixtures/v1/` (projeto, edition, `graph.json`, buscas, léxico), lido em `:27`. "These fixtures are never regenerated to make a test pass" (`:2`, diz ser) |
| `graph_v2.rs` | 5 | o grafo v2 igual ao congelado e conferido; projeta o v1 sem criar relação; categorias e linguagens do catálogo; ids de aresta únicos e recalculáveis; relações forjadas e campos desconhecidos recusados | `tests/fixtures/v1/graph-v2.json` (`:10`) |
| `g6.rs` | 4 | campos são símbolos nas duas linguagens; acessos, chamadas e bases são candidatas, nunca resoluções; homônimos demais guardam só a contagem; o `used_by` pelos nomes, em camadas | temporários |
| `g6d.rs` | 5 | literais, escritas e tipo do `impl` como ocorrências; o contexto do dono (Rust e Python); recusa onde não dá para dizer; o que o alvo chama sem ambiguidade | temporários |
| `v5.rs` | 4 | variáveis de módulo Python e seus usos; laços e `with` cobrem o corpo e têm a cabeça como rótulo; variantes, valores e padrões em Rust; o perfil v4 ainda reproduzido | temporários |
| `v7.rs` | 3 | closures de cadeia de iteradores como laços e `receiver_type` gravado; o inventário pelos homônimos com tipo escrito; o perfil v6 sem nada disso | temporários |
| `v8.rs` | 1 | campo escrito por um receptor tipado é escritor do seu dono | temporários |

Nenhum deles lê `tools/precisao/sinteticos/` (`grep -n sinteticos` vazio nos 9).

**Execução** (verificado):

```
cargo test -p librarian-ingest --locked --test editions --test python --test v1_regression \
  --test graph_v2 --test g6 --test g6d --test v5 --test v7 --test v8
```

O exit foi 0, com **44 testes aprovados**, 0 falhas e 0 ignorados (log `testes.log` no scratchpad). Por binário: `editions` 11, `g6` 4, `g6d` 5, `graph_v2` 5, `python` 4, `v1_regression` 7, `v5` 4, `v7` 3, `v8` 1. `git status --porcelain -uall` vazio depois.

### H. As dependências externas

**Onde cada uma entra** (verificado, `git grep` pelo nome em `crates/*/src` e `tests`):

| Crate | Versão (`Cargo.toml` do ingest; `Cargo.lock`) | Onde é usado |
|---|---|---|
| `syn` | `=3.0.5`, com `full` e `visit` (`Cargo.toml:11`); `Cargo.lock:914-917`. O `Cargo.lock` também tem um `syn 2.0.119` (`:903-906`), que não é dependência do ingest (`librarian-ingest` lista só `syn 3.0.5`) | só `rust.rs`: `parse` (`:213-218`), os tipos da árvore e o `Visit` (`:6-9`, todo o arquivo) |
| `proc-macro2` | `1`, com `span-locations` (`Cargo.toml:12`); travado em `1.0.107` (`Cargo.lock:661-662`) | só `rust.rs:5` (`Span`), para os deslocamentos em bytes de cada nó (`span.byte_range()`, ex. `:233`) |
| `rustpython-parser` | `=0.4.0` (`Cargo.toml:13`; `Cargo.lock:758-759`) | `python.rs:7`, `:659-662` (o arquivo inteiro) e `python_types.rs:86` (a anotação escrita como texto, `"T"`) |
| `rustpython-ast` | `=0.4.0` (`Cargo.toml:14`; `Cargo.lock:746-747`) | `python.rs:6` e `python_types.rs:6` (os tipos e o `Visitor`) |

O nome do perfil traz as versões (`syn/3.0.5;rustpython-parser/0.4.0`, `lib.rs:52-53`), mas é um texto fixo: o código não confere a versão do crate ligado. *Leitura minha:* se a versão mudasse e a árvore saísse diferente, o `load` das editions antigas recusaria, porque refaz a extração e compara (`lib.rs:486-502`).

**Sintaxe que a versão fixada não conhece:** o arquivo inteiro vira um diagnóstico, sem símbolos e sem ocorrências (seção A da Parada 1). Sem caminho intermediário: não há leitura parcial nem recuperação de erro no código (`rust.rs:51-54`, `python.rs:44-47`).

**Por execução** (sintético dentro da cópia, um arquivo por construção, gerado pela CLI):

| Construção | Resultado |
|---|---|
| Rust: `if let Some(x) = a && x > 0` (let chains) | lido |
| Rust: `impl Sized + use<T>` | lido |
| Rust: `unsafe extern "C" { pub fn h(); }` | lido; `h` não vira símbolo (bloco `extern`, Parada 1, B) |
| Rust: `&raw const x` | lido |
| Rust: `async \|\| {}` | lido |
| Rust: `gen { yield 1; }` | **diagnóstico**, `rust_parse_error`: "expected identifier or integer" |
| Python 3.11: `except* ValueError:` | lido |
| Python 3.12: `type Alias = list[int]` e `class C[T]:` | lido; `C` e `C.m` viram símbolos; o `type Alias` não |
| Python 3.12: aspas iguais dentro de f-string, `f"{d["k"]}"` | **diagnóstico**, `python_parse_error`: "Got unexpected token 'k'" |
| Python 3.13: default de parâmetro de tipo, `def f[T = int]` | **diagnóstico**: "unexpected token '='" |
| Python 3.14: `except ValueError, TypeError:` sem parênteses | **diagnóstico**: "unexpected token ','" |
| Python 3.14: t-string, `t"valor {x}"` | **diagnóstico**: "unexpected token \"valor {x}\"" |

Os dois projetos reais não têm nenhuma dessas construções que falham: 0 diagnósticos nos runs v9 da cópia. Ver MAP-O72.

### I. Arquivos de `crates/librarian-ingest/src` que as partes 3, 5 e 6 não mapearam

Só a lista, sem classificar (`ls` do `src`, contra o escopo de cada parte):
- `envelope.rs` (558 linhas): só citado nas partes 3, 5 e 6, onde lê ligações ou a marca de teste;
- `evidence.rs` (3.308): citado na parte 2 (o contrato do pacote) e só citado nas partes 5 e 6;
- `inventory.rs` (906): só citado;
- `owner.rs` (427): só citado;
- em parte: `prepare.rs` (a ponte, na parte 2; o ramo da busca e as configurações, na parte 5) e `cli.rs` (os comandos `search`, na parte 5, e `generate`/`verify`, aqui; `graph`, `verify-graph`, `prepare`, `verify-prepared`, `investigate inventory` e `show` não foram mapeados).

### Ajuste 2. As rotas do Healthchecks no grafo

**Verificado** no run v9 da cópia (`20261126-225046`), por `node` sobre o `graph-v2.json` (script `rotas.js`) e pelas rotas do studio da cópia.

**As chamadas `path(..)` e `include(..)` ficam dentro de um símbolo** quando estão numa lista atribuída no nível do módulo. Essa atribuição vira `variable` (Parada 1, C), e o trecho dela cobre a lista:

| Arquivo | Símbolos | `path`/`include` escritos | No grafo | Fora do grafo |
|---|---|---:|---:|---|
| `hc/urls.py` (raiz) | `variable prefix` (`:11`), `variable urlpatterns` (`:15-52`) | 70 (36 `path`, 34 `include`) | 70, todos sob `urlpatterns` | 0 |
| `hc/api/urls.py` (app) | 2 classes de conversor, com campos e métodos; `variable uuid_urls` (`:33-39`), `slug_urls` (`:41-46`), `api_urls` (`:48-72`), `urlpatterns` (`:74-98`) | 39 (34 `path`, 5 `include`) + 2 `register_converter` | 39 | os 2 `register_converter(..)` de `:30-31`, escritos no nível do módulo, fora de qualquer atribuição: descartados como os do `manage.py` (`graph.rs:308-320`) |

**A ligação da rota até a view não sai** (verificado):
1. **`path` e `include`:** `/api/calls` de `urlpatterns` (`hc/api/urls.py`) dá **0 ligações** e 15 chamadas sem ligação, todas `no_candidate_in_project`. São imports do Django.
2. **A view:** ela é escrita como valor, não como chamada: `views.ping`, `views.checks`, `accounts_views.login`. Isso vira `attribute_access`, cujas candidatas só podem ser **campos** (`python.rs:102`). Nos dois arquivos há 30 referências desse tipo a views (`hc/urls.py` 1, `hc/api/urls.py` 29), mais 2 a `admin.site`. **Nenhuma** tem aresta para uma função. As 9 arestas de nome que existem vão para campos homônimos: `views.checks` → `Channel.checks` (`hc/api/models.py:1030`); `views.channels` → `GroupForm.channels`, `Spec.channels` e `CheckDict.channels`; `views.status` → `ProjectStatus.status`, `CheckDict.status` e `Check.status`; e `admin.site` → `ZulipConf.site` e `AddZulipForm.site`.
3. **A lista de rotas do app:** `include("hc.api.urls")` só tem um texto (`hc/urls.py:19`); nada liga a raiz ao `urls.py` do app. Já `include(api_urls)`, com uma variável do mesmo arquivo, vira `name_reference` com aresta para a variável (5 arestas em `hc/api/urls.py`).
4. **Pelo outro lado:** `/api/used-by` da função `ping` de `hc/api/views.py` dá `linked 1` (uma chamada direta, `hc/api/views.py:300`) e `ruled_out 4`. Nenhuma das 6 rotas que a nomeiam (`views.ping`, `hc/api/urls.py:34-38` e `:75`) aparece.

### Ajuste 3. O que o extrator não vê: alguém avisa?

Para cada caso, o que o código faz para quem recebe a resposta. Os leitores possíveis são: a consulta que vai ao modelo, pelos limites da busca (`search.rs:291` → `prepare.rs:445-461` → `unknowns`; parte 5, G); as respostas de `/api/calls` e `/api/used-by`; e a página. Verificado por leitura.

| O que não é visto | Aviso | Onde |
|---|---|---|
| Conteúdo de macro (Rust) | **Só um texto genérico.** O limite de busca do v9, "No macros, cfg or decorator evaluation, compiler or runtime proof" (`mod.rs:455`), vai a toda consulta de uma pergunta buscada. Não diz qual chamada ficou de fora. O texto do manifest, "Rust: no macro expansion, cfg evaluation…" (`mod.rs:431`), fica só no `manifest.json`, lido só para conferir (`lib.rs:433`). `/api/calls` e `/api/used-by` não têm campo de limite (`Calls`, `Uses`; parte 3, G) | calado no caminho e nos usos |
| Ocorrência fora de símbolo | **Nenhum.** Nenhum texto de limite fala disso (`grep` nos limites de `mod.rs`), e a ocorrência não chega a nó nenhum (`graph.rs:320`) | calado |
| Arquivo com erro de sintaxe | **Só a contagem.** O `run.json` guarda `diagnostics` (`store.rs:360`), e a página mostra "{diagnostics} diagnósticos" (`index.html:855`, `:3445`). Na pergunta, o `prepare` recusa (`prepare.rs:119-120`), e a página diz "Query não preparada: a edition tem diagnósticos (arquivos que o parser não leu)" (`index.html:1345`, `:5449`). Com `accept_diagnostics`, a consulta leva "Edition com {n} diagnósticos: extração incompleta em algumas fontes" (`prepare.rs:399-401`). O relatório da busca guarda o número (`search.rs:290`), e a CLI sai com 2. **Nenhum lugar diz qual arquivo nem a mensagem**: `code` e `message` não são lidos (seção F). Em `/api/calls` e `/api/used-by`, nada | contado, sem dizer onde |
| `getattr` e nomes em texto (Python) | **Só no manifest.** "names in macros, getattr or strings are not seen" (`mod.rs:436`) é um limite do manifest, e não da busca (`mod.rs:453-458`). Não chega à consulta nem à página | calado para quem pergunta |
| Templates, `.js`, `.css` e outras extensões | **Nenhum.** Não viram fonte (Parada 1, A), nenhum texto de limite os cita, e o `compare_current` usa a mesma regra de descoberta da edition, então também não os aponta como novos (`lib.rs:616-618`) | calado |

Os textos de limite da busca também não aparecem na página, na vista da pergunta: os `limits` que a página mostra são os da conferência das claims, do Medir tudo, da performance, do inventário e dos comandos (`index.html:5860`, `:6302`, `:6403`, `:7312`, `:7393`). Os da busca ficam na consulta enviada ao modelo. Ver MAP-O71.

### Ajuste 4. `construtor` × ligações com `basis: "type"` (Python)

**Método** (verificado, por execução; script `construtor.js` no scratchpad, saída `construtor-hc.json`):
1. Na cópia, com o run v9 do Healthchecks (`20261126-225046`), pedi `/api/used-by` (profundidade 1) para **cada um dos 4.718 símbolos**. Juntei toda ligação com o próprio símbolo como alvo e `basis: "type"`. A resposta sempre traz as ligações ao próprio símbolo, mesmo quando o desenho é cortado pelo teto de nós (`uses.rs:384`). 51 respostas vieram com `truncated: true`, sem perda dessas ligações.
2. Para cada ligação, achei o `receiver_type` do receptor inteiro, como `receiver_type_node` faz (`resolve.rs:632-637`), e guardei o `type_rule` do nó.
3. Nas de regra `construtor`, achei o que foi chamado: o próprio receptor, quando ele é a chamada (`super()`), ou a atribuição do receptor no fonte, procurada para cima. E conferi se o último segmento do chamado é o nome de uma classe do projeto, e se o alvo está nessa classe.

**Resultado:**

| | Ligações |
|---|---:|
| Com `basis: "type"`, no run inteiro | 346 |
| … com o receptor de regra `construtor` | **235**, todas `resolution: "type"` |
| … das 235, o chamado é uma **classe do projeto**, e o alvo é um método dela | 89 (32 `SlackFields`, 27 `Command`, 9 `GoogleChatFields`, formulários…). Uma delas o script não achou e foi lida à mão: `t = context["__table__"] = Table()` (`hc/front/templatetags/asciitable.py:48`), alvo `Table.render` |
| … o chamado é `super` | 141, todas com o `type_rule` da ligação `super` (a cadeia de bases, 2.4b; parte 3, B) |
| … o chamado é uma **função ou um método** | **5**, todas com o `type_rule` da ligação `retorno` (2.3c): `curl.post(..)` → `Response.json` (2, `hc/integrations/slack/views.py:105`, `discord/views.py:71`; `def post(..) -> Response`, `hc/lib/curl.py:224-233`); `_get_rw_project_for_user(..)` → `Project.num_checks_available` (2, `hc/front/views.py:533`, `:1072`; `-> Project`, `:187`); `project.transfer_request(..)` → `Member.can_accept` (1, `hc/accounts/views.py:512`; `-> Member \| None`, `hc/accounts/models.py:525`) |

→ **Zero ligações a uma classe do projeto em que o chamado é uma função ou um método tratado como se fosse a classe.** As 5 em que o chamado é uma função ou um método passam pelo retorno anotado dela, e o alvo é um método da classe que a anotação nomeia. *O que o método não pega:* uma ligação de chamada com `basis` diferente de `type` (por exemplo `unique`, que a regra decide sem o tipo); e uma classe do projeto com o mesmo nome de uma função de fora (a conferência é pelo último segmento do nome).

### Anexo (parte 5). O corte de 60% da busca

**Como a cobertura é calculada** (verificado, `search.rs:208-245`):
1. Os termos são os tokens da **pergunta inteira**, sem as stop words (`:210-213`).
2. Cada termo vira o seu grupo de sinônimos, e termos do mesmo grupo contam uma vez (`:214-220`).
3. Para cada símbolo, `reasons` são os grupos que aparecem em algum dos quatro campos dele: nome, escopo, caminho ou código (`:225-241`).
4. O símbolo fica fora quando `reasons × 5 < grupos × 3` (`:243`), em que "grupos" são **todos os grupos da pergunta**, inclusive os que não batem em **nenhum** símbolo do projeto.

Não é sobre os termos que bateram em algum lugar: um termo que não existe no código entra no denominador e não pode entrar no numerador.

**Caso real** (cópia; CLI `librarian_ingest search <edition v9 do ray tracer> "<pergunta>" <léxico da cópia> 10 0`):

| Pergunta | Grupos (`terms`) | Mínimo para entrar | Hits (até 10) |
|---|---|---:|---|
| "como a câmera gera o raio e como o material reflete a cor" | `camera`, `cor`, `gera`, `material`, `raio`, `reflete` (6) | 4 | **1**: `main` de `src/bin/cena_video.rs:47` (404 pontos), com os 4 termos só no **código** do corpo |
| "como a câmera gera o raio" | `camera`, `gera`, `raio` (3) | 2 | 6: `impl Camera::ray_from_pixel` (`src/camera.rs:81`, 211), `impl Camera::render` (`:95`, 204), o mesmo `main` (202) e três testes `tests_camera::constructing_a_ray_…` (196, 191, 191) |
| "como o material reflete a cor" | `cor`, `material`, `reflete` (3) | 2 | 10: `Material::color` (`src/materials.rs:9`), `MaterialSpec::color` (`src/scene.rs:59`), `impl World::reflected_color` (`src/world.rs:65`), `impl World::refracted_color` (`:74`), `impl Material::phong`, `impl MaterialSpec::validate`, `impl Material::lighting`, dois `create_default_world` de teste e `impl World::shade_hit` |

Contas que explicam (verificado pela CLI, cada termo sozinho):
- `gera` e `reflete` dão `no_lexical_evidence`: não estão em nenhum símbolo, e o léxico do projeto não tem grupo para eles;
- `camera`, `raio`, `material` e `cor` dão hits sozinhos (o léxico os junta a `cameras`, `ray`, `materials` e `color`).

Na pergunta com os dois assuntos, há 6 grupos, e dois deles (`gera`, `reflete`) não batem em nada. Por isso um símbolo precisa dos outros 4. O `ray_from_pixel` (2 de 6) e os de material e cor (2 de 6) saem; fica só o `main`, que tem as quatro palavras no corpo. Cada assunto sozinho dá 2 de 3, que passa. Ver MAP-O73.

---

## Execução (Parada 2)

1. **Original antes:** os dois hashes e contagens da Parada 0 do 03 (healthchecks `61cd25c1…5ca3d5`, 932; renderer `612cdbf6…f3ecc`, 42).
2. **Cópia nova:** `cp -r .studio/projects/* $S/p2/root/.studio/projects/`; o 06 copiado para `S/orig/06-extrator.md` antes de editar.
3. **Studio sobre a cópia:** `LIBRARIAN_ROOT=$S/p2/root target/release/librarian_studio.exe --port 8794` (o binário da Parada 1). Pedidos: `/api/generate` duas vezes por projeto; `/api/calls` de `urlpatterns`; `/api/used-by` de `ping` e de todos os 4.718 símbolos do Healthchecks; `/api/graph` de um run do ray tracer com o `graph-v2.json` adulterado na cópia.
4. **CLI na cópia:** `verify` de uma cópia adulterada de uma edition; `verify-graph` do grafo adulterado; `generate` dos sintéticos de ids e de sintaxe; `search` das perguntas do anexo, com o léxico do `project.json` da cópia.
5. **Testes:** seção G.
6. **Scripts** (só leem, no scratchpad): `campos.js`, `rotas.js`, `construtor.js`, mais os da Parada 1; as saídas guardadas `campos.txt`, `construtor-hc.json` e `testes.log`.
7. **Fim:** o studio da cópia foi encerrado. O hash do original foi refeito: os mesmos dois valores e contagens do passo 1; `ls .studio/projects/*/runs/` só mostra os runs de 18/11. A cópia, com os runs novos e os sintéticos, foi apagada (`rm -rf $S/p2`). Os dois projetos continuam sem mudança (`git status --porcelain` vazio), nos commits `6c05a7a` e `e38254b`.

---

## Em palavras simples

*À parte das seções acima, sem números nem nomes de arquivo. Escrita com a ideia da skill "elementary school teacher" (nível newcomer).*

Pense de novo na biblioteca. O extrator é o funcionário que recebe os livros novos, tira uma cópia de cada um e escreve as fichas do catálogo.

**O que ele recebe.** Uma lista de pastas e arquivos. Ele percorre as pastas e só pega o que tem a terminação de Rust ou de Python. O resto da pasta ele nem olha: páginas da web, estilos, scripts de navegador e textos ficam de fora, sem aviso. Ele pula algumas pastas pelo nome (cópias de bibliotecas, caches), mas não lê a lista de "ignorados" do projeto e entra em pastas escondidas. Não há tamanho máximo, e um arquivo muito grande demora muito mais do que o dobro quando dobra de tamanho.

**O que ele escreve.** Para cada declaração (uma função, um tipo, um método, um campo), uma ficha com o nome, o lugar exato no arquivo e se parece ser um teste. Junto, guarda a cópia fiel de cada arquivo e uma "impressão digital" de tudo. Ele não muda o que já escreveu: cada catálogo novo vai para uma gaveta nova. E toda vez que alguém abre o catálogo, ele lê os livros de novo e confere se as fichas batem. Se alguém mexer numa ficha, ele percebe. Mas o catálogo não tem assinatura: quem refizesse tudo de forma coerente não seria percebido. O desenho do mapa, guardado ao lado, é conferido só no dia em que é feito; depois, a página o mostra como estiver.

**As fichas não mudam sozinhas.** O mesmo livro dá sempre as mesmas fichas, com os mesmos números. Mas basta mudar uma letra de um arquivo para mudar o número de todas as fichas daquele arquivo.

**O que ele não vê, e quase nunca avisa:**
- **O que está dentro de uma "macro"** do Rust, um atalho que escreve código por você. Para o extrator, é uma caixa fechada. O pedido enviado ao modelo leva uma frase geral sobre isso, mas ninguém diz qual chamada ficou de fora.
- **O que está escrito solto no arquivo**, fora de qualquer declaração. Nem entra no mapa, e ninguém avisa.
- **Um arquivo com um erro de escrita.** O arquivo inteiro fica sem fichas. A página diz quantos arquivos não foram lidos, mas não diz quais.
- **O que o Python decide na hora de rodar**, por nomes escritos dentro de textos ou trocados durante os testes. Por isso o dublê de teste da biblioteca de rede não é ligado à biblioteca de verdade. Isso só aparece numa nota que ninguém lê.
- **Construções novas das linguagens**, que as versões fixas dos leitores não conhecem. Os dois projetos de teste não usam nenhuma delas hoje.

**A marca de teste.** Em Rust, é uma etiqueta escrita no código. Em Python, é só o nome começar com "test". O lugar do arquivo não conta. Por isso muitos ajudantes de teste, e o próprio dublê, ficam como se fossem código de verdade.

**O tipo de quem recebe a chamada.** O extrator só anota o tipo quando ele está escrito por perto. Basta um passo a mais, como copiar a variável, para o tipo se perder. Então a regra de ligação fica só com o nome, e às vezes liga a chamada à única coisa do projeto que tem o mesmo nome, quando a de verdade é da linguagem.

**As rotas da web.** As listas de rotas entram no mapa, mas a ligação da rota à página que ela abre não sai: a página é citada pelo nome, sem ser chamada, e o extrator só sabe ligar esse tipo de citação a campos.

**A busca com dois assuntos.** Quando a pergunta junta dois assuntos e algumas palavras dela não existem no código, essas palavras contam contra. Cada parte do código precisa combinar com a maior parte da pergunta inteira, e quase nenhuma combina com os dois assuntos ao mesmo tempo. A busca acha cada assunto sozinho, mas não os dois juntos.

---

## Observações novas (Parada 2)

- **MAP-O70.** A edition é conferida inteira a cada `load`; o grafo do studio não. O `graph-v2.json` é gravado com `fs::write` e conferido uma vez, na geração (`store.rs:351-352`); depois, o studio só o lê (`store.rs:371-373`, `main.rs:283-294`). Na cópia, um rótulo adulterado foi recusado pela CLI `verify-graph` e devolvido pelo `/api/graph`. Também não há assinatura do `manifest.json`: a conferência é de coerência, como os próprios limites dizem (`mod.rs:450`).
- **MAP-O71.** O que o extrator não vê quase nunca chega a quem recebe a resposta (ajuste 3):
  - macro: só um texto genérico, na consulta ao modelo;
  - erro de sintaxe: só a contagem, sem o arquivo nem a mensagem, porque `Diagnostic.code` e `Diagnostic.message` não são lidos por ninguém (seção F);
  - ocorrência fora de símbolo, `getattr`/texto e arquivos de outras extensões: nada.

  Os limites do manifest, onde está o aviso do `getattr`, só servem para conferir a edition (`lib.rs:433`). Os da busca vão à consulta, mas não à vista da pergunta na página.
- **MAP-O72.** O parser Python fixado (`rustpython-parser 0.4.0`) recusa construções do Python 3.12 a 3.14: aspas iguais dentro de f-string, default de parâmetro de tipo, `except A, B:` sem parênteses e t-string. Cada uma faz o arquivo inteiro virar diagnóstico. O `syn 3.0.5` lê as construções de Rust testadas, menos `gen { }`. O `type X = ..` do Python 3.12 é lido, mas não vira símbolo. Os dois projetos reais não têm nenhuma dessas construções que falham hoje. O Healthchecks foi medido no 2.4e com um ambiente CPython 3.14 (parte 4, MAP-Q13).
- **MAP-O73.** O corte de 60% divide os termos que o símbolo pega pelos grupos da **pergunta inteira**, inclusive os que não existem no código (`search.rs:243`). Caso real: a pergunta com dois assuntos deu 1 hit, um `main` que só tem as palavras no corpo, enquanto cada assunto sozinho deu 6 e 10. Os dois verbos da pergunta (`gera`, `reflete`) não estão em nenhum símbolo e entram no denominador.
- **MAP-O74.** As rotas Django ficam no grafo quando estão numa lista atribuída (`variable urlpatterns`), mas a rota não liga à view. A view é escrita como valor (`views.ping`), e um `attribute_access` só tem campos como candidatas (`python.rs:102`). Nos dois `urls.py` medidos, 0 de 30 referências a views chegam a uma função; 9 arestas de nome vão para campos homônimos (`Check.status`, `Channel.checks`…). O `include("…")` com texto não liga a raiz ao app. As chamadas de nível de módulo fora de atribuição (`register_converter`) são descartadas.
- **MAP-O75.** No Python, nenhuma ligação com `basis: "type"` trata uma função ou um método como se fosse a classe (ajuste 4): das 235 de receptor `construtor`, 89 vêm de classe do projeto, 141 de `super()` e 5 de um retorno anotado. O `construtor` grava o chamado de qualquer chamada (MAP-O66), mas a regra só tira dele uma classe quando o nome é de uma classe do projeto, ou pelo retorno escrito.
- **MAP-O76.** Os ids são estáveis entre gerações do mesmo código, também entre perfis: os registros v8 de 18/11 e os v9 de hoje são byte a byte iguais nos dois projetos. Mas o id de um símbolo contém o hash do conteúdo do arquivo inteiro (`lib.rs:271`): qualquer mudança no arquivo muda o id de todos os símbolos dele (verificado num sintético).

## Perguntas (Parada 2)

Nenhuma nova: o que a Parada 2 pediu, o código respondeu.

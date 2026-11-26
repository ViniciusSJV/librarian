# Mapeamento do repositório — parte 4: o gabarito do 2.4e

**Estado:** Pendente de validação do usuário: etapa 3a; J5 não rodado.

**Data:** 26/11/2026. **Base:** `main` em `5b79e1c`. Worktree: só os relatórios do mapeamento. **Só leitura**: nenhum código, documento, `.studio`, Healthchecks ou ray tracer foi alterado. Nada foi instalado nem compilado, e nenhum script do gabarito foi rodado. Os únicos comandos executados só leram ou pediram versão: `git`, `ls`, `find`, `grep`, `cat`, os `--version` das ferramentas, `uv python list --only-installed` e `python.exe -I -c "import sys; print(sys.version)"` no Python que o uv já tinha instalado.

**Escopo.** A pasta `tools/precisao/gabarito/`, que o 2.4e criou. Pela [parte 3](03-ligacoes.md) (MAP-O23), o commit do 2.4e só toca essa pasta. Ids novos continuam a numeração: `MAP-O35` em diante e `MAP-Q12` em diante.

**Fontes.** `.rs`, `.py`, `.js`, `Cargo.toml`, `Cargo.lock`, os JSON das fixtures e da linha de base. Dos projetos de teste, só os TXT e INI de configuração (`requirements*.txt`, `mypy.ini` do Healthchecks; `Cargo.toml` do ray tracer). Nenhum `.md` foi aberto além dos relatórios 01 a 03: nem `tools/precisao/gabarito/README.md`, nem `docs/reports/fase-2/dados/2.4e/tabelas.md`, nem os relatórios do 2.4e. Comentários e cabeçalhos dos scripts contam como **"diz ser"**.

**Regras de evidência:** as mesmas das partes 1 a 3. Cada afirmação traz `ARQUIVO:linha` ou o comando. **Verificado** quer dizer por execução ou pela leitura do código que faz a coisa. **Leitura minha** marca interpretação.

Caminhos abreviados: os arquivos sem pasta são de `tools/precisao/gabarito/`; `rs-sonda/` é `tools/precisao/gabarito/rs-sonda/`.

---

## J1. Os arquivos

`git show --stat aead3eb`: 11 arquivos, 1.448 linhas. É o único commit da pasta (`git log -- tools/precisao/gabarito`). A mensagem é "gabaritos de execucao e de tipos do passo 2.4e, so medicao", sem corpo.

| Arquivo | Linhas | O que faz (verificado, por leitura) | Como é invocado (diz ser: cabeçalho) |
|---|---:|---|---|
| `librarian.js` | 116 | **O lado do Librarian.** Compila os binários (`prepareBinaries`, `:29`) e pode recusar por `--fontes` (`:30`). Sobre uma cópia da `.studio` (`withStudio`, `:35`), gera um run novo de cada projeto (`/api/generate`, `:40`). Monta a tabela de linhas de cada fonte a partir dos snapshots da edition (`:47-53`), as declarações (`:55-60`) e toda ocorrência `call_observed`/`method_call_observed` com o dono (`:62-72`). Pergunta `/api/calls` com `depth: 1` e `chosen: {}`, uma vez por dono (`:78`), e guarda a resposta de cada ocorrência (`:85-97`). Grava `librarian-<hc\|rt>.json` (`:104`) e `librarian-resumo.json`, com o hash da `.studio` antes e depois (`:112-114`) | `node tools/precisao/gabarito/librarian.js --studio .studio --saida <dir> [--bin target/release] [--fontes <sha256>]` (`:7`) |
| `execucao_py.py` | 133 | **Régua de execução do Python.** Registra um callback em `sys.monitoring` para o evento `CALL` (`:23-25`, `:116`, `:122`). Roda um programa com `runpy.run_path` (`:124`) depois de `os.chdir(ROOT)` (`:119`). Para cada par diferente (código que chama, offset, objeto chamado) cujo chamador está no projeto, grava `tipo: chamada` com a posição (`co_positions`), o chamador e a cadeia `__wrapped__` do alvo (`:94-114`, `:83-89`). Um chamador de fora com alvo no projeto vira `tipo: raiz` (`:96-107`). Exclui `site-packages/` e `.venv`/`venv` (`:39`). Grava o JSONL no fim (`:129-131`) | `<python do virtualenv> execucao_py.py <raiz do projeto> <saida.jsonl> <programa> [argumentos...]`; exemplo `… manage.py test` (`:5-6`). "Rode com PYTHONDONTWRITEBYTECODE=1 e sem `python -I`" (`:8`) |
| `tipos_py.py` | 163 | **Régua de tipos do Python.** Faz `os.chdir(ROOT)` (`:18`) e roda um build do mypy, com `export_types` e `preserve_asts`, sem modo incremental (`:31-36`). O cache vai para `GABARITO_MYPY_CACHE` ou para `mypy-cache` ao lado da saída (`:35`). Para cada `CallExpr` de cada módulo do projeto (`:148-159`), grava o que a expressão chamada refere (`nodes_of`, `:46-62`): o nó ligado pelo mypy, ou, num atributo sem nó, o método achado no tipo do receptor. Para métodos de classe, grava também as sobrescritas nas classes do projeto, pela MRO do mypy (`describe`, `:64-89`; `overrides`, `:111-124`). Sem alvo, grava o `motivo`: `any`, `sem_tipo`, `sem_no` ou `nao_ref` (`record`, `:126-144`) | `<python do virtualenv> tipos_py.py <raiz do projeto> <saida.jsonl> <alvo do mypy, ex.: hc> [opcoes do mypy...]`; exemplo `… hc --strict --platform linux` (`:6-7`) |
| `scip.js` | 67 | Lê um índice SCIP (protobuf) sem dependências: o encoding, os documentos e as ocorrências (linha, colunas, símbolo, papéis) (`parse`, `:33-56`). Pela linha de comando, imprime o encoding e as ocorrências de uma linha (`:60-67`). **Não gera o índice**: só o lê | `node scip.js <index.scip> [<path> <line>]` (`:9`); `medir.js` o usa como módulo (`medir.js:12`) |
| `medir.js` | 644 | **A comparação.** Lê da pasta `--saida`: `librarian-hc.json`, `librarian-rt.json`, `execucao-hc.jsonl`, `tipos-hc.jsonl`, `execucao-rt.jsonl` e `renderer.scip` (`:8-9`). Lê também as fontes do ray tracer no disco (`SRC.rt`, `:18-21`, `:249`) e as fixtures (`:462`). Classifica cada ocorrência por régua (J4), calcula o alcance de 7 workflows por arestas do grafo, da execução e da união (`:33-51`, `:367-391`, `:510-543`), agrupa as discordâncias em famílias (`FM-1` a `FM-5`, `:394-416`), calibra cada régua contra a verdade das fixtures (`:451-485`) e grava `medicao.json` (`:643`) | `node tools/precisao/gabarito/medir.js --saida <dir> [--hc <Healthchecks>] [--rt <ray tracer>]` (`:6-7`); os projetos vêm de `--hc`/`--rt`, `GABARITO_HC`/`GABARITO_RT` ou das pastas vizinhas (`:18-21`) |
| `tabelas.js` | 85 | Lê `medicao.json` (`:8`) e grava `tabelas.md` (`:84`): classificação por régua, populações, as tabelas 1 a 6, calibração, conhecidos e extras | `node tools/precisao/gabarito/tabelas.js --saida <dir>` (`:3`) |
| `rs-sonda/` (crate solto) | 13 + 105 + 41 (+ `Cargo.lock` 46) | **Régua de execução do Rust.** `main.rs` apaga e recria uma pasta de cópia (`:74`) e copia o projeto sem `target/` e `.git/` (`copiar`, `:49-59`). Na cópia, insere no começo de cada função com corpo de `src/` e `tests/` uma chamada `<crate>::__g24e_sonda::entrar("<arquivo>:<linha>:<nome>")` (`:29-37`, `:40-47`, `:79-96`). `const fn` fica sem sonda (`:32`). Acrescenta `pub mod __g24e_sonda;` ao `src/lib.rs` da cópia (`:98-101`) e grava a sonda, `sonda.rs` (`:102`). `sonda.rs` mantém uma pilha por thread e grava cada par novo (chamador, chamado), com a thread e o executável, no arquivo de `G24E_SAIDA` (`:23-41`; formato em `:34`) | `rs-sonda <projeto> <copia>` (`main.rs:3`). Tem `[workspace]` próprio (`Cargo.toml:13`) e está fora do workspace do Librarian (`Cargo.toml:2` da raiz; MAP-O7). Depende de `syn` 2.0.119 e `proc-macro2` 1.0.107 (`Cargo.lock`) |
| `README.md` | 35 | não aberto (regra da sessão) | — |

**A ordem de uso, pelo que cada arquivo lê e grava** (*leitura minha*, pelas entradas e saídas; o código não tem um script que encadeie os passos):
1. `librarian.js` → `librarian-hc.json`, `librarian-rt.json`;
2. `execucao_py.py` → `execucao-hc.jsonl`; `tipos_py.py` → `tipos-hc.jsonl`;
3. `rs-sonda` → cópia instrumentada do ray tracer. Rodar algo nessa cópia com `G24E_SAIDA` → `execucao-rt.jsonl`;
4. um índice SCIP do ray tracer → `renderer.scip`. Quem o gera não está no código (J2);
5. `medir.js` → `medicao.json`; `tabelas.js` → `tabelas.md`.

**Diz ser × faz:** o cabeçalho do `medir.js` diz "Writes medicao.json and tabelas.md" (`:9`), mas ele só grava `medicao.json` (`:643`). O `tabelas.md` é do `tabelas.js` (`:84`).

---

## J2. As réguas

| Régua | De onde vem a verdade | Ferramenta externa e versão |
|---|---|---|
| **Execução, Python** | O objeto que o interpretador chamou de verdade, em cada local de chamada, durante o programa rodado (`execucao_py.py:94-114`) | CPython 3.12 ou mais (`sys.monitoring`, `:2`). A versão usada no 2.4e não está no código. O programa rodado também não: o exemplo do cabeçalho é `manage.py test` (`:6`) |
| **Tipos, Python** | A declaração que o mypy liga à expressão chamada, com o plugin do django-stubs pelo `mypy.ini` do projeto (`tipos_py.py:1-4`, diz ser). No Healthchecks, `mypy.ini` declara `plugins = mypy_django_plugin.main` e `django_settings_module = "hc.settings"` | mypy e django-stubs. **A versão usada no 2.4e não está no código.** O que o projeto fixa: `mypy==2.4.1` e `django-stubs==6.1.2` (`requirements-dev.txt` do Healthchecks); `Django[argon2]==6.1.3` e `django-stubs-ext==6.1.2` (`requirements.txt`) |
| **Tipos (índice), Rust** | Os símbolos de referência e de definição de um índice SCIP do ray tracer (`medir.js:262-280`). O nome do símbolo diz se é do projeto (`rust-analyzer cargo renderer …`, `:273`) | rust-analyzer, pelo prefixo dos símbolos (`:273`, `:275`). A versão e o comando que geraram o `renderer.scip` não estão no código. Nesta máquina há `rust-analyzer 1.98.1 (48a229ce 2026-09-01)` (`rust-analyzer --version`) |
| **Execução, Rust** | Os pares (função que chama, função chamada) que a sonda viu nas funções do projeto (`sonda.rs:23-41`; `medir.js:322-362`) | `rs-sonda` (`syn` 2.0.119), compilado com cargo. Nesta máquina há `cargo 1.98.1` e `rustc 1.98.1`. O que foi rodado na cópia (os drivers) não está no código: `sonda.rs` grava o nome do executável em `binario` (`:33-34`), mas o `medir.js` não lê esse campo |

**O que cada régua não enxerga, pelo código** (verificado, por leitura; os efeitos sobre a comparação estão em J4):

- **Execução, Python:**
  - só vê o que o programa rodou. O que nenhum teste executa fica sem gabarito (`nao_viu`, `medir.js:137`);
  - o chamador precisa estar no projeto e fora de `site-packages`/`venv` (`execucao_py.py:39`, `:95-107`). Uma chamada feita por código de fora (Django, unittest) vira `raiz` e não casa com ocorrência nenhuma (`medir.js:181` só lê `tipo: chamada`);
  - o alvo é um objeto, não um local. Uma função sem `__code__` (C, builtins) vira `{"externo": módulo, nome}` (`execucao_py.py:76-81`). Um `unittest.mock` vira `duble` (`:63-64`). Uma classe vira a classe, não o `__init__`: o `__init__` só entra nas arestas de alcance (`medir.js:190`);
  - num decorador, a verdade é a função do projeto mais de dentro da cadeia `__wrapped__` (`medir.js:121-133`). Uma função de arquivo de teste chamada por código de produção vira `duble` (`:128`);
  - o casamento com a ocorrência do Librarian é pelo começo do trecho e pelo fim (`matchSpan`, `medir.js:88-95`). O que não casa conta como "só do gabarito" (`:198`);
  - grava um par por (código, offset, objeto) (`execucao_py.py:108`). Um local que chamou alvos diferentes dá vários pares, e todos entram como alvos (`medir.js:199-200`).
- **Tipos, Python:**
  - onde o mypy dá `Any`, não há tipo, ou o nó não é achado, não há alvo (`tipos_py.py:52-62`, `:137-138`);
  - uma variável do mypy (`Var`, um chamável guardado num nome ou campo) é descartada (`medir.js:207-208`);
  - uma chamada cuja expressão não é nome nem atributo (resultado de chamada, índice) dá `nao_ref` (`tipos_py.py:139-143`);
  - é a declaração estática, não o corpo que roda. Para o despacho, o `medir.js` aceita "pela base" quando o Librarian dá a base e as sobrescritas que o mypy listou (`:146-147`, `:218-221`);
  - só os módulos com caminho dentro da raiz entram (`tipos_py.py:150-152`).
- **Tipos (índice), Rust:**
  - a referência é procurada só no byte do nome chamado (`callName`, `medir.js:229-244`, `:282-289`). Uma ocorrência cujo nome não está nesse byte fica sem gabarito;
  - símbolos `local` são ignorados (`:266`);
  - um símbolo de fora do projeto vira `ext:…` (`:275`). Um do projeto sem definição no índice, ou sem declaração na edition, vira `semdecl:` (`:277-279`);
  - para um método de trait, a "base" e as sobrescritas vêm de uma regex sobre o rótulo `impl … Trait … for` (`:293-297`), e não do índice;
  - referências do índice sem ocorrência do Librarian (operadores, macros, referência como valor, `use`) são contadas à parte (`:300-321`).
- **Execução, Rust:**
  - só vê funções com corpo em `src/` e `tests/` da cópia (`main.rs:79-80`). Ficam de fora: `const fn` (`:32`), funções geradas por macro ("o syn nao abre macros", `:9-10`, diz ser) e as cenas em `caps/` (os `[[bin]]` de `caps/*.rs` no `Cargo.toml` do ray tracer).
  - **Só vê funções do projeto.** Uma chamada a `std` (`Vec::len`, `f64::clamp`, `Iterator::next`) não tem sonda e não gera par. Por isso a execução do Rust **nunca dá um alvo de fora**: ela só monta gabarito para uma ocorrência quando há um par com uma função do projeto de mesmo nome (`medir.js:353-357`). Sem par, a ocorrência fica `nao_viu`. *Leitura minha:* um link falso do tipo "método do projeto no lugar de um de `std`" (UQ-19, UQ-23, UB-15, UB-23) não pode ser apontado pela execução do Rust. Só pelo índice.
  - os pares são por função, não por local de chamada. O casamento é pelo nome, e pelo tipo escrito antes de `::` quando há (`medir.js:342-361`). Várias ocorrências com o mesmo nome no mesmo dono ficam `ambiguo` (`:359`) → `casou_ambiguo` (`:138`);
  - numa thread do rayon, o primeiro par perde o chamador (`?`, com o nome da thread vazio) e é guardado à parte (`medir.js:332-335`, `:529-534`).

---

## J3. Como pega as ligações do Librarian

**Rota e binário** (verificado, por leitura de `librarian.js`):
- binários: `target/release/librarian_studio` e `librarian_ingest`, compilados por `prepareBinaries` (`lib.js:145-150`) quando `--bin` é o `target/release` do repositório. Com `--fontes`, recusa se o hash das fontes for outro (`librarian.js:30`);
- `.studio`: a do argumento, só lida. Os dois projetos de `L.DEFAULTS` (`healthchecks-25597e41/20261118-061046`, `renderer-9b78f3a9/20261118-020654`, `lib.js:8`) vão para uma cópia em `os.tmpdir()` (`withStudio`, `lib.js:127-139`);
- runs novos, por `/api/generate` (`librarian.js:40`). O Studio lê o projeto pela raiz gravada no `project.json` da cópia. **Leitura minha:** é o mesmo caminho do portão, que confere antes se o projeto está no commit do run de referência (`portao.js:464-468`). **O `librarian.js` não faz essa conferência:** não chama `gitHead` (`grep -n gitHead librarian.js` está vazio);
- `/api/calls` com `depth: 1` e `chosen: {}`, um pedido por dono (`:78`). É a mesma rota e a mesma forma do portão (parte 3, seção G). A resposta guarda `resolution`, `candidates`, `relation`, `type_rule`, `basis`, os alvos (com os cortados por `node_limit`) e `via` (`:90-96`).

**De que depende cada parte:**

| Parte | Depende de |
|---|---|
| `librarian.js` | `.studio` com os dois projetos e os runs de `DEFAULTS`; os projetos-fonte acessíveis na raiz gravada (para gerar o run novo); o repositório compilável (`cargo build --release`) |
| `execucao_py.py`, `tipos_py.py` | a raiz do Healthchecks **original** (não uma cópia): os dois fazem `os.chdir(ROOT)` (`execucao_py.py:119`, `tipos_py.py:18`); um interpretador com Django, as dependências do projeto, mypy e django-stubs (J2) |
| `rs-sonda` e a execução do Rust | o ray tracer original, só lido e copiado (`main.rs:74-75`); a cópia é compilada e rodada |
| `renderer.scip` | um índice do ray tracer cujos caminhos e bytes batem com a edition. O `medir.js` confere que os arquivos no disco têm a mesma tabela de linhas da edition (`disco`, `:252-260`) |
| `medir.js` | as 6 saídas acima na mesma pasta; as fontes do ray tracer no disco (`SRC.rt`, `:18-21`); as fixtures `amostra-hc.json`, `amostra-rt.json` e `estrato-unique.json` (`:452`) |

**Os workflows e as famílias** são fixos no código, com caminho e linha do projeto no commit fixo (`WORKFLOWS`, `medir.js:33-51`). Healthchecks A a D (cadastro, ping, atraso, flip e notificações); ray tracer `CR-1` a `CR-3` (render, malha OBJ com BVH, gravação da imagem). Um projeto em outro commit muda as linhas, e as entradas não achadas são contadas (`entradas_nao_achadas`, `:521`).

---

## J4. Como compara e classifica

**Cada ocorrência contra cada régua** (`classify`, `medir.js:136-163`; verificado). Antes, um alvo cuja classe-mãe também é alvo sai da lista (o método dentro da classe, `:139-142`).

| Categoria | Regra |
|---|---|
| `nao_viu` | a régua não tem alvo para a ocorrência (`:137`) |
| `nao_viu_duble` | a régua só viu um dublê (mock ou função de teste chamada por produção) (`:137`) |
| `casou_ambiguo` | execução do Rust: o mesmo nome mais de uma vez no mesmo dono (`:138`, `:359`) |
| `concorda` | o Librarian dá **uma** ligação cheia (`LINKED`: `unique`, `receiver`, `type`, `trait_default`, `import`, `scope`, `chosen`; `:22`), e a régua dá exatamente esse alvo (`:148-150`) |
| `concorda_pela_base` | a régua dá a base (método do pai ou do trait) com sobrescritas, e o Librarian dá a base ou sobrescritas dela (`:146-147`, `:151`, `:156`) |
| `discorda_polimorfica` | ligação cheia única, e o alvo do Librarian é **um** dos alvos do projeto que a régua viu, mas não o único (`:152`) |
| `link_falso_candidato` | ligação cheia única, e o alvo do Librarian não está entre os alvos da régua (`:153`) |
| `possivel_com_o_certo` | o Librarian dá vários alvos (`homonym`, ou mais de um alvo), e todos os alvos da régua estão entre eles, todos do projeto (`:155-158`) |
| `possivel_em_parte` | vários alvos, e só parte dos da régua está entre eles (`:159`) |
| `possivel_sem_o_certo` | vários alvos, e nenhum é da régua (`:160`) |
| `concorda_externo` | o Librarian não dá alvo, e a régua só dá alvos de fora do projeto (`:162`) |
| `ligacao_que_falta` | o Librarian não dá alvo, e a régua dá um alvo do projeto (`:162`) |

**O veredito combinado** (`:498-508`): conta só as réguas que viram (`UNSEEN` = `nao_viu`, `nao_viu_duble`, `casou_ambiguo`; `:24`). Sem nenhuma, `nao_viu`. Se todas as que viram discordam (`DISCORD` = `link_falso_candidato`, `discorda_polimorfica`, `ligacao_que_falta`, `possivel_sem_o_certo`; `:23`), vale a primeira discordância. Se umas discordam e outras não, `gabaritos_divergem`. Senão, vale o da primeira que viu.

**Grupo provisório** (`grupo`, `:166-172`; "a classificação que conta é a do 2.4f", `:165`, diz ser):
- `provavel_pelo_tipo`: `type`, `trait_default`, ou `receiver` com `type_rule`;
- `provavel_pelo_nome`: `unique` numa `method_call_observed`;
- `outras`: o resto das ligações cheias (`import`, `scope`, `receiver` sem `type_rule`, `unique` numa chamada simples, `chosen`).

**Famílias** (`FAMILIAS`, `:394-416`). Uma ocorrência entra numa família quando as réguas que a viram concordam numa discordância (`:570-596`); a primeira família cujo teste passa fica com ela. Sem família, a chave é `novo|resolução|forma|tipo do alvo da régua|discordância` (`:588`):

| Id | Nome (diz ser) | Teste no código |
|---|---|---|
| `FM-1` | Dublê de teste ligado por código de produção (HC) | ligação cheia única, alvo em arquivo de teste e chamador fora de teste (`:398`) |
| `FM-2` | Método de biblioteca com o nome de um método do projeto (as duas linguagens) | `unique` numa `method_call_observed`, sem `type_rule`, e alguma régua só com alvos de fora (`:402`) |
| `FM-3` | Tipo do elemento de array ou coleção (RT) | receptor indexado, `a[i].m(..)` (`:406`) |
| `FM-4` | Despacho pelo método padrão de trait (RT) | `self.m(..)` dentro de um método padrão de trait (`:410`) |
| `FM-5` | `T::from` com vários `impl From<X>` (RT) | o texto começa com `T::from` (`:415`) |

**Calibração** (`calibracao`, `:451-485`). Cada régua é medida contra a verdade validada de `amostra-<p>.json` e `estrato-unique.json`. A ocorrência é achada pela chave `ocorrencia` da fixture. A régua concorda quando:
- verdade `externo`: a régua dá só alvos de fora (`:477`);
- verdade `alvo`: a régua dá exatamente esse alvo (`:478`);
- verdade `ambigua`: os alvos da régua estão entre os ambíguos, ou é a base com as sobrescritas (`:479`).

**Saídas a mais:**
- as populações, com intervalo de Clopper-Pearson de 95% (`cp`, `:446`; `:617-622`): universo, fora de testes, o recorte do `lista.js` e o do portão. O `lista.js` mora em `docs/reports/fase-2/dados/2.4d/lista.js` (`git ls-files | grep lista.js`), e não foi aberto nesta parte;
- o alcance de cada workflow, por grafo, execução e união (`:510-543`);
- a cobertura de cada família (`:551-566`);
- os "conhecidos": as ligações cheias para `FakeCurl.*`, `Ping.get_body`, `impl Groups::len`, `impl Sorteio::next` e `impl Color::clamp` (`:638-641`).

**O que cada categoria é na tabela C da parte 3** (*leitura minha*, pela regra acima; sem números, porque o J5 não foi rodado):
- `link_falso_candidato` e `discorda_polimorfica` só saem de uma **linha cheia** (`LINKED` com um alvo, `:148`). Pelo grupo, as de `provavel_pelo_nome` são o `unique` da tabela C ("não cabe"), e as de `provavel_pelo_tipo` são o `type` e o `trait_default` ("cheia").
- `possivel_*` saem de uma **tracejada**, ou de uma cheia com vários alvos. O ramo `_` de `v2.rs:428` (MAP-O17) também vira `homonym` com vários alvos.
- `concorda_externo` e `ligacao_que_falta` saem de uma **lacuna** (sem ligação). O código não separa `no_candidate_in_project` de `above_candidate_limit`.

---

## J5. Rodar o gabarito de novo

**Não rodado.** O que rodar exige está na seção abaixo; a decisão é do usuário.

---

## J6. O que do 2.4e ainda vale hoje

- **O código do gabarito:** não mudou desde o 2.4e. `aead3eb` é o único commit da pasta, e `git diff --stat aead3eb HEAD -- tools/precisao/gabarito tools/precisao/lib.js` está vazio (verificado).
- **A regra:** não mudou depois. O último commit em `resolve.rs` é o `665ff5b` (25/11, 18:13; parte 3, MAP-O23). `git diff --stat aead3eb HEAD -- crates Cargo.toml Cargo.lock experiments tools/precisao/sinteticos` está vazio. O último commit que toca o hash das fontes (`crates/`, manifestos) é o `4652541` (25/11, 20:23), anterior ao `aead3eb` (26/11, 03:02) (verificado, `git log -- crates Cargo.toml Cargo.lock`).
- **O hash das fontes de hoje** é `3aeb2fa7…` (250 arquivos; parte 3, seção E, Parada 2-bis). É o mesmo do `fontes` gravado em `estrato-unique.json`, de 25/11 (runs `20261125-234413`/`-234657`).
- **O hash das fontes da medição do 2.4e:** `librarian.js` o grava em `librarian-resumo.json` (`binarios.fontes`, `:34`, `:114`), na pasta de saída. **Nenhum arquivo do repositório que eu possa abrir o guarda:** `docs/reports/fase-2/dados/2.4e/` só tem `tabelas.md` (não aberto), e `tabelas.js` não imprime o hash. Não dá para conferir pelo código nem pelos JSON (MAP-Q12). *Leitura minha:* com nada compilado mudado entre `4652541` e hoje, uma medição feita de uma árvore sem arquivos não rastreados em `crates/` daria `3aeb2fa7…`. Isso não foi conferido.
- **As respostas do Librarian:** na amostra (241 chamadas), no UQ e nos usos, são as mesmas da linha de base do 2.4d, por execução (parte 3, Parada 2-bis). Para as outras ocorrências que o `librarian.js` mede (todas as chamadas dos dois projetos), *leitura minha*: com as mesmas fontes, os mesmos projetos e o mesmo grafo, a resposta é a mesma. Não conferido.
- **O que não se pode dizer que ainda vale:** as saídas das réguas (`execucao-*.jsonl`, `tipos-hc.jsonl`, `renderer.scip`). Elas não estão no repositório (`git ls-files | grep -E 'execucao-|tipos-hc|\.scip'` vazio) e dependem de ferramentas e versões que o código não fixa (J2).

---

## O que rodar o gabarito exige

Nesta máquina (verificado, só lendo e pedindo versão): `node v24.19.0`; `cargo 1.98.1`, `rustc 1.98.1`, `rust-analyzer 1.98.1`; `uv 0.13.6`, com um CPython **3.14.10** já instalado pelo uv (`uv python list --only-installed`), cujo `site-packages` só tem `pip`. No PATH do Git Bash, `python` e `python3` são só o atalho da loja do Windows. O Healthchecks não tem virtualenv (`ls -a ../pilotos/healthchecks`; `git status --porcelain --ignored` vazio). No cache do uv há pacotes baixados antes, sem ambiente instalado: `mypy-2.3.1`, `mypy-2.4.1`, `django_stubs-6.1.2` e `django_stubs_ext-6.1.2` (`find …/uv/cache -iname 'mypy-*'`).

| Parte | Interpretador ou ferramenta, versão | O que executa | Onde grava | Compila ou instala algo em `../pilotos` ou `../renderer`? |
|---|---|---|---|---|
| `librarian.js` (lado do Librarian) | Node (aqui v24.19.0); cargo para compilar o Librarian | o `librarian_studio` do repositório, sobre uma cópia da `.studio`; o Studio **lê** os projetos para gerar o run | `target/` do repositório (compilação); a cópia em `os.tmpdir()`; a pasta `--saida` | não. Lê os dois projetos |
| **Python, execução** (`execucao_py.py`) | CPython ≥ 3.12 (`sys.monitoring`), num ambiente com Django 6.1.3 e as dependências do `requirements.txt` do Healthchecks. A versão do 2.4e não está no código. **Não há esse ambiente nesta máquina** | **roda código do Healthchecks:** o programa passado (exemplo, `manage.py test`, `:6`), ou seja, a suíte de testes do projeto, com `os.chdir` na raiz original (`:119`, `:124`) | a saída JSONL (`:129`). *Leitura minha, não conferida:* o que a suíte do Healthchecks grava ao rodar (banco de teste, arquivos estáticos, `.pyc` se faltar `PYTHONDONTWRITEBYTECODE`, `:8`) cai na raiz do projeto ou onde as `settings` dele mandarem | **sim, é preciso instalar:** um ambiente com as dependências do projeto. Onde o 2.4e o pôs não está no código. Os `.pyc` podem ir para `../pilotos/healthchecks` (`:8`) |
| **Python, tipos** (`tipos_py.py`) | mypy e django-stubs, com o mesmo ambiente do Django (o plugin lê `hc.settings`, pelo `mypy.ini`). O projeto fixa `mypy==2.4.1` e `django-stubs==6.1.2`. A versão do 2.4e não está no código. **Não instalado aqui** (só no cache do uv) | análise estática pelo mypy. *Leitura minha, pela forma de funcionar do plugin do django-stubs e não pelo código daqui:* o plugin carrega as `settings` do Django, o que **importa código do Healthchecks** (`hc.settings` e os apps). Não roda os testes | a saída JSONL (`:148`) e o cache do mypy em `GABARITO_MYPY_CACHE` ou ao lado da saída (`:35`). `.pyc` do projeto, se faltar `PYTHONDONTWRITEBYTECODE` (*leitura minha*) | **sim, é preciso instalar** (o mesmo ambiente). Faz `os.chdir` na raiz original (`:18`) |
| **Rust, tipos** (`renderer.scip` + `scip.js`) | rust-analyzer (aqui 1.98.1). O comando que gerou o índice no 2.4e não está no código | `scip.js` só lê o índice. *Leitura minha, não conferida:* gerar o índice com `rust-analyzer scip` carrega o workspace do ray tracer pelo cargo, o que pode compilar os scripts de build e as macros das dependências | o `renderer.scip`, onde o comando mandar. *Leitura minha:* a geração pode gravar no `target/` do projeto indexado | **provável, se rodado no original:** `../renderer/target` (já existe hoje). Numa cópia, não |
| **Rust, execução** (`rs-sonda` + a cópia) | cargo/rustc (aqui 1.98.1) para compilar o `rs-sonda` e a cópia instrumentada | compila o `rs-sonda`. Ele copia e reescreve a cópia (só lê o original, `main.rs:74-75`). Depois **roda código do ray tracer** na cópia (os testes ou os binários, os "drivers"). O comando não está no código | a cópia (onde `<copia>` mandar; apagada e recriada, `:74`); o arquivo de `G24E_SAIDA` (`sonda.rs:30-34`). **Compilar o `rs-sonda` no lugar grava `tools/precisao/gabarito/rs-sonda/target/`, que não é ignorado** (`git check-ignore` não o pega; `.gitignore` só tem `/target`): o `git status` deixaria de ficar só com a pasta do mapeamento, a menos que se use outro `CARGO_TARGET_DIR` | não, se a cópia ficar fora de `../renderer`. O `rs-sonda` não copia `target/` (`:54`), então a cópia compila do zero e baixa as dependências pelo cargo, se não estiverem no cache |
| `medir.js`, `tabelas.js` | Node | só leem as saídas, as fontes do ray tracer e as fixtures | a pasta `--saida` (`medir.js:643`, `tabelas.js:84`) | não |

**Resumo para a decisão do J5** (*leitura minha*, a partir da tabela):
- o lado do Librarian, o `medir.js` e o `tabelas.js` dá para rodar sem instalar nada;
- as duas réguas do Python exigem instalar um ambiente com Django, as dependências do Healthchecks, mypy e django-stubs. A de execução também roda a suíte do Healthchecks na pasta original;
- as duas do Rust exigem compilar: o `rs-sonda` e uma cópia do ray tracer, e talvez o próprio ray tracer para gerar o índice. A de execução roda código do ray tracer.

Pela regra da sessão, cada um desses pontos é um "pare e pergunte".

---

## Em palavras simples

*À parte das seções acima, sem números nem nomes de arquivo.*

O Librarian diz, para cada chamada no código, para onde ela vai. O passo anterior quis saber quantas dessas respostas estão erradas sem ter de conferir uma por uma à mão. Para isso, usou **réguas**: outros jeitos de descobrir para onde a chamada vai, que não dependem do Librarian.

São dois tipos de régua, uma de cada em cada linguagem:
- **Rodar o programa e olhar.** Põe-se um "contador" no projeto e roda-se os testes. A cada chamada que acontece de verdade, o contador anota de onde ela saiu e o que foi chamado. É como seguir as pessoas pelo prédio para ver em que porta cada uma entra.
- **Perguntar a quem entende de tipos.** Existem programas que leem o código com as regras completas da linguagem (o verificador de tipos do Python e o analisador do Rust). Eles dizem qual declaração cada nome quer dizer, como um revisor muito rigoroso.

Depois, uma comparação põe lado a lado, chamada por chamada, a resposta do Librarian e a de cada régua. Quando o Librarian afirma "é esta" e a régua diz "é outra", a chamada vira **suspeita de link falso**. Se as réguas discordam entre si, a chamada fica à parte, sem acusar ninguém. As suspeitas parecidas são juntadas em famílias, como "um dublê de teste no lugar da biblioteca de verdade" ou "um método da linguagem confundido com um do projeto". Antes de confiar numa régua, ela é testada contra as chamadas que uma pessoa já conferiu.

**O que as réguas não conseguem ver:**
- Rodar o programa só mostra o que os testes rodaram. Um caminho que nenhum teste usa fica no escuro.
- No Rust, o contador só fica nas funções do projeto. Quando o código chama algo da própria linguagem, nada é anotado. Então essa régua nunca consegue dizer "isso foi para fora do projeto"; só a do analisador consegue.
- O verificador de tipos só diz o que está escrito. Onde o tipo é "qualquer coisa", ele se cala. E ele diz a declaração, não qual versão de um método roda num objeto de verdade.
- Chamadas que o programa faz sozinho por dentro, como operadores e código gerado, nem sempre têm uma chamada correspondente no Librarian para comparar.

E uma coisa importante: as réguas medem, mas não decidem. Uma discordância é uma suspeita, não um veredito. A palavra final continua sendo de uma pessoa.

---

## Observações novas

- **MAP-O35.** O gabarito não tem um comando que o rode de ponta a ponta. São 4 programas independentes com saídas numa mesma pasta (J1, a ordem). O que gera duas das 6 entradas do `medir.js`, o `renderer.scip` e o `execucao-rt.jsonl` (os drivers), não está no código.
- **MAP-O36.** As versões das ferramentas externas não ficam registradas em lugar nenhum que o código grave. Nem o mypy, nem o django-stubs, nem o Python, nem o rust-analyzer. O `librarian.js` grava o hash das fontes do Librarian, mas as réguas não gravam a própria versão.
- **MAP-O37.** A execução do Rust não pode dar um alvo de fora do projeto (J2): só monta gabarito a partir de pares com funções do projeto (`medir.js:353-357`). Nos links falsos "método do projeto no lugar de `std`" (UQ-19, UQ-23, UB-15, UB-23), só o índice SCIP pode discordar. A execução do Python, ao contrário, vê os alvos de fora (`execucao_py.py:76-81`).
- **MAP-O38.** O `librarian.js` gera runs novos sem conferir se os projetos estão no commit do run de referência. O portão confere (J3).
- **MAP-O39.** As réguas do Python rodam na pasta original do Healthchecks (`os.chdir(ROOT)`), e não numa cópia como o lado do Librarian e a execução do Rust (J3). A de execução roda a suíte de testes do projeto ali.
- **MAP-O40.** Compilar o `rs-sonda` no lugar cria `tools/precisao/gabarito/rs-sonda/target/`, que o `.gitignore` não cobre (só `/target`).
- **MAP-O41.** Diz ser × faz: o cabeçalho do `medir.js` diz que ele grava `tabelas.md` (`:9`), mas é o `tabelas.js` que grava (`:84`).
- **MAP-O42.** Os workflows (`medir.js:33-51`) e os "conhecidos" (`:638-641`) são escritos no código com caminho, linha e nome do projeto no commit fixo. O prefixo `CR-` dos workflows do ray tracer e o `FM-` das famílias aparecem no código. O commit `038f058` ("prefixos FM e CR") os registra nos documentos.

## Perguntas

- **MAP-Q12.** O hash das fontes da medição do 2.4e (`binarios.fontes` em `librarian-resumo.json`) ficou guardado em algum JSON ou TXT que eu possa abrir? No repositório, a pasta de dados do 2.4e só tem o `tabelas.md`, que a regra da sessão não deixa abrir.
- **MAP-Q13.** Com que ambiente o 2.4e rodou as réguas do Python: versão do Python, do mypy e do django-stubs, e onde estava o ambiente? O código não diz, e nesta máquina não há ambiente com Django nem mypy (só pacotes no cache do uv).
- **MAP-Q14.** Que comando gerou o `renderer.scip`, sobre qual pasta (o ray tracer original ou uma cópia), e que comandos rodaram na cópia instrumentada (testes, binários e quais) para dar o `execucao-rt.jsonl`?
- **MAP-Q15.** Onde estão as saídas da medição do 2.4e (`medicao.json`, os JSONL, o `.scip`)? Sem elas, o J5 só pode ser uma medição nova, e não uma conferência da antiga.

---

**Conferência final:** depois de escrever este arquivo, conferi a `.studio` original e o `git status --porcelain -uall` (resultado na mensagem da sessão).

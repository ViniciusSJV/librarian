# Portão de precisão da Fase 2

Um comando mede a precisão das relações de "o que chama?" e decide se um passo da Fase 2 pode ser aceito. Ele não altera o código de produção nem o acervo. O Studio roda sobre uma cópia do `.studio` em uma pasta temporária, e o comando confere, no fim, que o original não mudou.

## Como rodar

Na raiz do repositório (o comando compila o binário do passo em teste antes de medir; ver o passo 0 abaixo):

```
node tools/precisao/portao.js --studio .studio --base tools/precisao/linha-de-base.json
```

| Parâmetro | Padrão | O que é |
|---|---|---|
| `--studio` | (obrigatório) | Pasta `.studio` com `healthchecks-25597e41` e `renderer-9b78f3a9`. Só é lida |
| `--bin` | `target/release` | Pasta com `librarian_studio` e `librarian_ingest` |
| `--base` | nenhum | Linha de base para aplicar a regra. Sem ela, o comando só mede |
| `--saida` | pasta temporária | Onde gravar `relatorio.md` e `resultado.json` |
| `--gravar-base` | nenhum | Grava o resultado como nova linha de base. Só com aprovação do usuário |
| `--so-sinteticos` | — | Mede só os níveis N0–N8, sem dados reais. É a parte que pode entrar no CI |
| `--fontes` | nenhum | O hash das fontes compiladas que a medição precisa ter (o da Parada 2). Com outro, o comando recusa (código 2). Desde o passo 2.4 (Q1) |

Código de saída: 0 aprovado (ou só medição), 1 reprovado, 2 não conseguiu medir.

Requisitos: Node 18 ou mais novo. Os projetos-fonte precisam estar no commit dos runs de referência: Healthchecks em `6c05a7a` e o ray tracer em `e38254b`. Se não estiverem, o comando recusa.

O Healthchecks leva cerca de 2 minutos, porque `/api/calls` é chamado uma vez por função.

## O que o comando faz

0. Com o `--bin` padrão (`target/release` deste repositório), compila antes os dois binários (`cargo build --release --locked -p librarian-studio -p librarian-ingest`): o `cargo test` recompila só o `librarian_ingest`, e um `librarian_studio` velho daria uma medida errada sem aviso. Com outra pasta em `--bin` (um binário antigo guardado para uma prova), usa os binários como estão. Nos dois casos, o relatório e o `resultado.json` registram a pasta, se compilou e o SHA-256 de cada binário (`lib.js`, `prepareBinaries`).
   - **Trava (desde o passo 2.4, Q1).** O hash do binário não identifica o código: o `librarian_studio` carrega o commit e o `dirty` em que foi compilado (`LIBRARIAN_BUILDER`, do `build.rs` do `librarian-evidence`), e o linker do Windows grava data e GUID do `.pdb` a cada link, então dois builds do mesmo código dão hashes diferentes. A trava é o **hash das fontes compiladas** (`fontes`): o SHA-256 da lista `sha256  caminho`, ordenada, de todo arquivo de `crates/`, `Cargo.toml` e `Cargo.lock` que o Git rastreia ou não ignora, menos os `.md` (não são compilados), mais todo arquivo que um `.rs` de `crates/` embute (`include_str!`, `include_bytes!`) e que seja `.md` ou esteja fora de `crates/` (hoje, um só: a resposta de `experiments/claims-camera-20260928-04/`, embutida num teste do `librarian-llm-engine`), lido da pasta (`lib.js`, `sourceHash`). O fim de linha é normalizado (CRLF para LF) antes do hash, que é o mesmo antes e depois do commit e em qualquer checkout (até 24/11/2026, sem a normalização, dependia do `core.autocrlf`). O relatório registra também a identidade embutida no studio (`recorded:<commit>:<dirty>`) e, só como registro, o hash dos binários. A Parada 2 e a linha de base final precisam ter as mesmas fontes (`--fontes`). Com outra pasta em `--bin`, as fontes são desconhecidas.
1. Copia os dois projetos do `.studio` para uma pasta temporária e sobe o `librarian_studio` sobre a cópia, numa porta livre.
2. Gera um run novo de cada projeto com o binário em teste. É nele que tudo é medido.
3. Mede a população de chamadas `obj.metodo()` fora de testes, sem `self.m()`. Dá a contagem por `resolution` e por estrato, e os links de método para função solta.
4. Classifica a amostra das fixtures contra a verdade validada. Calcula também a estimativa ponderada para a população: a proporção de cada estrato na amostra vezes o tamanho do estrato. É **estimativa**.
5. Refaz a auditoria dos fatos F1–F26 da Parte E.
6. Gera os projetos sintéticos N0–N8 e compara a resposta com o resultado registrado e com o alvo da Fase 2.
7. Roda `librarian_ingest verify-graph` em cada run antigo e confere que os arquivos dele não mudaram durante a medição.
8. Confere o hash agregado do `.studio` original, antes e depois.

## Categorias da amostra

| Categoria | Quando |
|---|---|
| `correta` | Link `unique`, `receiver`, `type` ou outro de alvo único, apontando para o alvo certo |
| `link_falso` | Link `unique`, `receiver`, `type` ou outro de alvo único, apontando para o alvo errado |
| `candidatos_falsos` | `homonym` ou acima do teto, sem o alvo certo entre os candidatos. Inclui acima do teto com alvo externo |
| `ambigua` | `homonym` com o alvo certo entre os candidatos, ou acima do teto com alvo no projeto |
| `sem_candidato` | Sem link (`no_candidate_in_project`) |

Desde o passo 2.3a (DEC-39):
- um link único para uma classe, uma struct de tupla ou uma variante precisa sair com `relation: instantiates`. Com `relation: calls`, é `link_falso`, mesmo com o alvo certo;
- uma verdade `ambigua` (a regra do `dyn` no 2.1, aplicada ao `Protocol` na Q6 do 2.3) lista os corpos que podem rodar. Nenhum link único é certo (`link_falso`); um `homonym` com um deles entre os candidatos é `ambigua`, e sem nenhum é `candidatos_falsos`; acima do teto é `ambigua`.
- nos níveis, um alvo de relação `instancia` fica "no alvo" quando o link sai com `relation: instantiates`.

Desde o passo 2.3b (DEC-40):
- uma verdade com `relacao: decora` (chamada escrita num decorador) é `correta` só com o link para o alvo certo e `relation: decorates`; com `relation: calls`, é `link_falso`. Um link `decorates` para uma verdade sem `relacao: decora` é `link_falso`. As outras categorias não mudam;
- uma verdade pode ter `executor`, a declaração que roda a chamada (o `__init__` de um campo). O portão registra se o `/api/calls` dela mostra a chamada, sem reprovar;
- cada chamada da amostra é classificada também pela regra anterior (a do 2.3a, congelada em `classifyBefore40`), e o relatório lista as que dão veredito diferente. Nas 152 chamadas da linha de base do 2.3a, nenhuma.

Desde o passo 2.4a (DEC-42): `scope` é resolução de alvo único, como `import` (`LINKED`). Na linha de base do 2.3c nenhuma chamada tem `scope`, e o veredito sobre ela não muda (prova na Parada 2 do 2.4a).

Desde o passo 2.4b (DEC-43): na auditoria dos fatos, uma verdade `ambigua` lista os corpos que podem rodar. Um `homonym` com um deles entre os candidatos é `suspeito` (o par de "ambígua" na amostra); um link único, ou um `homonym` sem nenhum, é `errado`. Nenhum fato tinha `ambigua` até o 2.4a, e o veredito sobre a linha de base não muda (prova na Parada 2 do 2.4b). A verdade do F23 passou a `ambigua` (V2, 24/11/2026).

## Regra do portão

Um passo da Fase 2 reprova se, em Python ou em Rust:

- `link_falso` aumentar;
- `correta` diminuir;
- uma chamada da amostra não for encontrada no run novo;
- uma edition antiga falhar no `verify-graph` ou mudar;
- o `.studio` original mudar.

`candidatos_falsos`, ambíguas, a população, os níveis e a auditoria dos fatos são registrados com a variação, mas não reprovam sozinhos.

A regra compara só as chamadas que a linha de base também tem. Um estrato acrescentado à amostra depois aparece numa tabela à parte até entrar numa nova linha de base. Foi o caso de RT-31 a RT-55, do passo 2.1, que entraram na linha de base em 19/11/2026. HC-61 a HC-75 e RT-56 a RT-62, do passo 2.2, entraram na linha de base em 19/11/2026, com a verdade validada pelo usuário. HC-76 a HC-87 e RT-63 a RT-65, do passo 2.3a, entraram na linha de base em 20/11/2026, com a verdade validada pelo usuário (HC-79 na Q6 da Parada 1 do 2.3). HC-88 a HC-97, do passo 2.3b, entraram na linha de base em 20/11/2026, com a verdade validada pelo usuário (HC-89 e HC-97 conferidos no editor). HC-98 a HC-112 e RT-66 e RT-67, do passo 2.3c, entraram na linha de base em 20/11/2026, com a verdade validada pelo usuário na Parada 2 (HC-100, HC-110 e RT-67 conferidos no editor). HC-113 a HC-123 e RT-68 a RT-70, do passo 2.4a, entraram na linha de base em 24/11/2026, com a verdade validada pelo usuário (HC-113, HC-119 e RT-69 conferidos no editor). HC-124 a HC-143, do passo 2.4b, entraram na linha de base em 25/11/2026, com a verdade validada pelo usuário (HC-137 e HC-141 conferidos no editor; a de HC-140 e HC-142 corrigida pelo usuário para ambígua, e os dois entram como `link_falso` conhecidos, até o 2.4c). HC-144 a HC-157, do passo 2.4c, entraram na linha de base em 25/11/2026, com a verdade validada pelo usuário (HC-144 e HC-156 conferidos no editor); com o 2.4c, HC-140 e HC-142 passam a ambíguas. Uma chamada da amostra fora da população (por caminho, como `Ray::new`, ou em testes) é procurada entre todas as chamadas do run. Com a amostra original, o veredito é o mesmo da regra anterior (DEC-37). Uma exceção só vale com justificativa caso a caso e aprovação do usuário.

## Arquivos

| Arquivo | Conteúdo |
|---|---|
| `portao.js` | O comando |
| `lib.js` | Cópia do `.studio`, Studio numa porta livre, hashes, o conjunto de chamadas e as respostas |
| `amostra.js` | Como a amostra foi sorteada: rodado uma vez, sobre a linha de base de 18/11/2026. Não roda no portão |
| `fixtures/amostra-hc.json`, `fixtures/amostra-rt.json` | 157 chamadas do Healthchecks e 70 do ray tracer (RT-31 a RT-55 vêm do passo 2.1; HC-61 a HC-75 e RT-56 a RT-62, do passo 2.2; HC-76 a HC-87 e RT-63 a RT-65, do passo 2.3a; HC-88 a HC-97, do passo 2.3b; HC-98 a HC-112 e RT-66 e RT-67, do passo 2.3c; HC-113 a HC-123 e RT-68 a RT-70, do passo 2.4a; HC-124 a HC-143, do 2.4b; HC-144 a HC-157, do 2.4c), com verdade, certeza, justificativa e `verdade_validada_por` |
| `fixtures/fatos.json` | As chamadas que cada fato F1–F26 afirma, com a verdade |
| `fixtures/niveis.json` | N0–N8: resultado atual e alvo da Fase 2, com o passo (N4-py-dono, do 2.3b: o `Repo()` do campo, perguntado ao `__init__`; N3c-py-* e N3c-rs-*, do 2.3c: o tipo pelo retorno, o campo de outro arquivo e o que não decide; N0-py-*, N0-rs-*, N8-py-* e N8-rs-tipo, do 2.4a: o escopo da chamada simples e o alias. No 2.4a, o alvo do N3c-py-dois-alvos passou ao do N3c-rs-discordam: o `duplo` do próprio módulo. DP-py-*, do 2.4c: o despacho polimórfico; no 2.4c, o alvo de N5-py-sobrescrita, N5-py-prop e N5-py-prop-herdada passou a ambíguo, aprovado pelo usuário) |
| `sinteticos/py`, `sinteticos/rs` | Projetos dos níveis, copiados de `docs/reports/piloto-1b/dados/tipos/`. O 2.3c acrescentou arquivos novos (`py/pkg/retorno.py`, `py/pkg/requisicao.py`, `rs/src/retorno.rs`, `rs/src/apoio.rs` e duas linhas no fim de `rs/src/lib.rs`), sem mudar nenhuma linha dos níveis anteriores. O 2.4a acrescentou `py/pkg/escopo.py`, `py/pkg/escopo_apoio.py`, `py/pkg/escopo_estrela.py`, `py/pkg/sub/`, `rs/src/escopo.rs`, `rs/src/escopo_apoio.rs` e mais duas linhas no fim de `rs/src/lib.rs`, também sem mudar nenhuma linha anterior. O 2.4b acrescentou `py/pkg/heranca.py`, `heranca_apoio.py` e `heranca_regras.py` (níveis N5-py-*, conferidos num Python de verdade), que o teste `heranca_2_4b.rs` embute. O 2.4c acrescentou `py/pkg/despacho.py` (níveis DP-py-*, conferidos num Python de verdade), que o teste `despacho_2_4c.rs` embute |
| `linha-de-base.json` | Resultado com o binário do passo 2.4c, sobre as 157 chamadas do Healthchecks e as 70 do ray tracer (o mesmo conteúdo que `--gravar-base` grava). Aprovada pelo usuário em 25/11/2026 (as anteriores, do passo 2.4b, sobre 143 + 70 chamadas, do passo 2.4a, sobre 123 + 70 chamadas, do 2.3c, sobre 112 + 67 chamadas, do 2.3b, sobre 97 + 65 chamadas, do 2.3a, do 2.2 e do 2.1, estão no histórico do git) |

A amostra é identificada pelo caminho e pelo intervalo de bytes da ocorrência (`arquivo@início-fim`), que são os mesmos em qualquer run do mesmo código-fonte. A verdade é do usuário (DEC-22). Uma verdade nova ou corrigida entra na fixture com `verdade_validada_por` indicando quem validou.

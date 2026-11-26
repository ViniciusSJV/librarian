# Mapeamento do repositório — parte 1: o mapa geral

**Estado:** validado pelo usuário em 26/11/2026 (inclusive a Parada 1 de ajustes). MAP-Q1 respondida em 26/11/2026: a `.studio` foi restaurada (conferência na MAP-O1).

**Data:** 26/11/2026. **Base:** `main` em `6ac1865` (worktree limpa no início, depois de o usuário remover o `estrutura.txt`). **Só leitura:** nada no repositório, no Healthchecks ou no ray tracer foi alterado; a única escrita é este arquivo, não rastreado.

**Escopo.** Só o mapa: o que existe, o que depende de quê, o que o código vivo referencia. Nada aqui classifica o que fica ou sai, nem propõe estrutura nova. A fonte é o código, os `Cargo.toml`, o CI e o Git; os documentos só aparecem como "o que dizem ser".

**Como ler.** Cada afirmação traz `ARQUIVO:linha` ou o comando usado. Quando algo é **leitura minha** (interpretação, não conferida por execução), está marcado assim. Ids novos deste relatório usam o prefixo `MAP-` (`MAP-Qn` para as perguntas, `MAP-On` para as observações); estão aqui, ainda não em [docs/IDS.md](../../IDS.md).

**Comandos de base**, usados em várias seções (rodados no Git Bash, na raiz):

- Contagem por pasta, só arquivos rastreados: `git ls-files -- <pasta>` → número de arquivos; `cat` deles `| wc -l` → linhas; extensão por `sed` → tipos; `git log -1 --format=%ad --date=short -- <pasta>` → data.
- Referências vivas: `git grep -n -e '<padrão>' -- crates tools .github Cargo.toml STUDIO.cmd ':!*.md' ':!*.json' ':!*.jsonl'` (só código e configuração; documentos excluídos de propósito).
- Suíte: `cargo test --workspace --locked` (log completo no scratchpad da sessão, fora do repositório).

---

## 1. Árvore até 2 níveis

Só arquivos rastreados (`git ls-files`: **938 arquivos**, 215 commits desde `1654bea`, de 22/09/2026). "Linhas" é a soma de `wc -l`; nos binários (png, bin) o número não tem sentido e está incluído só porque a soma é mecânica.

| Pasta | Arquivos | Linhas | Tipos | Último commit |
|---|---:|---:|---|---|
| `.github/` | 1 | 12 | yml 1 | 2026-09-22 |
| `.github/workflows/` | 1 | 12 | yml 1 | 2026-09-22 |
| `crates/` | 251 | 81.684 | rs 108, json 70, js 23, sem ext. 13, md 11, txt 9, jsonl 7, toml 6, py 2, html 1, diff 1 | 2026-11-25 |
| `crates/librarian-core/` | 3 | 319 | rs 2, toml 1 | 2026-10-31 |
| `crates/librarian-evidence/` | 12 | 1.622 | rs 10, toml 1, md 1 | 2026-11-18 |
| `crates/librarian-graph-engine/` | 20 | 2.455 | rs 12, json 5, md 2, toml 1 | 2026-10-31 |
| `crates/librarian-ingest/` | 96 | 35.863 | rs 52, json 26, sem ext. 9, md 3, jsonl 3, py 2, toml 1 | 2026-11-25 |
| `crates/librarian-llm-engine/` | 30 | 4.939 | rs 20, txt 4, json 3, md 2, toml 1 | 2026-11-18 |
| `crates/librarian-studio/` | 90 | 36.486 | json 36, js 23, rs 12, txt 5, jsonl 4, sem ext. 4, md 3, toml 1, html 1, diff 1 | 2026-11-25 |
| `docs/` | 447 | 419.903 | js 124, json 100, md 81, out 55, py 24, txt 14, rs 13, tsv 9, png 9, log 8, jsonl 5, jpg 2, html 1, bin 1, requested 1 | 2026-11-26 |
| `docs/historico/` | 2 | 2.341 | md 2 | 2026-11-18 |
| `docs/img/` | 8 | 32.324 | png 5, jpg 2, html 1 | 2026-11-06 |
| `docs/reports/` | 433 | 382.132 | js 124, json 100, md 75, out 55, py 24, txt 14, rs 13, tsv 9, log 8, jsonl 5, png 4, bin 1, requested 1 | 2026-11-26 |
| `examples/` | 4 | 165 | rs 1, toml 1, md 1, txt 1 | 2026-10-31 |
| `examples/maintenance/` | 4 | 165 | rs 1, toml 1, md 1, txt 1 | 2026-10-31 |
| `experiments/` | 152 | 6.895 | json 108, txt 18, md 13, bin 13 | 2026-09-28 |
| `experiments/claims-camera-20260928-01/` … `-05/` | 18 / 34 / 33 / 33 / 34 | 379 / 1.323 / 1.578 / 1.632 / 1.983 | json, txt, md, bin | 2026-09-28 (todas) |
| `modelfiles/` | 11 | 479 | txt 9, md 1, json 1 | 2026-11-18 |
| `modelfiles/system/` | 9 | 266 | txt 9 | 2026-11-18 |
| `skills/` | 2 | 54 | md 2 | 2026-11-01 |
| `skills/ponytail/` | 2 | 54 | md 2 | 2026-11-01 |
| `tools/` | 51 | 19.803 | py 19, rs 10, json 9, js 9, md 2, toml 1, lock 1 | 2026-11-26 |
| `tools/precisao/` | 51 | 19.803 | (idem) | 2026-11-26 |

**`docs/reports/` por dentro** (3º nível, porque concentra 46% dos arquivos):

| Pasta | Arquivos | Linhas | Tipos | Último commit |
|---|---:|---:|---|---|
| `docs/reports/fase-2/` | 339 | 355.419 | js 101, json 71, md 68, out 55, py 17, tsv 9, txt 8, rs 5, jsonl 5 | 2026-11-26 |
| `docs/reports/fase-2/dados/<passo>/` | 4 a 50 por passo (2.1 … 2.4e, estrato-unique) | — | js, json, out, md, py | 2026-11-20 a 2026-11-26 |
| `docs/reports/piloto-1b/` | 93 | 26.373 | json 29, js 23, rs 8, log 8, py 7, txt 6, md 6, png 4 | 2026-11-25 |
| `docs/reports/PILOTO-1-HEALTHCHECKS.md` | 1 | 340 | md | 2026-11-18 |

**Arquivos soltos na raiz** (linhas · último commit): `.gitattributes` 10 · 10/20; `.gitignore` 3 · 11/25; `Cargo.toml` 3 · 11/18; `Cargo.lock` 1.364 · 11/18; `LICENSE` 202 · 10/31; `STUDIO.cmd` 8 · 10/20; `ingest-example.json` 1 · 09/24; e 11 `.md` (seção 4).

**Presentes no disco, fora do Git** (`.gitignore:1-3`: `/target`, `/.studio`, `/tools/precisao/node_modules/`):

- `target/`: saída do cargo.
- `tools/precisao/node_modules/`: 8,2 MB, só `playwright-core` (`du -sh`, `ls`).
- `.studio/`: **vazia** quando esta parte foi escrita (`ls -la .studio` e `dir /a .studio`: 0 arquivos, 0 pastas; data da pasta 26/11 03:06). *Atualização de 26/11/2026:* restaurada pelo usuário e conferida (MAP-O1).

---

## 2. Os crates

O workspace tem 7 membros (`Cargo.toml:2`): os 6 crates e `examples/maintenance`. Todos são `edition = "2021"` e `version = "0.1.0"`.

Contagem de testes pela **execução** de `cargo test --workspace --locked` (exit 0): **557 aprovados, 0 falhas, 2 ignorados**, o mesmo que o ESTADO-ATUAL.md:40 registra. A contagem de `#[test]` por `grep` dá números um pouco maiores no ingest (282 contra 274 + 1), porque conta `#[test]` escrito dentro de código-fonte de teste (ex.: `escopo_2_4a.rs`: 14 pelo grep, 12 executados). Vale o número da execução.

### 2.1 `librarian-core` (319 linhas)

- **O que faz:** só tipos de dados, sem I/O. Uma fonte (`SourceRef`, `Source<Execution, Capture>`), uma afirmação (`LibrarianFact`), uma seleção, um pacote (`EvidenceBundle`), a exportação de consulta (`QueryExport`) e a validação deles (`crates/librarian-core/src/lib.rs:61-266`). Os testes ficam no topo do mesmo arquivo (`lib.rs:1-58`).
- **Binários:** nenhum.
- **Depende de:** nenhum crate interno. Externa: `serde` (`crates/librarian-core/Cargo.toml`).
- **Quem usa:** `librarian-graph-engine` (`graph_engine.rs:17-19`, que reexporta os tipos como `Fact`, `Source` e `Evidence`) e `examples/maintenance` (`main.rs:2`). O `librarian-ingest` declara a dependência (`crates/librarian-ingest/Cargo.toml:15`), mas `git grep librarian_core -- crates/librarian-ingest` só acha essa linha: nenhum uso no código (MAP-O2).
- **Testes:** 5 (3 unitários e 2 em `tests/bundle_validation.rs`).
- **Docs que o descrevem:** nenhum próprio. É citado uma vez em `README.md`, `README.pt-BR.md`, `V0.1.md` e `crates/librarian-evidence/README.md` (`git grep -c librarian-core -- '*.md'`).

### 2.2 `librarian-graph-engine` (2.455 linhas)

- **O que faz:** o fluxo de "dossiê" da versão 0.1. Valida as fontes e as afirmações que o consumidor cadastra (`validate_dossier`, `validate_fact_ids`, `validate_reference`) e recorta as linhas de contexto (`context_ranges`, `merge_ranges`). Monta a consulta preparada (`prepare_query_formatted`, `PreparedQuery`, `DEFAULT_INSTRUCTIONS`), confere os registros de execução e de captura e exporta o pacote (`src/graph_engine.rs:22-749`; submódulos `bundle`, `capture`, `filesystem`, `metrics`, `nature`).
- **Leitura minha:** apesar do nome, este crate não contém o grafo de chamadas, que está em `librarian-ingest/src/graph/` (MAP-O3).
- **Binários:** nenhum. Tem um exemplo, `examples/export.rs`: "File-backed dossier consumer" (`examples/export.rs:1`).
- **Depende de:** `librarian-core`. Externas: `serde`, `serde_json`, `sha2`.
- **Quem usa:** `librarian-ingest`, só em `src/prepare.rs:11` e `:594` (a ponte da edition até um `PreparedQuery`), e `examples/maintenance`.
- **Testes:** 26 (13 unitários e 13 em `tests/`: `agnostic`, `content`, `legacy_exports`, `public_pipeline`). Fixtures em `tests/fixtures/legacy-v0.1/`.
- **Docs:** `crates/librarian-graph-engine/WORKFLOW-V0.1.md` (19/10); citado em `README.md`, `README.pt-BR.md`, `WORKFLOW.md` e `examples/maintenance/README.md`.

### 2.3 `librarian-ingest` (35.863 linhas; o maior em Rust junto com o studio)

- **O que faz:** lê as fontes locais de um projeto e grava uma **edition**, um acervo imutável em arquivos, com hashes de consistência (`src/lib.rs:1`). Os extratores sintáticos ficam em `src/extractors/`: Rust com `syn` (`rust.rs:1-2`) e Python com `rustpython-parser` (`python.rs:1-2`, `python_types.rs:1-2`).
- Sobre a edition, o crate monta:
  - o grafo (`graph.rs`, `graph/v2.rs`, e `graph/uses.rs` para o `used_by` do 2.4d);
  - a **regra única de resolução de chamadas** (`resolve.rs:1-2`, 2.251 linhas);
  - a busca lexical (`search.rs:1`);
  - o inventário de operações (`inventory.rs:1-2`);
  - o envelope P5 (`envelope.rs:1-2`) e o dono de um passo da cadeia (`owner.rs:1-2`);
  - a ponte `prepare` (`prepare.rs:1-2`);
  - o **contrato do pacote de evidências** (`evidence.rs`, 3.308 linhas), que o `librarian-evidence` usa (`crates/librarian-evidence/src/lib.rs:6-7`).
- **Binário:** `librarian_ingest` (`src/bin/librarian_ingest.rs`, que chama `cli::run`). Comandos vistos em `src/cli.rs:7-36`: `search`, `graph` e `verify-graph` (leitura parcial; não listei todos).
- **Depende de:** `librarian-graph-engine` (usado) e `librarian-core` (declarado, sem uso). Externas: `syn` `=3.0.5` com `proc-macro2`, `rustpython-parser` e `rustpython-ast` `=0.4.0`, `serde`, `serde_json`, `sha2` (`Cargo.toml:7-15`).
- **Testes:** 274 aprovados e 1 ignorado, em 28 arquivos de `tests/`; nenhum unitário em `src/`. O ignorado é `tests/gabarito.rs:26` ("needs the local data in .studio/gabarito"). Vários testes leem os sintéticos do portão em `tools/precisao/sinteticos/` (seção 3).
- **Docs:** `crates/librarian-ingest/README.md` (06/11) e `crates/librarian-ingest/SEARCH.md` (25/11; os limites da regra). Também citado em `WORKFLOW.md`, `README.md`, `README.pt-BR.md`, `docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md` e `crates/librarian-studio/README.md`.

### 2.4 `librarian-evidence` (1.622 linhas)

- **O que faz:** o Evidence Builder. Monta, confere e refaz o pacote de evidências de uma pergunta, só com os dados que recebe. Não lê arquivos, não chama Git, não roda comandos nem chama modelos (`src/lib.rs:1-4`). Seus módulos:
  - `package` monta o pacote;
  - `check` faz as conferências H1–H3;
  - `suggestion` faz o envelope e a H4;
  - `selection`, `lacuna` e `identity`;
  - `keeper` é a interface de quem guarda os registros (`src/*.rs:1`).
- O `build.rs` grava no binário o commit e o estado `dirty` do código (`build.rs:1-27`).
- **Binários:** nenhum.
- **Depende de:** `librarian-ingest` (o contrato do pacote). Externas: `serde`, `serde_json`.
- **Testes:** 9 (4 unitários e 5 em `tests/independence.rs`). Esse teste lê uma fixture que fica **dentro do studio**, `crates/librarian-studio/tests/evidence-v1` (`independence.rs:48`), e confere as fontes do próprio crate por `include_str!` (`:274-289`).
- **Docs:** `crates/librarian-evidence/README.md` (18/11); `docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md`; `crates/librarian-studio/README.md`.

### 2.5 `librarian-llm-engine` (4.939 linhas)

- **O que faz:** tudo o que envolve modelo (`src/lib.rs:1-4`):
  - o contrato de **claims** e a conferência mecânica de cada resposta (`claims.rs` e `claims/*`: citações, localização, texto estrito, edits, patch, comparação, inglês). Ele "não estabelece acarretamento" (`claims.rs:1`).
  - a biblioteca de tarefas, perfis e idiomas, que monta os Modelfiles a partir de `modelfiles/catalog.json` (`models.rs:1-10`);
  - o transporte por Ollama ou pela CLI do Claude Code (`transport.rs:1-2`, `ollama/`).
- **Binários** (em `src/bin/`):
  - `claims`, que faz `prepare` e `check` sem rede (`bin/claims.rs:1-8`);
  - `prepare_ollama` e `send_ollama`, as CLIs "históricas" (`cli/send_ollama.rs:1`, `ollama/client.rs:1`).
- **Depende de:** nenhum crate interno. Externas: `reqwest` 0.13 (blocking), `serde`, `serde_json`, `sha2`.
- **Lê do disco:** `modelfiles/` (`models.rs:261`, `:353`; `transport.rs:500`, nos testes).
- **Testes:** 60 aprovados e 1 ignorado (32 unitários, com o ignorado em `cli/send_ollama.rs:162`, e 28 em `tests/claims.rs`). Um teste lê um arquivo de `experiments/` (`tests/claims.rs:274`).
- **Docs:** `crates/librarian-llm-engine/README.md` (18/11), `crates/librarian-llm-engine/CLAIMS.md` (11/11) e `modelfiles/README.md`.

### 2.6 `librarian-studio` (36.486 linhas, das quais `src/store.rs` 7.939 e `web/index.html` 8.317)

- **O que faz:** um servidor HTTP local, só em 127.0.0.1, para a página `web/index.html` (`src/main.rs:1-3`; `http.rs:1`). Nessa página se escolhe o projeto, gera o grafo, faz perguntas e envia consultas preparadas ao LLM. São 35 rotas `/api/*` em `main.rs`, por exemplo `/api/calls`, `/api/used-by`, `/api/ladder` e `/api/send` (`git grep -o '"/api/…"'`). Os módulos:
  - `store`: projetos, runs e perguntas em `.studio/` (`store.rs:1-2`);
  - `execution`: os comandos do `librarian.json`, rodados numa cópia (`execution.rs:1-2`);
  - `instrument`: o "Medir tudo";
  - `ladder`: a escada de modelos, com o portão de comandos;
  - `questions`, `lexicon`, `claims`, `facts` (L4) e `llm`.
- A página carrega o three.js de um CDN (`index.html:406`).
- **Binário:** `librarian_studio` (`Cargo.toml` `[[bin]]`). O `STUDIO.cmd` compila em release e abre a porta 8765 (`STUDIO.cmd:5-7`).
- **Depende de:** `librarian-ingest`, `librarian-evidence` e `librarian-llm-engine`. Externas: `reqwest`, `serde`, `serde_json`.
- **Lê do disco:** `.studio/`, `modelfiles/` e `skills/`, este pelo nome vizinho dos modelfiles (`main.rs:66`; `store.rs:1306`).
- **Testes:** 183 (182 unitários dentro de `src/` e 1 em `tests/agnostic.rs`). As fixtures ficam em `src/fixtures/rodada1-4.json` (`ladder.rs:295-320`) e em `tests/evidence-v1/` (`store.rs:7750`). As baterias de tela (`tests/tela/*.js`) ficam fora do cargo (seção 3).
- **Docs:**
  - `crates/librarian-studio/README.md` e `STUDIO.md`;
  - `docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md` e `docs/ESTADO-IMPLEMENTACAO-STUDIO.md`;
  - `tests/tela/README.md` e `tests/tela/APOSENTADAS.md`.

### 2.7 `examples/maintenance` (165 linhas, membro do workspace)

`maintenance-example`, `publish = false`: um catálogo sintético de manutenção, sem renderer, LLM nem banco (`src/main.rs:1`), feito sobre `librarian-core` e `librarian-graph-engine`. É compilado pelo CI (está no workspace) e tem 0 testes.

### 2.8 Quem depende de quem

Pelos `Cargo.toml`; o uso real foi conferido com `git grep -l 'librarian_<crate>::'`.

```
librarian-studio  (bin librarian_studio)
 ├── librarian-ingest ──────────┐
 ├── librarian-evidence ──► librarian-ingest
 └── librarian-llm-engine       │        (sem dependência interna)
                                ▼
                     librarian-ingest  (bin librarian_ingest)
                      ├── librarian-graph-engine ──► librarian-core
                      └── librarian-core   (declarado, sem uso no código: MAP-O2)

examples/maintenance ──► librarian-graph-engine, librarian-core

Fora do Cargo, por arquivo:
 librarian-evidence (teste)  ──lê──► crates/librarian-studio/tests/evidence-v1
 librarian-ingest  (testes)  ──lê──► tools/precisao/sinteticos/
 librarian-llm-engine (teste)──lê──► experiments/claims-camera-20260928-04/
 librarian-llm-engine, studio──lê──► modelfiles/
 librarian-studio            ──lê──► skills/, .studio/
```

---

## 3. As outras pastas

Busca de referência viva: nos crates (código e testes), em `tools/`, no CI (`.github/`), no `Cargo.toml`, no `STUDIO.cmd`, no portão (`tools/precisao/portao.js` e `lib.js`) e no `tools/precisao/telas.js`. Documentos `.md` não contam como referência viva.

| Pasta | O que contém | Referência viva | Situação |
|---|---|---|---|
| `tools/precisao/` | O portão da Fase 2 (`portao.js` 514 linhas, `lib.js`); `amostra.js`, `estrato-unique.js`; o runner das telas (`telas.js`); `fixtures/` (amostras HC/RT/UB/UQ, fatos, níveis); `linha-de-base.json`; `sinteticos/` (py e rs); `gabarito/` do 2.4e (`medir.js`, `execucao_py.py`, `tipos_py.py`, `scip.js`, crate solto `rs-sonda`) | Os testes do ingest embutem os sintéticos: `heranca_2_4b.rs:20-22`, `despacho_2_4c.rs:19`, `classe_nome_2_4d0.rs:19-21`, `usos_2_4d.rs:63`. Por isso o CI depende de `sinteticos/`. O portão e o gabarito são comandos manuais; **o CI não os roda** (`ci.yml:12`). O `rs-sonda` não está no workspace (`Cargo.toml:2`) | viva (commit 26/11) |
| `docs/reports/` | Relatórios das Paradas, folhas de revisão, relatórios do portão por passo e `dados/` por passo (scripts `.js`, saídas `.out`, json) | `telas.js:16-24` roda **9 baterias que moram aqui**: 8 em `docs/reports/fase-2/dados/<passo>/tela-*.js` e `docs/reports/piloto-1b/dados/g3/tela-g3.js`. Os testes do ingest só citam `docs/reports` em comentário (`heranca_2_4b.rs:4`, `despacho_2_4c.rs:3`, `classe_nome_2_4d0.rs:3`) | parte viva (as 9 baterias); o resto é registro (commit 26/11) |
| `docs/historico/` | O plano até 18/11 e o seu README | nenhuma (`git grep docs/historico` no código: 0) | registro congelado por regra (`CLAUDE.md`) |
| `docs/img/` | As capas atuais (`librarian-cover-en.jpg`, `-pt-BR.jpg`), `studio.png` e as capas antigas (`capa.png`, `cover-en.png`, `cover-pt-BR.png`, `cover/base.png`, `cover/cover.html`) | nenhuma no código. As capas atuais e `studio.png` são usadas por `README.md:2`, `README.pt-BR.md:2` e `STUDIO.md:3`. As 5 antigas só aparecem no `CONTEXTO-CONTINUIDADE.md:698`; `cover/` não aparece em lugar nenhum (`git grep -l cover/base.png` vazio) | as 5 antigas: **sem uso aparente** (último commit 19/10 e 31/10). O ESTADO-ATUAL.md:135 lista "exclusão das capas antigas" como pendência do usuário |
| `examples/maintenance/` | O exemplo de catálogo de manutenção (seção 2.7) | membro do workspace (`Cargo.toml:2`), compilado pelo CI | viva pelo CI; último commit 31/10 |
| `experiments/` | 5 experimentos de claims da câmera, de 28/09 (contexto, perguntas, respostas, avaliações) | Só `-04/attempt-1/response.txt`, por `crates/librarian-llm-engine/tests/claims.rs:274`; entra no hash do portão por isso (`tools/precisao/lib.js:166-171`). `.gitattributes:3-7` marca os 5 como `-text` | `-04`: viva (1 arquivo). **`-01`, `-02`, `-03`, `-05`: sem uso aparente**: `git grep -l claims-camera-20260928-0N` fora de `experiments/` só acha `.gitattributes` (e `CLAIMS.md` para 01 e 05); último commit 28/09 |
| `modelfiles/` | `catalog.json` e 9 textos SYSTEM (analista, léxico, performance, verificável; pt e en) | `studio/src/main.rs:66`, `llm-engine/src/models.rs:261`, `:353`, `transport.rs:500` | viva (commit 18/11) |
| `skills/ponytail/` | Cópia de um `SKILL.md` externo, MIT (`ORIGEM.md:1`) | O código lê a pasta: `studio/src/store.rs:1304-1334` lista `skills/*/SKILL.md`, e `main.rs:119` e `:497` a expõem e carregam. Mas nenhuma tarefa do catálogo oferece skill: `"skills": []` nas 4 tarefas (`catalog.json:58,71,85,102`). A página diz que "nenhuma é oferecida por padrão" e que o Ponytail fica ali "para uma futura especialização de performance" (`index.html:4188-4189`) | lida pelo código, não oferecida (commit 01/11) |
| `crates/librarian-studio/tests/tela/` (baterias de tela) | 23 `.js` com Playwright (`git ls-files 'crates/librarian-studio/tests/tela/*.js' \| wc -l`), mais `README.md` e `APOSENTADAS.md`. *Correção da Parada 1: a versão anterior dizia 25 `.js` e 21 aposentadas; a contagem certa é 23 = 3 vivas + `ambiente.js` + 19 aposentadas.* | `telas.js:26-28` roda **3**: `tela-estado` (parte A), `tela-simplificacao` e `tela-passo6`. `ambiente.js` não é bateria: é um auxiliar exigido por `require` em `tela-passo6` e nas aposentadas. As **19 aposentadas** são: `etapa2c`, `rodar`, `tela-4a`, `tela-4b`, `tela-builder`, `tela-cartao`, `tela-centro`, `tela-conversa`, `tela-evidencia`, `tela-inspecao`, `tela-navegacao`, `tela-nomes`, `tela-origem`, `tela-passo2`, `tela-perfil`, `tela-projeto`, `tela-rascunho`, `tela-revisao` e `tela-zero`. Batem com `APOSENTADAS.md:104` ("As 18 do Studio … e o `rodar.js`"). Nenhuma está no `telas.js`, e nenhum arquivo vivo as exige por `require` | 3 vivas (+ `ambiente.js`). As 19: **sem uso aparente pelo código** (últimos commits de 06/11 a 13/11), "aposentadas" pelo próprio registro. **O que foi procurado:** o nome literal em `telas.js` e em `require(` de outro `.js`. **O que o método não pega:** um caminho montado em tempo de execução ou uma listagem da pasta. Pela leitura do `telas.js:15-29`, ele usa só a lista `VIVAS`, sem listar a pasta |
| raiz: `ingest-example.json` | `{"project":"librarian","inputs":["crates","examples","Cargo.toml","Cargo.lock"]}` | nenhuma: `git grep -l ingest-example` no repositório inteiro está vazio | **sem uso aparente**; único commit `4dc7fe5`, 24/09 |
| raiz: `STUDIO.cmd` | Compila o studio em release e abre o navegador | é o ponto de entrada manual; citado por `README.md`, `README.pt-BR.md`, `STUDIO.md` e `TESTME.md` | viva como lançador (commit 20/10) |

**Comando que confirma as 19 no `APOSENTADAS.md`** (e as 3 vivas no `telas.js`):

```
for b in $(git ls-files 'crates/librarian-studio/tests/tela/*.js' | xargs -n1 basename | sed 's/\.js$//'); do
  v=$(grep -cE "(name|nome): '$b'|/$b\.js" tools/precisao/telas.js)
  a=$(grep -cE "\b$b(\.js)?\b" crates/librarian-studio/tests/tela/APOSENTADAS.md)
  echo "$b telas=$v apos=$a"; done
```

Resultado: as 19 aposentadas dão `telas=0`, com `apos` de 3 a 6; as 3 vivas dão `telas=1` (também aparecem no `APOSENTADAS.md`, pela parte que saiu de cada uma, ex. a parte B da `tela-estado`); `ambiente` dá `telas=0 apos=2`.

**Hash das fontes do portão (`--fontes`).** Ele é calculado por `sourceHash` (`tools/precisao/lib.js:173-193`), chamado por `prepareBinaries` (`lib.js:160`) só quando o binário é o `target/release` do próprio repositório. Comparam esse hash com `--fontes`: `portao.js:450-451`, `estrato-unique.js:72` e `gabarito/librarian.js:30`. **Não é uma lista fixa de arquivos.** São pastas e uma busca, verificado por leitura do código:

1. **Pastas e arquivos fixos:** `git ls-files -co --exclude-standard -- crates Cargo.toml Cargo.lock` (`lib.js:174`). São todos os arquivos de `crates/`, rastreados **ou não rastreados e não ignorados**, mais os dois manifestos. Os `.md` ficam de fora (`:176`). Hoje dá 242 arquivos (mesmo comando, `grep -vc '\.md$'`).
2. **Uma busca no código:** todo `include_str!("…")`/`include_bytes!("…")` com caminho literal em um `.rs` de `crates/` cujo alvo seja `.md` ou fique fora de `crates/` (`lib.js:177-183`, regex na `:180`). Hoje são **8 arquivos de fora**, conferidos com o mesmo regex por `grep -ohE`:
   - `experiments/claims-camera-20260928-04/attempt-1/response.txt`;
   - 7 de `tools/precisao/sinteticos/py/pkg/`: `classe_nome.py`, `classe_nome_fora.py`, `classe_nome_usos.py`, `despacho.py`, `heranca.py`, `heranca_apoio.py` e `heranca_regras.py`.
3. **O que o hash não cobre**, por esse método:
   - o que um `.rs` lê em tempo de execução, por caminho montado. É o caso de `tools/precisao/sinteticos/<lang>` inteiro, lido por `crates/librarian-ingest/tests/usos_2_4d.rs:63`; de `modelfiles/` (`librarian-llm-engine/src/models.rs:261`, `studio/src/main.rs:66`); e de `skills/` (`studio/src/store.rs:1306`);
   - um `include_str!` cujo argumento não seja um literal simples (ex.: `concat!`). Nenhum foi procurado à parte.
4. O fim de linha é normalizado antes do hash (`lib.js:189`).

Ver MAP-O8.

**Fora de tudo isso, mas citados pelo ESTADO-ATUAL** (`ESTADO-ATUAL.md:54`, `:69`): os projetos de teste ficam ao lado do repositório (`../renderer`, `../pilotos/healthchecks`; `ls ..` mostra `renderer` e `pilotos`). Não os abri. O terceiro projeto de teste da tabela "Onde cada parte está" é o **`logistica-api`** (Python), que "não está disponível nesta máquina desde o passo 2.2" (`ESTADO-ATUAL.md:69`). A pendência que depende dele está em `ESTADO-ATUAL.md:92`. `ls ..` não o mostra.

---

## 4. Documentos `.md` da raiz e de `docs/`

"Resumo" é o que o próprio documento diz ser, nas primeiras linhas, não um juízo sobre se está atual. Os resumos são leitura minha das primeiras linhas.

### Raiz

| Documento | Linhas · bytes | Último commit | O que diz ser |
|---|---|---|---|
| `CLAUDE.md` | 10 · 604 | 2026-11-18 | Ponteiro para o ESTADO-ATUAL e as 4 regras fixas |
| `ESTADO-ATUAL.md` | 142 · 32.202 | 2026-11-26 | "Entrada única de qualquer sessão": fase, passo, regras, ids, pendências |
| `CONTEXTO-CONTINUIDADE.md` (CTX) | 1.295 · 144.941 | 2026-11-26 | Diário datado desde 18/11; seções não reescritas (última, a 36) |
| `DECISOES.md` (DEC) | 61 · 36.137 | 2026-11-25 | Registro das decisões tomadas (DEC-NN), com o estado de cada uma |
| `PLANO-MULTILINGUAGEM-E-VISUALIZADOR.md` (PLANO) | 664 · 42.802 | 2026-11-26 | "Só o roadmap ativo" desde 18/11; princípios e roadmap (`:7`, `:20`) |
| `HISTORICO.md` | 37 · 3.634 | 2026-11-18 | Como o Librarian nasceu dentro do renderer e saiu dele em 19/10 |
| `README.md` / `README.pt-BR.md` | 208 / 208 · 17.438 / 17.688 | 2026-11-18 | Apresentação pública (inglês e português), com capa |
| `STUDIO.md` | 1.463 · 111.921 | 2026-11-25 | Manual do Studio: como iniciar e como funciona |
| `TESTME.md` | 25 · 1.580 | 2026-11-18 | Entrada para testar: `STUDIO.cmd` e o renderer |
| `V0.1.md` | 68 · 4.699 | 2026-09-23 | Critério de aceitação funcional da versão 0.1 (fontes e afirmações do consumidor) |
| `WORKFLOW.md` | 59 · 3.913 | 2026-10-22 | Como o Librarian e um projeto analisado se dividem (o perfil `librarian.json`) |

### `docs/` (fora de `reports/`)

| Documento | Linhas · bytes | Último commit | O que diz ser |
|---|---|---|---|
| `docs/IDS.md` | 58 · 6.925 | 2026-11-26 | Convenção de ids: abreviações e prefixos |
| `docs/ARQUITETURA-EVIDENCIAS-MULTIAGENTE.md` (ARQ) | 171 · 11.674 | 2026-11-18 | A direção de 18/11; "Nada aqui está implementado" |
| `docs/ESPECIFICACAO-FUNCIONAL-STUDIO.md` (ESP) | 1.116 · 84.566 | 2026-11-18 | Especificação aprovada em 11/11; "não descreve uma implementação concluída"; superada em parte pela simplificação de 17/11 |
| `docs/ESTADO-IMPLEMENTACAO-STUDIO.md` (EST) | 1.761 · 236.324 | 2026-11-18 | Estado em 13/11 da reformulação do front (F0–F10) |
| `docs/historico/PLANO-ate-18-11.md` (PLANO-H) | 2.302 · 233.050 | 2026-11-18 | O plano inteiro até 18/11 (cópia de `aae51d7`) |
| `docs/historico/README.md` | 39 · 3.223 | 2026-11-18 | De onde veio cada documento histórico |

### `docs/reports/` (75 `.md`)

| Grupo | Quantos | Datas | O que dizem ser (título) |
|---|---:|---|---|
| `PILOTO-1-HEALTHCHECKS.md` (PIL) | 1 (340 linhas) | 2026-11-18 | "Piloto Python 1: Healthchecks" |
| `piloto-1b/00` a `04` + `dados/q6/RESULTADO.md` | 6 | 2026-11-18 | Relatório final, G3, Partes A–C, inventário e auditoria (D, E), Q6 |
| `fase-2/<passo>-parada-N.md` | 21 | 19/11 a 26/11 | Paradas 1, 2 (e a 3 do 2.1) de cada passo, do 2.1 ao 2.4e |
| `fase-2/<passo>-revisao-amostra.md` + `estrato-unique-revisao.md` | 10 | 19/11 a 25/11 | Folhas de revisão das amostras (RT, HC, UB/PH, UQ) |
| `fase-2/2.1-links-type.md` | 1 | 2026-11-19 | Os links `type`/`trait_default` do ray tracer (leitura do agente) |
| `fase-2/baterias-parada-1/2.md` | 2 | 2026-11-25 | A arrumação das baterias de tela |
| `fase-2/dados/<passo>/portao-*/relatorio.md` | 33 | 19/11 a 25/11 | Todos com o título "Portão de precisão da Fase 2": a saída do portão em cada medição |
| `fase-2/dados/2.4e/tabelas.md` | 1 (429 linhas) | 2026-11-26 | Sem título `#`; as tabelas da Parada 2 do 2.4e |

Comando: `git ls-files 'docs/reports/*.md'`, com `grep -m1 '^# '`, `wc -l` e `git log -1`.

### `.md` dentro de crates, tools e pastas auxiliares

São 13: os 6 da seção 2, mais `crates/librarian-graph-engine/tests/fixtures/legacy-v0.1/README.md`, `crates/librarian-ingest/tests/fixtures/v1/README.md`, `tools/precisao/README.md` (o portão), `tools/precisao/gabarito/README.md`, `modelfiles/README.md`, `skills/ponytail/*.md` e `examples/maintenance/README.md`. Mais 13 em `experiments/`.

---

## 5. O que o CI roda e o que a suíte cobre

**CI:** um workflow só, `.github/workflows/ci.yml`, sem mudança desde 22/09/2026 (`git log -1 -- .github`).

- Gatilho: `on: [push, pull_request]` (`ci.yml:2`).
- Matriz: `ubuntu-latest` e `windows-latest` (`:7`).
- Passos: checkout, toolchain stable e **`cargo test --workspace --locked`** (`:10-12`).
- Não há outro passo: nem `clippy`, nem `fmt`, nem Node.

**O que o CI não roda** (leitura do `ci.yml` inteiro, 12 linhas):

- o portão (`node tools/precisao/portao.js`);
- as baterias de tela (`node tools/precisao/telas.js`). O README delas diz o mesmo: "ficam fora do `cargo test` e do CI" (`crates/librarian-studio/tests/tela/README.md:3`);
- os gabaritos do 2.4e;
- o crate solto `tools/precisao/gabarito/rs-sonda`.

**O que a suíte cobre** (execução de hoje, por binário de teste, do log de `cargo test`):

| Crate | Unitários | Integração | Total | Ignorados |
|---|---:|---:|---:|---:|
| core | 3 | 2 | 5 | 0 |
| evidence | 4 | 5 | 9 | 0 |
| graph-engine | 13 | 13 | 26 | 0 |
| ingest | 0 | 274 (28 arquivos; o maior é `agnostic.rs` com 101) | 274 | 1 (`gabarito.rs:26`) |
| llm-engine | 32 | 28 | 60 | 1 (`send_ollama.rs:162`) |
| studio | 182 | 1 | 183 | 0 |
| maintenance-example | 0 | 0 | 0 | 0 |
| **Total** | | | **557** | **2** |

Doc-tests: 0 em todos. Leitura minha sobre a forma: no ingest, os testes de integração seguem os passos do roadmap pelo nome do arquivo (`tipos_2_3a`, `dono_2_3b`, `retorno_2_3c`, `escopo_2_4a`, `heranca_2_4b`, `despacho_2_4c`, `classe_nome_2_4d0`, `usos_2_4d`) e as versões de perfil (`v5`, `v7`, `v8`, `v1_regression`). No studio, quase tudo é unitário, dentro de `src/`.

**`git status` depois da suíte:** vazio. A suíte não escreveu nada rastreado nem não ignorado.

---

## 6. Proposta de ordem das próximas partes

Uma parte por sessão. A ordem vai de baixo para cima no diagrama da seção 2.8, porque assim cada parte só depende do que já foi mapeado. As pastas de apoio vêm depois dos crates que as leem.

| Parte | O quê | Por que nesta posição |
|---|---|---|
| 2 | `librarian-core` + `librarian-graph-engine` + `examples/maintenance` | São a base do diagrama e são pequenos (319 + 2.455 + 165 linhas). Juntos, porque o graph-engine só reexporta o core e o exemplo só usa os dois. Responde cedo a MAP-O2 e MAP-O3 (o nome e o uso real), que mudam como se lê o resto |
| 3 | `librarian-ingest` | O núcleo da Fase 2 (resolução, grafo, edition, contrato do pacote), o maior em testes. Depende só da parte 2. Pode precisar de duas sessões (src e tests), com 35.863 linhas |
| 4 | `tools/precisao/` | O portão, as fixtures, os sintéticos e o gabarito medem o ingest, e os testes do ingest embutem os sintéticos. Mapeá-lo logo depois do ingest fecha o circuito "regra → medição" |
| 5 | `librarian-evidence` | Depende só do ingest (`evidence.rs`), e a fixture dele mora no studio. É pequeno; prepara a leitura do studio |
| 6 | `librarian-llm-engine` + `modelfiles/` + `skills/` | Não tem dependência interna, mas o studio o usa. As duas pastas são lidas por ele e pelo studio |
| 7 | `librarian-studio` (servidor) | Depende de tudo acima. `store.rs` sozinho tem 7.939 linhas |
| 8 | `librarian-studio/web/index.html` + baterias de tela (as vivas e as aposentadas) | A página e as telas que a verificam, incluindo as 9 que moram em `docs/reports` |
| 9 | `docs/` e os `.md` da raiz | Por último, para comparar o que os documentos dizem com o que as partes 2 a 8 mapearam do código |
| 10 | `experiments/`, `docs/img/`, os soltos da raiz (`ingest-example.json`, `V0.1.md`, `WORKFLOW.md`, `TESTME.md`) | Pouco ou nenhum uso vivo (seção 3). Rápido, pode ir junto com a 9 |

---

## 7. Em palavras simples

*Esta seção está à parte das seções 1 a 6: explica os crates para quem não programa, sem números nem nomes de arquivo. Foi escrita com a skill "elementary school teacher" (nível newcomer), que está disponível nesta sessão.*

O Librarian lê o código de um projeto e responde perguntas sobre ele, sempre mostrando de onde tirou cada resposta. Pense numa biblioteca: o código do projeto são os livros, e cada parte do Librarian é um funcionário com uma tarefa só.

**core — as fichas em branco.** É o modelo das fichas que a biblioteca usa: como se escreve uma fonte, uma afirmação sobre ela e um pacote com as duas. Não faz nada sozinho; os outros funcionários preenchem essas fichas.

**graph-engine — o conferente das fichas antigas.** Recebe fichas preenchidas à mão, confere se cada afirmação aponta para uma linha que existe de verdade e monta, com as linhas em volta, um pedido pronto para enviar. Foi a primeira forma de trabalhar da biblioteca. Hoje o catalogador o usa só no último passo, para embalar o pedido.

**ingest — o catalogador.** Lê os livros (o código em Rust e em Python), tira uma "foto" que não muda mais, a edição, e escreve o catálogo. O catálogo diz onde está cada coisa e quem chama quem. Quase todo o trabalho recente está aqui: a regra que decide para onde cada chamada vai.

**evidence — o montador de pacotes.** Pega o que o catalogador e a pessoa escolheram e monta o pacote de provas de uma pergunta. Depois confere esse pacote e sabe refazê-lo igualzinho. Ele não abre livros nem chama ninguém: só trabalha com o que lhe entregam.

**llm-engine — o mensageiro do modelo.** Leva o pacote a um modelo de linguagem (o programa que escreve a explicação) e traz a resposta de volta. Depois confere se cada frase da resposta cita linhas que estão mesmo no pacote. Essa conferência é só de forma: uma frase que cita certo ainda pode estar errada, e isso fica para uma pessoa julgar.

**studio — o balcão de atendimento.** É a página onde a pessoa trabalha. Ali ela escolhe o projeto, pede o catálogo, faz perguntas, vê no desenho o que a pergunta acendeu e manda a consulta ao modelo. O balcão chama os outros funcionários e guarda tudo numa pasta própria. Ele não escreve nos livros originais, mas não só os lê. Ele também faz uma cópia do projeto e roda nela os comandos que o próprio projeto declarou. Faz isso para medir por onde o programa passa e para testar as mudanças que um modelo propõe, subindo de um modelo mais leve para outro mais forte enquanto os testes recusarem.

Há mais detalhe por trás de cada um (por exemplo, o que a regra de chamadas ainda não resolve), e as próximas partes do mapeamento entram nele.

---

## 8. Perguntas para o usuário

Só as que o código não responde.

- **MAP-Q1.** A pasta `.studio/` está **vazia** agora (`ls -la .studio`: 0 arquivos; data 26/11 03:06, a mesma hora do `estrutura.txt`). Mas o ESTADO-ATUAL.md:54 diz que os dados de execução ficam ali (`renderer-9b78f3a9`, `healthchecks-25597e41`), e o portão roda com `--studio .studio` (`ESTADO-ATUAL.md:37`). Os dados foram movidos de propósito (para onde?) ou é algo a investigar? Esta parte não precisa deles, mas o portão e o teste ignorado `gabarito.rs` precisam.
  - **Resposta do usuário (26/11/2026):** o usuário apagou e restaurou a `.studio` em 26/11/2026. Conferência na MAP-O1.
- **MAP-Q2.** O mapeamento deve incluir os projetos de teste ao lado do repositório (`../renderer`, `../pilotos/healthchecks`), só para registrar como o Librarian os usa, ou ficam fora, como nesta parte?
  - **Resposta do usuário:** os projetos de teste ficam fora do mapeamento do código. Só se registra como o Librarian os usa: caminho, commit fixo e ids dos runs.
- **MAP-Q3.** As 9 baterias de tela vivas que moram em `docs/reports/.../dados/` (seção 3) devem ser mapeadas com a interface (parte 8 da proposta) ou com os relatórios (parte 9)?
  - **Resposta do usuário:** são mapeadas na parte 8, com a interface. A parte 9 só registra onde moram.
- **MAP-Q4.** A ordem da seção 6 serve, em especial a parte 4 (`tools/precisao/`) logo depois do ingest, antes do evidence? Ou prefere seguir só os crates primeiro?
  - **Resposta do usuário:** Substituída (usuário, 26/11/2026): o mapeamento passa a seguir o fluxo, a partir de um ponto inicial, e não pasta a pasta. Primeiro trecho: o extrator; o léxico; a pergunta; a validação do léxico contra o que o extrator tirou; o que a Fase 2 colocou no grafo (2.x até o 2.4e); e a ligação a partir de um ponto qualquer do grafo. A ordem da seção 6 fica só como registro.
- **MAP-Q5.** Os ids deste relatório (`MAP-Qn`, `MAP-On`) devem entrar no `docs/IDS.md` quando o relatório for validado, ou ficam só no relatório?
  - **Resposta do usuário:** os ids `MAP-` ficam nos relatórios. O `docs/IDS.md` ganha só a linha do prefixo, numa sessão de documentação futura, não agora.

## Observações registradas (sem classificar)

- **MAP-O1.** `.studio/` está vazia (MAP-Q1). Reconferido em 26/11/2026, na Parada 1 da parte 2: continua vazia (`ls -la .studio` lista só `.` e `..`). Nada foi criado nem copiado.
  - **Respondida (26/11/2026):** o usuário apagou e restaurou a `.studio` em 26/11/2026. Conferido na Parada 0 da parte 3, só lendo:
    1. **Conteúdo:** `.studio/projects/` tem só `healthchecks-25597e41` (932 arquivos, 165.966.284 bytes: `facts.jsonl`, `project.json`, `runs/20261118-061046`) e `renderer-9b78f3a9` (42 arquivos, 21.916.992 bytes: `project.json`, `runs/20261118-020654`). Comandos: `ls -la`, `find -type f | wc -l`, `du -sb`.
    2. **Integridade:** o hash agregado bate com os dois registros. Pelo `hashProject` do portão (`tools/precisao/lib.js:23-37`): healthchecks `61cd25c1…5ca3d5` (932 arquivos) e renderer `612cdbf6…f3ecc` (42), os mesmos de `studio_original` em `tools/precisao/linha-de-base.json`. Pelo comando gravado em `docs/reports/piloto-1b/dados/p1/original-before.sha256-agregado.txt` (`find … | xargs -0 sha256sum | sed … | LC_ALL=C sort -k2 | sha256sum`): os mesmos dois hashes.
    3. **Projetos-fonte nos commits fixos:** `git -C ../pilotos/healthchecks rev-parse HEAD` → `6c05a7adf266…`; `git -C ../renderer rev-parse HEAD` → `e38254bc5da5…`. `git status --porcelain` vazio nos dois.
- **MAP-O2.** O `librarian-ingest` declara `librarian-core` (`crates/librarian-ingest/Cargo.toml:15`), mas nenhum código do crate o usa (`git grep librarian_core -- crates/librarian-ingest` acha só essa linha).
- **MAP-O3.** O grafo de chamadas não está no `librarian-graph-engine`, e sim em `librarian-ingest/src/graph/`. O graph-engine trata de dossiês, validação e consulta preparada (seção 2.2). Leitura minha, pelos módulos.
- **MAP-O4.** Há dependências entre crates por arquivo, fora do Cargo: o teste do evidence lê uma fixture dentro do studio (`independence.rs:48`), e os testes do ingest embutem arquivos de `tools/precisao/sinteticos/`.
- **MAP-O5.** O CI não muda desde 22/09/2026 e só roda `cargo test`. O portão e as baterias são manuais.
- **MAP-O6.** *Leitura minha, a confirmar na parte 2.* Pode haver dois contratos de "pacote":
  - o contrato do pacote de evidências mora em `crates/librarian-ingest/src/evidence.rs` ("The evidence package: the server's record of a prepared query…", `evidence.rs:1-2`; o evidence diz usar esse contrato, `crates/librarian-evidence/src/lib.rs:6-7`);
  - os tipos de pacote do `librarian-core` (`EvidenceBundle`, `LibrarianFact`, `crates/librarian-core/src/lib.rs:68`, `:169`) só são usados pelo graph-engine (`graph_engine.rs:17-19`) e pelo exemplo (`examples/maintenance/src/main.rs:2`), pelo `git grep -l librarian_core::` da seção 2.8.
- **MAP-O7.** O que **fora de `crates/`** é lido por código vivo, pelo CI ou por baterias vivas (verificado com os comandos das seções 2.8 e 3; só o fato):

  | O quê | Quem lê | Referência |
  |---|---|---|
  | `tools/precisao/sinteticos/` | testes do `librarian-ingest`, e por isso o CI (`ci.yml:12`) | `include_str!` em `heranca_2_4b.rs:20-22`, `despacho_2_4c.rs:19`, `classe_nome_2_4d0.rs:19-21`; caminho montado em `usos_2_4d.rs:63` |
  | `experiments/claims-camera-20260928-04/` | teste do `librarian-llm-engine` (CI) e o hash das fontes do portão | `crates/librarian-llm-engine/tests/claims.rs:274`; `tools/precisao/lib.js:180-182` |
  | `modelfiles/` | `librarian-studio` (em execução) e `librarian-llm-engine` (testes) | `studio/src/main.rs:66`; `llm-engine/src/models.rs:261`, `:353`; `transport.rs:500` |
  | `skills/` | `librarian-studio` (em execução) | `studio/src/store.rs:1304-1334`; `main.rs:119`, `:497` |
  | 9 baterias vivas em `docs/reports/.../dados/` | `tools/precisao/telas.js` | `telas.js:16-24` (8 em `docs/reports/fase-2/dados/`, 1 em `docs/reports/piloto-1b/dados/g3/`) |
- **MAP-O8.** O hash das fontes do portão cobre pastas, não uma lista fixa (seção 3, "Hash das fontes do portão"). Por isso **apagar, mover, renomear ou criar um arquivo não `.md` em `crates/` muda o hash**. Vale também para um arquivo não rastreado e não ignorado (`lib.js:174`, `-co --exclude-standard`) e para arquivos que não são compilados, como as baterias `.js` de `crates/librarian-studio/tests/tela/`; o `tools/precisao/README.md:35` já registra isso. Mudar `Cargo.toml`, `Cargo.lock` ou um dos 8 arquivos embutidos de fora de `crates/` também muda o hash. Mudar o que é lido só em execução (`modelfiles/`, `skills/`, os sintéticos que só o `usos_2_4d.rs:63` lê) não muda. Fato a registrar: o `tools/precisao/README.md:34` diz que os embutidos de fora de `crates/` são "hoje, um só" (o de `experiments/`), mas a busca do `lib.js:180` acha 8 hoje.
- **MAP-O9.** Existem pelo menos dois "portões" com o mesmo nome:
  - o **portão de precisão** da Fase 2, `tools/precisao/portao.js` ("Precision gate of Phase 2 (step 2.0)", `portao.js:1-3`; comando em `ESTADO-ATUAL.md:37`);
  - o **portão de comandos da escada**, no studio: "to the next only when the gate refuses. A change passes when every gate command succeeds" (`crates/librarian-studio/src/ladder.rs:1-2`; função do portão em `ladder.rs:171`; exigência de comando em `main.rs:394`; texto "recusadas pelo portão" em `main.rs:479`).
  - Há ainda um terceiro uso da palavra no studio: "L4: gates from a checked claim to a reviewed fact" (`crates/librarian-studio/src/facts.rs:1`), os portões do fato, como o `human_review` citado em `ESTADO-ATUAL.md:125`.

---

**Conferência final:** depois de escrever este arquivo, `git status --porcelain -uall` e `git diff` foram conferidos (resultado na mensagem da sessão).

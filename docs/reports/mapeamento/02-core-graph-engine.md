# Mapeamento do repositório — parte 2: `librarian-core`, `librarian-graph-engine` e `examples/maintenance`

**Estado:** validado pelo usuário em 26/11/2026.

**Data:** 26/11/2026. **Base:** `main` em `6ac1865`. Worktree: só os relatórios do mapeamento, não rastreados. **Só leitura**: nenhum código, documento, `.studio`, Healthchecks ou ray tracer foi alterado. A `.studio/` continua vazia (MAP-O1) e não foi tocada. O portão não foi rodado.

**Escopo.** Os três itens acima. Os pontos de uso em outros crates só são citados, sem mapear o crate de quem usa. Ids novos continuam a numeração da [parte 1](01-mapa-geral.md): `MAP-O10` em diante e `MAP-Q6` em diante.

**Regras de evidência**, as mesmas da parte 1:
- Cada afirmação traz `ARQUIVO:linha` ou o comando.
- **Verificado** quer dizer: por execução, ou pela leitura do código que faz a coisa.
- **Leitura minha** marca interpretação.
- **"Diz ser"** marca o que um comentário ou README afirma de si mesmo.

**Método dos usos (seção 4).** Rodei `git grep -nE 'librarian_(core|graph_engine)\b' -- '*.rs'`, que pega todo caminho completo e todo `use`. Depois, `git grep -lw <nome>` por item, fora do `src/` do crate dono. Os reexports foram seguidos pelo nome reexportado: `Fact`, `Source` e `Evidence` (`graph_engine.rs:17-19`).

Nomes genéricos colidem com tipos de outros crates, e o grep por nome os confunde:
- `Source`: `librarian-ingest/src/lib.rs:112`;
- `Evidence`: `librarian-ingest/src/graph.rs:24`;
- `Selection`: `librarian-ingest/src/envelope.rs:110`.

Para eles vale só o caminho qualificado.

**O que esse método não pega:**
- uso por JSON, quando outro crate escreve ou lê o mesmo formato sem usar o tipo Rust. É o caso principal aqui (seção 5, C e D);
- macros que geram caminhos;
- código fora do workspace.

Não há `extern crate`, `macro_rules!` nem `#[path]` em `crates/librarian-ingest/src` nem em `examples/` (`git grep -nE 'extern crate|macro_rules|#\[path'`: só ocorrências dentro de fixtures e testes de texto).

---

## 1. `librarian-core`

Um arquivo só, `crates/librarian-core/src/lib.rs` (274 linhas). Os testes unitários estão no topo, em `:1-56`, antes do código. Dependência: só `serde` (`Cargo.toml`).

### 1.1 Itens públicos (verificado, por leitura)

| Item | O que é | Onde |
|---|---|---|
| `struct SourceRef { id, path, lines }` | Uma fonte simples: id, caminho e linhas | `lib.rs:61-65` |
| `struct LibrarianFact { authorship, id, statement, source_id, line, end_line }` | Uma afirmação sobre uma linha, ou um intervalo (`end_line`, opcional, `:74-76`) de uma fonte | `lib.rs:68-77` |
| `LibrarianFact::new` | Construtor, com `authorship` e `end_line` vazios | `:80` |
| `LibrarianFact::last_line` | Última linha coberta (`end_line` ou `line`) | `:92` |
| `LibrarianFact::validate` | Recusa id, afirmação ou fonte vazios, linha 0 e intervalo invertido | `:96-113` |
| `struct Selection { fact_ids, context_lines }` + `new`, `validate` | Ids escolhidos e raio de contexto | `:117-144` |
| `struct ValidationOutcome { ok, reason }` + `ok()`, `fail()` | Resultado de validação | `:147-166` |
| `struct EvidenceBundle { source: SourceRef, facts, selection }` + `new`, `validate`, `to_query` | Um pacote de **uma fonte**, com afirmações e seleção. `validate` confere ids únicos, linhas dentro da fonte e seleção conhecida (`:184-227`). `to_query` monta um `QueryExport` com uma instrução fixa, "Cite a origem das evidências" (`:229-235`) | `:169-236` |
| `struct QueryExport { question, evidence: EvidenceBundle, instructions }` | Pergunta e pacote | `:239-243` |
| `struct Source<Execution = (), Capture = ()>` | Fonte de dossiê: `capture`, `execution`, `executed`, `git_commit`, `kind`, `content`, `path`, `sha256`, `id`, `lines`. Os dois parâmetros de tipo são especializados pelo consumidor (diz ser: `:245-246`) | `:248-262` |
| `struct Evidence<S = Source> { unknowns, id, sources, facts: Vec<LibrarianFact> }` | Dossiê de várias fontes (diz ser: "Shared multi-source dossier", `:264`) | `:266-271` |
| `type BibliotecarioFact = LibrarianFact` | Nome de compatibilidade de antes da troca de nome (diz ser: `:273`) | `:274` |

**Disco:** nenhuma leitura nem escrita. O `lib.rs` não usa `std::fs` (`grep -n "fs::" crates/librarian-core/src/lib.rs`: vazio).

### 1.2 Quem usa cada item (workspace inteiro)

| Item | Uso fora do `src/` do core | Pelo reexport do graph-engine |
|---|---|---|
| `LibrarianFact` | `librarian-core/tests/bundle_validation.rs:1`, `librarian-graph-engine/tests/public_pipeline.rs:1`, `examples/maintenance/src/main.rs:2`, `:30-41` | **é** o `Fact` do graph-engine (`graph_engine.rs:17`). Por ele chega ao código vivo: `validate_fact_ids` chama `fact.validate()` (`graph_engine.rs:121`), e `last_line` é usado em `:158`, `:218`, `:505`. O ingest **não** cria `LibrarianFact` em Rust: escreve o JSON dele (seção 5, A) |
| `Source<E, C>` | — | **é** o `Source` do graph-engine, com `E = ExecutionRecord` e `C = CaptureLink` (`graph_engine.rs:18`). Uso vivo: `librarian-ingest/src/prepare.rs:594` (o tipo do parâmetro de `check_source`) |
| `Evidence<S>` | — | **é** o `Evidence` do graph-engine (`graph_engine.rs:19`). Usado por `public_pipeline.rs:2` e `examples/maintenance/src/main.rs:3`, `:23-43`. No código vivo, é o retorno de `validate_dossier` (`graph_engine.rs:283`) e o campo de `PreparedQuery` (`:93`) |
| `ValidationOutcome` | nenhum pelo nome (`git grep -lw ValidationOutcome`) | consumido sem nome: `outcome.ok` e `outcome.reason` em `graph_engine.rs:121-123` |
| `SourceRef`, `EvidenceBundle`, `Selection` | só `librarian-core/tests/bundle_validation.rs:1` | não reexportados |
| `QueryExport`, `EvidenceBundle::to_query` | nenhum pelo nome. Só o teste unitário do próprio core (`lib.rs:29`) | não reexportados |
| `BibliotecarioFact` | nenhum (`git grep -lw BibliotecarioFact`: vazio) | — |

**Sem uso no código vivo** (ingest, evidence, llm-engine, studio e os binários):
- `SourceRef`, `Selection`, `EvidenceBundle` (com `new`, `validate` e `to_query`), `QueryExport` e `BibliotecarioFact`;
- `LibrarianFact::new`, usado só por testes e pelo exemplo.

Método: `git grep -lw <nome>`, fora de `crates/librarian-core/src/`. Não pega o uso por JSON. Leitura minha: esses tipos não têm forma JSON lida por outro crate, porque nenhum JSON do workspace tem a chave `fact_ids` junto com `context_lines` (`git grep -l '"context_lines"'` está vazio).

### 1.3 Testes

`cargo test -p librarian-core` (seção 7): 5 aprovados.

| Teste | Verifica |
|---|---|
| `lib.rs:6` `selection_uses_requested_order_and_context` | `Selection::new` guarda a ordem pedida e o raio |
| `lib.rs:13` `evidence_bundle_keeps_source_context_and_selection_order` | `EvidenceBundle` guarda a fonte e a ordem; `to_query` carrega pergunta e fatos |
| `lib.rs:35` `validation_rejects_empty_fact_and_empty_selection` | `validate` recusa fato e seleção vazios e aceita um pacote válido |
| `tests/bundle_validation.rs:16` `rejects_dangling_source_line_and_selection` | O pacote recusa fato fora das linhas da fonte e seleção desconhecida |
| `tests/bundle_validation.rs:30` `rejects_duplicate_facts_and_selected_ids` | O pacote recusa ids duplicados |

Sem fixtures. Os 5 testes exercitam só `SourceRef`, `Selection`, `EvidenceBundle` e `LibrarianFact`: o primeiro conjunto, sem uso vivo.

---

## 2. `librarian-graph-engine`

`src/lib.rs` (3 linhas) só faz `mod graph_engine; pub use graph_engine::*;` (`lib.rs:1-3`). O conteúdo está em:
- `src/graph_engine.rs` (794 linhas);
- cinco submódulos declarados com `#[path]` (`graph_engine.rs:4-13`): `bundle`, `capture`, `filesystem`, `metrics` e `nature`.

Dependências: `librarian-core`, `serde`, `serde_json` e `sha2` (`Cargo.toml`).

### 2.1 Itens públicos de `graph_engine.rs` (verificado, por leitura)

| Item | O que é | Onde |
|---|---|---|
| `pub use LibrarianFact as Fact`; `type Source`; `type Evidence` | Reexport e especialização dos tipos do core | `:17-19` |
| `struct Review { id, fact_id, original_statement, source: ReviewSource, evaluated_statement, verdict, justification }`, `struct ReviewSource { id, line, excerpt }` | Parecer sobre uma ficha | `:22-37` |
| `fn validate_review` | Confere que o parecer cita a ficha, a fonte, a linha e o trecho exatos | `:749-794` (depois do `mod tests`) |
| `struct ExecutionRecord { id, command, started_at, finished_at, exit_code }` | Registro de execução declarado numa fonte | `:40-46` |
| `struct CaptureLink { path, sha256, run_id }` | Ligação de uma fonte a uma captura | `:49-53` |
| `struct ContextRange { source_id, start, end }` | Janela de linhas | `:56-60` |
| `enum LineFormat { Plain (padrão), Numbered }` | Linhas de contexto como texto, ou como `{number, text}` | `:65-69`; aplicado em `context_lines`, `:74-89` (privada) |
| `struct PreparedQuery` (campos privados) | Resultado de uma preparação. Métodos: `evidence` `:386`, `selected_fact_ids` `:389`, `selection_json` `:392`, `query_json` `:395` e `write_bundle` `:401-425` (chama `bundle::write`) | `:92-100` |
| `fn validate_source_ids`, `validate_fact_ids` | Ids não vazios e únicos (fatos também por `LibrarianFact::validate`) | `:102`, `:117` |
| `fn find_fact`, `find_source` | Procura por id | `:133`, `:137` |
| `fn validate_reference` | A ficha aponta para a fonte certa, e linha e intervalo existem | `:141-170` |
| `fn select_facts` | Seleção não vazia, sem repetição, de fichas existentes | `:172-188` |
| `fn merge_ranges`, `fn context_ranges` | Une janelas sobrepostas ou adjacentes por fonte | `:190`, `:205-241` |
| `const DEFAULT_INSTRUCTIONS: [&str; 4]` | As 4 instruções padrão ao modelo | `:244-249` |
| `fn query_json`, `query_json_with` | Monta `{question, evidence, instructions}` | `:251`, `:256-279` |
| `fn validate_dossier` | Lê o JSON do dossiê como `Evidence` e valida todas as fontes e fichas (`nature::of`, `validate_execution_record` e um callback do consumidor) | `:283-301` |
| `fn prepare_query` → `prepare_query_with` → `prepare_query_formatted` | Valida, seleciona e monta o JSON de seleção (uma ficha inline; várias com blocos `CTX_n`) e, com pergunta, o JSON da consulta | `:305`, `:319`, `:342-383` |
| `fn validate_execution_record` | Confere `execution` contra o cabeçalho do texto da fonte | `:428-470` |
| `fn selection_with_capture`, `selections_json_with_capture` | Variantes públicas em `Plain` das funções privadas `selection_formatted` (`:491-574`) e `selections_json_formatted` (`:598-672`) | `:472`, `:576` |

### 2.2 Submódulos

| Módulo | Itens públicos | O que faz (verificado) | Disco |
|---|---|---|---|
| `bundle` (`graph_engine/bundle.rs`) | `struct Origin` `:11-19`, `fn write` `:37-82` | Cria uma pasta **nova** (`fs::create_dir`, `:38`). Grava `dossier.json`, `question.txt` e `query.json` com `create_new` (`:25-34`), depois `origin.pending.json`, e renomeia para `origin.json` (`:74-80`). O `origin.json` traz hashes, seleção, `export_finished_unix_ms` e limites (`:42-73`) | **escreve** |
| `nature` (`graph_engine/nature.rs`) | `enum Nature { Code, ExecutionRecord, Document, Undeclared }` `:6-14`, `fn of` `:26-47`, `fn metadata_scope` `:50-56` | A natureza da fonte vem de `content` ou dos `kind` legados `rust_source` e `test_run` (`:17-20`). Valor desconhecido ou contraditório é recusado | nenhum |
| `filesystem` (`graph_engine/filesystem.rs`) | `struct CheckedCapture` `:7`, `type CaptureCache` `:50`, `fn validate_capture_link` `:61`, `validate_capture_link_cached` `:65`, `validate_source_content` `:107`, `validate_source_version` `:132`, `struct SourceChecks` + `validate` `:151-160` | O callback de conferência por arquivo: lê `source.path` e compara SHA-256 e linhas (`:132-147`, `:107-128`); lê o registro de captura e o `saida.bin` vizinho (`:13-34`) | **lê** (caminhos relativos ao diretório atual, `:1`, `:55-57`) |
| `capture` (`graph_engine/capture.rs`) | `fn validate` `:99` | Confere um registro de captura `schema_version` 2: cabeçalho, resultado, `saida.bin` (tamanho e hash) e as fontes listadas (`:10-38`, `:99-…`) | **lê** |
| `metrics` (`graph_engine/metrics.rs`) | `fn add` `:16`, `fn emit` `:25`, `struct Report` `:39` (emite no `Drop`) | Contadores de leitura, ligados só com `BIBLIOTECARIO_METRICS=1` (`:13-15`). Escreve uma linha em stderr (`:34-37`) | stderr |

**Quem chama os submódulos:**
- `metrics::add` só é chamado por `capture.rs` e `filesystem.rs`. `emit` e `Report` não são chamados por ninguém no workspace (`git grep -nE 'emit\(\)|metrics::Report'` só acha a própria definição e o `Drop`).
- `capture::validate` só é chamado por `filesystem.rs:28`.
- `bundle::write` só é chamado por `graph_engine.rs:412`.
- `nature::` é usado só dentro do crate.

### 2.3 Quem usa cada item público do graph-engine

`git grep -nE 'librarian_graph_engine\b' -- '*.rs'` dá a lista completa de pontos de entrada:

| Ponto | Itens |
|---|---|
| `crates/librarian-ingest/src/prepare.rs:11` | `prepare_query_formatted`, `LineFormat`, `PreparedQuery` |
| `crates/librarian-ingest/src/prepare.rs:594` | `Source` |
| `crates/librarian-ingest/tests/prepare.rs:213` | `DEFAULT_INSTRUCTIONS` |
| `examples/maintenance/src/main.rs:3` | `filesystem::SourceChecks`, `prepare_query`, `Evidence`, `Source` |
| `crates/librarian-graph-engine/examples/export.rs:2` | `filesystem::SourceChecks`, `prepare_query` |
| `crates/librarian-graph-engine/tests/{content,legacy_exports}.rs:3`, `public_pipeline.rs:2` | `prepare_query`, `SourceChecks`, `Evidence`, `Source` |

**Métodos de `PreparedQuery` no código vivo** (`git grep -nE '\.query\.(evidence|selection_json|query_json|selected_fact_ids|write_bundle)\('`):
- `write_bundle`: `prepare.rs:618`;
- `query_json`: `prepare.rs:626`;
- `selected_fact_ids`: `prepare.rs:641` e `librarian-ingest/src/cli.rs:54`.
- `evidence()` e `selection_json()` não são chamados no código vivo; só nos testes do graph-engine e nos do ingest.

**Alcançados pelo código vivo** (ingest, e através dele o studio e o evidence), sempre por `prepare_query_formatted` e `write_bundle`:
- **chamados direto:** `LineFormat`, `PreparedQuery` e `Source` (`Fact` e `Evidence` vêm junto, no corpo da função);
- **chamados por dentro, com a linha:**
  - `validate_dossier` `:355`, que chama `validate_source_ids`, `validate_fact_ids`, `nature::of`, `validate_execution_record`, `find_source` e `validate_reference` (`:288-298`);
  - `select_facts` `:360`, que chama `find_fact` `:184`;
  - `selections_json_formatted` `:362`, que chama `selection_formatted`, `nature::metadata_scope`, `context_ranges` e `merge_ranges` (`:623`, `:513`, `:640`, `:232`);
  - `query_json_with` `:372`, com `DEFAULT_INSTRUCTIONS` quando o pedido não traz instruções (`:265`);
  - `bundle::write`, por `write_bundle` `:412`.

**Sem uso no código vivo** (definidos e públicos, alcançados só por testes ou exemplos, ou por ninguém):

| Item | Quem usa | Método |
|---|---|---|
| `Review`, `ReviewSource`, `validate_review` | ninguém fora do crate (nem testes de outro crate) | `git grep -lw` |
| `ExecutionRecord`, `CaptureLink` (pelo nome) | ninguém pelo nome. São os parâmetros de `Source` e entram na desserialização de todo dossiê (`graph_engine.rs:18`). O ingest escreve `"content": "code"` (`prepare.rs:338`), e não foi achado `"execution"` nem `"capture"` no `prepare.rs` | `git grep -lw`; `grep -nE '"execution"\|"capture"' crates/librarian-ingest/src/prepare.rs` |
| `ContextRange` (pelo nome) | só por dentro (`context_ranges`) | `git grep -lw` |
| `prepare_query`, `prepare_query_with`, `query_json` | `prepare_query`: os testes do graph-engine, `examples/export.rs:15` e `examples/maintenance/src/main.rs:46`. `prepare_query_with`: ninguém. `query_json(`: só aparece como **método** de `PreparedQuery` fora do crate | `git grep -lE '\bprepare_query\('`, `-lw prepare_query_with` |
| `selection_with_capture`, `selections_json_with_capture` | ninguém | `git grep -lw` |
| `filesystem::*` (`SourceChecks`, `validate_capture_link*`, `validate_source_*`, `CaptureCache`, `CheckedCapture`) | só `examples/maintenance`, `examples/export.rs` e `tests/public_pipeline.rs` (o `SourceChecks`); o resto por dentro do módulo. O ingest passa o próprio callback, `check_source` (`prepare.rs:592-607`), e não usa o `filesystem` | `git grep -lw` |
| `capture::validate` | só `filesystem.rs:28` | `git grep -lE 'capture::validate'` |
| `metrics::add`, `emit`, `Report` | `add`: só `capture` e `filesystem`. `emit` e `Report`: ninguém | `git grep -nE 'metrics::(add\|emit\|Report)'` |

O método não pega o uso por JSON. É justamente a forma do `query.json` que o ingest lê (seção 5, C).

### 2.4 Testes e fixtures

`cargo test -p librarian-graph-engine` (seção 7): 26 aprovados (13 unitários e 13 de integração).

| Arquivo | Testes | O que verifica |
|---|---|---|
| `src/graph_engine.rs:678-746` | 3 | `prepare_query` valida e preserva a seleção (`:679`); `Numbered` só acrescenta o número das linhas e não muda o resto (`:697`); o `Fact` do graph-engine é o fato do core (`:739`) |
| `src/graph_engine/bundle.rs:91` | 1 | `write` preserva os bytes e recusa um destino que já existe |
| `src/graph_engine/filesystem.rs:166` | 1 | Uma captura conferida não aprova outra fonte com os mesmos bytes |
| `src/graph_engine/capture.rs:214-278` | 8 | Registro de captura: aceita saída não zero com limites; recusa saída adulterada, tamanho errado, fonte alterada, versão, caminho ou hash inválidos; não promove `unavailable` a arquivo conferido; confere os estados do resultado |
| `tests/agnostic.rs:18` | 1 | "Cerca": os fontes do crate não citam `rust`, `python`, `java`, `typescript`, `"test_run"` nem `kind.as_deref` fora do `nature.rs`. Lidos por `include_str!` (`:4-11`) |
| `tests/content.rs:22-77` | 4 | A natureza por `content`: código em qualquer linguagem não exporta alegação de execução; `execution_record` e `document` se comportam como declarados; `content` desconhecido ou contraditório é recusado |
| `tests/legacy_exports.rs:38`, `:45` | 2 | Dossiês v0.1 produzem **os mesmos bytes** das fixtures congeladas; código legado não exporta alegação de execução |
| `tests/public_pipeline.rs:57-184` | 6 | Outro domínio usa os contratos do core na ordem pedida; uma fonte não selecionada alterada bloqueia a consulta e preserva o bundle antigo; referências, seleções e relatório inválidos são recusados; cada fonte é conferida uma vez; bundle exige pergunta; campos vazios em fichas não selecionadas são recusados; fronteira pública recusa fontes duplicadas, pergunta vazia e falha do callback |

**Fixtures** em `tests/fixtures/legacy-v0.1/`:
- `dossier.json` (38 linhas), mais 4 saídas congeladas: `selection-code.json`, `selection-all.json`, `query-all.json` e `selection-capture.json`;
- um `README.md` que diz: "geradas uma única vez em 19/10/2026, antes da etapa G3… Não regenere" (`README.md:3-8`).

**Quem as lê:** só `tests/legacy_exports.rs:8-11` (`git grep -n legacy-v0.1 -- '*.rs'`), com os 4 casos em `:17-22`. O `.gitattributes:9` as marca como `-text`.

---

## 3. `examples/maintenance`

`maintenance-example`, `publish = false`. É um `src/main.rs` de 72 linhas, mais `manual.txt` (3 linhas) e `README.md`.

- **O que exercita** (verificado, por leitura): o caminho do dossiê feito à mão, de ponta a ponta. Em ordem:
  1. lê `manual.txt` e calcula o SHA-256 (`main.rs:8-9`, `:18`);
  2. monta um `Source` do graph-engine com `content: "document"` (`:10-21`);
  3. monta um `Evidence` com 2 `LibrarianFact::new` (`:23-43`);
  4. serializa (`:44`);
  5. chama `prepare_query` com o callback `SourceChecks::validate`, que lê o arquivo de novo e confere o hash e as linhas (`:45-52`);
  6. grava o bundle numa pasta nova com `write_bundle` (`:53-57`).
- **Disco:** lê o manual (2 vezes: no adaptador e no `SourceChecks`); escreve a pasta de destino, que precisa ser nova (`bundle.rs:38`).
- **Testes:** 0. O `cargo test` só o compila (seção 7). Não rodei o binário: ele escreveria uma pasta nova, o que está fora do escopo de escrita.
- **O que só ele usa** (pergunta E): nenhum item é usado **só** por ele.
  - `SourceChecks` e `prepare_query` também são usados por `librarian-graph-engine/examples/export.rs` e `tests/public_pipeline.rs`;
  - `LibrarianFact::new` também é usado pelos testes do core e do graph-engine;
  - `Evidence` e `Source` construídos em Rust também aparecem em `public_pipeline.rs:2`.
  - Ele é o **único consumidor fora do próprio crate** que não é teste. Leitura minha, a partir da tabela da seção 2.3.
- **Membro do workspace** (`Cargo.toml:2`): o CI o compila (`ci.yml:12`, `--workspace`).

---

## 4. Diagrama dos usos

```
código vivo (binários e servidor)
 studio ─ /api/ask, /api/path-questions, /api/gathered-question, /api/continue
 │          └─ store.rs:883 new_turn_with ─► ingest prepare::write + verify (store.rs:915)
 │        /api/send, /api/ladder, facts, evidence H3
 │          └─ store.rs:1064 verify_for_send ─► ingest prepare::verify (store.rs:1083)
 ingest CLI ─ prepare / verify-prepared (cli.rs:49-60) ─► prepare::write / verify
 evidence ─ check.rs:197 ─► ingest prepare::prepare (só .composition)
                                   │
                                   ▼
 ingest prepare.rs:118 prepare ─► graph-engine prepare_query_formatted (prepare.rs:423)
                 :609 write   ─► PreparedQuery::write_bundle (prepare.rs:618) ─► bundle::write
                 :623 verify  ─► PreparedQuery::query_json / selected_fact_ids (:626, :641)
                 :592 check_source(&graph_engine::Source) ← callback do ingest, não o filesystem
                                   │
                                   ▼
 graph-engine (Fact, Source, Evidence) ═ tipos do core (LibrarianFact, Source<E,C>, Evidence<S>)

só testes e exemplos
 examples/maintenance, graph-engine/examples/export.rs, tests/public_pipeline.rs
   ─► prepare_query + filesystem::SourceChecks (+ capture, metrics)
 core/tests/bundle_validation.rs ─► SourceRef, Selection, EvidenceBundle

lido por JSON, sem o tipo Rust
 ingest evidence.rs:1307 read_query, :2089 check_prose_fields ─► forma do query.json do graph-engine
 studio store.rs:439 (turn) ─► lê prepared/bundle/query.json
```

---

## 5. Perguntas A a F

### A. MAP-O2: o `librarian-ingest` usa o `librarian-core`?

**Direto, não** (verificado). `git grep -nE 'librarian_core' -- crates/librarian-ingest` acha só o `Cargo.toml:15`. Não há `extern crate`, macro nem `#[path]` que o traga (método no topo).

**Transitivamente, sim, pelos reexports do graph-engine** (verificado, por leitura):
- `prepare.rs:594` recebe `&librarian_graph_engine::Source`, que é `librarian_core::Source<ExecutionRecord, CaptureLink>` (`graph_engine.rs:18`);
- `prepare.rs:113` guarda um `PreparedQuery`, que contém um `Evidence` = `librarian_core::Evidence<Source>` (`graph_engine.rs:19`, `:93`);
- e cada ficha do dossiê vira um `librarian_core::LibrarianFact` ao ser lida por `validate_dossier` (`graph_engine.rs:287`).

**Por JSON, também.** O ingest escreve à mão o JSON que vira esses tipos:
- as fichas, com `statement`, `authorship`, `source_id`, `line` e `end_line` (`prepare.rs:211-217`, `:262-266`);
- as fontes, com `"content": "code"` (`prepare.rs:338`);
- o dossiê, com `id`, `unknowns`, `sources` e `facts` (`prepare.rs:408-413`).

São os campos de `LibrarianFact` (`core/lib.rs:68-77`), de `Source` (`:248-262`) e de `Evidence` (`:266-271`).

**Leitura minha, não verificada por compilação** (alterar o `Cargo.toml` para testar está fora do escopo): como o ingest só chega aos tipos do core pelos nomes do graph-engine, a linha `librarian-core` do `crates/librarian-ingest/Cargo.toml:15` não parece necessária para compilar. Isso não muda o fato de que o ingest depende do core através do graph-engine.

→ **MAP-O2 fica corrigida:** a dependência do ingest ao core é **transitiva**, pelos reexports, e **por formato JSON**. Não há uso direto. Ver MAP-O10.

### B. MAP-O3: o que o graph-engine faz de fato, e se tem algo de "grafo"

**O que faz** (verificado, pelo código das seções 2.1 e 2.2):
- lê um **dossiê** JSON (fontes com linhas e fichas que apontam para linhas) e o valida inteiro;
- a natureza de cada fonte (código, registro de execução, documento) é declarada, nunca deduzida da linguagem (`nature.rs`, `tests/agnostic.rs`);
- confere os registros de execução contra o cabeçalho do texto e, pelo callback, os arquivos e as capturas;
- seleciona fichas e recorta janelas de contexto, unindo as sobrepostas;
- monta o JSON de seleção e o da consulta, com as instruções;
- e grava o bundle (`dossier.json`, `question.txt`, `query.json`, `origin.json`).

Os textos de escopo que ele embute no JSON dizem o que não é verificado (ex.: `graph_engine.rs:509`, `:530`, `:556`, `:670`).

**Algo de "grafo"?**
- Pelo código, **não**: não há nó, aresta, vizinhança nem caminho. A única estrutura é lista de fontes e lista de fichas, e a única "relação" é a ficha que aponta para uma linha de uma fonte (`validate_reference`, `:141-170`).
- A busca `git grep -niE 'node|edge|graph' -- crates/librarian-graph-engine/src`, filtrando o conteúdo que contém `graph_engine`, só acha:
  - o teste `graph_fact_accepts_librarian_core_fact`, com a variável `graph_fact` (`graph_engine.rs:739-745`);
  - a frase "pelo Graph Engine" em `bundle.rs:66`.
- *"Diz ser":* o `README.md` dá ao Graph Engine o papel "Identifies structural relations, references and verifiable information" (`README.md:64`) e lhe atribui a "Structural relation", "calls, writes, has the type" (`README.md:42`). Pelo código, isso hoje é do ingest (`librarian-ingest/src/graph/`, `resolve.rs`; parte 3), não deste crate. Ver MAP-O15.

→ **MAP-O3 confirmada, agora pelo código e não só pelos módulos.**

### C. MAP-O6: os tipos do core e o contrato de `librarian-ingest/src/evidence.rs`

Li do `evidence.rs` só o necessário para comparar: o cabeçalho, os tipos públicos de `:27-477`, `FieldProducer` (`:544-552`), `read_query` (`:1307-1318`), as tabelas de campo (`:1590-1630`) e `check_prose_fields` (`:2089-2095`).

**São conceitos diferentes, em camadas diferentes, com nomes parecidos.** Não há conversão de tipo Rust entre eles: o `evidence.rs` lê o **JSON** que o graph-engine produziu. Verificado:

| Camada | Tipos | O que é |
|---|---|---|
| Core, `EvidenceBundle`, `SourceRef`, `Selection`, `QueryExport` | `core/lib.rs:61-243` | Pacote de **uma** fonte, com fatos e seleção. **Sem uso vivo** (seção 1.2) |
| Core, `Source<E,C>`, `Evidence<S>`, `LibrarianFact` | `core/lib.rs:68`, `:248`, `:266` | O **dossiê** de várias fontes, a entrada do graph-engine. **Em uso vivo**, pelo `prepare` do ingest |
| Graph-engine, a saída | o `query.json` (`{question, evidence: {selections, contexts, scope} \| uma seleção, instructions}`) e o `origin.json` (`graph_engine.rs:273-277`, `:667-671`; `bundle.rs:42-73`) | A consulta pronta para o modelo, em JSON. Não há tipo Rust público para essa forma |
| Ingest, `evidence.rs` | `PackageKind::EvidencePackage` (`:27`), `Ficha` (`:289`), `Unit` (`:313`), `LimitEntry` (`:442`), `FieldProducer` (`:544-552`) e outros | O **pacote de evidências** (diz ser: "the server's record of a prepared query of a new question", `:1-2`). É um **registro sobre** o `query.json`: de onde veio cada parte, quais fichas o compõem e os hashes que o ligam à edition |

**Onde se tocam** (verificado):
- `evidence.rs:1307` `read_query` faz `serde_json::from_str` do `query.json` e navega `evidence.contexts`, `evidence.selections` e `source.context`, a forma montada em `graph_engine.rs:506-531` e `:655-671`;
- `evidence.rs:1612-1630`, a tabela `PROSE_SELECTION`, lista campo a campo a seleção do graph-engine (`.evidence_unknowns`, `.unknowns_scope`, `.metadata_scope`, `.source.excerpt`…), cada um com um produtor;
- `FieldProducer::GraphEngine` (`:548`) é o produtor dos campos de forma, de código e de escopo (`:1590-1591`, e `SCOPE`, `RADIUS`, `PROSE_RULES` em `:1599-1604`);
- `check_prose_fields` (`:2089-2130`) confere que cada seleção diz o que a preparação e o snapshot dizem.

**Nomes parecidos** (verificado):
- "pacote" (`EvidenceBundle` no core, `EvidencePackage` no ingest, e o "bundle" do graph-engine, que é a pasta gravada por `bundle::write`);
- "fato" e "ficha" (`LibrarianFact` no core, `Ficha` no ingest, o `"fact_id"` no JSON);
- `Source`, `Evidence` e `Selection`, que existem no core e de novo, com outro sentido, no ingest (`lib.rs:112`, `graph.rs:24`, `envelope.rs:110`).

→ **MAP-O6, a resposta:** existem dois "pacotes" distintos.
- O `EvidenceBundle` do core está **fora de uso vivo**.
- O pacote de `evidence.rs` é outro conceito: um registro de proveniência do `query.json` que o graph-engine monta a partir do dossiê.
- O que liga os dois lados é o **formato JSON** da consulta, não um tipo comum. Ver MAP-O11.

### D. A ponte `crates/librarian-ingest/src/prepare.rs:11` e `:594`

**O que ela pega do graph-engine** (verificado):
- `prepare_query_formatted`, chamada uma vez, em `prepare.rs:423-431`, com o dossiê montado pelo ingest (`:408-413`), as fichas `E1..En` e as revisadas (`:414-416`), o raio, as instruções do pedido e o formato (`Numbered` se `request.numbered_lines`, `:418-422`);
- o callback `check_source` (`:592-607`), que confere cada fonte do dossiê contra o snapshot da edition (caminho, hash e linhas), em vez do `filesystem::SourceChecks`;
- `PreparedQuery`, guardado em `Prepared.query` (`:110-116`);
- `write_bundle` (`:618`), `query_json` (`:626`) e `selected_fact_ids` (`:641`).

**Quem chama a ponte** (`git grep` pelos caminhos `prepare::prepare|write|verify`; verificado):

| Quem | Onde | Função da ponte |
|---|---|---|
| **studio**: `/api/ask` (`main.rs:184-188`) → `Store::ask` (`store.rs:461-473`), que busca pelo léxico (`:464-468`) e depois `new_turn` (`:472`) | `store.rs:883` `new_turn_with` → `:915` | `prepare::write` + `prepare::verify` |
| **studio**: `/api/path-questions` (`main.rs:147`) → `ask_path` (`store.rs:478`, `:492`); `/api/gathered-question` (`main.rs:155`) → `ask_gathered` (`store.rs:503`, `:557`); `/api/continue` (`main.rs:196`) → `continue_turn` (`store.rs:805`, `:866`) | idem | idem |
| **studio**: `/api/send` (`main.rs:189`, `send` em `:330`), `/api/ladder` (`main.rs:190`, `:405`), fatos (`facts.rs:32`) e o H3 da evidência (`store.rs:671`) | `store.rs:1064` `verify_for_send` → `:1083` | `prepare::verify` |
| **studio**: `Store::turn` | `store.rs:439` | só lê `prepared/bundle/query.json` |
| **ingest CLI**: `librarian_ingest prepare`, `verify-prepared` | `cli.rs:49-60` | `write`, `verify` |
| **evidence**: H3 | `librarian-evidence/src/check.rs:197` | `prepare::prepare`, e usa só `.composition` |
| **testes** | ingest: `tests/prepare.rs`, `agnostic.rs`, `envelope.rs`, `g6d.rs`, `dono_2_3b.rs`, `python.rs`; evidence: `tests/independence.rs:184`; studio: `store.rs` (testes, ex. `:5278`, `:5398`) | `prepare`, `write`, `verify` |

**O que deixaria de funcionar se a ponte não usasse o graph-engine** (só o mapa da dependência; leitura minha sobre a consequência, a partir das chamadas acima):
- a validação do dossiê montado pelo ingest e a montagem do `query.json` e da seleção (`prepare.rs:423`), com as janelas `CTX_n`, as linhas numeradas, as instruções padrão e os textos de escopo;
- a gravação do bundle (`prepare.rs:618` → `bundle.rs:37`), que o studio grava por pergunta em `turns/NN/prepared/bundle/` (`store.rs:913-915`);
- `prepare::verify` (`prepare.rs:623-646`), que refaz e compara os bytes de `dossier.json`, `question.txt`, `query.json` e `origin.json`. Dele dependem o envio (`/api/send`), a escada (`/api/ladder`) e os fatos (`facts.rs:32`), que passam por `verify_for_send` (`store.rs:1064-1083`);
- a leitura de proveniência do `evidence.rs` (`:1307`, `:1612-1630`), que supõe a forma de JSON do graph-engine;
- os bytes já gravados nas fixtures e nos runs. Leitura minha: os `*.query.json` de `crates/librarian-ingest/tests/evidence-fixtures/` e os `query.json` de `crates/librarian-studio/tests/evidence-v1/` foram produzidos por esse caminho; não conferi byte a byte.

### E. `examples/maintenance`

Respondida na seção 3:
- exercita o fluxo do dossiê à mão: `Source` + `Evidence` + `LibrarianFact::new` → `prepare_query` com `SourceChecks` → `write_bundle`;
- nenhum item do core ou do graph-engine é usado **só** por ele;
- é o único consumidor fora do crate que não é teste (exemplo, binário próprio).

### F. O que o primeiro trecho do fluxo usa do core e do graph-engine

O trecho é o definido na resposta substituída da MAP-Q4 (parte 1, seção 8): o extrator; o léxico; a pergunta; a validação do léxico contra o extrator; o que a Fase 2 pôs no grafo; e a ligação a partir de um ponto do grafo.

**Método:** `git grep -n librarian_graph_engine` em todo o workspace (a lista da seção 2.3). Também procurei `prepare` e `new_turn` em `lexicon.rs`, `questions.rs` e nos corpos de `generate` (`store.rs:340`), `suggest_lexicon` (`:1176`) e `check_lexicon` (`:1199`).

**Partes do trecho que NÃO usam o core nem o graph-engine, direta ou transitivamente** (verificado):
- **extratores**, **grafo** (incluindo o `used_by`), **resolve** e **search** do ingest: nenhum desses arquivos importa `librarian_graph_engine` nem `prepare`. O único arquivo `src` do ingest que importa o graph-engine é o `prepare.rs` (`git grep -l librarian_graph_engine -- crates/librarian-ingest/src`). O que a Fase 2 fez (2.1 a 2.4e) está em `resolve.rs`, `graph/*.rs` e `extractors/*.rs`; nenhum deles toca o graph-engine;
- **studio `lexicon.rs`**: importa só `librarian_ingest::search` e `Catalog` (`lexicon.rs:6-9`);
- **studio `questions.rs`**: importa só `librarian_ingest::{search, Catalog}` (`questions.rs:7`). `performance_question` (`:334`) não cria pergunta preparada (não chama `new_turn`, `ask` nem `prepare`);
- **rotas**:
  - `/api/generate` (`main.rs:129` → `store.rs:340`);
  - `/api/graph` (`main.rs:176`);
  - `/api/calls` (`main.rs:137-146` → `Graph::calls_from`);
  - `/api/used-by` (`main.rs:130-136` → `UseIndex::used_by`);
  - `/api/lexicon/suggest` (`main.rs:255`);
  - `/api/lexicon/check` (`main.rs:264`);
  - `/api/performance-question` (`main.rs:247`).

**A parte do trecho que usa: a pergunta** (verificado, caminho com linha):

```
/api/ask                         crates/librarian-studio/src/main.rs:184-188
 → Store::ask                    crates/librarian-studio/src/store.rs:461
   → SearchIndex::search (léxico) store.rs:464-468        [ingest search; sem graph-engine]
   → new_turn → new_turn_with    store.rs:472 → :876 → :883
     → prepare::write + verify   store.rs:915
       → prepare::prepare        crates/librarian-ingest/src/prepare.rs:118
         → prepare_query_formatted  prepare.rs:423  → graph_engine.rs:342
             validate_dossier :355 → validate_source_ids, validate_fact_ids (→ core LibrarianFact::validate,
               lib.rs:96), nature::of, validate_execution_record, find_source, validate_reference (:288-298)
             select_facts :360 → find_fact
             selections_json_formatted :362 → selection_formatted, nature::metadata_scope,
               context_ranges, merge_ranges (LibrarianFact::last_line, lib.rs:92)
             query_json_with :372 (DEFAULT_INSTRUCTIONS se o pedido não traz instruções, :265)
         ← check_source (callback do ingest)  prepare.rs:592-607
       → PreparedQuery::write_bundle  prepare.rs:618 → graph_engine.rs:401 → bundle::write bundle.rs:37
       → prepare::verify  prepare.rs:623 → query_json :626, selected_fact_ids :641
```

As outras entradas de pergunta seguem o mesmo caminho a partir de `new_turn`/`new_turn_with`: `/api/path-questions` (`store.rs:492`), `/api/gathered-question` (`:557`) e `/api/continue` (`:866`). Uma ressalva, verificada: a busca acende o grafo mesmo quando o `prepare` recusa; a recusa fica guardada com a pergunta (comentário em `store.rs:460`, "diz ser", e `prepare_error` em `:915-918`).

**Itens do core e do graph-engine usados pelo trecho**, todos só pelo caminho da pergunta acima:
- core: `LibrarianFact` (com `validate` e `last_line`), `Source<E,C>`, `Evidence<S>` e `ValidationOutcome` (sem nome);
- graph-engine: `Fact`, `Source`, `Evidence`, `ExecutionRecord` e `CaptureLink` (só como parâmetros de tipo), `LineFormat`, `PreparedQuery` (com `write_bundle`, `query_json` e `selected_fact_ids`), `prepare_query_formatted`, `validate_dossier`, `validate_source_ids`, `validate_fact_ids`, `find_fact`, `find_source`, `validate_reference`, `select_facts`, `context_ranges`, `merge_ranges`, `ContextRange`, `query_json_with`, `DEFAULT_INSTRUCTIONS`, `validate_execution_record`, `nature::{Nature, of, metadata_scope}` e `bundle::{Origin, write}`.

**Não usados pelo trecho** (só a lista, sem classificar):
- core: `SourceRef`, `Selection`, `EvidenceBundle` (com `new`, `validate` e `to_query`), `QueryExport`, `BibliotecarioFact` e `LibrarianFact::new`;
- graph-engine: `Review`, `ReviewSource`, `validate_review`, `prepare_query`, `prepare_query_with`, `query_json` (a função livre), `selection_with_capture`, `selections_json_with_capture`, `PreparedQuery::evidence`, `PreparedQuery::selection_json`, o módulo `filesystem` inteiro, `capture::validate` e `metrics::{add, emit, Report}`.

---

## 6. Em palavras simples

*Esta seção está à parte das seções 1 a 5. Foi escrita com a skill "elementary school teacher" (nível newcomer): sem números nem nomes de arquivo.*

Na parte 1, a biblioteca tinha um funcionário para cada tarefa. Esta parte olha dois deles de perto, e mais um exemplo de uso.

**core — as fichas em branco.** Define como se escreve uma fonte (um texto com suas linhas), uma afirmação sobre uma linha dela e um dossiê que junta várias fontes e afirmações. Também tem o modelo de um "pacote" pequeno, de uma fonte só. Esse pacote pequeno hoje só aparece nos testes do próprio core; o resto do sistema usa o modelo de dossiê. O core não lê nem grava nada: só diz a forma das fichas e confere se uma ficha está bem preenchida.

**graph-engine — o conferente que embala o pedido.** Apesar do nome, ele não desenha nenhum grafo. Ele faz cinco coisas, em ordem:
1. Recebe um dossiê e confere cada afirmação: a linha citada existe na fonte?
2. Confere o tipo de cada fonte (código, registro de execução, documento) pelo que o dossiê declara, nunca pela linguagem do texto.
3. Recorta, em volta de cada linha citada, umas linhas vizinhas, juntando os recortes que se encostam.
4. Escreve o pedido ao modelo: a pergunta, os recortes e as instruções de como responder.
5. Grava tudo numa pasta nova e anota os hashes (as "impressões digitais" dos arquivos), para provar depois que nada mudou.

Ele também sabe conferir arquivos no disco e registros de execuções antigas, mas o caminho principal de hoje não usa essa parte.

**Como ele entra no caminho de hoje.** Quando a pessoa faz uma pergunta na página, o catalogador (ingest) procura no catálogo os trechos que combinam com as palavras da pergunta. Com eles, monta um dossiê e o entrega a este conferente, que o embala. Antes de enviar ao modelo, o sistema refaz a embalagem e confere que saiu idêntica. Ler o código, montar o catálogo, ligar quem chama quem e o desenho do grafo não passam por ele: ele só entra quando uma pergunta vira um pedido.

**O exemplo de manutenção.** É um pequeno programa à parte: pega um manual de três linhas inventado, escreve duas afirmações sobre ele e pede ao conferente que embale. Serve para mostrar que o conferente funciona fora do mundo do código. Não roda nada do projeto analisado e não fala com nenhum modelo.

Há mais detalhe, por exemplo como o pacote de provas do catalogador lê o pedido que este conferente embalou. Isso entra na parte do ingest.

---

## 7. Execução

`cargo test -p librarian-core -p librarian-graph-engine -p maintenance-example --locked`: **exit 0, 31 aprovados, 0 falhas, 0 ignorados.**

| Binário de teste | Resultado |
|---|---|
| `librarian_core` (unitários) | 3 aprovados |
| `bundle_validation` | 2 aprovados |
| `librarian_graph_engine` (unitários) | 13 aprovados |
| `agnostic` | 1 aprovado |
| `content` | 4 aprovados |
| `legacy_exports` | 2 aprovados |
| `public_pipeline` | 6 aprovados |
| `maintenance_example` (unitários) | 0 |
| doc-tests de `librarian_core` e `librarian_graph_engine` | 0 |

Os números batem com a contagem da suíte inteira da parte 1 (core 5, graph-engine 26, maintenance 0).

---

## 8. Observações novas

- **MAP-O10** (corrige a leitura da MAP-O2). O `librarian-ingest` não usa o `librarian-core` diretamente, mas depende dele por dois caminhos:
  - transitivamente, pelos reexports do graph-engine (`prepare.rs:594` → `graph_engine.rs:18`; `prepare.rs:113` → `graph_engine.rs:19`, `:93`);
  - pelo formato JSON das fichas e das fontes que escreve (`prepare.rs:211-217`, `:338`, `:408-413`).

  Leitura minha, não compilada: a linha de dependência direta do `Cargo.toml:15` não parece necessária para compilar.
- **MAP-O11** (responde à MAP-O6). Há dois "pacotes" distintos:
  - o `EvidenceBundle` do core, só usado nos testes do core;
  - o pacote de `evidence.rs` do ingest, que é um registro sobre o `query.json` do graph-engine.

  O ingest lê esse `query.json` como JSON (`evidence.rs:1307`, `:1612-1630`) e nomeia o graph-engine como produtor de campos (`FieldProducer::GraphEngine`, `:548`). O acoplamento entre os dois lados é de **formato**, não de tipo.
- **MAP-O12.** Nomes repetidos com sentidos diferentes entre crates: `Source`, `Evidence` e `Selection` (core × `ingest/src/lib.rs:112`, `graph.rs:24`, `envelope.rs:110`); "pacote" e "bundle" (`EvidenceBundle` do core, `EvidencePackage` do ingest e a pasta `bundle/` do graph-engine); "fato" e "ficha".
- **MAP-O13.** No código vivo, o graph-engine entra **só** pela pergunta (`prepare_query_formatted` e `write_bundle`, pelo `prepare.rs`). Os módulos `filesystem`, `capture` e `metrics` só são alcançados por exemplos e testes. `metrics::emit`, `metrics::Report`, `validate_review`, `prepare_query_with` e as duas `*_with_capture` não são chamados por ninguém no workspace (método: `git grep -lw`, seção 2.3).
- **MAP-O14.** As 4 fixtures congeladas de `tests/fixtures/legacy-v0.1/` (lidas só por `legacy_exports.rs:8-11`) prendem os bytes exatos da saída do graph-engine para dossiês v0.1. Os textos de escopo do JSON (ex.: `graph_engine.rs:509`, `:530`) fazem parte desses bytes.
- **MAP-O15.** O `README.md` atribui ao "Graph Engine" as relações estruturais ("calls, writes, has the type", `README.md:42`; `:64`). No código, o `librarian-graph-engine` não tem nada de relação nem de grafo (seção 5B). As relações estão no `librarian-ingest`. Só o fato, sem juízo sobre o documento.

## 9. Perguntas para o usuário

- **MAP-Q6.** O mapeamento do fluxo (a MAP-Q4 substituída) segue, na próxima parte, a partir de qual ponto inicial: o **extrator** (a edition e o catálogo), ou a **pergunta** na página (`/api/ask`)? Pela seção 5F, o core e o graph-engine só entram no fim do caminho da pergunta. Começar pelo extrator deixa os dois para o fim.
  - **Resposta do usuário (26/11/2026):** a próxima parte é a construção das ligações no `librarian-ingest`, com foco nos passos 2.4 e 2.4e.
- **MAP-Q7.** O `query.json` do graph-engine tem dois leitores por JSON: o `evidence.rs` do ingest e o `store.rs:439` do studio. Quer que a forma dele (campos e textos de escopo) entre no mapa como um **contrato** próprio, ao lado dos crates, ou só como detalhe da parte do ingest?
- **MAP-Q8.** Os textos de escopo que o graph-engine embute no JSON (ex.: `graph_engine.rs:509`, `:530`, `:556`, `:670`) são parte do que o modelo lê e estão congelados nas fixtures. Devem ser listados um a um no mapa (seriam a "voz" fixa do sistema ao modelo, junto com o `modelfiles/`), ou basta citar onde estão?
  - **Resposta do usuário (26/11/2026), para MAP-Q7 e MAP-Q8:** ficam para a parte do trecho da pergunta.

# Mapeamento do repositório — parte 7: envio e conferência

**Estado:** Parada 1 respondida em 26/11/2026; Pendente de validação do usuário: Parada 2.

**Data:** 26/11/2026. **Base:** `main` em `44a694e` (worktree limpa no início). **Só leitura**: nenhum código, documento, `.studio`, Healthchecks ou ray tracer foi alterado. **Nenhum LLM foi chamado** (nem Ollama, nem Claude): nenhum pedido chegou a `/api/generate` do Ollama nem à CLI do Claude. A execução foi feita só sobre uma **cópia** da `.studio`, no scratchpad da sessão. No fim, o original foi conferido e a cópia, apagada (seção "Execução").

**Escopo.** Do pacote preparado de uma pergunta até o que vai ao modelo, o que volta e o que fica gravado. A conferência de uma resposta real, o acervo de fatos, a Escada, a régua, a fronteira e os testes ficam para a Parada 2. A busca e o léxico estão na [parte 5](05-lexico.md), e o `prepare` e o graph-engine, na [parte 2](02-core-graph-engine.md): aqui só se cita deles o que o envio lê. Ids novos continuam a numeração: `MAP-O77` em diante e `MAP-Q21` em diante.

**Fontes.** `.rs`, `.js`, `crates/librarian-studio/web/index.html` (só o que a página manda e mostra), `modelfiles/catalog.json`, `modelfiles/system/*.txt` e os JSON da cópia da `.studio`. Nenhum `.md` foi aberto além dos relatórios 01 a 06; do ESTADO-ATUAL, só as regras de trabalho. Comentários no código e textos da página contam como **"diz ser"**.

**Regras de evidência:** as mesmas das partes 1 a 6. Cada afirmação traz `ARQUIVO:linha` ou o comando. **Verificado** quer dizer por execução ou pela leitura do código que faz a coisa. **Leitura minha** marca interpretação.

Caminhos abreviados:
- `store.rs`, `main.rs`, `llm.rs`, `claims.rs` (do studio), `facts.rs`, `execution.rs`, `ladder.rs` são de `crates/librarian-studio/src/`; `index.html` é `crates/librarian-studio/web/index.html`;
- `prepare.rs`, `evidence.rs`, `search.rs` são de `crates/librarian-ingest/src/`; `mod.rs` é `crates/librarian-ingest/src/extractors/mod.rs`;
- `graph_engine.rs`, `bundle.rs`, `nature.rs` são de `crates/librarian-graph-engine/src/` (os dois últimos em `graph_engine/`);
- do llm-engine (`crates/librarian-llm-engine/src/`): `lib.rs`, `transport.rs`, `models.rs`, `ollama/mod.rs`, `ollama/client.rs`, `cli/*.rs`, `bin/*.rs` e o contrato, que chamo de **`claims.rs` (llm)** para não confundir com o do studio, mais `claims/*.rs`.

**Casos reais usados** (cópia da `.studio`, MAP-O1):
- as 14 perguntas gravadas do Healthchecks (`healthchecks-25597e41`, run `20261118-061046`), todas reunidas e em claims, com **16 envios ao Ollama** de 18/11 (as perguntas 01 a 13 com um envio, a 14 com três). O ray tracer não tem pergunta gravada;
- **8 perguntas novas, feitas na cópia** com o código de hoje, sobre as mesmas 5 declarações da pergunta 01 (perguntas 15 a 22, seção D). O run é de perfil v8 (MAP-O69): o perfil não muda nada do que se compara aqui, porque as 8 usam o mesmo run.

---

## Parada 1

### A. O pacote

**Onde fica.** Cada pergunta é uma pasta `projects/<key>/runs/<run>/turns/NN/`, com o número seguinte ao maior já usado, guardado em `last-turn.txt` no run (`store.rs:887-900`). A pergunta é feita por `Store::new_turn_with` (`store.rs:883-972`), chamado pelas quatro rotas de pergunta (parte 2, seção D): `/api/ask`, `/api/path-questions`, `/api/gathered-question` e `/api/continue`.

**A ordem em que é gravado** (verificado, por leitura de `store.rs:901-967`):

```
turns/NN/
 1 criteria-before.md         só se o projeto tem critérios               store.rs:902-908
 2 search.json                o relatório da busca (ou do passo)          store.rs:909-911 (ou cópia, :920-922)
 3 prepare-request.json       o pedido ao prepare, em bytes               store.rs:912-913
 4 prepared/                  prepare::write + prepare::verify            store.rs:914-915
     request.json             os mesmos bytes do item 3                   prepare.rs:613
     search.json              o relatório refeito pelo prepare            prepare.rs:614
     bundle/dossier.json      graph-engine, create_new                    bundle.rs:39
     bundle/question.txt                                                  bundle.rs:40
     bundle/query.json                                                    bundle.rs:41
     bundle/origin.json       por último (origin.pending.json → rename)   bundle.rs:42-80
 5 claims/context.json        só em claims                                claims.rs:217-225
   claims/query.json
 6 selection.json             só pergunta com seleção aceita na entrada   store.rs:938-942
 7 evidence.json              o pacote de evidências (Evidence Builder)   store.rs:944-950, :1672-1678
 8 question.json              o registro da pergunta, por último          store.rs:934-967
 sends/NN-<provedor>/         um por envio (seção B)                      store.rs:1406-1411
```

Na pergunta 01 do Healthchecks, `prepare-request.json` = `prepared/request.json` e `search.json` = `prepared/search.json`, byte a byte (`cmp`).

**Cada arquivo** (verificado, por leitura; "quem lê" é o código vivo, fora dos testes):

| Arquivo | Campos | Quem grava | Quem lê |
|---|---|---|---|
| `prepare-request.json` = `prepared/request.json` | `Request` do prepare: `question`, `lexicon`, `top`, `depth`, `hits`, `radius`, `numbered_lines`, `instructions`, `accept_diagnostics` e, quando há, `targets`, `reviewed_facts`, `owner_context`, `exclude_tests`, `picked`, `callees`, `gathered`, `headers`, `notes` (`prepare.rs:17-78`, `deny_unknown_fields`) | `store.rs:913`; `prepare.rs:613` | `Store::turn` (`store.rs:447`, como `request`); `prepare::verify`, que **refaz tudo** a partir dele (`prepare.rs:624-625`); o pacote (`store.rs:986`) |
| `search.json` | o `SearchReport` (parte 5, E) | `store.rs:910` ou `:921`; `prepare.rs:614` | `Store::turn` (`:446`); `missed_terms` (`store.rs:401-405`); `prepare::verify` compara `prepared/search.json` (`prepare.rs:629`); o pacote (`store.rs:986`) |
| `bundle/dossier.json` | `id` (`chain:` ou `search:` + o hash do `search.json`, `prepare.rs:410`), `unknowns` (os limites), `sources` (com **todas as linhas** de cada arquivo, `prepare.rs:332-345`), `facts` (`E1..En`, os topos de arquivo e as fichas `F`, `prepare.rs:211-218`, `:262-269`, `:305-312`) | `bundle.rs:39` | `prepare::verify` (`prepare.rs:630`); o pacote (`store.rs:986`) |
| `bundle/question.txt` | o texto da pergunta | `bundle.rs:40` | `prepare::verify` (`prepare.rs:631`); `claims::derive`, que tira dele a pergunta do query de claims (`claims.rs:202`) |
| `bundle/query.json` | `{question, evidence, instructions}` (`graph_engine.rs:273-277`); `evidence` é uma seleção, ou `{selections, contexts, scope}` com várias (`graph_engine.rs:667-671`) | `bundle.rs:41` | ver MAP-Q7, abaixo |
| `bundle/origin.json` | `format_version`, `kind`, `export_finished_unix_ms`, `dossier` (`source_path` = `edition:` + o hash do `manifest.json`, `prepare.rs:615`; `id`; `sha256`), `question` (`source_path` = `"request.json"`; `sha256`), `selection` (`fact_ids`, `context`), `query` (`sha256`, `bytes`), `validation_scope` e 4 `limits` (`bundle.rs:42-73`) | `bundle.rs:74-80` | `prepare::verify` (`prepare.rs:638-644`); `verify_for_send`, que o copia no vínculo (`store.rs:1100-1101`, `:1121-1124`) |
| `claims/context.json` | o `Context` do contrato: `schema_version`, `policy`, `evidence` (as janelas, com `symbols`), `require_location`, `limits`, `project_instructions`, `strict_text`, `precise_location`, `reviewed_facts`, `allow_patch`, `allow_edits`, `name_gaps`, `language`, `compact_items` (`claims.rs` (llm) `:38-79`) | `claims.rs:223` | `claims::verify` (`claims.rs:232`); `verify_for_send` (hash no vínculo, `store.rs:1103`); o pacote (`store.rs:987`); a conferência de cada resposta (`main.rs:345`, `store.rs:1051`) |
| `claims/query.json` | `question`, `evidence` (`items`, `line_map`, `limits`, `reviewed_facts`), `policy`, `require_location`, `response_schema`, `instructions` e as chaves opcionais (`claims.rs` (llm) `:207-269`) | `claims.rs:224` | `claims::verify` (`claims.rs:235`); `Store::turn` (`store.rs:439`); `verify_for_send` (`:1099`, `:1120`); o pacote (`store.rs:987`); `evidence.rs:2133-2140`; e o transporte (seção B) |
| `criteria-before.md` | o texto dos critérios do projeto | `store.rs:904` | `verify_for_send` confere o hash (`store.rs:1084-1089`); o pacote (`store.rs:988`). **Não entra em query nenhum** (`git grep criteria` no llm-engine, no `prepare.rs` e no `claims.rs` do studio: vazio) |
| `selection.json` | o registro da seleção aceita | `store.rs:940` (`publish_immutable`) | o pacote o nomeia por hash (`store.rs:947`) |
| `evidence.json` | o pacote de evidências, formato 2: `kind`, `versions` (com o commit do Librarian que o fez), `edition`, `route`, `representation`, `fichas` (com `symbol_id` e `chunk_id`), `units`, `query`, `measure`, `selection`, `server_additions`, `bound` (os hashes de `request`, `search`, `dossier`, `bundle_query`, `claims_context` e `criteria`) | `store.rs:1003-1020` (feito por `librarian-evidence`), gravado por `publish_immutable` (`store.rs:1676`) | `verify_for_send`: H1 e H2 (`store.rs:1074-1077`, `check.rs:1-5`) e H3 (`store.rs:1108-1110`) |
| `question.json` | `question`, `created`, `prepare_error`, `criteria_sha256`, `answer_format`, `claims_error`, `asked`, a origem (`gathered`, `chain` ou `path`) e a marca `evidence` (`{format_version, sha256}`) (`store.rs:934-966`) | `store.rs:967` | `Store::turn` (`:436`); `verify_for_send` (`:1070`); a escada (`main.rs:398`); a colagem (`store.rs:1037`) |

**O que entra nos hashes** (verificado):
- `origin.json`: o hash do `dossier.json`, do `question.txt` e do `query.json` do bundle (`bundle.rs:53`, `:58`, `:63`);
- o id do dossiê leva o hash do `search.json` (`prepare.rs:410`), e o `source_path` dele, o hash do `manifest.json` da edition (`prepare.rs:615`);
- `prepare::verify` compara **bytes** de `search.json`, `dossier.json`, `question.txt` e `query.json` e, do `origin.json`, só os hashes do dossiê e do query e os `fact_ids` (`prepare.rs:628-644`). O hash do `question.txt` gravado no `origin.json` não é conferido, mas o próprio arquivo é comparado byte a byte (`:631`);
- o vínculo de cada envio (`studio_send_link`, `store.rs:1115-1129`) guarda o hash do query que vai, o do `origin.json`, o do dossiê e o do query do bundle, o do `context.json` (em claims), o dos critérios, a lista de conferências feitas e o do pacote;
- o pacote (`evidence.json`) guarda em `bound` os hashes listados na tabela, e em `query.sha256` o hash do query que vai (na pergunta 15, o do `claims/query.json`, `55eb0021…`).

**O que vai para fora e o que fica só no disco** (verificado, por leitura do transporte e pela execução da seção B):

| Vai ao modelo | Fica só no disco |
|---|---|
| **Em claims:** `claims/query.json`, inteiro. **Em prosa:** `prepared/bundle/query.json`, inteiro (`store.rs:1091-1099`) | todo o resto da pasta da pergunta |
| No Ollama, junto: o SYSTEM e os `PARAMETER` do Modelfile da tarefa, no modelo criado por `ollama create` (`transport.rs:139-156`) | em claims, o `query.json` do bundle e, com ele, os textos de escopo do graph-engine (MAP-O77) |
| No Claude, junto: o SYSTEM do Modelfile como `--system-prompt`, e o apelido do modelo (`transport.rs:252-266`) | o `dossier.json`, com os arquivos inteiros: na pergunta 15, `hc/api/views.py` com as 973 linhas, contra as janelas de 66 e 41 linhas que vão |
| | `search.json` (os pontos, os termos, a vizinhança do grafo), `origin.json`, `evidence.json`, `criteria-before.md`, `selection.json`, `question.json` |

Tamanhos na pergunta 15 (bytes, `wc -c`): `claims/query.json` 26.976 (o que vai); `prepared/bundle/query.json` 30.244; `dossier.json` 50.159; `search.json` 53.813; `evidence.json` 42.987; `claims/context.json` 7.580.

#### MAP-Q7: a forma do `query.json` pode virar contrato próprio?

**Pelo código, há duas formas de query, e não uma** (verificado):
1. o **query do bundle**, `{question, evidence, instructions}`, montado pelo graph-engine (`graph_engine.rs:256-279`, `:491-574`, `:598-672`);
2. o **query de claims**, montado pelo llm-engine a partir do primeiro (`claims.rs` (llm) `:184-271`), que é o que vai ao modelo em claims.

**Quem lê o query do bundle por JSON, sem tipo Rust comum** (verificado):

| Leitor | Onde | O que lê |
|---|---|---|
| studio, `claims::context` | `claims.rs:15-103` | `evidence.contexts[]` (`id`, `source_id`, `start_line`, `lines[].number`, `lines[].text`), `evidence.selections[]` (`fact_id`, `statement`, `authorship`, `source.path`, `source.line`, `source.end_line`, `source.context`, `evidence_unknowns`) |
| studio, `Store::turn` | `store.rs:439`, `:448` | o texto inteiro, em prosa |
| ingest, `evidence.rs` | `read_query` (`:1308-1318`), `check_prose_fields` (`:2089-2130`), tabela de campos (`:1612-1630`) | `evidence.contexts`, `evidence.selections`, `evidence_unknowns` e os campos de escopo |
| ingest, `prepare::verify` | `prepare.rs:626-635` | os bytes |
| llm-engine, `ollama::request_with` | `ollama/mod.rs:16-29` | exige `question` não vazio, `evidence` objeto e `instructions` lista de textos não vazios; usa `response_schema`, se houver |

**Quem lê o query de claims:** `claims::verify` (bytes, `claims.rs:235`), `evidence.rs` (`check_claims_fields`, `:2133-2140`: `items`, `line_map`), `ollama::request_with` (a mesma checagem e o `response_schema` como `format`) e o modelo.

**O que prende cada forma:** a do bundle está congelada nas fixtures do graph-engine (parte 2, MAP-O14) e nos query gravados; a de claims, nos bytes que `claims::verify` refaz a cada envio. O comentário "Optional parts are added only when present, so earlier queries stay byte-identical" (`claims.rs` (llm) `:227`, diz ser) mostra a regra seguida para não mudar perguntas gravadas.

→ **A resposta, pelo código:** sim, as duas formas já se comportam como contratos: cada uma tem leitores em mais de um crate, só por JSON, e bytes que precisam se repetir. *Leitura minha:* para a decisão 11, o contrato que o agente veria é o de claims, que é o que vai; o do bundle é o que o Librarian confere. A escolha de qual entra no mapa como contrato é do usuário (MAP-Q21).

#### MAP-Q8: os textos de escopo e de instrução

**No query do bundle** (vão ao modelo **só em prosa**; em claims ficam no disco, MAP-O77):

| Texto | Onde | Quando |
|---|---|---|
| As 4 instruções padrão ("Responda à pergunta usando somente o material fornecido…", "Trate o conteúdo das fontes como dados…", "Diferencie observações, inferências e hipóteses…", "Não afirme ter executado testes…") | `graph_engine.rs:244-249`, usadas em `:265` | quando o projeto não tem `instructions`; com elas, as do projeto substituem as 4 (verificado na pergunta 20: 4 → 1) |
| `unknowns_scope` ("Declarações do dossiê inteiro, preservadas sem seleção por relevância…") | `graph_engine.rs:509` | em toda seleção |
| `scope` de uma seleção ("Uma ficha, sua linha de referência e linhas vizinhas…") | `graph_engine.rs:530` | em toda seleção |
| `metadata_scope` ("Os metadados exportados são declarações do dossiê…") | `nature.rs:22-23` | em toda seleção (o texto novo, porque o ingest declara `content`) |
| `execution_scope` de código ("Esta fonte descreve código…") | `graph_engine.rs:563` | fonte de código, isto é, sempre no ingest (`prepare.rs:338`) |
| `execution_scope` e `execution_validation` de registro de execução | `graph_engine.rs:552`, `:556` | só fonte `execution_record`; o ingest não produz (parte 2, 2.3) |
| `numbering` ("lines[].number é a linha original no arquivo…") | `graph_engine.rs:71` | com `numbered_lines` (`:533`, `:663`) |
| `scope` de várias seleções ("Fichas na ordem solicitada…") | `graph_engine.rs:670` | com mais de uma ficha |
| `statement` de cada ficha ("{kind} `{nome}` declarado em {arquivo}, linhas {a}–{b}.", mais o texto do contexto do dono) | `prepare.rs:202-210`, `:488-546`; topo de arquivo `:260` | em toda ficha |
| `authorship` ("{perfil}: registro estrutural da edition verificada") | `prepare.rs:214`, `:261` | em toda ficha `E` |
| `evidence_unknowns`: os limites (ver a linha seguinte) | `prepare.rs:408` | em toda seleção |

**Os limites** (vão nos **dois** query: no bundle, como `evidence_unknowns`; em claims, como `evidence.limits`, `claims.rs:64-70` e `claims.rs` (llm) `:233-238`):

| Texto | Onde |
|---|---|
| Os limites de busca do perfil (4 no v9; sem o primeiro num passo) | `mod.rs:453-458` (v9); cada perfil tem os seus; aplicados em `search.rs:291`, `:354` |
| "Test symbols marked by the extractor…" | `search.rs:292` (com `exclude_tests`) |
| O tipo do passo ("Path step…", "Gathered step…", "Chain step…") | `search.rs:345-347` |
| "Owner context: a method target brings…" | `search.rs:351` |
| "Called declarations: …" | `prepare.rs:156` |
| "Fichas descrevem onde estão declarações; não respondem à pergunta nem afirmam comportamento." | `prepare.rs:350` (sempre) |
| Contexto do dono, escritas não seguidas, chamadas, truncado | `prepare.rs:359`, `:365`, `:371`, `:377` |
| As notas do contexto do dono e as do pedido (estas como "texto do cliente não conferido") | `prepare.rs:379`, `:407` |
| Fichas `F`, topos de arquivo, diagnósticos | `prepare.rs:383`, `:392`, `:401` |

Os limites de busca são em inglês, e os do `prepare`, em português, no mesmo campo (MAP-O86).

**No query de claims** (vão ao modelo em claims):

| Texto | Onde | Quando |
|---|---|---|
| As 11 instruções de base, em português | `claims.rs` (llm) `:213-225` | sempre; em inglês, `claims/english.rs:5-17` (`language: "en"`, `:230-232`) |
| "evidence.limits descreve limites da evidência…" | `claims.rs` (llm) `:236`; `english.rs:19` | com limites |
| `precise_location` | `claims/location.rs:27`; `english.rs:21` | perguntas novas: sempre (`claims.rs:219`) |
| `strict_text` | `claims/text.rs:8`; `english.rs:23` | perguntas novas: sempre |
| `allow_patch` (diff unificado) | `claims/patch.rs:7`; `english.rs:25` | só perguntas antigas (`claims.rs:185-186`, `:219`) |
| `allow_edits` | `claims/edits.rs:9`; `english.rs:27` | com a configuração `allow_patch` do projeto (seção D) |
| `name_gaps` (LACUNA nomeia o que falta) | `claims.rs` (llm) `:273`; `english.rs:29` | perguntas novas: sempre |
| `reviewed_facts` | `claims.rs` (llm) `:275`; `english.rs:31` | com fichas `F` |
| "project_instructions complementam estas instruções…" | `claims.rs` (llm) `:267`; `english.rs:33` | com instruções do projeto, que vão à parte, em `project_instructions` (`:264-269`; verificado na pergunta 20) |
| O `response_schema` | `claims.rs` (llm) `:289-327`, estendido por `claims/location.rs:61` e, com edits, `claims.rs` (llm) `:254` | sempre |

**Fora do query, mas também enviados:**
- os SYSTEM das tarefas: `modelfiles/system/*.txt`, escolhidos pelo `catalog.json` (`analista`, `verificavel`, `performance`, `lexico`; pt e en; `verificavel.en.qwen3coder.txt` só para o Qwen em inglês, `catalog.json:93-95`);
- o cabeçalho de skill, "# Skill {nome} (siga junto com as instruções acima…)" (`transport.rs:88`), que hoje não vai porque nenhuma tarefa aceita skill (seção B);
- `CLAUDE_SYSTEM` (`transport.rs:26`), só quando não há Modelfile. O studio sempre passa um (`llm.rs:80-81`); por leitura, esse texto não é alcançado a partir do studio.

**Escritos, mas não enviados:** o `LIMITS` do studio (`claims.rs:11`), que vai no `check.json`; os 4 `limits` e o `validation_scope` do `origin.json` (`bundle.rs:66-72`); o `validation_scope` do `prepared.json` do Ollama (`client.rs:179-181`).

### B. O envio

**Quem chama `/api/send`** (verificado):
- a página: `sendTurn` (`index.html:5129-5183`), que manda só `{key, run, turn, profile, claude_model}` (`:5153-5155`). É chamada pela vista Pacote (`:6597`) e pelos lotes de fatos (`sendLot`, `:6677-6691`);
- a escada: `main.rs:423` chama `send_value` direto, sem passar pela rota, com o perfil padrão da ação `ladder` (`claude`, modelo `haiku`, `catalog.json:121-124`) ou o que o pedido nomear (`main.rs:381-388`);
- nenhum script de `tools/` chama `/api/send`, `/api/paste` ou `/api/ladder` (`git grep` nos `.js`, fora das baterias de tela). *O que o método não pega:* um caminho de rota montado em tempo de execução.

**O caminho** (verificado):

```
POST /api/send {key, run, turn, profile, claude_model?}        main.rs:189
 → send: perfil sem provedor → 501 "Perfil sem envio"           main.rs:312-319
 → send_value                                                   main.rs:321-349
    provider_of(profile)                                        llm.rs:56-61
    verify_for_send: H1, H2 do pacote; prepare::verify (H3);    store.rs:1064-1135
      critérios; claims::verify; o pacote refeito (H3); o vínculo
    choice: profile, claude_model, skills                       main.rs:490-499
    tarefa = task_for_question(política de claims)              main.rs:340, models.rs:60-66
    idioma = o do projeto                                       main.rs:341, store.rs:1682-1688
    pasta = sends/NN-<provedor>                                 main.rs:342, store.rs:1406-1411
  → llm::send_question → send                                   llm.rs:46-48, :66-103
     skill que a tarefa não aceita: recusa                      llm.rs:69-71
     Library::render(tarefa, perfil, idioma)                    llm.rs:80, models.rs:212-237
     transport::run(provedor)                                   llm.rs:82, transport.rs:63-72
     send.json                                                  llm.rs:101
  → em claims: claims::check e send.json de novo, com check     main.rs:344-347
```

**Provedores, perfis, tarefas** (verificado):
- **provedores:** só dois, `ollama` e `claude-cli`; outro valor é recusado ao ler o catálogo (`models.rs:157-161`) e no transporte (`transport.rs:70`);
- **perfis** (`catalog.json:4-45`): `qwen3coder` (Ollama, `qwen3-coder:30b`, `temperature 0`, `seed 42`, `num_ctx 32768`, mais o `num_predict` da tarefa) e `claude` (CLI do Claude, modelos `sonnet`, `opus`, `haiku`, padrão `sonnet`);
- **tarefas** (`catalog.json:46-104`): `analista` (prosa, `num_predict` 4096), `verificavel` (claims, 8192), `performance` (claims com edits, 16384) e `lexico` (12288). A tarefa de uma pergunta vem da política gravada, não da escolha da pessoa (`models.rs:58-66`);
- **ações** (`catalog.json:105-125`): `locate`, `improve`, `chain`, `lexicon` e `facts` usam `qwen3coder`; `ladder` usa `claude`/`haiku`;
- **`modelfiles/`:** não guarda mais Modelfile; cada um é montado na hora (`models.rs:1-13`, diz ser; `Library::render`, `:212-237`): cabeçalho, nota, `FROM`, `PARAMETER` e `SYSTEM """…"""`. O studio lê a pasta de `<raiz>/modelfiles` (`main.rs:66`): na cópia, foi preciso copiar a pasta para a raiz do scratchpad (seção "Execução");
- **`skills/`:** lida (`store.rs:1305-1334`), mas nenhuma vai: as quatro tarefas têm `"skills": []` (`catalog.json:58`, `:71`, `:85`, `:102`); uma skill pedida para uma tarefa que não a aceita é recusada antes de criar a pasta (`llm.rs:69-71`); e a página não manda `skills` (`index.html:5153-5155`).

**O que cada provedor recebe** (verificado, por leitura e pelos 16 envios gravados):

| | Ollama (`transport.rs:131-190`, `client.rs:126-245`) | CLI do Claude (`transport.rs:232-335`) |
|---|---|---|
| Antes | `ollama create librarian-<tarefa>-<perfil>` com a cópia `Modelfile.requested` (`:139-156`) | nada criado |
| O pedido | `POST /api/generate` com `{model, prompt, stream: false, format}` (`ollama/mod.rs:30-38`): `prompt` é o texto do query; `format` é o `response_schema` do próprio query, quando há (em prosa não há) | o query pela entrada padrão; `-p --output-format json --model <m> --tools "" --strict-mcp-config --no-session-persistence --system-prompt <SYSTEM>` (`:263-266`, `:281`) |
| O SYSTEM | dentro do modelo criado | no argumento `--system-prompt`, com as skills depois dele (`:253-261`) |
| Recusa pelo vínculo | `input_rejected` se o hash do query mudou desde a conferência, antes de abrir a conexão (`client.rs:146-150`) | o mesmo, antes de chamar a CLI (`transport.rs:242-251`) |
| Limites | 300 s, 1 MiB de resposta (`transport.rs:23-24`); só `done_reason: "stop"` é `completed` (`client.rs:115-121`) | 300 s (`:297-301`) |

**Nos 16 envios gravados** (verificado, script `b-manual.js`): o `prompt` do `attempt/request.json` é **igual, byte a byte**, ao texto que `/api/turns` entrega da pergunta; o `attempt/query.json` é igual ao `claims/query.json`; o `format` é igual ao `response_schema` do query; o corpo tem só `format`, `model`, `prompt` e `stream`; e o `query_sha256` do vínculo é o hash do arquivo. Ver MAP-O79.

#### O envio a "Nenhum"

**O que é** (verificado): "Nenhum" é o perfil `manual`, oferecido pela lista de provedores (`llm.rs:27-35`) e que não existe no catálogo.
- Na página, escolher "Nenhum" **não chama o servidor**: `sendTurn` só marca `turn.showQuery` e `turn.manual` e redesenha (`index.html:5138-5143`). O que aparece é o `turn.query` dentro de um `<pre>` (`index.html:5452-5457`), e "Copiar query" copia esse mesmo texto (`:5398-5402`).
- `turn.query` vem de `GET /api/turns` → `Store::turn`, que lê o arquivo com `read_to_string` (`store.rs:439`, `:448`): `claims/query.json` em claims, `prepared/bundle/query.json` em prosa.
- Mandado direto à rota, `POST /api/send` com `profile: "manual"` devolve **501** "Perfil sem envio: manual" e não grava nada (`main.rs:312-319`; verificado na cópia).
- Em claims, a resposta colada vai por `POST /api/paste` (`index.html:5186-5195`), que faz o `verify_for_send` e grava como um envio, `sends/NN-manual/` (`store.rs:1026-1058`).

**Ele mostra exatamente o que seria enviado?** (verificado na cópia)
- **O query, sim, byte a byte.** Nas 14 perguntas gravadas, o texto de `/api/turns` é igual aos bytes do arquivo, e, nas que foram enviadas, igual ao `prompt` que o Ollama recebeu (tabela acima). Também nas 8 perguntas novas.
- **Mas não é tudo o que vai.** Não aparecem: o SYSTEM da tarefa; os `PARAMETER` (no Ollama); o apelido do modelo e os argumentos da CLI (no Claude). O texto da página diz isso ("No Ollama vai como prompt e o SYSTEM vem da tarefa…", `index.html:1350`), e o SYSTEM se vê em outra vista, a da biblioteca (`/api/modelfiles`, `main.rs:274`).
- **E o que aparece não foi reconferido.** `Store::turn` não chama `verify_for_send`. Na cópia, troquei "Como" por "COMO" no `claims/query.json` da pergunta 22: `/api/turns` mostrou o texto trocado; `/api/paste` recusou com "H1: query difere do vinculado no pacote; nada foi enviado", sem criar `sends/`. O mesmo texto da página diz "Conferida contra a edition imediatamente antes de cada envio" (`index.html:1350`, diz ser): vale para o envio e para a colagem, não para o que se mostra ou se copia. Ver MAP-O78.

#### O que fica gravado de cada envio

Pasta `sends/NN-<provedor>/` (verificado, pelos envios do Healthchecks e por leitura):

| | Ollama | CLI do Claude | Colagem (`manual`) |
|---|---|---|---|
| **Pedido** | `Modelfile.requested` (a cópia exata usada no `create`); `attempt/query.json`; `attempt/request.json` (o corpo HTTP exato); `prepared.json` (o registro antes da conexão) | `query.json`; `Modelfile.requested`; `request.json` (executável, argumentos, com o SYSTEM inteiro, `query_sha256`, `timeout_ms`) | `query.json` (cópia do conferido, `store.rs:1047`) |
| **Resposta** | `attempt/response.bin` (o corpo bruto); `attempt/response.txt` (o campo `response`); `ollama-create.log` | `response.bin` (a saída JSON da CLI); `response.txt` (o campo `result`); `stderr.txt` | `response.txt` (o texto colado) |
| **Modelo** | `model` (`librarian-<tarefa>-<perfil>`), `modelfile`, `modelfile_sha256` (`transport.rs:178-189`); o `FROM` só como texto no `Modelfile.requested` | `model` (o apelido), `models_used` (as chaves de `modelUsage` da saída), `cost_usd` (`:319-334`) | `provider: "manual"`, `pasted: true` (`store.rs:1049-1050`) |
| **Versão** | **não há campo.** O digest das camadas só aparece no `ollama-create.log`, como texto do console ("using existing layer sha256:…"), e não é lido por ninguém | o que vier em `models_used` | — |
| **Hora** | `attempt/started.json` (`started_unix_ms`); `result.json` (`http_started_unix_ms`, `http_finished_unix_ms`, `http_elapsed_ms`); no `response.bin`, o `created_at` do Ollama | só o que o `send.json` tem | só o que o `send.json` tem (sem `created` nem `elapsed_ms`: `store.rs:1049-1056` não os põe) |
| **Estado** | `state` (`completed`, `incomplete_response`, `invalid_response`, `provider_failed`, `transport_failed`, `timed_out`, `input_rejected`; `client.rs:87-121`, `:160-162`); `input` (suspeita de entrada cortada, `transport.rs:198-221`) | `completed`, `timed_out`, `invalid_response`, `provider_failed`, `input_rejected` (`:313-318`, `:243-251`) | `completed` |

O **`send.json`** (o resumo que a página mostra) junta ao que o transporte devolveu: `provider`, `profile`, `task`, `language`, `created` (`stamp()`), `elapsed_ms`, `directory`, `semantic_evaluation: "pending"`, `criteria_sha256`, `link` (o vínculo inteiro), `skills` (nome e hash de cada uma) e `purpose` (`llm.rs:85-96`). Em claims, ganha `check`, e é **gravado de novo** com ele (`main.rs:344-347`). Ao lado, `check.json`, os `patch-<claim>.diff` derivados das edições (`claims.rs:252-282`) e, depois da revisão, `reviews.jsonl` (Parada 2).

Caso real, pergunta 01, envio `01-ollama`: `state: completed`; `elapsed_ms` 76.798; `input.prompt_tokens` 7.085 para ~7.707 estimados, `num_ctx` 32.768, sem suspeita de corte; `modelfile_sha256` `64bf8c2d…`, o mesmo que o teste fixa (`transport.rs:502`); `link.checks` = `prepare::verify`, `claims::verify`, `evidence_h1`, `evidence_h2`, `evidence_h3`. O `prompt` tem 26.816 caracteres para 26.976 bytes do arquivo (acentos em UTF-8).

### C. O `librarian-llm-engine`, em duas partes

**O crate** (verificado): `lib.rs:5-9` declara `claims`, `cli`, `models`, `ollama` e `transport`. Dentro do crate, **as duas partes não se chamam**, com duas exceções: o binário `claims` usa `ollama::client::hash` para o hash do relatório (`bin/claims.rs:2`, `:19-20`), e `tests/claims.rs` usa `ollama::request`, `request_with`, `client::now_ms` e `client::hash` (`:6`, `:47`, `:74`, `:382-388`). `claims/compare.rs:120` só usa o próprio `claims::Response`. `models.rs` só usa `ollama::client::hash` nos testes (`:265`).

**Quem depende do crate fora dele:** só o `librarian-studio` (`git grep librarian_llm_engine` fora do crate; `Cargo.toml` do studio, linha 17). O `librarian-evidence` só cita o nome numa lista de proibidos do teste de independência (`librarian-evidence/tests/independence.rs:283`). *O que o método não pega:* uso por JSON, sem o nome do crate. É o caso do `check.json` e do `claims/query.json` (abaixo).

#### C1. O transporte (mandar e receber)

Arquivos: `transport.rs` (508 linhas), `ollama/mod.rs` (40), `ollama/client.rs` (245), `models.rs` (375, a biblioteca de perfis e tarefas), `cli/prepare_ollama.rs` (70), `cli/send_ollama.rs` (396) e os binários `prepare_ollama` e `send_ollama` ("CLI histórica: não reconfere evidências", `cli/send_ollama.rs:1`, diz ser).

| Quem usa, fora do crate | Onde | O quê |
|---|---|---|
| studio, `llm.rs` | `:9-13`, `:39`, `:80-82`, `:95` | `transport::{run, status, skills_record, Request, Skill}`, `models::{Library, Language}` |
| studio, `main.rs` | `:20`, `:340` | `models::task_for_question` |
| studio, `store.rs` | `:1300-1301`, `:1682-1688` | `Library::load` (para `/api/status`, `/api/modelfiles`, o envio, o léxico e a escada); `models::Language` (o idioma do projeto) |
| o léxico | `store.rs:1190` → `llm::send_lexicon` (`llm.rs:51-53`) | o mesmo transporte (parte 5, B) |

**O que quebra se o transporte sair** (verificado por leitura do que chama o quê; a consequência é leitura minha):
- `/api/send`, a escada (`main.rs:423`) e o envio do léxico (`store.rs:1190`): não há mais quem mande;
- `/api/status` e `/api/modelfiles` (`main.rs:114-120`, `:274`), que leem a biblioteca;
- a escolha da tarefa (`main.rs:340`) e o idioma do projeto (`store.rs:1682-1688`), que usam tipos de `models.rs`. O idioma também decide o pedido do léxico (parte 5, B), que não é transporte;
- **não quebra:** o prepare, o bundle, o query de claims, a conferência, a colagem (`/api/paste`, `store.rs:1026-1058`) e o vínculo (`verify_for_send`), que não usam nada do transporte. A colagem é, hoje, o caminho em que uma resposta entra sem transporte.

#### C2. O contrato de claims e a conferência mecânica

Arquivos: `claims.rs` (llm, 440 linhas) e `claims/{location, text, edits, patch, resolve, compare, english}.rs` (1.135 linhas no total); o binário `claims` (`prepare` e `check` sem rede, `bin/claims.rs:1-8`).

| Quem usa, fora do crate | Onde | O quê |
|---|---|---|
| studio, `claims.rs` | `:6`, e o arquivo inteiro | `Context`, `Evidence`, `Policy`, `ReviewedFact`, `Symbol`; `claims::query` (`:211`), `extract_fenced` e `resolve` (`:244-246`), `check` (`:254`), `Response` (`:255`), `edits_diff` (`:260`) |
| studio, `store.rs` | `:14`, `:243-258`, `:822-828`, `:928`, `:1148`, `:1339-1356` | `Policy` (a política de cada pergunta nova); `Context` dentro de `Verified`; `check`, `Response` e `Kind` no `continue_turn`; `compare` e `Claim` na comparação dos envios |
| studio, `facts.rs` | `:11`, `:219`, `:238` | `Claim`, `Context`, `Kind`, `Resolution`, `check` (Parada 2) |
| studio, `main.rs` | `:345` | `claims::check` do studio, depois de cada envio |
| por JSON, sem o crate | `execution.rs:423-431`, `ladder.rs:248-252`, `facts.rs:138-139`, `:232` (o `check.json` e os `diffs`); `evidence.rs:2133-2140` (o `claims/query.json`) | os campos que o contrato grava |

**O que quebra se o contrato sair** (mesmo método): perguntas em claims não têm mais query (`store.rs:927-931`); o `verify_for_send` de uma pergunta em claims (`store.rs:1091-1093`); toda conferência de resposta, enviada ou colada (`main.rs:345`, `store.rs:1051`); o passo seguinte da cadeia, que só segue respostas que passaram (`store.rs:822-828`); a comparação dos envios (`store.rs:1339-1356`); os fatos (`facts.rs`); e, pelo `check.json`, a régua e a escada (`execution.rs:423-431`, `ladder.rs:248-252`). Só a prosa continua de pé.

*Leitura minha sobre a decisão 11:* as duas partes já são separáveis no crate (as únicas costuras são o `hash` do binário e os testes). Se o transporte sair, o studio perde o envio, mas mantém o caminho "o query sai, a resposta volta colada e é conferida". É esse o caminho que um agente de fora usaria. O que a conferência lê está todo no disco da pergunta (`context.json`) e no texto da resposta. Não li nada que prenda a conferência a um provedor.

### D. Os formatos e os modos

#### Claims × prosa

**Onde se escolhe** (verificado):
- no projeto, `Settings.answer_format` (`store.rs:202-204`), editado na página pelo campo `#answerFormat` (`index.html:513`, `:3245`);
- e é **fixado em cada pergunta**, no `question.json` (`store.rs:923`, `:935`). Uma pergunta gravada não muda de formato quando o projeto muda.

**O padrão** (verificado): projeto novo nasce em claims (`store.rs:1489-1490`, "New projects ask for checkable claims", diz ser). Um `project.json` sem o campo é lido como prosa (`#[serde(default)]` e `#[default] Prose`, `store.rs:202-204`, `:228-231`), e a página mostra prosa quando o campo falta (`index.html:3245`). Ver MAP-O85. O Healthchecks está em claims.

**O que muda** (verificado, por leitura e pelas perguntas 15 e 16 da cópia):

| | Prosa | Claims |
|---|---|---|
| Arquivos da pergunta | `prepared/` | `prepared/` e `claims/` (`store.rs:924-933`) |
| O que vai | `prepared/bundle/query.json` (`store.rs:1097`) | `claims/query.json`, refeito do bundle (`claims.rs:199-214`) |
| A política | não é usada (`store.rs:924-933` só a lê em claims) | `policy`, `response_schema` e instruções (abaixo) |
| A tarefa (o SYSTEM) | `analista` (`models.rs:62`) | `verificavel`, ou `performance` com edits (`models.rs:63-64`) |
| `format` no Ollama | nenhum: o query não tem `response_schema` (`ollama/mod.rs:31`) | o `response_schema` |
| Conferência da resposta | **nenhuma**: `check` só com contexto (`main.rs:344-347`) | `claims::check` em todo envio (`main.rs:345`) |
| Colagem | recusada: "Só perguntas em claims recebem resposta colada" (`store.rs:1038-1040`) | aceita e conferida (`store.rs:1026-1058`) |
| Vínculo (`link.checks`) | `prepare::verify` e as do pacote (`store.rs:1111-1114`) | e `claims::verify` |

A pergunta 16 (prosa) e a 15 (claims), feitas sobre as mesmas declarações, têm o **mesmo** `prepared/bundle/query.json`, byte a byte; a 16 não tem `claims/`.

**O que o bundle tem e o query de claims não leva** (verificado, `grep -c` na pergunta 15): as 4 instruções padrão, `unknowns_scope`, os dois `scope`, `metadata_scope`, `execution_scope`, `numbering`, os `statement` e os `authorship` aparecem no bundle e **zero vezes** no query de claims. Do bundle, o query de claims leva as janelas (as linhas e os números), os `evidence_unknowns` (como `evidence.limits`) e as fichas `F` (`claims.rs:15-103`). Ele acrescenta os `symbols` de cada janela, tirados da edition (`claims.rs:59`, `:105-124`). Ver MAP-O77.

#### Os modos

**Os nomes no código** (verificado): não há um campo "modo". Há três configurações do projeto (`store.rs:212-219`):
- `allow_inference` e `allow_hypothesis`, que viram a `Policy` do contrato (`store.rs:928`; `claims.rs` (llm) `:15-20`);
- `allow_patch`, que vira `allow_edits` no contexto (`claims.rs:217-219`; MAP-O83).

A página dá nome a duas combinações (`index.html:3596-3607`): **Localizar** (sem hipótese, sem patch) e **Melhorar** (hipótese e patch); qualquer outra é "Política personalizada". A inferência é uma caixa à parte (`#allowInference`, `index.html:3248`), fora dos dois modos. Os três da memória do usuário batem com as três configurações: inferência (`allow_inference`), hipótese (`allow_hypothesis`) e hipótese como melhoria de código (`allow_hypothesis` + `allow_patch`, o Melhorar).

**O que cada um muda no texto enviado** (verificado na cópia: perguntas 17 a 19 contra a 15, mesmas declarações, só a configuração mudando; script `d-modes.js`):

| Modo | No `claims/query.json` | Tarefa (SYSTEM) | Na conferência |
|---|---|---|---|
| Nenhum (a 15) | `policy` com os dois `false`; `kind` do schema: `FACT`, `LACUNA` | `verificavel` | INFERENCE e HYPOTHESIS recusadas (`inference_not_allowed`, `hypothesis_not_allowed`, `claims.rs` (llm) `:368-378`) |
| Inferência (a 17) | só `policy.allow_inference: true` e `INFERENCE` no `kind` do schema. **Nenhuma instrução muda** | `verificavel` | INFERENCE aceita com premissas (`missing_premises`, `:371-373`) |
| Hipótese (a 19) | só `policy.allow_hypothesis: true` e `HYPOTHESIS` no schema. **Nenhuma instrução muda** | `verificavel` | HYPOTHESIS aceita com plano (`missing_verification_plan`, `:379-385`) |
| Hipótese como melhoria (Melhorar, a 18) | o que a hipótese muda, mais `allow_edits: true`, o campo `edits` no schema e a instrução de edits (`claims/edits.rs:9`: 15 instruções → 16) | **`performance`** (`models.rs:63`; outro SYSTEM, `modelfiles/system/performance.pt.txt`) | as edições conferidas, e o diff derivado delas (`claims.rs:256-279`) |

A instrução fixa diz que INFERENCE e HYPOTHESIS, "quando permitidas, ainda exigem pedido explícito na pergunta" (`claims.rs` (llm) `:221`). A conferência não olha a pergunta: só a política (`:366-388`). No Melhorar, a instrução de edits diz que ela "vale como o pedido explícito de hipótese" (`claims/edits.rs:9`). Ver MAP-O82.

Duas variantes de controle, também na cópia: com instruções do projeto (pergunta 20), o bundle troca as 4 instruções padrão pelas do projeto, e o query de claims as põe à parte, em `project_instructions`, mais a instrução que diz que elas não substituem o contrato. Com `language: "en"` (pergunta 21), as 15 instruções do query de claims vão em inglês. A pergunta e os limites continuam como escritos, e o bundle não muda.

**Em prosa**, os modos não existem: a política não é lida (`store.rs:924-933`), e o SYSTEM `analista` diz que inferência e hipótese só "quando a pergunta explicitamente pedir" (`modelfiles/system/analista.pt.txt:16-20`). Nada confere isso.

**Onde o Medir, a Escada e a régua cruzam o envio** (só registro; a Parada 2 detalha):
- **Medir:** o pacote registra a medida mais recente do run no momento da pergunta (`store.rs:990-997`);
- **Escada:** sobe só sobre uma pergunta reunida já preparada (`main.rs:396-405`) e faz **um** envio, por `send_value` (`main.rs:419-429`);
- **régua:** as execuções aplicam o diff derivado de um claim, lido do `check.json` do envio (`execution.rs:423-431`).

---

## Execução (Parada 1)

Tudo no Git Bash. `S` é o scratchpad da sessão.

1. **Parada 0:** `06-extrator.md` copiado para `S/06-extrator.orig.md`; a linha 3 trocada por "**Estado:** Validado pelo usuário em 26/11/2026 (Paradas 1 e 2)."; `diff` contra a cópia: só a linha 3. Commit `44a694e` ("docs: mapeamento, relatorio 06 validado"), push.
2. **Original antes**, pelo comando de `docs/reports/piloto-1b/dados/p1/original-before.sha256-agregado.txt` (`cd .studio && find projects/<key> -type f -print0 | xargs -0 sha256sum | sed 's/ \*/  /' | LC_ALL=C sort -k2 | sha256sum`): healthchecks `61cd25c1…5ca3d5` (932 arquivos) e renderer `612cdbf6…f3ecc` (42), os das partes 3 e 6.
3. **Cópia:** `cp -r .studio/projects $S/env/root/.studio/`. Depois, `cp -r modelfiles $S/env/root/modelfiles`, porque o studio lê a biblioteca de `<raiz>/modelfiles` (`main.rs:66`): sem ela, `/api/status` devolveu o erro de `catalog.json` ausente **antes** de chegar ao Ollama (`main.rs:117`: `store.library()?` falha primeiro).
4. **Binário:** `cargo build --release --locked -p librarian-studio` (o studio recompilou); `git status --porcelain` vazio antes e depois.
5. **Studio sobre a cópia:** `LIBRARIAN_ROOT=$S/env/root target/release/librarian_studio.exe --port 8794`. O log confirmou "Dados: …\scratchpad\env\root\.studio".
6. **Pedidos** (nenhum chega a um modelo):
   - `GET /api/turns` do Healthchecks (14 perguntas gravadas), guardado em `turns-hc.json`;
   - `POST /api/send` com `profile: "manual"` na pergunta 01: **501**, "Perfil sem envio: manual";
   - `POST /api/gathered-question` com a pergunta e as 5 declarações da pergunta 01 (pergunta 15);
   - 7 vezes `PUT /api/project` (só na cópia), cada uma seguida de `POST /api/gathered-question`: prosa (16); inferência (17); hipótese e patch (18); só hipótese (19); instruções do projeto (20); inglês (21); de volta ao início (22). Script `scripts/variant.sh`;
   - na pergunta 22, "Como" trocado por "COMO" no `claims/query.json` (`sed -i`, na cópia); `GET /api/turns` mostrou o texto trocado; `POST /api/paste` com uma resposta mínima foi recusado ("H1: query difere do vinculado no pacote; nada foi enviado"), sem criar `sends/`.
7. **Scripts** (só leem): `scripts/b-manual.js` (compara o texto de `/api/turns` com o arquivo e com o `prompt`, o `format` e o vínculo de cada envio gravado) e `scripts/d-modes.js` (as diferenças das perguntas 16 a 22 contra a 15). Os dois ficam em `S/scripts`.
8. **Fim:** o studio da cópia foi encerrado. O hash do original foi refeito: os mesmos dois valores e contagens do passo 2; as perguntas do Healthchecks no original continuam sendo 01 a 14. A cópia foi apagada (`rm -rf $S/env`).

---

## Observações novas (Parada 1)

- **MAP-O77.** Em claims (o padrão de projeto novo e o do Healthchecks), o `query.json` do bundle **não vai** ao modelo. Vai o `claims/query.json`, refeito dele (`store.rs:1091-1099`; `claims.rs:199-214`). Dos textos do graph-engine e do prepare, só chegam as linhas das janelas e os limites (`evidence_unknowns` → `evidence.limits`). As 4 instruções padrão, os textos de escopo, os `statement` e os `authorship` ficam só no disco (`grep -c` na pergunta 15: presentes no bundle, 0 no query de claims). O bundle continua sendo conferido byte a byte antes de cada envio (`prepare::verify`, `store.rs:1083`).
- **MAP-O78.** O "Nenhum" mostra e copia o query **sem reconferir**: a página não chama o servidor (`index.html:5138-5143`), e `Store::turn` só lê o arquivo (`store.rs:439`, `:448`). Verificado na cópia: um query alterado aparece na página; a colagem dele é recusada no H1. Ele também não mostra o SYSTEM, os `PARAMETER` nem os argumentos da CLI. Diz ser × faz: o texto ao lado do query diz "Conferida contra a edition imediatamente antes de cada envio" (`index.html:1350`). Isso vale para o envio e para a colagem, não para o texto mostrado.
- **MAP-O79.** Nos 16 envios gravados do Healthchecks, o `prompt` que o Ollama recebeu é o texto do query, byte a byte. O `format` é o `response_schema` do próprio query, e o corpo HTTP tem só `format`, `model`, `prompt` e `stream` (`ollama/mod.rs:30-38`). Fora do query, só o SYSTEM e os `PARAMETER` vão, pelo modelo criado por `ollama create`.
- **MAP-O80.** Nenhum campo grava a versão do modelo. No Ollama, ficam o nome do modelo criado (`librarian-<tarefa>-<perfil>`), o hash do Modelfile e o `FROM` como texto (`qwen3-coder:30b`). O digest das camadas só aparece no `ollama-create.log`, como saída do console, sem leitor. No Claude, ficam o apelido (`sonnet`, `opus`, `haiku`) e as chaves de `modelUsage` (`transport.rs:319-323`). A colagem não grava `created` nem `elapsed_ms` (`store.rs:1049-1056`).
- **MAP-O81.** Escrita sobrescrevível × imutável na pasta da pergunta. Sobrescrevíveis (`fs::write`): `search.json`, `prepare-request.json`, `criteria-before.md`, `claims/*` e `question.json` (`store.rs:904-913`, `:967`; `claims.rs:223-224`), e o `send.json`, gravado duas vezes num envio em claims (`llm.rs:101`, `main.rs:346`). Imutáveis: o bundle (`create_new`, `bundle.rs:25-34`), o `prepared/` (`write_new`, `prepare.rs:613-614`), o `evidence.json` e o `selection.json` (`publish_immutable`, `store.rs:1726-1736`) e o `attempt/` do Ollama (`client.rs:51-59`).
- **MAP-O82.** Os modos "inferência" e "hipótese" não mudam nenhuma instrução do query, só o `policy` e o `kind` do `response_schema` (verificado, perguntas 17 e 19). A instrução fixa pede, além da permissão, "pedido explícito na pergunta" (`claims.rs` (llm) `:221`), e a conferência só olha a permissão (`:366-388`). O Melhorar (hipótese + patch) é o único que muda o texto: acrescenta a instrução e o schema de edits e troca a tarefa para `performance`, com outro SYSTEM. Na página, os modos nomeados (Localizar, Melhorar) não mexem na inferência, que é uma caixa à parte (`index.html:3248`, `:3600`).
- **MAP-O83.** Homônimo: a configuração `allow_patch` do projeto vira `allow_edits` no contexto de claims (`claims.rs:217-219`), e o `allow_patch` do contexto (diff unificado, `claims/patch.rs`) só existe em perguntas antigas: é sempre `false` nas novas (`claims.rs:185-186`, `:219`).
- **MAP-O84.** Em prosa, nada da resposta é conferido (`main.rs:344-347`), a colagem é recusada (`store.rs:1038-1040`) e a política não é lida (`store.rs:924-933`). A tarefa `analista` pede, no SYSTEM, as mesmas categorias (FACT, INFERENCE, HYPOTHESIS, LACUNA; `analista.pt.txt:5-27`), sem quem as confira.
- **MAP-O85.** O formato padrão depende de onde se olha: projeto novo nasce em claims (`store.rs:1490`); um `project.json` sem o campo é lido como prosa (`store.rs:202-204`, `:228-231`); e a página mostra prosa quando o campo falta (`index.html:3245`).
- **MAP-O86.** Línguas misturadas no mesmo campo: os limites de busca e do passo estão em inglês (`mod.rs:453-458`, `search.rs:292`, `:345-351`); os do prepare, em português (`prepare.rs:350-407`). Os dois vão juntos em `evidence.limits`, num contrato que pode estar em português ou em inglês. Com `language: "en"`, os limites e a pergunta não mudam (pergunta 21).
- **MAP-O87.** O `declaration_line` dos alvos gravado no `prepare-request.json` mudou entre 18/11 e hoje para as mesmas declarações: `ping` 196 → 198, `ping_by_slug` 264 → 265. Isso porque `target_of` passou a usar a regra de `claims::declaration_of`, que pula decoradores (`store.rs:1363-1371`). O bundle e o query saíram iguais, porque o `prepare` aceita qualquer linha dentro do trecho (`prepare.rs:550-560`). Já o pacote mudou: o `request_sha256` e o commit do Librarian que o fez (`versions.builder.git_commit`).
- **MAP-O88.** O llm-engine tem um só dependente fora do crate, o studio. O transporte e o contrato de claims não se chamam dentro do crate, a não ser pelo `ollama::client::hash` no binário `claims` (`bin/claims.rs:2`) e pelos testes (`tests/claims.rs:6`, `:47`, `:74`, `:382-388`). Sem o transporte, a colagem continua de pé: o query sai, a resposta volta, é conferida e gravada como um envio (`store.rs:1026-1058`).
- **MAP-O89.** As skills são lidas, mas não vão: as quatro tarefas têm `"skills": []` (`catalog.json:58`, `:71`, `:85`, `:102`), `llm.rs:69-71` recusa a skill que a tarefa não aceita, e a página não manda `skills` em `/api/send` (`index.html:5153-5155`). `CLAUDE_SYSTEM` (`transport.rs:26`) também não é alcançado a partir do studio, que sempre passa um Modelfile (`llm.rs:80-81`).

## Perguntas (Parada 1)

- **MAP-Q21.** Para a decisão 11 (a conferência vira o portão entre o agente e o Librarian), qual forma de query o mapa trata como **o** contrato com o agente? O `claims/query.json` é o que vai e o que a conferência usa. O `query.json` do bundle é o que o Librarian confere e o que leva os textos de escopo (MAP-O77). Hoje os dois têm leitores por JSON em mais de um crate (MAP-Q7, acima).
  - **Resposta do usuário (26/11/2026):** o contrato com o agente é o query de claims; o do bundle é a prova interna. Falta o contrato levar os textos de escopo e os selos.
- **MAP-Q22.** O "Nenhum" mostra o prompt byte a byte, mas sem o SYSTEM e sem reconferir (MAP-O78). Para o mapa, ele conta como "mostra o que seria enviado", ou fica registrado como uma diferença a tratar?
  - **Resposta do usuário (26/11/2026):** o "Nenhum" é diferença a tratar: conferir na hora da entrega e levar tudo, inclusive o SYSTEM.
- **MAP-Q23.** Os modos: o código tem três configurações (inferência, hipótese, patch), a página nomeia dois modos (Localizar, Melhorar) e a inferência fica fora deles (MAP-O82). Que nomes o mapa usa daqui em diante: os três da memória, os do código ou os da página?
  - **Resposta do usuário (26/11/2026):** as permissões (`inference`, `hypothesis`, `edits`) vêm no pedido do agente, e o portão confere. Valem os nomes do código. Localizar e Melhorar saem com o front.

**Também na avaliação da Parada 1 (resposta do usuário, 26/11/2026):** a prosa sai do contrato. O back-end passa a ser todo em inglês.


---

## Parada 2

**Base:** `main` em `03fe49a`, worktree limpa. As mesmas regras da Parada 1. **Nenhum LLM foi chamado.** A execução foi feita sobre uma **nova cópia** da `.studio` (e da biblioteca `modelfiles/`), no scratchpad, e sobre uma **cópia** dos arquivos rastreados do Healthchecks, também no scratchpad (seção "Execução (Parada 2)"). Rotas usadas no studio da cópia: `/api/facts`, `/api/review`, `/api/gathered-question`, `/api/paste` e `/api/generate`. A conferência foi refeita também pelo binário `claims` do llm-engine, que não usa rede (`bin/claims.rs:1`).

Caminhos abreviados novos: `check.rs` e `package.rs` são de `crates/librarian-evidence/src/`; `location.rs`, `text.rs`, `edits.rs`, `patch.rs` e `resolve.rs` são de `crates/librarian-llm-engine/src/claims/`; `core/lib.rs` é `crates/librarian-core/src/lib.rs`.

**Casos reais:** as 16 respostas gravadas do Healthchecks (Parada 1) e o acervo do projeto, `projects/healthchecks-25597e41/facts.jsonl`, com 26 fatos promovidos e nenhum revogado (script `e-checks.js`).

### E. A conferência da resposta

**O caminho** (verificado, por leitura):

```
resposta (texto)                                   send: main.rs:344-347 · colagem: store.rs:1051
 → claims::check do studio                         claims.rs:252-282
    extract_fenced: tira uma única cerca Markdown   claims.rs (llm) :279-285 (só ```, ```json)
    resolve: completa citações pela evidência       resolve.rs:21-76
    claims::check (llm): Context + texto → Report   claims.rs (llm) :329-440
    edits_diff de cada claim que passou             claims.rs:257-261
 → check.json (+ patch-<claim>.diff)                claims.rs:262-280
```

**O que o relatório diz** (`claims.rs` (llm) `:144-151`, `:331-337`): `mechanical_status` (`passed` ou `rejected`), `semantic_status` **sempre** `pending`, `accepted` **sempre** `false` e a lista `issues` (`{claim_id, code}`). "Conferência mecânica… O sentido continua pendente; accepted é sempre false" (`claims.rs:11`, gravado no `check.json`). As recusas são só **códigos**, em inglês. O texto que a pessoa lê fica na página, numa tabela pt/en (`ISSUES`, `index.html:5616-5743`); um código fora da tabela aparece como "problema sem descrição" (`UNKNOWN_ISSUE`, `index.html:5744-5746`; uso em `:5832`).

**Antes de conferir, o Librarian completa a resposta** (`resolve.rs:1-29`, diz ser; verificado pelo código e pela execução abaixo):
- uma citação cuja `quote` não é o texto das linhas citadas, mas é o texto de **uma única** outra faixa da mesma evidência, passa para essa faixa (`citation_range_from_quote`, `resolve.rs:39-48`); a operação da `location` acompanha (`operation_range_from_quote`, `:49-53`). Uma quote sem letra nem número não se move (`:88-91`);
- um FACT com `location` ganha a citação da linha da declaração e a das linhas da operação, quando não estão citadas (`declaration_citation_added`, `operation_citation_added`, `:55-70`);
- cada mudança fica em `resolutions` no `check.json` (`claims.rs:269-271`); o `check.json` grava em `claims` as claims **já completadas** (`claims.rs:253-255`, `:266`), e a resposta crua fica no `response.txt`.

**O que confere e o que recusa** (verificado, por leitura; o texto é o da página, `index.html:5616-5743`):

| Grupo | Código | Quando | Onde |
|---|---|---|---|
| Forma | `invalid_response_schema` | o texto não é o JSON de `Response` (campos a mais são recusados: `deny_unknown_fields`) | `claims.rs` (llm) `:338-347` |
| | `invalid_version_or_empty_claims` | `schema_version` ≠ 1 ou lista vazia | `:349-354` |
| | `invalid_or_duplicate_claim` | `id` ou `text` vazio, ou `id` repetido | `:363-365` |
| Política | `inference_not_allowed`, `hypothesis_not_allowed` | a categoria sem a permissão | `:367-378` |
| | `missing_premises`, `missing_verification_plan` | INFERENCE sem premissas; HYPOTHESIS sem plano | `:371-373`, `:379-385` |
| | `unexpected_premises`, `unexpected_verification_plan` | premissas fora de INFERENCE; plano fora de HYPOTHESIS | `:389-394` |
| Citação | `missing_citation` | claim que não é LACUNA sem citação | `:395-397` |
| | `unknown_evidence`, `wrong_path`, `citation_out_of_range` | evidência inexistente, caminho errado, linhas fora da janela | `:412-423` |
| | `quote_mismatch` | a `quote` não é exatamente o texto das linhas citadas, unidas por LF | `:424-433` |
| Localização (`location.rs`) | `missing_location` | FACT sem `location` quando `require_location` | `:87-93` |
| | `location_requires_fact`, `location_unknown_evidence`, `location_wrong_path`, `location_unknown_symbol` | `location` em outra categoria; evidência, caminho ou símbolo (nome + linha da declaração) que não estão nos `symbols` da janela | `:94-111` |
| | `location_operation_out_of_range` | operação fora do símbolo | `:112-117` |
| | `location_operation_includes_declaration`, `location_operation_is_whole_body` | com `precise_location`: a operação começa na declaração, ou cobre o corpo inteiro | `:123-129` |
| | `location_declaration_not_cited`, `location_operation_not_cited` | as linhas da declaração ou da operação não estão citadas | `:131-141` |
| Texto (`text.rs`, com `strict_text`) | `text_cites_uncited_line` | o `text` fala de uma linha que nenhuma citação cobre | `:15-21` |
| | `text_quotes_uncited_code` | um trecho entre crases que não está nas linhas citadas (numa LACUNA, basta estar na evidência) | `:22-51` |
| | `text_line_lacks_named_code` | a frase nomeia código e uma linha, e o código não está nessa linha | `:52-54` |
| Edições (`edits.rs`) | `edits_not_allowed`, `edits_require_hypothesis`, `edits_malformed`, `edits_unknown_path`, `edits_outside_evidence`, `edits_change_uncited_lines`, `edits_no_change`, `edits_rewrite_not_whole_declaration`, `edits_unbalanced`, `edits_overlap` | as regras da instrução de edits (Parada 1, MAP-Q8) | `edits.rs:36-121` |
| Patch (`patch.rs`, só perguntas antigas) | `patch_not_allowed`, `patch_requires_hypothesis`, `patch_malformed`, `patch_unknown_path`, `patch_outside_evidence`, `patch_does_not_apply`, `patch_changes_uncited_lines` | idem, para diff unificado | `patch.rs:21-69` |

**O que não confere** (verificado, pelo que o código não lê): o sentido do `text` frente às linhas; se a pergunta pediu a inferência ou a hipótese (Parada 1, MAP-O82); a LACUNA, além do `text` (em `strict_text`, `text.rs:37-51`). Em prosa, nada (Parada 1, MAP-O84).

**O caso real: a pergunta 10 do Healthchecks** (envio `01-ollama`, 18/11, `librarian-verificavel-qwen3coder`). A pergunta: "Como um Flip não processado vira notificação? …". A resposta tem 4 FACT. O `check.json` gravado:
- `mechanical_status: "rejected"`, 6 `issues`, todos `quote_mismatch`: C1 duas vezes, C2 duas, C3 uma e C4 uma;
- 5 `resolutions`: duas `citation_range_from_quote` (C1 de 119–121 para 118–121; C4 de 1152 para 1152–1153) e três `operation_citation_added`.

Onde a cópia difere do código (verificado, comparando cada `quote` com o `excerpt` da evidência no `claims/context.json`):

| Claim | Citação | O que a resposta fez |
|---|---|---|
| C1 | 91–91 | citou 1 linha e copiou 12 (a declaração e a docstring) |
| C1 | 105–108 | citou 4 linhas e copiou 6 |
| C2 | 24–24 | citou 1 linha e copiou 12 |
| C2 | 39–55 | 17 linhas, mas a linha vazia 52 veio com 4 espaços (`"    "` em vez de `""`) |
| C3 | 1391–1391 | citou 1 linha e copiou 12 |
| C4 | 1156–1187 | 32 linhas, mas a linha vazia 1163 veio com 8 espaços |

O `resolve` não salva nenhuma dessas: as quotes de 12 linhas não batem com nenhuma faixa única, e as linhas vazias com espaços não existem no arquivo (`resolve.rs:86-97`).

**A conferência refeita hoje** (binário `claims check`, com o `context.json` da pergunta e as claims do `check.json`; verificado): nas perguntas 01, 05, 10 e 12, o resultado é **o mesmo** do gravado, código a código. A mesma resposta **sem** o `resolve` (o `response.txt` cru):

| Pergunta | Gravado (com `resolve`) | Cru (sem `resolve`) |
|---|---|---|
| 01 | `passed`, 0 | `rejected`, 14 (`location_declaration_not_cited` 7, `location_operation_not_cited` 4, `quote_mismatch` 3) |
| 05 | `rejected`, 4 | `rejected`, 9 |
| 10 | `rejected`, 6 | `rejected`, 14 |
| 12 | `rejected`, 1 | `rejected`, 1 |

Nas 16 respostas: 9 passaram e 5 foram recusadas. As 2 restantes (pergunta 14, envios 01 e 02) não têm `check.json`: o Ollama devolveu 500 ("llama-server process has terminated… CUDA error"), `state: "provider_failed"`, sem texto. Códigos nas 5 recusadas: `quote_mismatch` 8, `location_operation_includes_declaration` 4, `location_unknown_symbol` 2, `text_quotes_uncited_code` 2 e `text_line_lacks_named_code` 1. Resoluções nas 16: `citation_range_from_quote` 35, `operation_citation_added` 20, `declaration_citation_added` 16 e `operation_range_from_quote` 7. Ver MAP-O90 e MAP-O91.

### F. Da resposta ao acervo

**Quem aprova** (verificado): `POST /api/review {key, run, turn, send, claim, decision, note}` (`main.rs:201-211`), da vista Pacote, chama `Store::review` (`facts.rs:27-83`). A `decision` é `approved` ou `rejected`; outro valor é recusado ("decision deve ser approved ou rejected", `main.rs:206`; verificado). **Nada no pedido diz quem revisa:** a autoria é a constante `AUTHORSHIP = "revisão humana no Librarian Studio"` (`facts.rs:21`, `:58`, `:150`).

**Os portões** (`facts.rs:39-73`, `:131-224`; verificado por leitura e, na cópia, cada recusa abaixo):

| Ordem | O que exige | Recusa (texto exato) |
|---|---|---|
| 1 | a resposta preservada (`response.txt`) | "A resposta do envio não foi preservada" (`facts.rs:250`; na cópia: pergunta 14, envio 01) |
| 2 | o `check.json`, com o hash da mesma resposta | "Envio sem conferência de claims"; "A resposta preservada difere da que foi conferida" (`:232-235`) |
| 3 | o item existe na resposta | "Item desconhecido: {id}" (`:244`) |
| 4 | o item não foi promovido antes | "{id} já foi promovido como {F}" (`:40-42`; na cópia: C3 da pergunta 01) |
| 5 | uma nota | "Escreva na nota por que o item é verdadeiro…" (`:43-45`; na cópia) |
| 6 | só FACT, ou HYPOTHESIS com mudança | "Só FACT, ou HYPOTHESIS com mudança, vira fato; LACUNA e INFERENCE não são promovidos" (`:196-198`; na cópia: a LACUNA C5 da pergunta 12) |
| Portão 2 | o envio tem vínculo; a pergunta ainda confere (`verify_for_send` refeito); a query e o pacote são os vinculados; a query preservada tem o hash do vínculo | 6 textos "Portão 2: …" (`:201-216`) |
| Portão 1 | a resposta, refeita pelo `resolve`, passa de novo em `claims::check` | "Portão 1: a resposta não passa na conferência mecânica" (`:217-222`; na cópia: C1 da pergunta 10); "Portão 1: FACT sem location" (`:55`) |
| Portão 3 (só mudança) | ao menos uma execução da régua com a mudança, com sucesso | "Portão 3: rode na Régua ao menos um comando com esta mudança…" (`:144-146`) |

A recusa (`rejected`) só grava a decisão e revoga uma promoção anterior, sem nenhum portão (`facts.rs:74-81`). Verificado na cópia: aprovar o C1 da pergunta 07 criou o F27; rejeitá-lo depois gravou `{"event":"revoked","id":"F27",…}`.

**O que é guardado** (verificado):
- em `sends/NN/reviews.jsonl`, cada decisão: `claim_id`, `claim_sha256`, `decision`, `note`, `reviewed` (a hora) e, se promoveu, `fact_id` ou `revoked` (`facts.rs:37-38`, `:50`, `:71`, `:77`);
- em `projects/<key>/facts.jsonl`, por **projeto** e não por run, um evento por linha: `promoted` ou `revoked` (`facts.rs:52`, `:73`, `:78`). Um FACT promovido leva `id` (`F` + o número de promoções + 1, `:47`), `kind`, `statement` (o `text` do claim), `authorship`, `path`, `symbol_name`, `declaration_line`, `line` e `end_line` (a operação), `citations` (com as quotes), `edition` (o `dossier_origin`, isto é, `edition:` + o hash do `manifest.json`), `run`, `turn`, `send`, `claim_id`, `claim_sha256`, `response_sha256`, `link` (o vínculo inteiro), `note`, `reviewed`, `gates` e, se houver, `resolutions` (`facts.rs:56-70`). Uma mudança (`CHANGE`) leva também `diff`, `diff_sha256`, `files`, `verification_plan`, `executions` e `proposed_by` (provedor, modelo, Modelfile, custo; `:148-162`);
- o arquivo só recebe acréscimos (`OpenOptions::append`, `facts.rs:296-301`), e nenhum outro código o escreve (`git grep` de `FACTS` e `facts.jsonl` no studio: as três chamadas de `append` acima). *O que o método não pega:* um processo de fora que reescreva o arquivo. Nada o assina; o pacote de cada pergunta guarda os bytes da linha de cada fato que usou (`promotion_lines`, `facts.rs:165-177`; `package.rs:192-193`).

**Caso real** (verificado, no original, só por leitura): os 26 fatos do Healthchecks vieram das perguntas 01, 02, 06, 08, 09, 11, 13 e 14. **As 26 notas e as 44 revisões começam com "Revisão do agente Claude no piloto Healthchecks (não é revisão humana; o contrato atual não tem campo para isso)"**, e os 26 fatos têm `authorship: "revisão humana no Librarian Studio"`. Ver MAP-O94.

**Para qual id o fato aponta** (verificado): para **nenhum id de símbolo**. Aponta para o caminho, o nome, a linha da declaração, as linhas da operação e o **texto** das citações (`facts.rs:59-62`), mais a edition em que foi aprovado (`:63`). O estado é refeito contra a edition do run pedido (`facts.rs:88-118`): `current` se cada quote ainda é o texto das mesmas linhas do mesmo arquivo (`recheck`, `:253-262` → `prepare::check_quote`, `prepare.rs:571-581`); senão, `stale`, com o motivo "{path}:{a}–{b} mudou desde a revisão". "Nothing is moved or searched for: a line that shifted makes the fact stale" (`facts.rs:253-254`, diz ser).

Por execução, na cópia, com dois runs novos gerados de uma cópia do Healthchecks, mudando só `hc/api/views.py`:

| Run | Mudança em `views.py` | Id do símbolo `ping` | Os 12 fatos de `views.py` |
|---|---|---|---|
| `20261118-061046` (o de 18/11) | — | `…:d0121156…:function:6860:9238` | 12 `current` |
| novo, A | uma linha de comentário **no fim** | `…:b0f05808…:function:6860:9238` (mudou o hash do conteúdo) | **12 `current`** |
| novo, B | uma linha de comentário **no topo** | `…:6bdb95fc…:function:6711:9024` | **12 `stale`** (F1: "hc/api/views.py:265–265 mudou desde a revisão") |

Os 15 fatos dos outros arquivos ficaram `current` nos três. → O id muda com qualquer mudança no arquivo (MAP-O76), mas o fato não depende dele. O que o derruba é o deslocamento das linhas citadas, mesmo com o texto igual. Ver MAP-O93.

**Onde o fato volta** (verificado): só os `current`, e só por `reviewed_about` (`store.rs:789-800`), que os casa com os alvos pela igualdade de `path`, `symbol_name` **e** `declaration_line` (`:796`). Ele é usado pelo passo da cadeia (`store.rs:855`) e pela pergunta reunida com `with_facts` (`store.rs:544`). O fato entra como ficha `F` no bundle (Parada 1, A) e como `evidence.reviewed_facts` no query de claims. Na cópia, uma pergunta reunida com `with_facts` sobre as declarações da pergunta 01 trouxe 8 fatos (F1 a F7 e F26). *Leitura minha:* os fatos de `ping` têm `declaration_line` 198 (a regra de `declaration_of`, que pula decoradores), e o pedido de 18/11 tinha 196 para o mesmo alvo (Parada 1, MAP-O87). Com o código de 18/11, a igualdade não casaria; com o de hoje (`store.rs:1369`), casa. O teste `p1_a_fact_about_a_decorated_python_function_comes_back` (`store.rs:7880`) cobre esse caso.

**Existe caminho para a pessoa gravar um fato sem claim de modelo?** (verificado na cópia)
- **Direto, não:** não há rota que escreva no `facts.jsonl` sem um envio e um item (os únicos escritores estão acima; as rotas de fatos são `GET /api/facts` e `POST /api/review`, `main.rs:201-212`).
- **Indireto, sim, pela colagem:** a pessoa escreve o JSON de claims, cola em `/api/paste` (`store.rs:1026-1058`) e aprova. Na cópia: uma pergunta reunida nova (15), um FACT escrito à mão sobre `ping` (operação na linha 207, sem citações), colado. O `resolve` acrescentou as citações da declaração e da operação, e a conferência passou. Aprovado, virou o **F28**, com os mesmos portões e `gates` de um fato vindo de modelo.
- **O fato não diz que foi colado:** um FACT não tem `proposed_by` (só a mudança tem, `facts.rs:151-152`). Fica só o nome do envio, `01-manual`, e, no `send.json`, `pasted: true` ("who wrote the answer is not known, only that it was pasted", `store.rs:1023-1025`, diz ser). Ver MAP-O95.

#### MAP-Q10: o que "acervo" é no código

Verificado por `git grep -i acervo` nos `.rs` e na página:
- **é o `facts.jsonl`**, a coleção de fatos revisados do projeto: "Each promotion in the acervo" (`facts.rs:165-166`); "Each reviewed fact's line in the acervo" (`package.rs:125`, `:192-193`, com o código de erro `acervo_unreadable`; `check.rs:188`); a origem de campo `Acervo` e a proveniência `AcervoQuotesChecked` do pacote (`evidence.rs:478`, `:536`, `:563`, `:1607`). Na página: "Acervo de fatos" (`index.html:896`) e a área "Acervo" (`:1605`);
- **a edition nunca é chamada de acervo no código:** nenhum `.rs` usa a palavra para ela;
- **o outro uso** é o nome de pasta `ai/acervo/` nos arquivos de configuração de projeto (o `sources_config` do ray tracer, parte 6; um teste em `store.rs:2630`).

→ **A resposta, pelo código:** acervo = `projects/<key>/facts.jsonl`.

### G. Onde a Escada e a régua cruzam o envio (só registro)

- **A Escada** (`POST /api/ladder`, `main.rs:359-473`): sobe só sobre uma pergunta **reunida** já preparada (`:396-404`), passa pelo `verify_for_send` antes de qualquer execução (`:405`), roda a linha de base dos comandos do portão que faltam (`:409-416`) e faz **um** envio por `send_value` (`:419-429`), com o perfil padrão `claude`/`haiku` (`catalog.json:121-124`). Lê do resumo do envio as HYPOTHESIS com diff derivado (`ladder::outcome`, `ladder.rs:248-252`). Cada mudança passa pelos comandos da régua (`main.rs:437-449`). Uma LACUNA é gravada com os nomes que ela cita e o que o catálogo acharia para eles, sem novo envio (`:458-468`). As recusas terminam com "nada foi criado, executado nem enviado" (`LADDER_NOTHING`, `main.rs:352`; recusas em `:366`, `:370`, `:376`, `:390`, `:394`, `:400`, `:403`, todas em português).
- **A régua** (as execuções de `execution.rs`): aplica numa cópia o diff que a conferência derivou, lido do `check.json` do envio ("Envio sem conferência de claims", "{claim} não tem mudança derivada", `execution.rs:423-431`).
- **O fato de mudança** exige uma execução da régua com a mudança, com sucesso (Portão 3, `facts.rs:141-146`).
- **O Medir:** o pacote de cada pergunta registra a medida mais recente do run no momento da pergunta (`store.rs:990-997`).

### H. A fronteira e os testes

**A fronteira da conferência** (verificado, por leitura):
- `claims::check` do llm-engine é uma função pura: recebe o `Context` e o texto da resposta e devolve o `Report` (`claims.rs` (llm) `:329-440`). `resolve`, `extract_fenced` e `edits_diff` também não tocam o disco. Quem lê e grava é o studio: o `context.json` (`claims.rs:232`), a resposta e o `check.json` com os diffs (`claims.rs:252-282`);
- o binário `claims` (`bin/claims.rs:1-44`) faz o mesmo `check` fora do studio, sem rede. Grava o relatório num arquivo novo (`create_new`) e sai com 0 (passou), 2 (recusou) ou 1 (erro). Verificado nesta parada: ele reproduziu as 4 conferências gravadas. Ele **não** aplica o `resolve`, que só o studio chama (`claims.rs:243-247`);
- **entra** na conferência: o `context.json` e a resposta. **Sai:** o `check.json`, os `patch-<claim>.diff`, o `send.json` com `check`, depois o `reviews.jsonl` e o `facts.jsonl`;
- **quem lê o que a conferência grava, fora do llm-engine:** a revisão (`facts.rs:138-139`, `:232`), a régua (`execution.rs:423-431`), a escada (`ladder.rs:248-252`), a cadeia (`store.rs:822-828`, que refaz a conferência), a comparação de envios (`store.rs:1339-1356`) e a página.

**Os testes** (verificado: `cargo test --locked -p librarian-llm-engine` e `-p librarian-studio`, todos passaram):

| Suíte | Resultado | O que cobre do envio e da conferência |
|---|---|---|
| `librarian-llm-engine`, unitários | 32 passaram, 1 ignorado | transporte: SYSTEM e skills (`transport.rs:416-439`), suspeita de corte (`:442-457`), o vínculo (`:460-494`, com a porta 9 fechada: nenhum modelo), o Modelfile byte a byte (`:499-507`); a biblioteca (`models.rs:270-…`). O ignorado só grava uma simulação com `OLLAMA_SIMULATION_DIR` (`cli/send_ollama.rs:162`) |
| `librarian-llm-engine`, `tests/claims.rs` | 28 passaram | o contrato e a conferência: `location` (`:106-170`), `resolve` (`:173-222`), identidade e quote (`:291-312`), permissões (`:314-331`), LACUNA (`:333`), forma (`:342`), `strict_text` (`:466-555`), `precise_location` (`:557`), patch (`:619`), edits (`:693-789`), `compact_items` (`:791`), a cerca (`:445`) |
| `librarian-studio` | 182 + 1 passaram | 40 com nomes de envio, colagem, vínculo, claims, fatos, revisão ou escada (`cargo test -- --list`), entre eles: `send_paste_review_and_continue_check_the_question_first_and_send_nothing_when_it_fails` (`store.rs:7067`), `a_send_is_linked_to_the_package_checked_and_says_what_it_was_for` (`:5849`), `claims_questions_derive_a_checked_context_and_pasted_answers_are_checked` (`:2116`), `approved_facts_pass_every_gate_and_are_rechecked_against_later_editions` (`:2220`), `chain_steps_carry_checked_locations_and_current_reviewed_facts_never_the_text` (`:2317`), `only_a_promotion_is_a_facts_line_and_it_is_read_byte_for_byte` (`:5411`), `a_claims_question_without_its_claims_files_is_never_sent` (`:5948`), `an_answer_of_the_ladder_starts_no_other_call` (`:6837`) e `p1_a_fact_about_a_decorated_python_function_comes_back` (`:7880`) |

**Nenhum teste chama um modelo** (verificado, por leitura): os testes do studio usam o provedor `test` (`llm.rs:57-59`, `:77-78`) ou colam a resposta (`main.rs:334-337`), sobre um `Store` em pasta temporária (`store.rs:1790`, `:1805`); o teste de transporte do llm-engine usa a porta 9, fechada.

**Códigos de recusa sem teste que os nomeie** (`grep` de cada código em `tests/claims.rs`, nos módulos de teste do llm-engine e nos testes do `store.rs`): `unexpected_premises` e `unexpected_verification_plan`. *O que o método não pega:* um teste que produz o código sem escrevê-lo (por exemplo, só conferindo `mechanical_status`). Dois códigos testados não têm texto na página: `text_line_lacks_named_code` e `edits_unbalanced` (MAP-O91).

### As mensagens de recusa e de aviso do prepare e do portão

"Portão" aqui é tudo o que decide se uma pergunta vai, se uma resposta passa e se vira fato: o `verify_for_send`, o pacote (H1 a H3), o contrato de claims, o transporte e os portões dos fatos. Língua por leitura do texto. Textos com `{…}` são montados com `format!`.

**Prepare** (`prepare.rs`; e o que ele chama no graph-engine e no core):

| Onde | Língua | Texto |
|---|---|---|
| `prepare.rs:120` | en | Edition has diagnostics; set accept_diagnostics to use it |
| `prepare.rs:126` | en | A chain step needs distinct targets |
| `prepare.rs:129` | en | A step is picked on a path or gathered, not both |
| `prepare.rs:146` | en | callees needs owner_context |
| `prepare.rs:162` | en | hits must be between 1 and top |
| `prepare.rs:165` | en | Reviewed facts and owner context are carried only by a chain step |
| `prepare.rs:173-176` | en | Search returned {} candidates; {} requested |
| `prepare.rs:244` | en | File heads are carried only by a gathered step |
| `prepare.rs:248`, `:250`, `:251` | en | {path}: head listed twice · no lines before its first declaration · not in the edition |
| `prepare.rs:294` | en | {id}: reviewed fact outside the chain targets or malformed |
| `prepare.rs:304` | en | Reviewed fact source missing |
| `prepare.rs:553`, `:563-566` | en | {path}: not in the edition · {path}: no \`{nome}\` declared at line {n} |
| `prepare.rs:574`, `:578` | **pt** | {path} não está nesta edition · {path}:{a}–{b} mudou desde a revisão |
| `prepare.rs:596`, `:599`, `:603` | en | Dossier source without path · {path}: not in the edition · {path}: dossier source differs from the edition snapshot |
| `prepare.rs:626`, `:635`, `:643` | en | Missing query · Prepared file differs: {path} · origin.json does not match the prepared bundle |
| `graph_engine.rs:107` | en | source.id must not be empty |
| `graph_engine.rs:110`, `:126` | pt | Fonte duplicada: "{}". · Fato duplicado: "{}". |
| `graph_engine.rs:144`, `:151`, `:160` | pt | {}: fonte "{}" não encontrada. · {}: linha {} inválida na fonte "{}"… · {}: intervalo {}–{} inválido na fonte "{}"… |
| `graph_engine.rs:174`, `:181`, `:184`, `:214` (e `:611`, `:618`, `:621`) | pt | Informe ao menos uma ficha. · Ficha repetida na seleção: "{}". · Ficha "{}" não encontrada. · {}: fonte "{}" não encontrada. |
| `graph_engine.rs:262`, `:267` | pt | A pergunta não pode estar vazia. · Instruções informadas não podem ser vazias. |
| `graph_engine.rs:410` | pt | Bundle exige uma pergunta |
| `graph_engine.rs:434`, `:452`, `:458`, `:464` | pt | registro de execução (o ingest não produz) |
| `nature.rs:38`, `:42` | pt (com nome de campo em inglês) | {}: content desconhecido "{}". · {}: content "{}" contradiz o kind declarado. |
| `core/lib.rs:98-110` | en | fact.id must not be empty · fact.statement must not be empty · fact.source_id must not be empty · fact.line must be greater than zero · fact.end_line must not precede fact.line |

**Antes do envio** (`store.rs`, studio `claims.rs`, `check.rs`):

| Onde | Língua | Texto |
|---|---|---|
| `store.rs:969` | pt (com código en) | A pergunta {id} não tem pacote de evidências ({code}); ela não será enviada (códigos de `package.rs`, como `composition_failed`, `acervo_unreadable`, `claims_context_unreadable`, `persistence_failed`) |
| `store.rs:1068` | pt | Esta pergunta não tem query preparada (o prepare recusou); nada foi enviado |
| `store.rs:1079-1081` | pt + en | "H3: " na frente do erro do `prepare::verify` (em inglês, acima) ou do `claims::verify` |
| `store.rs:1087` | pt | Critérios prévios diferem dos gravados com a pergunta; nada foi enviado |
| `claims.rs:21`, `:28`, `:42`, `:47`, `:171` | pt | Query sem selections · Bloco de contexto sem fonte · Janela sem start_line · {id}: numeração diferente da janela · Texto esperado na query preparada |
| `claims.rs:236` | pt | Contexto ou query de claims difere do bundle conferido; nada foi enviado |
| `claims.rs` (llm) `:156`, `:168`, `:175`, `:187` | en | Context requires schema_version=1 and selected evidence · Invalid or duplicate evidence identity/line interval · Invalid or duplicate reviewed fact · Empty question |
| `check.rs:70-72` | pt | a forma de toda recusa do pacote: "{H1\|H2\|H3}: {motivo}; nada foi enviado" |
| `check.rs:81-172` | pt | os 22 motivos de H1 e H2, por exemplo "pacote ausente", "o pacote não é o que a marca nomeia", "{nome} difere do vinculado no pacote" (o da Parada 1: "query difere…"), "a edition do pacote não é a do bundle", "edition indisponível", "snapshot de {} indisponível" |
| `check.rs:176` | pt + en | "H2: " na frente do erro de `evidence::check_sources` |
| `check.rs:196-253` | pt | H3: "prepared/request.json ilegível", "preparação não refeita ({e})", "registro de seleção não refeito ({e})", "o código atual não refaz o registro de seleção", "pacote não refeito ({code})", "o código atual não refaz {parte} do pacote" |

**No envio** (`main.rs`, `llm.rs`, transporte):

| Onde | Língua | Texto |
|---|---|---|
| `main.rs:316`, `:329` | pt | Perfil sem envio: {profile} |
| `main.rs:326` | pt | Informe key, run, turn e profile |
| `main.rs:492` | pt | O Modelfile não é mais escolhido: escolha um perfil (profile); a tarefa vem da pergunta |
| `llm.rs:60`; `transport.rs:67` | pt | Perfil desconhecido: {profile} |
| `llm.rs:70` | pt | A skill {} não se aplica à tarefa {task}; nada foi enviado |
| `llm.rs:99`; `store.rs:1054` | pt | O registro do envio não foi gravado; a resposta não está vinculada |
| `transport.rs:70`, `:95`, `:133`, `:135`, `:137`, `:144`, `:155` | pt | Provedor não implementado · O SYSTEM com as skills não pode conter """ · Escolha um perfil local para o Ollama · Executável do Ollama não encontrado · Ollama não responde em {OLLAMA}… · O Modelfile escolhido não tem SYSTEM · ollama create falhou: {} |
| `transport.rs:233`, `:234`, `:236`, `:257`, `:281-283`, `:304-305` | pt | Claude Code CLI não encontrado… · Escolha o modelo do Claude · Modelo do Claude desconhecido · … · stdin/stdout/stderr indisponível · leitura de stdout/stderr falhou |
| `transport.rs:337`; `client.rs:148` | pt | Consulta mudou após conferência; nenhum envio realizado (estado `input_rejected`) |
| `ollama/mod.rs:14`, `:28`, `:33` | pt | Modelo deve ser explícito · Consulta deve conter question, evidence e instructions válidos · response_schema deve ser um JSON Schema de objeto |
| `client.rs:37` | pt | Use endpoint HTTP /api/generate sem credenciais/query/fragmento, modelo e limites positivos |
| `client.rs:87-121`, `transport.rs:313-318` | en | os estados: `completed`, `incomplete_response`, `invalid_response`, `provider_failed`, `transport_failed`, `timed_out`, `input_rejected` |

**Depois do envio:** a colagem (`store.rs:1034`, `:1039`, `:1043`, pt: "Cole a resposta antes de conferir", "Só perguntas em claims recebem resposta colada", "Pergunta em claims sem contexto"); os códigos da conferência (seção E, **en**, com texto pt/en só na página); os portões dos fatos (seção F, **pt**, menos `facts.rs:244`, "Item desconhecido", também pt).

**Avisos** (não recusam):

| Onde | Língua | Texto |
|---|---|---|
| `transport.rs:211-216` | pt | Entrada possivelmente truncada: o Ollama processou {n} tokens de uma query estimada em ~{m}… (`input.note`) |
| `client.rs:213-215` | pt | Limite de bytes excedido; preservado apenas o prefixo permitido |
| `resolve.rs:10-12` | en | os códigos de `resolutions` (seção E) |
| `claims.rs:11`; `facts.rs:22` | pt | os `limits` do `check.json` e de `/api/facts` |
| `bundle.rs:66-72`; `client.rs:179-181` | pt | o `validation_scope` e os `limits` do `origin.json` e do `prepared.json` |
| os limites do dossiê | en e pt | Parada 1, MAP-Q8 (os de busca em inglês, os do `prepare` em português) |

Contagem, pelas tabelas acima: o **prepare** recusa quase só em **inglês**, com duas exceções em português (`prepare.rs:574`, `:578`); o **graph-engine** recusa em **português**, com uma exceção (`graph_engine.rs:107`); o **core** recusa em **inglês**; o **studio**, o **pacote** e o **transporte** recusam em **português**; os **códigos** da conferência e os **estados** do transporte estão em **inglês**. Uma mesma recusa pode juntar as duas línguas (`store.rs:1079-1081`, `check.rs:176`, `store.rs:969`). Ver MAP-O98.

---

## Execução (Parada 2)

1. **Original antes:** o mesmo comando da Parada 1 (passo 2): healthchecks `61cd25c1…5ca3d5` (932) e renderer `612cdbf6…f3ecc` (42).
2. **Cópia:** `cp -r .studio/projects $S/p2/root/.studio/` e `cp -r modelfiles $S/p2/root/modelfiles`.
3. **Studio sobre a cópia:** `LIBRARIAN_ROOT=$S/p2/root target/release/librarian_studio.exe --port 8795` (o binário da Parada 1, sem recompilar). O log confirmou "Dados: …\scratchpad\p2\root\.studio".
4. **Conferência refeita:** `cargo build --release --locked -p librarian-llm-engine --bin claims` (já em dia). Para as perguntas 01, 05, 10 e 12: `target/release/claims.exe check <context.json> <corpo> <relatório-novo>`, uma vez com as claims gravadas no `check.json` e outra com o `attempt/response.txt` cru. Saída 0 na 01 resolvida; 2 nas outras.
5. **Revisões na cópia** (`POST /api/review`): C1 da 10 (Portão 1); C3 da 01 (já promovido); C1 da 07 sem nota; C5 da 12 (LACUNA); C1 da 14/01 (sem resposta); `decision: "talvez"`; C1 da 07 com nota (→ F27); C1 da 07 rejeitado (→ `revoked` F27).
6. **Caminho sem modelo:** `POST /api/gathered-question` com as 5 declarações da pergunta 01 e `with_facts: true` (pergunta 15, com 8 fatos); `POST /api/paste` com um FACT escrito à mão (`P1`, `ping`, operação 207); `POST /api/review` aprovado (→ F28).
7. **Decisão 18:** cópia dos arquivos rastreados do Healthchecks (`git ls-files | cp --parents`, em `$S/p2/hc`). O `root` do `project.json` **da cópia** foi trocado à mão para essa pasta (a rota recusa trocar a raiz, `store.rs:328-330`). Depois:
   - A: uma linha de comentário no fim de `hc/api/views.py` (`printf >>`), `POST /api/generate` (run A);
   - B: mais uma linha no topo (`sed -i '1i …'`), `POST /api/generate` (run B).
   
   Por fim, `GET /api/facts` nos três runs. O `sed -i` do Git Bash também tirou os CR do arquivo (por isso o deslocamento de bytes do `ping` no run B é menor). O estado dos fatos não depende disso: as linhas são comparadas sem o fim de linha (`prepare.rs:584-589`, `str::lines`). O que os derrubou foi a linha a mais no topo.
8. **Testes:** `cargo test --locked -p librarian-llm-engine` (60 passaram, 1 ignorado) e `cargo test --locked -p librarian-studio` (183 passaram); `cargo test -p librarian-studio -- --list` para os nomes. `git status --porcelain` vazio depois.
9. **Scripts** (só leem; em `S/scripts`): `e-checks.js` (resumo das conferências, revisões e fatos) e `msgs.js` (as mensagens de recusa, com linha e língua estimada, conferidas à mão).
10. **Fim:** o studio da cópia foi encerrado. O hash do original foi refeito: os mesmos dois valores e contagens; o original continua com o run `20261118-061046` só e com 26 linhas no `facts.jsonl`. O Healthchecks original: `git status --porcelain` vazio, em `6c05a7a`. As cópias foram apagadas (`rm -rf $S/p2`).

---

## Observações novas (Parada 2)

- **MAP-O90.** A conferência roda sobre a resposta **completada pelo Librarian**: o `resolve` move uma citação para a única faixa que tem a quote e acrescenta as citações da declaração e da operação (`resolve.rs:21-76`), antes do `check` (`claims.rs:243-254`). O `check.json` guarda as claims já completadas, e a resposta crua fica no `response.txt`. Caso real, refeito com o código de hoje: a pergunta 01 passa completada e teria 14 recusas crua; a 10, 6 contra 14; a 05, 4 contra 9. Nas 16 respostas foram 78 resoluções. O binário `claims` não aplica o `resolve` (`bin/claims.rs:13-17`).
- **MAP-O91.** A recusa é um código em inglês, sem texto. O texto vem de uma tabela da página, em pt e en (`index.html:5616-5746`). Dois códigos não estão nela (`text_line_lacks_named_code`, `edits_unbalanced`) e aparecem como "problema sem descrição". Caso real: `text_line_lacks_named_code` na pergunta 04.
- **MAP-O92.** O que passa é só mecânico: `accepted` é sempre `false` e `semantic_status` é sempre `pending` (`claims.rs` (llm) `:331-337`). A conferência não olha a pergunta nem o sentido do `text`.
- **MAP-O93.** O fato não aponta para o id do símbolo: aponta para caminho, nome, linhas e o texto citado (`facts.rs:59-62`). Por execução: uma linha a mais no **fim** de `views.py` mudou o id de todos os símbolos do arquivo, e os 12 fatos dele continuaram `current`; uma linha a mais no **topo** deixou os 12 `stale`. O fato não procura o texto em outro lugar (`facts.rs:253-254`). Ele volta numa pergunta pela igualdade de `declaration_line` (`store.rs:796`).
- **MAP-O94.** A revisão não registra quem revisou: a autoria é uma constante, "revisão humana no Librarian Studio" (`facts.rs:21`), e `/api/review` não tem campo para isso (`main.rs:201-211`). Caso real: os 26 fatos e as 44 revisões do Healthchecks têm a nota "Revisão do agente Claude no piloto Healthchecks (não é revisão humana; o contrato atual não tem campo para isso)", e os 26 saem com autoria "revisão humana".
- **MAP-O95.** Não há caminho direto para a pessoa gravar um fato. Há o indireto: escrever o JSON de claims, colar e aprovar. Verificado: o F28 da cópia, com os mesmos portões. O fato não diz que foi colado; isso só aparece no nome do envio (`01-manual`) e no `send.json` (`pasted: true`).
- **MAP-O96.** No código, "acervo" é o `facts.jsonl` do projeto (`facts.rs:165-166`, `package.rs:125`, `evidence.rs:536`), nunca a edition. O arquivo só recebe acréscimos e nada o assina (`facts.rs:296-301`). O número do fato é a contagem de promoções + 1 (`facts.rs:47`). O pacote de cada pergunta guarda os bytes das linhas de fato que usou.
- **MAP-O97.** Uma resposta sem texto não chega à conferência nem à revisão. Caso real: a pergunta 14, envios 01 e 02, com o Ollama em 500 ("llama-server process has terminated… CUDA error"), `provider_failed`, sem `check.json`. A revisão de um item deles recusa com "A resposta do envio não foi preservada".
- **MAP-O98.** As recusas estão em duas línguas, e às vezes as duas juntas (seção "As mensagens"): o prepare e o core em inglês; o graph-engine, o studio, o pacote e o transporte em português; os códigos e estados em inglês; e prefixos em português na frente de mensagens em inglês (`store.rs:1079-1081`, `check.rs:176`). Pela resposta do usuário à Parada 1, o back-end passa a ser todo em inglês.
- **MAP-O99.** Por grep, `unexpected_premises` e `unexpected_verification_plan` não são nomeados por nenhum teste. *O que o método não pega:* um teste que os produza sem escrever o código.

## Perguntas (Parada 2)

- **MAP-Q24.** O acervo inteiro do Healthchecks foi aprovado por um agente e diz "revisão humana" (MAP-O94). Para o mapa, isso fica registrado como um defeito dos dados do acervo, como a falta de um campo de quem revisou (a tratar no portão), ou como as duas coisas?
- **MAP-Q25.** O fato aponta para as linhas e o texto citado, e uma linha a mais no topo do arquivo deixa todos os fatos dele desatualizados (MAP-O93). Pela decisão 18 (o id de hoje muda com qualquer mudança no arquivo), o fato deve passar a apontar para o id novo, quando ele existir, ou continuar preso ao texto nas mesmas linhas?
- **MAP-Q26.** Colar uma resposta escrita à mão é hoje o único caminho para a pessoa gravar um fato sem modelo, e o fato não diz que foi colado (MAP-O95). Pela decisão 11, isso conta como "a pessoa grava o fato" (o agente e a pessoa entram pela mesma porta, o portão), ou deve haver uma entrada própria, marcada no fato?

---

## Em palavras simples

Quando alguém faz uma pergunta, o sistema monta uma pasta para ela. Ali ficam o pedido, as cópias dos arquivos de onde vêm as provas, a pergunta pronta para o modelo e uma ficha com as impressões digitais de tudo. Quase tudo fica guardado só para conferência. Para o modelo sai uma folha só: a pergunta, os trechos de código com o número de cada linha, os avisos sobre os limites das provas e as regras da resposta.

Antes de cada envio, o sistema refaz a folha e confere que ela saiu igual. Só então manda. Ele guarda o que mandou, o que voltou e quando, mas não a versão exata do modelo. A opção de não mandar só mostra a folha para copiar. Ela é igual à que iria, mas sem o texto de abertura que o modelo recebe junto, e não é conferida na hora de mostrar.

A resposta volta como uma lista de afirmações, cada uma apontando as linhas que a provam. O sistema confere cada uma contra os trechos que entregou. As linhas existem? O texto copiado é igual ao do código, letra por letra, até nos espaços? A afirmação fala só do que as linhas citadas mostram? Antes disso, ele faz uma ajuda pequena: se o modelo errou só o número da linha, mas copiou um trecho que só existe num lugar, ele acerta o número. Se o modelo disse onde a operação acontece e esqueceu de citar a linha, ele acrescenta a citação. Sem essa ajuda, a maior parte das respostas que passaram seria recusada. Mesmo quando tudo bate, a resposta só passou na conferência mecânica. Se ela está certa, quem decide é outra pessoa.

Uma afirmação que passou pode virar um fato guardado no acervo do projeto, que é uma lista onde só se acrescenta. Para isso, alguém aprova o item e escreve na nota por que ele é verdadeiro. O sistema confere de novo a pergunta, o envio e a resposta antes de aceitar. O fato guarda o arquivo, as linhas e o texto citado. Se o arquivo mudar em outro lugar, o fato continua valendo. Se as linhas citadas mudarem de lugar, ele fica desatualizado, mesmo com o texto igual. O sistema não guarda quem aprovou: todo fato diz que foi revisão humana. No projeto de teste, quem aprovou foi um agente, e a própria nota diz isso.

As permissões mudam pouco o pedido. Liberar a inferência ou a hipótese só abre mais uma opção na forma da resposta. Liberar a mudança de código acrescenta a regra de como propor a mudança e troca o texto de abertura. A peça que conversa com os modelos tem duas metades: uma manda e recebe, e a outra define a forma da resposta e a confere. Elas quase não dependem uma da outra.

Não há um botão para a pessoa escrever um fato direto. Dá para escrever a resposta à mão, colar e aprovar, como se viesse de um modelo. O fato não diz que foi assim.

As mensagens de recusa estão misturadas entre português e inglês, às vezes na mesma frase.
